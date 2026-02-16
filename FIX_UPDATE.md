# Security Fix Update

This document summarizes concrete hardening changes applied from the security audit findings.

## 1) Refresh Token Cryptographic Hashing
- Replaced non-cryptographic `DefaultHasher` refresh token hashing with HMAC-SHA256 + server pepper.
- Added shared hashing helper and environment-based pepper resolution.
- Updated tests to assert deterministic cryptographic hash output properties.

Files:
- `crates/api-server/src/auth.rs`

## 2) WASM Sandbox Hard Enforcement
- Added canonical module path enforcement via `std::fs::canonicalize`.
- Enforced allowed storage roots via `WASM_ALLOWED_ROOTS`.
- Added runtime timeout cancellation using `tokio::time::timeout` around execution.
- Added tests for module-path containment checks.

Files:
- `crates/wasm-engine/src/lib.rs`

## 3) API Key Hashing & Storage
- Added API key HMAC-SHA256 hashing (`hash_api_key`) and removed plaintext key persistence path.
- Registration flow now stores only hash in `api_keys` table and returns plaintext key once.
- Added API key authentication middleware (`X-API-Key`) with hashed lookup.

Files:
- `crates/api-server/src/auth.rs`
- `crates/api-server/src/lib.rs`
- `crates/api-server/src/middleware/auth.rs`
- `crates/api-server/migrations/20240109000000_harden_key_storage.sql`

## 4) RBAC Authorization Policies
- Added centralized RBAC middleware (`require_admin_middleware`) using JWT claims.
- Added scope middleware (`require_scope_middleware`) for scoped permission checks.
- Applied middleware to admin-prefixed routes.

Files:
- `crates/api-server/src/middleware/auth.rs`
- `crates/api-server/src/lib.rs`

## 5) Secure Rate Limiting Enhancements
- Hardened client IP extraction with trusted proxy CIDR list (`TRUSTED_PROXY_CIDRS`).
- Added optional Redis-backed distributed limiter path using Lua script and `REDIS_URL`.

Files:
- `crates/api-server/src/rate_limit.rs`
- `crates/api-server/Cargo.toml`

## 6) PostgreSQL Migration Correction
- Rewrote migrations using PostgreSQL-valid standalone `CREATE INDEX` statements.
- Removed inline `INDEX ...` syntax from table declarations.
- Added migration smoke test and CI job to validate migration application.

Files:
- `crates/api-server/migrations/20240103000000_add_refresh_tokens.sql`
- `crates/api-server/migrations/20240104000000_add_task_runs.sql`
- `crates/api-server/migrations/20240105000000_add_proof_artifacts.sql`
- `crates/api-server/migrations/20240106000000_add_api_keys.sql`
- `crates/api-server/migrations/20240107000000_add_audit_log.sql`
- `crates/api-server/migrations/20240108000000_add_node_heartbeat_history.sql`
- `.github/workflows/ci.yml`
- `crates/api-server/tests/migration_sql_smoke.rs`

## 7) Metrics Middleware Attachment
- Attached Prometheus metrics middleware globally.
- Normalized metric names to requested counters/histograms (`http_request_duration`, `http_request_count`, `http_errors`).

Files:
- `crates/api-server/src/lib.rs`
- `crates/api-server/src/middleware/metrics.rs`

## 8) Input Validation Hardening
- Added deep JSON validation for task input payloads (depth, object/array cardinality, key/value size).
- WASM path canonicalization + allowed-root enforcement prevents traversal and out-of-root execution.

Files:
- `crates/api-server/src/models.rs`
- `crates/wasm-engine/src/lib.rs`

## 9) Middleware Improvements
- Tightened CORS origin handling: no wildcard, requires explicit configured origin(s).
- Removed `unsafe-inline` from CSP.
- Added `Expect-CT` response header.

Files:
- `crates/api-server/src/middleware/cors.rs`
- `crates/api-server/src/middleware/headers.rs`

## 10) Concurrency & Blocking Ops
- Wrapped bcrypt password verification in `spawn_blocking` via async helper.
- Wrapped ZK proof verification in `spawn_blocking` to avoid async runtime blocking.

Files:
- `crates/api-server/src/auth.rs`
- `crates/api-server/src/lib.rs`
- `crates/api-server/src/state.rs`

## Environment template updates
- Added production env placeholders for peppers, trusted proxies, CORS origins, wasm roots, Redis.

Files:
- `render.yaml`
