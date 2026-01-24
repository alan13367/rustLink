use crate::error::{AppError, AppResult};
use crate::models::{CreateUrlRequest, CreateUrlResponse, UrlInfoResponse};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Redirect};
use chrono::{Duration, Utc};
use regex::Regex;
use std::sync::Arc;
use validator::Validate;
use url::Url as UrlParser;

use super::AppState;
use super::helpers::{generate_short_code, hours_from_now};

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
