use std::pin::Pin;

use futures::Future;

use crate::{args, http_proxy, mc_proxy, tls_proxy};

pub async fn start() -> anyhow::Result<()> {
    let args = args::Args::get();
    let tasks = args.bind.iter().map(|listener| match listener.kind {
        args::ListenerKind::HTTP => {
            let f: Pin<Box<dyn Future<Output = Result<(), anyhow::Error>>>> = Box::pin(
                http_proxy::actor(listener.addr.clone(), listener.chain.clone()),
            );
            f
        }
        args::ListenerKind::TLS => Box::pin(tls_proxy::actor(
            listener.addr.clone(),
            listener.chain.clone(),
        )),
        args::ListenerKind::MC => Box::pin(mc_proxy::actor(
            listener.addr.clone(),
            listener.chain.clone(),
        )),
    });
    futures::future::try_join_all(tasks).await?;
    Ok(())
}
