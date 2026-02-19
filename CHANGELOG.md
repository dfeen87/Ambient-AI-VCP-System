# Changelog

All notable changes to this project will be documented in this file.

---

## [v2.2.0] – 2026-02-19

### Heartbeat UI — New Feature & Iterative Hardening

This release delivers a complete, production-quality **Heartbeat button** for node owners, replacing a bare browser `confirm()` dialog with a fully accessible modal experience. It also resolves every residual state-management bug found in the modal across multiple targeted follow-up PRs, and fixes six backend stability/security issues surfaced by deep code review.

---

### ✨ New Features

#### Dashboard – Heartbeat Button (PR #158)

Node owners now have a dedicated **Heartbeat** button in the node table, positioned between the **Eject** and **View** buttons.

- Blue-themed `.heartbeat-btn` styling that visually distinguishes it from Eject (red) and Observability (teal)
- Renders only for the authenticated node owner (`isOwner` gate), same access-control model as Eject
- Calls `PUT /api/v1/nodes/{id}/heartbeat`; refreshes the node list on success
- Includes an `aria-label` for screen-reader accessibility

**Before / After**

```
Before: [active]  [Eject]  [View]
After:  [active]  [Eject]  [Heartbeat]  [View]
```

#### Dashboard – Heartbeat Modal UI (PR #159)

The heartbeat confirmation was upgraded from a plain browser dialog to a rich, purpose-built modal:

- Displays the **Node ID** and **current status** so the operator knows which node they are acting on
- Explains that the action updates the `last_heartbeat` timestamp and prevents the node from being marked stale
- Shows the **timestamp returned by the API** on success, or a clear error message on failure
- Multiple close affordances: **×** button, **Cancel** button, overlay click, and **Escape key**
- DOM-based element creation throughout — no `innerHTML` with user data (XSS prevention)
- ARIA: `role="status"`, `aria-live="polite"`, `aria-hidden` toggling, and `aria-label` on all interactive controls
- Follows the same overlay/modal pattern as the existing observability view for visual consistency

#### Dashboard – Node Activity Button Repositioned (PR #157)

The **Node Activity (View)** observability button was reordered in `renderNodes()` to sit **after** the Eject button instead of before it.

```
Before: [Status]  [Node Activity]  [Eject]
After:  [Status]  [Eject]  [Node Activity]
```

Button visibility rules (owner-only, requires `observability_port`) are unchanged.

---

### 🐛 Bug Fixes

#### Dashboard

| PR | Summary |
|----|---------|
| #163 | **`nodes` variable scope** — `handleHeartbeat` referenced `nodes` but the array was only a block-scoped `const` inside `fetchData()`'s success branch, causing a `ReferenceError` on every click. The variable is now declared at module scope (`let nodes = []`) and assigned inside `fetchData()`. |
| #160 | **Modal button state not resetting** — after a successful heartbeat the send button switched to "Close" mode (`data-mode="close"`). Reopening the modal left the button stuck. `closeHeartbeatModal()` now resets button text, removes `data-mode`, clears status messages, re-enables the button, and resets CSS classes. |
| #162 | **Stale `data-mode` on HTTP error responses** — the HTTP-error handler inside `sendHeartbeatRequest()` restored button text and re-enabled the button but left the `data-mode` attribute set, so a subsequent open showed a "Close" button instead of "Send Heartbeat". Added `sendBtn.removeAttribute('data-mode')` to the HTTP-error path. |
| #165 | **Stale `data-mode` on network errors** — same root cause as #162 but in the `catch` block for network failures. Both error paths now fully reset all button state. Also applied `cargo fmt` to resolve a `cargo fmt --check` CI failure in `federated-learning`, `zk-prover` aggregator/prover/verifier files. |

#### Backend – Stability & Security (PR #164)

A deep code-review pass identified and fixed six latent bugs across the Rust crates:

| # | Crate | Issue | Fix |
|---|-------|-------|-----|
| 1 | `api-server` | **Rate limiter split-brain** — two separate `GLOBAL_LIMITER` statics were defined; the cleanup routine targeted the wrong one, so the active limiter was never flushed | Unified to a single authoritative static |
| 2 | `federated-learning` | **Aggregator panic on layer mismatch** — mismatched client model layer counts caused an index out-of-bounds panic during FedAvg aggregation | Added bounds check; returns a descriptive error instead of panicking |
| 3 | `federated-learning` | **Infinite noise in differential privacy** — `.unwrap()` on privacy-budget computation and degenerate `epsilon`/`delta` values (0 or negative) produced `Inf`/`NaN` noise and injected invalid gradients | Added early validation returning `Err` on degenerate parameters |
| 4 | `zk-prover` | **Predictable seed fallback** — key-deserialization failure silently fell back to `seed_from_u64(0)`, making the prover deterministic and trivially predictable | `ZKProver::new()` now returns `Result`; callers receive an explicit error |
| 5 | `zk-prover` | **Verifier panic on bad key data** — `ZKVerifier::new()` called `.expect()` on key deserialization, crashing the process on malformed input | Returns `Result`; callers handle the error explicitly |
| 6 | `api-server` | **Audit-queue serialization panic** — `PersistentAuditQueue::append` called `.expect()` on `serde_json` serialization, crashing on any non-serializable audit entry | Propagates `io::Error` via `?` operator |

10 new targeted unit tests were added for every new error path. Full test suite: **213 tests, 0 failures**.

---

### 📊 Code Quality

- ✅ **213 tests passing** (10 new tests added in this release)
- ✅ **Zero compiler warnings**
- ✅ **`cargo fmt` CI compliance restored**
- ✅ **Security review completed on all PRs**
- ✅ **Accessibility review: ARIA attributes, keyboard navigation, screen-reader support**

---

### 🔐 Security Summary

- No new attack surface introduced
- XSS vectors removed from heartbeat modal (DOM element creation, no `innerHTML` with external data)
- ZK prover and verifier no longer panic on untrusted key material — errors propagate cleanly
- Rate limiter split-brain closed — cleanup now correctly targets the active limiter
- Audit queue no longer panics on serialization edge cases

---

### 📋 Pull Requests

| PR | Title |
|----|-------|
| #157 | Fix node activity button position in registered node row |
| #158 | Add missing heartbeat button to node table |
| #159 | Replace heartbeat button alert with modal interface |
| #160 | Fix heartbeat modal button state not resetting between uses |
| #162 | Fix stale data-mode attribute on heartbeat error responses |
| #163 | Fix heartbeat button click silently failing due to inaccessible `nodes` variable |
| #164 | Deep code review: fix 6 stability and security issues |
| #165 | Fix stale data-mode attribute on send button error paths + rustfmt CI |

---

## [v2.1.0] – 2026-02-18

See [GitHub Release v2.1.0](https://github.com/dfeen87/Ambient-AI-VCP-System/releases/tag/v2.1.0) for full notes.

**Focus:** Node Observability & Owner Dashboard Enhancement — privacy-preserving, owner-only local observability for node operators.

---

## [v2.0.0] – earlier

See [GitHub Release v2.0.0](https://github.com/dfeen87/Ambient-AI-VCP-System/releases/tag/v2.0.0) for full notes.

**Focus:** Multi-backhaul connectivity layer — universal/open-access node WAN orchestration, seamless failover, hotspot fallback, and tether support.
