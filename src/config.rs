/*!
 * Bot-ter-en-touche config file
 */

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub auth: AuthConfig,
    pub db_config: DbConfig,
    pub bot_config: BotConfig,
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

pub fn load_config(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    Ok(toml::from_str(&std::fs::read_to_string(path)?)?)
}
