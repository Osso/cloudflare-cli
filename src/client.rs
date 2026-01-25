use anyhow::{Context, Result};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::de::DeserializeOwned;

use crate::config::SiteConfig;

const BASE_URL: &str = "https://api.cloudflare.com/client/v4";

pub struct Client {
    http: reqwest::Client,
    account_id: String,
}

impl Client {
    pub fn new(site_config: &SiteConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();

        // Use Global API Key auth if email is set, otherwise use Bearer token
        if let Some(email) = &site_config.email {
            headers.insert(
                "X-Auth-Key",
                HeaderValue::from_str(&site_config.api_token)
                    .context("Invalid API key")?,
            );
            headers.insert(
                "X-Auth-Email",
                HeaderValue::from_str(email)
                    .context("Invalid email")?,
            );
        } else {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", site_config.api_token))
                    .context("Invalid API token")?,
            );
        }
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self {
            http,
            account_id: site_config.account_id.clone(),
        })
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let body = self.get_raw(path).await?;
        serde_json::from_str(&body).with_context(|| format!("Failed to parse response: {}", body))
    }

    pub async fn get_raw(&self, path: &str) -> Result<String> {
        let url = format!("{}{}", BASE_URL, path);
        let response = self.http.get(&url).send().await?;
        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            anyhow::bail!("API error ({}): {}", status, body);
        }

        Ok(body)
    }

    pub async fn post<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = format!("{}{}", BASE_URL, path);
        let response = self.http.post(&url).json(body).send().await?;
        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            anyhow::bail!("API error ({}): {}", status, body);
        }

        serde_json::from_str(&body).with_context(|| format!("Failed to parse response: {}", body))
    }

    pub async fn delete(&self, path: &str) -> Result<()> {
        let url = format!("{}{}", BASE_URL, path);
        let response = self.http.delete(&url).send().await?;
        let status = response.status();

        if !status.is_success() {
            let body = response.text().await?;
            anyhow::bail!("API error ({}): {}", status, body);
        }

        Ok(())
    }

    pub async fn put<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = format!("{}{}", BASE_URL, path);
        let response = self.http.put(&url).json(body).send().await?;
        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            anyhow::bail!("API error ({}): {}", status, body);
        }

        serde_json::from_str(&body).with_context(|| format!("Failed to parse response: {}", body))
    }

    pub async fn graphql<T: DeserializeOwned, B: serde::Serialize>(&self, body: &B) -> Result<T> {
        let url = format!("{}/graphql", BASE_URL);
        let response = self.http.post(&url).json(body).send().await?;
        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            anyhow::bail!("GraphQL error ({}): {}", status, body);
        }

        serde_json::from_str(&body).with_context(|| format!("Failed to parse response: {}", body))
    }

    pub fn account_id(&self) -> &str {
        &self.account_id
    }
}
