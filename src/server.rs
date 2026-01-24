//! Server startup, shutdown, and worker spawning logic.
//!
//! This module contains the `run_server` function which handles:
//! - Database and cache initialization
//! - Migration running
//! - Application state creation
//! - Router creation
//! - Server binding and graceful shutdown
//! - Background worker spawning and cleanup

use crate::auth::AuthService;
use crate::cache::Cache;
use crate::config::Config;
use crate::error::{AppError, AppResult};
use crate::jobs::{create_job_channel, Worker};
use crate::routes;
use crate::state;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};

/// Run the web server with the given configuration.
///
/// This function initializes all required components (database, cache, auth service),
/// creates the application state, sets up the router, and starts the server with
/// graceful shutdown handling.
///
/// # Arguments
///
/// * `config` - The application configuration
/// * `addr` - The address to bind the server to (e.g., "127.0.0.1:3000")
/// * `should_migrate` - Whether to run database migrations on startup
///
/// # Returns
///
/// Returns `Ok(())` if the server shuts down gracefully, or an `AppError` if
/// initialization or startup fails.
///
/// # Errors
///
/// This function will return an error if:
/// - Database connection fails
/// - Cache connection fails (non-fatal, continues with warning)
/// - Migration fails
/// - Server binding fails
/// - Server runtime error occurs
pub async fn run_server(config: Config, addr: String, should_migrate: bool) -> AppResult<()> {
    info!("Starting rustLink server...");

    // Initialize database connection pool
    info!("Connecting to database...");
    let repository = crate::db::Repository::new(
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

    let state = Arc::new(state::AppState {
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
    let app = routes::create_router(state, config.cors.allowed_origins, config.rate_limit)?;

    // Start server
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to bind to address {}: {}", addr, e)))?;

    info!("Server listening on {}", addr);
    info!("Base URL: {}", config.url.base_url);

    // Set up graceful shutdown
    let shutdown_signal = create_shutdown_signal();

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

/// Create a future that resolves when a shutdown signal is received.
///
/// On Unix-like systems, this listens for both Ctrl+C (SIGINT) and SIGTERM.
/// On other platforms, it only listens for Ctrl+C.
///
/// # Returns
///
/// A future that resolves when a shutdown signal is received.
///
/// # Panics
///
/// Panics if signal handler installation fails. This is intentional because
/// signal handler failures are unrecoverable system-level errors that indicate
/// the OS cannot deliver shutdown signals, making graceful shutdown impossible.
async fn create_shutdown_signal() {
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
}
