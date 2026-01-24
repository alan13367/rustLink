use serde::{Deserialize, Serialize};

/// Query parameters for listing URLs
#[derive(Debug, Deserialize)]
pub struct ListUrlsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthCheckResponse {
    pub status: String,
    pub database: HealthStatus,
    pub cache: HealthStatus,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Individual health status
#[derive(Debug, Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub latency_ms: Option<u64>,
}
