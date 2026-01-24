pub mod admin_handlers;
pub mod auth_handlers;
pub mod health;
pub mod helpers;
mod router;
pub mod types;
pub mod url_handlers;

pub use router::create_router;

// Re-export AppState for convenience
pub use crate::state::AppState;
