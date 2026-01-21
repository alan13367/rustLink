# rustLink - URL Shortener

## Project Overview

A high-performance URL shortener built with Rust, using:
- **Axum** - Web framework
- **SQLx** - PostgreSQL database with compile-time query checking
- **Redis** - Caching layer (deadpool-redis managed pool)
- **Clap** - CLI with subcommands
- **thiserror** - Error handling (no `.unwrap()` or `.expect()` allowed)
- **Tokio** - Async runtime
- **jsonwebtoken** - JWT token authentication
- **bcrypt** - Password hashing for user authentication
- **url** - URL validation and parsing
- **tower_governor** - Rate limiting middleware

## Architecture

```
┌─────────────┐     ┌──────────────────┐     ┌─────────────┐
│   HTTP      │────▶│    Axum        │────▶│  AppState   │
│  Client     │     │  Router + Rate  │     │  (Arc<>)    │
└─────────────┘     │  Limiting      │     └──────┬──────┘
                    └──────────────────┘            │
                         │                     │
                         │            ┌────────┴────────┬────────────┐
                         │            ▼                 ▼            ▼
                    ┌─────────┐   ┌─────────────┐  ┌─────────────┐
                    │Governor │   │   Redis     │  │ PostgreSQL  │
                    │Middleware│   │   Cache     │  │  SQLx Pool  │
                    └─────────┘   └─────────────┘  └─────────────┘
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
| `src/auth.rs` | JWT authentication service (token generation/validation) |
| `src/middleware.rs` | User repository extension for authentication |
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
| GET | `/_health` | Health check (DB + Redis connectivity, no auth) |
| POST | `/login` | Get JWT token (body: `{"username": "...", "password": "..."}`) |
| POST | `/` | Create short URL (body: `{"url": "...", "custom_code"?, "expiry_hours"?}`) |
| GET | `/:code` | Resolve short URL → 301 redirect |
| GET | `/:code/info` | Get URL metadata |
| DELETE | `/:code` | Delete URL **(requires JWT)** |
| GET | `/_stats` | Global stats **(requires JWT)** |
| GET | `/_list?limit=50&offset=0` | List URLs **(requires JWT)** |

## Environment Variables

Copy `.env.example` to `.env` and configure:

```bash
# Server
SERVER_HOST=127.0.0.1
SERVER_PORT=3000

# Database
DATABASE_URL=postgresql://...
DB_MAX_CONNECTIONS=10
DB_MIN_CONNECTIONS=1
DB_ACQUIRE_TIMEOUT_SECONDS=30

# Cache (Redis)
REDIS_URL=redis://127.0.0.1:6379
CACHE_MAX_CONNECTIONS=10
CACHE_DEFAULT_TTL_SECONDS=3600

# URL Configuration
SHORT_CODE_LENGTH=8
SHORT_CODE_MAX_ATTEMPTS=10
BASE_URL=http://localhost:3000
DEFAULT_EXPIRY_HOURS=720
CACHE_ENABLED=true
STRICT_URL_VALIDATION=true

# Authentication
JWT_SECRET=your-secret-key-here
JWT_EXPIRATION_HOURS=24

# Rate Limiting
RATE_LIMIT_PER_MINUTE=10
RATE_LIMIT_BURST=5

# CORS
ALLOWED_ORIGINS=http://localhost:3000,https://yourdomain.com
```

## Database Schema

```sql
-- URLs table
CREATE TABLE urls (
    id BIGSERIAL PRIMARY KEY,
    short_code VARCHAR(16) UNIQUE NOT NULL,
    original_url TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    click_count BIGINT DEFAULT 0,
    last_clicked_at TIMESTAMPTZ
);

-- Users table for authentication
CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    username VARCHAR(50) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_active BOOLEAN NOT NULL DEFAULT TRUE
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
    #[error("Authentication failed: {0}")]
    Unauthorized(String),
    #[error("User not found: {0}")]
    UserNotFound(String),
    // ...
}
```

`AppError` implements `IntoResponse` for Axum, returning appropriate HTTP status codes.

## Short Code Generation

Uses `nanoid!` macro with alphanumeric alphabet (62 characters). Retries up to 10 times on collision.

## Caching Strategy

- Cache key format: `url:{short_code}`
- Default TTL: 3600 seconds (configurable)
- Cache is populated on cache miss from database
- Click updates spawn async tasks and invalidate cache

## Rate Limiting

- **Strict limits** applied to sensitive endpoints:
  - `POST /` - URL creation
  - `POST /login` - Authentication
  - `DELETE /{code}` - URL deletion
  - `GET /_stats` - Statistics
  - `GET /_list` - URL listing

- **Lenient limits** applied to public endpoints:
  - `GET /{code}` - URL resolution
  - `GET /{code}/info` - URL metadata

- **No rate limiting** on health check endpoint:
  - `GET /_health` - Health monitoring

- Configurable via `RATE_LIMIT_PER_MINUTE` and `RATE_LIMIT_BURST` environment variables
- Uses IP-based rate limiting with `tower_governor`

## State Management

`AppState` is wrapped in `Arc` for sharing across request handlers:

```rust
pub struct AppState {
    pub repository: Repository,  // Cloneable (PgPool is Arc internally)
    pub cache: Cache,            // Cloneable (Pool is Arc internally)
    pub auth_service: AuthService, // JWT token generation/validation
    pub base_url: String,
    pub default_expiry_hours: i64,
    pub short_code_length: usize,
    pub short_code_max_attempts: u32,
    pub cache_enabled: bool,
    pub strict_url_validation: bool,
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
- Rate limiting is applied to prevent abuse - configure via environment variables
- Health check endpoint (`/_health`) provides service health monitoring

---

## Maintenance Notes

> **IMPORTANT**: Keep documentation updated after making code changes!
>
> When modifying the codebase, ensure the following documentation files are kept current:
>
> 1. **AGENTS.md** (this file) - Maintain architecture overview, file structure,
>    API endpoints, configuration options, database schema, and development patterns
>
> 2. **README.md** - Update user-facing documentation when:
>    - New features are added
>    - API endpoints change
>    - Configuration options are added
>    - Usage examples need updating
>    - Installation steps change
>
> ### Checklist Before Committing
>
> After code changes, verify you've updated:
>
> - [ ] File structure (added/removed files in this document)
> - [ ] Architecture overview (new components, data flow changes)
> - [ ] API endpoints (new/modified endpoints, auth requirements)
> - [ ] Configuration (new environment variables)
> - [ ] Database schema (new tables, migrations)
> - [ ] Error handling (new error variants)
> - [ ] README.md if user-facing features changed
>
> This ensures the documentation remains an accurate reference for future development.

