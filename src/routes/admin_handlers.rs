use crate::error::{AppError, AppResult};
use crate::models::{PaginatedResponse, StatsResponse, UrlInfoResponse};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json};
use std::sync::Arc;

use super::AppState;
use super::helpers::extract_claims;
use super::types::ListUrlsQuery;

/// Delete a short URL (requires authentication)
pub async fn delete_url(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
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
pub async fn get_stats(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
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
    headers: HeaderMap,
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
