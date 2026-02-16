-- Migration: Add proof_artifacts table for ZK proof storage
-- Created: 2024-01-05

CREATE TABLE IF NOT EXISTS proof_artifacts (
    proof_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_id UUID REFERENCES tasks(task_id) ON DELETE CASCADE,
    run_id UUID REFERENCES task_runs(run_id) ON DELETE CASCADE,
    proof_data BYTEA NOT NULL,
    public_inputs BYTEA NOT NULL,
    circuit_id VARCHAR(64) NOT NULL,
    proof_system VARCHAR(32) NOT NULL DEFAULT 'groth16-bn254',
    verified BOOLEAN NOT NULL DEFAULT FALSE,
    verification_time_ms BIGINT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    verified_at TIMESTAMP WITH TIME ZONE,
    INDEX idx_proof_artifacts_task_id (task_id),
    INDEX idx_proof_artifacts_run_id (run_id),
    INDEX idx_proof_artifacts_verified (verified),
    INDEX idx_proof_artifacts_created_at (created_at)
);

COMMENT ON TABLE proof_artifacts IS 'Stores ZK proofs generated during task execution';
COMMENT ON COLUMN proof_artifacts.proof_data IS 'Serialized proof bytes';
COMMENT ON COLUMN proof_artifacts.public_inputs IS 'Public inputs for proof verification';
COMMENT ON COLUMN proof_artifacts.circuit_id IS 'Identifier for the ZK circuit used';
COMMENT ON COLUMN proof_artifacts.verified IS 'Whether the proof has been cryptographically verified';
