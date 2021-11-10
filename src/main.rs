#[macro_use]
extern crate clap;
#[macro_use]
extern crate diesel;

use diesel::{r2d2::{ConnectionManager, Pool, PooledConnection}, PgConnection};
use opentelemetry::KeyValue;
use serenity::{
    framework::StandardFramework,
    prelude::*,
};
use tracing_subscriber::{EnvFilter, prelude::*};

mod bot;
mod cmd;
mod cache;
mod config;
mod error;
mod extensions;
mod models;
mod paginate;
mod schema;

use bot::Bot;
use cache::Cache;

struct PgPool;
impl TypeMapKey for PgPool {
    type Value = Pool<ConnectionManager<PgConnection>>;
}
pub type PgPooledConn = PooledConnection<ConnectionManager<PgConnection>>;

struct WinSentences;
impl TypeMapKey for WinSentences {
    type Value = Vec<String>;
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let matches = clap_app!(bot =>
        (version: env!("CARGO_PKG_VERSION"))
        (author: env!("CARGO_PKG_AUTHORS"))
        (@arg CONFIG: -c --config +takes_value "Config file path. Defaults to config.toml")
    ).get_matches();

    let config = matches.value_of("CONFIG").unwrap_or("config.toml");
    let config = config::load_config(&config)
        .map_err(|e| format!("Failed to load {}: {}", config, e))
        .unwrap();

    // Install tracing framework with Jaeger sink
    if let Some(ref tracing) = config.tracing_config {
        if let Some(ref jaeger) = tracing.jaeger {
            let tracer = opentelemetry_jaeger::new_pipeline()
                .with_agent_endpoint(jaeger)
                .with_service_name("bot-ticelli")
                .with_tags(vec![KeyValue::new("version", env!("CARGO_PKG_VERSION"))])
                .install_simple()
                .expect("Failed to install jaeger tracing");
            let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
            tracing_subscriber::registry()
                .with(tracing_subscriber::fmt::layer())
                .with(EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("debug")))
                .with(telemetry)
                .try_init()
                .expect("Failed to install tracing");
            tracing::info!("Installed jaeger tracing");
        }
    }

    // Connect to database
    tracing::info!("Connecting to postgres...");
    let manager = ConnectionManager::<PgConnection>::new(&config.db_config.database_url);
    let pool = Pool::builder().build(manager).expect("Failed to create connection pool");

    // Create client instance
    tracing::info!("Connecting to discord...");
    let mut framework = StandardFramework::new()
        .configure(|c| c.allow_dm(false).prefix(&config.bot_config.command_prefix))
        .group(&bot::GENERAL_GROUP)
        .help(&bot::CMD_HELP)
        .normal_message(bot::on_message)
        .before(bot::filter_command);

    if let Some(rl) = config.bot_config.ratelimit {
        framework = framework.bucket("command_limiter", |b| {
            if let Some(delay) = rl.delay { b.delay(delay); }
            if let Some(time_span) = rl.time_span { b.time_span(time_span); }
            if let Some(limit) = rl.limit { b.limit(limit); }
            b
        }).await;
    }

    let mut client = Client::builder(&config.auth.token)
        .event_handler(Bot)
        .framework(framework)
        .type_map_insert::<PgPool>(pool)
        .type_map_insert::<WinSentences>(config.bot_config.win_sentences)
        .type_map_insert::<Cache>(Cache::default())
        .await
        .expect("Failed to create discord client");

    tracing::info!("Runing app...");
    client.start().await.expect("Client error")
}
