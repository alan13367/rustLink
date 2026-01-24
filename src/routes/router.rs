use crate::config::RateLimitConfig;
use crate::middleware_impls::AuthAwareKeyExtractor;
use axum::middleware;
use axum::routing::{get, post, delete};
use std::sync::Arc;
use tower_governor::GovernorLayer;
use tower_http::cors::{Any, CorsLayer};

use super::admin_handlers;
use super::auth_handlers;
use super::health;
use super::url_handlers;
use super::AppState;

/// Create application router
pub fn create_router(
    state: Arc<AppState>,
    allowed_origins: Vec<String>,
    rate_limit_config: RateLimitConfig,
) -> axum::Router {
    use crate::middleware_impls::{request_context_middleware, request_id_middleware};

    // Configure rate limiting for sensitive endpoints (auth-aware)
    let governor_layer_strict = GovernorLayer::new(
        tower_governor::governor::GovernorConfigBuilder::default()
            .per_millisecond(60000 / rate_limit_config.requests_per_minute)
            .burst_size(rate_limit_config.burst_size)
            .key_extractor(AuthAwareKeyExtractor)
            .finish()
            .expect("Failed to build strict governor config")
    );

    // More lenient limits for public endpoints (auth-aware)
    let governor_layer_lenient = GovernorLayer::new(
        tower_governor::governor::GovernorConfigBuilder::default()
            .per_millisecond(60000 / (rate_limit_config.requests_per_minute * 2))
            .burst_size(rate_limit_config.burst_size * 2)
            .key_extractor(AuthAwareKeyExtractor)
            .finish()
            .expect("Failed to build lenient governor config")
    );

    // Configure CORS with specific origins
    let cors = if allowed_origins.iter().any(|o| o == "*") {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        let origins: Vec<http::HeaderValue> = allowed_origins
            .iter()
            .filter_map(|s| s.parse::<http::HeaderValue>().ok())
            .collect();
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods(Any)
            .allow_headers(Any)
    };

    // Build router with rate limiting using merge
    // Strict rate limit for sensitive endpoints (POST /, POST /login, DELETE /{code}, /_stats, /_list)
    let sensitive_routes = axum::Router::new()
        .route("/", post(url_handlers::create_url))
        .route("/login", post(auth_handlers::login))
        .route("/{code}", delete(admin_handlers::delete_url))
        .route("/_stats", get(admin_handlers::get_stats))
        .route("/_list", get(admin_handlers::list_urls))
        .layer(governor_layer_strict);

    // Lenient rate limit for public endpoints (GET /{code}, GET /{code}/info)
    let public_routes = axum::Router::new()
        .route("/{code}", get(url_handlers::resolve_url))
        .route("/{code}/info", get(url_handlers::get_url_info))
        .layer(governor_layer_lenient);

    // Health check endpoint (no rate limiting)
    let health_routes = axum::Router::new()
        .route("/_health", get(health::health_check));

    // Merge routers and apply middleware layers
    sensitive_routes
        .merge(public_routes)
        .merge(health_routes)
        .layer(cors)
        .layer(middleware::from_fn(request_id_middleware))
        .layer(middleware::from_fn(request_context_middleware))
        .with_state(state)
}
