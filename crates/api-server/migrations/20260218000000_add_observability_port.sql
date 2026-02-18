-- Add observability_port column to nodes table for local-only observability access
-- This port is bound to 127.0.0.1 on the node's local machine and allows the node
-- owner to view read-only observability data for their own node.

ALTER TABLE nodes
    ADD COLUMN IF NOT EXISTS observability_port INTEGER;
