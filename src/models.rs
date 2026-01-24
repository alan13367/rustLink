//! Data models for the rustLink API.
//!
//! This module contains all request and response types used by the API,
//! with validation rules and OpenAPI schema definitions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use validator::Validate;

/// URL entry in the database
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct UrlEntry {
    /// Unique database ID
    pub id: i64,
    /// Short code for the URL (e.g., "abc123")
    pub short_code: String,
    /// Original long URL
    pub original_url: String,
    /// When the short URL was created
    pub created_at: DateTime<Utc>,
    /// When the short URL expires (if set)
    pub expires_at: Option<DateTime<Utc>>,
    /// Number of times the short URL has been accessed
    pub click_count: i64,
    /// When the URL was last accessed
    pub last_clicked_at: Option<DateTime<Utc>>,
}

/// Request to create a short URL
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateUrlRequest {
    /// The URL to shorten (must be a valid HTTP/HTTPS URL)
    #[validate(url(message = "Must be a valid URL"))]
    #[schema(example = "https://example.com/very/long/path")]
    pub url: String,

    /// Optional expiry time in hours (1-87600)
    #[validate(range(
        min = 1,
        max = 87600,
        message = "Expiry must be between 1 and 87600 hours"
    ))]
    #[schema(example = 720)]
    pub expiry_hours: Option<i64>,

    /// Optional custom short code (4-16 alphanumeric characters)
    #[validate(length(min = 4, max = 16, message = "Custom code must be 4-16 characters"))]
    #[schema(example = "mycustomcode")]
    pub custom_code: Option<String>,
}

/// Response after creating a short URL
#[derive(Debug, Serialize, ToSchema)]
pub struct CreateUrlResponse {
    /// The generated or custom short code
    #[schema(example = "abc123XY")]
    pub short_code: String,
    /// Full short URL for sharing
    #[schema(example = "http://localhost:3000/abc123XY")]
    pub short_url: String,
    /// The original URL that was shortened
    #[schema(example = "https://example.com/very/long/path")]
    pub original_url: String,
    /// When the short URL expires (if set)
    pub expires_at: Option<DateTime<Utc>>,
}

/// Response for URL info
#[derive(Debug, Serialize, ToSchema)]
pub struct UrlInfoResponse {
    /// The short code
    #[schema(example = "abc123XY")]
    pub short_code: String,
    /// The original URL
    #[schema(example = "https://example.com")]
    pub original_url: String,
    /// When the URL was created
    pub created_at: DateTime<Utc>,
    /// When the URL expires (if set)
    pub expires_at: Option<DateTime<Utc>>,
    /// Number of times accessed
    #[schema(example = 42)]
    pub click_count: i64,
    /// Last access time
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

/// Pagination metadata for list responses
#[derive(Debug, Serialize, ToSchema)]
pub struct PaginationMeta {
    /// Total number of items
    #[schema(example = 100)]
    pub total: i64,
    /// Number of items per page
    #[schema(example = 50)]
    pub limit: i64,
    /// Current offset
    #[schema(example = 0)]
    pub offset: i64,
    /// Whether there are more items
    pub has_next: bool,
    /// Whether there are previous items
    pub has_prev: bool,
}

impl PaginationMeta {
    pub fn new(total: i64, limit: i64, offset: i64) -> Self {
        let has_next = offset + limit < total;
        let has_prev = offset > 0;

        Self {
            total,
            limit,
            offset,
            has_next,
            has_prev,
        }
    }
}

/// Paginated response wrapper
#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedResponse<T: ToSchema> {
    /// List of items
    pub data: Vec<T>,
    /// Pagination information
    pub pagination: PaginationMeta,
}

impl<T: ToSchema> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, total: i64, limit: i64, offset: i64) -> Self {
        let pagination = PaginationMeta::new(total, limit, offset);
        Self { data, pagination }
    }
}

/// Statistics summary
#[derive(Debug, Serialize, ToSchema)]
pub struct StatsResponse {
    /// Total number of URLs created
    #[schema(example = 1000)]
    pub total_urls: i64,
    /// Total number of clicks across all URLs
    #[schema(example = 15000)]
    pub total_clicks: i64,
    /// Number of non-expired URLs
    #[schema(example = 950)]
    pub active_urls: i64,
    /// Number of expired URLs
    #[schema(example = 50)]
    pub expired_urls: i64,
}

/// Error response format (for OpenAPI documentation)
#[derive(Debug, Serialize, ToSchema)]
#[allow(dead_code)] // Used for OpenAPI schema generation
pub struct ErrorResponse {
    /// Error code (e.g., "NOT_FOUND", "INVALID_URL")
    #[schema(example = "NOT_FOUND")]
    pub error: String,
    /// Human-readable error message
    #[schema(example = "URL not found: abc123")]
    pub message: String,
}
