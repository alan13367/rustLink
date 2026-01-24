use crate::auth::AuthService;
use crate::cache::Cache;
use crate::db::Repository;
use crate::jobs::JobSender;

/// Application state shared across all HTTP handlers.
///
/// This struct is wrapped in `Arc` and shared across all request handlers
/// via Axum's State extraction. It contains all the necessary dependencies
/// for handling HTTP requests.
#[derive(Clone)]
pub struct AppState {
    /// Database repository for URL and user operations
    pub repository: Repository,

    /// Redis cache for storing frequently accessed URLs
    pub cache: Cache,

    /// JWT authentication service for token generation and validation
    pub auth_service: AuthService,

    /// Background job sender for asynchronous tasks (e.g., click count updates)
    pub job_sender: JobSender,

    /// Base URL for constructing short URLs (e.g., "http://localhost:3000")
    pub base_url: String,

    /// Default expiry time for newly created short URLs (in hours)
    pub default_expiry_hours: i64,

    /// Length of randomly generated short codes
    pub short_code_length: usize,

    /// Maximum number of attempts to generate a unique short code
    pub short_code_max_attempts: u32,

    /// Whether caching is enabled for URL lookups
    pub cache_enabled: bool,

    /// Whether strict URL validation is enabled (requires http:// or https://)
    pub strict_url_validation: bool,
}
