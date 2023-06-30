mod config;
mod configurator;

#[async_std::main]
async fn main() -> tide::Result<()> {
    config::initialize_config().await;
    let mut app = tide::new();
    configurator::setup_handlers(&mut app);
    app.listen("127.0.0.1:8080").await?;
    Ok(())
}
