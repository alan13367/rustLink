use crate::error::AppResult;
use crate::routes::types::{HealthCheckResponse, HealthStatus};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use super::AppState;

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
        timestamp: chrono::Utc::now(),
    };

    Ok(Json(response))
}
