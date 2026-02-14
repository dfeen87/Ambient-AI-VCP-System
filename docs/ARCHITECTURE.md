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

**Current Implementation**: Placeholder (hash-based)

**Planned Integration**: RISC Zero zkVM or Plonky2

### 4. Mesh Coordinator

**Purpose**: Network-wide orchestration and task assignment

**Task Assignment Strategies**:
1. **Weighted**: Select node with highest health score
2. **Round-Robin**: Rotate through eligible nodes
3. **Least-Loaded**: Select node with lowest CPU usage
4. **Latency-Aware**: Select node with lowest latency

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
