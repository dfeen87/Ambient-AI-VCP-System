-- Migration: Add task_runs table for execution tracking
-- Created: 2024-01-04

CREATE TABLE IF NOT EXISTS task_runs (
    run_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_id UUID NOT NULL REFERENCES tasks(task_id) ON DELETE CASCADE,
    node_id VARCHAR(64) REFERENCES nodes(node_id) ON DELETE SET NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'pending',
    started_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE,
    execution_time_ms BIGINT,
    gas_used BIGINT,
    result JSONB,
    error_message TEXT,
    wasm_module_hash VARCHAR(64),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_task_runs_task_id (task_id),
    INDEX idx_task_runs_node_id (node_id),
    INDEX idx_task_runs_status (status),
    INDEX idx_task_runs_created_at (created_at)
);

COMMENT ON TABLE task_runs IS 'Tracks individual execution runs of tasks';
COMMENT ON COLUMN task_runs.status IS 'Execution status: pending, running, completed, failed';
COMMENT ON COLUMN task_runs.execution_time_ms IS 'Total execution time in milliseconds';
COMMENT ON COLUMN task_runs.gas_used IS 'Amount of computational gas consumed';
