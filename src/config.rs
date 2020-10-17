/*!
 * Bot-ter-en-touche config file
 */

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub auth: AuthConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuthConfig {
    /// Authentication token for the discord bot
    pub token: String,
}

pub fn load_config(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    Ok(toml::from_str(&std::fs::read_to_string(path)?)?)
}
