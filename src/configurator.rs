use std::collections::HashMap;

use hyper::StatusCode;
use warp::{Filter, Reply};

use crate::config::{self, Config, DomainPool};

pub async fn start() {
    let config = {
        let get = warp::get().then(get_config);
        let set = warp::put().and(warp::body::json()).then(set_config);
        warp::path!("config").and(get.or(set))
    };

    let listeners = {
        let get = warp::get().then(get_listeners);
        let set = warp::put().and(warp::body::json()).then(set_listeners);
        let add = warp::post().and(warp::body::json()).then(add_listeners);
        warp::path!("config" / "listeners").and(get.or(set).or(add))
    };

    let stash = {
        let get = warp::get().then(get_stash);
        let set = warp::put().and(warp::body::json()).then(set_stash);
        warp::path!("config" / "stash").and(get.or(set))
    };

    let domain_pools = {
        let get = warp::get().then(get_domain_pools);
        let set = warp::put().and(warp::body::json()).then(set_domain_pools);
        warp::path!("config" / "stash" / "domain_pools").and(get.or(set))
    };

    let domain_pool = {
        let path = warp::path!("config" / "stash" / "domain_pools" / String);
        let get = warp::get().and(path).then(get_domain_pool);
        let set = warp::put()
            .and(path)
            .and(warp::body::json())
            .then(set_domain_pool);
        let add = warp::post()
            .and(path)
            .and(warp::body::json())
            .then(add_domain_pool);
        let del = warp::delete().and(path).then(del_domain_pool);
        get.or(set).or(add).or(del)
    };

    let chains = {
        let get = warp::get().then(get_chains);
        let set = warp::put().and(warp::body::json()).then(set_chains);
        warp::path!("config" / "chains").and(get.or(set))
    };

    let chain = {
        let path = warp::path!("config" / "chains" / String);
        let get = warp::get().and(path).then(get_chain);
        let set = warp::put()
            .and(path)
            .and(warp::body::json())
            .then(set_chain);
        let add = warp::post()
            .and(path)
            .and(warp::body::json())
            .then(add_chain);
        let del = warp::delete().and(path).then(del_chain);
        get.or(set).or(add).or(del)
    };

    let routes = config
        .or(listeners)
        .or(stash)
        .or(domain_pools)
        .or(domain_pool)
        .or(chains)
        .or(chain);

    let addr = crate::args::Args::get().control;
    warp::serve(routes).run(addr).await;
}

async fn get_config() -> warp::reply::Json {
    let config = config::get_current_config().await;
    warp::reply::json(config.as_ref())
}

async fn set_config(config: Config) -> StatusCode {
    config::set_new_config(config).await;
    StatusCode::ACCEPTED
}

async fn get_listeners() -> warp::reply::Json {
    let config = config::get_current_config().await;
    warp::reply::json(&config.as_ref().listeners)
}

async fn set_listeners(listeners: Vec<config::Listener>) -> StatusCode {
    let old_config = config::get_current_config().await;
    let config = Config {
        listeners: listeners,
        ..old_config.as_ref().clone()
    };
    config::set_new_config(config).await;
    StatusCode::ACCEPTED
}

async fn add_listeners(listener: config::Listener) -> StatusCode {
    let old_config = config::get_current_config().await;
    let mut config = old_config.as_ref().clone();
    config.listeners.push(listener);
    config::set_new_config(config).await;
    StatusCode::ACCEPTED
}

async fn get_stash() -> warp::reply::Json {
    let config = config::get_current_config().await;
    warp::reply::json(&config.as_ref().stash)
}

async fn set_stash(stash: config::Stash) -> StatusCode {
    let old_config = config::get_current_config().await;
    let config = Config {
        stash: stash,
        ..old_config.as_ref().clone()
    };
    config::set_new_config(config).await;
    StatusCode::ACCEPTED
}

async fn get_domain_pools() -> warp::reply::Json {
    let config = config::get_current_config().await;
    warp::reply::json(&config.as_ref().stash.domain_pools)
}

async fn set_domain_pools(domain_pools: HashMap<String, config::DomainPool>) -> StatusCode {
    let old_config = config::get_current_config().await;
    let mut config = old_config.as_ref().clone();
    config.stash.domain_pools = domain_pools;
    config::set_new_config(config).await;
    StatusCode::ACCEPTED
}

async fn get_domain_pool(pool_name: String) -> warp::reply::Response {
    let config = config::get_current_config().await;
    let pool = config.as_ref().stash.domain_pools.get(&pool_name);
    match pool {
        Some(value) => warp::reply::json(value).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn set_domain_pool(pool_name: String, domain_pool: config::DomainPool) -> StatusCode {
    let old_config = config::get_current_config().await;
    let mut config = old_config.as_ref().clone();
    let old_domain_pool = config
        .stash
        .domain_pools
        .insert(String::from(pool_name), domain_pool);
    let status = if old_domain_pool.is_some() {
        StatusCode::ACCEPTED
    } else {
        StatusCode::CREATED
    };
    config::set_new_config(config).await;
    status
}

async fn add_domain_pool(pool_name: String, domain: String) -> StatusCode {
    let old_config = config::get_current_config().await;
    let mut config = old_config.as_ref().clone();
    let mut status = StatusCode::ACCEPTED;
    let domain_pool = config
        .stash
        .domain_pools
        .entry(String::from(pool_name))
        .or_insert_with(|| {
            status = StatusCode::CREATED;
            DomainPool::default()
        });
    domain_pool.0.insert(domain);
    config::set_new_config(config).await;
    status
}

async fn del_domain_pool(pool_name: String) -> StatusCode {
    let old_config = config::get_current_config().await;
    let mut config = old_config.as_ref().clone();
    let removed = config.stash.domain_pools.remove(&pool_name);
    config::set_new_config(config).await;
    if removed.is_some() {
        StatusCode::ACCEPTED
    } else {
        StatusCode::NOT_MODIFIED
    }
}

async fn get_chains() -> warp::reply::Json {
    let config = config::get_current_config().await;
    warp::reply::json(&config.as_ref().chains)
}

async fn set_chains(chains: HashMap<String, Vec<config::ChainRule>>) -> StatusCode {
    let old_config = config::get_current_config().await;
    let config = Config {
        chains: chains,
        ..old_config.as_ref().clone()
    };
    config::set_new_config(config).await;
    StatusCode::ACCEPTED
}

async fn get_chain(chain_name: String) -> warp::reply::Response {
    let config = config::get_current_config().await;
    let chain = config.as_ref().chains.get(&chain_name);
    match chain {
        Some(value) => warp::reply::json(value).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn set_chain(chain_name: String, chain: Vec<config::ChainRule>) -> StatusCode {
    let old_config = config::get_current_config().await;
    let mut config = old_config.as_ref().clone();
    let old_chain = config.chains.insert(String::from(chain_name), chain);
    let status = if old_chain.is_some() {
        StatusCode::ACCEPTED
    } else {
        StatusCode::CREATED
    };
    config::set_new_config(config).await;
    status
}

async fn add_chain(chain_name: String, chain_route: config::ChainRule) -> StatusCode {
    let old_config = config::get_current_config().await;
    let mut config = old_config.as_ref().clone();
    let mut status = StatusCode::ACCEPTED;
    let chain = config
        .chains
        .entry(String::from(chain_name))
        .or_insert_with(|| {
            status = StatusCode::CREATED;
            Vec::default()
        });
    chain.push(chain_route);
    config::set_new_config(config).await;
    status
}

async fn del_chain(chain_name: String) -> StatusCode {
    let old_config = config::get_current_config().await;
    let mut config = old_config.as_ref().clone();
    let removed = config.chains.remove(&chain_name);
    config::set_new_config(config).await;
    if removed.is_some() {
        StatusCode::ACCEPTED
    } else {
        StatusCode::NOT_MODIFIED
    }
}
