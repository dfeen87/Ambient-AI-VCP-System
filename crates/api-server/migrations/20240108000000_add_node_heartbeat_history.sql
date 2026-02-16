-- Migration: Add node_heartbeat_history table for health tracking
-- Created: 2024-01-08

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
    recorded_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_node_heartbeat_node_id (node_id),
    INDEX idx_node_heartbeat_recorded_at (recorded_at),
    INDEX idx_node_heartbeat_health_score (health_score)
);

-- Create a hypertable for time-series data (if TimescaleDB is available)
-- SELECT create_hypertable('node_heartbeat_history', 'recorded_at', if_not_exists => TRUE);

COMMENT ON TABLE node_heartbeat_history IS 'Historical heartbeat data from nodes for health monitoring';
COMMENT ON COLUMN node_heartbeat_history.health_score IS 'Overall health score (0-100)';
COMMENT ON COLUMN node_heartbeat_history.cpu_usage IS 'CPU utilization percentage';
COMMENT ON COLUMN node_heartbeat_history.memory_usage IS 'Memory utilization percentage';
COMMENT ON COLUMN node_heartbeat_history.network_latency_ms IS 'Network latency in milliseconds';
