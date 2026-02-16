-- Migration: Add proof_artifacts table for ZK proof storage

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
    verified_at TIMESTAMP WITH TIME ZONE
);

CREATE INDEX IF NOT EXISTS idx_proof_artifacts_task_id ON proof_artifacts(task_id);
CREATE INDEX IF NOT EXISTS idx_proof_artifacts_run_id ON proof_artifacts(run_id);
CREATE INDEX IF NOT EXISTS idx_proof_artifacts_verified ON proof_artifacts(verified);
CREATE INDEX IF NOT EXISTS idx_proof_artifacts_created_at ON proof_artifacts(created_at);
