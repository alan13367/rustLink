<p align="center">
  <img src="assets/logo.png" alt="rustLink Logo" width="200"/>
</p>

# rustLink

A high-performance URL shortener built with Rust, featuring PostgreSQL persistence, Redis caching, and JWT authentication.

## Features

- **Fast**: Built on Axum with async/await for high concurrency
- **Persistent**: PostgreSQL database with automatic migrations
- **Cached**: Optional Redis caching for faster lookups
- **Custom Short Codes**: Support for custom aliases or auto-generated codes
- **Expiry Support**: Set optional expiry times for shortened URLs
- **Click Tracking**: Track click counts and last click time
- **RESTful API**: Clean JSON API with CORS support
- **CLI**: Easy-to-use command-line interface
- **JWT Authentication**: Secure admin endpoints with JWT token authentication
- **Rate Limiting**: Protect against abuse with configurable rate limiting
- **Graceful Shutdown**: Handle SIGTERM/SIGINT signals properly

## Tech Stack

- **[Axum](https://github.com/tokio-rs/axum)** - Web framework
- **[SQLx](https://github.com/launchbadge/sqlx)** - PostgreSQL with compile-time checked queries
- **[Redis](https://redis.io/)** - Optional caching layer
- **[Tokio](https://tokio.rs/)** - Async runtime
- **[Clap](https://github.com/clap-rs/clap)** - CLI argument parsing
- **[jsonwebtoken](https://github.com/Keats/jsonwebtoken)** - JWT token authentication
- **[bcrypt](https://github.com/Keats/bcrypt)** - Password hashing

## Prerequisites

- Rust 1.80+
- PostgreSQL 14+
- Redis (optional - server will run without it)

## Installation

1. Clone the repository:
```bash
git clone https://github.com/alan13367/rustlink.git
cd rustlink
```

2. Copy the example environment file and configure:
```bash
cp .env.example .env
```

3. Edit `.env` with your database credentials:
```env
DATABASE_URL=postgresql://postgres:password@localhost:5432/rustlink
REDIS_URL=redis://127.0.0.1:6379
BASE_URL=http://localhost:3000
```

4. Create the PostgreSQL database:
```bash
psql -U postgres -c "CREATE DATABASE rustlink;"
```

## Usage

### Start the Server

```bash
cargo run -- server
```

Options:
- `--host HOST` - Server host (default: from env)
- `--port PORT` - Server port (default: from env)
- `--no-migrate` - Skip database migrations

### Admin Commands

```bash
# Clean expired URLs
cargo run -- admin clean-expired

# Run migrations manually
cargo run -- admin migrate

# Show statistics
cargo run -- admin stats

# Ping Redis cache
cargo run -- admin ping-cache
```

## API Endpoints

### Login (Get JWT Token)

```http
POST /login
Content-Type: application/json

{
  "username": "admin",
  "password": "password123"
}
```

Response:
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "username": "admin"
}
```

**Note**: You need a JWT token to access admin endpoints (`/_stats`, `/_list`, `DELETE /{code}`). First, create a user in the database manually:

```sql
INSERT INTO users (username, password_hash) VALUES ('admin', '<bcrypt_hash_of_password>');
```

To generate a bcrypt hash, you can use an online tool or run: `echo -n "your_password" | bcrypt-cli`

### Create Short URL

**Note**: Requires no authentication.

```http
POST /
Content-Type: application/json

{
  "url": "https://example.com",
  "custom_code": "mylink",
  "expiry_hours": 24
}
```

Response:
```json
{
  "short_code": "mylink",
  "short_url": "http://localhost:3000/mylink",
  "original_url": "https://example.com",
  "expires_at": "2026-01-22T22:00:00Z"
}
```

### Resolve URL (Redirect)

```http
GET /{code}
```

Returns `301 Moved Permanently` redirect to the original URL.

### Get URL Info

```http
GET /{code}/info
```

Response:
```json
{
  "short_code": "mylink",
  "original_url": "https://example.com",
  "created_at": "2026-01-21T22:00:00Z",
  "expires_at": "2026-01-22T22:00:00Z",
  "click_count": 42,
  "last_clicked_at": "2026-01-21T23:30:00Z"
}
```

### Delete URL

```http
DELETE /{code}
Authorization: Bearer <your_jwt_token>
```

Returns `204 No Content` on success.

**Requires**: JWT authentication token.

### Get Statistics

```http
GET /_stats
Authorization: Bearer <your_jwt_token>
```

```http
GET /_stats
```

Response:
```json
{
  "total_urls": 100,
  "total_clicks": 5234,
  "active_urls": 85,
  "expired_urls": 15
}
```

### List URLs

```http
GET /_list?limit=50&offset=0
Authorization: Bearer <your_jwt_token>
```

Response:
```json
[
  {
    "short_code": "mylink",
    "original_url": "https://example.com",
    "created_at": "2026-01-21T22:00:00Z",
    "expires_at": "2026-01-22T22:00:00Z",
    "click_count": 42,
    "last_clicked_at": "2026-01-21T23:30:00Z"
  }
]
```

## Configuration

Configuration is loaded from environment variables or `.env` file:

| Variable | Description | Default |
|----------|-------------|---------|
| `SERVER_HOST` | Server bind address | `127.0.0.1` |
| `SERVER_PORT` | Server port | `3000` |
| `DATABASE_URL` | PostgreSQL connection string | (required) |
| `REDIS_URL` | Redis connection string | `redis://127.0.0.1:6379` |
| `SHORT_CODE_LENGTH` | Auto-generated code length | `8` |
| `SHORT_CODE_MAX_ATTEMPTS` | Max attempts to generate unique code | `10` |
| `BASE_URL` | Base URL for short links | `http://localhost:3000` |
| `DEFAULT_EXPIRY_HOURS` | Default URL expiry (hours) | `720` (30 days) |
| `CACHE_ENABLED` | Enable/disable Redis caching | `true` |
| `STRICT_URL_VALIDATION` | Use strict URL validation | `true` |
| `JWT_SECRET` | Secret key for JWT tokens | (required for auth) |
| `JWT_EXPIRATION_HOURS` | JWT token expiration (hours) | `24` |
| `RATE_LIMIT_PER_MINUTE` | Rate limit for URL creation | `10` |
| `RATE_LIMIT_BURST` | Rate limit burst size | `5` |
| `ALLOWED_ORIGINS` | CORS allowed origins (comma-separated) | `*` |

## Database Schema

```sql
-- URLs table
CREATE TABLE urls (
    id BIGSERIAL PRIMARY KEY,
    short_code VARCHAR(16) UNIQUE NOT NULL,
    original_url TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    click_count BIGINT NOT NULL DEFAULT 0,
    last_clicked_at TIMESTAMPTZ
);

CREATE INDEX idx_urls_short_code ON urls(short_code);
CREATE INDEX idx_urls_expires_at ON urls(expires_at) WHERE expires_at IS NOT NULL;
CREATE INDEX idx_urls_click_count ON urls(click_count);

-- Users table for authentication
CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    username VARCHAR(50) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_active ON users(is_active) WHERE is_active = TRUE;
```

## Development

### Create Admin User

Before using admin endpoints, create an admin user in the database:

```sql
INSERT INTO users (username, password_hash)
VALUES ('admin', '$2b$12$...');
```

To generate a bcrypt password hash:

```bash
echo -n "your_password" | bcrypt-cli
```

Or use an online bcrypt generator.

### Run Tests

```bash
cargo test
```

### Run with Debug Output

```bash
RUST_LOG=debug cargo run -- server
```

### Build Release Binary

```bash
cargo build --release
```

The binary will be at `target/release/rustlink.exe`.

## License

MIT

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
