use crate::auth::{LoginRequest, LoginResponse};
use crate::error::{AppError, AppResult};
use axum::extract::State;
use axum::response::{IntoResponse, Json};
use std::sync::Arc;

use super::AppState;

/// Login to get JWT token
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> AppResult<impl IntoResponse> {
    let user = state
        .repository
        .get_user_by_username(&payload.username)
        .await?
        .ok_or(AppError::UserNotFound(payload.username.clone()))?;

    // Verify password
    bcrypt::verify(&payload.password, &user.password_hash)
        .map_err(|e| AppError::Internal(format!("Password verification failed: {}", e)))?;

    if !user.is_active {
        return Err(AppError::Unauthorized("User account is inactive".to_string()));
    }

    // Generate JWT token using auth service from state
    let token = state.auth_service.generate_token(&user.id.to_string(), &user.username)?;

    Ok(Json(LoginResponse {
        token,
        username: user.username,
    }))
}
