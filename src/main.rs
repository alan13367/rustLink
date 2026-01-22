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

use crate::auth::AuthService;
use crate::cache::Cache;
use crate::config::Config;
use crate::db::Repository;
use crate::error::{AppError, AppResult};
use crate::jobs::{create_job_channel, Worker};
use clap::{Parser, Subcommand};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info, Level};
use tracing_subscriber::EnvFilter;

#[cfg(unix)]
use tokio::signal::unix;

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

#[derive(Subcommand, Debug)]
enum AdminCommands {
    /// Clean expired URLs from the database
    CleanExpired,

    /// Run database migrations
    Migrate,

    /// Show statistics
    Stats,

    /// Ping the cache server
    PingCache,
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

            run_server(config, addr, migrate).await
        }
        Commands::Admin { admin_command } => match admin_command {
            AdminCommands::CleanExpired => {
                run_admin_cleanup(config).await
            }
            AdminCommands::Migrate => {
                run_admin_migrate(config).await
            }
            AdminCommands::Stats => {
                run_admin_stats(config).await
            }
            AdminCommands::PingCache => {
                run_admin_ping_cache(config).await
            }
        },
    }
}

/// Run the web server
async fn run_server(
    config: Config,
    addr: String,
    should_migrate: bool,
) -> AppResult<()> {
    info!("Starting rustLink server...");

    // Initialize database connection pool
    info!("Connecting to database...");
    let repository = Repository::new(
        &config.database.url,
        config.database.max_connections,
        config.database.min_connections,
        config.database.acquire_timeout_seconds,
    )
    .await?;

    // Run migrations if requested
    if should_migrate {
        info!("Running database migrations...");
        repository.run_migrations().await?;
        info!("Migrations completed successfully");
    }

    // Initialize cache
    info!("Connecting to cache...");
    let cache = Cache::new(
        &config.cache.url,
        config.cache.max_connections,
        config.cache.default_ttl_seconds,
    )
    .await?;

    // Ping cache to verify connection
    match cache.ping().await {
        Ok(_) => info!("Cache connection verified"),
        Err(e) => {
            tracing::warn!("Cache ping failed: {}. Continuing without cache.", e);
        }
    }

    // Create application state
    let auth_service = AuthService::new(
        config.auth.jwt_secret.clone(),
        config.auth.jwt_expiration_hours,
    );

    // Create background job worker
    let (job_sender, job_receiver) = create_job_channel();
    let worker = Worker::new(repository.clone(), job_receiver);

    // Start background worker in separate task
    let worker_handle = tokio::spawn(worker.run());

    let state = Arc::new(routes::AppState {
        repository,
        cache,
        auth_service,
        job_sender,
        base_url: config.url.base_url.clone(),
        default_expiry_hours: config.url.default_expiry_hours,
        short_code_length: config.url.short_code_length,
        short_code_max_attempts: config.url.short_code_max_attempts,
        cache_enabled: config.url.cache_enabled,
        strict_url_validation: config.url.strict_url_validation,
    });

    // Create router
    let app = routes::create_router(state, config.cors.allowed_origins, config.rate_limit);

    // Start server
    let listener = TcpListener::bind(&addr).await.map_err(|e| {
        AppError::Internal(format!("Failed to bind to address {}: {}", addr, e))
    })?;

    info!("Server listening on {}", addr);
    info!("Base URL: {}", config.url.base_url);

    // Set up graceful shutdown
    let shutdown_signal = async {
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(unix)]
        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }

        #[cfg(not(unix))]
        ctrl_c.await;
    };

    // Run server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await
        .map_err(|e| AppError::Internal(format!("Server error: {}", e)))?;

    // Wait for background worker to finish
    worker_handle.await.unwrap_or_else(|e| {
        error!("Worker task failed: {:?}", e);
    });

    info!("Server shutdown complete");
    Ok(())
}

/// Run the admin cleanup command
async fn run_admin_cleanup(config: Config) -> AppResult<()> {
    info!("Cleaning expired URLs...");

    let repository = Repository::new(
        &config.database.url,
        config.database.max_connections,
        config.database.min_connections,
        config.database.acquire_timeout_seconds,
    )
    .await?;

    let deleted_count = repository.delete_expired_urls().await?;

    info!("Deleted {} expired URL(s)", deleted_count);
    Ok(())
}

/// Run the admin migrate command
async fn run_admin_migrate(config: Config) -> AppResult<()> {
    info!("Running database migrations...");

    let repository = Repository::new(
        &config.database.url,
        config.database.max_connections,
        config.database.min_connections,
        config.database.acquire_timeout_seconds,
    )
    .await?;

    repository.run_migrations().await?;

    info!("Migrations completed successfully");
    Ok(())
}

/// Run the admin stats command
async fn run_admin_stats(config: Config) -> AppResult<()> {
    info!("Fetching statistics...");

    let repository = Repository::new(
        &config.database.url,
        config.database.max_connections,
        config.database.min_connections,
        config.database.acquire_timeout_seconds,
    )
    .await?;

    let stats = repository.get_stats().await?;

    println!("\n=== rustLink Statistics ===");
    println!("Total URLs:      {}", stats.total_urls);
    println!("Total Clicks:    {}", stats.total_clicks);
    println!("Active URLs:     {}", stats.active_urls);
    println!("Expired URLs:    {}", stats.expired_urls);
    println!();

    Ok(())
}

/// Run the admin ping-cache command
async fn run_admin_ping_cache(config: Config) -> AppResult<()> {
    info!("Pinging cache server...");

    let cache = Cache::new(
        &config.cache.url,
        config.cache.max_connections,
        config.cache.default_ttl_seconds,
    )
    .await?;

    let response = cache.ping().await?;

    info!("Cache server responded: {}", response);

    Ok(())
}
