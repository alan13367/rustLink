use crate::error::{AppError, AppResult};
use crate::models::UrlEntry;
use chrono::{DateTime, Utc};
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    PgPool, ConnectOptions,
};
use std::str::FromStr;

/// Database repository
pub struct Repository {
    pool: PgPool,
}

impl Repository {
    /// Create a new repository with a connection pool
    pub async fn new(database_url: &str, max_connections: u32, min_connections: u32) -> AppResult<Self> {
        let options = PgConnectOptions::from_str(database_url)
            .map_err(|e| AppError::Configuration(format!("Invalid database URL: {}", e)))?
            .disable_statement_logging();

        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .min_connections(min_connections)
            .connect_with(options)
            .await?;

        Ok(Self { pool })
    }

    /// Run database migrations
    pub async fn run_migrations(&self) -> AppResult<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    /// Create a new URL entry
    pub async fn create_url(
        &self,
        short_code: &str,
        original_url: &str,
        expires_at: Option<DateTime<Utc>>,
    ) -> AppResult<UrlEntry> {
        let now = Utc::now();

        let result = sqlx::query_as::<_, UrlEntry>(
            r#"
            INSERT INTO urls (short_code, original_url, created_at, expires_at, click_count)
            VALUES ($1, $2, $3, $4, 0)
            RETURNING *
            "#,
        )
        .bind(short_code)
        .bind(original_url)
        .bind(now)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }

    /// Get a URL entry by short code
    pub async fn get_url_by_short_code(&self, short_code: &str) -> AppResult<Option<UrlEntry>> {
        let result = sqlx::query_as::<_, UrlEntry>(
            r#"
            SELECT * FROM urls
            WHERE short_code = $1
            "#,
        )
        .bind(short_code)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    /// Increment click count for a URL
    pub async fn increment_click_count(&self, short_code: &str) -> AppResult<()> {
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE urls
            SET click_count = click_count + 1,
                last_clicked_at = $1
            WHERE short_code = $2
            "#,
        )
        .bind(now)
        .bind(short_code)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Check if a short code exists
    pub async fn short_code_exists(&self, short_code: &str) -> AppResult<bool> {
        let result = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM urls WHERE short_code = $1
            "#,
        )
        .bind(short_code)
        .fetch_one(&self.pool)
        .await?;

        Ok(result > 0)
    }

    /// Delete a URL by short code
    pub async fn delete_url(&self, short_code: &str) -> AppResult<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM urls WHERE short_code = $1
            "#,
        )
        .bind(short_code)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Update expiry for a URL
    #[allow(dead_code)]
    pub async fn update_expiry(
        &self,
        short_code: &str,
        expires_at: DateTime<Utc>,
    ) -> AppResult<Option<UrlEntry>> {
        let result = sqlx::query_as::<_, UrlEntry>(
            r#"
            UPDATE urls
            SET expires_at = $1
            WHERE short_code = $2
            RETURNING *
            "#,
        )
        .bind(expires_at)
        .bind(short_code)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    /// Delete all expired URLs
    pub async fn delete_expired_urls(&self) -> AppResult<u64> {
        let now = Utc::now();

        let result = sqlx::query(
            r#"
            DELETE FROM urls WHERE expires_at IS NOT NULL AND expires_at < $1
            "#,
        )
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Get statistics
    pub async fn get_stats(&self) -> AppResult<Stats> {
        let row = sqlx::query_as::<_, (i64, i64, i64, i64)>(
            r#"
            SELECT
                COUNT(*) as total_urls,
                COALESCE(CAST(SUM(click_count) AS BIGINT), 0) as total_clicks,
                COUNT(*) FILTER (WHERE expires_at IS NULL OR expires_at > NOW()) as active_urls,
                COUNT(*) FILTER (WHERE expires_at IS NOT NULL AND expires_at <= NOW()) as expired_urls
            FROM urls
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Stats {
            total_urls: row.0,
            total_clicks: row.1,
            active_urls: row.2,
            expired_urls: row.3,
        })
    }

    /// Get all URLs (paginated)
    pub async fn get_all_urls(&self, limit: i64, offset: i64) -> AppResult<Vec<UrlEntry>> {
        let results = sqlx::query_as::<_, UrlEntry>(
            r#"
            SELECT * FROM urls
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(results)
    }
}

/// Statistics struct
#[derive(Debug)]
pub struct Stats {
    pub total_urls: i64,
    pub total_clicks: i64,
    pub active_urls: i64,
    pub expired_urls: i64,
}

/// Clone implementation for Repository
impl Clone for Repository {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_creation() {
        let stats = Stats {
            total_urls: 100,
            total_clicks: 1000,
            active_urls: 80,
            expired_urls: 20,
        };

        assert_eq!(stats.total_urls, 100);
        assert_eq!(stats.total_clicks, 1000);
        assert_eq!(stats.active_urls, 80);
        assert_eq!(stats.expired_urls, 20);
    }
}
