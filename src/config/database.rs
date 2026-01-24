use serde::Deserialize;

/// Database configuration
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    /// PostgreSQL connection URL
    pub url: String,

    /// Maximum number of database connections in the pool
    pub max_connections: u32,

    /// Minimum number of database connections to maintain
    pub min_connections: u32,

    /// Timeout in seconds for acquiring a connection from the pool
    pub acquire_timeout_seconds: u64,
}

impl DatabaseConfig {
    /// Validate database configuration values
    pub fn validate(&self) -> Result<(), String> {
        if self.min_connections > self.max_connections {
            return Err("DB_MIN_CONNECTIONS cannot be greater than DB_MAX_CONNECTIONS".to_string());
        }

        if self.acquire_timeout_seconds == 0 {
            return Err("DB_ACQUIRE_TIMEOUT_SECONDS must be greater than 0".to_string());
        }

        Ok(())
    }
}
