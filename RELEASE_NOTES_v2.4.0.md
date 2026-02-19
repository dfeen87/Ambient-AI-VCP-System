# Release Notes — v2.4.0

**Mesh Connectivity • Auth Hardening • Offline-First Rendering**

Version 2.4.0 ships the full internet-routing layer for mesh nodes, hardens the authentication stack, fixes a silent task-assignment gap that caused live nodes to stop receiving new work after registration, and eliminates the last external font CDN dependency — making the dashboard fully functional in air-gapped environments. This release also restores safe routing defaults so operators can adopt the backhaul manager without any risk to their host network.

---

## ✨ New Features

### Mesh Peer Router — Internet Path Resolution

*PR #178*

Nodes in the mesh now have a first-class mechanism to determine internet reachability and route through relay peers when direct connectivity is unavailable (`crates/ambient-node/src/peer_routing.rs`):

- **`NodeConnectivityStatus`** (`Online` / `Offline` / `Unknown`) — per-node internet reachability state
- **`NodeKind`** — derived from `NodeId::node_type`; `Universal` and `Open` nodes are relay-eligible, `Standard` nodes are not
- **`PeerRoute`** — resolved path to the internet; empty `hops` = direct connection, one hop = peer-relayed
- **`PeerRouter`** — resolves the best route for a node: direct if `Online`, otherwise a one-hop relay preferring `Universal` over `Open`, or `None` if no relay is available

`MeshCoordinator` now embeds a `PeerRouter` and exposes two new methods:

```rust
// Called when a node's backhaul state changes
coordinator.sync_connectivity("node-id", NodeConnectivityStatus::Online);

// Resolves the internet path for any registered node
let route = coordinator.find_peer_route("worker-1");
// route.hops == []                                          → direct
// route.hops == [RoutingHop { node_id: "relay-uni", kind: Universal }] → relayed
```

`register_node` / `unregister_node` now keep the peer router in sync automatically.

---

### DataPlaneGateway — Runtime Session Lifecycle Management

*PR #179*

Previously, nodes could continue relaying internet traffic after the API server ended a connect session, because the gateway's in-memory session store had no revocation path — sessions persisted until their `expires_at_epoch_seconds` timestamp elapsed regardless of explicit `stop_connect_session` or sweep-based expiry.

Two new methods on `DataPlaneGateway` close this gap:

- **`add_session(session: GatewaySession)`** — provisions a session into the live store at runtime, complementing startup-time loading via `new()` / `from_sessions_file()`
- **`revoke_session(session_id: &str) -> bool`** — immediately removes a session from the store; returns `true` if it existed. With an empty store the gateway rejects all relay handshakes.

```rust
// Endpoint connects → provision gateway session immediately
gateway.add_session(session).await;

// Endpoint disconnects / session swept → revoke immediately
let was_present = gateway.revoke_session(&session_id).await;
```

---

### Auth — Configurable bcrypt Cost + Non-blocking Password Hashing

*PR #185*

Two auth improvements that remove both a startup warning and a concurrency bottleneck:

- **`bcrypt_cost()`** — reads the `BCRYPT_COST` env var, defaults to `12`, clamps to the valid bcrypt range `4–31`. No rebuild required to tune cost for your deployment.
- **`hash_password_async()`** — new async wrapper that runs `hash_password` inside `tokio::task::spawn_blocking`, matching the existing `verify_password_async` pattern. Registration no longer blocks the async executor during hashing.

```rust
pub fn bcrypt_cost() -> u32 {
    std::env::var("BCRYPT_COST")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .map(|c| c.clamp(4, 31))
        .unwrap_or(12)
}
```

`.env.example` updated with `BCRYPT_COST=12`, commented-out pepper stubs, and generation instructions (`openssl rand -base64 32`).

---

### Self-Hosted Dashboard Fonts — Air-Gap / Offline Support

*PR #188*

The dashboard loaded fonts from Google Fonts CDN (`fonts.googleapis.com`), which silently broke rendering in offline and air-gapped environments — exactly the scenario this platform is designed to support.

- Font assets bundled locally: **Syne** and **JetBrains Mono** woff2 files (~148 KB total) served under `/assets/fonts`
- `package.json` added with `@fontsource/syne@5.2.7` and `@fontsource/jetbrains-mono@5.2.8` as npm dependencies
- `scripts/setup-fonts.js` copies only the required latin woff2 subset and generates `assets/fonts/fonts.css` with `@font-face` declarations
- `index.html`: replaced 3 Google Fonts CDN `<link>` tags with a single `<link rel="stylesheet" href="/assets/fonts/fonts.css">`
- `lib.rs`: `tower-http`'s `ServeDir` now serves `/assets/fonts`; path defaults to `crates/api-server/assets/fonts` but is overridable via the `FONTS_DIR` env var
- `Dockerfile`: Node.js 20 LTS installed (pinned via NodeSource) in the builder stage to regenerate fonts; `assets/fonts/` copied into the runtime image

---

## 🐛 Bug Fixes

### Heartbeat — Pending Tasks Never Assigned After Registration

*PR #187*

`update_node_heartbeat` updated timestamps and recorded history but never called `assign_pending_tasks_for_node`, so a live node would only receive new tasks at registration time — never again during normal operation.

```rust
// After recording heartbeat history row…
self.assign_pending_tasks_for_node(node_id).await?;
```

`assign_pending_tasks_for_node` already gates on `status = 'online'`, so no additional guard is needed. This brings heartbeat parity with the three other call-sites (registration, task completion, task deletion) that already trigger task sync.

---

## 🔒 Security & Configuration Fixes

### Routing — Safe-Default `monitor_only` Flag, Scoped IP Rules, Interface-Bound Probes

*PR #191*

Three gaps in the backhaul manager that could silently break host connectivity or produce false-healthy probe results:

**`monitor_only` flag (Gaps 1 & 2)**

Added `monitor_only: bool` to `RoutingConfig`, **defaulting to `true`**. Per-interface routing tables are still prepared, but no `ip rule` is inserted into the kernel policy database — the host's existing routing is completely untouched. Active failover now requires an explicit opt-in:

```toml
[routing_config]
monitor_only = false   # required to enable automatic failover
```

This is distinct from `execute_commands = false`, which remains a dry-run/test flag.

**Source-IP scoped rules (Gap 1)**

When `monitor_only = false`, the `ip rule` entry is now scoped to `from <interface-ip>` instead of `from all`. This bounds the rule's effect to traffic originating from that interface's address, leaving unrelated host traffic on the main routing table.

**Interface-bound health probes (Gap 3)**

`HealthProber` gains a `local_addr: Option<IpAddr>` field and a `with_local_addr(addr)` builder. When set, `tcp_probe` binds the outgoing socket to that address before connecting, forcing the probe through the correct interface regardless of active policy routing:

```rust
// Before — follows OS default route; eth0 probe may travel through wlan0
TcpStream::connect("1.1.1.1:53").await

// After — socket bound to eth0's address
TcpSocket::new_v4()?.bind("192.168.1.100:0")?.connect("1.1.1.1:53").await
```

---

### Dev Environment — All Pepper Variables Pre-configured

*PR #186*

`REFRESH_TOKEN_PEPPER is not configured` was emitted at `WARN` level during the first login in development, making it appear as a request-level failure when the login itself succeeded.

- `auth.rs`: `tracing::warn!` → `tracing::debug!` in `warn_missing_pepper_once` for the dev fallback. Production already panics on missing/weak peppers via `validate_hash_pepper_configuration()` — no change there.
- `docker-compose.yml`: Added `REFRESH_TOKEN_PEPPER` and `API_KEY_PEPPER` dev values; previously only `AUTH_HASH_PEPPER` and `CONNECT_SESSION_TOKEN_PEPPER` were set.
- `.env.example`: All four pepper variables now pre-configured with dev-safe defaults; users copying `.env.example → .env` won't see even the debug-level log.

---

## 📚 Documentation Updates

*PRs #183, #184, #192*

- **AILEE Mesh & Offline AI Infrastructure findings** — README updated with complete documentation of the offline-first mesh architecture, AILEE trust layer, and air-gap capabilities
- **README architecture section** expanded with `PeerRouter` description and `MeshCoordinator` integration details
- **Post-v2.3.0 Improvements** subsection added to the Key Features section
- **Phase 2.8 roadmap entry** added covering this release's scope
- **Test count** updated: 129 → **246 passing tests** across all crates
- **Footer version** updated from 2.1.0 → 2.3.0 (reflecting latest tagged release)

---

## 🔧 Code Quality

- `cargo fmt` compliance restored after each feature PR (PRs #182, #190, #193 — applied rustfmt to `gateway.rs`, `lib.rs`, `peer_routing.rs`, `health.rs`, `routing.rs`)
- Zero compiler warnings maintained
- All CI quality gates passing

---

## Pull Requests Included

| PR | Summary |
|---|---|
| #178 | Add mesh/peer routing and node internet connectivity analysis |
| #179 | Add dynamic session lifecycle management to `DataPlaneGateway` |
| #182 | fix: apply `cargo fmt` to `gateway.rs`, `lib.rs`, `peer_routing.rs` |
| #183 | docs: update README with complete AILEE Mesh & Offline AI Infrastructure findings |
| #184 | docs: replace Latest Release section with general software overview |
| #185 | auth: configurable bcrypt cost, `hash_password_async`, pepper env docs |
| #186 | fix: silence `REFRESH_TOKEN_PEPPER` dev-fallback warning; pre-configure all pepper env vars |
| #187 | fix: call `assign_pending_tasks_for_node` on heartbeat update |
| #188 | feat: self-host dashboard fonts via npm (offline/air-gap rendering fix) |
| #190 | fix: rustfmt line break in `fonts_dir` assignment (`lib.rs`) |
| #191 | fix: `monitor_only` routing default, source-IP rule scoping, interface-bound health probes |
| #192 | docs: README reflects all post-v2.3.0 changes, 246 test count |
| #193 | fix: apply `cargo fmt` to `health.rs` and `routing.rs` |

---

## Security Summary

- No new attack surface introduced
- `monitor_only = true` is the new safe default — the backhaul manager no longer modifies kernel routing tables unless explicitly opted in
- Source-IP scoped `ip rule` entries limit routing changes to the configured interface, preventing unintended traffic redirection
- Interface-bound TCP health probes prevent a compromised default route from producing false-healthy results
- Non-blocking `hash_password_async` eliminates thread-starvation risk under high concurrent registration load
- All pepper variables now have dev defaults; production startup-panic on missing/short peppers is unchanged
- `DataPlaneGateway::revoke_session` closes the window where a node could relay traffic after session expiry
