# AILEE Trust Layer Integration

## Overview

This document describes the integration boundary between **Ambient AI VCP** (Verifiable Computation Protocol) and the **AILEE Trust Layer** (external generative intelligence engine).

## Architecture

### Clean Separation of Concerns

```
┌─────────────────────────────────────────┐
│      Ambient AI VCP System              │
│  ┌─────────────────────────────────┐   │
│  │  VCP Substrate Layer            │   │
│  │  - Node lifecycle               │   │
│  │  - Connectivity management      │   │
│  │  - Task routing                 │   │
│  │  - Offline tolerance            │   │
│  └──────────┬──────────────────────┘   │
│             │                            │
│  ┌──────────▼──────────────────────┐   │
│  │  Integration Adapter            │   │
│  │  (AileeEngineAdapter)           │   │
│  │  - VCP context passing          │   │
│  │  - Result routing               │   │
│  └──────────┬──────────────────────┘   │
└─────────────┼──────────────────────────┘
              │ Clean Interface
┌─────────────▼──────────────────────────┐
│      AILEE Trust Layer (External)      │
│  ┌─────────────────────────────────┐   │
│  │  Trust Scoring Engine           │   │
│  │  - Confidence scoring           │   │
│  │  - Safety analysis              │   │
│  │  - Consistency checking         │   │
│  └─────────────────────────────────┘   │
│  ┌─────────────────────────────────┐   │
│  │  Consensus Engine               │   │
│  │  - Multi-model execution        │   │
│  │  - Output selection             │   │
│  │  - Lineage tracking             │   │
│  └─────────────────────────────────┘   │
└────────────────────────────────────────┘
```

### Responsibilities

#### Ambient AI VCP (Substrate)
- **Distributed execution** - Task scheduling and node management
- **Connectivity awareness** - Network state tracking
- **Offline tolerance** - Queue management and reconciliation
- **Node lifecycle** - Health monitoring and safety policies
- **NOT responsible for**: Trust scoring, consensus logic, model selection

#### AILEE Trust Layer (Intelligence)
- **Trust scoring** - Confidence, safety, consistency analysis
- **Consensus** - Multi-model output selection
- **Lineage tracking** - Model provenance and audit trail
- **Deterministic execution** - Reproducible results with hashing
- **NOT responsible for**: VCP substrate concerns, connectivity, routing

## Integration Points

### 1. Dependency Declaration

AILEE Trust Layer is declared as an **external dependency** in `ambient-node`:

```toml
[dependencies]
# AILEE Trust Layer - external dependency
ailee-trust-layer = { path = "../ailee-trust-layer" }
```

### 2. VCP Execution Context

VCP passes execution context to AILEE without leaking substrate internals:

```rust
pub struct VcpExecutionContext {
    /// Current connectivity state
    pub is_online: bool,
    /// Node locality (region, type)
    pub node_region: String,
    pub node_type: String,
    /// Execution constraints
    pub max_execution_time_ms: u64,
    pub allow_offline_execution: bool,
}
```

### 3. Integration Adapter

`AileeEngineAdapter` provides the clean boundary:

```rust
pub struct AileeEngineAdapter {
    consensus_engine: ConsensusEngine,
}

impl AileeEngineAdapter {
    pub async fn execute_with_context(
        &self,
        prompt: impl Into<String>,
        task_type: TaskType,
        trust_threshold: f64,
        vcp_context: &VcpExecutionContext,
    ) -> anyhow::Result<GenerationResult>
}
```

### 4. Interface Contract

#### Request (VCP → AILEE)
```rust
pub struct GenerationRequest {
    pub prompt: String,
    pub task_type: TaskType,
    pub trust_threshold: f64,
    pub execution_mode: ExecutionMode,
    pub allow_offline: bool,
}
```

#### Result (AILEE → VCP)
```rust
pub struct GenerationResult {
    pub final_output: String,
    pub trust_score: f64,
    pub model_lineage: Vec<String>,
    pub execution_metadata: ExecutionMetadata,
    pub input_hash: String,
    pub output_hash: String,
}
```

## Design Principles

### 1. No Logic Duplication
- AILEE logic is **never re-implemented** in VCP
- All trust, consensus, and scoring logic lives in AILEE crate
- VCP only provides integration glue

### 2. Substrate Agnostic
- AILEE has **zero dependencies** on VCP internals
- AILEE can be used by any substrate (not just VCP)
- Clean, versioned interfaces only

### 3. In-Process Invocation
- AILEE is invoked **in-process** (not over network)
- Direct function calls for efficiency
- No service discovery, URLs, or endpoints

### 4. Offline First
- AILEE supports **local-only** execution mode
- Deterministic behavior preserved offline
- Graceful degradation when disconnected

### 5. Fully Async
- All operations use **async/await**
- No blocking operations
- No global state

## Usage Patterns

### Pattern 1: Direct AILEE Usage (Substrate-Agnostic)

```rust
use ailee_trust_layer::{
    ConsensusEngine, GenerationRequest, TaskType, ExecutionMode,
    LocalModelAdapter, ModelAdapter,
};

let engine = ConsensusEngine::new(2);
let adapters: Vec<Box<dyn ModelAdapter>> = vec![
    Box::new(LocalModelAdapter::new("model-1")),
    Box::new(LocalModelAdapter::new("model-2")),
];

let request = GenerationRequest::new(
    "prompt",
    TaskType::Chat,
    0.7,
    ExecutionMode::Local,
    true,
);

let result = engine.execute(&request, adapters).await?;
```

### Pattern 2: VCP Integration (Context-Aware)

```rust
use ambient_node::{AileeEngineAdapter, VcpExecutionContext, TaskType};

let adapter = AileeEngineAdapter::new(2);

let context = VcpExecutionContext::new(
    is_online,      // From VCP connectivity layer
    node_region,    // From VCP node metadata
    node_type,      // From VCP node metadata
    time_budget_ms, // From VCP task constraints
    allow_offline,  // From VCP policy
);

let result = adapter.execute_with_context(
    "prompt",
    TaskType::Chat,
    0.7,
    &context,
).await?;
```

## Testing Strategy

### Unit Tests
- AILEE Trust Layer: 28 tests (all internal logic)
- VCP Integration: 5 tests (adapter behavior)

### Integration Tests
- Direct AILEE API: 10 tests (substrate-agnostic)
- VCP Adapter: 7 tests (VCP-specific context)

### Test Coverage
- ✅ Online execution
- ✅ Offline execution and resilience
- ✅ Deterministic replay validation
- ✅ Trust threshold enforcement
- ✅ Connectivity-aware mode selection
- ✅ Lineage tracking
- ✅ Hash verification
- ✅ Multiple task types

## Security Considerations

### 1. Trust Boundary
- AILEE is treated as a **trusted component**
- Trust scores provided by AILEE are authoritative
- VCP validates constraints but not trust logic

### 2. Determinism
- Input hashing ensures **reproducible requests**
- Output hashing enables **verification**
- Lineage tracking provides **audit trail**

### 3. Offline Safety
- Local models available when **disconnected**
- No network dependencies for core functionality
- Graceful degradation without trust loss

## Future Enhancements

### Versioning
- Add explicit version negotiation between VCP and AILEE
- Support multiple AILEE versions simultaneously
- Semantic versioning for interface stability

### Advanced Context
- Pass telemetry data for model selection optimization
- Include node health scores in adapter filtering
- Support custom adapter registries

### Enhanced Determinism
- Seed-based randomness for reproducibility
- Snapshot/restore for exact replay
- Proof generation for execution verification

## References

- AILEE Trust Layer crate: `crates/ailee-trust-layer/`
- VCP Integration adapter: `crates/ambient-node/src/ailee_integration.rs`
- Integration tests: `crates/ambient-node/tests/ailee_integration_test.rs`
