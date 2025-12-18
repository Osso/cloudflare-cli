use anyhow::Result;
use serde::Deserialize;

use crate::client::Client;
use crate::commands::tunnels::ApiResponse;

#[derive(Debug, Deserialize)]
pub struct Application {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub app_type: String,
    pub domain: String,
    #[serde(default)]
    pub aud: String,
    #[serde(default)]
    pub allowed_idps: Vec<String>,
    #[serde(default)]
    pub auto_redirect_to_identity: bool,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn list(client: &Client) -> Result<()> {
    let path = format!("/accounts/{}/access/apps", client.account_id());
    let response: ApiResponse<Vec<Application>> = client.get(&path).await?;

    if response.result.is_empty() {
        println!("No applications found");
        return Ok(());
    }

    for app in response.result {
        println!("{} [{}]", app.name, app.app_type);
        println!("  Domain: {}", app.domain);
        println!("  ID: {}", app.id);
        println!();
    }

    Ok(())
}

pub async fn show(client: &Client, app_id: &str) -> Result<()> {
    let path = format!("/accounts/{}/access/apps/{}", client.account_id(), app_id);
    let response: ApiResponse<Application> = client.get(&path).await?;

    let app = response.result;
    println!("Name: {}", app.name);
    println!("Type: {}", app.app_type);
    println!("Domain: {}", app.domain);
    println!("ID: {}", app.id);
    println!("Audience (AUD): {}", app.aud);
    println!("Auto-redirect to IdP: {}", app.auto_redirect_to_identity);
    println!("Created: {}", app.created_at);
    println!("Updated: {}", app.updated_at);

    if !app.allowed_idps.is_empty() {
        println!("Allowed IdPs: {}", app.allowed_idps.join(", "));
    }

    Ok(())
}
