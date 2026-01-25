use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::commands::tunnels::ApiResponse;

#[derive(Debug, Deserialize)]
pub struct Application {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub app_type: String,
    #[serde(default)]
    pub domain: String,
    #[serde(default)]
    pub self_hosted_domains: Vec<String>,
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
        if !app.self_hosted_domains.is_empty() {
            for domain in &app.self_hosted_domains {
                println!("  - {}", domain);
            }
        } else if !app.domain.is_empty() {
            println!("  - {}", app.domain);
        }
        println!("  ID: {}", app.id);
        println!();
    }

    Ok(())
}

pub async fn show(client: &Client, app_id: &str, json: bool) -> Result<()> {
    let path = format!("/accounts/{}/access/apps/{}", client.account_id(), app_id);

    if json {
        let raw = client.get_raw(&path).await?;
        let parsed: serde_json::Value = serde_json::from_str(&raw)?;
        println!("{}", serde_json::to_string_pretty(&parsed.get("result").unwrap_or(&parsed))?);
        return Ok(());
    }

    let response: ApiResponse<Application> = client.get(&path).await?;

    let app = response.result;
    println!("Name: {}", app.name);
    println!("Type: {}", app.app_type);
    println!("Domain: {}", app.domain);
    if !app.self_hosted_domains.is_empty() {
        println!("Hostnames:");
        for domain in &app.self_hosted_domains {
            println!("  - {}", domain);
        }
    }
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

#[derive(Debug, Serialize)]
pub struct CreateAppRequest {
    pub name: String,
    pub domain: String,
    #[serde(rename = "type")]
    pub app_type: String,
    pub session_duration: String,
    pub policies: Vec<Policy>,
}

#[derive(Debug, Serialize)]
pub struct Policy {
    pub name: String,
    pub decision: String,
    pub include: Vec<PolicyRule>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum PolicyRule {
    ServiceToken { service_token: ServiceTokenRef },
}

#[derive(Debug, Serialize)]
pub struct ServiceTokenRef {
    pub token_id: String,
}

pub async fn create(
    client: &Client,
    name: &str,
    domain: &str,
    service_token_id: &str,
) -> Result<()> {
    let path = format!("/accounts/{}/access/apps", client.account_id());

    let request = CreateAppRequest {
        name: name.to_string(),
        domain: domain.to_string(),
        app_type: "self_hosted".to_string(),
        session_duration: "24h".to_string(),
        policies: vec![Policy {
            name: "Service Token Only".to_string(),
            decision: "non_identity".to_string(),
            include: vec![PolicyRule::ServiceToken {
                service_token: ServiceTokenRef {
                    token_id: service_token_id.to_string(),
                },
            }],
        }],
    };

    let response: ApiResponse<Application> = client.post(&path, &request).await?;
    let app = response.result;

    println!("Created Access application: {}", app.name);
    println!("  ID:     {}", app.id);
    println!("  Domain: {}", app.domain);
    println!("  Type:   {}", app.app_type);
    println!("  AUD:    {}", app.aud);

    Ok(())
}

pub async fn delete(client: &Client, app_id: &str) -> Result<()> {
    let path = format!(
        "/accounts/{}/access/apps/{}",
        client.account_id(),
        app_id
    );

    client.delete(&path).await?;
    println!("Deleted application: {}", app_id);

    Ok(())
}

#[derive(Debug, Serialize)]
struct UpdateAppRequest {
    name: String,
    #[serde(rename = "type")]
    app_type: String,
    self_hosted_domains: Vec<String>,
}

pub async fn remove_hostname(client: &Client, app_id: &str, hostname: &str) -> Result<()> {
    let path = format!("/accounts/{}/access/apps/{}", client.account_id(), app_id);

    // Get current app
    let response: ApiResponse<Application> = client.get(&path).await?;
    let app = response.result;

    // Remove hostname
    let initial_len = app.self_hosted_domains.len();
    let new_domains: Vec<String> = app
        .self_hosted_domains
        .clone()
        .into_iter()
        .filter(|h| h != hostname)
        .collect();

    if new_domains.len() == initial_len {
        anyhow::bail!("Hostname '{}' not found in application '{}'", hostname, app.name);
    }

    if new_domains.is_empty() {
        anyhow::bail!(
            "Cannot remove '{}': it's the last hostname. Delete the application instead.",
            hostname
        );
    }

    // Update app
    let request = UpdateAppRequest {
        name: app.name.clone(),
        app_type: app.app_type.clone(),
        self_hosted_domains: new_domains,
    };
    let _response: ApiResponse<Application> = client.put(&path, &request).await?;

    println!("Removed {} from {}", hostname, app.name);
    Ok(())
}
