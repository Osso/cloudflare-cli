use anyhow::{Result, bail};
use serde::Deserialize;

use crate::client::Client;
use crate::commands::cache::Zone;
use crate::commands::tunnels::ApiResponse;

#[derive(Debug, Deserialize)]
pub struct WaitingRoom {
    pub id: String,
    pub name: String,
    pub host: String,
    pub path: String,
    #[serde(default)]
    pub suspended: bool,
    #[serde(default)]
    pub total_active_users: u32,
    #[serde(default)]
    pub new_users_per_minute: u32,
    #[serde(default)]
    pub session_duration: u32,
    #[serde(default)]
    pub queue_all: bool,
    #[serde(default)]
    pub disable_session_renewal: bool,
    #[serde(default)]
    pub json_response_enabled: bool,
    #[serde(default)]
    pub queueing_method: String,
    #[serde(default)]
    pub cookie_suffix: String,
    #[serde(default)]
    pub description: String,
    pub created_on: String,
    pub modified_on: String,
}

async fn find_zone_id(client: &Client, zone: &str) -> Result<String> {
    // If it looks like a zone ID (32 hex chars), use it directly
    if zone.len() == 32 && zone.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(zone.to_string());
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

    Ok(response.result[0].id.clone())
}

pub async fn list(client: &Client, zone: &str) -> Result<()> {
    let zone_id = find_zone_id(client, zone).await?;
    let path = format!("/zones/{}/waiting_rooms", zone_id);
    let response: ApiResponse<Vec<WaitingRoom>> = client.get(&path).await?;

    if response.result.is_empty() {
        println!("No waiting rooms found");
        return Ok(());
    }

    for room in response.result {
        let status = if room.suspended {
            "suspended"
        } else {
            "active"
        };
        let status_icon = if room.suspended { "○" } else { "●" };
        println!("{} {} ({})", status_icon, room.name, status);
        println!("  Host: {}{}", room.host, room.path);
        println!("  ID: {}", room.id);
        if room.total_active_users > 0 {
            println!(
                "  Queue: {} max users, {} new/min",
                room.total_active_users, room.new_users_per_minute
            );
        }
        println!();
    }

    Ok(())
}

pub async fn show(client: &Client, zone: &str, id: &str) -> Result<()> {
    let zone_id = find_zone_id(client, zone).await?;
    let path = format!("/zones/{}/waiting_rooms/{}", zone_id, id);
    let response: ApiResponse<WaitingRoom> = client.get(&path).await?;

    let room = response.result;
    let status = if room.suspended {
        "suspended"
    } else {
        "active"
    };
    let status_icon = if room.suspended { "○" } else { "●" };

    println!("{} {} ({})", status_icon, room.name, status);
    println!("  ID: {}", room.id);
    if !room.description.is_empty() {
        println!("  Description: {}", room.description);
    }
    println!();
    println!("Target:");
    println!("  Host: {}", room.host);
    println!("  Path: {}", room.path);
    println!();
    println!("Queue Settings:");
    println!("  Total active users: {}", room.total_active_users);
    println!("  New users per minute: {}", room.new_users_per_minute);
    println!("  Session duration: {} minutes", room.session_duration);
    println!("  Queue all: {}", room.queue_all);
    println!("  Queueing method: {}", room.queueing_method);
    println!();
    println!("Options:");
    println!(
        "  Disable session renewal: {}",
        room.disable_session_renewal
    );
    println!("  JSON response enabled: {}", room.json_response_enabled);
    if !room.cookie_suffix.is_empty() {
        println!("  Cookie suffix: {}", room.cookie_suffix);
    }
    println!();
    println!("Timestamps:");
    println!("  Created: {}", room.created_on);
    println!("  Modified: {}", room.modified_on);

    Ok(())
}
