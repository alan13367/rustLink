use crate::error::{AppError, AppResult};
use crate::models::UrlEntry;
use deadpool_redis::{redis::AsyncCommands, Manager, Pool, Runtime};
use serde_json;
use std::time::Duration;

/// Cache layer for URL lookups
#[derive(Clone)]
pub struct Cache {
    pool: Pool,
    default_ttl: Duration,
}

impl Cache {
    /// Create a new cache connection pool
    pub async fn new(redis_url: &str, max_connections: u32, default_ttl_seconds: u64) -> AppResult<Self> {
        let manager = Manager::new(redis_url)
            .map_err(|e| AppError::Configuration(format!("Invalid Redis URL: {}", e)))?;

        let pool = Pool::builder(manager)
            .max_size(max_connections as usize)
            .runtime(Runtime::Tokio1)
            .build()
            .map_err(|e| AppError::Configuration(format!("Failed to create Redis pool: {}", e)))?;

        Ok(Self {
            pool,
            default_ttl: Duration::from_secs(default_ttl_seconds),
        })
    }

    /// Ping the Redis server to check connectivity
    pub async fn ping(&self) -> AppResult<String> {
        let mut conn = self.pool.get().await?;
        let response: String = redis::cmd("PING").query_async(&mut *conn).await?;
        Ok(response)
    }

    /// Get a URL from cache by short code
    /// Returns None if cache fails or entry not found
    pub async fn get_url(&self, short_code: &str) -> AppResult<Option<UrlEntry>> {
        let key = Self::url_key(short_code);

        // Try to get connection with timeout, return None if Redis is unavailable
        let mut conn = match self.pool.get().await {
            Ok(c) => c,
            Err(_) => return Ok(None),
        };

        let value: Option<String> = match conn.get(&key).await {
            Ok(v) => v,
            Err(_) => return Ok(None), // Cache error treated as miss
        };

        match value {
            Some(v) => {
                let entry: UrlEntry = serde_json::from_str(&v)
                    .map_err(|e| AppError::Internal(format!("Cache deserialization error: {}", e)))?;
                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }

    /// Set a URL in cache
    pub async fn set_url(&self, entry: &UrlEntry) -> AppResult<()> {
        let key = Self::url_key(&entry.short_code);
        let value = serde_json::to_string(entry)?;
        let ttl = self.default_ttl.as_secs();
        let mut conn = self.pool.get().await?;

        // Type annotation needed for return type
        let _: () = conn.set_ex(&key, value, ttl).await?;

        Ok(())
    }

    /// Delete a URL from cache
    pub async fn delete_url(&self, short_code: &str) -> AppResult<()> {
        let key = Self::url_key(short_code);
        let mut conn = self.pool.get().await?;

        let _: () = conn.del(&key).await?;

        Ok(())
    }

    /// Check if a short code exists in cache
    #[allow(dead_code)]
    #[deprecated(note = "TODO: Use for analytics endpoint")]
    pub async fn exists(&self, short_code: &str) -> AppResult<bool> {
        let key = Self::url_key(short_code);
        let mut conn = self.pool.get().await?;

        let result: bool = conn.exists(&key).await?;
        Ok(result)
    }

    /// Set a custom TTL for a URL
    #[allow(dead_code)]
    #[deprecated(note = "TODO: Use for premium URL features")]
    pub async fn set_expiry(&self, short_code: &str, ttl_seconds: u64) -> AppResult<()> {
        let key = Self::url_key(short_code);
        let ttl = ttl_seconds as i64;
        let mut conn = self.pool.get().await?;

        let _: () = conn.expire(&key, ttl).await?;

        Ok(())
    }

    /// Clear all cached URLs (use with caution)
    #[allow(dead_code)]
    #[deprecated(note = "TODO: Use for admin maintenance endpoint")]
    pub async fn clear_all(&self) -> AppResult<()> {
        let pattern = format!("{}:*", Self::KEY_PREFIX);
        let mut conn = self.pool.get().await?;

        // Use SCAN instead of KEYS for better performance in production
        let mut keys: Vec<String> = Vec::new();
        let mut cursor: u64 = 0;

        loop {
            let (next_cursor, batch_keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(100)
                .query_async(&mut *conn)
                .await?;

            keys.extend(batch_keys);

            cursor = next_cursor;
            if cursor == 0 {
                break;
            }
        }

        if !keys.is_empty() {
            let _: () = conn.del(keys).await?;
        }

        Ok(())
    }

    /// Get cache statistics
    #[allow(dead_code)]
    #[deprecated(note = "TODO: Use for health check endpoint")]
    pub async fn get_stats(&self) -> AppResult<CacheStats> {
        let mut conn = self.pool.get().await?;

        // Use DBSIZE for approximate key count
        let db_size: u32 = redis::cmd("DBSIZE")
            .query_async(&mut *conn)
            .await
            .unwrap_or(0);

        Ok(CacheStats {
            keys: db_size as i64,
            status: "connected".to_string(),
        })
    }

    /// Generate cache key for a URL
    fn url_key(short_code: &str) -> String {
        format!("{}:{}", Self::KEY_PREFIX, short_code)
    }

    const KEY_PREFIX: &'static str = "url";
}

/// Cache statistics
#[derive(Debug)]
#[allow(dead_code)]
#[deprecated(note = "TODO: Use for health check endpoint")]
pub struct CacheStats {
    pub keys: i64,
    pub status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_key_generation() {
        assert_eq!(Cache::url_key("abc123"), "url:abc123");
        assert_eq!(Cache::url_key("test"), "url:test");
    }
}
