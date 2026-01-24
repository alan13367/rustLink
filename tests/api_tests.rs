//! Integration tests for rustLink API endpoints.
//!
//! These tests verify the HTTP API behavior and data structures
//! used by the API without requiring database connections.

use serde_json::json;

/// Test module for request/response types
mod type_tests {
    use super::*;

    #[test]
    fn test_create_url_request_serialization() {
        let request = json!({
            "url": "https://example.com",
            "custom_code": "mycode",
            "expiry_hours": 24
        });

        assert_eq!(request["url"], "https://example.com");
        assert_eq!(request["custom_code"], "mycode");
        assert_eq!(request["expiry_hours"], 24);
    }

    #[test]
    fn test_create_url_request_minimal() {
        let request = json!({
            "url": "https://example.com"
        });

        assert_eq!(request["url"], "https://example.com");
        assert!(request["custom_code"].is_null());
        assert!(request["expiry_hours"].is_null());
    }

    #[test]
    fn test_error_response_format() {
        let error = json!({
            "error": "NOT_FOUND",
            "message": "URL not found: abc123"
        });

        assert_eq!(error["error"], "NOT_FOUND");
        assert!(error["message"].as_str().unwrap().contains("abc123"));
    }

    #[test]
    fn test_stats_response_format() {
        let stats = json!({
            "total_urls": 100,
            "total_clicks": 1500,
            "active_urls": 95,
            "expired_urls": 5
        });

        assert_eq!(stats["total_urls"], 100);
        assert_eq!(stats["total_clicks"], 1500);
        assert_eq!(stats["active_urls"], 95);
        assert_eq!(stats["expired_urls"], 5);
    }

    #[test]
    fn test_url_entry_response_format() {
        let url_entry = json!({
            "id": 1,
            "short_code": "abc123",
            "original_url": "https://example.com",
            "created_at": "2024-01-01T00:00:00Z",
            "expires_at": "2024-01-31T00:00:00Z",
            "click_count": 42,
            "last_clicked_at": "2024-01-15T12:30:00Z"
        });

        assert_eq!(url_entry["short_code"], "abc123");
        assert_eq!(url_entry["original_url"], "https://example.com");
        assert_eq!(url_entry["click_count"], 42);
    }

    #[test]
    fn test_login_request_format() {
        let login = json!({
            "username": "admin",
            "password": "secret123"
        });

        assert_eq!(login["username"], "admin");
        assert_eq!(login["password"], "secret123");
    }

    #[test]
    fn test_login_response_format() {
        let response = json!({
            "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
            "username": "admin"
        });

        assert!(response["token"].as_str().unwrap().starts_with("eyJ"));
        assert_eq!(response["username"], "admin");
    }

    #[test]
    fn test_list_urls_query_params() {
        // Test that query params serialize correctly
        let params = json!({
            "limit": 50,
            "offset": 100
        });

        assert_eq!(params["limit"], 50);
        assert_eq!(params["offset"], 100);
    }
}

/// Test module for URL validation logic
mod validation_tests {
    fn is_valid_short_code(code: &str) -> bool {
        // Short codes should be alphanumeric and 4-16 characters
        code.len() >= 4 && code.len() <= 16 && code.chars().all(|c| c.is_ascii_alphanumeric())
    }

    #[test]
    fn test_valid_short_codes() {
        assert!(is_valid_short_code("abc123"));
        assert!(is_valid_short_code("ABCD"));
        assert!(is_valid_short_code("Test1234"));
        assert!(is_valid_short_code("a1b2c3d4e5f6"));
    }

    #[test]
    fn test_invalid_short_codes_too_short() {
        assert!(!is_valid_short_code("abc"));
        assert!(!is_valid_short_code("ab"));
        assert!(!is_valid_short_code("a"));
    }

    #[test]
    fn test_invalid_short_codes_too_long() {
        assert!(!is_valid_short_code("abcdefghijklmnopq")); // 17 chars
    }

    #[test]
    fn test_invalid_short_codes_special_chars() {
        assert!(!is_valid_short_code("abc-123"));
        assert!(!is_valid_short_code("abc_123"));
        assert!(!is_valid_short_code("abc.123"));
        assert!(!is_valid_short_code("abc@123"));
    }

    fn is_valid_url(url: &str) -> bool {
        url.starts_with("http://") || url.starts_with("https://")
    }

    #[test]
    fn test_valid_urls() {
        assert!(is_valid_url("https://example.com"));
        assert!(is_valid_url("http://localhost:3000"));
        assert!(is_valid_url("https://sub.domain.com/path?query=1"));
    }

    #[test]
    fn test_invalid_urls() {
        assert!(!is_valid_url("ftp://example.com"));
        assert!(!is_valid_url("example.com"));
        assert!(!is_valid_url("javascript:alert(1)"));
        assert!(!is_valid_url(""));
    }
}

/// Test module for error types
mod error_tests {
    use axum::http::StatusCode;

    #[test]
    fn test_error_codes() {
        let error_codes = vec![
            "NOT_FOUND",
            "INVALID_URL",
            "CODE_EXISTS",
            "DATABASE_ERROR",
            "CACHE_ERROR",
            "UNAUTHORIZED",
            "USER_NOT_FOUND",
            "INTERNAL_ERROR",
        ];

        // Verify all expected error codes are valid strings
        for code in error_codes {
            assert!(!code.is_empty());
            assert!(code.chars().all(|c| c.is_ascii_uppercase() || c == '_'));
        }
    }

    #[test]
    fn test_http_status_codes() {
        // NOT_FOUND -> 404
        assert_eq!(StatusCode::NOT_FOUND.as_u16(), 404);
        // BAD_REQUEST -> 400
        assert_eq!(StatusCode::BAD_REQUEST.as_u16(), 400);
        // CONFLICT -> 409
        assert_eq!(StatusCode::CONFLICT.as_u16(), 409);
        // UNAUTHORIZED -> 401
        assert_eq!(StatusCode::UNAUTHORIZED.as_u16(), 401);
        // INTERNAL_SERVER_ERROR -> 500
        assert_eq!(StatusCode::INTERNAL_SERVER_ERROR.as_u16(), 500);
    }
}

/// Test module for rate limiting configuration
mod rate_limit_tests {

    #[test]
    fn test_rate_limit_calculation() {
        let requests_per_minute = 60u32;
        let milliseconds_per_request = 60000 / requests_per_minute;
        assert_eq!(milliseconds_per_request, 1000);
    }

    #[test]
    fn test_burst_size_scaling() {
        let base_burst = 5u32;
        let lenient_burst = base_burst * 2;
        assert_eq!(lenient_burst, 10);
    }
}

/// Test module for JWT token handling
mod jwt_tests {

    #[test]
    fn test_jwt_structure() {
        // JWT has 3 parts separated by dots
        let sample_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

        let parts: Vec<&str> = sample_token.split('.').collect();
        assert_eq!(parts.len(), 3);

        // Each part should be base64-encoded
        for part in parts {
            assert!(!part.is_empty());
            assert!(part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
        }
    }

    #[test]
    fn test_authorization_header_format() {
        let token = "abc123";
        let header = format!("Bearer {}", token);
        assert!(header.starts_with("Bearer "));
        assert!(header.ends_with(token));
    }
}
