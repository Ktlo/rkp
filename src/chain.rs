use anyhow::Result;
use fast_socks5::client::Socks5Stream;
use tokio::net::TcpStream;
use wildmatch::WildMatch;

use crate::config::{self, ChainAction, ChainFilter, ChainRule, Credentials};

#[derive(Clone, Debug)]
pub struct Context {
    pub host: String,
    pub port: u16,
    pub address: String,
}

pub async fn connect(context: Context, start: String) -> Result<TcpStream, anyhow::Error> {
    log::debug!("resolve proxy stream for context: {:?}", &context);
    let config = config::get_current_config().await;
    resolve(&config, &context, &start).await
}

#[async_recursion::async_recursion]
async fn resolve(config: &config::Config, context: &Context, start: &str) -> Result<TcpStream> {
    match config.chains.get(start) {
        Some(chain) => resolve_chain(config, context, chain).await,
        None => direct_connect(&context.address).await,
    }
}

async fn resolve_chain(
    config: &config::Config,
    context: &Context,
    chain: &Vec<ChainRule>,
) -> Result<TcpStream> {
    for rule in chain {
        let matches = match &rule.filter {
            ChainFilter::Anything => true,
            ChainFilter::DomainPool { pool } => match config.stash.domain_pools.get(pool) {
                Some(pool) => pool.0.contains(&context.host),
                None => {
                    log::warn!("domain pool \"{}\" not found in configuration", pool);
                    false
                }
            },
            ChainFilter::DomainWildcard { wildcard } => {
                WildMatch::new(&wildcard).matches(&context.host)
            }
        };
        if matches {
            return match &rule.action {
                ChainAction::DirectConnect => direct_connect(&context.address).await,
                ChainAction::GotoChain { chain } => resolve(config, context, &chain).await,
                ChainAction::Socks5Proxy {
                    credentials,
                    address,
                } => socks5_connect(address, &context.host, context.port, credentials).await,
                ChainAction::Forward { address } => direct_connect(&address).await,
                ChainAction::Drop => Err(anyhow::anyhow!("drop")),
            };
        }
    }
    direct_connect(&context.address).await
}

async fn socks5_connect(
    address: &str,
    host: &str,
    port: u16,
    credentials: &Option<Credentials>,
) -> Result<TcpStream> {
    let config = fast_socks5::client::Config::default();
    let target_addr = String::from(host);
    let stream = match credentials {
        Some(Credentials { username, password }) => {
            Socks5Stream::connect_with_password(
                address,
                target_addr,
                port,
                String::from(username),
                String::from(&password.0),
                config,
            )
            .await
        }
        None => Socks5Stream::connect(address, target_addr, port, config).await,
    }
    .or_else(|error| {
        log::error!(
            "failed to create a direct connection with {}: {}",
            address,
            error
        );
        Err(error)
    })?;
    Ok(stream.get_socket())
}

async fn direct_connect(address: &str) -> Result<TcpStream> {
    let stream = TcpStream::connect(address).await.or_else(|error| {
        log::error!(
            "failed to create a direct connection with {}: {}",
            address,
            error
        );
        Err(error)
    })?;
    Ok(stream)
}
