use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde_json::json;
use thiserror::Error;

/// Main application error type
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Redis pool error: {0}")]
    RedisPool(#[from] deadpool_redis::PoolError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("URL not found: {0}")]
    UrlNotFound(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Short code already exists: {0}")]
    ShortCodeExists(String),

    #[error("Short code generation failed")]
    ShortCodeGenerationFailed,

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Environment variable missing: {0}")]
    MissingEnvVar(String),

    #[error("Internal server error: {0}")]
    Internal(String),
}

/// Convert AppError to HTTP response
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message, error_code) = match &self {
            AppError::UrlNotFound(_) => (StatusCode::NOT_FOUND, self.to_string(), "NOT_FOUND"),
            AppError::InvalidUrl(_) => (StatusCode::BAD_REQUEST, self.to_string(), "INVALID_URL"),
            AppError::ShortCodeExists(_) => (StatusCode::CONFLICT, self.to_string(), "CODE_EXISTS"),
            AppError::Database(e) => {
                tracing::error!("Database error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error occurred".to_string(),
                    "DATABASE_ERROR",
                )
            }
            AppError::Migration(e) => {
                tracing::error!("Migration error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Migration error occurred".to_string(),
                    "MIGRATION_ERROR",
                )
            }
            AppError::Redis(e) => {
                tracing::error!("Redis error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Cache error occurred".to_string(),
                    "CACHE_ERROR",
                )
            }
            AppError::RedisPool(e) => {
                tracing::error!("Redis pool error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Cache error occurred".to_string(),
                    "CACHE_ERROR",
                )
            }
            AppError::Serialization(e) => {
                tracing::error!("Serialization error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Data serialization error".to_string(),
                    "SERIALIZATION_ERROR",
                )
            }
            _ => {
                tracing::error!("Internal error: {}", self);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "An internal error occurred".to_string(),
                    "INTERNAL_ERROR",
                )
            }
        };

        let body = json!({
            "error": error_code,
            "message": error_message,
        });

        (status, Json(body)).into_response()
    }
}

/// Result type alias for AppResult
pub type AppResult<T> = Result<T, AppError>;
