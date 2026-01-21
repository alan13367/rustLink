-- Create URLs table
CREATE TABLE IF NOT EXISTS urls (
    id BIGSERIAL PRIMARY KEY,
    short_code VARCHAR(16) NOT NULL UNIQUE,
    original_url TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    click_count BIGINT NOT NULL DEFAULT 0,
    last_clicked_at TIMESTAMPTZ
);

-- Index for fast lookups by short_code
CREATE INDEX IF NOT EXISTS idx_urls_short_code ON urls(short_code);

-- Index for expiry cleanup queries
CREATE INDEX IF NOT EXISTS idx_urls_expires_at ON urls(expires_at) WHERE expires_at IS NOT NULL;

-- Index for click stats
CREATE INDEX IF NOT EXISTS idx_urls_click_count ON urls(click_count);
