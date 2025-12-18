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
    /// Manage Gateway firewall rules
    Gateway {
        #[command(subcommand)]
        action: GatewayCommands,
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
}

#[derive(Subcommand)]
enum AppCommands {
    /// List all Access applications
    List,
    /// Show details for an application
    Show {
        /// Application ID
        app_id: String,
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
enum ConfigCommands {
    /// Set API credentials
    Set {
        /// Cloudflare API token
        #[arg(long)]
        token: String,
        /// Cloudflare Account ID
        #[arg(long)]
        account_id: String,
    },
    /// Show config file path
    Path,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Config { action } => match action {
            ConfigCommands::Set { token, account_id } => {
                let config = Config {
                    api_token: token,
                    account_id,
                };
                config.save()?;
                println!("Config saved to {}", Config::path()?.display());
            }
            ConfigCommands::Path => {
                println!("{}", Config::path()?.display());
            }
        },
        _ => {
            let config = Config::load()?;
            let client = Client::new(&config)?;

            match cli.command {
                Commands::Tunnels { action } => match action {
                    TunnelCommands::List => commands::tunnels::list(&client).await?,
                    TunnelCommands::Domains { tunnel } => {
                        commands::tunnels::domains(&client, &tunnel).await?
                    }
                },
                Commands::Apps { action } => match action {
                    AppCommands::List => commands::applications::list(&client).await?,
                    AppCommands::Show { app_id } => {
                        commands::applications::show(&client, &app_id).await?
                    }
                },
                Commands::Gateway { action } => match action {
                    GatewayCommands::Dns => commands::gateway::dns_rules(&client).await?,
                    GatewayCommands::Network => commands::gateway::network_rules(&client).await?,
                    GatewayCommands::Http => commands::gateway::http_rules(&client).await?,
                },
                Commands::Config { .. } => unreachable!(),
            }
        }
    }

    Ok(())
}
