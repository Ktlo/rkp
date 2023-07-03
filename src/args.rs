use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short = 'f', long, default_value_t = String::from("config.json"))]
    pub config: String,

    #[arg(short, long, default_value_t = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 1339))]
    pub control: SocketAddr,

    #[arg(short, long, default_value_t = String::from("log.yaml"))]
    pub logging: String,
}

impl Args {
    pub fn get() -> &'static Self {
        &ARGS
    }
}

lazy_static::lazy_static! {
    static ref ARGS: Args = Args::parse();
}
