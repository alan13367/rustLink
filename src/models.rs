use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

/// URL entry in the database
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UrlEntry {
    pub id: i64,
    pub short_code: String,
    pub original_url: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub click_count: i64,
    pub last_clicked_at: Option<DateTime<Utc>>,
}

/// Request to create a short URL
#[derive(Debug, Deserialize, Validate)]
pub struct CreateUrlRequest {
    #[validate(url(message = "Must be a valid URL"))]
    pub url: String,

    #[validate(range(
        min = 1,
        max = 87600,
        message = "Expiry must be between 1 and 87600 hours"
    ))]
    pub expiry_hours: Option<i64>,

    #[validate(length(min = 4, max = 16, message = "Custom code must be 4-16 characters"))]
    pub custom_code: Option<String>,
}

/// Response after creating a short URL
#[derive(Debug, Serialize)]
pub struct CreateUrlResponse {
    pub short_code: String,
    pub short_url: String,
    pub original_url: String,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Response for URL info
#[derive(Debug, Serialize)]
pub struct UrlInfoResponse {
    pub short_code: String,
    pub original_url: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub click_count: i64,
    pub last_clicked_at: Option<DateTime<Utc>>,
}

impl From<UrlEntry> for UrlInfoResponse {
    fn from(entry: UrlEntry) -> Self {
        UrlInfoResponse {
            short_code: entry.short_code,
            original_url: entry.original_url,
            created_at: entry.created_at,
            expires_at: entry.expires_at,
            click_count: entry.click_count,
            last_clicked_at: entry.last_clicked_at,
        }
    }
}

/// Statistics summary
#[derive(Debug, Serialize)]
#[allow(dead_code)]
#[deprecated(note = "TODO: Use for admin endpoint with filtering")]
pub struct StatsResponse {
    pub total_urls: i64,
    pub total_clicks: i64,
    pub active_urls: i64,
    pub expired_urls: i64,
}

/// Error response
#[derive(Debug, Serialize)]
#[allow(dead_code)]
#[deprecated(note = "TODO: Use for custom error formatting")]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
#[deprecated(note = "TODO: Implement PUT /{code} endpoint for expiry updates")]
pub struct UpdateExpiryRequest {
    pub expiry_hours: i64,
}
