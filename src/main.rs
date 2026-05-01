#[macro_use]
extern crate diesel;

use clap::Parser;
use diesel_async::{
    pg::AsyncPgConnection,
    pooled_connection::{AsyncDieselConnectionManager, deadpool::Pool},
};
use opentelemetry::{KeyValue, trace::TracerProvider};
use opentelemetry_otlp::{Protocol, WithExportConfig};
use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};
use poise::{Framework, FrameworkOptions, PrefixFrameworkOptions, samples::register_in_guild};
use serenity::prelude::*;
use tokio::spawn;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::context::Data;

mod bot;
mod cache;
mod cmd;
mod config;
mod context;
mod cron;
mod error;
mod extensions;
mod models;
mod schema;

use bot::Bot;

/// A small Discord bot for managing picture-guessing games.
#[derive(Parser, Debug)]
#[clap(author, version)]
struct Args {
    /// Config file path.
    #[clap(short, long, default_value = "config.toml")]
    config: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args = Args::parse();
    let config = config::load_config(&args.config)
        .map_err(|e| format!("Failed to load {}: {e}", args.config))
        .unwrap();

    // Install tracing framework with OTLP sink
    if let Some(ref tracing) = config.tracing_config {
        if let Some(ref otel) = tracing.otel {
            // You can thank the OTEL guys for such a complicated setup.
            let provider = SdkTracerProvider::builder()
                .with_resource(
                    Resource::builder()
                        .with_service_name("bot-ticelli")
                        .with_attribute(KeyValue::new("version", env!("CARGO_PKG_VERSION")))
                        .build(),
                )
                .with_batch_exporter(
                    opentelemetry_otlp::SpanExporter::builder()
                        .with_http()
                        .with_protocol(Protocol::HttpBinary)
                        .with_endpoint(otel)
                        .build()
                        .expect("Failed to build exporter"),
                )
                .build();
            tracing_subscriber::registry()
                .with(tracing_subscriber::fmt::layer())
                .with(
                    EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| EnvFilter::new("bot_ticelli=debug,warn")),
                )
                .with(tracing_opentelemetry::layer().with_tracer(provider.tracer("bot-ticelli")))
                .init();
            tracing::info!("Installed tracing");
        }
    }

    // Connect to database
    tracing::info!("Connecting to postgres...");
    let manager =
        AsyncDieselConnectionManager::<AsyncPgConnection>::new(&config.db_config.database_url);
    let pool = Pool::builder(manager).build().expect("Failed to create connection pool");

    let context = Data::new(&config, pool.clone());

    // Create client instance
    tracing::info!("Connecting to discord...");
    let framework = Framework::builder()
        .options(FrameworkOptions {
            commands: cmd::commands(),
            prefix_options: PrefixFrameworkOptions {
                prefix: Some(config.bot_config.command_prefix.clone()),
                ..Default::default()
            },
            on_error: |e| Box::pin(bot::on_error(e)),
            event_handler: |ctx, event, fw_ctx, data| {
                Box::pin(bot::on_event(ctx, event, fw_ctx, data))
            },
            ..Default::default()
        })
        .setup(|ctx, ready, framework| {
            Box::pin(async move {
                for guild in &ready.guilds {
                    register_in_guild(ctx, &framework.options().commands, guild.id).await?;
                }
                Ok(context)
            })
        })
        .build();

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MESSAGE_REACTIONS;
    let mut client = Client::builder(&config.auth.token, intents)
        .event_handler(Bot)
        .framework(framework)
        .await
        .expect("Failed to create discord client");

    if let Some(autoskip) = config.bot_config.auto_skip {
        spawn(cron::task_auto_skip(client.http.clone(), pool, autoskip));
    }

    tracing::info!("Runing app...");
    client.start().await.expect("Client error")
}
