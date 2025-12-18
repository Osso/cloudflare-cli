use anyhow::Result;
use serde::Deserialize;

use crate::client::Client;

#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub result: T,
    pub success: bool,
}

#[derive(Debug, Deserialize)]
pub struct Tunnel {
    pub id: String,
    pub name: String,
    pub status: String,
    pub created_at: String,
    #[serde(default)]
    pub connections: Vec<TunnelConnection>,
}

#[derive(Debug, Deserialize)]
pub struct TunnelConnection {
    pub colo_name: String,
    pub id: String,
    pub is_pending_reconnect: bool,
    pub client_id: String,
    pub client_version: String,
}

#[derive(Debug, Deserialize)]
pub struct TunnelConfig {
    pub config: TunnelConfigInner,
}

#[derive(Debug, Deserialize)]
pub struct TunnelConfigInner {
    #[serde(default)]
    pub ingress: Vec<IngressRule>,
}

#[derive(Debug, Deserialize)]
pub struct IngressRule {
    #[serde(default)]
    pub hostname: Option<String>,
    pub service: String,
    #[serde(default)]
    pub path: Option<String>,
}

pub async fn list(client: &Client) -> Result<()> {
    let path = format!("/accounts/{}/cfd_tunnel", client.account_id());
    let response: ApiResponse<Vec<Tunnel>> = client.get(&path).await?;

    if response.result.is_empty() {
        println!("No tunnels found");
        return Ok(());
    }

    for tunnel in response.result {
        let status_icon = match tunnel.status.as_str() {
            "healthy" => "●",
            "inactive" => "○",
            _ => "◌",
        };
        println!("{} {} ({})", status_icon, tunnel.name, tunnel.id);

        for conn in &tunnel.connections {
            println!("  └─ {} ({})", conn.colo_name, conn.client_version);
        }
    }

    Ok(())
}

pub async fn domains(client: &Client, tunnel_id: &str) -> Result<()> {
    let path = format!(
        "/accounts/{}/cfd_tunnel/{}/configurations",
        client.account_id(),
        tunnel_id
    );
    let response: ApiResponse<TunnelConfig> = client.get(&path).await?;

    if response.result.config.ingress.is_empty() {
        println!("No ingress rules configured");
        return Ok(());
    }

    for rule in response.result.config.ingress {
        if let Some(hostname) = rule.hostname {
            let path = rule.path.unwrap_or_default();
            println!("{}{} → {}", hostname, path, rule.service);
        } else {
            println!("(catch-all) → {}", rule.service);
        }
    }

    Ok(())
}
