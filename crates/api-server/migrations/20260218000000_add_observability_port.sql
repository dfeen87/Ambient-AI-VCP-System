-- Add observability_port column to nodes table for local-only observability access
-- This port is bound to 127.0.0.1 on the node's local machine and allows the node
-- owner to view read-only observability data for their own node.

ALTER TABLE nodes
    ADD COLUMN IF NOT EXISTS observability_port INTEGER;

-- Create index on observability_port for faster lookups (optional, for future use)
CREATE INDEX IF NOT EXISTS idx_nodes_observability_port ON nodes(observability_port);
