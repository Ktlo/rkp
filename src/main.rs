mod args;
mod config;
mod configurator;
mod logging;
mod tls;

use args::Args;

#[tokio::main]
async fn main() {
    logging::init_logging();
    config::init_config().await;

    configurator::start().await;
}
