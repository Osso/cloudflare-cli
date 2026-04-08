use anyhow::{Result, bail};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

use super::{ApiResponse, find_zone_id};
use crate::client::Client;

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

fn build_events_query(zone_id: &str, ip: Option<&str>, hours: u32, limit: u32) -> GraphQLQuery {
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

    GraphQLQuery {
        query,
        variables: serde_json::json!({}),
    }
}

fn check_graphql_errors(response: &GraphQLResponse) -> Result<()> {
    if let Some(errors) = &response.errors {
        for err in errors {
            eprintln!("GraphQL error: {}", err.message);
        }
        bail!("GraphQL query failed");
    }
    Ok(())
}

fn print_firewall_event(event: &FirewallEvent) {
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
    let query_suffix = if event.request_query.is_empty() {
        String::new()
    } else {
        format!("?{}", event.request_query)
    };
    println!(
        "{} {} [{}] {} {}{}",
        action_icon, event.action, event.source, event.client_ip, event.request_path, query_suffix
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

pub async fn events(
    client: &Client,
    zone: &str,
    ip: Option<&str>,
    hours: u32,
    limit: u32,
) -> Result<()> {
    let zone_id = find_zone_id(client, zone).await?;
    let gql_request = build_events_query(&zone_id, ip, hours, limit);
    let response: GraphQLResponse = client.graphql(&gql_request).await?;

    check_graphql_errors(&response)?;

    let data = response
        .data
        .ok_or_else(|| anyhow::anyhow!("No data returned"))?;

    if data.viewer.zones.is_empty() {
        println!("No zone data found");
        return Ok(());
    }

    let events = &data.viewer.zones[0].firewall_events;

    if events.is_empty() {
        match ip {
            Some(ip_addr) => println!(
                "No security events for IP {} in the last {} hours",
                ip_addr, hours
            ),
            None => println!("No security events in the last {} hours", hours),
        }
        return Ok(());
    }

    for event in events {
        print_firewall_event(event);
    }

    Ok(())
}

// Request type for creating IP access rules
#[derive(Debug, Serialize)]
struct AccessRuleCreateRequest {
    mode: String,
    configuration: AccessRuleConfigurationCreate,
    notes: String,
}

#[derive(Debug, Serialize)]
struct AccessRuleConfigurationCreate {
    target: String,
    value: String,
}

pub async fn block(client: &Client, zone: &str, ip: &str, note: Option<&str>) -> Result<()> {
    let zone_id = find_zone_id(client, zone).await?;

    // Determine if this is an IP, IP range (CIDR), or single address
    // ip_range works for both IPv4 and IPv6 CIDR notation
    let (target, value) = if ip.contains('/') {
        ("ip_range", ip.to_string())
    } else if ip.contains(':') {
        ("ip6", ip.to_string())
    } else {
        ("ip", ip.to_string())
    };

    let request = AccessRuleCreateRequest {
        mode: "block".to_string(),
        configuration: AccessRuleConfigurationCreate {
            target: target.to_string(),
            value,
        },
        notes: note.unwrap_or("Blocked via CLI").to_string(),
    };

    let path = format!("/zones/{}/firewall/access_rules/rules", zone_id);
    let response: ApiResponse<AccessRule> = client.post(&path, &request).await?;

    println!("⛔ Blocked {} {}", target, ip);
    println!("  Rule ID: {}", response.result.id);
    if let Some(n) = note {
        println!("  Note: {}", n);
    }

    Ok(())
}

pub async fn unblock(client: &Client, zone: &str, ip: &str) -> Result<()> {
    let zone_id = find_zone_id(client, zone).await?;

    // First find the rule ID for this IP
    let search_path = format!(
        "/zones/{}/firewall/access_rules/rules?configuration.value={}&per_page=100",
        zone_id, ip
    );
    let response: PaginatedResponse<Vec<AccessRule>> = client.get(&search_path).await?;

    if response.result.is_empty() {
        println!("No access rule found for IP {}", ip);
        return Ok(());
    }

    for rule in response.result {
        let delete_path = format!("/zones/{}/firewall/access_rules/rules/{}", zone_id, rule.id);
        client.delete(&delete_path).await?;
        println!("✓ Removed {} rule for {}", rule.mode, ip);
        println!("  Rule ID: {}", rule.id);
    }

    Ok(())
}

async fn fetch_existing_ratelimit_rules(
    client: &Client,
    api_path: &str,
) -> Result<Vec<WafRuleCreate>> {
    match client.get::<RulesetResponse>(api_path).await {
        Ok(response) => Ok(response
            .result
            .rules
            .into_iter()
            .map(|r| WafRuleCreate {
                id: Some(r.id),
                description: r.description,
                expression: r.expression,
                action: r.action,
                action_parameters: r.action_parameters,
                ratelimit: r.ratelimit,
                enabled: r.enabled,
            })
            .collect()),
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("404")
                || err_str.contains("10003")
                || err_str.contains("could not find")
            {
                Ok(Vec::new())
            } else {
                Err(e)
            }
        }
    }
}

fn build_ratelimit_rule(path: &str, requests: u32, period: u32, action: &str) -> WafRuleCreate {
    let mitigation_action = if action == "challenge" {
        "managed_challenge"
    } else {
        "block"
    };

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

    let ratelimit = serde_json::json!({
        "characteristics": ["cf.colo.id", "ip.src"],
        "period": period,
        "requests_per_period": requests,
        "mitigation_timeout": period
    });

    WafRuleCreate {
        id: None,
        description: format!("Rate limit {} ({} req/{}s)", path, requests, period),
        expression: format!(r#"(http.request.uri.path contains "{}")"#, path),
        action: mitigation_action.to_string(),
        action_parameters,
        ratelimit: Some(ratelimit),
        enabled: true,
    }
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
    let api_path = format!(
        "/zones/{}/rulesets/phases/http_ratelimit/entrypoint",
        zone_id
    );

    let mut all_rules = fetch_existing_ratelimit_rules(client, &api_path).await?;
    let new_rule = build_ratelimit_rule(path, requests, period, action);
    let mitigation_action = new_rule.action.clone();
    let description = new_rule.description.clone();
    all_rules.push(new_rule);

    let update_request = RulesetUpdateRequest { rules: all_rules };
    let _response: RulesetResponse = client.put(&api_path, &update_request).await?;

    println!("Created rate limit rule: {}", description);
    println!("  Path: {}", path);
    println!("  Limit: {} requests per {} seconds", requests, period);
    println!("  Action: {} when exceeded", mitigation_action);

    Ok(())
}
