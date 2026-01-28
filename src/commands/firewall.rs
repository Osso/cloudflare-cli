use anyhow::{Result, bail};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::commands::cache::Zone;
use crate::commands::tunnels::ApiResponse;

#[derive(Debug, Deserialize)]
pub struct AccessRule {
    pub id: String,
    pub mode: String,
    pub configuration: AccessRuleConfiguration,
    #[serde(default)]
    pub notes: String,
    pub created_on: String,
    pub modified_on: String,
    #[serde(default)]
    pub scope: Option<AccessRuleScope>,
}

#[derive(Debug, Deserialize)]
pub struct AccessRuleConfiguration {
    pub target: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct AccessRuleScope {
    pub id: String,
    #[serde(rename = "type")]
    pub scope_type: String,
}

#[derive(Debug, Deserialize)]
struct ResultInfo {
    page: u32,
    per_page: u32,
    total_count: u32,
    total_pages: u32,
}

#[derive(Debug, Deserialize)]
struct PaginatedResponse<T> {
    result: T,
    #[serde(default)]
    result_info: Option<ResultInfo>,
    success: bool,
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
            "Zone '{}' not found. Use 'cloudflare cache zones' to list available zones.",
            zone
        );
    }

    Ok(response.result[0].id.clone())
}

pub async fn list(client: &Client, zone: &str) -> Result<()> {
    let zone_id = find_zone_id(client, zone).await?;
    let path = format!(
        "/zones/{}/firewall/access_rules/rules?per_page=100",
        zone_id
    );
    let response: PaginatedResponse<Vec<AccessRule>> = client.get(&path).await?;

    if response.result.is_empty() {
        println!("No IP access rules found");
        return Ok(());
    }

    for rule in response.result {
        let mode_icon = match rule.mode.as_str() {
            "block" => "⛔",
            "challenge" => "❓",
            "whitelist" => "✓",
            "js_challenge" => "🔒",
            "managed_challenge" => "🤖",
            _ => "?",
        };
        let notes = if rule.notes.is_empty() {
            String::new()
        } else {
            format!(" ({})", rule.notes)
        };
        println!(
            "{} {} {} {}{}",
            mode_icon, rule.mode, rule.configuration.target, rule.configuration.value, notes
        );
    }

    Ok(())
}

pub async fn check(client: &Client, zone: &str, ip: &str) -> Result<()> {
    let zone_id = find_zone_id(client, zone).await?;

    // Search for exact IP match
    let path = format!(
        "/zones/{}/firewall/access_rules/rules?configuration.value={}&per_page=100",
        zone_id, ip
    );
    let response: PaginatedResponse<Vec<AccessRule>> = client.get(&path).await?;

    if response.result.is_empty() {
        println!("IP {} is not in any access rules", ip);
        return Ok(());
    }

    for rule in response.result {
        let mode_icon = match rule.mode.as_str() {
            "block" => "⛔",
            "challenge" => "❓",
            "whitelist" => "✓",
            "js_challenge" => "🔒",
            "managed_challenge" => "🤖",
            _ => "?",
        };
        let notes = if rule.notes.is_empty() {
            String::new()
        } else {
            format!(" ({})", rule.notes)
        };
        println!(
            "{} {} {} {}{}",
            mode_icon, rule.mode, rule.configuration.target, rule.configuration.value, notes
        );
        println!("  ID: {}", rule.id);
        println!("  Created: {}", rule.created_on);
    }

    Ok(())
}

// GraphQL types for security events
#[derive(Debug, Serialize)]
struct GraphQLQuery {
    query: String,
    variables: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct GraphQLResponse {
    data: Option<GraphQLData>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Deserialize)]
struct GraphQLError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct GraphQLData {
    viewer: Viewer,
}

#[derive(Debug, Deserialize)]
struct Viewer {
    zones: Vec<ZoneData>,
}

#[derive(Debug, Deserialize)]
struct ZoneData {
    #[serde(rename = "firewallEventsAdaptive")]
    firewall_events: Vec<FirewallEvent>,
}

#[derive(Debug, Deserialize)]
struct FirewallEvent {
    action: String,
    #[serde(rename = "clientAsn")]
    client_asn: String,
    #[serde(rename = "clientCountryName")]
    client_country: String,
    #[serde(rename = "clientIP")]
    client_ip: String,
    #[serde(rename = "clientRequestPath")]
    request_path: String,
    #[serde(rename = "clientRequestQuery")]
    request_query: String,
    datetime: String,
    source: String,
    #[serde(rename = "userAgent")]
    user_agent: String,
    #[serde(rename = "rayName")]
    ray_id: String,
    #[serde(rename = "ruleId")]
    rule_id: String,
}

// WAF custom rules types
#[derive(Debug, Deserialize)]
struct RulesetResponse {
    result: Ruleset,
}

#[derive(Debug, Deserialize)]
struct Ruleset {
    id: String,
    name: String,
    #[serde(default)]
    rules: Vec<WafRule>,
}

#[derive(Debug, Deserialize)]
struct WafRule {
    id: String,
    #[serde(default)]
    description: String,
    expression: String,
    action: String,
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    action_parameters: Option<serde_json::Value>,
    #[serde(default)]
    ratelimit: Option<serde_json::Value>,
}

// Request types for creating/updating rules
#[derive(Debug, Serialize)]
struct RulesetUpdateRequest {
    rules: Vec<WafRuleCreate>,
}

#[derive(Debug, Serialize)]
struct WafRuleCreate {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    description: String,
    expression: String,
    action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    action_parameters: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ratelimit: Option<serde_json::Value>,
    enabled: bool,
}

pub async fn rules(client: &Client, zone: &str) -> Result<()> {
    let zone_id = find_zone_id(client, zone).await?;

    let path = format!(
        "/zones/{}/rulesets/phases/http_request_firewall_custom/entrypoint",
        zone_id
    );

    let response: RulesetResponse = match client.get(&path).await {
        Ok(r) => r,
        Err(e) => {
            if e.to_string().contains("404") {
                println!("No custom WAF rules configured");
                return Ok(());
            }
            return Err(e);
        }
    };

    if response.result.rules.is_empty() {
        println!("No custom WAF rules configured");
        return Ok(());
    }

    for rule in &response.result.rules {
        let status = if rule.enabled { "●" } else { "○" };
        let action_icon = match rule.action.as_str() {
            "block" => "⛔",
            "challenge" => "❓",
            "js_challenge" => "🔒",
            "managed_challenge" => "🤖",
            "skip" => "⏭",
            "log" => "📝",
            _ => "?",
        };
        println!(
            "{} {} {} {}",
            status,
            action_icon,
            rule.action,
            if rule.description.is_empty() {
                "(no description)"
            } else {
                &rule.description
            }
        );
        println!("  ID: {}", rule.id);
        println!("  Expression: {}", rule.expression);
        println!();
    }

    Ok(())
}

pub async fn events(
    client: &Client,
    zone: &str,
    ip: Option<&str>,
    hours: u32,
    limit: u32,
) -> Result<()> {
    let zone_id = find_zone_id(client, zone).await?;

    let now = Utc::now();
    let start = now - Duration::hours(hours as i64);

    let mut filter = format!(
        r#"datetime_geq: "{}", datetime_leq: "{}""#,
        start.format("%Y-%m-%dT%H:%M:%SZ"),
        now.format("%Y-%m-%dT%H:%M:%SZ")
    );

    if let Some(ip_addr) = ip {
        filter.push_str(&format!(r#", clientIP: "{}""#, ip_addr));
    }

    let query = format!(
        r#"query {{
            viewer {{
                zones(filter: {{ zoneTag: "{}" }}) {{
                    firewallEventsAdaptive(
                        filter: {{ {} }}
                        limit: {}
                        orderBy: [datetime_DESC]
                    ) {{
                        action
                        clientAsn
                        clientCountryName
                        clientIP
                        clientRequestPath
                        clientRequestQuery
                        datetime
                        source
                        userAgent
                        rayName
                        ruleId
                    }}
                }}
            }}
        }}"#,
        zone_id, filter, limit
    );

    let gql_request = GraphQLQuery {
        query,
        variables: serde_json::json!({}),
    };

    let response: GraphQLResponse = client.graphql(&gql_request).await?;

    if let Some(errors) = response.errors {
        for err in errors {
            eprintln!("GraphQL error: {}", err.message);
        }
        bail!("GraphQL query failed");
    }

    let data = response
        .data
        .ok_or_else(|| anyhow::anyhow!("No data returned"))?;

    if data.viewer.zones.is_empty() {
        println!("No zone data found");
        return Ok(());
    }

    let events = &data.viewer.zones[0].firewall_events;

    if events.is_empty() {
        if let Some(ip_addr) = ip {
            println!(
                "No security events for IP {} in the last {} hours",
                ip_addr, hours
            );
        } else {
            println!("No security events in the last {} hours", hours);
        }
        return Ok(());
    }

    for event in events {
        let action_icon = match event.action.as_str() {
            "block" => "⛔",
            "challenge" => "❓",
            "jschallenge" => "🔒",
            "managed_challenge" => "🤖",
            "log" => "📝",
            "allow" => "✓",
            "skip" => "⏭",
            _ => "?",
        };

        println!(
            "{} {} [{}] {} {}{}",
            action_icon,
            event.action,
            event.source,
            event.client_ip,
            event.request_path,
            if event.request_query.is_empty() {
                String::new()
            } else {
                format!("?{}", event.request_query)
            }
        );
        println!("  Time: {}", event.datetime);
        println!(
            "  Country: {} | ASN: {}",
            event.client_country, event.client_asn
        );
        if !event.rule_id.is_empty() {
            println!("  Rule: {}", event.rule_id);
        }
        println!("  Ray: {}", event.ray_id);
        println!();
    }

    Ok(())
}

pub async fn ratelimit(
    client: &Client,
    zone: &str,
    path: &str,
    requests: u32,
    period: u32,
    action: &str,
) -> Result<()> {
    let zone_id = find_zone_id(client, zone).await?;

    // Rate limiting uses the http_ratelimit phase, not http_request_firewall_custom
    let api_path = format!(
        "/zones/{}/rulesets/phases/http_ratelimit/entrypoint",
        zone_id
    );

    // Get existing ruleset (may not exist yet - returns 10003 error if no ruleset)
    let existing_rules: Vec<WafRuleCreate> = match client.get::<RulesetResponse>(&api_path).await {
        Ok(response) => response
            .result
            .rules
            .into_iter()
            .map(|r| WafRuleCreate {
                id: Some(r.id),
                description: r.description,
                expression: r.expression,
                action: r.action,
                action_parameters: r.action_parameters,
                ratelimit: r.ratelimit, // Preserve existing ratelimit config
                enabled: r.enabled,
            })
            .collect(),
        Err(e) => {
            let err_str = e.to_string();
            // 404 or "could not find entrypoint ruleset" means no ruleset exists yet
            if err_str.contains("404")
                || err_str.contains("10003")
                || err_str.contains("could not find")
            {
                Vec::new()
            } else {
                return Err(e);
            }
        }
    };

    // Build the new rate limit rule
    let description = format!("Rate limit {} ({} req/{}s)", path, requests, period);
    let expression = format!(r#"(http.request.uri.path contains "{}")"#, path);

    // For http_ratelimit phase, action is "block" or "managed_challenge"
    let mitigation_action = if action == "challenge" {
        "managed_challenge"
    } else {
        "block"
    };

    // action_parameters is only valid for "block" action, not for challenges
    let action_parameters = if mitigation_action == "block" {
        Some(serde_json::json!({
            "response": {
                "status_code": 429,
                "content_type": "text/plain",
                "content": "Too many requests. Please try again later."
            }
        }))
    } else {
        None
    };

    // Rate limit configuration - cf.colo.id is mandatory
    let ratelimit = serde_json::json!({
        "characteristics": ["cf.colo.id", "ip.src"],
        "period": period,
        "requests_per_period": requests,
        "mitigation_timeout": period
    });

    let new_rule = WafRuleCreate {
        id: None,
        description: description.clone(),
        expression,
        action: mitigation_action.to_string(),
        action_parameters,
        ratelimit: Some(ratelimit),
        enabled: true,
    };

    // Append new rule to existing rules
    let mut all_rules = existing_rules;
    all_rules.push(new_rule);

    let update_request = RulesetUpdateRequest { rules: all_rules };

    let _response: RulesetResponse = client.put(&api_path, &update_request).await?;

    println!("Created rate limit rule: {}", description);
    println!("  Path: {}", path);
    println!("  Limit: {} requests per {} seconds", requests, period);
    println!("  Action: {} when exceeded", mitigation_action);

    Ok(())
}
