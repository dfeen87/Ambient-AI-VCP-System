-- Migration: Add refresh_tokens table for JWT token rotation
-- Created: 2024-01-03

CREATE TABLE IF NOT EXISTS refresh_tokens (
    token_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    token_hash VARCHAR(128) NOT NULL UNIQUE,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    revoked_at TIMESTAMP WITH TIME ZONE,
    revoked_reason VARCHAR(255),
    device_info VARCHAR(255),
    ip_address INET,
    INDEX idx_refresh_tokens_user_id (user_id),
    INDEX idx_refresh_tokens_token_hash (token_hash),
    INDEX idx_refresh_tokens_expires_at (expires_at)
);

COMMENT ON TABLE refresh_tokens IS 'Stores refresh tokens for JWT token rotation and revocation';
COMMENT ON COLUMN refresh_tokens.token_hash IS 'SHA-256 hash of the refresh token';
COMMENT ON COLUMN refresh_tokens.revoked_at IS 'Timestamp when token was revoked (NULL if still valid)';
