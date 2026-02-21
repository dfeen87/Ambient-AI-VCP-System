# **Security Hardening • Sign-In Reliability • Relay Keepalive**

**Version 2.8.0**

Version 2.8.0 focuses on three pillars: security hardening across the authentication and P2P policy layers, reliable sign-in connectivity for the dashboard, and new relay-resilience primitives that keep `connect_only` WAN links alive and detect silently-vanished nodes.

---

# 🔒 Security Hardening

## JWT Middleware — AppState Injection (PR #229)

`jwt_auth_middleware` was crashing because `AppState` was not present in request extensions. The router now includes `.layer(Extension(state))` so the middleware can reach the shared state without panicking.

## P2P Policy Sync — Ed25519 Signature Verification (PR #229)

`PeerPolicySyncMessage` now carries a mandatory Ed25519 signature. `import_peer_sync` verifies the signature before accepting any policy snapshot, closing an offline-mode attack surface where an unauthenticated peer could inject arbitrary policies into the mesh.

## Middleware Cleanup (PR #229, #230, #231)

- Removed the redundant `require_scope_middleware` (was functionally identical to `require_admin_middleware` for JWT users)
- Auth middleware refactored: dead code paths removed, unused imports dropped
- Clippy `doc_lazy_continuation` warning in auth middleware resolved by switching to an inner doc comment

---

# 🛠️ Sign-In & Dashboard Connectivity Fixes

## CORS Wildcard Panic Fixed (PR #226)

`.env.example` was shipping `CORS_ALLOWED_ORIGINS=*`. Axum's CORS layer panics on a bare wildcard at startup. The default is now a comma-separated list of explicit origins:

```
CORS_ALLOWED_ORIGINS=http://localhost:3000,http://localhost:5173
```

## Real Client IPs in Rate Limiting (PR #226)

`into_make_service_with_connect_info::<SocketAddr>()` is now used when binding the listener. Previously, all clients shared a single `127.0.0.1` rate-limit bucket. Direct connections no longer receive a 400 error from `extract_client_ip`.

A new test `test_extract_client_ip_direct_connection` verifies this path.

## Dashboard Auto-Connect on Login / Disconnect on Logout (PR #228)

`handleLogin` now validates and establishes the connection to the endpoint specified in the input field before attempting authentication. `logout` explicitly marks the connection as disconnected. Application state now matches user intent throughout the session lifecycle.

## Dashboard Polling Resilience (PR #227)

- **Removed the `!isApiConnected` early-return from `fetchData()`** — the guard was preventing data from loading on a fresh login
- **Transient failures no longer kill polling** — `setConnectionStatus(false)` was removed from the `fetchData()` catch block so a brief outage does not hide already-loaded node and task data
- **Interval guard relaxed** from `authToken && isApiConnected` → `authToken` so the client auto-reconnects when the endpoint recovers without a page reload

```js
// Before: polling stopped permanently after any outage or fresh login
setInterval(() => {
    if (authToken && isApiConnected) fetchData();
}, 5000);

// After: always retries while logged in
setInterval(() => {
    if (authToken) fetchData();
}, 5000);
```

---

# 🖥️ Frontend Enhancements

## Offline Node Ejection (PR #232)

The **Eject** button was hidden for nodes in `offline` status, preventing users from cleaning up disconnected nodes. The `canEject` condition in `renderNodes` no longer excludes `offline` nodes:

```js
// Before
const canEject = isOwner && n.status !== 'offline';

// After
const canEject = isOwner;
```

## Heartbeat Button Privacy (PR #232)

Confirmed and verified: the **Heartbeat** button remains visible only to the node owner. No regression was introduced by the eject fix.

---

# 🔗 Relay Resilience — `ambient-node`

## Hardware Keepalive (`BackhaulManager`) (PR #234)

`connect_only` relay nodes had no mechanism to prevent WAN hardware (LTE modems, USB adapters) from dropping idle connections.

- New `HardwareKeepaliveConfig` struct — default: **enabled**, **30 s interval**
- `hardware_keepalive_tick(now_secs) -> bool` — lock-free (`Arc<AtomicU64>`) interval gate; returns `true` and stamps the timestamp when the interval elapses
- `last_hardware_keepalive_secs()` — observability accessor
- `management_iteration()` calls `hardware_keepalive_tick()` after each probe cycle; the existing 5 s TCP health probes to `1.1.1.1:53` / `8.8.8.8:53` double as hardware keepalive traffic at no extra cost

## Internet-Required Signal (`LocalSessionManager`) (PR #234)

`internet_required() -> bool` returns `true` while any relay session is active. This is the explicit contract between the session lifecycle and the backhaul keepalive loop:

```rust
// connect_only task assigns session → keepalive becomes mandatory
mgr.activate_session(lease, now)?;
assert!(mgr.internet_required());

// session expires → keepalive obligation lifts
mgr.expire_stale_sessions(expired_ts, timeout);
assert!(!mgr.internet_required());
```

## Heartbeat Tracking & Stale Node Ejection (`NodeRegistry`) (PR #234)

The mesh coordinator previously had no way to detect nodes that had gone silent.

- `heartbeats: HashMap<NodeId, u64>` tracks last liveness timestamp per node
- `record_heartbeat(node_id, now) -> bool` — returns `false` for unknown nodes
- `last_heartbeat(node_id) -> Option<u64>` — observability accessor
- `eject_stale_nodes(now, timeout_secs) -> Vec<NodeId>` — removes nodes exceeding the timeout **and** nodes that never sent a heartbeat; returns ejected IDs for the caller to update routing state
- `unregister()` now clears the heartbeat entry to avoid ghost state

---

# 📝 Documentation Updates (PR #235)

README Phase changelog updated:

- Phase 2.9 demoted from 🆕
- **Phase 2.10 — Hardware Keepalive & Node Heartbeat Tracking (COMPLETED) 🆕** added, covering `hardware_keepalive_tick`, `NodeRegistry` heartbeat tracking (`record_heartbeat` / `is_node_alive`), and `LocalSessionManager::internet_required()`

---

# 🔢 Merged Pull Requests

| PR | Title |
|----|-------|
| [#226](https://github.com/dfeen87/Ambient-AI-VCP-System/pull/226) | Fix sign-in endpoint connectivity: CORS wildcard panic, missing ConnectInfo, extract_client_ip error |
| [#227](https://github.com/dfeen87/Ambient-AI-VCP-System/pull/227) | Fix dashboard connection after login; preserve node/task state during endpoint outages |
| [#228](https://github.com/dfeen87/Ambient-AI-VCP-System/pull/228) | Refactor sign-in to auto-connect and sign-out to disconnect |
| [#229](https://github.com/dfeen87/Ambient-AI-VCP-System/pull/229) | Security Hardening: Middleware Fix & P2P Signatures |
| [#230](https://github.com/dfeen87/Ambient-AI-VCP-System/pull/230) | Address all review findings from PR #229 (auth middleware cleanup) |
| [#231](https://github.com/dfeen87/Ambient-AI-VCP-System/pull/231) | Fix empty-line-after-doc-comment clippy warning in auth middleware |
| [#232](https://github.com/dfeen87/Ambient-AI-VCP-System/pull/232) | Allow ejecting offline nodes and verify heartbeat button privacy |
| [#234](https://github.com/dfeen87/Ambient-AI-VCP-System/pull/234) | Add hardware keepalive and node heartbeat tracking for connect_only relay resilience |
| [#235](https://github.com/dfeen87/Ambient-AI-VCP-System/pull/235) | cargo fmt fixes + README Phase 2.10 documentation |
