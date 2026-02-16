-- Add node ownership and lifecycle management features

-- Add owner_id to nodes table to track which user owns each node
ALTER TABLE nodes ADD COLUMN owner_id UUID;

-- Add foreign key constraint to link nodes to users
ALTER TABLE nodes ADD CONSTRAINT fk_nodes_owner 
    FOREIGN KEY (owner_id) REFERENCES users(user_id) ON DELETE CASCADE;

-- Create index on owner_id for faster queries
CREATE INDEX IF NOT EXISTS idx_nodes_owner ON nodes(owner_id);

-- Add heartbeat timestamp for detecting stale nodes
ALTER TABLE nodes ADD COLUMN last_heartbeat TIMESTAMP WITH TIME ZONE DEFAULT NOW();

-- Create index on last_heartbeat for cleanup queries
CREATE INDEX IF NOT EXISTS idx_nodes_heartbeat ON nodes(last_heartbeat);

-- Update existing nodes to have NULL owner (migration compatibility)
-- In production, you may want to assign these to a system user or admin
UPDATE nodes SET last_heartbeat = last_seen WHERE last_heartbeat IS NULL;

-- Add soft delete support (optional - for audit trail)
ALTER TABLE nodes ADD COLUMN deleted_at TIMESTAMP WITH TIME ZONE;

-- Create index for active nodes queries (excludes soft-deleted)
CREATE INDEX IF NOT EXISTS idx_nodes_active ON nodes(node_id) WHERE deleted_at IS NULL;
