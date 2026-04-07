use anyhow::Result;
use serde::Deserialize;

use super::ApiResponse;
use crate::client::Client;

#[derive(Debug, Deserialize)]
pub struct GatewayRule {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub action: String,
    pub filters: Vec<String>,
    pub traffic: Option<String>,
    pub identity: Option<String>,
    pub precedence: i32,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn dns_rules(client: &Client) -> Result<()> {
    let path = format!(
        "/accounts/{}/gateway/rules?traffic_type=dns",
        client.account_id()
    );
    let response: ApiResponse<Vec<GatewayRule>> = client.get(&path).await?;

    print_rules("DNS", &response.result);
    Ok(())
}

pub async fn network_rules(client: &Client) -> Result<()> {
    let path = format!(
        "/accounts/{}/gateway/rules?traffic_type=l4",
        client.account_id()
    );
    let response: ApiResponse<Vec<GatewayRule>> = client.get(&path).await?;

    print_rules("Network", &response.result);
    Ok(())
}

pub async fn http_rules(client: &Client) -> Result<()> {
    let path = format!(
        "/accounts/{}/gateway/rules?traffic_type=http",
        client.account_id()
    );
    let response: ApiResponse<Vec<GatewayRule>> = client.get(&path).await?;

    print_rules("HTTP", &response.result);
    Ok(())
}

fn print_rules(rule_type: &str, rules: &[GatewayRule]) {
    if rules.is_empty() {
        println!("No {} rules found", rule_type);
        return;
    }

    println!("{} Rules:", rule_type);
    println!();

    for rule in rules {
        let status = if rule.enabled { "●" } else { "○" };
        println!("{} {} [{}]", status, rule.name, rule.action);

        if let Some(desc) = &rule.description {
            if !desc.is_empty() {
                println!("  {}", desc);
            }
        }

        if let Some(traffic) = &rule.traffic {
            println!("  Traffic: {}", traffic);
        }

        println!("  Precedence: {}", rule.precedence);
        println!();
    }
}
