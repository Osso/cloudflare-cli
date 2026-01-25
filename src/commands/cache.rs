use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

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
    // If it looks like a zone ID (32 hex chars), use it directly
    if zone.len() == 32 && zone.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(zone.to_string());
    }

    // Otherwise, look up by domain name
    let path = format!("/zones?name={}&account.id={}", zone, client.account_id());
    let response: ApiResponse<Vec<Zone>> = client.get(&path).await?;

    if response.result.is_empty() {
        bail!("Zone '{}' not found. Use 'cloudflare cache zones' to list available zones.", zone);
    }

    Ok(response.result[0].id.clone())
}

pub async fn purge(client: &Client, zone: &str, urls: Option<Vec<String>>, all: bool) -> Result<()> {
    let zone_id = find_zone_id(client, zone).await?;

    let request = if all {
        PurgeRequest {
            purge_everything: Some(true),
            files: None,
        }
    } else if let Some(files) = urls {
        if files.is_empty() {
            bail!("No URLs provided. Use --url <url> or --all to purge everything.");
        }
        PurgeRequest {
            purge_everything: None,
            files: Some(files),
        }
    } else {
        bail!("Specify --url <url> to purge specific URLs, or --all to purge everything.");
    };

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
        let url = rule
            .targets
            .first()
            .map(|t| t.constraint.value.as_str())
            .unwrap_or("unknown");
        println!("{} [{}] {}", status_icon, rule.priority, url);

        for action in &rule.actions {
            let value_str = match &action.value {
                Some(Value::String(s)) => s.clone(),
                Some(Value::Bool(b)) => b.to_string(),
                Some(Value::Number(n)) => n.to_string(),
                Some(Value::Object(obj)) => {
                    // For complex objects, show a summary
                    serde_json::to_string(obj).unwrap_or_else(|_| "...".to_string())
                }
                Some(other) => other.to_string(),
                None => "on".to_string(),
            };
            println!("  {} = {}", action.id, value_str);
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

pub async fn cache_rules(client: &Client, zone: &str) -> Result<()> {
    let zone_id = find_zone_id(client, zone).await?;

    // First get the list of rulesets to find the cache settings ruleset
    let path = format!("/zones/{}/rulesets?phase=http_request_cache_settings", zone_id);
    let response: ApiResponse<Vec<Ruleset>> = client.get(&path).await?;

    if response.result.is_empty() {
        println!("No cache rules configured");
        return Ok(());
    }

    // Get the full ruleset with rules
    for ruleset in response.result {
        let ruleset_path = format!("/zones/{}/rulesets/{}", zone_id, ruleset.id);
        let full_ruleset: ApiResponse<Ruleset> = client.get(&ruleset_path).await?;

        if full_ruleset.result.rules.is_empty() {
            println!("No cache rules in ruleset");
            continue;
        }

        for rule in &full_ruleset.result.rules {
            let status_icon = if rule.enabled { "●" } else { "○" };
            let desc = if rule.description.is_empty() {
                &rule.action
            } else {
                &rule.description
            };
            println!("{} {}", status_icon, desc);
            println!("  Expression: {}", rule.expression);
            println!("  Action: {}", rule.action);
            if let Some(params) = &rule.action_parameters {
                if let Some(obj) = params.as_object() {
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
            println!();
        }
    }

    Ok(())
}

pub async fn create_rule(client: &Client, zone: &str, name: &str, expression: &str) -> Result<()> {
    let zone_id = find_zone_id(client, zone).await?;

    // Check if a cache settings ruleset already exists
    let list_path = format!("/zones/{}/rulesets", zone_id);
    let all_rulesets: ApiResponse<Vec<Ruleset>> = client.get(&list_path).await?;

    // Filter for cache settings phase
    let existing: Vec<&Ruleset> = all_rulesets
        .result
        .iter()
        .filter(|r| r.phase == "http_request_cache_settings")
        .collect();

    let new_rule = json!({
        "expression": expression,
        "description": name,
        "action": "set_cache_settings",
        "action_parameters": {
            "cache": true,
            "edge_ttl": {
                "mode": "respect_origin"
            },
            "browser_ttl": {
                "mode": "respect_origin"
            }
        },
        "enabled": true
    });

    if let Some(ruleset) = existing.first() {
        // Ruleset exists - fetch full ruleset and add new rule
        let get_path = format!("/zones/{}/rulesets/{}", zone_id, ruleset.id);
        let full_ruleset: ApiResponse<Ruleset> = client.get(&get_path).await?;

        // Build updated rules list: existing rules + new rule
        let mut rules: Vec<Value> = full_ruleset
            .result
            .rules
            .iter()
            .map(|r| {
                json!({
                    "id": r.id,
                    "expression": r.expression,
                    "description": r.description,
                    "action": r.action,
                    "action_parameters": r.action_parameters,
                    "enabled": r.enabled
                })
            })
            .collect();
        rules.push(new_rule);

        let update_body = json!({ "rules": rules });
        let update_path = format!("/zones/{}/rulesets/{}", zone_id, ruleset.id);
        let response: ApiResponse<Ruleset> = client.put(&update_path, &update_body).await?;

        println!("Added cache rule to existing ruleset (ID: {})", ruleset.id);
        if let Some(rule) = response.result.rules.last() {
            println!("  Rule ID: {}", rule.id);
            println!("  Description: {}", rule.description);
            println!("  Expression: {}", rule.expression);
        }
    } else {
        // No ruleset exists - create new one
        let create_body = json!({
            "name": "Cache Rules",
            "kind": "zone",
            "phase": "http_request_cache_settings",
            "rules": [new_rule]
        });

        let create_path = format!("/zones/{}/rulesets", zone_id);
        let response: ApiResponse<Ruleset> = client.post(&create_path, &create_body).await?;

        println!("Created cache rule ruleset (ID: {})", response.result.id);
        if let Some(rule) = response.result.rules.first() {
            println!("  Rule ID: {}", rule.id);
            println!("  Description: {}", rule.description);
            println!("  Expression: {}", rule.expression);
        }
    }

    Ok(())
}

pub async fn update_rule(client: &Client, zone: &str, name: &str, expression: &str) -> Result<()> {
    let zone_id = find_zone_id(client, zone).await?;

    // Get all rulesets for the zone
    let list_path = format!("/zones/{}/rulesets", zone_id);
    let all_rulesets: ApiResponse<Vec<Ruleset>> = client.get(&list_path).await?;

    // Find the cache settings ruleset
    let cache_ruleset = all_rulesets
        .result
        .iter()
        .find(|r| r.phase == "http_request_cache_settings");

    let ruleset = match cache_ruleset {
        Some(r) => r,
        None => bail!("No cache rules ruleset found for zone '{}'", zone),
    };

    // Get the full ruleset with all rules
    let get_path = format!("/zones/{}/rulesets/{}", zone_id, ruleset.id);
    let full_ruleset: ApiResponse<Ruleset> = client.get(&get_path).await?;

    // Find the rule by name (description field)
    let rule_index = full_ruleset
        .result
        .rules
        .iter()
        .position(|r| r.description == name);

    let rule_idx = match rule_index {
        Some(idx) => idx,
        None => bail!(
            "Rule '{}' not found. Available rules: {}",
            name,
            full_ruleset
                .result
                .rules
                .iter()
                .map(|r| r.description.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    };

    // Build updated rules list with the new expression for the matching rule
    let rules: Vec<Value> = full_ruleset
        .result
        .rules
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let expr = if i == rule_idx { expression } else { &r.expression };
            json!({
                "id": r.id,
                "expression": expr,
                "description": r.description,
                "action": r.action,
                "action_parameters": r.action_parameters,
                "enabled": r.enabled
            })
        })
        .collect();

    let update_body = json!({ "rules": rules });
    let update_path = format!("/zones/{}/rulesets/{}", zone_id, ruleset.id);
    let response: ApiResponse<Ruleset> = client.put(&update_path, &update_body).await?;

    println!("Updated cache rule (ruleset ID: {})", ruleset.id);
    if let Some(rule) = response.result.rules.get(rule_idx) {
        println!("  Rule ID: {}", rule.id);
        println!("  Description: {}", rule.description);
        println!("  Expression: {}", rule.expression);
    }

    Ok(())
}

pub async fn delete_rule(client: &Client, zone: &str, rule_id: &str) -> Result<()> {
    let zone_id = find_zone_id(client, zone).await?;

    // Get all rulesets for the zone
    let list_path = format!("/zones/{}/rulesets", zone_id);
    let all_rulesets: ApiResponse<Vec<Ruleset>> = client.get(&list_path).await?;

    // Find the cache settings ruleset
    let cache_ruleset = all_rulesets
        .result
        .iter()
        .find(|r| r.phase == "http_request_cache_settings");

    let ruleset = match cache_ruleset {
        Some(r) => r,
        None => bail!("No cache rules ruleset found for zone '{}'", zone),
    };

    // Get the full ruleset with all rules
    let get_path = format!("/zones/{}/rulesets/{}", zone_id, ruleset.id);
    let full_ruleset: ApiResponse<Ruleset> = client.get(&get_path).await?;

    // Find the rule by ID
    let rule_to_delete = full_ruleset
        .result
        .rules
        .iter()
        .find(|r| r.id == rule_id);

    let deleted_description = match rule_to_delete {
        Some(r) => r.description.clone(),
        None => bail!(
            "Rule '{}' not found. Available rules:\n{}",
            rule_id,
            full_ruleset
                .result
                .rules
                .iter()
                .map(|r| format!("  {} - {}", r.id, r.description))
                .collect::<Vec<_>>()
                .join("\n")
        ),
    };

    // Build updated rules list excluding the deleted rule
    let rules: Vec<Value> = full_ruleset
        .result
        .rules
        .iter()
        .filter(|r| r.id != rule_id)
        .map(|r| {
            json!({
                "id": r.id,
                "expression": r.expression,
                "description": r.description,
                "action": r.action,
                "action_parameters": r.action_parameters,
                "enabled": r.enabled
            })
        })
        .collect();

    let update_body = json!({ "rules": rules });
    let update_path = format!("/zones/{}/rulesets/{}", zone_id, ruleset.id);
    let _response: ApiResponse<Ruleset> = client.put(&update_path, &update_body).await?;

    println!("Deleted cache rule '{}'", rule_id);
    if !deleted_description.is_empty() {
        println!("  Description: {}", deleted_description);
    }

    Ok(())
}
