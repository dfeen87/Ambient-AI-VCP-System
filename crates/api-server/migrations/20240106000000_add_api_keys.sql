-- Migration: Add api_keys table for programmatic access
-- Created: 2024-01-06

CREATE TABLE IF NOT EXISTS api_keys (
    key_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    key_hash VARCHAR(128) NOT NULL UNIQUE,
    key_prefix VARCHAR(16) NOT NULL,
    name VARCHAR(128),
    scopes TEXT[] DEFAULT '{}',
    rate_limit_tier VARCHAR(32) DEFAULT 'general',
    expires_at TIMESTAMP WITH TIME ZONE,
    last_used_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    revoked_at TIMESTAMP WITH TIME ZONE,
    revoked_reason VARCHAR(255),
    INDEX idx_api_keys_user_id (user_id),
    INDEX idx_api_keys_key_hash (key_hash),
    INDEX idx_api_keys_key_prefix (key_prefix),
    INDEX idx_api_keys_expires_at (expires_at)
);

COMMENT ON TABLE api_keys IS 'API keys for programmatic access to the system';
COMMENT ON COLUMN api_keys.key_hash IS 'SHA-256 hash of the API key';
COMMENT ON COLUMN api_keys.key_prefix IS 'First 8 characters of the key for display purposes';
COMMENT ON COLUMN api_keys.scopes IS 'Array of permission scopes (e.g., ["tasks:read", "nodes:write"])';
COMMENT ON COLUMN api_keys.rate_limit_tier IS 'Rate limiting tier for this key';
