use serde::Deserialize;

/// Server configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    /// Host address to bind to (e.g., "127.0.0.1")
    pub host: String,

    /// Port number to bind to (e.g., 3000)
    pub port: u16,
}
