# rustLink - URL Shortener

## Project Overview

A high-performance URL shortener built with Rust, using:
- **Axum** - Web framework
- **SQLx** - PostgreSQL database with compile-time query checking
- **Redis** - Caching layer (deadpool-redis managed pool)
- **Clap** - CLI with subcommands
- **thiserror** - Error handling (no `.unwrap()` or `.expect()` allowed)
- **Tokio** - Async runtime

## Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   HTTP      │────▶│    Axum     │────▶│  AppState   │
│  Client     │     │  Router     │     │  (Arc<>)    │
└─────────────┘     └─────────────┘     └──────┬──────┘
                                                 │
                     ┌────────────────────────────┼────────────────────────────┐
                     ▼                            ▼                            ▼
              ┌─────────────┐            ┌─────────────┐            ┌─────────────┐
              │   Redis     │            │ PostgreSQL  │            │   Config    │
              │   Cache     │            │  SQLx Pool  │            │  (env vars) │
              └─────────────┘            └─────────────┘            └─────────────┘
```

## File Structure

| File | Purpose |
|------|---------|
| `src/main.rs` | CLI entry point - parses `server` and `admin` subcommands |
| `src/error.rs` | `AppError` enum with `thiserror` - converts to HTTP responses |
| `src/config.rs` | `Config` struct loaded from environment variables |
| `src/models.rs` | `UrlEntry`, request/response DTOs |
| `src/db.rs` | `Repository` - SQLx database operations |
| `src/cache.rs` | `Cache` - Redis wrapper with connection pooling |
| `src/routes.rs` | Axum handlers and `AppState` definition |

## CLI Commands

```bash
cargo run -- server [--host HOST] [--port PORT] [--migrate]
cargo run -- admin clean-expired
cargo run -- admin migrate
cargo run -- admin stats
cargo run -- admin ping-cache
```

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | `/` | Create short URL (body: `{"url": "...", "custom_code"?, "expiry_hours"?}`) |
| GET | `/:code` | Resolve short URL → 301 redirect |
| GET | `/:code/info` | Get URL metadata |
| DELETE | `/:code` | Delete URL |
| GET | `/_stats` | Global stats |
| GET | `/_list?limit=50&offset=0` | List URLs |

## Environment Variables

Copy `.env.example` to `.env` and configure:

```bash
SERVER_HOST=127.0.0.1
SERVER_PORT=3000
DATABASE_URL=postgresql://...
REDIS_URL=redis://127.0.0.1:6379
SHORT_CODE_LENGTH=8
BASE_URL=http://localhost:3000
DEFAULT_EXPIRY_HOURS=720
```

## Database Schema

```sql
CREATE TABLE urls (
    id BIGSERIAL PRIMARY KEY,
    short_code VARCHAR(16) UNIQUE NOT NULL,
    original_url TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ,
    click_count BIGINT DEFAULT 0,
    last_clicked_at TIMESTAMPTZ
);
```

Migrations run automatically on server startup (use `--no-migrate` to skip).

## Error Handling Policy

**NEVER use `.unwrap()` or `.expect()`.** All errors must propagate via `?` through the `AppError` enum:

```rust
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),
    // ...
}

pub type AppResult<T> = Result<T, AppError>;
```

`AppError` implements `IntoResponse` for Axum, returning appropriate HTTP status codes.

## Short Code Generation

Uses `nanoid!` macro with alphanumeric alphabet (62 characters). Retries up to 10 times on collision.

## Caching Strategy

- Cache key format: `url:{short_code}`
- Default TTL: 3600 seconds (configurable)
- Cache is populated on cache miss from database
- Click updates spawn async tasks and invalidate cache

## State Management

`AppState` is wrapped in `Arc` for sharing across request handlers:

```rust
pub struct AppState {
    pub repository: Repository,  // Cloneable (PgPool is Arc internally)
    pub cache: Cache,            // Cloneable (Pool is Arc internally)
    pub base_url: String,
    pub default_expiry_hours: i64,
    pub short_code_length: usize,
}
```

## Adding New Features

1. **New route**: Add handler in `routes.rs`, register in `create_router()`
2. **New DB query**: Add method to `Repository` in `db.rs`
3. **New config**: Add field to `Config` struct, load from env in `from_env()`
4. **New CLI command**: Add variant to `Commands` or `AdminCommands` enum

## Testing Notes

- Repository methods have return type `AppResult<T>`
- Cache operations are fallible - handle gracefully if Redis is unavailable
- The server continues without cache if Redis connection fails on startup
