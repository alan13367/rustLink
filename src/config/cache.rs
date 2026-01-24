use serde::Deserialize;

/// Redis cache configuration
#[derive(Debug, Clone, Deserialize)]
pub struct CacheConfig {
    /// Redis connection URL
    pub url: String,

    /// Maximum number of Redis connections in the pool
    pub max_connections: u32,

    /// Default TTL for cached entries in seconds
    pub default_ttl_seconds: u64,
}

impl CacheConfig {
    /// Validate cache configuration values
    pub fn validate(&self) -> Result<(), String> {
        if self.default_ttl_seconds == 0 {
            return Err("CACHE_DEFAULT_TTL_SECONDS must be greater than 0".to_string());
        }

        Ok(())
    }
}
