use crate::error::AppError;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

pub type AppResult<T> = std::result::Result<T, AppError>;

/// JWT Claims
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String, // User ID
    pub username: String,
    pub exp: i64, // Expiration time as Unix timestamp
    pub iat: i64, // Issued at time as Unix timestamp
}

/// Login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub username: String,
}

/// JWT authentication service
#[derive(Clone)]
pub struct AuthService {
    secret: String,
    expiration_hours: i64,
}

impl AuthService {
    /// Create a new authentication service
    pub fn new(secret: String, expiration_hours: i64) -> Self {
        Self {
            secret,
            expiration_hours,
        }
    }

    /// Generate a JWT token for a user
    pub fn generate_token(&self, user_id: &str, username: &str) -> AppResult<String> {
        let now = Utc::now();
        let exp = now + Duration::hours(self.expiration_hours);

        let claims = Claims {
            sub: user_id.to_string(),
            username: username.to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_ref()),
        )
        .map_err(|e| AppError::Internal(format!("Token generation failed: {}", e)))
    }

    /// Validate a JWT token and return claims
    pub fn validate_token(&self, token: &str) -> AppResult<Claims> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_ref()),
            &Validation::new(Algorithm::HS256),
        )
        .map(|data| data.claims)
        .map_err(|e| AppError::Internal(format!("Token validation failed: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_generation_and_validation() {
        let secret = "test_secret_key".to_string();
        let auth_service = AuthService::new(secret, 24);

        let user_id = "123";
        let username = "testuser";

        let token = auth_service
            .generate_token(user_id, username)
            .expect("Failed to generate token");

        let claims = auth_service
            .validate_token(&token)
            .expect("Failed to validate token");

        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.username, username);
    }

    #[test]
    fn test_invalid_token_validation() {
        let secret = "test_secret_key".to_string();
        let auth_service = AuthService::new(secret, 24);

        let result = auth_service.validate_token("invalid_token");
        assert!(result.is_err());
    }
}
