use serde::Deserialize;
use std::error::Error;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub model: String,
    pub temperature: f32,
    pub max_tokens: usize,
    pub enable_logging: bool,
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let path = if let Ok(x_dg_config_home) = std::env::var("XDG_CONFIG_HOME") {
            std::path::PathBuf::from(x_dg_config_home).join("logos/config.toml")
        } else {
            let mut p =
                std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()));
            p.push(".config");
            p.push("logos");
            p.push("config.toml");
            p
        };
        if !path.exists() {
            // Default configuration if file is missing
            return Ok(Config {
                model: "gpt-4o".to_string(),
                temperature: 0.7,
                max_tokens: 2048,
                enable_logging: true,
            });
        }
        let content = fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }
}
