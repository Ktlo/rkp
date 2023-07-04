mod args;
mod chain;
mod config;
mod configurator;
mod http_proxy;
mod logging;
mod server;
mod tls_proxy;

#[tokio::main]
async fn main() {
    logging::init_logging();
    config::init_config().await;

    futures::future::try_join(configurator::start(), server::start())
        .await
        .unwrap();
}
