use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::error::Error;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub model: String,
    pub temperature: f32,
    pub max_tokens: usize,
    pub enable_logging: bool,
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let path = Path::from("config.toml");
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
