-- Migration: Add node_heartbeat_history table for health tracking

CREATE TABLE IF NOT EXISTS node_heartbeat_history (
    heartbeat_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    node_id VARCHAR(64) NOT NULL REFERENCES nodes(node_id) ON DELETE CASCADE,
    health_score NUMERIC(5, 2) NOT NULL,
    cpu_usage NUMERIC(5, 2),
    memory_usage NUMERIC(5, 2),
    disk_usage NUMERIC(5, 2),
    network_latency_ms INTEGER,
    active_tasks INTEGER DEFAULT 0,
    status VARCHAR(32) NOT NULL,
    metadata JSONB,
    recorded_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_node_heartbeat_node_id ON node_heartbeat_history(node_id);
CREATE INDEX IF NOT EXISTS idx_node_heartbeat_recorded_at ON node_heartbeat_history(recorded_at);
CREATE INDEX IF NOT EXISTS idx_node_heartbeat_health_score ON node_heartbeat_history(health_score);
