<p align="center">
  <img src="logo.png" alt="rustLink Logo" width="200"/>
</p>

# rustLink

A high-performance URL shortener built with Rust, featuring PostgreSQL persistence and Redis caching.

## Features

- **Fast**: Built on Axum with async/await for high concurrency
- **Persistent**: PostgreSQL database with automatic migrations
- **Cached**: Optional Redis caching for faster lookups
- **Custom Short Codes**: Support for custom aliases or auto-generated codes
- **Expiry Support**: Set optional expiry times for shortened URLs
- **Click Tracking**: Track click counts and last click time
- **RESTful API**: Clean JSON API with CORS support
- **CLI**: Easy-to-use command-line interface

## Tech Stack

- **[Axum](https://github.com/tokio-rs/axum)** - Web framework
- **[SQLx](https://github.com/launchbadge/sqlx)** - PostgreSQL with compile-time checked queries
- **[Redis](https://redis.io/)** - Optional caching layer
- **[Tokio](https://tokio.rs/)** - Async runtime
- **[Clap](https://github.com/clap-rs/clap)** - CLI argument parsing

## Prerequisites

- Rust 1.80+
- PostgreSQL 14+
- Redis (optional - server will run without it)

## Installation

1. Clone the repository:
```bash
git clone https://github.com/yourusername/rustlink.git
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

### Create Short URL

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
```

Returns `204 No Content` on success.

### Get Statistics

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
| `BASE_URL` | Base URL for short links | `http://localhost:3000` |
| `DEFAULT_EXPIRY_HOURS` | Default URL expiry (hours) | `720` (30 days) |

## Database Schema

```sql
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
```

## Development

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
