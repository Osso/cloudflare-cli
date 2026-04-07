use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use super::ApiResponse;
use crate::client::Client;

#[derive(Debug, Deserialize)]
pub struct Site {
    pub site_tag: String,
    pub site_token: String,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub auto_install: bool,
    pub created: Option<String>,
    pub ruleset: Option<Ruleset>,
}

#[derive(Debug, Deserialize)]
pub struct Ruleset {
    pub id: String,
    pub zone_tag: String,
    pub zone_name: String,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Serialize)]
struct UpdateSiteRequest {
    auto_install: bool,
    enabled: bool,
}

/// Find a site by zone name, host, or site_tag
async fn find_site(client: &Client, identifier: &str) -> Result<Site> {
    let sites = list_sites(client).await?;

    // Try to match by zone_name first, then host, then site_tag
    for site in sites {
        let zone_name = site.ruleset.as_ref().map(|r| r.zone_name.as_str());
        if zone_name == Some(identifier)
            || site.host.as_deref() == Some(identifier)
            || site.site_tag == identifier
        {
            return Ok(site);
        }
    }

    bail!(
        "Web Analytics site '{}' not found. Use 'cloudflare rum list' to list available sites.",
        identifier
    );
}

async fn list_sites(client: &Client) -> Result<Vec<Site>> {
    let path = format!("/accounts/{}/rum/site_info/list", client.account_id());
    let response: ApiResponse<Vec<Site>> = client.get(&path).await?;
    Ok(response.result)
}

fn display_name(site: &Site) -> &str {
    site.ruleset
        .as_ref()
        .map(|r| r.zone_name.as_str())
        .or(site.host.as_deref())
        .unwrap_or("(no host)")
}

fn is_enabled(site: &Site) -> bool {
    site.ruleset.as_ref().is_some_and(|r| r.enabled)
}

pub async fn list(client: &Client) -> Result<()> {
    let sites = list_sites(client).await?;

    if sites.is_empty() {
        println!("No Web Analytics sites found");
        return Ok(());
    }

    for site in sites {
        let enabled = is_enabled(&site);
        let icon = if enabled { "●" } else { "○" };
        let name = display_name(&site);
        println!("{} {}", icon, name);
        println!("  ID: {}", site.site_tag);
        println!(
            "  Beacon injection: {}",
            if enabled { "enabled" } else { "disabled" }
        );
    }

    Ok(())
}

pub async fn info(client: &Client, site: &str) -> Result<()> {
    let site = find_site(client, site).await?;

    let enabled = is_enabled(&site);
    let icon = if enabled { "●" } else { "○" };
    let name = display_name(&site);

    println!("{} {}", icon, name);
    println!("  ID: {}", site.site_tag);
    println!("  Token: {}", site.site_token);
    println!(
        "  Beacon injection: {}",
        if enabled { "enabled" } else { "disabled" }
    );
    if let Some(created) = &site.created {
        println!("  Created: {}", created);
    }

    Ok(())
}

pub async fn disable(client: &Client, site: &str) -> Result<()> {
    let site_data = find_site(client, site).await?;
    let name = display_name(&site_data);

    if !is_enabled(&site_data) {
        println!("Beacon injection already disabled for {}", name);
        return Ok(());
    }

    let path = format!(
        "/accounts/{}/rum/site_info/{}",
        client.account_id(),
        site_data.site_tag
    );
    let request = UpdateSiteRequest {
        auto_install: true,
        enabled: false,
    };

    let _response: ApiResponse<Site> = client.put(&path, &request).await?;
    println!("Disabled beacon injection for {}", name);
    println!("  beacon.min.js will no longer be injected automatically");

    Ok(())
}

pub async fn enable(client: &Client, site: &str) -> Result<()> {
    let site_data = find_site(client, site).await?;
    let name = display_name(&site_data);

    if is_enabled(&site_data) {
        println!("Beacon injection already enabled for {}", name);
        return Ok(());
    }

    let path = format!(
        "/accounts/{}/rum/site_info/{}",
        client.account_id(),
        site_data.site_tag
    );
    let request = UpdateSiteRequest {
        auto_install: true,
        enabled: true,
    };

    let _response: ApiResponse<Site> = client.put(&path, &request).await?;
    println!("Enabled beacon injection for {}", name);
    println!("  beacon.min.js will be injected automatically");

    Ok(())
}

pub async fn delete(client: &Client, site: &str) -> Result<()> {
    let site_data = find_site(client, site).await?;
    let name = display_name(&site_data).to_string();

    let path = format!(
        "/accounts/{}/rum/site_info/{}",
        client.account_id(),
        site_data.site_tag
    );
    client.delete(&path).await?;

    println!("Deleted Web Analytics site: {}", name);

    Ok(())
}
