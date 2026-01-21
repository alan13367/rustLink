use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde_json::json;
use std::fmt;
use thiserror::Error;

/// Main application error type
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

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

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Database(e) => write!(f, "Database error: {}", e),
            AppError::Redis(e) => write!(f, "Redis error: {}", e),
            AppError::RedisPool(e) => write!(f, "Redis pool error: {}", e),
            AppError::Serialization(e) => write!(f, "Serialization error: {}", e),
            AppError::UrlNotFound(code) => write!(f, "URL not found: {}", code),
            AppError::InvalidUrl(url) => write!(f, "Invalid URL: {}", url),
            AppError::ShortCodeExists(code) => write!(f, "Short code already exists: {}", code),
            AppError::ShortCodeGenerationFailed => write!(f, "Failed to generate short code"),
            AppError::Configuration(msg) => write!(f, "Configuration error: {}", msg),
            AppError::MissingEnvVar(key) => write!(f, "Missing environment variable: {}", key),
            AppError::Internal(msg) => write!(f, "Internal server error: {}", msg),
        }
    }
}

/// Convert AppError to HTTP response
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message, error_code) = match self {
            AppError::UrlNotFound(_) => (StatusCode::NOT_FOUND, self.to_string(), "NOT_FOUND"),
            AppError::InvalidUrl(_) => (StatusCode::BAD_REQUEST, self.to_string(), "INVALID_URL"),
            AppError::ShortCodeExists(_) => {
                (StatusCode::CONFLICT, self.to_string(), "CODE_EXISTS")
            }
            AppError::Database(ref e) => {
                tracing::error!("Database error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error occurred".to_string(),
                    "DATABASE_ERROR",
                )
            }
            AppError::Redis(ref e) => {
                tracing::error!("Redis error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Cache error occurred".to_string(),
                    "CACHE_ERROR",
                )
            }
            AppError::RedisPool(ref e) => {
                tracing::error!("Redis pool error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Cache error occurred".to_string(),
                    "CACHE_ERROR",
                )
            }
            AppError::Serialization(ref e) => {
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
