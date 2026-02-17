# API Reference

Complete API documentation for the Ambient AI + VCP System.

## CLI Commands

### `ambient-vcp node`

Start an ambient compute node.

**Usage:**
```bash
ambient-vcp node --id <NODE_ID> --region <REGION> --node-type <TYPE>
```

**Arguments:**
- `--id, -i <NODE_ID>`: Unique identifier for the node
- `--region, -r <REGION>`: Geographic region (default: "us-west")
- `--node-type, -t <TYPE>`: Node type: compute, gateway, storage, validator, open_internet, any (default: "compute")

**Example:**
```bash
ambient-vcp node --id node-001 --region us-west --node-type compute
```

### `ambient-vcp coordinator`

Start a mesh coordinator.

**Usage:**
```bash
ambient-vcp coordinator --cluster-id <CLUSTER_ID> --strategy <STRATEGY>
```

**Arguments:**
- `--cluster-id, -c <CLUSTER_ID>`: Unique cluster identifier
- `--strategy, -s <STRATEGY>`: Task assignment strategy (default: "weighted")
  - `weighted`: Health score based
  - `round-robin`: Rotate through nodes
  - `least-loaded`: Lowest CPU usage
  - `latency-aware`: Lowest latency

**Example:**
```bash
ambient-vcp coordinator --cluster-id demo-cluster --strategy weighted
```

### `ambient-vcp health`

Run system health check.

**Usage:**
```bash
ambient-vcp health
```

**Output:**
```
Running system health check...
✓ Ambient Node module loaded
✓ WASM Engine module loaded
✓ ZK Prover module loaded
✓ Mesh Coordinator module loaded
All systems operational!
```

### `ambient-vcp info`

Show node information.

**Usage:**
```bash
ambient-vcp info --id <NODE_ID>
```

**Arguments:**
- `--id, -i <NODE_ID>`: Node ID to query

## Rust API

### ambient-node

#### `AmbientNode`

Main node structure.

```rust
pub struct AmbientNode {
    pub id: NodeId,
    pub telemetry: TelemetrySample,
    pub reputation: Reputation,
    safety_policy: SafetyPolicy,
}
```

**Methods:**

```rust
// Create a new node
pub fn new(id: NodeId, safety_policy: SafetyPolicy) -> Self

// Ingest telemetry data
pub fn ingest_telemetry(&mut self, sample: TelemetrySample)

// Calculate health score (0.0 - 1.0)
pub fn health_score(&self) -> f64

// Check if in safe mode
pub fn is_safe_mode(&self) -> bool

// Update reputation
pub fn update_reputation(&mut self, success: bool, delta: f64)
```

#### `TelemetrySample`

Telemetry metrics.

```rust
pub struct TelemetrySample {
    pub bandwidth_mbps: f64,
    pub avg_latency_ms: f64,
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub temperature_c: f64,
    pub power_watts: f64,
    pub timestamp: u64,
}
```

**Methods:**

```rust
// Calculate bandwidth score
pub fn bandwidth_score(&self) -> f64

// Calculate latency score
pub fn latency_score(&self) -> f64

// Calculate compute score
pub fn compute_score(&self) -> f64

// Check if healthy
pub fn is_healthy(&self) -> bool
```

#### `Reputation`

Reputation tracking.

```rust
pub struct Reputation {
    pub completed_tasks: u64,
    pub failed_tasks: u64,
    pub disputes: u64,
    pub total_compute_time_ms: u64,
}
```

**Methods:**

```rust
// Calculate reputation score (0.0 - 1.0)
pub fn score(&self) -> f64

// Record successful task
pub fn record_success(&mut self, delta: f64)

// Record failed task
pub fn record_failure(&mut self, delta: f64)

// Record dispute
pub fn record_dispute(&mut self)

// Get success rate
pub fn success_rate(&self) -> f64
```

### wasm-engine

#### `WasmEngine`

WASM execution engine.

```rust
pub struct WasmEngine {
    runtime: WasmRuntime,
    limits: SandboxLimits,
}
```

**Methods:**

```rust
// Create new engine
pub fn new(runtime: WasmRuntime, limits: SandboxLimits) -> Self

// Execute WASM call
pub async fn execute(&self, call: WasmCall) -> Result<WasmResult>

// Execute with trace
pub async fn execute_with_trace(&self, call: WasmCall) 
    -> Result<(WasmResult, ExecutionTrace)>

// Verify determinism
pub async fn verify_determinism(&self, module_hash: &str, inputs: &[u8]) -> bool

// Get limits
pub fn limits(&self) -> &SandboxLimits
```

#### `SandboxLimits`

Resource limits.

```rust
pub struct SandboxLimits {
    pub memory_mb: u32,
    pub timeout_seconds: u32,
    pub max_instructions: u64,
    pub gas_metering_enabled: bool,
}
```

**Presets:**

```rust
// Default limits
SandboxLimits::default()

// Strict limits
SandboxLimits::strict()

// Relaxed limits
SandboxLimits::relaxed()
```

### zk-prover

#### `ZKProver`

ZK proof generator (placeholder).

```rust
pub struct ZKProver {
    proving_key: ProvingKey,
    verification_key: VerificationKey,
}
```

**Methods:**

```rust
// Create new prover
pub fn new(proving_key: ProvingKey, verification_key: VerificationKey) -> Self

// Generate proof from trace
pub fn generate_proof(&self, trace: ExecutionTrace) -> Result<ZKProof>

// Get verification key
pub fn verification_key(&self) -> &VerificationKey
```

#### `ZKVerifier`

ZK proof verifier.

```rust
pub struct ZKVerifier {
    verification_key: VerificationKey,
}
```

**Methods:**

```rust
// Create new verifier
pub fn new(verification_key: VerificationKey) -> Self

// Verify proof
pub fn verify_proof(&self, proof: &ZKProof, public_inputs: &[u8]) -> bool

// Get proof size
pub fn proof_size(&self, proof: &ZKProof) -> usize
```

### mesh-coordinator

#### `MeshCoordinator`

Task orchestration.

```rust
pub struct MeshCoordinator {
    cluster_id: String,
    nodes: HashMap<String, AmbientNode>,
    strategy: TaskAssignmentStrategy,
}
```

**Methods:**

```rust
// Create new coordinator
pub fn new(cluster_id: String, strategy: TaskAssignmentStrategy) -> Self

// Register node
pub fn register_node(&mut self, node: AmbientNode)

// Unregister node
pub fn unregister_node(&mut self, node_id: &str)

// Select node for task
pub fn select_node_for_task(&self, requirements: TaskRequirements) 
    -> Option<&AmbientNode>

// Dispatch and reward
pub async fn dispatch_and_reward(&mut self, task: Task) 
    -> Result<TaskResult>

// Verify result
pub fn verify_result(&self, result: &TaskResult, proof: &ZKProof) -> bool

// Get cluster stats
pub fn cluster_stats(&self) -> ClusterStats
```

#### `TaskAssignmentStrategy`

Assignment strategies.

```rust
pub enum TaskAssignmentStrategy {
    Weighted,      // Health score based
    RoundRobin,    // Rotate through nodes
    LeastLoaded,   // Lowest CPU usage
    LatencyAware,  // Lowest latency
}
```

## Health Scoring

### Formula

```
health_score = (bandwidth_score × 0.4) 
             + (latency_score × 0.3) 
             + (compute_score × 0.2) 
             + (reputation_score × 0.1)
```

### Component Scores

**Bandwidth Score:**
```rust
bandwidth_score = min(bandwidth_mbps / 1000.0, 1.0)
```

**Latency Score:**
```rust
latency_score = max(1.0 - (avg_latency_ms / 100.0), 0.0)
```

**Compute Score:**
```rust
cpu_available = 100.0 - cpu_usage_percent
memory_available = 100.0 - memory_usage_percent
compute_score = (cpu_available + memory_available) / 200.0
```

**Reputation Score:**
```rust
success_rate = completed_tasks / (completed_tasks + failed_tasks)
dispute_penalty = min(disputes × 0.05, 0.3)
reputation_score = max(success_rate - dispute_penalty, 0.0)
```

## Error Handling

All async functions return `Result<T>` types:

```rust
use anyhow::Result;

pub async fn execute(&self, call: WasmCall) -> Result<WasmResult>
```

Common errors:
- `ModuleNotFound`: WASM module doesn't exist
- `TimeoutExceeded`: Execution exceeded time limit
- `ProofVerificationFailed`: ZK proof verification failed
- `NoEligibleNodes`: No nodes meet task requirements

## Future API Extensions (Phase 2)

- REST API for task submission
- WebSocket for real-time updates
- GraphQL for flexible queries
- gRPC for high-performance RPC


## API Server Runtime Configuration

### Task Assignment Capacity

These environment variables control how many active task attachments a single node can hold at once.

- `MAX_CONCURRENT_TASKS_PER_NODE`: primary configuration key.
- `MAX_ACTIVE_TASK_ATTACHMENTS_PER_NODE`: backward-compatible alias.
- Effective value rules:
  - Positive integer → accepted.
  - Missing, non-integer, or `<= 0` → defaults to `50`.

### Assignment Lifecycle Behavior

- A task assignment is considered active while `disconnected_at` is `NULL`.
- Task completion disconnects active assignments to free node capacity while keeping assignment history.
- Assignment selection avoids over-allocation by limiting new attachments to the number of nodes still needed to satisfy `min_nodes`.
- Reattachment upserts only reactivate previously disconnected assignment rows.
