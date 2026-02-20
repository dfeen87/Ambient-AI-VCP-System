-- Store the cleartext connect-session token so the data-plane gateway process
-- running on the assigned node can retrieve and validate incoming relay
-- connections without needing the hash-verification path.
--
-- Access is restricted by the authenticated gateway-sessions endpoint
-- (GET /api/v1/nodes/:node_id/gateway-sessions) which only returns sessions
-- for nodes owned by the requesting user.
ALTER TABLE connect_sessions
    ADD COLUMN IF NOT EXISTS session_token_cleartext TEXT;
