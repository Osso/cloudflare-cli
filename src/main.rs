#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

mod client;
mod commands;
mod config;
mod dispatch;

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
    /// Manage Web Analytics (RUM/beacon.min.js)
    Rum {
        #[command(subcommand)]
        action: RumCommands,
    },
    /// Rate limiting rules (http_ratelimit phase)
    RateLimiting {
        #[command(subcommand)]
        action: RateLimitingCommands,
    },
    /// HTTP traffic analytics
    Analytics {
        #[command(subcommand)]
        action: AnalyticsCommands,
    },
    /// Abuse reports (DMCA, etc.)
    Abuse {
        #[command(subcommand)]
        action: AbuseCommands,
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
    /// Set the TLS certificate hostname for an existing ingress rule
    SetOriginServerName {
        /// Tunnel ID or name
        tunnel: String,
        /// Existing ingress hostname
        hostname: String,
        /// Hostname expected in the origin certificate
        origin_server_name: String,
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
    /// Add a hostname to an Access application
    AddHostname {
        /// Application ID
        app_id: String,
        /// Hostname to add
        hostname: String,
    },
    /// Remove a hostname from an Access application
    RemoveHostname {
        /// Application ID
        app_id: String,
        /// Hostname to remove
        hostname: String,
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
        /// Bypass cache instead of enabling it
        #[arg(long)]
        bypass: bool,
    },
    /// Update an existing cache rule's expression
    UpdateRule {
        /// Zone name (domain) or zone ID
        zone: String,
        /// Rule name/description to find
        #[arg(long)]
        name: String,
        /// New filter expression
        #[arg(long)]
        expression: String,
    },
    /// Delete a cache rule by ID
    DeleteRule {
        /// Zone name (domain) or zone ID
        zone: String,
        /// Rule ID to delete
        rule_id: String,
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
    /// Block an IP address or CIDR range
    Block {
        /// Zone name (domain) or zone ID
        zone: String,
        /// IP address or CIDR range to block (e.g., 1.2.3.4 or 1.2.3.0/24)
        ip: String,
        /// Optional note for the block rule
        #[arg(long, short)]
        note: Option<String>,
    },
    /// Unblock an IP address (remove access rule)
    Unblock {
        /// Zone name (domain) or zone ID
        zone: String,
        /// IP address or CIDR range to unblock
        ip: String,
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
        /// Proxy through Cloudflare (orange cloud) [default]
        #[arg(long, short, conflicts_with = "no_proxy")]
        proxied: bool,
        /// Do not proxy through Cloudflare (grey cloud)
        #[arg(long, conflicts_with = "proxied")]
        no_proxy: bool,
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
enum RumCommands {
    /// List Web Analytics sites
    List,
    /// Show site details
    Info {
        /// Web Analytics site (host or site_tag)
        #[arg(name = "SITE")]
        rum_site: String,
    },
    /// Disable auto-install (stops beacon.min.js injection)
    Disable {
        /// Web Analytics site (host or site_tag)
        #[arg(name = "SITE")]
        rum_site: String,
    },
    /// Enable auto-install (injects beacon.min.js)
    Enable {
        /// Web Analytics site (host or site_tag)
        #[arg(name = "SITE")]
        rum_site: String,
    },
    /// Delete a Web Analytics site
    Delete {
        /// Web Analytics site (host or site_tag)
        #[arg(name = "SITE")]
        rum_site: String,
    },
}

#[derive(Subcommand)]
enum RateLimitingCommands {
    /// List all rate limiting rules for a zone
    List {
        /// Zone name (domain) or zone ID
        zone: String,
    },
    /// Get details of a specific rate limiting rule
    Get {
        /// Zone name (domain) or zone ID
        zone: String,
        /// Rule ID
        rule_id: String,
    },
}

#[derive(Subcommand)]
enum AnalyticsCommands {
    /// Show HTTP status code breakdown by day
    StatusCodes {
        /// Zone name (domain) or zone ID
        zone: String,
        /// Number of days to look back (default: 1, max: 30)
        #[arg(long, short, default_value = "1")]
        days: u32,
    },
}

#[derive(Subcommand)]
enum AbuseCommands {
    /// List abuse reports
    List,
    /// Show details of an abuse report
    Show {
        /// Report ID (e.g. eebcab2542155a49)
        report_id: String,
        /// Output raw JSON
        #[arg(long)]
        json: bool,
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

#[cfg_attr(coverage_nightly, coverage(off))]
fn config_set(site: String, token: String, account_id: String, default: bool) -> Result<()> {
    let mut config = Config::load().unwrap_or_default();
    config.set_site(&site, &token, &account_id, default);
    config.save()?;
    let marker = if config.default_site.as_deref() == Some(&site) {
        " (default)"
    } else {
        ""
    };
    println!(
        "Site '{}'{} saved to {}",
        site,
        marker,
        Config::path()?.display()
    );
    Ok(())
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn config_list() -> Result<()> {
    let config = Config::load()?;
    if config.sites.is_empty() {
        if config.api_token.is_some() && config.account_id.is_some() {
            println!(
                "(legacy) account_id: {}",
                config.account_id.as_ref().unwrap()
            );
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
    Ok(())
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn config_set_default(site: String) -> Result<()> {
    let mut config = Config::load()?;
    if !config.sites.contains_key(&site) {
        anyhow::bail!(
            "Site '{}' not found. Available: {}",
            site,
            config.list_sites()
        );
    }
    config.default_site = Some(site.clone());
    config.save()?;
    println!("Default site set to '{}'", site);
    Ok(())
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn config_remove(site: String) -> Result<()> {
    let mut config = Config::load()?;
    if config.remove_site(&site) {
        config.save()?;
        println!("Site '{}' removed", site);
    } else {
        println!("Site '{}' not found", site);
    }
    Ok(())
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn handle_config(action: ConfigCommands) -> Result<()> {
    match action {
        ConfigCommands::Set {
            site,
            token,
            account_id,
            default,
        } => config_set(site, token, account_id, default),
        ConfigCommands::List => config_list(),
        ConfigCommands::Default { site } => config_set_default(site),
        ConfigCommands::Remove { site } => config_remove(site),
        ConfigCommands::Path => {
            println!("{}", Config::path()?.display());
            Ok(())
        }
    }
}

#[tokio::main]
#[cfg_attr(coverage_nightly, coverage(off))]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Config { action } => handle_config(action),
        command => {
            let config = Config::load()?;
            let site_config = config.get_site(cli.site.as_deref())?;
            let client = Client::new(&site_config)?;
            dispatch::dispatch_command(command, &client).await
        }
    }
}
