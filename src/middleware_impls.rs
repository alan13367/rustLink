use crate::auth::Claims;
use axum::{
    extract::Request,
    http::{HeaderMap, HeaderValue},
    middleware::Next,
    response::Response,
};
use tower_governor::key_extractor::KeyExtractor;
use uuid::Uuid;

/// Request ID wrapper for use in request extensions
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

impl RequestId {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for RequestId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Request context containing request metadata
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RequestContext {
    pub request_id: String,
    pub client_ip: String,
    pub user_agent: Option<String>,
    pub user_id: Option<String>,
}

impl RequestContext {
    pub fn new(request_id: String, client_ip: String, user_agent: Option<String>) -> Self {
        Self {
            request_id,
            client_ip,
            user_agent,
            user_id: None,
        }
    }

    #[allow(dead_code)]
    pub fn with_user(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }
}

/// Extract client IP address from headers
pub fn extract_client_ip(headers: &HeaderMap) -> String {
    // Check for X-Forwarded-For header (proxy/load balancer)
    if let Some(forwarded) = headers.get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            if let Some(first_ip) = forwarded_str.split(',').next() {
                return first_ip.trim().to_string();
            }
        }
    }

    // Check for X-Real-IP header
    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(real_ip_str) = real_ip.to_str() {
            return real_ip_str.to_string();
        }
    }

    // Fallback to a default
    "unknown".to_string()
}

/// Extract user agent from headers
pub fn extract_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
}

/// Request ID middleware - adds a unique ID to each request
pub async fn request_id_middleware(
    mut req: Request,
    next: Next,
) -> Response {
    // Try to get existing request ID from header, or generate new one
    let request_id: String = req
        .headers()
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Add request ID to request extensions for use in handlers
    req.extensions_mut().insert(RequestId(request_id.clone()));

    tracing::info!(
        request_id = %request_id,
        method = %req.method(),
        path = %req.uri().path(),
        "Incoming request"
    );

    let mut response = next.run(req).await;

    // Add request ID to response headers
    if let Ok(header_value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert("x-request-id", header_value);
    }

    response
}

/// Request context middleware - adds context to each request
pub async fn request_context_middleware(
    mut req: Request,
    next: Next,
) -> Response {
    let headers = req.headers();
    let request_id = req
        .extensions()
        .get::<RequestId>()
        .map(|r| r.0.clone())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let client_ip = extract_client_ip(headers);
    let user_agent = extract_user_agent(headers);

    let context = RequestContext::new(request_id, client_ip, user_agent);
    req.extensions_mut().insert(context);

    next.run(req).await
}

/// Custom key extractor for rate limiting that considers user authentication
#[derive(Clone)]
pub struct AuthAwareKeyExtractor;

impl KeyExtractor for AuthAwareKeyExtractor {
    type Key = String;

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, tower_governor::GovernorError> {
        // Check if user is authenticated
        if let Some(claims) = req.extensions().get::<Claims>() {
            // Rate limit per user ID for authenticated users
            Ok(format!("user:{}", claims.sub))
        } else {
            // Rate limit per IP for anonymous users
            let headers = req.headers();
            let ip = extract_client_ip(headers);
            Ok(format!("ip:{}", ip))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_id_new() {
        let id = RequestId::new();
        assert_eq!(id.0.len(), 36); // UUID v4 length
    }

    #[test]
    fn test_request_context_new() {
        let ctx = RequestContext::new(
            "test-123".to_string(),
            "127.0.0.1".to_string(),
            Some("Mozilla/5.0".to_string()),
        );

        assert_eq!(ctx.request_id, "test-123");
        assert_eq!(ctx.client_ip, "127.0.0.1");
        assert_eq!(ctx.user_agent, Some("Mozilla/5.0".to_string()));
        assert!(ctx.user_id.is_none());
    }

    #[test]
    fn test_request_context_with_user() {
        let ctx = RequestContext::new(
            "test-123".to_string(),
            "127.0.0.1".to_string(),
            None,
        )
        .with_user("user-456".to_string());

        assert_eq!(ctx.user_id, Some("user-456".to_string()));
    }

    #[test]
    fn test_extract_client_ip_from_x_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "192.168.1.1".parse().unwrap());

        let ip = extract_client_ip(&headers);
        assert_eq!(ip, "192.168.1.1");
    }

    #[test]
    fn test_extract_client_ip_from_multiple_forwarded() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            "192.168.1.1, 10.0.0.1".parse().unwrap(),
        );

        let ip = extract_client_ip(&headers);
        assert_eq!(ip, "192.168.1.1");
    }

    #[test]
    fn test_extract_client_ip_unknown() {
        let headers = HeaderMap::new();
        let ip = extract_client_ip(&headers);
        assert_eq!(ip, "unknown");
    }
}
