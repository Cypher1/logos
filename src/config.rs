use anyhow::Result;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub model: String,
    pub temperature: f32,
    pub max_tokens: i32,
    pub top_p: f32,
    pub top_k: u32,
    pub repeat_penalty: f32,
    pub stop: Vec<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
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
                temperature: 0.3,
                max_tokens: 512,
                top_p: 0.9,
                top_k: 40,
                repeat_penalty: 1.1,
                stop: vec![
                    "</think>".to_string(),
                    "\n\nWait,".to_string(),
                    "\n\nActually,".to_string(),
                ],
            });
        }
        let content = fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }
}
