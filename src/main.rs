mod args;
mod chain;
mod config;
mod configurator;
mod http_proxy;
mod logging;
mod tls;

#[tokio::main]
async fn main() {
    logging::init_logging();
    config::init_config().await;

    configurator::start().await;
}
