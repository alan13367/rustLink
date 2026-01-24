use serde::Deserialize;

/// CORS configuration
#[derive(Debug, Clone, Deserialize)]
pub struct CorsConfig {
    /// List of allowed origins for CORS (use ["*"] for all origins)
    pub allowed_origins: Vec<String>,
}
