#[macro_use]
extern crate clap;
#[macro_use]
extern crate diesel;

use diesel::{r2d2::{ConnectionManager, Pool, PooledConnection}, PgConnection};
use serenity::prelude::*;
use serenity::framework::StandardFramework;

mod bot;
mod config;
mod schema;
mod models;

use bot::Bot;

struct PgPool;
impl TypeMapKey for PgPool {
    type Value = Pool<ConnectionManager<PgConnection>>;
}

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

    // Connect to database
    println!("Connecting to postgres...");
    let manager = ConnectionManager::<PgConnection>::new(&config.db_config.database_url);
    let pool = Pool::builder().build(manager).expect("Failed to create connection pool");

    // Create client instance
    println!("Connecting to discord...");
    let framework = StandardFramework::new()
        .configure(|c| c.allow_dm(false).prefix("!"))
        .group(&bot::GENERAL_GROUP)
        .help(&bot::CMD_HELP);

    let mut client = Client::new(&config.auth.token)
        .event_handler(Bot)
        .framework(framework)
        .await
        .expect("Failed to create discord client");
    client.data.write().await.insert::<PgPool>(pool);

    println!("Runing app...");
    client.start().await.expect("Client error")
}
