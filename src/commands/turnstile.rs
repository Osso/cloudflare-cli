use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::commands::tunnels::ApiResponse;

#[derive(Debug, Deserialize)]
pub struct Widget {
    pub sitekey: String,
    #[serde(default)]
    pub secret: Option<String>,
    pub name: String,
    pub domains: Vec<String>,
    pub mode: String,
    pub created_on: String,
    pub modified_on: String,
    #[serde(default)]
    pub bot_fight_mode: bool,
    #[serde(default)]
    pub clearance_level: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateWidgetRequest {
    pub name: String,
    pub domains: Vec<String>,
    pub mode: String,
}

#[derive(Debug, Serialize)]
pub struct RotateSecretRequest {
    pub invalidate_immediately: bool,
}

pub async fn list(client: &Client) -> Result<()> {
    let path = format!("/accounts/{}/challenges/widgets", client.account_id());
    let response: ApiResponse<Vec<Widget>> = client.get(&path).await?;

    if response.result.is_empty() {
        println!("No Turnstile widgets found");
        return Ok(());
    }

    for widget in response.result {
        let mode_icon = match widget.mode.as_str() {
            "managed" => "◐",
            "invisible" => "○",
            "non-interactive" => "●",
            _ => "◌",
        };
        println!("{} {} ({})", mode_icon, widget.name, widget.sitekey);
        println!("  Domains: {}", widget.domains.join(", "));
    }

    Ok(())
}

pub async fn show(client: &Client, sitekey: &str, json: bool) -> Result<()> {
    let path = format!(
        "/accounts/{}/challenges/widgets/{}",
        client.account_id(),
        sitekey
    );

    if json {
        let raw = client.get_raw(&path).await?;
        println!("{}", raw);
        return Ok(());
    }

    let response: ApiResponse<Widget> = client.get(&path).await?;
    let widget = response.result;

    println!("Name:       {}", widget.name);
    println!("Site Key:   {}", widget.sitekey);
    if let Some(secret) = &widget.secret {
        println!("Secret:     {}", secret);
    }
    println!("Mode:       {}", widget.mode);
    println!("Domains:    {}", widget.domains.join(", "));
    if let Some(level) = widget.clearance_level {
        println!("Clearance:  {}", level);
    }
    println!("Created:    {}", widget.created_on);
    println!("Modified:   {}", widget.modified_on);

    Ok(())
}

pub async fn create(client: &Client, name: &str, domains: Vec<String>, mode: &str) -> Result<()> {
    let path = format!("/accounts/{}/challenges/widgets", client.account_id());

    let request = CreateWidgetRequest {
        name: name.to_string(),
        domains,
        mode: mode.to_string(),
    };

    let response: ApiResponse<Widget> = client.post(&path, &request).await?;
    let widget = response.result;

    println!("Created Turnstile widget:");
    println!("  Name:     {}", widget.name);
    println!("  Site Key: {}", widget.sitekey);
    if let Some(secret) = &widget.secret {
        println!("  Secret:   {}", secret);
    }
    println!("  Mode:     {}", widget.mode);
    println!("  Domains:  {}", widget.domains.join(", "));

    Ok(())
}

pub async fn delete(client: &Client, sitekey: &str) -> Result<()> {
    let path = format!(
        "/accounts/{}/challenges/widgets/{}",
        client.account_id(),
        sitekey
    );

    client.delete(&path).await?;
    println!("Deleted widget: {}", sitekey);

    Ok(())
}

pub async fn rotate_secret(
    client: &Client,
    sitekey: &str,
    invalidate_immediately: bool,
) -> Result<()> {
    let path = format!(
        "/accounts/{}/challenges/widgets/{}/rotate_secret",
        client.account_id(),
        sitekey
    );

    let request = RotateSecretRequest {
        invalidate_immediately,
    };

    let response: ApiResponse<Widget> = client.post(&path, &request).await?;
    let widget = response.result;

    println!("Rotated secret for widget: {}", widget.name);
    if let Some(secret) = &widget.secret {
        println!("  New Secret: {}", secret);
    }
    if !invalidate_immediately {
        println!("  Note: Old secret remains valid for 2 hours");
    }

    Ok(())
}
