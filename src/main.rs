#[macro_use]
extern crate clap;

use serenity::{async_trait, model::{channel::Message, gateway::Ready}, prelude::*};

mod config;

#[tokio::main]
async fn main() {
    let matches = clap_app!(bot =>
        (version: env!("CARGO_PKG_VERSION"))
        (author: env!("CARGO_PKG_AUTHORS"))
        (@arg CONFIG: -c --config "Config file path. Defaults to config.toml")
    ).get_matches();

    let config = matches.value_of("CONFIG").unwrap_or("config.toml");
    let config = config::load_config(&config)
        .map_err(|e| format!("Failed to load {}: {}", config, e))
        .unwrap();
}
