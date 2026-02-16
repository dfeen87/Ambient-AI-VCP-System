-- Migration: Add audit_log table for security tracking
-- Created: 2024-01-07

CREATE TABLE IF NOT EXISTS audit_log (
    log_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(user_id) ON DELETE SET NULL,
    action VARCHAR(64) NOT NULL,
    resource_type VARCHAR(64),
    resource_id VARCHAR(128),
    ip_address INET,
    user_agent TEXT,
    request_id VARCHAR(36),
    status VARCHAR(32) NOT NULL,
    error_message TEXT,
    metadata JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_audit_log_user_id (user_id),
    INDEX idx_audit_log_action (action),
    INDEX idx_audit_log_resource_type (resource_type),
    INDEX idx_audit_log_created_at (created_at),
    INDEX idx_audit_log_status (status)
);

COMMENT ON TABLE audit_log IS 'Audit trail of all significant actions in the system';
COMMENT ON COLUMN audit_log.action IS 'Action performed (e.g., "user.login", "task.submit", "proof.verify")';
COMMENT ON COLUMN audit_log.resource_type IS 'Type of resource affected (e.g., "user", "task", "node")';
COMMENT ON COLUMN audit_log.status IS 'Result status (e.g., "success", "failure", "unauthorized")';
COMMENT ON COLUMN audit_log.metadata IS 'Additional context data in JSON format';
