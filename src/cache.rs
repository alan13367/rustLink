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
            .max_size(max_connections)
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
        let response: String = redis::cmd("PING").query_async(&mut conn).await?;
        Ok(response)
    }

    /// Get a URL from cache by short code
    pub async fn get_url(&self, short_code: &str) -> AppResult<Option<UrlEntry>> {
        let key = Self::url_key(short_code);
        let mut conn = self.pool.get().await?;

        let value: Option<String> = conn.get(&key).await?;

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
        let mut conn = self.pool.get().await?;

        conn.set_ex(&key, value, self.default_ttl.as_secs() as usize)
            .await?;

        Ok(())
    }

    /// Delete a URL from cache
    pub async fn delete_url(&self, short_code: &str) -> AppResult<()> {
        let key = Self::url_key(short_code);
        let mut conn = self.pool.get().await?;

        conn.del(&key).await?;

        Ok(())
    }

    /// Check if a short code exists in cache
    pub async fn exists(&self, short_code: &str) -> AppResult<bool> {
        let key = Self::url_key(short_code);
        let mut conn = self.pool.get().await?;

        let result: bool = conn.exists(&key).await?;
        Ok(result)
    }

    /// Set a custom TTL for a URL
    pub async fn set_expiry(&self, short_code: &str, ttl_seconds: u64) -> AppResult<()> {
        let key = Self::url_key(short_code);
        let mut conn = self.pool.get().await?;

        conn.expire(&key, ttl_seconds).await?;

        Ok(())
    }

    /// Clear all cached URLs (use with caution)
    pub async fn clear_all(&self) -> AppResult<()> {
        let pattern = format!("{}:*", Self::KEY_PREFIX);
        let mut conn = self.pool.get().await?;

        // Get all keys matching the pattern
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(&pattern)
            .query_async(&mut conn)
            .await?;

        if !keys.is_empty() {
            conn.del(keys).await?;
        }

        Ok(())
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> AppResult<CacheStats> {
        let mut conn = self.pool.get().await?;

        let info: String = redis::cmd("INFO")
            .arg("stats")
            .query_async(&mut conn)
            .await?;

        // Parse key_count from INFO response
        let key_count = info
            .lines()
            .find(|line| line.starts_with("keyspace_hits:"))
            .and_then(|line| line.split(':').nth(1))
            .and_then(|s| s.parse().unwrap_or(0))
            .unwrap_or(0);

        Ok(CacheStats {
            keys: key_count,
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
