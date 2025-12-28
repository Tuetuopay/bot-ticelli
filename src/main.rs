#[macro_use]
extern crate diesel;

use clap::Parser;
use diesel_async::{
    pg::AsyncPgConnection,
    pooled_connection::{deadpool::Pool, AsyncDieselConnectionManager},
};
use opentelemetry::{
    sdk::{trace::Config, Resource},
    KeyValue,
};
use serenity::{framework::StandardFramework, model::id::UserId, prelude::*};
use tokio::spawn;
use tracing_subscriber::{prelude::*, EnvFilter};

mod bot;
mod cache;
mod cmd;
mod config;
mod cron;
mod error;
mod extensions;
mod models;
mod paginate;
mod schema;

use bot::Bot;
use cache::Cache;

struct PgPool;
impl TypeMapKey for PgPool {
    type Value = Pool<AsyncPgConnection>;
}

struct WinSentences;
impl TypeMapKey for WinSentences {
    type Value = Vec<String>;
}

struct BotUserId;
impl TypeMapKey for BotUserId {
    type Value = UserId;
}

/// A small Discord bot for managing picture-guessing games.
#[derive(Parser, Debug)]
#[clap(author, version)]
struct Args {
    /// Config file path.
    #[clap(short, long, default_value = "config.toml")]
    config: String,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let args = Args::parse();
    let config = config::load_config(&args.config)
        .map_err(|e| format!("Failed to load {}: {e}", args.config))
        .unwrap();

    // Install tracing framework with Jaeger sink
    if let Some(ref tracing) = config.tracing_config {
        if let Some(ref jaeger) = tracing.jaeger {
            let tags = vec![KeyValue::new("version", env!("CARGO_PKG_VERSION"))];
            let tracer = opentelemetry_jaeger::new_agent_pipeline()
                .with_endpoint(jaeger)
                .with_service_name("bot-ticelli")
                .with_trace_config(Config::default().with_resource(Resource::new(tags)))
                .install_simple()
                .expect("Failed to install jaeger tracing");
            let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
            tracing_subscriber::registry()
                .with(tracing_subscriber::fmt::layer())
                .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug")))
                .with(telemetry)
                .try_init()
                .expect("Failed to install tracing");
            tracing::info!("Installed jaeger tracing");
        }
    }

    // Connect to database
    tracing::info!("Connecting to postgres...");
    let manager =
        AsyncDieselConnectionManager::<AsyncPgConnection>::new(&config.db_config.database_url);
    let pool = Pool::builder(manager).build().expect("Failed to create connection pool");

    // Create client instance
    tracing::info!("Connecting to discord...");
    let mut framework = StandardFramework::new()
        .configure(|c| c.allow_dm(false).prefix(&config.bot_config.command_prefix))
        .group(&bot::GENERAL_GROUP)
        .help(&bot::CMD_HELP)
        .normal_message(bot::on_message)
        .before(bot::filter_command);

    if let Some(rl) = config.bot_config.ratelimit {
        for bucket in ["show_limiter", "pic_limiter"] {
            framework = framework
                .bucket(bucket, |b| {
                    if let Some(delay) = rl.delay {
                        b.delay(delay);
                    }
                    if let Some(time_span) = rl.time_span {
                        b.time_span(time_span);
                    }
                    if let Some(limit) = rl.limit {
                        b.limit(limit);
                    }
                    b
                })
                .await;
        }
    }

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MESSAGE_REACTIONS;
    let mut client = Client::builder(&config.auth.token, intents)
        .event_handler(Bot)
        .framework(framework)
        .type_map_insert::<PgPool>(pool.clone())
        .type_map_insert::<WinSentences>(config.bot_config.win_sentences)
        .type_map_insert::<Cache>(Cache::default())
        .await
        .expect("Failed to create discord client");

    if let Some(autoskip) = config.bot_config.auto_skip {
        spawn(cron::task_auto_skip(client.cache_and_http.http.clone(), pool, autoskip));
    }

    tracing::info!("Runing app...");
    client.start().await.expect("Client error")
}
