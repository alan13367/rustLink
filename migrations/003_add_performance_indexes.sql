-- Add performance indexes for common queries

-- Composite index for pagination queries (order by created_at desc with limit/offset)
-- This improves the performance of the list URLs endpoint
CREATE INDEX IF NOT EXISTS idx_urls_created_expires 
ON urls(created_at DESC, expires_at);

-- Index for analytics queries (most clicked URLs)
-- This improves performance if we add analytics endpoints later
CREATE INDEX IF NOT EXISTS idx_urls_clicks_desc 
ON urls(click_count DESC, created_at DESC);

-- Composite index for cleanup operations (expired URLs)
-- This improves the performance of delete_expired_urls
-- Note: We can't use NOW() in partial index as it's not immutable
CREATE INDEX IF NOT EXISTS idx_urls_expires_at 
ON urls(expires_at) 
WHERE expires_at IS NOT NULL;
