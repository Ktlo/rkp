use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
};

use serde::{Deserialize, Serialize};
use tokio::{fs, sync::RwLock};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::args;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Config {
    #[serde(default)]
    pub chains: HashMap<String, Vec<ChainRule>>,
    #[serde(default)]
    pub stash: Stash,
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
    {
        let mut configuration = CONFIGURATION.write().await;
        *configuration = Arc::new(config);
    }
    let config = get_current_config().await;
    log::info!("update configuration to {:?}", config);
    match serde_json::to_string(config.as_ref()) {
        Ok(contents) => match fs::write(&args::Args::get().config, contents).await {
            Ok(_) => {
                log::info!("saved updated configuration to disk")
            }
            Err(_) => {}
        },
        Err(_) => {}
    }
}

pub async fn init_config() {
    match fs::read(&args::Args::get().config).await {
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
                &args::Args::get().config,
                error
            );
        }
    }
}

#[test]
fn default_parse_test() {
    let config: serde_json::Result<Config> =
        serde_json::from_str("{\"listeners\":[{\"forward\":\"http\"}]}");
    println!("config: {:?}", config);
    assert!(config.is_ok());
}
