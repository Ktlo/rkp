use core::{task, task::Poll};
use std::{future::Future, net::SocketAddr, pin::Pin, str::FromStr};

use crate::chain;
use hyper::{body::HttpBody, http, Body, Client, Request, Response, Server, Uri};
use tokio::net::TcpStream;

#[derive(Debug)]
struct HttpProxy {
    chain: String,
}

impl hyper::service::Service<Request<Body>> for HttpProxy {
    type Response = Response<Body>;
    type Error = anyhow::Error;
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

pub async fn actor(address: SocketAddr, chain: String) -> anyhow::Result<()> {
    let make_service = MakeHttpProxy {
        chain: chain.to_owned(),
    };
    let server = Server::try_bind(&address)?
        .http1_preserve_header_case(true)
        .http1_title_case_headers(true)
        .serve(make_service);
    Ok(server.await?)
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

async fn proxy(req: Request<Body>, chain: String) -> Result<Response<Body>, anyhow::Error> {
    let address = match req.headers().get(hyper::header::HOST) {
        Some(address) => match address.to_str() {
            Ok(value) => value,
            Err(error) => {
                log::debug!("bad Host header; drop: {}", error);
                return respond_status(http::StatusCode::BAD_REQUEST);
            }
        },
        None => {
            log::debug!("no Host header in http request; drop");
            return respond_status(http::StatusCode::BAD_REQUEST);
        }
    };
    let uri = Uri::from_str(&format!("proto://{}", address))?;
    let host = uri.host().unwrap();
    let port = match uri.port() {
        Some(port) => port.as_u16(),
        None => 80,
    };
    let address = format!("{}:{}", host, port);
    let context = chain::Context {
        host: host.to_owned(),
        port: port,
        address: address.to_owned(),
    };
    let connector = ChainConnector {
        context: context,
        start: chain.to_owned(),
    };
    let client = Client::builder()
        .http1_title_case_headers(true)
        .http1_preserve_header_case(true)
        .set_host(false)
        .build(connector);
    let old_uri = req.uri();
    let new_uri = Uri::builder().scheme("http").authority(address);
    let new_uri = if let Some(value) = old_uri.path_and_query() {
        new_uri.path_and_query(value.to_owned())
    } else {
        new_uri
    };
    let new_uri = new_uri
        .path_and_query(old_uri.path_and_query().unwrap().to_owned())
        .build()?;
    let builder = Request::builder()
        .method(req.method())
        .uri(new_uri)
        .version(req.version());
    let builder = req
        .headers()
        .into_iter()
        .fold(builder, |builder, (key, value)| builder.header(key, value));
    let new_req = builder.body(req.boxed())?;
    Ok(client.request(new_req).await?)
}

fn respond_status(status: http::StatusCode) -> Result<Response<Body>, anyhow::Error> {
    Ok(Response::builder().status(status).body(Body::empty())?)
}
