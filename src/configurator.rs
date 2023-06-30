use std::collections::HashMap;

use serde::Serialize;
use tide::{http::mime, Request, Response, Server, StatusCode};

use crate::config::{self, Config, DomainPool};

pub fn setup_handlers(app: &mut Server<()>) {
    app.at("/config").get(get_config);
    app.at("/config").put(set_config);
    app.at("/config/listeners").get(get_listeners);
    app.at("/config/listeners").put(set_listeners);
    app.at("/config/listeners").post(add_listeners);
    app.at("/config/stash").get(get_stash);
    app.at("/config/stash").put(set_stash);
    app.at("/config/stash/domain_pools").get(get_domain_pools);
    app.at("/config/stash/domain_pools").put(set_domain_pools);
    app.at("/config/stash/domain_pools/:pool")
        .get(get_domain_pool);
    app.at("/config/stash/domain_pools/:pool")
        .put(set_domain_pool);
    app.at("/config/stash/domain_pools/:pool")
        .post(add_domain_pool);
    app.at("/config/stash/domain_pools/:pool")
        .delete(del_domain_pool);
    app.at("/config/chains").get(get_chains);
    app.at("/config/chains").put(set_chains);
    app.at("/config/chains/:chain").get(get_chain);
    app.at("/config/chains/:chain").put(set_chain);
    app.at("/config/chains/:chain").post(add_chain);
    app.at("/config/chains/:chain").delete(del_chain);
}

async fn get_config(_req: Request<()>) -> tide::Result {
    let config = config::get_current_config().await;
    json_response(StatusCode::Ok, &config.as_ref())
}

async fn set_config(mut req: Request<()>) -> tide::Result {
    let config: config::Config = req.body_json().await?;
    config::set_new_config(config).await;
    simple_response(StatusCode::Accepted)
}

async fn get_listeners(_req: Request<()>) -> tide::Result {
    let config = config::get_current_config().await;
    json_response(StatusCode::Ok, &config.as_ref().listeners)
}

async fn set_listeners(mut req: Request<()>) -> tide::Result {
    let listeners: Vec<config::Listener> = req.body_json().await?;
    let old_config = config::get_current_config().await;
    let config = Config {
        listeners: listeners,
        ..old_config.as_ref().clone()
    };
    config::set_new_config(config).await;
    simple_response(StatusCode::Accepted)
}

async fn add_listeners(mut req: Request<()>) -> tide::Result {
    let listener: config::Listener = req.body_json().await?;
    let old_config = config::get_current_config().await;
    let mut config = old_config.as_ref().clone();
    config.listeners.push(listener);
    config::set_new_config(config).await;
    simple_response(StatusCode::Accepted)
}

async fn get_stash(_req: Request<()>) -> tide::Result {
    let config = config::get_current_config().await;
    json_response(StatusCode::Ok, &config.as_ref().stash)
}

async fn set_stash(mut req: Request<()>) -> tide::Result {
    let stash: config::Stash = req.body_json().await?;
    let old_config = config::get_current_config().await;
    let config = Config {
        stash: stash,
        ..old_config.as_ref().clone()
    };
    config::set_new_config(config).await;
    simple_response(StatusCode::Accepted)
}

async fn get_domain_pools(_req: Request<()>) -> tide::Result {
    let config = config::get_current_config().await;
    json_response(StatusCode::Ok, &config.as_ref().stash.domain_pools)
}

async fn set_domain_pools(mut req: Request<()>) -> tide::Result {
    let domain_pools: HashMap<String, config::DomainPool> = req.body_json().await?;
    let old_config = config::get_current_config().await;
    let mut config = old_config.as_ref().clone();
    config.stash.domain_pools = domain_pools;
    config::set_new_config(config).await;
    simple_response(StatusCode::Accepted)
}

async fn get_domain_pool(req: Request<()>) -> tide::Result {
    let pool_name = req.param("pool")?;
    let config = config::get_current_config().await;
    let pool = config.as_ref().stash.domain_pools.get(pool_name);
    match pool {
        Some(value) => json_response(StatusCode::Ok, value),
        None => simple_response(StatusCode::NotFound),
    }
}

async fn set_domain_pool(mut req: Request<()>) -> tide::Result {
    let domain_pool: config::DomainPool = req.body_json().await?;
    let pool_name = req.param("pool")?;
    let old_config = config::get_current_config().await;
    let mut config = old_config.as_ref().clone();
    let old_domain_pool = config
        .stash
        .domain_pools
        .insert(String::from(pool_name), domain_pool);
    let status = if old_domain_pool.is_some() {
        StatusCode::Accepted
    } else {
        StatusCode::Created
    };
    config::set_new_config(config).await;
    simple_response(status)
}

async fn add_domain_pool(mut req: Request<()>) -> tide::Result {
    let domain: String = req.body_json().await?;
    let pool_name = req.param("pool")?;
    let old_config = config::get_current_config().await;
    let mut config = old_config.as_ref().clone();
    let mut status = StatusCode::Accepted;
    let domain_pool = config
        .stash
        .domain_pools
        .entry(String::from(pool_name))
        .or_insert_with(|| {
            status = StatusCode::Created;
            DomainPool::default()
        });
    domain_pool.0.insert(domain);
    config::set_new_config(config).await;
    simple_response(status)
}

async fn del_domain_pool(req: Request<()>) -> tide::Result {
    let pool_name = req.param("pool")?;
    let old_config = config::get_current_config().await;
    let mut config = old_config.as_ref().clone();
    let removed = config.stash.domain_pools.remove(pool_name);
    config::set_new_config(config).await;
    let status = if removed.is_some() {
        StatusCode::Accepted
    } else {
        StatusCode::NotModified
    };
    simple_response(status)
}

async fn get_chains(_req: Request<()>) -> tide::Result {
    let config = config::get_current_config().await;
    json_response(StatusCode::Ok, &config.as_ref().chains)
}

async fn set_chains(mut req: Request<()>) -> tide::Result {
    let chains: HashMap<String, Vec<config::ChainRoute>> = req.body_json().await?;
    let old_config = config::get_current_config().await;
    let config = Config {
        chains: chains,
        ..old_config.as_ref().clone()
    };
    config::set_new_config(config).await;
    simple_response(StatusCode::Accepted)
}

async fn get_chain(req: Request<()>) -> tide::Result {
    let chain_name = req.param("chain")?;
    let config = config::get_current_config().await;
    let chain = config.as_ref().chains.get(chain_name);
    match chain {
        Some(value) => json_response(StatusCode::Ok, value),
        None => simple_response(StatusCode::NotFound),
    }
}

async fn set_chain(mut req: Request<()>) -> tide::Result {
    let chain: Vec<config::ChainRoute> = req.body_json().await?;
    let chain_name = req.param("chain")?;
    let old_config = config::get_current_config().await;
    let mut config = old_config.as_ref().clone();
    let old_chain = config.chains.insert(String::from(chain_name), chain);
    let status = if old_chain.is_some() {
        StatusCode::Accepted
    } else {
        StatusCode::Created
    };
    config::set_new_config(config).await;
    simple_response(status)
}

async fn add_chain(mut req: Request<()>) -> tide::Result {
    let chain_route: config::ChainRoute = req.body_json().await?;
    let chain_name = req.param("chain")?;
    let old_config = config::get_current_config().await;
    let mut config = old_config.as_ref().clone();
    let mut status = StatusCode::Accepted;
    let chain = config
        .chains
        .entry(String::from(chain_name))
        .or_insert_with(|| {
            status = StatusCode::Created;
            Vec::default()
        });
    chain.push(chain_route);
    config::set_new_config(config).await;
    simple_response(status)
}

async fn del_chain(req: Request<()>) -> tide::Result {
    let chain_name = req.param("chain")?;
    let old_config = config::get_current_config().await;
    let mut config = old_config.as_ref().clone();
    let removed = config.chains.remove(chain_name);
    config::set_new_config(config).await;
    let status = if removed.is_some() {
        StatusCode::Accepted
    } else {
        StatusCode::NotModified
    };
    simple_response(status)
}

fn simple_response(status: StatusCode) -> tide::Result {
    let response = Response::builder(status).build();
    Ok(response)
}

fn json_response<T: Serialize>(status: StatusCode, body: &T) -> tide::Result {
    let contents = serde_json::to_string(body)?;
    let response = Response::builder(status)
        .body(contents)
        .content_type(mime::JSON)
        .build();
    Ok(response)
}
