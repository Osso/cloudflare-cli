use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::commands::tunnels::ApiResponse;

#[derive(Debug, Deserialize)]
pub struct ServiceToken {
    pub id: String,
    pub name: String,
    pub client_id: String,
    #[serde(default)]
    pub client_secret: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub expires_at: String,
    #[serde(default)]
    pub duration: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateTokenRequest {
    pub name: String,
    pub duration: String,
}

pub async fn list(client: &Client) -> Result<()> {
    let path = format!("/accounts/{}/access/service_tokens", client.account_id());
    let response: ApiResponse<Vec<ServiceToken>> = client.get(&path).await?;

    if response.result.is_empty() {
        println!("No service tokens found");
        return Ok(());
    }

    for token in response.result {
        println!("{}", token.name);
        println!("  ID:        {}", token.id);
        println!("  Client ID: {}", token.client_id);
        println!("  Expires:   {}", token.expires_at);
        println!();
    }

    Ok(())
}

pub async fn create(client: &Client, name: &str, duration: &str) -> Result<()> {
    let path = format!("/accounts/{}/access/service_tokens", client.account_id());

    let request = CreateTokenRequest {
        name: name.to_string(),
        duration: duration.to_string(),
    };

    let response: ApiResponse<ServiceToken> = client.post(&path, &request).await?;
    let token = response.result;

    println!("Created service token: {}", token.name);
    println!();
    println!("  ID:            {}", token.id);
    println!("  Client ID:     {}", token.client_id);
    if let Some(secret) = &token.client_secret {
        println!();
        println!("  ============================================");
        println!("  CLIENT SECRET (save this - shown only once!):");
        println!("  {}", secret);
        println!("  ============================================");
    }
    println!();
    println!("  Expires: {}", token.expires_at);

    Ok(())
}

pub async fn delete(client: &Client, token_id: &str) -> Result<()> {
    let path = format!(
        "/accounts/{}/access/service_tokens/{}",
        client.account_id(),
        token_id
    );

    client.delete(&path).await?;
    println!("Deleted service token: {}", token_id);

    Ok(())
}
