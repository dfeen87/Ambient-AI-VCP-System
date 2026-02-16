-- Migration: Add task_runs table for execution tracking

CREATE TABLE IF NOT EXISTS task_runs (
    run_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_id UUID NOT NULL REFERENCES tasks(task_id) ON DELETE CASCADE,
    node_id VARCHAR(64) REFERENCES nodes(node_id) ON DELETE SET NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'pending',
    wasm_module_hash VARCHAR(64),
    inputs_hash VARCHAR(64),
    output_hash VARCHAR(64),
    execution_time_ms BIGINT,
    gas_used BIGINT,
    error_message TEXT,
    started_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_task_runs_task_id ON task_runs(task_id);
CREATE INDEX IF NOT EXISTS idx_task_runs_node_id ON task_runs(node_id);
CREATE INDEX IF NOT EXISTS idx_task_runs_status ON task_runs(status);
CREATE INDEX IF NOT EXISTS idx_task_runs_created_at ON task_runs(created_at);
