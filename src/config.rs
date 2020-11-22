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
}

pub fn load_config(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    Ok(toml::from_str(&std::fs::read_to_string(path)?)?)
}
