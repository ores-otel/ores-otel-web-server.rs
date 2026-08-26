#![forbid(unsafe_code)]

use ores_otel_web_server::{config::WebConfig, server};

fn main() {
    let cfg = WebConfig::from_env();
    server::run(&cfg);
}

