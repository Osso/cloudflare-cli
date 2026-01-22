mod client;
mod commands;
mod config;

use anyhow::Result;
use clap::{Parser, Subcommand};

use client::Client;
use config::Config;

#[derive(Parser)]
#[command(name = "cloudflare")]
#[command(about = "Cloudflare Zero Trust CLI")]
struct Cli {
    /// Site/account to use (from config)
    #[arg(short, long, global = true)]
    site: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage Cloudflare Tunnels
    Tunnels {
        #[command(subcommand)]
        action: TunnelCommands,
    },
    /// Manage Access Applications
    Apps {
        #[command(subcommand)]
        action: AppCommands,
    },
    /// Manage Access Service Tokens
    Tokens {
        #[command(subcommand)]
        action: TokenCommands,
    },
    /// Manage Gateway firewall rules
    Gateway {
        #[command(subcommand)]
        action: GatewayCommands,
    },
    /// Manage Turnstile CAPTCHA widgets
    Turnstile {
        #[command(subcommand)]
        action: TurnstileCommands,
    },
    /// Manage cache (purge)
    Cache {
        #[command(subcommand)]
        action: CacheCommands,
    },
    /// Manage IP access rules (firewall)
    Firewall {
        #[command(subcommand)]
        action: FirewallCommands,
    },
    /// Manage DNS records
    Dns {
        #[command(subcommand)]
        action: DnsCommands,
    },
    /// Manage zones
    Zones {
        #[command(subcommand)]
        action: ZoneCommands,
    },
    /// Manage waiting rooms
    WaitingRoom {
        #[command(subcommand)]
        action: WaitingRoomCommands,
    },
    /// Configure API credentials
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },
}

#[derive(Subcommand)]
enum TunnelCommands {
    /// List all tunnels
    List,
    /// Show domains/ingress rules for a tunnel
    Domains {
        /// Tunnel ID or name
        tunnel: String,
    },
    /// Show full details for a hostname across all tunnels
    Show {
        /// Hostname to look up
        hostname: String,
    },
    /// Add a domain/ingress rule to a tunnel
    AddDomain {
        /// Tunnel ID
        tunnel: String,
        /// Hostname (e.g., example.globalcomixdev.com)
        hostname: String,
        /// Service URL (e.g., http://localhost:8080 or tcp://dragonfly:6379)
        service: String,
        /// Access Application AUD tag (enables access control)
        #[arg(long)]
        access_aud: Option<String>,
    },
    /// Remove a domain/ingress rule from a tunnel
    RemoveDomain {
        /// Tunnel ID
        tunnel: String,
        /// Hostname to remove
        hostname: String,
    },
    /// Get tunnel token for running cloudflared
    Token {
        /// Tunnel name or UUID
        tunnel: String,
    },
    /// Create a new tunnel
    Create {
        /// Tunnel name
        name: String,
    },
}

#[derive(Subcommand)]
enum AppCommands {
    /// List all Access applications
    List,
    /// Show details for an application
    Show {
        /// Application ID
        app_id: String,
        /// Output raw JSON
        #[arg(long)]
        json: bool,
    },
    /// Create a new Access application protected by service token
    Create {
        /// Application name
        name: String,
        /// Domain (e.g., dragonfly.globalcomixdev.com)
        domain: String,
        /// Service token ID for authentication
        #[arg(long)]
        service_token: String,
    },
    /// Delete an Access application
    Delete {
        /// Application ID
        app_id: String,
    },
}

#[derive(Subcommand)]
enum TokenCommands {
    /// List all service tokens
    List,
    /// Create a new service token
    Create {
        /// Token name
        name: String,
        /// Token duration (default: 8760h = 1 year)
        #[arg(long, default_value = "8760h")]
        duration: String,
    },
    /// Delete a service token
    Delete {
        /// Token ID
        token_id: String,
    },
}

#[derive(Subcommand)]
enum GatewayCommands {
    /// List DNS firewall rules
    Dns,
    /// List Network (L4) firewall rules
    Network,
    /// List HTTP firewall rules
    Http,
}

#[derive(Subcommand)]
enum TurnstileCommands {
    /// List all Turnstile widgets
    List,
    /// Show details for a widget (includes secret key)
    Show {
        /// Widget site key
        sitekey: String,
        /// Output raw JSON
        #[arg(long)]
        json: bool,
    },
    /// Create a new Turnstile widget
    Create {
        /// Widget name
        #[arg(long)]
        name: String,
        /// Allowed domains (comma-separated)
        #[arg(long, value_delimiter = ',')]
        domains: Vec<String>,
        /// Widget mode: managed, invisible, or non-interactive
        #[arg(long, default_value = "managed")]
        mode: String,
    },
    /// Delete a Turnstile widget
    Delete {
        /// Widget site key
        sitekey: String,
    },
    /// Rotate the secret key for a widget
    RotateSecret {
        /// Widget site key
        sitekey: String,
        /// Invalidate old secret immediately (default: keep valid for 2 hours)
        #[arg(long)]
        invalidate_immediately: bool,
    },
}

#[derive(Subcommand)]
enum CacheCommands {
    /// List zones in account
    Zones,
    /// Purge cache for a zone
    Purge {
        /// Zone name (domain) or zone ID
        zone: String,
        /// URL(s) to purge (can be specified multiple times)
        #[arg(long, short)]
        url: Option<Vec<String>>,
        /// Purge all cached content
        #[arg(long)]
        all: bool,
    },
    /// List page rules for a zone
    PageRules {
        /// Zone name (domain) or zone ID
        zone: String,
    },
    /// List cache rules (rulesets) for a zone
    Rules {
        /// Zone name (domain) or zone ID
        zone: String,
    },
    /// Create a cache rule for a zone
    CreateRule {
        /// Zone name (domain) or zone ID
        zone: String,
        /// Rule name/description
        #[arg(long)]
        name: String,
        /// Filter expression (e.g., '(http.host eq "example.com")')
        #[arg(long)]
        expression: String,
    },
}

#[derive(Subcommand)]
enum FirewallCommands {
    /// List IP access rules for a zone
    List {
        /// Zone name (domain) or zone ID
        zone: String,
    },
    /// Check if an IP is in any access rules
    Check {
        /// Zone name (domain) or zone ID
        zone: String,
        /// IP address to check
        ip: String,
    },
    /// List WAF custom rules
    Rules {
        /// Zone name (domain) or zone ID
        zone: String,
    },
    /// Show security events (WAF, firewall, etc.)
    Events {
        /// Zone name (domain) or zone ID
        zone: String,
        /// Filter by IP address
        #[arg(long, short)]
        ip: Option<String>,
        /// Hours to look back (default: 24)
        #[arg(long, short = 'H', default_value = "24")]
        hours: u32,
        /// Maximum events to return (default: 25)
        #[arg(long, short, default_value = "25")]
        limit: u32,
    },
    /// Create a rate limiting rule via WAF custom rules
    Ratelimit {
        /// Zone name (domain) or zone ID
        zone: String,
        /// Path to rate limit (e.g., "/manga/browse")
        #[arg(long)]
        path: String,
        /// Maximum requests allowed per period
        #[arg(long)]
        requests: u32,
        /// Time period in seconds
        #[arg(long)]
        period: u32,
        /// Action when limit exceeded: challenge or block (default: challenge)
        #[arg(long, default_value = "challenge")]
        action: String,
    },
}

#[derive(Subcommand)]
enum DnsCommands {
    /// List DNS records for a zone
    List {
        /// Zone name (domain) or zone ID
        zone: String,
    },
    /// Create a DNS record
    Create {
        /// Zone name (domain) or zone ID
        zone: String,
        /// Record type (A, AAAA, CNAME, TXT, etc.)
        #[arg(long, short = 't')]
        record_type: String,
        /// Record name (e.g., "www" or "api.example.com")
        #[arg(long, short)]
        name: String,
        /// Record content (IP address, hostname, etc.)
        #[arg(long, short)]
        content: String,
        /// Proxy through Cloudflare (orange cloud)
        #[arg(long, short, default_value = "true")]
        proxied: bool,
    },
    /// Delete a DNS record
    Delete {
        /// Zone name (domain) or zone ID
        zone: String,
        /// Record name to delete
        name: String,
    },
}

#[derive(Subcommand)]
enum ZoneCommands {
    /// List all zones in account
    List,
    /// Add a new zone
    Add {
        /// Domain name (e.g., example.org)
        domain: String,
    },
    /// Get zone details
    Info {
        /// Zone name (domain) or zone ID
        zone: String,
    },
    /// Delete a zone
    Delete {
        /// Zone name (domain) or zone ID
        zone: String,
    },
}

#[derive(Subcommand)]
enum WaitingRoomCommands {
    /// List waiting rooms for a zone
    List {
        /// Zone name (domain) or zone ID
        zone: String,
    },
    /// Show waiting room details
    Show {
        /// Zone name (domain) or zone ID
        zone: String,
        /// Waiting room ID
        id: String,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Set API credentials for a site
    Set {
        /// Site name (e.g., "personal", "work")
        #[arg(short, long)]
        site: String,
        /// Cloudflare API token
        #[arg(long)]
        token: String,
        /// Cloudflare Account ID
        #[arg(long)]
        account_id: String,
        /// Set as default site
        #[arg(long)]
        default: bool,
    },
    /// List configured sites
    List,
    /// Set default site
    Default {
        /// Site name to set as default
        site: String,
    },
    /// Remove a site from config
    Remove {
        /// Site name to remove
        site: String,
    },
    /// Show config file path
    Path,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Config { action } => match action {
            ConfigCommands::Set {
                site,
                token,
                account_id,
                default,
            } => {
                let mut config = Config::load().unwrap_or_default();
                config.set_site(&site, &token, &account_id, default);
                config.save()?;
                let marker = if config.default_site.as_deref() == Some(&site) {
                    " (default)"
                } else {
                    ""
                };
                println!("Site '{}'{} saved to {}", site, marker, Config::path()?.display());
            }
            ConfigCommands::List => {
                let config = Config::load()?;
                if config.sites.is_empty() {
                    // Check for legacy config
                    if config.api_token.is_some() && config.account_id.is_some() {
                        println!("(legacy) account_id: {}", config.account_id.as_ref().unwrap());
                    } else {
                        println!("No sites configured.");
                    }
                } else {
                    for (name, site_cfg) in &config.sites {
                        let marker = if config.default_site.as_deref() == Some(name) {
                            " *"
                        } else {
                            ""
                        };
                        println!("{}{}", name, marker);
                        println!("  account_id: {}", site_cfg.account_id);
                    }
                }
            }
            ConfigCommands::Default { site } => {
                let mut config = Config::load()?;
                if !config.sites.contains_key(&site) {
                    anyhow::bail!("Site '{}' not found. Available: {}", site, config.list_sites());
                }
                config.default_site = Some(site.clone());
                config.save()?;
                println!("Default site set to '{}'", site);
            }
            ConfigCommands::Remove { site } => {
                let mut config = Config::load()?;
                if config.remove_site(&site) {
                    config.save()?;
                    println!("Site '{}' removed", site);
                } else {
                    println!("Site '{}' not found", site);
                }
            }
            ConfigCommands::Path => {
                println!("{}", Config::path()?.display());
            }
        },
        _ => {
            let config = Config::load()?;
            let site_config = config.get_site(cli.site.as_deref())?;
            let client = Client::new(&site_config)?;

            match cli.command {
                Commands::Tunnels { action } => match action {
                    TunnelCommands::List => commands::tunnels::list(&client).await?,
                    TunnelCommands::Domains { tunnel } => {
                        commands::tunnels::domains(&client, &tunnel).await?
                    }
                    TunnelCommands::Show { hostname } => {
                        commands::tunnels::show(&client, &hostname).await?
                    }
                    TunnelCommands::AddDomain { tunnel, hostname, service, access_aud } => {
                        commands::tunnels::add_domain(&client, &tunnel, &hostname, &service, access_aud.as_deref()).await?
                    }
                    TunnelCommands::RemoveDomain { tunnel, hostname } => {
                        commands::tunnels::remove_domain(&client, &tunnel, &hostname).await?
                    }
                    TunnelCommands::Token { tunnel } => {
                        commands::tunnels::token(&client, &tunnel).await?
                    }
                    TunnelCommands::Create { name } => {
                        commands::tunnels::create(&client, &name).await?
                    }
                },
                Commands::Apps { action } => match action {
                    AppCommands::List => commands::applications::list(&client).await?,
                    AppCommands::Show { app_id, json } => {
                        commands::applications::show(&client, &app_id, json).await?
                    }
                    AppCommands::Create {
                        name,
                        domain,
                        service_token,
                    } => {
                        commands::applications::create(&client, &name, &domain, &service_token)
                            .await?
                    }
                    AppCommands::Delete { app_id } => {
                        commands::applications::delete(&client, &app_id).await?
                    }
                },
                Commands::Tokens { action } => match action {
                    TokenCommands::List => commands::service_tokens::list(&client).await?,
                    TokenCommands::Create { name, duration } => {
                        commands::service_tokens::create(&client, &name, &duration).await?
                    }
                    TokenCommands::Delete { token_id } => {
                        commands::service_tokens::delete(&client, &token_id).await?
                    }
                },
                Commands::Gateway { action } => match action {
                    GatewayCommands::Dns => commands::gateway::dns_rules(&client).await?,
                    GatewayCommands::Network => commands::gateway::network_rules(&client).await?,
                    GatewayCommands::Http => commands::gateway::http_rules(&client).await?,
                },
                Commands::Turnstile { action } => match action {
                    TurnstileCommands::List => commands::turnstile::list(&client).await?,
                    TurnstileCommands::Show { sitekey, json } => {
                        commands::turnstile::show(&client, &sitekey, json).await?
                    }
                    TurnstileCommands::Create { name, domains, mode } => {
                        commands::turnstile::create(&client, &name, domains, &mode).await?
                    }
                    TurnstileCommands::Delete { sitekey } => {
                        commands::turnstile::delete(&client, &sitekey).await?
                    }
                    TurnstileCommands::RotateSecret {
                        sitekey,
                        invalidate_immediately,
                    } => {
                        commands::turnstile::rotate_secret(&client, &sitekey, invalidate_immediately)
                            .await?
                    }
                },
                Commands::Cache { action } => match action {
                    CacheCommands::Zones => commands::cache::list_zones(&client).await?,
                    CacheCommands::Purge { zone, url, all } => {
                        commands::cache::purge(&client, &zone, url, all).await?
                    }
                    CacheCommands::PageRules { zone } => {
                        commands::cache::page_rules(&client, &zone).await?
                    }
                    CacheCommands::Rules { zone } => {
                        commands::cache::cache_rules(&client, &zone).await?
                    }
                    CacheCommands::CreateRule { zone, name, expression } => {
                        commands::cache::create_rule(&client, &zone, &name, &expression).await?
                    }
                },
                Commands::Firewall { action } => match action {
                    FirewallCommands::List { zone } => {
                        commands::firewall::list(&client, &zone).await?
                    }
                    FirewallCommands::Check { zone, ip } => {
                        commands::firewall::check(&client, &zone, &ip).await?
                    }
                    FirewallCommands::Rules { zone } => {
                        commands::firewall::rules(&client, &zone).await?
                    }
                    FirewallCommands::Events {
                        zone,
                        ip,
                        hours,
                        limit,
                    } => {
                        commands::firewall::events(&client, &zone, ip.as_deref(), hours, limit)
                            .await?
                    }
                    FirewallCommands::Ratelimit {
                        zone,
                        path,
                        requests,
                        period,
                        action,
                    } => {
                        commands::firewall::ratelimit(&client, &zone, &path, requests, period, &action)
                            .await?
                    }
                },
                Commands::Dns { action } => match action {
                    DnsCommands::List { zone } => {
                        commands::dns::list(&client, &zone).await?
                    }
                    DnsCommands::Create {
                        zone,
                        record_type,
                        name,
                        content,
                        proxied,
                    } => {
                        commands::dns::create(&client, &zone, &record_type, &name, &content, proxied)
                            .await?
                    }
                    DnsCommands::Delete { zone, name } => {
                        commands::dns::delete(&client, &zone, &name).await?
                    }
                },
                Commands::Zones { action } => match action {
                    ZoneCommands::List => commands::zones::list(&client).await?,
                    ZoneCommands::Add { domain } => {
                        commands::zones::add(&client, &domain).await?
                    }
                    ZoneCommands::Info { zone } => {
                        commands::zones::info(&client, &zone).await?
                    }
                    ZoneCommands::Delete { zone } => {
                        commands::zones::delete(&client, &zone).await?
                    }
                },
                Commands::WaitingRoom { action } => match action {
                    WaitingRoomCommands::List { zone } => {
                        commands::waiting_room::list(&client, &zone).await?
                    }
                    WaitingRoomCommands::Show { zone, id } => {
                        commands::waiting_room::show(&client, &zone, &id).await?
                    }
                },
                Commands::Config { .. } => unreachable!(),
            }
        }
    }

    Ok(())
}
