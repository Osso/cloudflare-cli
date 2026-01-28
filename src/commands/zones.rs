use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::commands::tunnels::ApiResponse;

#[derive(Debug, Deserialize)]
pub struct Zone {
    pub id: String,
    pub name: String,
    pub status: String,
    #[serde(default)]
    pub paused: bool,
    #[serde(default)]
    pub name_servers: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CreateZoneRequest {
    name: String,
    account: AccountRef,
    #[serde(rename = "type")]
    zone_type: String,
}

#[derive(Debug, Serialize)]
struct AccountRef {
    id: String,
}

async fn find_zone(client: &Client, zone: &str) -> Result<Zone> {
    // If it looks like a zone ID (32 hex chars), fetch by ID
    if zone.len() == 32 && zone.chars().all(|c| c.is_ascii_hexdigit()) {
        let path = format!("/zones/{}", zone);
        let response: ApiResponse<Zone> = client.get(&path).await?;
        return Ok(response.result);
    }

    // Otherwise, look up by domain name
    let path = format!("/zones?name={}&account.id={}", zone, client.account_id());
    let response: ApiResponse<Vec<Zone>> = client.get(&path).await?;

    if response.result.is_empty() {
        bail!(
            "Zone '{}' not found. Use 'cloudflare zones list' to list available zones.",
            zone
        );
    }

    Ok(response.result.into_iter().next().unwrap())
}

pub async fn list(client: &Client) -> Result<()> {
    let path = format!("/zones?account.id={}", client.account_id());
    let response: ApiResponse<Vec<Zone>> = client.get(&path).await?;

    if response.result.is_empty() {
        println!("No zones found");
        return Ok(());
    }

    for zone in response.result {
        let status_icon = match zone.status.as_str() {
            "active" => "●",
            "pending" => "○",
            _ => "◌",
        };
        let paused = if zone.paused { " (paused)" } else { "" };
        println!("{} {}{}", status_icon, zone.name, paused);
        println!("  ID: {}", zone.id);
    }

    Ok(())
}

pub async fn add(client: &Client, domain: &str) -> Result<()> {
    let request = CreateZoneRequest {
        name: domain.to_string(),
        account: AccountRef {
            id: client.account_id().to_string(),
        },
        zone_type: "full".to_string(),
    };

    let response: ApiResponse<Zone> = client.post("/zones", &request).await?;

    println!("Created zone: {}", response.result.name);
    println!("  ID: {}", response.result.id);
    println!("  Status: {}", response.result.status);

    if !response.result.name_servers.is_empty() {
        println!("\nUpdate your registrar to use these nameservers:");
        for ns in &response.result.name_servers {
            println!("  {}", ns);
        }
    }

    Ok(())
}

pub async fn info(client: &Client, zone: &str) -> Result<()> {
    let zone = find_zone(client, zone).await?;

    let status_icon = match zone.status.as_str() {
        "active" => "●",
        "pending" => "○",
        _ => "◌",
    };
    let paused = if zone.paused { " (paused)" } else { "" };

    println!("{} {}{}", status_icon, zone.name, paused);
    println!("  ID: {}", zone.id);
    println!("  Status: {}", zone.status);

    if !zone.name_servers.is_empty() {
        println!("  Nameservers:");
        for ns in &zone.name_servers {
            println!("    {}", ns);
        }
    }

    Ok(())
}

pub async fn delete(client: &Client, zone: &str) -> Result<()> {
    let zone_data = find_zone(client, zone).await?;
    let path = format!("/zones/{}", zone_data.id);
    client.delete(&path).await?;

    println!("Deleted zone: {}", zone_data.name);

    Ok(())
}
