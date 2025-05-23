/*!
 * Bot-ter-en-touche config file
 */

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub auth: AuthConfig,
    pub db_config: DbConfig,
    pub bot_config: BotConfig,
    pub tracing_config: Option<TracingConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuthConfig {
    /// Authentication token for the discord bot
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct DbConfig {
    /// Database URL
    pub database_url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct BotConfig {
    /// Discord command prefix
    pub command_prefix: String,
    /// Sentences to use on win
    pub win_sentences: Vec<String>,
    /// Command ratelimiting
    pub ratelimit: Option<RatelimitConfig>,
    /// Automatic picture skipping
    pub auto_skip: Option<AutoskipConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct RatelimitConfig {
    /// The "break" time, in seconds, between invocations of a command.
    pub delay: Option<u64>,
    /// How long, in seconds, the ratelimit will apply for
    pub time_span: Option<u64>,
    /// Number of invocations allowed per `time_span`
    pub limit: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct AutoskipConfig {
    /// Delay for auto-skipping pictures, in seconds.
    pub autoskip_delay: u32,
    /// Delay before warning the picture will be auto-skipped, in seconds.
    pub warn_delay: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct TracingConfig {
    /// Where to send opentelemetry data, in Jaeger format. `<ip|hostname>:<port>`.
    pub jaeger: Option<String>,
    // TODO sentry, prometheus
}

pub fn load_config(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    Ok(toml::from_str(&std::fs::read_to_string(path)?)?)
}
