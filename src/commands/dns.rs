use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::client::Client;
use super::{ApiResponse, find_zone_id};

#[derive(Debug, Deserialize)]
pub struct DnsRecord {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub record_type: String,
    pub content: String,
    pub proxied: bool,
    pub ttl: u32,
}

#[derive(Debug, Serialize)]
struct CreateDnsRecord {
    #[serde(rename = "type")]
    record_type: String,
    name: String,
    content: String,
    proxied: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    ttl: Option<u32>,
}

pub async fn list(client: &Client, zone: &str) -> Result<()> {
    let zone_id = find_zone_id(client, zone).await?;
    let path = format!("/zones/{}/dns_records", zone_id);
    let response: ApiResponse<Vec<DnsRecord>> = client.get(&path).await?;

    if response.result.is_empty() {
        println!("No DNS records found");
        return Ok(());
    }

    for record in response.result {
        let proxied = if record.proxied { " [proxied]" } else { "" };
        println!(
            "{} {} → {}{}",
            record.record_type, record.name, record.content, proxied
        );
    }

    Ok(())
}

pub async fn create(
    client: &Client,
    zone: &str,
    record_type: &str,
    name: &str,
    content: &str,
    proxied: bool,
) -> Result<()> {
    let zone_id = find_zone_id(client, zone).await?;

    let request = CreateDnsRecord {
        record_type: record_type.to_uppercase(),
        name: name.to_string(),
        content: content.to_string(),
        proxied,
        ttl: if proxied { None } else { Some(1) }, // 1 = auto
    };

    let path = format!("/zones/{}/dns_records", zone_id);
    let response: ApiResponse<DnsRecord> = client.post(&path, &request).await?;

    let proxied_str = if response.result.proxied {
        " [proxied]"
    } else {
        ""
    };
    println!(
        "Created {} {} → {}{}",
        response.result.record_type, response.result.name, response.result.content, proxied_str
    );

    Ok(())
}

pub async fn delete(client: &Client, zone: &str, name: &str) -> Result<()> {
    let zone_id = find_zone_id(client, zone).await?;

    // Find the record by name
    let path = format!("/zones/{}/dns_records?name={}", zone_id, name);
    let response: ApiResponse<Vec<DnsRecord>> = client.get(&path).await?;

    if response.result.is_empty() {
        bail!("DNS record '{}' not found", name);
    }

    let record = &response.result[0];
    let path = format!("/zones/{}/dns_records/{}", zone_id, record.id);
    client.delete(&path).await?;

    println!(
        "Deleted {} {} → {}",
        record.record_type, record.name, record.content
    );

    Ok(())
}
