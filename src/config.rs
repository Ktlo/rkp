use std::{
    collections::{BTreeSet, HashMap, HashSet},
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};

use serde::{Deserialize, Serialize};
use tokio::{fs, sync::RwLock};
use zeroize::{Zeroize, ZeroizeOnDrop};

const CONFIG_FILENAME: &str = "config.json";

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Config {
    #[serde(default)]
    pub listeners: Vec<Listener>,
    #[serde(default)]
    pub chains: HashMap<String, Vec<ChainRule>>,
    #[serde(default)]
    pub stash: Stash,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct Listener {
    #[serde(default)]
    pub kind: ListenerKind,
    #[serde(default = "default_listener_address")]
    pub address: SocketAddr,
    pub forward: String, // chain
}

fn default_listener_address() -> SocketAddr {
    SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::UNSPECIFIED), 80)
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq, Eq, Hash)]
pub enum ListenerKind {
    #[default]
    HTTP,
    TLS,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Stash {
    #[serde(default)]
    pub domain_pools: HashMap<String, DomainPool>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct DomainPool(#[serde(default)] pub BTreeSet<String>);

impl std::fmt::Debug for DomainPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut list = f.debug_list();
        if self.0.len() > 10 {
            list.entries(self.0.iter().take(10)).entry(&"<trimmed>")
        } else {
            list.entries(&self.0)
        }
        .finish()
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ChainRule {
    #[serde(default)]
    pub filter: ChainFilter,
    #[serde(default)]
    pub action: ChainAction,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub enum ChainFilter {
    #[default]
    Anything,
    DomainPool {
        pool: String,
    },
    DomainWildcard {
        wildcard: String,
    },
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub enum ChainAction {
    #[default]
    DirectConnect,
    GotoChain {
        chain: String,
    },
    Socks5Proxy {
        #[serde(default)]
        credentials: Option<Credentials>,
        address: String,
    },
    Drop,
}

fn default_socks_port() -> u16 {
    1080
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Credentials {
    pub username: String,
    pub password: Password,
}

#[derive(Serialize, Deserialize, Clone, Zeroize, ZeroizeOnDrop)]
pub struct Password(pub String);

impl std::fmt::Debug for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("<password>").finish()
    }
}

lazy_static::lazy_static! {
    static ref CONFIGURATION: RwLock<Arc<Config>> = RwLock::new(Arc::new(Config::default()));
}

pub async fn get_current_config() -> Arc<Config> {
    CONFIGURATION.read().await.clone()
}

pub async fn set_new_config(config: Config) {
    let mut listeners_to_delete: HashSet<Listener>;
    let mut listeners_to_create: HashSet<Listener>;
    {
        let mut configuration = CONFIGURATION.write().await;
        let old_config = configuration.as_ref();
        listeners_to_delete = old_config.listeners.clone().into_iter().collect();
        for listener in &config.listeners {
            listeners_to_delete.remove(&listener);
        }
        listeners_to_create = config.listeners.clone().into_iter().collect();
        for listener in &old_config.listeners {
            listeners_to_create.remove(&listener);
        }
        *configuration = Arc::new(config);
    }
    let config = get_current_config().await;
    log::info!("update configuration to {:?}", config);
    match serde_json::to_string(config.as_ref()) {
        Ok(contents) => match fs::write(CONFIG_FILENAME, contents).await {
            Ok(_) => {
                log::info!("saved updated configuration to disk")
            }
            Err(_) => {}
        },
        Err(_) => {}
    }
    for listener in listeners_to_delete {
        delete_listener(listener)
    }
    for listener in listeners_to_create {
        create_listener(listener)
    }
}

pub async fn init_config() {
    match fs::read(CONFIG_FILENAME).await {
        Ok(content_bytes) => match String::from_utf8(content_bytes) {
            Ok(content) => {
                log::info!("loaded configuration from disk");
                let parse_result: Result<Config, serde_json::Error> =
                    serde_json::from_str(&content);
                match parse_result {
                    Ok(config) => {
                        set_new_config(config).await;
                        log::info!("configuration is installed");
                    }
                    Err(error) => {
                        log::error!("failed to parse configuration: {}", error);
                    }
                }
            }
            Err(error) => {
                log::error!("failed to decode as utf8 configuration file: {}", error);
            }
        },
        Err(error) => {
            log::error!(
                "failed to read configuration file \"{}\": {}",
                CONFIG_FILENAME,
                error
            );
        }
    }
}

fn create_listener(listener: Listener) {}

fn delete_listener(listener: Listener) {}

#[test]
fn default_parse_test() {
    let config: serde_json::Result<Config> =
        serde_json::from_str("{\"listeners\":[{\"forward\":\"http\"}]}");
    println!("config: {:?}", config);
    assert!(config.is_ok());
}
