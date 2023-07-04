use crate::{args, http_proxy};

pub async fn start() -> anyhow::Result<()> {
    let args = args::Args::get();
    let tasks = args.bind.iter().map(|listener| match listener.kind {
        args::ListenerKind::HTTP => {
            http_proxy::actor(listener.addr.clone(), listener.chain.clone())
        }
        args::ListenerKind::TLS => todo!("not implemented yet"),
    });
    futures::future::try_join_all(tasks).await?;
    Ok(())
}
