use crate::auth::{AuthService, Claims, LoginRequest, LoginResponse};
use crate::cache::Cache;
use crate::config::RateLimitConfig;
use crate::db::Repository;
use crate::error::{AppError, AppResult};
use crate::jobs::JobSender;
use crate::middleware_impls::{
    AuthAwareKeyExtractor, request_context_middleware, request_id_middleware,
};
use crate::models::{CreateUrlRequest, CreateUrlResponse, UrlInfoResponse, PaginatedResponse};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Json, Redirect},
    routing::{get, post, delete},
    Router,
};
use chrono::{Duration, Utc};
use nanoid::nanoid;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration as StdDuration;
use tower_governor::GovernorLayer;
use tower_http::cors::{Any, CorsLayer};
use url::Url as UrlParser;
use validator::Validate;

/// Helper to extract JWT claims from Authorization header
fn extract_claims(headers: &axum::http::HeaderMap, auth_service: &AuthService) -> AppResult<Claims> {
    let auth_header = headers
        .get("Authorization")
        .ok_or_else(|| AppError::Internal("Missing Authorization header".to_string()))?;

    let auth_str = auth_header
        .to_str()
        .map_err(|e| AppError::Internal(format!("Invalid Authorization header: {}", e)))?;

    if !auth_str.starts_with("Bearer ") {
        return Err(AppError::Internal(
            "Authorization header must start with 'Bearer '".to_string(),
        ));
    }

    let token = &auth_str[7..];
    auth_service.validate_token(token)
}

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    pub repository: Repository,
    pub cache: Cache,
    pub auth_service: AuthService,
    pub job_sender: JobSender,
    pub base_url: String,
    pub default_expiry_hours: i64,
    pub short_code_length: usize,
    pub short_code_max_attempts: u32,
    pub cache_enabled: bool,
    pub strict_url_validation: bool,
}

/// Query parameters for listing URLs
#[derive(Debug, Deserialize)]
pub struct ListUrlsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Health check response
#[derive(Debug, Serialize)]
struct HealthCheckResponse {
    pub status: String,
    pub database: HealthStatus,
    pub cache: HealthStatus,
    pub timestamp: chrono::DateTime<Utc>,
}

/// Individual health status
#[derive(Debug, Serialize)]
struct HealthStatus {
    pub status: String,
    pub latency_ms: Option<u64>,
}

/// Health check endpoint
pub async fn health_check(State(state): State<Arc<AppState>>) -> AppResult<impl IntoResponse> {
    let start = std::time::Instant::now();

    // Check database connectivity
    let db_health = match tokio::time::timeout(
        StdDuration::from_secs(5),
        state.repository.pool.acquire(),
    )
    .await
    {
        Ok(Ok(_conn)) => {
            let latency = start.elapsed().as_millis() as u64;
            HealthStatus {
                status: "healthy".to_string(),
                latency_ms: Some(latency),
            }
        }
        Ok(Err(_)) | Err(_) => HealthStatus {
            status: "unhealthy".to_string(),
            latency_ms: None,
        },
    };

    // Check cache connectivity
    let cache_start = std::time::Instant::now();
    let cache_health = match tokio::time::timeout(
        StdDuration::from_secs(5),
        state.cache.ping(),
    )
    .await
    {
        Ok(Ok(_)) => {
            let latency = cache_start.elapsed().as_millis() as u64;
            HealthStatus {
                status: "healthy".to_string(),
                latency_ms: Some(latency),
            }
        }
        Ok(Err(_)) | Err(_) => HealthStatus {
            status: "unhealthy".to_string(),
            latency_ms: None,
        },
    };

    // Determine overall health
    let overall_status = if db_health.status == "healthy" {
        "healthy"
    } else {
        "degraded"
    };

    let response = HealthCheckResponse {
        status: overall_status.to_string(),
        database: db_health,
        cache: cache_health,
        timestamp: Utc::now(),
    };

    Ok(Json(response))
}

/// Create application router
pub fn create_router(state: Arc<AppState>, allowed_origins: Vec<String>, rate_limit_config: RateLimitConfig) -> Router {
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
    let sensitive_routes = Router::new()
        .route("/", post(create_url))
        .route("/login", post(login))
        .route("/{code}", delete(delete_url))
        .route("/_stats", get(get_stats))
        .route("/_list", get(list_urls))
        .layer(governor_layer_strict);

    // Lenient rate limit for public endpoints (GET /{code}, GET /{code}/info)
    let public_routes = Router::new()
        .route("/{code}", get(resolve_url))
        .route("/{code}/info", get(get_url_info))
        .layer(governor_layer_lenient);

    // Health check endpoint (no rate limiting)
    let health_routes = Router::new()
        .route("/_health", get(health_check));

    // Merge routers and apply middleware layers
    let app = sensitive_routes
        .merge(public_routes)
        .merge(health_routes)
        .layer(cors)
        .layer(middleware::from_fn(request_id_middleware))
        .layer(middleware::from_fn(request_context_middleware))
        .with_state(state);
    
    app
}

/// Login to get JWT token
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> AppResult<impl IntoResponse> {
    let user = state
        .repository
        .get_user_by_username(&payload.username)
        .await?
        .ok_or(AppError::UserNotFound(payload.username.clone()))?;

    // Verify password
    bcrypt::verify(&payload.password, &user.password_hash)
        .map_err(|e| AppError::Internal(format!("Password verification failed: {}", e)))?;

    if !user.is_active {
        return Err(AppError::Unauthorized("User account is inactive".to_string()));
    }

    // Generate JWT token using auth service from state
    let token = state.auth_service.generate_token(&user.id.to_string(), &user.username)?;

    Ok(Json(LoginResponse {
        token,
        username: user.username,
    }))
}

/// Create a short URL
pub async fn create_url(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateUrlRequest>,
) -> AppResult<impl IntoResponse> {
    payload.validate().map_err(|e| {
        AppError::InvalidUrl(format!("Validation failed: {}", e))
    })?;

    // Proper URL validation
    if state.strict_url_validation {
        UrlParser::parse(&payload.url)
            .map_err(|_| AppError::InvalidUrl("Invalid URL format".to_string()))?;

        if !payload.url.starts_with("http://") && !payload.url.starts_with("https://") {
            return Err(AppError::InvalidUrl("URL must start with http:// or https://".to_string()));
        }
    }

    // Validate custom code with regex if provided
    if let Some(custom) = &payload.custom_code {
        let code_regex = Regex::new(r"^[a-zA-Z0-9_-]{4,16}$")
            .map_err(|e| AppError::Internal(format!("Invalid regex pattern: {}", e)))?;
        if !code_regex.is_match(custom) {
            return Err(AppError::InvalidUrl(
                "Custom code must be 4-16 alphanumeric characters, underscores, or hyphens".to_string(),
            ));
        }
    }

    // Use custom code or generate a random one
    let short_code = if let Some(custom) = &payload.custom_code {
        if state.repository.short_code_exists(custom).await? {
            return Err(AppError::ShortCodeExists(custom.clone()));
        }
        custom.clone()
    } else {
        generate_short_code(state.short_code_length, state.short_code_max_attempts, &state.repository).await?
    };

    // Calculate expiry
    let expires_at = payload
        .expiry_hours
        .map_or_else(
            || {
                Some(Utc::now() + Duration::hours(state.default_expiry_hours))
            },
            |hours| Some(Utc::now() + Duration::hours(hours)),
        )
        .filter(|&t| hours_from_now(t) >= 0); // Never store already-expired URLs

    // Create URL entry
    let entry = state
        .repository
        .create_url(&short_code, &payload.url, expires_at)
        .await?;

    // Cache new URL if enabled
    if state.cache_enabled {
        let _ = state.cache.set_url(&entry).await;
    }

    let short_url = format!("{}/{}", state.base_url, short_code);

    let response = CreateUrlResponse {
        short_code,
        short_url,
        original_url: entry.original_url,
        expires_at: entry.expires_at,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// Resolve a short URL and redirect
pub async fn resolve_url(
    State(state): State<Arc<AppState>>,
    Path(code): Path<String>,
) -> AppResult<impl IntoResponse> {
    // Check cache first if enabled
    if state.cache_enabled {
        if let Some(entry) = state.cache.get_url(&code).await? {
            return handle_url_resolution(&state, &entry).await;
        }
    }

    // Cache miss - check database
    let entry = state
        .repository
        .get_url_by_short_code(&code)
        .await?
        .ok_or(AppError::UrlNotFound(code.clone()))?;

    // Check if expired
    if let Some(expires_at) = entry.expires_at {
        if expires_at < Utc::now() {
            return Err(AppError::UrlNotFound(code));
        }
    }

    // Cache for future requests if enabled
    if state.cache_enabled {
        let _ = state.cache.set_url(&entry).await;
    }

    handle_url_resolution(&state, &entry).await
}

/// Handle actual URL resolution (increment click count and redirect)
async fn handle_url_resolution(
    state: &Arc<AppState>,
    entry: &crate::models::UrlEntry,
) -> AppResult<Redirect> {
    // Submit click count increment job to worker
    state.job_sender.increment_click_count(entry.short_code.clone());

    // Invalidate cache entry asynchronously
    if state.cache_enabled {
        let cache = state.cache.clone();
        let code = entry.short_code.clone();
        tokio::spawn(async move {
            if let Err(e) = cache.delete_url(&code).await {
                tracing::error!("Failed to invalidate cache for {}: {:?}", code, e);
            }
        });
    }

    Ok(Redirect::permanent(&entry.original_url))
}

/// Get information about a short URL
pub async fn get_url_info(
    State(state): State<Arc<AppState>>,
    Path(code): Path<String>,
) -> AppResult<impl IntoResponse> {
    // Check cache first if enabled
    if state.cache_enabled {
        if let Some(entry) = state.cache.get_url(&code).await? {
            let response = UrlInfoResponse::from(entry);
            return Ok(Json(response));
        }
    }

    // Cache miss - check database
    let entry = state
        .repository
        .get_url_by_short_code(&code)
        .await?
        .ok_or(AppError::UrlNotFound(code.clone()))?;

    // Cache for future requests if enabled
    if state.cache_enabled {
        let _ = state.cache.set_url(&entry).await;
    }

    let response = UrlInfoResponse::from(entry);
    Ok(Json(response))
}

/// Delete a short URL (requires authentication)
pub async fn delete_url(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(code): Path<String>,
) -> AppResult<impl IntoResponse> {
    let _claims = extract_claims(&headers, &state.auth_service)?;
    let deleted = state.repository.delete_url(&code).await?;

    if !deleted {
        return Err(AppError::UrlNotFound(code));
    }

    // Also remove from cache if enabled
    if state.cache_enabled {
        let _ = state.cache.delete_url(&code).await;
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Get global statistics (requires authentication)
#[derive(Serialize)]
pub struct StatsResponse {
    pub total_urls: i64,
    pub total_clicks: i64,
    pub active_urls: i64,
    pub expired_urls: i64,
}

pub async fn get_stats(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> AppResult<impl IntoResponse> {
    let _claims = extract_claims(&headers, &state.auth_service)?;
    let stats = state.repository.get_stats().await?;

    let response = StatsResponse {
        total_urls: stats.total_urls,
        total_clicks: stats.total_clicks,
        active_urls: stats.active_urls,
        expired_urls: stats.expired_urls,
    };

    Ok(Json(response))
}

/// List all URLs (paginated, requires authentication)
pub async fn list_urls(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListUrlsQuery>,
    headers: axum::http::HeaderMap,
) -> AppResult<impl IntoResponse> {
    let _claims = extract_claims(&headers, &state.auth_service)?;
    let limit = query.limit.unwrap_or(50).min(100); // Max 100
    let offset = query.offset.unwrap_or(0);

    let urls = state.repository.get_all_urls(limit, offset).await?;
    let total = state.repository.count_urls().await?;
    let responses: Vec<UrlInfoResponse> = urls.into_iter().map(Into::into).collect();

    let paginated_response = PaginatedResponse::new(responses, total, limit, offset);
    Ok(Json(paginated_response))
}

/// Generate a unique short code
async fn generate_short_code(
    length: usize,
    max_attempts: u32,
    repository: &Repository,
) -> AppResult<String> {
    const ALPHABET_CHARS: &[char] = &[
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
        'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M',
        'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm',
        'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    ];

    for _ in 0..max_attempts {
        let code = nanoid!(length, ALPHABET_CHARS);

        if !repository.short_code_exists(&code).await? {
            return Ok(code);
        }
    }

    Err(AppError::ShortCodeGenerationFailed)
}

/// Calculate hours from now until a given datetime
fn hours_from_now(dt: chrono::DateTime<Utc>) -> i64 {
    let now = Utc::now();
    let duration = dt.signed_duration_since(now);
    duration.num_hours()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hours_from_now() {
        let now = Utc::now();
        let future = now + Duration::hours(24);
        assert!(hours_from_now(future) > 20);

        let past = now - Duration::hours(24);
        assert!(hours_from_now(past) < -20);
    }
}
