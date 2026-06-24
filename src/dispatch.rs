use anyhow::{Result, bail};

use crate::client::Client;
use crate::commands;
use crate::{
    AnalyticsCommands, AppCommands, CacheCommands, Commands, DnsCommands, FirewallCommands,
    GatewayCommands, RateLimitingCommands, RumCommands, TokenCommands, TunnelCommands,
    TurnstileCommands, WaitingRoomCommands, ZoneCommands,
};

#[cfg_attr(coverage_nightly, coverage(off))]
pub async fn dispatch_command(command: Commands, client: &Client) -> Result<()> {
    match command {
        Commands::Tunnels { action } => dispatch_tunnels(action, client).await,
        Commands::Apps { action } => dispatch_apps(action, client).await,
        Commands::Tokens { action } => dispatch_tokens(action, client).await,
        Commands::Gateway { action } => dispatch_gateway(action, client).await,
        Commands::Turnstile { action } => dispatch_turnstile(action, client).await,
        Commands::Cache { action } => dispatch_cache(action, client).await,
        Commands::Firewall { action } => dispatch_firewall(action, client).await,
        Commands::Dns { action } => dispatch_dns(action, client).await,
        Commands::Zones { action } => dispatch_zones(action, client).await,
        Commands::WaitingRoom { action } => dispatch_waiting_room(action, client).await,
        Commands::Rum { action } => dispatch_rum(action, client).await,
        Commands::RateLimiting { action } => dispatch_rate_limiting(action, client).await,
        Commands::Analytics { action } => dispatch_analytics(action, client).await,
        Commands::Config { .. } => unreachable!(),
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
async fn dispatch_tunnels(action: TunnelCommands, client: &Client) -> Result<()> {
    match action {
        TunnelCommands::List => commands::tunnels::list(client).await,
        TunnelCommands::Domains { tunnel } => commands::tunnels::domains(client, &tunnel).await,
        TunnelCommands::Show { hostname } => commands::tunnels::show(client, &hostname).await,
        TunnelCommands::AddDomain {
            tunnel,
            hostname,
            service,
            access_aud,
        } => {
            commands::tunnels::add_domain(
                client,
                &tunnel,
                &hostname,
                &service,
                access_aud.as_deref(),
            )
            .await
        }
        TunnelCommands::RemoveDomain { tunnel, hostname } => {
            commands::tunnels::remove_domain(client, &tunnel, &hostname).await
        }
        TunnelCommands::Token { tunnel } => commands::tunnels::token(client, &tunnel).await,
        TunnelCommands::Create { name } => commands::tunnels::create(client, &name).await,
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
async fn dispatch_apps(action: AppCommands, client: &Client) -> Result<()> {
    match action {
        AppCommands::List => commands::applications::list(client).await,
        AppCommands::Show { app_id, json } => {
            commands::applications::show(client, &app_id, json).await
        }
        AppCommands::Create {
            name,
            domain,
            service_token,
        } => commands::applications::create(client, &name, &domain, &service_token).await,
        AppCommands::Delete { app_id } => commands::applications::delete(client, &app_id).await,
        AppCommands::AddHostname { app_id, hostname } => {
            commands::applications::add_hostname(client, &app_id, &hostname).await
        }
        AppCommands::RemoveHostname { app_id, hostname } => {
            commands::applications::remove_hostname(client, &app_id, &hostname).await
        }
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
async fn dispatch_tokens(action: TokenCommands, client: &Client) -> Result<()> {
    match action {
        TokenCommands::List => commands::service_tokens::list(client).await,
        TokenCommands::Create { name, duration } => {
            commands::service_tokens::create(client, &name, &duration).await
        }
        TokenCommands::Delete { token_id } => {
            commands::service_tokens::delete(client, &token_id).await
        }
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
async fn dispatch_gateway(action: GatewayCommands, client: &Client) -> Result<()> {
    match action {
        GatewayCommands::Dns => commands::gateway::dns_rules(client).await,
        GatewayCommands::Network => commands::gateway::network_rules(client).await,
        GatewayCommands::Http => commands::gateway::http_rules(client).await,
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
async fn dispatch_turnstile(action: TurnstileCommands, client: &Client) -> Result<()> {
    match action {
        TurnstileCommands::List => commands::turnstile::list(client).await,
        TurnstileCommands::Show { sitekey, json } => {
            commands::turnstile::show(client, &sitekey, json).await
        }
        TurnstileCommands::Create {
            name,
            domains,
            mode,
        } => commands::turnstile::create(client, &name, domains, &mode).await,
        TurnstileCommands::Delete { sitekey } => {
            commands::turnstile::delete(client, &sitekey).await
        }
        TurnstileCommands::RotateSecret {
            sitekey,
            invalidate_immediately,
        } => commands::turnstile::rotate_secret(client, &sitekey, invalidate_immediately).await,
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
async fn dispatch_cache(action: CacheCommands, client: &Client) -> Result<()> {
    match action {
        CacheCommands::Zones => commands::cache::list_zones(client).await,
        CacheCommands::Purge { zone, url, all } => {
            commands::cache::purge(client, &zone, url, all).await
        }
        CacheCommands::PageRules { zone } => commands::cache::page_rules(client, &zone).await,
        CacheCommands::Rules { zone } => commands::cache::cache_rules(client, &zone).await,
        CacheCommands::CreateRule {
            zone,
            name,
            expression,
            bypass,
        } => commands::cache::create_rule(client, &zone, &name, &expression, bypass).await,
        CacheCommands::UpdateRule {
            zone,
            name,
            expression,
        } => commands::cache::update_rule(client, &zone, &name, &expression).await,
        CacheCommands::DeleteRule { zone, rule_id } => {
            commands::cache::delete_rule(client, &zone, &rule_id).await
        }
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
async fn dispatch_firewall(action: FirewallCommands, client: &Client) -> Result<()> {
    match action {
        FirewallCommands::List { zone } => commands::firewall::list(client, &zone).await,
        FirewallCommands::Check { zone, ip } => commands::firewall::check(client, &zone, &ip).await,
        FirewallCommands::Rules { zone } => commands::firewall::rules(client, &zone).await,
        FirewallCommands::Events {
            zone,
            ip,
            hours,
            limit,
        } => commands::firewall::events(client, &zone, ip.as_deref(), hours, limit).await,
        FirewallCommands::Ratelimit {
            zone,
            path,
            requests,
            period,
            action,
        } => commands::firewall::ratelimit(client, &zone, &path, requests, period, &action).await,
        FirewallCommands::Block { zone, ip, note } => {
            commands::firewall::block(client, &zone, &ip, note.as_deref()).await
        }
        FirewallCommands::Unblock { zone, ip } => {
            commands::firewall::unblock(client, &zone, &ip).await
        }
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
async fn dispatch_dns(action: DnsCommands, client: &Client) -> Result<()> {
    match action {
        DnsCommands::List { zone } => commands::dns::list(client, &zone).await,
        DnsCommands::Create {
            zone,
            record_type,
            name,
            content,
            proxied,
            no_proxy,
        } => {
            let use_proxied = if no_proxy {
                false
            } else if proxied {
                true
            } else {
                true
            };
            commands::dns::create(client, &zone, &record_type, &name, &content, use_proxied).await
        }
        DnsCommands::Delete { zone, name } => commands::dns::delete(client, &zone, &name).await,
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
async fn dispatch_zones(action: ZoneCommands, client: &Client) -> Result<()> {
    match action {
        ZoneCommands::List => commands::zones::list(client).await,
        ZoneCommands::Add { domain } => commands::zones::add(client, &domain).await,
        ZoneCommands::Info { zone } => commands::zones::info(client, &zone).await,
        ZoneCommands::Delete { zone } => commands::zones::delete(client, &zone).await,
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
async fn dispatch_waiting_room(action: WaitingRoomCommands, client: &Client) -> Result<()> {
    match action {
        WaitingRoomCommands::List { zone } => commands::waiting_room::list(client, &zone).await,
        WaitingRoomCommands::Show { zone, id } => {
            commands::waiting_room::show(client, &zone, &id).await
        }
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
async fn dispatch_rum(action: RumCommands, client: &Client) -> Result<()> {
    match action {
        RumCommands::List => commands::rum::list(client).await,
        RumCommands::Info { rum_site } => commands::rum::info(client, &rum_site).await,
        RumCommands::Disable { rum_site } => commands::rum::disable(client, &rum_site).await,
        RumCommands::Enable { rum_site } => commands::rum::enable(client, &rum_site).await,
        RumCommands::Delete { rum_site } => commands::rum::delete(client, &rum_site).await,
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
async fn dispatch_analytics(action: AnalyticsCommands, client: &Client) -> Result<()> {
    match action {
        AnalyticsCommands::StatusCodes { zone, days } => {
            if days > 30 {
                bail!("--days must be 30 or less");
            }
            commands::analytics::status_codes(client, &zone, days).await
        }
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
async fn dispatch_rate_limiting(action: RateLimitingCommands, client: &Client) -> Result<()> {
    match action {
        RateLimitingCommands::List { zone } => commands::rate_limiting::list(client, &zone).await,
        RateLimitingCommands::Get { zone, rule_id } => {
            commands::rate_limiting::get(client, &zone, &rule_id).await
        }
    }
}
