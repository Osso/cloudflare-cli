use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::client::Client;
use crate::commands::tunnels::ApiResponse;

#[derive(Debug, Deserialize)]
pub struct Zone {
    pub id: String,
    pub name: String,
    pub status: String,
    #[serde(default)]
    pub paused: bool,
}

#[derive(Debug, Serialize)]
struct PurgeRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    purge_everything: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    files: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct PurgeResult {
    id: String,
}

pub async fn list_zones(client: &Client) -> Result<()> {
    let path = format!("/zones?account.id={}", client.account_id());
    let response: ApiResponse<Vec<Zone>> = client.get(&path).await?;

    if response.result.is_empty() {
        println!("No zones found");
        return Ok(());
    }

    for zone in response.result {
        let status_icon = match zone.status.as_str() {
            "active" => "●",
            "pending" => "○",
            _ => "◌",
        };
        let paused = if zone.paused { " (paused)" } else { "" };
        println!("{} {}{}", status_icon, zone.name, paused);
        println!("  ID: {}", zone.id);
    }

    Ok(())
}

async fn find_zone_id(client: &Client, zone: &str) -> Result<String> {
    if zone.len() == 32 && zone.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(zone.to_string());
    }

    let path = format!("/zones?name={}&account.id={}", zone, client.account_id());
    let response: ApiResponse<Vec<Zone>> = client.get(&path).await?;

    if response.result.is_empty() {
        bail!("Zone '{}' not found. Use 'cloudflare cache zones' to list available zones.", zone);
    }

    Ok(response.result[0].id.clone())
}

fn build_purge_request(urls: Option<Vec<String>>, all: bool) -> Result<PurgeRequest> {
    if all {
        return Ok(PurgeRequest { purge_everything: Some(true), files: None });
    }
    if let Some(files) = urls {
        if files.is_empty() {
            bail!("No URLs provided. Use --url <url> or --all to purge everything.");
        }
        return Ok(PurgeRequest { purge_everything: None, files: Some(files) });
    }
    bail!("Specify --url <url> to purge specific URLs, or --all to purge everything.");
}

pub async fn purge(client: &Client, zone: &str, urls: Option<Vec<String>>, all: bool) -> Result<()> {
    let zone_id = find_zone_id(client, zone).await?;
    let request = build_purge_request(urls, all)?;

    let path = format!("/zones/{}/purge_cache", zone_id);
    let response: ApiResponse<PurgeResult> = client.post(&path, &request).await?;

    if all {
        println!("Purged all cache for zone (request ID: {})", response.result.id);
    } else {
        println!("Purged cache (request ID: {})", response.result.id);
    }

    Ok(())
}

// Page Rules structs
#[derive(Debug, Deserialize)]
struct PageRule {
    id: String,
    status: String,
    priority: i32,
    targets: Vec<PageRuleTarget>,
    actions: Vec<PageRuleAction>,
}

#[derive(Debug, Deserialize)]
struct PageRuleTarget {
    target: String,
    constraint: PageRuleConstraint,
}

#[derive(Debug, Deserialize)]
struct PageRuleConstraint {
    operator: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct PageRuleAction {
    id: String,
    value: Option<Value>,
}

fn format_action_value(value: &Option<Value>) -> String {
    match value {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Object(obj)) => serde_json::to_string(obj).unwrap_or_else(|_| "...".to_string()),
        Some(other) => other.to_string(),
        None => "on".to_string(),
    }
}

pub async fn page_rules(client: &Client, zone: &str) -> Result<()> {
    let zone_id = find_zone_id(client, zone).await?;
    let path = format!("/zones/{}/pagerules", zone_id);
    let response: ApiResponse<Vec<PageRule>> = client.get(&path).await?;

    if response.result.is_empty() {
        println!("No page rules found");
        return Ok(());
    }

    for rule in response.result {
        let status_icon = if rule.status == "active" { "●" } else { "○" };
        let url = rule.targets.first().map(|t| t.constraint.value.as_str()).unwrap_or("unknown");
        println!("{} [{}] {}", status_icon, rule.priority, url);
        for action in &rule.actions {
            println!("  {} = {}", action.id, format_action_value(&action.value));
        }
    }

    Ok(())
}

// Cache Rules (Rulesets) structs
#[derive(Debug, Deserialize)]
struct Ruleset {
    id: String,
    name: String,
    phase: String,
    #[serde(default)]
    rules: Vec<CacheRule>,
}

#[derive(Debug, Deserialize)]
struct CacheRule {
    id: String,
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    expression: String,
    action: String,
    #[serde(default)]
    action_parameters: Option<Value>,
    #[serde(default)]
    description: String,
}

fn print_cache_rule(rule: &CacheRule) {
    let status_icon = if rule.enabled { "●" } else { "○" };
    let desc = if rule.description.is_empty() { &rule.action } else { &rule.description };
    println!("{} {}", status_icon, desc);
    println!("  ID: {}", rule.id);
    println!("  Expression: {}", rule.expression);
    println!("  Action: {}", rule.action);
    if let Some(Value::Object(obj)) = &rule.action_parameters {
        for (key, value) in obj {
            let value_str = match value {
                Value::String(s) => s.clone(),
                Value::Bool(b) => b.to_string(),
                Value::Number(n) => n.to_string(),
                other => other.to_string(),
            };
            println!("    {}: {}", key, value_str);
        }
    }
}

async fn find_cache_ruleset(client: &Client, zone_id: &str) -> Result<Option<Ruleset>> {
    let path = format!("/zones/{}/rulesets", zone_id);
    let response: ApiResponse<Vec<Ruleset>> = client.get(&path).await?;
    Ok(response.result.into_iter().find(|r| r.phase == "http_request_cache_settings"))
}

async fn fetch_full_ruleset(client: &Client, zone_id: &str, ruleset_id: &str) -> Result<Ruleset> {
    let path = format!("/zones/{}/rulesets/{}", zone_id, ruleset_id);
    let response: ApiResponse<Ruleset> = client.get(&path).await?;
    Ok(response.result)
}

fn rules_to_json(rules: &[CacheRule]) -> Vec<Value> {
    rules.iter().map(|r| {
        json!({
            "id": r.id,
            "expression": r.expression,
            "description": r.description,
            "action": r.action,
            "action_parameters": r.action_parameters,
            "enabled": r.enabled
        })
    }).collect()
}

pub async fn cache_rules(client: &Client, zone: &str) -> Result<()> {
    let zone_id = find_zone_id(client, zone).await?;

    let ruleset = match find_cache_ruleset(client, &zone_id).await? {
        Some(r) => r,
        None => { println!("No cache rules configured"); return Ok(()); }
    };

    let full = fetch_full_ruleset(client, &zone_id, &ruleset.id).await?;
    if full.rules.is_empty() {
        println!("No cache rules in ruleset");
        return Ok(());
    }

    for rule in &full.rules {
        print_cache_rule(rule);
        println!();
    }

    Ok(())
}

fn build_rule_json(name: &str, expression: &str, bypass: bool) -> Value {
    if bypass {
        json!({
            "expression": expression,
            "description": name,
            "action": "set_cache_settings",
            "action_parameters": { "cache": false },
            "enabled": true
        })
    } else {
        json!({
            "expression": expression,
            "description": name,
            "action": "set_cache_settings",
            "action_parameters": {
                "cache": true,
                "edge_ttl": { "mode": "respect_origin" },
                "browser_ttl": { "mode": "respect_origin" }
            },
            "enabled": true
        })
    }
}

fn print_rule_summary(label: &str, id: &str, rule: Option<&CacheRule>) {
    println!("{} (ID: {})", label, id);
    if let Some(rule) = rule {
        println!("  Rule ID: {}", rule.id);
        println!("  Description: {}", rule.description);
        println!("  Expression: {}", rule.expression);
    }
}

async fn append_to_ruleset(client: &Client, zone_id: &str, ruleset: &Ruleset, new_rule: Value) -> Result<()> {
    let full = fetch_full_ruleset(client, zone_id, &ruleset.id).await?;
    let mut rules = rules_to_json(&full.rules);
    rules.push(new_rule);

    let path = format!("/zones/{}/rulesets/{}", zone_id, ruleset.id);
    let response: ApiResponse<Ruleset> = client.put(&path, &json!({ "rules": rules })).await?;
    print_rule_summary("Added cache rule to existing ruleset", &ruleset.id, response.result.rules.last());
    Ok(())
}

async fn create_new_ruleset(client: &Client, zone_id: &str, new_rule: Value) -> Result<()> {
    let body = json!({
        "name": "Cache Rules",
        "kind": "zone",
        "phase": "http_request_cache_settings",
        "rules": [new_rule]
    });
    let path = format!("/zones/{}/rulesets", zone_id);
    let response: ApiResponse<Ruleset> = client.post(&path, &body).await?;
    print_rule_summary("Created cache rule ruleset", &response.result.id, response.result.rules.first());
    Ok(())
}

pub async fn create_rule(client: &Client, zone: &str, name: &str, expression: &str, bypass: bool) -> Result<()> {
    let zone_id = find_zone_id(client, zone).await?;
    let new_rule = build_rule_json(name, expression, bypass);

    match find_cache_ruleset(client, &zone_id).await? {
        Some(ruleset) => append_to_ruleset(client, &zone_id, &ruleset, new_rule).await,
        None => create_new_ruleset(client, &zone_id, new_rule).await,
    }
}

async fn find_required_cache_ruleset(client: &Client, zone_id: &str, zone: &str) -> Result<Ruleset> {
    find_cache_ruleset(client, zone_id).await?.ok_or_else(|| {
        anyhow::anyhow!("No cache rules ruleset found for zone '{}'", zone)
    })
}

fn find_rule_index(rules: &[CacheRule], name: &str) -> Result<usize> {
    rules.iter().position(|r| r.description == name).ok_or_else(|| {
        let available: Vec<&str> = rules.iter().map(|r| r.description.as_str()).collect();
        anyhow::anyhow!("Rule '{}' not found. Available rules: {}", name, available.join(", "))
    })
}

pub async fn update_rule(client: &Client, zone: &str, name: &str, expression: &str) -> Result<()> {
    let zone_id = find_zone_id(client, zone).await?;
    let ruleset = find_required_cache_ruleset(client, &zone_id, zone).await?;
    let full = fetch_full_ruleset(client, &zone_id, &ruleset.id).await?;
    let rule_idx = find_rule_index(&full.rules, name)?;

    let rules: Vec<Value> = full.rules.iter().enumerate().map(|(i, r)| {
        let expr = if i == rule_idx { expression } else { &r.expression };
        json!({
            "id": r.id, "expression": expr, "description": r.description,
            "action": r.action, "action_parameters": r.action_parameters, "enabled": r.enabled
        })
    }).collect();

    let path = format!("/zones/{}/rulesets/{}", zone_id, ruleset.id);
    let response: ApiResponse<Ruleset> = client.put(&path, &json!({ "rules": rules })).await?;
    print_rule_summary("Updated cache rule", &ruleset.id, response.result.rules.get(rule_idx));
    Ok(())
}

fn find_rule_for_delete<'a>(rules: &'a [CacheRule], rule_id: &str) -> Result<&'a str> {
    let rule = rules.iter().find(|r| r.id == rule_id).ok_or_else(|| {
        let available: Vec<String> = rules.iter().map(|r| format!("  {} - {}", r.id, r.description)).collect();
        anyhow::anyhow!("Rule '{}' not found. Available rules:\n{}", rule_id, available.join("\n"))
    })?;
    Ok(&rule.description)
}

pub async fn delete_rule(client: &Client, zone: &str, rule_id: &str) -> Result<()> {
    let zone_id = find_zone_id(client, zone).await?;
    let ruleset = find_required_cache_ruleset(client, &zone_id, zone).await?;
    let full = fetch_full_ruleset(client, &zone_id, &ruleset.id).await?;
    let description = find_rule_for_delete(&full.rules, rule_id)?.to_string();

    let rules: Vec<Value> = full.rules.iter().filter(|r| r.id != rule_id).map(|r| {
        json!({
            "id": r.id, "expression": r.expression, "description": r.description,
            "action": r.action, "action_parameters": r.action_parameters, "enabled": r.enabled
        })
    }).collect();

    let path = format!("/zones/{}/rulesets/{}", zone_id, ruleset.id);
    let _: ApiResponse<Ruleset> = client.put(&path, &json!({ "rules": rules })).await?;
    println!("Deleted cache rule '{}'", rule_id);
    if !description.is_empty() {
        println!("  Description: {}", description);
    }
    Ok(())
}
