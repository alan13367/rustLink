use serde::Deserialize;

/// URL shortening configuration
#[derive(Debug, Clone, Deserialize)]
pub struct UrlConfig {
    /// Length of randomly generated short codes
    pub short_code_length: usize,

    /// Base URL for constructing short URLs (e.g., "http://localhost:3000")
    pub base_url: String,

    /// Default expiry time for newly created short URLs (in hours)
    pub default_expiry_hours: i64,

    /// Maximum number of attempts to generate a unique short code
    pub short_code_max_attempts: u32,

    /// Whether caching is enabled for URL lookups
    pub cache_enabled: bool,

    /// Whether strict URL validation is enabled (requires http:// or https://)
    pub strict_url_validation: bool,
}

impl UrlConfig {
    /// Validate URL configuration values
    pub fn validate(&self) -> Result<(), String> {
        if self.short_code_length < 4 || self.short_code_length > 16 {
            return Err("SHORT_CODE_LENGTH must be between 4 and 16".to_string());
        }

        if self.default_expiry_hours < 1 {
            return Err("DEFAULT_EXPIRY_HOURS must be at least 1".to_string());
        }

        if self.short_code_max_attempts < 1 || self.short_code_max_attempts > 100 {
            return Err("SHORT_CODE_MAX_ATTEMPTS must be between 1 and 100".to_string());
        }

        Ok(())
    }
}
