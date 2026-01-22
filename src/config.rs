use crate::error::{AppError, AppResult};
use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub cache: CacheConfig,
    pub url: UrlConfig,
    pub auth: AuthConfig,
    pub rate_limit: RateLimitConfig,
    pub cors: CorsConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CacheConfig {
    pub url: String,
    pub max_connections: u32,
    pub default_ttl_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UrlConfig {
    pub short_code_length: usize,
    pub base_url: String,
    pub default_expiry_hours: i64,
    pub short_code_max_attempts: u32,
    pub cache_enabled: bool,
    pub strict_url_validation: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub jwt_expiration_hours: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_minute: u64,
    pub burst_size: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> AppResult<Self> {
        dotenvy::dotenv().ok();

        let server_host = env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let server_port = env::var("SERVER_PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .map_err(|_| AppError::Configuration("Invalid SERVER_PORT".to_string()))?;

        let database_url = env::var("DATABASE_URL")
            .map_err(|_| AppError::MissingEnvVar("DATABASE_URL".to_string()))?;
        let db_max_connections = env::var("DB_MAX_CONNECTIONS")
            .unwrap_or_else(|_| "10".to_string())
            .parse()
            .map_err(|_| AppError::Configuration("Invalid DB_MAX_CONNECTIONS".to_string()))?;
        let db_min_connections = env::var("DB_MIN_CONNECTIONS")
            .unwrap_or_else(|_| "1".to_string())
            .parse()
            .map_err(|_| AppError::Configuration("Invalid DB_MIN_CONNECTIONS".to_string()))?;
        let db_acquire_timeout = env::var("DB_ACQUIRE_TIMEOUT_SECONDS")
            .unwrap_or_else(|_| "30".to_string())
            .parse()
            .map_err(|_| {
                AppError::Configuration("Invalid DB_ACQUIRE_TIMEOUT_SECONDS".to_string())
            })?;

        let redis_url =
            env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        let cache_max_connections = env::var("CACHE_MAX_CONNECTIONS")
            .unwrap_or_else(|_| "10".to_string())
            .parse()
            .map_err(|_| AppError::Configuration("Invalid CACHE_MAX_CONNECTIONS".to_string()))?;
        let cache_default_ttl = env::var("CACHE_DEFAULT_TTL_SECONDS")
            .unwrap_or_else(|_| "3600".to_string())
            .parse()
            .map_err(|_| {
                AppError::Configuration("Invalid CACHE_DEFAULT_TTL_SECONDS".to_string())
            })?;

        let short_code_length = env::var("SHORT_CODE_LENGTH")
            .unwrap_or_else(|_| "8".to_string())
            .parse()
            .map_err(|_| AppError::Configuration("Invalid SHORT_CODE_LENGTH".to_string()))?;
        let base_url = env::var("BASE_URL")
            .unwrap_or_else(|_| format!("http://{}:{}", server_host, server_port));
        let default_expiry_hours = env::var("DEFAULT_EXPIRY_HOURS")
            .unwrap_or_else(|_| "720".to_string())
            .parse()
            .map_err(|_| AppError::Configuration("Invalid DEFAULT_EXPIRY_HOURS".to_string()))?;
        let short_code_max_attempts = env::var("SHORT_CODE_MAX_ATTEMPTS")
            .unwrap_or_else(|_| "10".to_string())
            .parse()
            .map_err(|_| AppError::Configuration("Invalid SHORT_CODE_MAX_ATTEMPTS".to_string()))?;
        let cache_enabled = env::var("CACHE_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .map_err(|_| AppError::Configuration("Invalid CACHE_ENABLED".to_string()))?;
        let strict_url_validation = env::var("STRICT_URL_VALIDATION")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .map_err(|_| AppError::Configuration("Invalid STRICT_URL_VALIDATION".to_string()))?;

        // Authentication config
        let jwt_secret = env::var("JWT_SECRET")
            .map_err(|_| AppError::MissingEnvVar("JWT_SECRET".to_string()))?;
        let jwt_expiration_hours = env::var("JWT_EXPIRATION_HOURS")
            .unwrap_or_else(|_| "24".to_string())
            .parse()
            .map_err(|_| AppError::Configuration("Invalid JWT_EXPIRATION_HOURS".to_string()))?;

        // Rate limit config
        let requests_per_minute = env::var("RATE_LIMIT_PER_MINUTE")
            .unwrap_or_else(|_| "10".to_string())
            .parse()
            .map_err(|_| AppError::Configuration("Invalid RATE_LIMIT_PER_MINUTE".to_string()))?;
        let burst_size = env::var("RATE_LIMIT_BURST")
            .unwrap_or_else(|_| "5".to_string())
            .parse()
            .map_err(|_| AppError::Configuration("Invalid RATE_LIMIT_BURST".to_string()))?;

        // CORS config
        let allowed_origins_str = env::var("ALLOWED_ORIGINS").unwrap_or_else(|_| "*".to_string());
        let allowed_origins: Vec<String> = if allowed_origins_str == "*" {
            vec!["*".to_string()]
        } else {
            allowed_origins_str
                .split(',')
                .map(|s| s.trim().to_string())
                .collect()
        };

        let config = Config {
            server: ServerConfig {
                host: server_host,
                port: server_port,
            },
            database: DatabaseConfig {
                url: database_url,
                max_connections: db_max_connections,
                min_connections: db_min_connections,
                acquire_timeout_seconds: db_acquire_timeout,
            },
            cache: CacheConfig {
                url: redis_url,
                max_connections: cache_max_connections,
                default_ttl_seconds: cache_default_ttl,
            },
            url: UrlConfig {
                short_code_length,
                base_url,
                default_expiry_hours,
                short_code_max_attempts,
                cache_enabled,
                strict_url_validation,
            },
            auth: AuthConfig {
                jwt_secret,
                jwt_expiration_hours,
            },
            rate_limit: RateLimitConfig {
                requests_per_minute,
                burst_size,
            },
            cors: CorsConfig { allowed_origins },
        };

        // Validate configuration
        config.validate()?;

        Ok(config)
    }

    /// Validate configuration values
    pub fn validate(&self) -> AppResult<()> {
        // Validate database settings
        if self.database.min_connections > self.database.max_connections {
            return Err(AppError::Configuration(
                "DB_MIN_CONNECTIONS cannot be greater than DB_MAX_CONNECTIONS".to_string(),
            ));
        }

        if self.database.acquire_timeout_seconds == 0 {
            return Err(AppError::Configuration(
                "DB_ACQUIRE_TIMEOUT_SECONDS must be greater than 0".to_string(),
            ));
        }

        // Validate URL settings
        if self.url.short_code_length < 4 || self.url.short_code_length > 16 {
            return Err(AppError::Configuration(
                "SHORT_CODE_LENGTH must be between 4 and 16".to_string(),
            ));
        }

        if self.url.default_expiry_hours < 1 {
            return Err(AppError::Configuration(
                "DEFAULT_EXPIRY_HOURS must be at least 1".to_string(),
            ));
        }

        if self.url.short_code_max_attempts < 1 || self.url.short_code_max_attempts > 100 {
            return Err(AppError::Configuration(
                "SHORT_CODE_MAX_ATTEMPTS must be between 1 and 100".to_string(),
            ));
        }

        // Validate JWT settings
        if self.auth.jwt_secret.len() < 32 {
            return Err(AppError::Configuration(
                "JWT_SECRET must be at least 32 characters for security".to_string(),
            ));
        }

        if self.auth.jwt_expiration_hours < 1 {
            return Err(AppError::Configuration(
                "JWT_EXPIRATION_HOURS must be at least 1".to_string(),
            ));
        }

        // Validate rate limiting settings
        if self.rate_limit.requests_per_minute == 0 {
            return Err(AppError::Configuration(
                "RATE_LIMIT_PER_MINUTE must be greater than 0".to_string(),
            ));
        }

        if self.rate_limit.burst_size == 0 {
            return Err(AppError::Configuration(
                "RATE_LIMIT_BURST must be greater than 0".to_string(),
            ));
        }

        // Validate cache settings
        if self.cache.default_ttl_seconds == 0 {
            return Err(AppError::Configuration(
                "CACHE_DEFAULT_TTL_SECONDS must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = Config {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3000,
            },
            database: DatabaseConfig {
                url: "postgresql://localhost/test".to_string(),
                max_connections: 10,
                min_connections: 1,
                acquire_timeout_seconds: 30,
            },
            cache: CacheConfig {
                url: "redis://127.0.0.1".to_string(),
                max_connections: 10,
                default_ttl_seconds: 3600,
            },
            url: UrlConfig {
                short_code_length: 8,
                base_url: "http://localhost:3000".to_string(),
                default_expiry_hours: 720,
                short_code_max_attempts: 10,
                cache_enabled: true,
                strict_url_validation: true,
            },
            auth: AuthConfig {
                jwt_secret: "test_secret".to_string(),
                jwt_expiration_hours: 24,
            },
            rate_limit: RateLimitConfig {
                requests_per_minute: 10,
                burst_size: 5,
            },
            cors: CorsConfig {
                allowed_origins: vec!["*".to_string()],
            },
        };

        assert_eq!(config.server.port, 3000);
        assert_eq!(config.server.host, "127.0.0.1");
    }
}
