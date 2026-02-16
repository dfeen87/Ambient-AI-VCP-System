# Node Security and Lifecycle Management

## Overview

This document describes the security enhancements implemented for node registration and lifecycle management in the Ambient AI VCP System.

## Problem Statement

The original system had several critical security vulnerabilities:

1. **No Node Ownership** - Any authenticated user could register any node_id
2. **No Deregistration** - Nodes could not be removed when going offline
3. **No Authorization** - Users could potentially manipulate nodes they didn't own
4. **No Offline Detection** - No mechanism to detect stale or offline nodes

## Solution: Node Ownership and Lifecycle Management

### Database Schema Changes

A new migration `20240102000000_add_node_ownership.sql` adds:

```sql
-- Link nodes to users
ALTER TABLE nodes ADD COLUMN owner_id UUID;
ALTER TABLE nodes ADD CONSTRAINT fk_nodes_owner 
    FOREIGN KEY (owner_id) REFERENCES users(user_id) ON DELETE CASCADE;

-- Track heartbeat for offline detection
ALTER TABLE nodes ADD COLUMN last_heartbeat TIMESTAMP WITH TIME ZONE DEFAULT NOW();

-- Soft delete support
ALTER TABLE nodes ADD COLUMN deleted_at TIMESTAMP WITH TIME ZONE;
```

### Security Features

#### 1. Node Ownership Verification

All node operations now verify ownership:

```rust
pub async fn check_node_ownership(&self, node_id: &str, user_id: Uuid) -> ApiResult<bool>
```

Only the user who registered a node can:
- Update the node's heartbeat
- Delete/deregister the node
- Modify the node's status

#### 2. Authenticated Node Registration

The `POST /api/v1/nodes` endpoint now requires:
- Valid JWT authentication token
- User ID extracted from the token
- Ownership link created automatically

```rust
async fn register_node(
    State(state): State<Arc<AppState>>,
    auth_user: auth::AuthUser,  // ← JWT required
    Json(registration): Json<NodeRegistration>,
) -> ApiResult<(StatusCode, Json<NodeInfo>)>
```

#### 3. Node Deletion (Soft Delete)

New endpoint: `DELETE /api/v1/nodes/{node_id}`

Features:
- Requires JWT authentication
- Verifies node ownership
- Performs soft delete (sets `deleted_at` timestamp)
- Maintains audit trail
- Sets status to 'offline'

Response:
```json
{
  "message": "Node deleted successfully",
  "node_id": "node-123"
}
```

Error handling:
- Returns 404 if node doesn't exist OR user doesn't own it (prevents information leakage)

#### 4. Heartbeat Mechanism

New endpoint: `PUT /api/v1/nodes/{node_id}/heartbeat`

Features:
- Requires JWT authentication
- Verifies node ownership
- Updates `last_heartbeat` and `last_seen` timestamps
- Can be used to detect stale nodes

Response:
```json
{
  "message": "Heartbeat updated successfully",
  "node_id": "node-123",
  "timestamp": "2024-02-16T12:00:00Z"
}
```

### API Changes

#### New Endpoints

| Method | Path | Auth Required | Purpose |
|--------|------|---------------|---------|
| DELETE | /api/v1/nodes/{node_id} | Yes (JWT) | Soft delete a node |
| PUT | /api/v1/nodes/{node_id}/heartbeat | Yes (JWT) | Update node heartbeat |

#### Modified Endpoints

| Method | Path | Change |
|--------|------|--------|
| POST | /api/v1/nodes | Now requires JWT authentication and links to user |
| GET | /api/v1/nodes | Excludes soft-deleted nodes |
| GET | /api/v1/nodes/{node_id} | Excludes soft-deleted nodes |

### Security Best Practices

#### For Node Operators

1. **Register with your own account**: Each user can only manage their own nodes
2. **Use heartbeat updates**: Send periodic heartbeat updates to indicate the node is online
3. **Deregister when offline**: Delete nodes when permanently taking them offline

Example workflow:
```bash
# 1. Register (requires login)
TOKEN=$(curl -X POST https://api.example.com/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"user1","password":"pass"}' | jq -r '.access_token')

# 2. Register node
curl -X POST https://api.example.com/api/v1/nodes \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "node_id": "my-node-001",
    "region": "us-west",
    "node_type": "compute",
    "capabilities": {
      "bandwidth_mbps": 1000,
      "cpu_cores": 16,
      "memory_gb": 64,
      "gpu_available": true
    }
  }'

# 3. Send periodic heartbeat (every 5 minutes)
curl -X PUT https://api.example.com/api/v1/nodes/my-node-001/heartbeat \
  -H "Authorization: Bearer $TOKEN"

# 4. Deregister when going offline
curl -X DELETE https://api.example.com/api/v1/nodes/my-node-001 \
  -H "Authorization: Bearer $TOKEN"
```

#### For System Administrators

1. **Monitor stale nodes**: Query for nodes with old `last_heartbeat` timestamps
2. **Review node ownership**: Audit which users own which nodes
3. **Cleanup deleted nodes**: Periodically purge old soft-deleted nodes

Example query for stale nodes:
```sql
SELECT node_id, owner_id, last_heartbeat, 
       EXTRACT(EPOCH FROM (NOW() - last_heartbeat)) as seconds_since_heartbeat
FROM nodes
WHERE deleted_at IS NULL
  AND last_heartbeat < NOW() - INTERVAL '1 hour'
ORDER BY last_heartbeat ASC;
```

### Error Handling

#### Common Error Responses

**401 Unauthorized** - Missing or invalid JWT token:
```json
{
  "error": "unauthorized",
  "message": "Missing or invalid authorization header"
}
```

**403 Forbidden / 404 Not Found** - Node doesn't exist or user doesn't own it:
```json
{
  "error": "not_found",
  "message": "Node node-123 not found or you don't have permission to delete it"
}
```

Note: We use 404 instead of 403 to prevent attackers from enumerating valid node IDs.

**409 Conflict** - Node ID already exists:
```json
{
  "error": "conflict",
  "message": "A resource with this identifier already exists"
}
```

### Migration Guide

#### For Existing Deployments

1. **Run the migration**: The system will automatically run the migration on startup
2. **Existing nodes**: Nodes registered before this update will have NULL owner_id
3. **Handle orphaned nodes**: Admin intervention required to assign ownership or delete

Example orphaned node cleanup:
```sql
-- Option 1: Assign to a system user
UPDATE nodes SET owner_id = 'system-user-uuid' WHERE owner_id IS NULL;

-- Option 2: Soft delete orphaned nodes
UPDATE nodes SET deleted_at = NOW(), status = 'offline' WHERE owner_id IS NULL;
```

### Future Enhancements

1. **Automatic Stale Node Cleanup**: Background job to soft-delete nodes with old heartbeats
2. **Node Transfer**: Allow transferring node ownership between users
3. **Rate Limiting**: Per-user limits on number of nodes
4. **Audit Logging**: Comprehensive logging of all node lifecycle events
5. **Node Verification**: Cryptographic proof that the registering user controls the node

## Security Considerations

### Attack Vectors Mitigated

1. ✅ **Unauthorized Node Registration**: Users can only register nodes under their own account
2. ✅ **Node Hijacking**: Users cannot delete or modify nodes they don't own
3. ✅ **Information Leakage**: 404 responses prevent node enumeration
4. ✅ **Stale Node DOS**: Heartbeat mechanism enables detection of offline nodes

### Remaining Considerations

1. ⚠️ **Node Identity Verification**: No cryptographic proof that the user controls the physical node
2. ⚠️ **Rate Limiting**: No per-user limit on number of nodes registered
3. ⚠️ **Audit Trail**: Lifecycle events not logged to a separate audit log

## References

- Database Migration: `/crates/api-server/migrations/20240102000000_add_node_ownership.sql`
- API Implementation: `/crates/api-server/src/lib.rs`
- Database Operations: `/crates/api-server/src/state.rs`
- Security Documentation: `/crates/api-server/SECURITY.md`
