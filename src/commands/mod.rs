#[cfg_attr(coverage_nightly, coverage(off))]
pub mod abuse;
#[cfg_attr(coverage_nightly, coverage(off))]
pub mod analytics;
#[cfg_attr(coverage_nightly, coverage(off))]
pub mod applications;
#[cfg_attr(coverage_nightly, coverage(off))]
pub mod cache;
#[cfg_attr(coverage_nightly, coverage(off))]
pub mod dns;
#[cfg_attr(coverage_nightly, coverage(off))]
pub mod firewall;
#[cfg_attr(coverage_nightly, coverage(off))]
pub mod gateway;
#[cfg_attr(coverage_nightly, coverage(off))]
pub mod rate_limiting;
#[cfg_attr(coverage_nightly, coverage(off))]
pub mod rum;
#[cfg_attr(coverage_nightly, coverage(off))]
pub mod service_tokens;
#[cfg_attr(coverage_nightly, coverage(off))]
pub mod tunnels;
#[cfg_attr(coverage_nightly, coverage(off))]
pub mod turnstile;
#[cfg_attr(coverage_nightly, coverage(off))]
pub mod waiting_room;
#[cfg_attr(coverage_nightly, coverage(off))]
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

#[cfg_attr(coverage_nightly, coverage(off))]
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
