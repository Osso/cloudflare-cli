use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use super::ApiResponse;
use crate::client::Client;

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

#[derive(Debug, Deserialize, Serialize)]
pub struct TunnelConfig {
    pub config: Option<TunnelConfigInner>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TunnelConfigInner {
    #[serde(default)]
    pub ingress: Vec<IngressRule>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IngressRule {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    pub service: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(
        default,
        rename = "originRequest",
        skip_serializing_if = "Option::is_none"
    )]
    pub origin_request: Option<OriginRequest>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OriginRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access: Option<AccessConfig>,
    #[serde(
        default,
        rename = "noTLSVerify",
        skip_serializing_if = "Option::is_none"
    )]
    pub no_tls_verify: Option<bool>,
    #[serde(
        default,
        rename = "httpHostHeader",
        skip_serializing_if = "Option::is_none"
    )]
    pub http_host_header: Option<String>,
    #[serde(
        default,
        rename = "originServerName",
        skip_serializing_if = "Option::is_none"
    )]
    pub origin_server_name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AccessConfig {
    #[serde(default, rename = "audTag", skip_serializing_if = "Vec::is_empty")]
    pub aud_tag: Vec<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default, rename = "teamName")]
    pub team_name: String,
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
    let tunnel_id = resolve_tunnel_id(client, tunnel_id).await?;
    let path = format!(
        "/accounts/{}/cfd_tunnel/{}/configurations",
        client.account_id(),
        tunnel_id
    );
    let response: ApiResponse<TunnelConfig> = client.get(&path).await?;

    let Some(config) = response.result.config else {
        println!("No ingress rules configured");
        return Ok(());
    };

    if config.ingress.is_empty() {
        println!("No ingress rules configured");
        return Ok(());
    }

    for rule in config.ingress {
        if let Some(hostname) = rule.hostname {
            let path = rule.path.unwrap_or_default();
            print!("{}{} → {}", hostname, path, rule.service);
            print_origin_request(&rule.origin_request);
            println!();
        } else {
            println!("(catch-all) → {}", rule.service);
        }
    }

    Ok(())
}

fn print_origin_request(origin_request: &Option<OriginRequest>) {
    let Some(or) = origin_request else { return };

    let mut flags = Vec::new();

    if let Some(access) = &or.access {
        if access.required {
            flags.push("access:required".to_string());
        }
    }

    if or.no_tls_verify == Some(true) {
        flags.push("noTLSVerify".to_string());
    }

    if let Some(header) = &or.http_host_header {
        flags.push(format!("host:{}", header));
    }

    if let Some(server_name) = &or.origin_server_name {
        flags.push(format!("originServerName:{}", server_name));
    }

    if !flags.is_empty() {
        print!(" [{}]", flags.join(", "));
    }
}

fn print_access_config(access: &AccessConfig) {
    println!("Access:");
    println!("  Required: {}", access.required);
    println!("  Team: {}", access.team_name);
    if !access.aud_tag.is_empty() {
        println!("  AUD Tags:");
        for tag in &access.aud_tag {
            println!("    - {}", tag);
        }
    }
}

fn print_origin_details(or: &OriginRequest) {
    if let Some(access) = &or.access {
        print_access_config(access);
    }
    if or.no_tls_verify == Some(true) {
        println!("TLS Verify: disabled");
    }
    if let Some(header) = &or.http_host_header {
        println!("Host Header: {}", header);
    }
    if let Some(server_name) = &or.origin_server_name {
        println!("Origin Server Name: {}", server_name);
    }
}

fn print_matching_rule(tunnel: &Tunnel, rule: &IngressRule) {
    let rule_hostname = rule.hostname.as_deref().unwrap_or("");
    println!("Tunnel: {} ({})", tunnel.name, tunnel.id);
    println!("Hostname: {}", rule_hostname);
    if let Some(path) = &rule.path {
        println!("Path: {}", path);
    }
    println!("Service: {}", rule.service);
    if let Some(or) = &rule.origin_request {
        print_origin_details(or);
    }
}

pub async fn show(client: &Client, hostname: &str) -> Result<()> {
    let path = format!("/accounts/{}/cfd_tunnel", client.account_id());
    let tunnels_response: ApiResponse<Vec<Tunnel>> = client.get(&path).await?;

    let active_tunnels: Vec<_> = tunnels_response
        .result
        .into_iter()
        .filter(|t| t.status == "healthy")
        .collect();

    for tunnel in active_tunnels {
        let config_path = format!(
            "/accounts/{}/cfd_tunnel/{}/configurations",
            client.account_id(),
            tunnel.id
        );
        let config_response: ApiResponse<TunnelConfig> = client.get(&config_path).await?;

        let Some(config) = config_response.result.config else {
            continue;
        };

        for rule in &config.ingress {
            let rule_hostname = rule.hostname.as_deref().unwrap_or("");
            if rule_hostname == hostname || rule_hostname.contains(hostname) {
                print_matching_rule(&tunnel, rule);
                return Ok(());
            }
        }
    }

    println!("No ingress rule found for hostname: {}", hostname);
    Ok(())
}

async fn get_config(client: &Client, tunnel_id: &str) -> Result<TunnelConfig> {
    let path = format!(
        "/accounts/{}/cfd_tunnel/{}/configurations",
        client.account_id(),
        tunnel_id
    );
    let response: ApiResponse<TunnelConfig> = client.get(&path).await?;
    let mut config = response.result;
    // Ensure config is not None - create empty config with catch-all if needed
    if config.config.is_none() {
        config.config = Some(TunnelConfigInner {
            ingress: vec![IngressRule {
                hostname: None,
                service: "http_status:404".to_string(),
                path: None,
                origin_request: None,
            }],
        });
    }
    Ok(config)
}

async fn put_config(client: &Client, tunnel_id: &str, config: &TunnelConfig) -> Result<()> {
    let path = format!(
        "/accounts/{}/cfd_tunnel/{}/configurations",
        client.account_id(),
        tunnel_id
    );
    let _response: ApiResponse<TunnelConfig> = client.put(&path, config).await?;
    Ok(())
}

fn ensure_hostname_not_present(config: &TunnelConfigInner, hostname: &str) -> Result<()> {
    if config
        .ingress
        .iter()
        .any(|rule| rule.hostname.as_deref() == Some(hostname))
    {
        bail!(
            "Hostname '{}' already exists in tunnel configuration",
            hostname
        );
    }
    Ok(())
}

fn build_access_config(access_aud: Option<&str>) -> Option<AccessConfig> {
    access_aud.map(|aud| AccessConfig {
        aud_tag: vec![aud.to_string()],
        required: true,
        team_name: "globalcomixdev".to_string(),
    })
}

fn build_ingress_rule(hostname: &str, service: &str, access: Option<AccessConfig>) -> IngressRule {
    IngressRule {
        hostname: Some(hostname.to_string()),
        service: service.to_string(),
        path: None,
        origin_request: Some(OriginRequest {
            access,
            no_tls_verify: None,
            http_host_header: None,
            origin_server_name: None,
        }),
    }
}

fn find_insert_position(config: &TunnelConfigInner) -> usize {
    config
        .ingress
        .iter()
        .position(|rule| rule.hostname.is_none())
        .unwrap_or(config.ingress.len())
}

fn print_add_domain_result(hostname: &str, service: &str, access_aud: Option<&str>) {
    if access_aud.is_some() {
        println!("Added {} -> {} [access:required]", hostname, service);
    } else {
        println!("Added {} -> {}", hostname, service);
    }
}

pub async fn add_domain(
    client: &Client,
    tunnel_id: &str,
    hostname: &str,
    service: &str,
    access_aud: Option<&str>,
) -> Result<()> {
    let tunnel_id = resolve_tunnel_id(client, tunnel_id).await?;
    let mut config = get_config(client, &tunnel_id).await?;
    let inner = config.config.as_mut().unwrap(); // Safe: get_config ensures it's Some

    ensure_hostname_not_present(inner, hostname)?;
    let access = build_access_config(access_aud);
    let new_rule = build_ingress_rule(hostname, service, access);
    let insert_pos = find_insert_position(inner);

    inner.ingress.insert(insert_pos, new_rule);

    put_config(client, &tunnel_id, &config).await?;
    print_add_domain_result(hostname, service, access_aud);
    Ok(())
}

fn set_origin_server_name(
    config: &mut TunnelConfigInner,
    hostname: &str,
    origin_server_name: &str,
) -> Result<()> {
    let rule = config
        .ingress
        .iter_mut()
        .find(|rule| rule.hostname.as_deref() == Some(hostname))
        .ok_or_else(|| {
            anyhow::anyhow!("Hostname '{}' not found in tunnel configuration", hostname)
        })?;

    let origin_request = rule.origin_request.get_or_insert(OriginRequest {
        access: None,
        no_tls_verify: None,
        http_host_header: None,
        origin_server_name: None,
    });
    origin_request.origin_server_name = Some(origin_server_name.to_string());
    Ok(())
}

pub async fn update_origin_server_name(
    client: &Client,
    tunnel_id: &str,
    hostname: &str,
    origin_server_name: &str,
) -> Result<()> {
    let tunnel_id = resolve_tunnel_id(client, tunnel_id).await?;
    let mut config = get_config(client, &tunnel_id).await?;
    let inner = config.config.as_mut().unwrap(); // Safe: get_config ensures it's Some

    set_origin_server_name(inner, hostname, origin_server_name)?;
    put_config(client, &tunnel_id, &config).await?;

    println!(
        "Set origin server name for {} to {}",
        hostname, origin_server_name
    );
    Ok(())
}

pub async fn remove_domain(client: &Client, tunnel_id: &str, hostname: &str) -> Result<()> {
    let tunnel_id = resolve_tunnel_id(client, tunnel_id).await?;
    let mut config = get_config(client, &tunnel_id).await?;
    let inner = config.config.as_mut().unwrap(); // Safe: get_config ensures it's Some

    // Find the rule to remove
    let initial_len = inner.ingress.len();
    inner
        .ingress
        .retain(|r| r.hostname.as_deref() != Some(hostname));

    if inner.ingress.len() == initial_len {
        bail!("Hostname '{}' not found in tunnel configuration", hostname);
    }

    put_config(client, &tunnel_id, &config).await?;

    println!("Removed {}", hostname);
    Ok(())
}

/// Resolve tunnel name to ID. If input is already a UUID, return it as-is.
async fn resolve_tunnel_id(client: &Client, name_or_id: &str) -> Result<String> {
    // If it looks like a UUID, use it directly
    if name_or_id.len() == 36 && name_or_id.chars().filter(|c| *c == '-').count() == 4 {
        return Ok(name_or_id.to_string());
    }

    // Otherwise, look up by name
    let path = format!("/accounts/{}/cfd_tunnel", client.account_id());
    let response: ApiResponse<Vec<Tunnel>> = client.get(&path).await?;

    for tunnel in response.result {
        if tunnel.name == name_or_id {
            return Ok(tunnel.id);
        }
    }

    bail!("Tunnel '{}' not found", name_or_id);
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    result: String,
    success: bool,
}

pub async fn token(client: &Client, name_or_id: &str) -> Result<()> {
    let tunnel_id = resolve_tunnel_id(client, name_or_id).await?;
    let path = format!(
        "/accounts/{}/cfd_tunnel/{}/token",
        client.account_id(),
        tunnel_id
    );
    let response: TokenResponse = client.get(&path).await?;
    println!("{}", response.result);
    Ok(())
}

#[derive(Debug, Serialize)]
struct CreateTunnelRequest {
    name: String,
    config_src: String,
}

#[derive(Debug, Deserialize)]
struct CreatedTunnel {
    id: String,
    name: String,
    token: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_origin_server_name_preserves_existing_origin_settings() {
        let mut config = TunnelConfigInner {
            ingress: vec![IngressRule {
                hostname: Some("deploy-bot-ci.globalcomixdev.com".to_string()),
                service: "https://deploy-bot.ops.svc.cluster.local:443".to_string(),
                path: None,
                origin_request: Some(OriginRequest {
                    access: Some(AccessConfig {
                        aud_tag: vec!["aud-tag".to_string()],
                        required: true,
                        team_name: "globalcomixdev".to_string(),
                    }),
                    no_tls_verify: None,
                    http_host_header: Some("existing.example.com".to_string()),
                    origin_server_name: None,
                }),
            }],
        };

        set_origin_server_name(
            &mut config,
            "deploy-bot-ci.globalcomixdev.com",
            "deploy-bot-ci.globalcomixdev.com",
        )
        .expect("existing hostname should be updated");

        let value = serde_json::to_value(config).expect("configuration should serialize");
        let origin = &value["ingress"][0]["originRequest"];
        assert_eq!(
            origin["originServerName"],
            "deploy-bot-ci.globalcomixdev.com"
        );
        assert_eq!(origin["httpHostHeader"], "existing.example.com");
        assert_eq!(origin["access"]["audTag"][0], "aud-tag");
        assert!(origin.get("noTLSVerify").is_none());
    }
}

pub async fn create(client: &Client, name: &str) -> Result<()> {
    let path = format!("/accounts/{}/cfd_tunnel", client.account_id());
    let request = CreateTunnelRequest {
        name: name.to_string(),
        config_src: "cloudflare".to_string(),
    };
    let response: ApiResponse<CreatedTunnel> = client.post(&path, &request).await?;
    let tunnel = response.result;

    println!("Created tunnel: {}", tunnel.name);
    println!("ID: {}", tunnel.id);

    // Fetch the token separately since the create response may not include it
    if let Some(token) = tunnel.token {
        println!("Token: {}", token);
    } else {
        let token_path = format!(
            "/accounts/{}/cfd_tunnel/{}/token",
            client.account_id(),
            tunnel.id
        );
        let token_response: TokenResponse = client.get(&token_path).await?;
        println!("Token: {}", token_response.result);
    }

    Ok(())
}
