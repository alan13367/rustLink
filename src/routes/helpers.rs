use crate::auth::AuthService;
use crate::error::{AppError, AppResult};

// Re-export hours_from_now from util module for convenience
pub use crate::util::hours_from_now;

// Re-export generate_short_code from services module for convenience
pub use crate::services::ShortCodeService;

/// Helper to extract JWT claims from Authorization header
pub(crate) fn extract_claims(
    headers: &axum::http::HeaderMap,
    auth_service: &AuthService,
) -> AppResult<crate::auth::Claims> {
    let auth_header = headers
        .get("Authorization")
        .ok_or_else(|| AppError::Internal("Missing Authorization header".to_string()))?;

    let auth_str = auth_header
        .to_str()
        .map_err(|e| AppError::Internal(format!("Invalid Authorization header: {}", e)))?;

    if !auth_str.starts_with("Bearer ") {
        return Err(AppError::Internal(
            "Authorization header must start with 'Bearer '".to_string(),
        ));
    }

    let token = &auth_str[7..];
    auth_service.validate_token(token)
}
