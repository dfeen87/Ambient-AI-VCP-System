-- Add migration script to create nodes and tasks tables

-- Ensure UUID generation function is available on managed PostgreSQL
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Create nodes table
CREATE TABLE IF NOT EXISTS nodes (
    node_id VARCHAR(64) PRIMARY KEY,
    region VARCHAR(32) NOT NULL,
    node_type VARCHAR(32) NOT NULL,
    bandwidth_mbps DOUBLE PRECISION NOT NULL,
    cpu_cores INTEGER NOT NULL,
    memory_gb DOUBLE PRECISION NOT NULL,
    gpu_available BOOLEAN NOT NULL DEFAULT FALSE,
    health_score DOUBLE PRECISION NOT NULL DEFAULT 100.0,
    status VARCHAR(32) NOT NULL DEFAULT 'online',
    registered_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    last_seen TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create index on region for faster queries
CREATE INDEX IF NOT EXISTS idx_nodes_region ON nodes(region);

-- Create index on status for health monitoring
CREATE INDEX IF NOT EXISTS idx_nodes_status ON nodes(status);

-- Create index on node_type for filtering
CREATE INDEX IF NOT EXISTS idx_nodes_type ON nodes(node_type);

-- Create tasks table
CREATE TABLE IF NOT EXISTS tasks (
    task_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_type VARCHAR(64) NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'pending',
    wasm_module TEXT,
    inputs JSONB NOT NULL DEFAULT '{}'::jsonb,
    result JSONB,
    proof_id VARCHAR(128),
    min_nodes INTEGER NOT NULL,
    max_execution_time_sec BIGINT NOT NULL,
    require_gpu BOOLEAN NOT NULL DEFAULT FALSE,
    require_proof BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMP WITH TIME ZONE
);

-- Create index on status for filtering active tasks
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);

-- Create index on created_at for time-based queries
CREATE INDEX IF NOT EXISTS idx_tasks_created_at ON tasks(created_at DESC);

-- Create index on task_type for filtering
CREATE INDEX IF NOT EXISTS idx_tasks_type ON tasks(task_type);

-- Create task_assignments table for many-to-many relationship
CREATE TABLE IF NOT EXISTS task_assignments (
    task_id UUID NOT NULL REFERENCES tasks(task_id) ON DELETE CASCADE,
    node_id VARCHAR(64) NOT NULL REFERENCES nodes(node_id) ON DELETE CASCADE,
    assigned_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    PRIMARY KEY (task_id, node_id)
);

-- Create index for reverse lookup
CREATE INDEX IF NOT EXISTS idx_task_assignments_node ON task_assignments(node_id);

-- Create users table for authentication
CREATE TABLE IF NOT EXISTS users (
    user_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(64) UNIQUE NOT NULL,
    password_hash VARCHAR(128) NOT NULL,
    role VARCHAR(32) NOT NULL DEFAULT 'user',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    last_login TIMESTAMP WITH TIME ZONE
);

-- Create index on username for login queries
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
