use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteConfig {
    pub api_token: String,
    pub account_id: String,
    /// Email for Global API Key auth (if set, uses X-Auth-Key + X-Auth-Email)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub default_site: Option<String>,
    #[serde(default)]
    pub sites: HashMap<String, SiteConfig>,

    // Legacy fields for backward compatibility
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            bail!(
                "Config file not found: {}\nRun 'cloudflare config set' to configure.",
                path.display()
            );
        }
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

    /// Get site config - requires explicit -s flag
    pub fn get_site(&self, site: Option<&str>) -> Result<SiteConfig> {
        let Some(s) = site else {
            bail!(
                "Site required. Use -s <site> to specify. Available sites: {}",
                self.list_sites()
            );
        };

        if let Some(cfg) = self.sites.get(s) {
            return Ok(cfg.clone());
        }

        bail!(
            "Site '{}' not found in config. Available sites: {}",
            s,
            self.list_sites()
        );
    }

    pub fn list_sites(&self) -> String {
        if self.sites.is_empty() {
            return "(none)".to_string();
        }
        self.sites
            .keys()
            .map(|s| {
                if Some(s) == self.default_site.as_ref() {
                    format!("{}*", s)
                } else {
                    s.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn set_site(&mut self, name: &str, token: &str, account_id: &str, set_default: bool) {
        self.sites.insert(
            name.to_string(),
            SiteConfig {
                api_token: token.to_string(),
                account_id: account_id.to_string(),
                email: None,
            },
        );
        if set_default || self.default_site.is_none() {
            self.default_site = Some(name.to_string());
        }
    }

    pub fn remove_site(&mut self, name: &str) -> bool {
        let removed = self.sites.remove(name).is_some();
        if self.default_site.as_deref() == Some(name) {
            self.default_site = self.sites.keys().next().cloned();
        }
        removed
    }
}
