use anyhow::Result;
use serde::Deserialize;

use crate::client::Client;
use super::find_zone_id;

#[derive(Debug, Deserialize)]
struct RulesetResponse {
    result: Ruleset,
}

#[derive(Debug, Deserialize)]
struct Ruleset {
    id: String,
    #[serde(default)]
    rules: Vec<RateLimitRule>,
}

#[derive(Debug, Deserialize)]
struct RateLimitRule {
    id: String,
    #[serde(default)]
    description: String,
    expression: String,
    action: String,
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    ratelimit: Option<RateLimitConfig>,
}

#[derive(Debug, Deserialize)]
struct RateLimitConfig {
    #[serde(default)]
    requests_per_period: u32,
    #[serde(default)]
    period: u32,
    #[serde(default)]
    mitigation_timeout: u32,
    #[serde(default)]
    characteristics: Vec<String>,
}

fn is_ruleset_missing(err: &anyhow::Error) -> bool {
    let s = err.to_string();
    s.contains("404") || s.contains("10003") || s.contains("could not find")
}

async fn fetch_ruleset(client: &Client, zone_id: &str) -> Result<Option<Ruleset>> {
    let path = format!("/zones/{}/rulesets/phases/http_ratelimit/entrypoint", zone_id);
    match client.get::<RulesetResponse>(&path).await {
        Ok(r) => Ok(Some(r.result)),
        Err(e) if is_ruleset_missing(&e) => Ok(None),
        Err(e) => Err(e),
    }
}

fn format_rule(rule: &RateLimitRule) {
    let status = if rule.enabled { "●" } else { "○" };
    let action_icon = match rule.action.as_str() {
        "block" => "⛔",
        "managed_challenge" => "🤖",
        "challenge" => "❓",
        "js_challenge" => "🔒",
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
    if let Some(rl) = &rule.ratelimit {
        println!(
            "  Limit: {} req / {}s  (mitigation timeout: {}s)",
            rl.requests_per_period, rl.period, rl.mitigation_timeout
        );
        if !rl.characteristics.is_empty() {
            println!("  Characteristics: {}", rl.characteristics.join(", "));
        }
    }
    println!();
}

pub async fn list(client: &Client, zone: &str) -> Result<()> {
    let zone_id = find_zone_id(client, zone).await?;

    let Some(ruleset) = fetch_ruleset(client, &zone_id).await? else {
        println!("No rate limiting rules configured for zone '{}'", zone);
        return Ok(());
    };

    if ruleset.rules.is_empty() {
        println!("No rate limiting rules configured for zone '{}'", zone);
        return Ok(());
    }

    println!("Rate limiting rules for zone '{}' (ruleset: {}):", zone, ruleset.id);
    println!();

    for rule in &ruleset.rules {
        format_rule(rule);
    }

    Ok(())
}

pub async fn get(client: &Client, zone: &str, rule_id: &str) -> Result<()> {
    let zone_id = find_zone_id(client, zone).await?;

    let ruleset = fetch_ruleset(client, &zone_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("No rate limiting ruleset found for zone '{}'", zone))?;

    let rule = ruleset
        .rules
        .iter()
        .find(|r| r.id == rule_id)
        .ok_or_else(|| anyhow::anyhow!("Rule '{}' not found in zone '{}'", rule_id, zone))?;

    format_rule(rule);

    Ok(())
}
