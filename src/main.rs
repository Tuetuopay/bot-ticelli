#[macro_use]
extern crate clap;

use serenity::prelude::*;
use serenity::framework::StandardFramework;

mod bot;
mod config;

use bot::Bot;

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

    // Create client instance
    println!("Connecting to discord...");
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("!"))
        .group(&bot::GENERAL_GROUP)
        .help(&bot::CMD_HELP);

    let mut client = Client::new(&config.auth.token)
        .event_handler(Bot)
        .framework(framework)
        .await
        .expect("Failed to create discord client");

    println!("Runing app...");
    client.start().await.expect("Client error")
}
