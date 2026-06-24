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
    #[cfg_attr(coverage_nightly, coverage(off))]
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

    #[cfg_attr(coverage_nightly, coverage(off))]
    pub fn path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().context("Could not find config directory")?;
        Ok(config_dir.join("cloudflare").join("config.toml"))
    }

    #[cfg_attr(coverage_nightly, coverage(off))]
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

#[cfg(test)]
mod tests {
    use super::*;

    fn site(token: &str, account_id: &str) -> SiteConfig {
        SiteConfig {
            api_token: token.to_string(),
            account_id: account_id.to_string(),
            email: None,
        }
    }

    #[test]
    fn get_site_requires_explicit_site() {
        let mut config = Config::default();
        config
            .sites
            .insert("prod".to_string(), site("token", "acct"));

        let error = config.get_site(None).unwrap_err().to_string();

        assert!(error.contains("Site required"));
        assert!(error.contains("prod"));
    }

    #[test]
    fn get_site_returns_named_site() {
        let mut config = Config::default();
        config
            .sites
            .insert("prod".to_string(), site("token", "acct"));

        let found = config.get_site(Some("prod")).unwrap();

        assert_eq!(found.api_token, "token");
        assert_eq!(found.account_id, "acct");
    }

    #[test]
    fn get_site_reports_available_sites_when_missing() {
        let mut config = Config::default();
        config
            .sites
            .insert("prod".to_string(), site("token", "acct"));

        let error = config.get_site(Some("stage")).unwrap_err().to_string();

        assert!(error.contains("Site 'stage' not found"));
        assert!(error.contains("prod"));
    }

    #[test]
    fn list_sites_marks_default_site() {
        let mut config = Config::default();
        config
            .sites
            .insert("stage".to_string(), site("stage", "acct1"));
        config
            .sites
            .insert("prod".to_string(), site("prod", "acct2"));
        config.default_site = Some("prod".to_string());

        let sites = config.list_sites();

        assert!(sites.contains("prod*"));
        assert!(sites.contains("stage"));
    }

    #[test]
    fn list_sites_reports_none_for_empty_config() {
        let config = Config::default();

        assert_eq!(config.list_sites(), "(none)");
    }

    #[test]
    fn set_site_stores_site_and_sets_first_site_as_default() {
        let mut config = Config::default();

        config.set_site("prod", "token", "acct", false);

        let found = config.sites.get("prod").unwrap();
        assert_eq!(found.api_token, "token");
        assert_eq!(found.account_id, "acct");
        assert_eq!(config.default_site.as_deref(), Some("prod"));
    }

    #[test]
    fn set_site_respects_existing_default_unless_requested() {
        let mut config = Config::default();
        config.set_site("prod", "token", "acct", false);

        config.set_site("stage", "stage-token", "stage-acct", false);

        assert_eq!(config.default_site.as_deref(), Some("prod"));

        config.set_site("stage", "stage-token", "stage-acct", true);

        assert_eq!(config.default_site.as_deref(), Some("stage"));
    }

    #[test]
    fn remove_site_removes_existing_site_and_moves_default() {
        let mut config = Config::default();
        config.set_site("prod", "token", "acct", false);
        config.set_site("stage", "stage-token", "stage-acct", false);

        assert!(config.remove_site("prod"));

        assert!(!config.sites.contains_key("prod"));
        assert_eq!(config.default_site.as_deref(), Some("stage"));
    }

    #[test]
    fn remove_site_reports_missing_site_without_changing_default() {
        let mut config = Config::default();
        config.set_site("prod", "token", "acct", false);

        assert!(!config.remove_site("stage"));

        assert_eq!(config.default_site.as_deref(), Some("prod"));
    }
}
