use log::LevelFilter;
use log4rs::{
    append::console::ConsoleAppender,
    config::{Appender, Root},
};

use crate::args::Args;

const EXPECTAION_MSG: &str = "failed to initialize the default logging configuration";

pub fn init_logging() {
    log4rs::init_file(&Args::get().logging, Default::default()).unwrap_or_else(|_error| {
        let stdout = ConsoleAppender::builder().build();
        let config = log4rs::Config::builder()
            .appender(Appender::builder().build("stdout", Box::new(stdout)))
            .build(Root::builder().appender("stdout").build(LevelFilter::Info))
            .expect(EXPECTAION_MSG);
        log4rs::init_config(config).expect(EXPECTAION_MSG);
        log::warn!("no logging configuration found; use the default one");
    });
}
