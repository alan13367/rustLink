use crate::db::Repository;
use crate::error::AppError;
use sqlx::FromRow;

/// User model from database
#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub is_active: bool,
}

/// Repository extension for user operations
impl Repository {
    /// Create a new user
    #[allow(dead_code)]
    pub async fn create_user(&self, username: &str, password_hash: &str) -> Result<User, AppError> {
        let result = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (username, password_hash)
            VALUES ($1, $2)
            RETURNING *
            "#,
        )
        .bind(username)
        .bind(password_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }

    /// Get a user by username
    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, AppError> {
        let result = sqlx::query_as::<_, User>(
            r#"
            SELECT * FROM users WHERE username = $1
            "#,
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    /// Get a user by ID
    #[allow(dead_code)]
    pub async fn get_user_by_id(&self, user_id: i64) -> Result<Option<User>, AppError> {
        let result = sqlx::query_as::<_, User>(
            r#"
            SELECT * FROM users WHERE id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_struct() {
        let user = User {
            id: 1,
            username: "testuser".to_string(),
            password_hash: "hash".to_string(),
            is_active: true,
        };
        assert_eq!(user.id, 1);
        assert_eq!(user.username, "testuser");
    }
}
