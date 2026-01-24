//! Administrative command handlers.
//!
//! This module contains CLI command handlers for administrative tasks
//! such as cleaning expired URLs, running migrations, viewing statistics,
//! and pinging the cache server.

use crate::cache::Cache;
use crate::config::Config;
use crate::db::Repository;
use crate::error::AppResult;
use clap::Subcommand;
use tracing::info;

/// Administrative commands available via CLI.
#[derive(Subcommand, Debug)]
pub enum AdminCommands {
    /// Clean expired URLs from the database
    CleanExpired,

    /// Run database migrations
    Migrate,

    /// Show statistics
    Stats,

    /// Ping the cache server
    PingCache,
}

/// Run an administrative command with the given configuration.
///
/// # Arguments
///
/// * `config` - The application configuration
/// * `admin_command` - The admin command to execute
///
/// # Returns
///
/// Returns `Ok(())` if the command succeeds, or an `AppError` if it fails.
pub async fn run(config: Config, admin_command: AdminCommands) -> AppResult<()> {
    match admin_command {
        AdminCommands::CleanExpired => {
            clean_expired(config).await
        }
        AdminCommands::Migrate => {
            migrate(config).await
        }
        AdminCommands::Stats => {
            stats(config).await
        }
        AdminCommands::PingCache => {
            ping_cache(config).await
        }
    }
}

/// Clean expired URLs from the database.
async fn clean_expired(config: Config) -> AppResult<()> {
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

/// Run database migrations.
async fn migrate(config: Config) -> AppResult<()> {
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

/// Display statistics.
async fn stats(config: Config) -> AppResult<()> {
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

/// Ping the cache server.
async fn ping_cache(config: Config) -> AppResult<()> {
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
