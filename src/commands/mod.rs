pub mod analytics;
pub mod applications;
pub mod cache;
pub mod dns;
pub mod firewall;
pub mod gateway;
pub mod rate_limiting;
pub mod rum;
pub mod service_tokens;
pub mod tunnels;
pub mod turnstile;
pub mod waiting_room;
pub mod zones;

use anyhow::{Result, bail};
use serde::Deserialize;

use crate::client::Client;

#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub result: T,
    pub success: bool,
}

#[derive(Debug, Deserialize)]
pub struct Zone {
    pub id: String,
    pub name: String,
    pub status: String,
    #[serde(default)]
    pub paused: bool,
}

pub async fn find_zone_id(client: &Client, zone: &str) -> Result<String> {
    if zone.len() == 32 && zone.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(zone.to_string());
    }

    let path = format!("/zones?name={}&account.id={}", zone, client.account_id());
    let response: ApiResponse<Vec<Zone>> = client.get(&path).await?;

    if response.result.is_empty() {
        bail!(
            "Zone '{}' not found. Use 'cloudflare zones list' to list available zones.",
            zone
        );
    }

    Ok(response.result[0].id.clone())
}
