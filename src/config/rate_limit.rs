use serde::Deserialize;

/// Rate limiting configuration
#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum number of requests per minute
    pub requests_per_minute: u64,

    /// Maximum burst size for rate limiting
    pub burst_size: u32,
}

impl RateLimitConfig {
    /// Validate rate limiting configuration values
    pub fn validate(&self) -> Result<(), String> {
        if self.requests_per_minute == 0 {
            return Err("RATE_LIMIT_PER_MINUTE must be greater than 0".to_string());
        }

        if self.burst_size == 0 {
            return Err("RATE_LIMIT_BURST must be greater than 0".to_string());
        }

        Ok(())
    }
}
