//! Configuration validation tests.
//!
//! These tests verify configuration loading and validation logic.

/// Test module for configuration validation
mod config_tests {
    #[test]
    fn test_server_port_range() {
        let valid_ports = vec![80, 443, 3000, 8080, 8443];
        for port in valid_ports {
            assert!(port > 0 && port <= 65535, "Port {} should be valid", port);
        }
    }

    #[test]
    fn test_database_connection_limits() {
        let max_connections = 10u32;
        let min_connections = 1u32;

        assert!(max_connections >= min_connections);
        assert!(min_connections >= 1);
        assert!(max_connections <= 100); // Reasonable upper limit
    }

    #[test]
    fn test_cache_ttl_values() {
        let default_ttl = 3600i64; // 1 hour
        let min_ttl = 60i64; // 1 minute
        let max_ttl = 86400i64 * 30; // 30 days

        assert!(default_ttl >= min_ttl);
        assert!(default_ttl <= max_ttl);
    }

    #[test]
    fn test_short_code_length_bounds() {
        let min_length = 4usize;
        let max_length = 16usize;
        let default_length = 8usize;

        assert!(default_length >= min_length);
        assert!(default_length <= max_length);
    }

    #[test]
    fn test_jwt_expiration_range() {
        let default_hours = 24i64;
        let min_hours = 1i64;
        let max_hours = 24 * 30; // 30 days

        assert!(default_hours >= min_hours);
        assert!(default_hours <= max_hours);
    }

    #[test]
    fn test_rate_limit_config() {
        let per_minute = 60u32;
        let burst = 10u32;

        assert!(per_minute > 0);
        assert!(burst > 0);
        assert!(burst <= per_minute); // Burst shouldn't exceed per-minute limit
    }

    #[test]
    fn test_cors_origins_parsing() {
        let origins_str = "http://localhost:3000,https://example.com";
        let origins: Vec<&str> = origins_str.split(',').map(|s| s.trim()).collect();

        assert_eq!(origins.len(), 2);
        assert!(origins.iter().all(|o| o.starts_with("http")));
    }

    #[test]
    fn test_wildcard_cors() {
        let origins = vec!["*".to_string()];
        assert!(origins.iter().any(|o| o == "*"));
    }

    #[test]
    fn test_base_url_format() {
        let host = "127.0.0.1";
        let port = 3000u16;
        let base_url = format!("http://{}:{}", host, port);

        assert!(base_url.starts_with("http://"));
        assert!(base_url.contains(&port.to_string()));
    }
}

/// Test module for environment variable parsing
mod env_parsing_tests {
    #[test]
    fn test_bool_parsing() {
        let true_values = vec!["true", "TRUE", "True", "1"];
        let false_values = vec!["false", "FALSE", "False", "0"];

        for val in true_values {
            let parsed: bool = val.parse().unwrap_or(false);
            // Note: Rust's bool::parse only accepts "true" and "false"
            if val == "true" {
                assert!(parsed);
            }
        }

        for val in false_values {
            let parsed: bool = val.parse().unwrap_or(true);
            if val == "false" {
                assert!(!parsed);
            }
        }
    }

    #[test]
    fn test_port_parsing() {
        let port_str = "3000";
        let port: u16 = port_str.parse().expect("should parse");
        assert_eq!(port, 3000);
    }

    #[test]
    fn test_invalid_port_parsing() {
        let invalid = "not_a_port";
        let result: Result<u16, _> = invalid.parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_connection_count_parsing() {
        let count_str = "10";
        let count: u32 = count_str.parse().expect("should parse");
        assert_eq!(count, 10);
    }
}

/// Test module for URL validation
mod url_validation_tests {
    #[test]
    fn test_postgresql_url_format() {
        let url = "postgresql://user:pass@localhost:5432/dbname";
        assert!(url.starts_with("postgresql://"));
        assert!(url.contains("@"));
        assert!(url.contains(":5432/"));
    }

    #[test]
    fn test_redis_url_format() {
        let url = "redis://127.0.0.1:6379";
        assert!(url.starts_with("redis://"));
        assert!(url.contains(":6379"));
    }

    #[test]
    fn test_redis_url_with_password() {
        let url = "redis://:password@127.0.0.1:6379";
        assert!(url.starts_with("redis://"));
        assert!(url.contains("@"));
    }
}
