use crate::auth::AuthService;
use crate::db::Repository;
use crate::error::{AppError, AppResult};

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

/// Generate a unique short code
pub(crate) async fn generate_short_code(
    length: usize,
    max_attempts: u32,
    repository: &Repository,
) -> AppResult<String> {
    const ALPHABET_CHARS: &[char] = &[
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
        'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M',
        'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm',
        'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    ];

    for _ in 0..max_attempts {
        let code = nanoid::nanoid!(length, ALPHABET_CHARS);

        if !repository.short_code_exists(&code).await? {
            return Ok(code);
        }
    }

    Err(AppError::ShortCodeGenerationFailed)
}

/// Calculate hours from now until a given datetime
pub(crate) fn hours_from_now(dt: chrono::DateTime<chrono::Utc>) -> i64 {
    let now = chrono::Utc::now();
    let duration = dt.signed_duration_since(now);
    duration.num_hours()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_hours_from_now() {
        let now = chrono::Utc::now();
        let future = now + Duration::hours(24);
        assert!(hours_from_now(future) > 20);

        let past = now - Duration::hours(24);
        assert!(hours_from_now(past) < -20);
    }
}
