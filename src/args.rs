use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
};

use clap::Parser;

#[derive(Debug, Clone)]
pub struct Listener {
    pub kind: ListenerKind,
    pub addr: SocketAddr,
    pub chain: String,
}

#[derive(Debug, Clone)]
pub enum ListenerKind {
    HTTP,
    TLS,
    MC,
}

impl FromStr for Listener {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut kind = ListenerKind::HTTP;
        let mut addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8080);
        let mut chain: Option<&str> = None;
        let params = s.split(",").map(|it| it.trim()).filter(|it| !it.is_empty());
        for param in params {
            let (key, value) = match param.find("=") {
                Some(tok) => (param[..tok].trim(), param[tok + 1..].trim()),
                None => return Err(anyhow::anyhow!("parameter \"{}\" has no value", param)),
            };
            match key {
                "kind" | "k" => {
                    kind = match FromStr::from_str(value) {
                        Ok(value) => value,
                        Err(error) => {
                            return Err(anyhow::anyhow!("in parameter \"{}\": {}", key, error))
                        }
                    }
                }
                "addr" | "a" => {
                    addr = match FromStr::from_str(value) {
                        Ok(value) => value,
                        Err(error) => {
                            return Err(anyhow::anyhow!("in parameter \"{}\": {}", key, error))
                        }
                    }
                }
                "chain" | "c" => chain = Some(value),
                _ => return Err(anyhow::anyhow!("unknown parameter \"{}\"", key)),
            }
        }
        match chain {
            Some(chain) => Ok(Listener {
                kind: kind,
                addr: addr,
                chain: chain.to_owned(),
            }),
            None => Err(anyhow::anyhow!("\"chain\" parameter is required")),
        }
    }
}

impl FromStr for ListenerKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_lowercase();
        match &lower[..] {
            "http" => Ok(ListenerKind::HTTP),
            "tls" => Ok(ListenerKind::TLS),
            "mc" => Ok(ListenerKind::MC),
            _ => Err(anyhow::anyhow!("unknown listener kind \"{}\"", s)),
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short = 'f', long, default_value_t = String::from("config.json"))]
    pub config: String,

    #[arg(short, long, default_value_t = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 1339))]
    pub control: SocketAddr,

    #[arg(short, long, default_value_t = String::from("log.yaml"))]
    pub logging: String,

    #[arg(short, long, num_args = 0..)]
    pub bind: Vec<Listener>,
}

impl Args {
    pub fn get() -> &'static Self {
        &ARGS
    }
}

lazy_static::lazy_static! {
    static ref ARGS: Args = Args::parse();
}
