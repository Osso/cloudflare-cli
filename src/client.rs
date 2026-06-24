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
                HeaderValue::from_str(&site_config.api_token).context("Invalid API key")?,
            );
            headers.insert(
                "X-Auth-Email",
                HeaderValue::from_str(email).context("Invalid email")?,
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

    #[cfg_attr(coverage_nightly, coverage(off))]
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let body = self.get_raw(path).await?;
        serde_json::from_str(&body).with_context(|| format!("Failed to parse response: {}", body))
    }

    #[cfg_attr(coverage_nightly, coverage(off))]
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

    #[cfg_attr(coverage_nightly, coverage(off))]
    async fn send_json<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<T> {
        let url = format!("{}{}", BASE_URL, path);
        let mut req = self.http.request(method, &url);
        if let Some(b) = body {
            req = req.json(b);
        }
        let response = req.send().await?;
        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            anyhow::bail!("API error ({}): {}", status, text);
        }

        serde_json::from_str(&text).with_context(|| format!("Failed to parse response: {}", text))
    }

    #[cfg_attr(coverage_nightly, coverage(off))]
    pub async fn post<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        self.send_json(reqwest::Method::POST, path, Some(body))
            .await
    }

    #[cfg_attr(coverage_nightly, coverage(off))]
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

    #[cfg_attr(coverage_nightly, coverage(off))]
    pub async fn put<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        self.send_json(reqwest::Method::PUT, path, Some(body)).await
    }

    #[cfg_attr(coverage_nightly, coverage(off))]
    pub async fn graphql<T: DeserializeOwned, B: serde::Serialize>(&self, body: &B) -> Result<T> {
        self.send_json::<T, B>(reqwest::Method::POST, "/graphql", Some(body))
            .await
    }

    pub fn account_id(&self) -> &str {
        &self.account_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site_config(api_token: &str, email: Option<&str>) -> SiteConfig {
        SiteConfig {
            api_token: api_token.to_string(),
            account_id: "account-123".to_string(),
            email: email.map(str::to_string),
        }
    }

    fn new_client_error(api_token: &str, email: Option<&str>) -> String {
        match Client::new(&site_config(api_token, email)) {
            Ok(_) => panic!("expected invalid client config to fail"),
            Err(error) => error.to_string(),
        }
    }

    #[test]
    fn new_builds_bearer_token_client() {
        let client = Client::new(&site_config("token", None)).unwrap();

        assert_eq!(client.account_id(), "account-123");
    }

    #[test]
    fn new_builds_global_api_key_client() {
        let client = Client::new(&site_config("global-key", Some("user@example.com"))).unwrap();

        assert_eq!(client.account_id(), "account-123");
    }

    #[test]
    fn new_rejects_invalid_bearer_token_header() {
        let error = new_client_error("bad\nvalue", None);

        assert!(error.contains("Invalid API token"));
    }

    #[test]
    fn new_rejects_invalid_global_api_key_header() {
        let error = new_client_error("bad\nvalue", Some("user@example.com"));

        assert!(error.contains("Invalid API key"));
    }

    #[test]
    fn new_rejects_invalid_email_header() {
        let error = new_client_error("global-key", Some("bad\nemail"));

        assert!(error.contains("Invalid email"));
    }
}
