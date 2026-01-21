use crate::cache::Cache;
use crate::db::Repository;
use crate::error::{AppError, AppResult};
use crate::models::{CreateUrlRequest, CreateUrlResponse, UrlInfoResponse};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Redirect},
    routing::{get, post, delete},
    Router,
};
use chrono::{Duration, Utc};
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use validator::Validate;

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    pub repository: Repository,
    pub cache: Cache,
    pub base_url: String,
    pub default_expiry_hours: i64,
    pub short_code_length: usize,
}

/// Query parameters for listing URLs
#[derive(Debug, Deserialize)]
pub struct ListUrlsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Create the application router
pub fn create_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/", post(create_url))
        .route("/:code", get(resolve_url))
        .route("/:code/info", get(get_url_info))
        .route("/:code", delete(delete_url))
        .route("/_stats", get(get_stats))
        .route("/_list", get(list_urls))
        .layer(cors)
        .with_state(state)
}

/// Create a short URL
pub async fn create_url(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateUrlRequest>,
) -> AppResult<impl IntoResponse> {
    payload.validate().map_err(|e| {
        AppError::InvalidUrl(format!("Validation failed: {}", e.to_string()))
    })?;

    // Check if URL is valid format-wise
    if !payload.url.starts_with("http://") && !payload.url.starts_with("https://") {
        return Err(AppError::InvalidUrl("URL must start with http:// or https://".to_string()));
    }

    // Use custom code or generate a random one
    let short_code = if let Some(custom) = &payload.custom_code {
        if state.repository.short_code_exists(custom).await? {
            return Err(AppError::ShortCodeExists(custom.clone()));
        }
        custom.clone()
    } else {
        generate_short_code(state.short_code_length, &state.repository).await?
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
        .filter(|&t| hours_from_now(t) > 0); // Never store already-expired URLs

    // Create the URL entry
    let entry = state
        .repository
        .create_url(&short_code, &payload.url, expires_at)
        .await?;

    // Cache the new URL
    let _ = state.cache.set_url(&entry).await;

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
    // Check cache first
    if let Some(entry) = state.cache.get_url(&code).await? {
        return handle_url_resolution(&state, &entry).await;
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

    // Cache for future requests
    let _ = state.cache.set_url(&entry).await;

    handle_url_resolution(&state, &entry).await
}

/// Handle the actual URL resolution (increment click count and redirect)
async fn handle_url_resolution(
    state: &Arc<AppState>,
    entry: &crate::models::UrlEntry,
) -> AppResult<Redirect> {
    // Increment click count asynchronously (don't block the redirect)
    let repo = state.repository.clone();
    let code = entry.short_code.clone();
    let code_for_cache = code.clone();
    tokio::spawn(async move {
        let _ = repo.increment_click_count(&code).await;
    });

    // Invalidate cache after click count update
    let _ = state.cache.delete_url(&code_for_cache).await;

    Ok(Redirect::permanent(&entry.original_url))
}

/// Get information about a short URL
pub async fn get_url_info(
    State(state): State<Arc<AppState>>,
    Path(code): Path<String>,
) -> AppResult<impl IntoResponse> {
    // Check cache first
    if let Some(entry) = state.cache.get_url(&code).await? {
        let response = UrlInfoResponse::from(entry);
        return Ok(Json(response));
    }

    // Cache miss - check database
    let entry = state
        .repository
        .get_url_by_short_code(&code)
        .await?
        .ok_or(AppError::UrlNotFound(code.clone()))?;

    // Cache for future requests
    let _ = state.cache.set_url(&entry).await;

    let response = UrlInfoResponse::from(entry);
    Ok(Json(response))
}

/// Delete a short URL
pub async fn delete_url(
    State(state): State<Arc<AppState>>,
    Path(code): Path<String>,
) -> AppResult<impl IntoResponse> {
    let deleted = state.repository.delete_url(&code).await?;

    if !deleted {
        return Err(AppError::UrlNotFound(code));
    }

    // Also remove from cache
    let _ = state.cache.delete_url(&code).await;

    Ok(StatusCode::NO_CONTENT)
}

/// Get global statistics
#[derive(Serialize)]
struct StatsResponse {
    total_urls: i64,
    total_clicks: i64,
    active_urls: i64,
    expired_urls: i64,
}

pub async fn get_stats(
    State(state): State<Arc<AppState>>,
) -> AppResult<impl IntoResponse> {
    let stats = state.repository.get_stats().await?;

    let response = StatsResponse {
        total_urls: stats.total_urls,
        total_clicks: stats.total_clicks,
        active_urls: stats.active_urls,
        expired_urls: stats.expired_urls,
    };

    Ok(Json(response))
}

/// List all URLs (paginated)
pub async fn list_urls(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListUrlsQuery>,
) -> AppResult<impl IntoResponse> {
    let limit = query.limit.unwrap_or(50).min(100); // Max 100
    let offset = query.offset.unwrap_or(0);

    let urls = state.repository.get_all_urls(limit, offset).await?;
    let responses: Vec<UrlInfoResponse> = urls.into_iter().map(Into::into).collect();

    Ok(Json(responses))
}

/// Generate a unique short code
async fn generate_short_code(length: usize, repository: &Repository) -> AppResult<String> {
    const MAX_ATTEMPTS: u32 = 10;
    const ALPHABET_CHARS: &[char] = &[
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
        'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M',
        'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm',
        'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    ];

    for _ in 0..MAX_ATTEMPTS {
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
