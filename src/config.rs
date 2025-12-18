use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub api_token: String,
    pub account_id: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        toml::from_str(&content).context("Failed to parse config file")
    }

    pub fn path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().context("Could not find config directory")?;
        Ok(config_dir.join("cloudflare").join("config.toml"))
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}
