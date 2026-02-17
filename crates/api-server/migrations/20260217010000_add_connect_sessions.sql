-- Add connect session state for connect_only task data-plane handoff.
CREATE TABLE IF NOT EXISTS connect_sessions (
    session_id VARCHAR(128) PRIMARY KEY,
    task_id UUID NOT NULL REFERENCES tasks(task_id) ON DELETE CASCADE,
    requester_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    node_id VARCHAR(64) NOT NULL REFERENCES nodes(node_id) ON DELETE CASCADE,
    tunnel_protocol VARCHAR(32) NOT NULL,
    egress_profile VARCHAR(64) NOT NULL,
    destination_policy_id VARCHAR(128) NOT NULL,
    bandwidth_limit_mbps DOUBLE PRECISION NOT NULL,
    session_token_hash VARCHAR(128) NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'active',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    last_heartbeat_at TIMESTAMP WITH TIME ZONE,
    ended_at TIMESTAMP WITH TIME ZONE
);

CREATE INDEX IF NOT EXISTS idx_connect_sessions_task_id ON connect_sessions(task_id);
CREATE INDEX IF NOT EXISTS idx_connect_sessions_requester_status
ON connect_sessions(requester_id, status);
CREATE INDEX IF NOT EXISTS idx_connect_sessions_expires_at ON connect_sessions(expires_at);
