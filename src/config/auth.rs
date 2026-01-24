use serde::Deserialize;

/// Authentication configuration
#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    /// JWT secret key for token signing and validation
    pub jwt_secret: String,

    /// JWT token expiration time in hours
    pub jwt_expiration_hours: i64,
}

impl AuthConfig {
    /// Validate authentication configuration values
    pub fn validate(&self) -> Result<(), String> {
        if self.jwt_secret.len() < 32 {
            return Err("JWT_SECRET must be at least 32 characters for security".to_string());
        }

        if self.jwt_expiration_hours < 1 {
            return Err("JWT_EXPIRATION_HOURS must be at least 1".to_string());
        }

        Ok(())
    }
}
