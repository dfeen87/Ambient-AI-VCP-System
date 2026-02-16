-- Add node ownership and lifecycle tracking
-- Safe additive migration (no destructive changes)

ALTER TABLE nodes
    ADD COLUMN IF NOT EXISTS owner_id UUID
        REFERENCES users(user_id)
        ON DELETE CASCADE;

ALTER TABLE nodes
    ADD COLUMN IF NOT EXISTS last_heartbeat TIMESTAMPTZ
        DEFAULT NOW();

ALTER TABLE nodes
    ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ;

-- Indexes to support authorization, cleanup, and queries
CREATE INDEX IF NOT EXISTS idx_nodes_owner_id
    ON nodes(owner_id);

CREATE INDEX IF NOT EXISTS idx_nodes_last_heartbeat
    ON nodes(last_heartbeat);

CREATE INDEX IF NOT EXISTS idx_nodes_deleted_at
    ON nodes(deleted_at);
