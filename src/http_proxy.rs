use core::{task, task::Poll};
use std::{future::Future, net::SocketAddr, pin::Pin};

use crate::chain;
use hyper::{http, Body, Client, Request, Response, Server, Uri};
use tokio::net::TcpStream;

#[derive(Debug)]
struct HttpProxy {
    chain: String,
}

impl hyper::service::Service<Request<Body>> for HttpProxy {
    type Response = Response<Body>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        Box::pin(proxy(req, self.chain.to_owned()))
    }
}

struct MakeHttpProxy {
    chain: String,
}

impl<T> hyper::service::Service<T> for MakeHttpProxy {
    type Response = HttpProxy;
    type Error = std::io::Error;
    type Future = std::future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, _: T) -> Self::Future {
        std::future::ready(Ok(HttpProxy {
            chain: self.chain.clone(),
        }))
    }
}

pub async fn actor(address: &SocketAddr, chain: &String) {
    let make_service = MakeHttpProxy {
        chain: chain.to_owned(),
    };

    let server = match Server::try_bind(&address) {
        Ok(builder) => builder
            .http1_preserve_header_case(true)
            .http1_title_case_headers(true)
            .serve(make_service),
        Err(error) => {
            log::error!("failed to bind a port: {}", error);
            return;
        }
    };
    if let Err(error) = server.await {
        log::error!("error in http proxy: {}", error);
    };
}

#[derive(Clone)]
struct ChainConnector {
    context: chain::Context,
    start: String,
}

impl hyper::service::Service<Uri> for ChainConnector {
    type Response = TcpStream;
    type Error = anyhow::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: Uri) -> Self::Future {
        Box::pin(chain::connect(
            self.context.to_owned(),
            self.start.to_owned(),
        ))
    }
}

async fn proxy(req: Request<Body>, chain: String) -> Result<Response<Body>, hyper::Error> {
    let uri = req.uri();
    let host = match uri.host() {
        Some(host) => host,
        None => {
            log::debug!("no Host header in http request; drop");
            return respond_status(http::StatusCode::BAD_REQUEST);
        }
    };
    let port = match uri.port() {
        Some(port) => port.as_u16(),
        None => 80,
    };
    let address = format!("{}:{}", host, port);
    let context = chain::Context {
        host: host.to_owned(),
        port: port,
        address: address,
    };
    let connector = ChainConnector {
        context: context,
        start: chain.to_owned(),
    };
    let client = Client::builder()
        .http1_title_case_headers(true)
        .http1_preserve_header_case(true)
        .build(connector);
    client.request(req).await
}

fn respond_status(status: http::StatusCode) -> Result<Response<Body>, hyper::Error> {
    Ok(Response::builder()
        .status(status)
        .body(Body::empty())
        .unwrap())
}
