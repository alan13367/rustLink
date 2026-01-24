use crate::auth::AuthService;
use crate::cache::Cache;
use crate::db::Repository;
use crate::jobs::JobSender;

pub mod admin_handlers;
pub mod auth_handlers;
pub mod health;
pub mod helpers;
mod router;
pub mod types;
pub mod url_handlers;

pub use router::create_router;

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
