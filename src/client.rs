use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::de::DeserializeOwned;

use crate::config::Config;

const BASE_URL: &str = "https://api.cloudflare.com/client/v4";

pub struct Client {
    http: reqwest::Client,
    account_id: String,
}

impl Client {
    pub fn new(config: &Config) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", config.api_token))
                .context("Invalid API token")?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self {
            http,
            account_id: config.account_id.clone(),
        })
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", BASE_URL, path);
        let response = self.http.get(&url).send().await?;
        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            anyhow::bail!("API error ({}): {}", status, body);
        }

        serde_json::from_str(&body).with_context(|| format!("Failed to parse response: {}", body))
    }

    pub fn account_id(&self) -> &str {
        &self.account_id
    }
}
