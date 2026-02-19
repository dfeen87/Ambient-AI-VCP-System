# Ambient AI VCP — v1.3.0 Release Notes

**Heartbeat Observability • AILEE Integration • Backend Hardening**

Version 1.3.0 deepens the Heartbeat workflow introduced in 1.2.0 with rich node-side observability, activates the AILEE ∆v efficiency metric, and adds peer-to-peer policy synchronisation so nodes can stay coordinated even when the central API is unreachable. This release also fixes a production-affecting security misconfiguration and ships the back-end persistence that makes heartbeat history meaningful.

---

## ✨ New Features

### Heartbeat Modal — Connected Tasks & Activity Log

*PR #167*

The Heartbeat modal now surfaces real-time node context at a glance:

**Active Tasks panel**

- Filters the module-level `tasks` snapshot by `assigned_nodes` and renders each matching task as a row with a truncated task ID, task type, and status badge.
- Shows "No tasks currently assigned to this node" when the list is empty.

**Activity Log panel**

- On every "Send Heartbeat" click, logs the outgoing `PUT` URL with an ISO timestamp, then appends the HTTP status and round-trip latency (green on success, red on error).
- Both panels reset to hidden on modal close.

*Plumbing:* `tasks` was promoted from a `fetchData`-local `const` to a module-level `let` so `openHeartbeatModal` can read the current snapshot without a redundant fetch.

---

### Heartbeat Modal — Pre-populated Task History

*PR #168*

The Activity Log is now populated the moment the modal opens, not only after the first heartbeat send:

- **Finished tasks** (`completed ✓` / `failed ✗`) that were assigned to this node appear immediately.
- **Disconnected tasks** — active `connect_only` sessions where this node is no longer listed in `assigned_nodes` — are shown with a "disconnected from node" label.
- A `— — —` separator divides pre-populated history entries from live HTTP ping entries added by subsequent heartbeat sends.
- The "Connected Tasks" section was renamed to **"Active Tasks"** and now filters to non-terminal tasks only, eliminating duplication between the task list and the log.

Example log output on modal open:

```
[2026-02-19T15:40:01.000Z] Task a1b2c3d4e5f6… [inference] — finished (completed) ✓
[2026-02-19T15:40:01.001Z] Task 9f8e7d6c5b4a… [inference] — finished (failed) ✗
[2026-02-19T15:40:01.002Z] Task 123abc456def… [connect_only] — disconnected from node
```

---

### Heartbeat History — Cleared Task Events

*PR #172*

Tasks that are deleted (cleared) from the system were invisible in the modal's Activity Log because `DELETE FROM tasks` removes the row entirely. This release introduces a durable audit trail for task-clear events:

**Backend (`state.rs`)**

- `delete_task` now inserts a `node_heartbeat_history` row per assigned node at deletion time:

  ```sql
  INSERT INTO node_heartbeat_history
      (node_id, health_score, active_tasks, status, metadata, recorded_at)
  VALUES ($1, 0, 0, 'task_cleared', $2, NOW())
  -- metadata: { "task_id": "…", "task_type": "…", "event": "task_cleared" }
  ```

  Failures are warned but do not block the delete.

- New method `get_node_cleared_task_events`: ownership-checked query of `node_heartbeat_history` for `status = 'task_cleared'` rows, returned as a JSON array with a `cleared_at` field.

**New API endpoint**

`GET /api/v1/nodes/:node_id/heartbeat/activity` — returns `{ events: [...] }`. Registered in the protected router and OpenAPI spec.

**Frontend**

`openHeartbeatModal` converted to `async`. After building the existing active/finished/cleared-from-DB task lists, it fetches the new endpoint and appends one `err`-styled Activity Log entry per event:

```
Task 3f9a1c004b2e… [zk_proof] — cleared
```

---

### AILEE ∆v Metric

*PR #174*

Implements Equation 1 from the AILEE paper as a time-integrated accumulator in `crates/ailee-trust-layer/src/metric.rs`:

```
∆v = Isp · η · e^(−α·v₀²) · ∫ P_input(t) · e^(−α·w²) · e^(2α·v₀·v) / M(t) dt
```

- `AileeMetric` accumulates `AileeSample` observations via `integrate()` and exposes `delta_v()` on demand.
- Both resonance gates are clamped to `[-700, 700]` before `exp()` to prevent `f64` overflow at extreme telemetry values.
- `AileeParams` docs include calibration guidance (unit conventions, safe `alpha` ranges).

```rust
let mut metric = AileeMetric::default();
metric.integrate(&AileeSample::new(100.0, 0.5, 1.2, 10.0, 1.0)); // P, w, v, M, dt
let gain = metric.delta_v();
```

---

### Peer-to-Peer Policy Sync for Offline Nodes

*PR #174*

Nodes in `OfflineControlPlane` or `NoUpstream` state can now exchange verified policy snapshots directly — no control plane required (`crates/ambient-node/src/offline.rs`):

- `PeerPolicySyncMessage` carries a SHA3-256 integrity hash over the **full policy content** (destination strings + raw key bytes), so destination tampering invalidates the hash.
- `export_peer_sync()` / `import_peer_sync()` on `LocalSessionManager`.
- Import is strictly non-destructive — existing local entries are never overwritten by peer data.
- Every import is appended to the audit queue as `peer_sync_applied`.

```rust
// Node A (fresh cache) → Node B (stale, API offline)
let msg = node_a.export_peer_sync("node-A");
let added = node_b.import_peer_sync(&msg)?; // merges new policies only
```

| State | Internet egress | Peer sync |
|---|:---:|:---:|
| `OnlineControlPlane` | ✅ | ✅ |
| `OfflineControlPlane` | ✅ (cached) | ✅ |
| `NoUpstream` | ❌ | ✅ (receive) |

---

## 🐛 Bug Fixes

### Heartbeat Activity Log — Correct Labels and History Recording

*PR #171*

| Scenario | Before | After |
|---|---|---|
| Active task on node | Not shown in log | `Task abc123… [compute] — running (active)` 🟢 |
| Task cleared from node | `… — disconnected from node` 🔴 | `… — cleared from node` 🔴 |
| `connect_only` completed | `… — session ended (disconnected)` 🔴 | `… — session completed ✓` 🟢 |
| No finished/cleared tasks | Log hidden | Log shown if any active tasks exist |

Backend: `update_node_heartbeat` now inserts a row into `node_heartbeat_history` on every heartbeat, capturing `health_score`, `status`, and live active task count. Previously the table was created by migration but never written to.

### Heartbeat Modal — Missing Disconnected/Completed Tasks

*PR #169*

Root cause: `assigned_nodes` only contains rows where `disconnected_at IS NULL`, so once a `connect_only` session ends, the node vanishes from every task record and the frontend filters produced empty results.

**Backend**

- Added `former_assigned_nodes: Vec<String>` to `TaskInfo`, populated via a second subquery on `task_assignments WHERE disconnected_at IS NOT NULL`.
- Updated both `list_tasks` and `get_task` SQL to include the new column.

```sql
COALESCE(
    (SELECT ARRAY_AGG(ta.node_id) FROM task_assignments ta
     WHERE ta.task_id = t.task_id AND ta.disconnected_at IS NOT NULL),
    ARRAY[]::VARCHAR[]
) as former_assigned_nodes
```

**Frontend**

- `finishedNodeTasks` now checks `assigned_nodes || former_assigned_nodes`.
- `disconnectedNodeTasks` filter replaced with `former_assigned_nodes.includes(nodeId) && !assigned_nodes.includes(nodeId) && non-terminal status`.
- Completed `connect_only` tasks render as **"session ended (disconnected)"** rather than "finished (completed) ✓".

---

## 🔒 Security & Configuration Fixes

### CONNECT_SESSION_TOKEN_PEPPER Warning Rate-Limiting

*PR #170*

`CONNECT_SESSION_TOKEN_PEPPER` was missing a `Once` guard, causing its "not configured" warning to fire on **every call** to `hash_connect_session_token` rather than once per process. Additionally, it was absent from `validate_hash_pepper_configuration`, meaning a production server could pass startup checks and then panic at runtime on the first connect session.

**Changes in `auth.rs`**

```rust
// Before — fell through to the unguarded wildcard:
match label {
    "REFRESH_TOKEN_PEPPER" => REFRESH_TOKEN_PEPPER_WARNING.call_once(warning),
    "API_KEY_PEPPER"       => API_KEY_PEPPER_WARNING.call_once(warning),
    _                      => warning(),  // fired every call
}

// After:
match label {
    "REFRESH_TOKEN_PEPPER"         => REFRESH_TOKEN_PEPPER_WARNING.call_once(warning),
    "API_KEY_PEPPER"               => API_KEY_PEPPER_WARNING.call_once(warning),
    "CONNECT_SESSION_TOKEN_PEPPER" => CONNECT_SESSION_TOKEN_PEPPER_WARNING.call_once(warning),
    _                              => warning(),
}
```

- Added `CONNECT_SESSION_TOKEN_PEPPER` to `validate_hash_pepper_configuration` so production startup now **fails fast** if the secret is missing or too short.
- Updated `render.yaml` with `generateValue: true` to auto-provision the secret on Render deployments.
- Added dev placeholder to `docker-compose.yml` and documented the variable in `.env.example`.

---

## 📚 Documentation Updates

*PR #174*

- README: Added architecture sections for the AILEE ∆v metric and peer-to-peer policy sync.
- README: Added node-state table, inline usage examples, updated test counts, and a Phase 2.7 roadmap entry.

---

## 🔧 Code Quality

- `cargo fmt` compliance restored after each PR (PR #173 — applied rustfmt to `state.rs`)
- Zero compiler warnings maintained
- All CI quality gates passing

---

## Pull Requests Included

| PR | Summary |
|---|---|
| #167 | Heartbeat modal: show active tasks and ping activity log |
| #168 | Add finished/disconnected task details to heartbeat modal Activity Log |
| #169 | Fix heartbeat modal activity log missing disconnected/completed task entries |
| #170 | Fix CONNECT_SESSION_TOKEN_PEPPER warning spam and missing production startup validation |
| #171 | Fix heartbeat activity logic: record history, correct labels, include active tasks |
| #172 | Record cleared tasks in node heartbeat activity log + new `/heartbeat/activity` endpoint |
| #173 | fix: apply rustfmt to state.rs |
| #174 | Add AILEE ∆v metric and peer-to-peer policy sync for API-disconnected node operation |

---

## Security Summary

- No new attack surface introduced
- `CONNECT_SESSION_TOKEN_PEPPER` now validated at startup; missing configuration fails fast rather than panicking at runtime
- Peer policy sync uses SHA3-256 content-addressed integrity checks; peer data never overwrites existing local entries
- All audit-worthy events (peer sync imports, task clears) are appended to the audit queue
- Heartbeat activity endpoint is owner-scoped (ownership checked before any data is returned)
