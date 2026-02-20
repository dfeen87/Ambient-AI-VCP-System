# **AILEE Performance • Node Lifecycle Integrity • Heartbeat Telemetry**

**Version 2.6.0**

Version 2.6.0 eliminates two Linux HTB upload‑speed bottlenecks in the relay and hotspot QoS layers, introduces parallel adapter execution and per‑adapter timeouts in the AILEE consensus engine, upgrades trust scoring to word‑level Jaccard similarity, closes a ghost‑assignment bug on node ejection, surfaces real‑time active‑task counts through the heartbeat API, and ships a comprehensive suite of end‑to‑end simulation tests across all three core subsystems.

---

# ✨ **New Features & Enhancements**

## **AILEE Parallel Execution + Per‑Adapter Timeout** *(PR #207)*

`ConsensusEngine` previously called `filter_adapters` and `generate_all` sequentially — O(N × RTT) — meaning one slow or hung adapter delayed every consensus round.

**`crates/ailee-trust-layer/src/consensus.rs`:**

- `filter_adapters` and `generate_all` now use `futures::future::join_all`, reducing wall time to O(max RTT)
- New `adapter_timeout_ms` builder field (default **30 s**) wraps each adapter call with `tokio::time::timeout`; a timed‑out adapter is treated as unavailable and excluded from the quorum
- Added `with_adapter_timeout_ms()` builder method

```rust
let engine = ConsensusEngine::new(2)
    .with_adapter_timeout_ms(5_000); // any adapter that takes > 5 s is skipped
```

**2 new tests:** `test_generate_all_returns_all_adapter_outputs`, `test_filter_adapters_parallel_respects_mode`

---

## **Word‑Level Jaccard Similarity in Trust Scoring** *(PR #207)*

`ConsistencyScore::compute_similarity` previously used a byte‑length denominator with character‑set intersection — incorrect for multi‑byte text and insensitive to word boundaries.

**`crates/ailee-trust-layer/src/trust.rs`:**

- Replaced with **word‑level Jaccard similarity**: `|unique‑word intersection| / |unique‑word union|`
  - Identical outputs → **1.0**; completely disjoint → **0.0**; partial overlap scales proportionally
- Legacy code path removed; the new metric applies to all consensus rounds

**1 new test:** `test_similarity_word_level_jaccard`

---

## **Hotspot QoS Upload‑Speed Fix + Directional Bandwidth Telemetry** *(PR #210)*

`hotspot/qos.rs` had the same Linux HTB parent‑class rate bug already fixed in `relay_qos.rs`: the root class `1:1` rate equalled the sum of child class minimums, hard‑capping all WAN egress at that sum.

**`crates/ambient-node/src/backhaul/hotspot/qos.rs`:**

- Added `max_bandwidth_kbps` field to `QosConfig` (default **1 Gbps**)
- Root HTB class rate now uses `max_bandwidth_kbps`; leaf class ceilings also set to `max_bandwidth_kbps`, removing both upload and download caps

**`crates/ambient-node/src/lib.rs` / `TelemetrySample`:**

- Added `upload_bandwidth_mbps` and `download_bandwidth_mbps` fields so AILEE can score true duplex performance
- `bandwidth_score()` now uses the **bottleneck direction** (`min(upload, download)`) when directional values are present; legacy `bandwidth_mbps` scalar is preserved as a fallback

```rust
// Example: node with fast download but slow upload
let sample = TelemetrySample {
    upload_bandwidth_mbps: Some(5.0),
    download_bandwidth_mbps: Some(100.0),
    ..Default::default()
};
// bandwidth_score uses 5.0 (the bottleneck)
```

**7 new tests:** `test_parent_htb_rate_equals_max_bandwidth`, `test_leaf_class_ceil_equals_max_bandwidth`, plus 5 targeted `bandwidth_score` tests (legacy, upload‑only, download‑only, bottleneck, symmetric)

---

## **Heartbeat Response Includes `active_tasks` Count** *(PR #218)*

The heartbeat endpoint previously returned only a boolean success flag, making it impossible for callers to determine how many tasks a node holds without a separate request.

**`crates/api-server/src/state.rs`:**

- `update_node_heartbeat` return type changed from `bool` to `Option<u32>` — the value reflects the active‑task count *after* any pending‑task sync triggered by the heartbeat

**`crates/api-server/src/routes.rs`** (heartbeat HTTP handler):

- Response body now includes `"active_tasks": <count>` alongside the existing acknowledgement

```json
// POST /nodes/{id}/heartbeat  →  200 OK
{ "active_tasks": 3 }
```

**1 new integration test:** `test_heartbeat_triggers_pending_task_assignment` — registers a node, completes its current task, submits a second task, fires a heartbeat, then asserts `active_tasks > 0` and the task transitions to `Running` with the node in `assigned_nodes`

---

# 🐛 **Bug Fixes**

## **Node Ejection Leaves Ghost Task Assignments** *(PR #213)*

`reject_node` only set `status = 'rejected'` on the node row; it never updated `task_assignments`. Because `get_assigned_nodes` and `update_task_status_from_assignments` key on `disconnected_at IS NULL`, ejected nodes stayed listed in `task.assigned_nodes` and tasks remained `Running` indefinitely.

**`crates/api-server/src/state.rs` — `reject_node`:**

- After marking the node rejected, collects all active (`disconnected_at IS NULL`) assignments for that node
- Sets `disconnected_at = NOW()` on those rows
- Recalculates each affected task's status — tasks drop back to `Pending` when node count falls below `min_nodes`
- Attempts reassignment via `assign_available_nodes_for_task` so tasks aren't permanently stuck

`delete_node` already performed this cleanup; `reject_node` was simply missing the equivalent post‑rejection logic.

**1 new integration test:** `test_node_rejection_disconnects_task_assignments`

---

## **Relay QoS HTB Parent Rate Bug** *(PR #207)*

`relay_qos.rs` root class `1:1` rate was set to `relay_min + node_min` (≈ 11 Mbps default), hard‑capping all WAN egress. Downloads bypassed `tc` rules and ran at full WAN speed, but uploads were stuck at the sum.

**`crates/ambient-node/src/backhaul/relay_qos.rs`:**

- Root class rate changed to `relay_max_bandwidth_kbps` (default 1 Gbps) — relay traffic can now burst to full WAN speed
- Node class `ceil` set to `relay_min + node_min` to preserve the node‑internal bandwidth reserve

**2 new tests:** `test_parent_htb_rate_equals_relay_max`, `test_node_class_ceil_does_not_consume_relay_bandwidth`

---

## **Heartbeat Modal Active Tasks Stale After Sending Heartbeat** *(PR #206)*

The "Active Tasks" section in the heartbeat modal was only populated on modal open and never refreshed after a heartbeat was sent, even though `fetchData()` refreshed the global `tasks` array.

**`crates/api-server/static/index.html`:**

- Extracted active‑tasks rendering from `openHeartbeatModal` into a standalone `renderHeartbeatTasks(nodeId)` helper
- `sendNodeHeartbeat()` now calls `renderHeartbeatTasks(nodeId)` after `await fetchData()`, keeping the modal in sync with server state

---

## **Heartbeat Modal Active Tasks Stale During Polling** *(PR #209)*

Tasks assigned to a node *after* the heartbeat modal was opened would never appear without closing and re‑opening it.

**`crates/api-server/static/index.html`:**

- `fetchData()` (5‑second polling loop) now checks whether the heartbeat modal is open and, if so, calls `renderHeartbeatTasks(currentNodeId)` to refresh the list in place

---

# 🧪 **Test Coverage**

## **Simulation Tests Across Core Subsystems** *(PR #215)*

Added integration‑level simulation tests that exercise real application code end‑to‑end and assert on actual runtime return values.

**28 new tests across 3 new test files:**

| Crate | File | Count | Coverage |
|---|---|---|---|
| `ambient-node` | `tests/simulation_test.rs` | 11 | `ConsensusEngine` result fields (trust score, model lineage, output prefix, hash integrity), `VcpExecutionContext` online/offline paths, `AmbientNode` health scoring, safe‑mode triggering, reputation decay |
| `mesh-coordinator` | `tests/simulation_test.rs` | 10 | `ClusterStats`, weighted and latency‑aware node selection, task dispatch happy/error paths, direct and relayed `PeerRoute` returns |
| `federated-learning` | `tests/simulation_test.rs` | 7 | FedAvg equal‑weight and sample‑weighted averaging, multi‑round version tracking, mismatched layer/weight error paths |

```rust
// Example — what a local consensus call returns
let result = ConsensusEngine::new(2)
    .execute(&request, adapters).await?;

// result.final_output  → "Chat response: Explain quantum computing"
// result.trust_score   → 0.85
// result.model_lineage → ["sim-local-a", "sim-local-b"]
// result.verify_hash() → true
// result.execution_metadata.was_offline → true
```

---

# 🔧 **Code Quality**

| PR | Change |
|---|---|
| #208 | Expand inline `if/else` and long `assert!` in `consensus.rs` to satisfy `cargo fmt` |
| #211 | Fix `doc_lazy_continuation` Clippy warnings in `qos.rs` doc comments |
| #212 | Align comment spacing in `test_bandwidth_score_uses_bottleneck_direction` |
| #214 | Reformat `assert_eq!` in `integration_test.rs` to multi‑line form |
| #216 | Normalise `assert!` formatting in `ambient-node/tests/simulation_test.rs` |
| #217 | Apply `cargo fmt` to three test files (`ambient-node`, `federated-learning`, `mesh-coordinator`) |

---

# 📊 **Test Count Delta**

| Subsystem | v2.5.0 | v2.6.0 | Δ |
|---|---|---|---|
| `ambient-node` | 117 | 135 | +18 |
| `api-server` (integration) | 20 | 22 | +2 |
| `ailee-trust-layer` | — | 6 | +6 |
| `mesh-coordinator` | — | 10 | +10 |
| `federated-learning` | — | 7 | +7 |
| **Total** | **≈ 274** | **≈ 317** | **+43** |

---

# 🔒 **Security Summary**

- No new attack surface introduced
- `reject_node` task‑assignment cleanup is performed inside the same database transaction scope as the node status update, preventing partial‑eject race conditions
- Heartbeat `active_tasks` count is derived from read‑only assignment queries; no new write paths exposed via the heartbeat endpoint
- Per‑adapter AILEE timeouts prevent a compromised or slow remote adapter from holding a consensus round open indefinitely
