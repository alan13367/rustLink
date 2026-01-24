mod admin;
mod auth;
mod cache;
mod config;
mod db;
mod error;
mod jobs;
mod middleware;
mod middleware_impls;
mod models;
mod routes;
mod server;
mod services;
mod state;
mod util;

use crate::admin::AdminCommands;
use crate::config::Config;
use crate::error::AppResult;
use clap::{Parser, Subcommand};
use tracing::Level;
use tracing_subscriber::EnvFilter;

/// rustLink - A high-performance URL shortener
#[derive(Parser, Debug)]
#[command(name = "rustlink")]
#[command(version = "0.1.0")]
#[command(about = "A high-performance URL shortener", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start the web server
    Server {
        /// Host to bind to (overrides SERVER_HOST env var)
        #[arg(long)]
        host: Option<String>,

        /// Port to bind to (overrides SERVER_PORT env var)
        #[arg(long)]
        port: Option<u16>,

        /// Run migrations on startup
        #[arg(long, default_value_t = true)]
        migrate: bool,
    },

    /// Administrative commands
    Admin {
        #[command(subcommand)]
        admin_command: AdminCommands,
    },
}

#[tokio::main]
async fn main() -> AppResult<()> {
    let cli = Cli::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(Level::INFO.to_string())),
        )
        .init();

    // Load configuration
    let config = Config::from_env()?;

    match cli.command {
        Commands::Server { host, port, migrate } => {
            // Override config with CLI args if provided
            let host = host.unwrap_or_else(|| config.server.host.clone());
            let port = port.unwrap_or(config.server.port);
            let addr = format!("{}:{}", host, port);

            // Re-compute base_url after CLI overrides
            let mut config = config;
            config.url.base_url = format!("http://{}:{}", host, port);

            server::run_server(config, addr, migrate).await
        }
        Commands::Admin { admin_command } => {
            admin::run(config, admin_command).await
        }
    }
}
