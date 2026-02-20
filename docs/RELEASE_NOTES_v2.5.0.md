# Release Notes — v2.5.0
**Client Connectivity Stack • Relay Traffic QoS • Heartbeat Completeness**

Version 2.5.0 delivers a complete client-connectivity solution for nodes that act as internet providers, introduces WAN-side traffic shaping to prevent relay sessions from starving node-internal traffic, and closes a visibility gap in heartbeat activity so that connect sessions appear in the activity log the moment they start. This release also extends AILEE integration test coverage to all six node types.

---

## ✨ New Features

### NcsiSpoofServer — Suppress ERR_INTERNET_DISCONNECTED on Relay Nodes
*PR #195*

When a VCP node is the upstream internet provider for a connected client, the client OS's native connectivity probes (NCSI on Windows, NetworkManager on Linux/GNOME) still travel the broken direct path, causing the OS to raise `ERR_INTERNET_DISCONNECTED` and block traffic even though the node is providing a working connection.

`NcsiSpoofServer` (`crates/ambient-node/src/gateway.rs`) is a lightweight Tokio TCP/HTTP server that answers these probes locally, disabled by default:

- `GET /connecttest.txt` → `"Microsoft Connect Test"` HTTP 200 (Windows NCSI)
- `GET /check_network_status.txt` → `"NetworkManager is online\n"` HTTP 200 (Linux NM/GNOME)
- All other paths → HTTP 204 No Content (Ubuntu, Apple captive-portal probes, etc.)

`handle_ncsi_connection` reads the request line with a 5-second timeout, drains headers, and writes the minimal HTTP response using split read/write halves.

```rust
// Spin up alongside the data-plane gateway when the node provides internet:
let ncsi = NcsiSpoofServer::new(NcsiSpoofConfig {
    listen_addr: "0.0.0.0:80".to_string(),
    enabled: true,
});
tokio::spawn(async move { let _ = ncsi.run().await; });
```

**8 new tests** — 3 unit tests for path dispatch, 1 for disabled-server early-exit, 4 integration tests that bind real ephemeral ports and assert correct HTTP response bodies.

---

### HttpConnectProxy — Route Browser Traffic Through a Relay Node
*PR #203*

`NcsiSpoofServer` suppresses the OS-level connectivity indicator, but browsers still fail to open actual connections because outbound TCP is blocked at the OS routing level. `HttpConnectProxy` closes that gap by providing a standard HTTP CONNECT tunnel running on the relay node.

`HttpConnectProxy` + `HttpConnectProxyConfig` (`crates/ambient-node/src/gateway.rs`):

- Configures listen address (default `0.0.0.0:3128`), bearer token, enabled flag, connect/idle timeouts
- `handle_connect_proxy` — parses the `CONNECT host:port HTTP/1.1` request line, drains headers capturing `Proxy-Authorization: Bearer` (case-insensitive), validates the token, connects upstream, replies `200 Connection Established`, then runs `copy_bidirectional` for the tunnel lifetime
- `405` — non-CONNECT method (plain-HTTP forwarding intentionally unsupported to prevent open-relay abuse)
- `407` — missing or wrong bearer token
- `502`/`504` — upstream connect failure/timeout

```rust
let proxy = HttpConnectProxy::new(HttpConnectProxyConfig {
    listen_addr: "0.0.0.0:3128".to_string(),
    session_token: "your-bearer-token".to_string(),
    enabled: true,
    connect_timeout_secs: 10,
    idle_timeout_secs: 300,
});
tokio::spawn(proxy.run());
```

Browser configuration: **Settings → Network → Manual proxy → `:3128`**.
The browser sends `CONNECT api.example.com:443 HTTP/1.1` + `Proxy-Authorization: Bearer your-bearer-token`; the proxy tunnels the TLS stream transparently.

**10 new tests** covering token validation, upstream dialing, bidirectional tunnelling, and error code paths.

---

### Relay QoS — WAN-Side Traffic Shaping for connect_only Sessions
*PR #197*

When an `open_internet` or `any` node runs a `connect_only` task, node-internal traffic (health probes, control-plane keepalives) competes equally with relayed client traffic on the WAN backhaul interface, degrading relay throughput and latency.

**New: `crates/ambient-node/src/backhaul/relay_qos.rs`**

- `RelayQosConfig` — configures guaranteed relay bandwidth floor, burst ceiling, node-internal traffic reserve, FQ-CoDel toggle, and DSCP EF value
- `RelayQosManager` — installs/removes Linux `tc` rules on the active WAN interface via `activate_on_interface()` / `deactivate_from_interface()`

`tc` topology installed on relay session start:

| Class | Priority | Guaranteed | Ceiling |
|---|---|---|---|
| `1:10` relay | 1 | `relay_min_bandwidth_kbps` (default 10 Mbps) | `relay_max_bandwidth_kbps` (default 1 Gbps) |
| `1:20` node-internal | 2 | `node_min_bandwidth_kbps` (default 1 Mbps) | — |

- HTB root qdisc; default class `1:10` (unmarked relay TCP connections prioritised without requiring end-to-end DSCP support)
- FQ-CoDel leaf on `1:10` for active queue management and bufferbloat reduction
- `u32` DSCP EF (46) TOS filter → `1:10`

`RelayQosConfig` is added to `BackhaulConfig` with safe production defaults; no behaviour change when `enabled: false`.

```rust
// Call when a connect_only session starts on an open_internet/any node
backhaul_manager.activate_relay_qos().await?;

// Call when the session ends
backhaul_manager.deactivate_relay_qos().await?;
```

Both methods are no-ops when no interface is currently active or when relay QoS is disabled.

---

### Heartbeat Activity — Immediate task_connected Events
*PR #202*

When a `connect_only` task transitioned to "connected" (active connect session), the node's `node_heartbeat_history` was not updated until the node sent its next explicit heartbeat, leaving the active task invisible in the heartbeat activity log.

**`state.rs` changes:**

- `start_connect_session` now calls the new private helper `record_task_connected_heartbeat_event` immediately after the session INSERT succeeds
- `record_task_connected_heartbeat_event` fetches the node's `health_score`, counts active `task_assignments`, and inserts a `task_connected` row into `node_heartbeat_history` with full metadata (`task_id`, `task_type`, `session_id`, `event`). Best-effort — DB errors are logged as warnings and do not affect the session start.
- `get_node_cleared_task_events` broadens its SQL filter from `status = 'task_cleared'` to `status IN ('task_cleared', 'task_connected')` so both lifecycle events surface via `GET /nodes/{id}/heartbeat/activity`

`GET /api/v1/nodes/{node_id}/heartbeat/activity` now immediately reflects a newly connected task without waiting for the next node heartbeat:

```json
{
  "events": [
    {
      "task_id": "...",
      "task_type": "connect_only",
      "session_id": "...",
      "event": "task_connected",
      "cleared_at": "2026-02-19T23:53:07Z"
    }
  ]
}
```

**`lib.rs`** — OpenAPI description for `get_node_heartbeat_activity` updated to reflect both event types.

---

## 🧪 Test Coverage Expansion

### AILEE Integration — All Six Node Types Verified
*PR #199*

AILEE integration tests previously covered only `compute`, `gateway`, `storage`, and `validator` node types, leaving `open_internet` and `any` unverified.

**New tests added (`ailee_integration.rs` + `ailee_integration_test.rs`)**:

- `test_open_internet_node_execution` and `test_any_node_type_execution` unit tests
- Per-type integration tests for `open_internet` and `any`
- Exhaustive `test_vcp_adapter_all_node_types` that iterates all six types and asserts non-empty output for each:

```rust
let node_types = ["compute", "gateway", "storage", "validator", "open_internet", "any"];
for node_type in node_types {
    let result = adapter
        .execute_with_context("Test for node type", TaskType::Chat, 0.5, &context)
        .await
        .unwrap_or_else(|e| panic!("AILEE execution failed for node_type='{node_type}': {e}"));
    assert!(!result.final_output.is_empty());
}
```

---

## 🔧 Code Quality
- `cargo fmt` compliance restored after each feature PR (PRs #196, #198, #201, #204 — applied rustfmt to `gateway.rs`, `relay_qos.rs`, `connectivity/mod.rs`, `ailee_integration_test.rs`)
- Zero compiler warnings maintained
- All CI quality gates passing

---

## Pull Requests Included

| PR | Summary |
|---|---|
| #195 | feat: `NcsiSpoofServer` to prevent ERR_INTERNET_DISCONNECTED when node provides internet |
| #196 | fix: apply `cargo fmt` to `gateway.rs` |
| #197 | feat: WAN-side relay QoS (`RelayQosManager`) for `connect_only` tasks on `open_internet`/`any` nodes |
| #198 | fix: apply `cargo fmt` to `relay_qos.rs` and `connectivity/mod.rs` |
| #199 | test: AILEE integration tests for `open_internet` and `any` node types; exhaustive all-types test |
| #201 | fix: apply `cargo fmt` to `ailee_integration_test.rs` |
| #202 | feat: auto-record `task_connected` heartbeat event on `start_connect_session` |
| #203 | feat: `HttpConnectProxy` to route browser traffic through a connected relay node |
| #204 | fix: apply `cargo fmt` to `gateway.rs` |

---

## Test Count
**246 → 274 passing tests** across all crates (+28 new tests this release)

| Crate | Before | After |
|---|---|---|
| `ambient-node` | 100 | 128 |
| Others | 146 | 146 |

---

## Security Summary
- No new attack surface introduced
- `HttpConnectProxy` accepts only the `CONNECT` method; plain-HTTP forwarding is explicitly rejected with `405` to prevent open-relay abuse
- All proxy connections require a pre-shared bearer token; missing or incorrect tokens return `407`
- `NcsiSpoofServer` responds only to connectivity-probe paths; all other requests receive `204 No Content`
- Relay QoS rules are scoped to the active WAN interface and are fully removed on session end
- `record_task_connected_heartbeat_event` is best-effort and cannot affect session establishment on DB error
