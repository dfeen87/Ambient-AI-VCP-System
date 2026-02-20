# Task Security Review: Wiring & Execution Flow

**Date**: 2025-05-24
**Scope**: `crates/api-server/src/state.rs` (Task Payload Analysis & Execution Wiring)

## 1. Summary

This review focused on the "wiring" of tasks—how user-submitted payloads are parsed, validated, and translated into execution instructions for nodes. The system generally uses strong typing and structured parsing (via `serde_json`), which mitigates many common injection risks. However, several areas were identified where resource exhaustion (DoS) or logical inconsistencies could occur.

## 2. Findings

### 2.1. Unbounded Input Processing (Potential DoS)
**Severity**: **Medium**
**Location**: `crates/api-server/src/state.rs` (`evaluate_arithmetic_expression`, `analyze_plain_text_prompt`, `extract_keywords`)

**Issue**: Functions like `evaluate_arithmetic_expression` and `analyze_plain_text_prompt` process string inputs without explicit length limits. While `axum` likely enforces a global request body limit, extremely long strings within that limit could still cause excessive CPU usage in the regex-like parsing or loop iterations (e.g., `extract_keywords` iterates over all tokens).
*   `evaluate_arithmetic_expression`: Iterates chars and builds vectors. A massive expression could consume significant memory/cycles.
*   `extract_keywords`: Uses `BTreeSet` and string splitting.

**Recommendation**: Enforce maximum length limits on `expression` (e.g., 1024 chars) and `prompt` (e.g., 10KB) fields *before* processing them in `analyze_task_payload`.

### 2.2. WASM Limit Overrides
**Severity**: **Low/Medium**
**Location**: `crates/api-server/src/state.rs` (`build_wasm_sandbox_limits`)

**Issue**: The `build_wasm_sandbox_limits` function allows the task payload to specify `timeout_ms`, `memory_limit_mb`, and `max_instructions`. There are no apparent upper bounds enforced here. A user could request `memory_limit_mb: 100000` or `timeout_ms: 3600000`. If the node blindly accepts these, it could lead to resource exhaustion on the worker node.
*   Note: The *node* might have its own hard caps, but the API should probably enforce reasonable policy limits (e.g., max 4GB RAM, max 60s timeout) to prevent "valid but abusive" tasks from reaching nodes.

**Recommendation**: Clamp these values to safe maximums in `build_wasm_sandbox_limits` (e.g., max 512MB or 2GB, max 30s).

### 2.3. Task Lifecycle Race Conditions
**Severity**: **Low**
**Location**: `crates/api-server/src/state.rs` (`complete_task_if_running`, `submit_task_result`)

**Issue**: Both functions check `status == 'running'` before proceeding. However, under high concurrency, two requests (e.g., a node submitting a result and a timeout firing simultaneously) could race.
*   The code correctly uses `UPDATE ... WHERE status = 'running'` which is atomic in PostgreSQL. This prevents double-completion at the DB level.
*   However, `submit_task_result` performs the check `task_status != "running"` *before* the transaction. If state changes between read and write, the `UPDATE` will simply affect 0 rows. This is safe but might result in a confusing "success" response (or lack of error) if the rows_affected check isn't strict.
*   Actually, `submit_task_result` does *not* check `rows_affected` on the `tasks` update. If the task was already completed by a race, the update does nothing, but the function proceeds to update assignments and return success.

**Recommendation**: Check `rows_affected` on the `UPDATE tasks` query in `submit_task_result`. If 0, it means the task is no longer in a runnable state (was completed by another race), and the operation should probably fail or return a specific "already completed" status.

### 2.4. Recursive JSON Parsing
**Severity**: **Low**
**Location**: `crates/api-server/src/state.rs` (`analyze_task_payload`)

**Issue**: `serde_json` has a default recursion limit (usually 128), which prevents stack overflow from deeply nested JSON. However, the custom analysis logic matches on `Value::Object` and `Value::Array`. It doesn't appear to recurse deeply itself (it mostly looks at top-level fields), so this risk is minimal.

**Recommendation**: No action needed; standard `serde` protections are sufficient.

## 3. Conclusion

The task wiring is relatively robust. The main improvements needed are **defensive limits** on string inputs and WASM configuration to prevent resource abuse, and a stricter **concurrency check** in the result submission flow.
