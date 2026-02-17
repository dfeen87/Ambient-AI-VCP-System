# Architecture Documentation

## System Overview

The Ambient AI + VCP System is a decentralized compute network designed to orchestrate AI workloads across heterogeneous edge devices with cryptographic verification.

## Component Architecture

### 1. Ambient Node

**Purpose**: Individual compute node with telemetry and health tracking

**Key Classes**:
- `AmbientNode`: Main node structure
- `TelemetrySample`: Real-time metrics
- `Reputation`: Task completion tracking
- `SafetyPolicy`: Circuit breaker configuration

**Health Scoring Algorithm**:
```
health_score = (bandwidth_score × 0.4) 
             + (latency_score × 0.3) 
             + (compute_score × 0.2) 
             + (reputation_score × 0.1)
```

**Safety Mechanisms**:
- Temperature monitoring (max 85°C)
- Latency thresholds (max 100ms)
- Error count tracking (max 25 errors)

### 2. WASM Execution Engine

**Purpose**: Secure sandboxed execution of WASM modules

**Runtime**: WasmEdge SDK

**Resource Limits**:
- Memory: 512MB (default)
- Timeout: 30 seconds
- Max instructions: 10 billion
- Gas metering: Enabled

**Security**:
- No filesystem access
- No network access
- Memory isolation per task
- Deterministic execution verification

### 3. ZK Proof System

**Purpose**: Cryptographic verification of execution correctness

**Current Implementation**: Groth16 verification on BN254 via the `zk-prover` crate.

**Validation Pipeline**:
- API request validation constrains proof/public input payload sizes and encoding.
- Proof bytes and public inputs are decoded and passed to the verifier.
- Verification result is persisted and returned in API responses for downstream policy checks.

**Notes**:
- The system keeps the verification layer modular, so additional proving systems can be introduced over time.
- Existing integration is designed for production request/response flows rather than demo-only placeholders.

### 4. Mesh Coordinator

**Purpose**: Network-wide orchestration and task assignment

**Task Assignment Strategies**:
1. **Weighted**: Select node with highest health score
2. **Round-Robin**: Rotate through eligible nodes
3. **Least-Loaded**: Select node with lowest CPU usage
4. **Latency-Aware**: Select node with lowest latency

### 5. API Server Assignment Semantics

**Purpose**: Admission control and fair assignment of tasks to online nodes.

**Node Selection Rules**:
1. Node must be online and not soft-deleted.
2. Node capabilities must satisfy task policy (CPU, memory, bandwidth, optional GPU).
3. Node type must match task preference (`preferred_node_type`) or be universal (`any`).
4. Node cannot exceed per-node active assignment caps.

**Capacity Controls**:
- `MAX_CONCURRENT_TASKS_PER_NODE` (preferred)
- `MAX_ACTIVE_TASK_ATTACHMENTS_PER_NODE` (legacy alias)
- Default when unset/invalid/non-positive: `50`

**Lifecycle Semantics**:
- Assignments are treated as active while `disconnected_at IS NULL`.
- On task completion, active assignment rows are disconnected (not hard-deleted) to preserve history.
- Re-attachment only reactivates previously disconnected rows and avoids over-attaching nodes beyond each task's `min_nodes` requirement.

## Data Flow

### Task Execution Flow

```
1. Client → Coordinator: Submit Task
2. Coordinator → Node Registry: Select Node
3. Coordinator → Selected Node: Dispatch Task
4. Node → WASM Engine: Execute
5. WASM Engine → ZK Prover: Generate Proof
6. Node → Coordinator: Return Result + Proof
7. Coordinator → Verifier: Verify Proof
8. Coordinator → Settlement: Distribute Reward
9. Coordinator → Client: Return Result
```

## Technology Stack

- **Rust**: Core system implementation
- **WasmEdge**: WASM runtime
- **Tokio**: Async runtime
- **serde**: Serialization
- **sha3**: Cryptographic hashing
