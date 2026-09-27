# Ambient AI + VCP System

[![CI](https://github.com/dfeen87/Ambient-AI-VCP-System/actions/workflows/ci.yml/badge.svg)](https://github.com/dfeen87/Ambient-AI-VCP-System/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust 2021](https://img.shields.io/badge/Rust-2021-orange.svg?logo=rust)](Cargo.toml)
[![Version](https://img.shields.io/badge/version-3.2.0-6f42c1.svg)](Cargo.toml)
[![Public Demo](https://img.shields.io/badge/demo-live-2ea44f.svg)](https://ambient-ai-vcp-system.onrender.com)

An open-source implementation of a **Verifiable Computation Protocol (VCP)** for orchestrating and validating distributed workloads across heterogeneous machines.

## 📑 Table of Contents

- [Project Status](#project-status)
- [Overview](#overview)
- [How It Works](#how-it-works)
- [Live Demo](#live-demo)
- [Core Concepts](#core-concepts)
- [Key Features](#key-features)
- [Architecture](#architecture)
- [Technology Stack](#technology-stack)
- [Why Clone This Repository?](#why-clone-this-repository)
- [Quick Start](#quick-start)
- [Minimal Examples](#minimal-examples)
- [Testing](#testing)
- [Security & Validation](#security--validation)
- [Threat Model](#threat-model)
- [Health Scoring Formula](#health-scoring-formula)
- [Deployment Guide](#deployment-guide)
- [Deployment Options](#deployment-options)
- [Performance Targets](#performance-targets)
- [Executable Specification](#executable-specification-living-contract)
- [Roadmap](#roadmap)
- [Project Structure](#project-structure)
- [Contributing](#contributing)
- [License](#license)
- [Acknowledgments](#acknowledgments)
- [Enterprise Consulting & Integration](#enterprise-consulting--integration)
- [Support & Contact](#support--contact)

---

## 🎯 Project Status

The project includes a hosted public demo, automated CI checks, load tests, and a Groth16-based zero-knowledge proof implementation.

> [!NOTE]
> The hosted deployment is intended for evaluation and demonstration. Self-hosted production deployments should be configured and hardened for their operating environment; see the [deployment guide](./docs/DEPLOYMENT.md) and [security report](./docs/SECURITY_REPORT.md).

---

## 🧩 Overview

Ambient AI + VCP is a platform for **distributed, verifiable AI computing**. It connects available compute resources—including laptops, servers, and edge devices—into a coordinated mesh where workloads can be submitted, scheduled, and cryptographically verified. Its control plane provides node registration, health scoring, task routing, and result validation.

The system combines a **Verifiable Computation Protocol** with a practical mesh runtime. Tasks can be backed by Groth16/BN254 zero-knowledge proofs, while node trust scores are derived from operational telemetry. The AILEE trust layer adds multi-model consensus, an energy-weighted efficiency metric (∆v), and offline-first capabilities such as local session authentication, cached egress policies, and direct peer-to-peer policy synchronization when the central API is unavailable.

---

## 🧾 How It Works

The platform connects compute providers with users who need distributed processing:

- **Node operators** register available laptops, servers, or edge devices and advertise their capabilities.
- **Task submitters** define private workloads and their execution requirements.
- **The control plane** selects suitable nodes, coordinates execution, and tracks results.
- **The verification layer** can validate completed work with cryptographic proofs.

The included dashboard provides a unified view for submitting work, monitoring parallel execution, and observing cluster health in real time.

Because the runtime is model-agnostic, workloads can target GPU nodes, CPU workers, proof generators, or multi-model workflows. This allows heterogeneous components to be coordinated through one consistent interface.

## 🚀 Live Demo

[https://ambient-ai-vcp-system.onrender.com](https://ambient-ai-vcp-system.onrender.com)

| Endpoint | URL |
|----------|-----|
| Dashboard | https://ambient-ai-vcp-system.onrender.com |
| Swagger UI | https://ambient-ai-vcp-system.onrender.com/swagger-ui |
| OpenAPI JSON | https://ambient-ai-vcp-system.onrender.com/api-docs/openapi.json |

Tip: To quickly verify the public demo is reachable, run:
`curl https://ambient-ai-vcp-system.onrender.com/api/v1/health`
 
---

## 🎯 Core Concepts

**New to the system?** Here's what you need to know:

**The system serves two primary groups:**
- **Node operators** provide computing capacity by registering their devices.
- **Task submitters** are developers, researchers, or organizations that require compute resources.
- **The platform** matches tasks to suitable nodes, orchestrates execution, and returns results.

**Nodes** = Devices that join the network to contribute computing power (your laptop, server, etc.)
  - **5 Node Types**: Compute (run tasks), Gateway (route traffic), Storage (store data), Validator (verify proofs), Resonator (FEEN physics)
  - 👉 [Learn more about node types →](./docs/NODES_AND_TASKS_GUIDE.md#node-types-explained)

**Tasks** = Work submitted to the network for execution (train a model, run a computation, etc.)
  - **6 Task Types**: Federated Learning, ZK Proof, WASM Execution, General Computation, Connect-Only, FEEN Connectivity
  - **Who creates tasks?** App developers, data scientists, researchers, businesses - anyone who needs computation
  - 👉 [Learn more about task types →](./docs/NODES_AND_TASKS_GUIDE.md#task-types-explained)
  - 👉 [Who creates tasks and why? →](./docs/WHO_CREATES_TASKS.md)

**The Dashboard** (https://ambient-ai-vcp-system.onrender.com) lets you:
  - ✅ Register your device as a node
  - ✅ View all registered nodes and their health
  - ✅ Monitor submitted tasks and their status
  - ✅ See real-time cluster statistics
  - ✅ View observability data for your own nodes (owner-only)

📖 **For complete guides:**
- [Understanding Nodes & Tasks](./docs/NODES_AND_TASKS_GUIDE.md) - What are nodes and tasks?
- [Who Creates Tasks?](./docs/WHO_CREATES_TASKS.md) - The demand side explained

---

## 🌟 Key Features

### Core Capabilities
- 🌐 **Ambient Node Mesh**: Self-organizing network of heterogeneous edge devices
- 🧠 **Intelligent Orchestration**: Health-based task assignment with reputation scoring
- 🤖 **AILEE Trust Layer**: External generative intelligence with multi-model consensus and trust scoring
- 📐 **AILEE ∆v Metric**: Energy-weighted optimization gain functional for continuous efficiency monitoring (see [AILEE paper](https://github.com/dfeen87/AILEE-Trust-Layer))
- 🔌 **Offline-First / API-Disconnected Operation**: Nodes remain fully operational without a central API endpoint — local session management, policy caching, and internet egress continue via the [`LocalSessionManager`](crates/ambient-node/src/offline.rs)
- 🔗 **Peer-to-Peer Policy Sync**: Nodes in `OfflineControlPlane` or `NoUpstream` state can exchange cryptographically-verified policy snapshots with peer nodes, letting the mesh distribute fresh session policies without ever touching the control plane
- 🔒 **WASM Execution Engine**: Secure sandboxed computation with strict resource limits
- 🔐 **Zero-Knowledge Proofs**: Cryptographic verification with Groth16 implementation
- 🤝 **Federated Learning**: Privacy-preserving multi-node model training with FedAvg and differential privacy
- ✓ **Verifiable Computation**: Proof-of-Execution for trustless distributed computing
- ⚡ **Energy Telemetry**: Verifiable sustainability metrics

### Production Enhancements (NEW)
- ✅ **Comprehensive Input Validation**: All API endpoints validate input data
- ✅ **Zero Compiler Warnings**: Clean, maintainable codebase
- ✅ **Integration Tests**: 13 new integration tests for API validation
- ✅ **Error Handling**: Proper error propagation and user-friendly messages
- ✅ **Type Safety**: Full Rust type system guarantees
- 🔍 **Local Node Observability**: Privacy-preserving, operator-only inspection interface (localhost-only, read-only, no sensitive data exposure)

### Post-v2.3.0 Improvements
- 🛣️ **Internet Path Routing**: `PeerRouter` resolves direct or one-hop relay paths through `Universal`/`Open` nodes; `MeshCoordinator` exposes `sync_connectivity()` and `find_peer_route()` for runtime reachability updates
- 🔌 **Gateway Session Lifecycle**: `DataPlaneGateway::add_session()` / `revoke_session()` — sessions provisioned and revoked at runtime so nodes stop relaying traffic the instant a connect session ends
- ⚡ **Non-Blocking Password Hashing**: `hash_password_async()` offloads bcrypt to a blocking thread pool; configurable cost via `BCRYPT_COST` env var (default 12, range 4–31)
- 🔔 **Heartbeat-Triggered Task Sync**: `update_node_heartbeat` now calls `assign_pending_tasks_for_node` on every ping, so live nodes receive eligible pending tasks continuously — not only at registration
- 🎨 **Offline-First Dashboard Fonts**: Syne and JetBrains Mono fonts are self-hosted from bundled woff2 files; no Google Fonts CDN dependency, dashboard renders fully in air-gapped environments
- 🛡️ **Safe-Default Backhaul Routing**: `monitor_only = true` is the new default — the backhaul manager observes interfaces and scores them without touching kernel routing tables until explicitly opted in; `ip rule` entries are scoped to `from <src-ip>` to avoid affecting unrelated host traffic; health probes bind to the interface's own address for accurate per-interface metrics
- 🌐 **NCSI Spoof Server**: `NcsiSpoofServer` prevents false `ERR_INTERNET_DISCONNECTED` errors when the node acts as internet gateway for connected clients. When a client's direct internet is gone and the VCP node is the upstream provider, the client OS connectivity checks (Windows NCSI `GET /connecttest.txt`, Linux NetworkManager `GET /check_network_status.txt`, and generic captive-portal probes) are answered locally by a lightweight HTTP listener configured with `NcsiSpoofConfig`, stopping the OS from blocking traffic with a false disconnection signal.
- 🌍 **HTTP CONNECT Proxy**: `HttpConnectProxy` lets a browser on an offline node route all HTTPS traffic through a connected relay node, permanently bypassing `ERR_INTERNET_DISCONNECTED`. Point the browser's proxy settings at `<relay-ip>:3128`; it issues `CONNECT host:443 HTTP/1.1` with a `Proxy-Authorization: Bearer <token>` header, the proxy validates the token and opens a bidirectional TCP tunnel to the real destination. Non-CONNECT requests are rejected (405), bad or missing tokens return 407, and upstream failures surface as 502/504. Configured via `HttpConnectProxyConfig` (listen address, bearer token, connect/idle timeouts, enabled flag).
- 📶 **Relay Session QoS**: `RelayQosManager` installs WAN-side `tc` HTB + FQ-CoDel rules on the active backhaul interface when a `connect_only` session is active on an `open_internet` or `any` node — guaranteeing minimum bandwidth and low latency for relayed traffic while preventing node-internal traffic from crowding out the relay stream. Call `BackhaulManager::activate_relay_qos()` when a session starts and `deactivate_relay_qos()` when it ends.
- 💓 **Hardware Keepalive**: `BackhaulManager` now emits periodic hardware-level keepalive probes via `hardware_keepalive_tick(now_secs)`; the interval and enabled flag are controlled by `HardwareKeepaliveConfig` inside `BackhaulConfig` — keeping `connect_only` relay links alive through NAT devices and stateful firewalls that would otherwise expire idle sessions.
- 🏓 **Node Heartbeat Tracking** (`NodeRegistry`): `record_heartbeat(id, now_secs)` stores the last-seen timestamp for each registered node; `is_node_alive(id, now_secs, timeout_secs)` returns `true` while the node is within its liveness window — enabling the mesh coordinator to detect stale nodes without a round-trip to the API server.
- 🌐 **`internet_required()` on `LocalSessionManager`**: reports whether any active local session requires outbound internet connectivity, allowing `BackhaulManager` to prioritise interface selection for relay tasks.

### Node-to-Task Connectivity (v2.4.0)
- 🏥 **Heartbeat Modal Response**: `PUT /nodes/{id}/heartbeat` now returns `health_score`, `node_status`, `active_tasks`, `assigned_task_ids`, and a rich `assigned_tasks` array — each entry carries `task_id`, `task_type`, and `execution_status` so node processes can react immediately (e.g. activate gateway mode for `connect_only` tasks).
- 📋 **Execution Status Lifecycle**: `task_assignments` now tracks `execution_status` (`assigned` → `in_progress` → `completed`/`failed`), `execution_started_at`, and `execution_completed_at` across all paths: node result submission, synthetic fallback, connect_only session end, and forced disconnection (delete / reject / offline sweep).
- 🔄 **Activity Registration**: The first heartbeat a node sends after being assigned to a task advances `execution_status` from `assigned` → `in_progress`, recording the exact moment the node confirmed it is actively working.
- 📤 **Task Result Submission** (`POST /api/v1/tasks/{id}/result`): Nodes can now submit real execution outputs to the API instead of relying solely on synthetic fallback. When `require_proof = true`, a ZK proof must accompany the result and is verified before the task is marked completed.
- ⏱️ **Honest Fallback Timeout**: For non-`connect_only` tasks the synthetic fallback now waits the full `max_execution_time_sec` before firing — giving nodes time to submit real results. Previously it fired immediately, preempting any real output.
- 🛡️ **Completed Task Protection**: `update_task_status_from_assignments` now carries an `AND status NOT IN ('completed','failed')` guard, preventing a node going offline from silently reverting an already-completed task to `pending`.
- 🌐 **Gateway Session Polling** (`GET /api/v1/nodes/{id}/gateway-sessions`): `open_internet` / relay nodes can poll this endpoint each heartbeat cycle to receive the current set of active `connect_only` sessions they should relay, including the cleartext `session_token` the `DataPlaneGateway` needs to validate incoming relay connections. The session token is stored server-side on session creation and returned only to the authenticated node owner.
- 💤 **Node Offline Sweep**: A background task runs every `NODE_OFFLINE_SWEEP_INTERVAL_SECONDS` (default 60 s). Any node whose `last_heartbeat` is older than `NODE_HEARTBEAT_TIMEOUT_MINUTES` (default 5 min) is marked `offline`, its active assignments are disconnected (in-progress ones marked `failed`), and affected tasks are immediately reassigned to other eligible nodes.
- 🏆 **Health-Score Node Selection**: Task assignment now orders candidates by `health_score DESC` (then `registered_at ASC` as tiebreaker) so healthiest nodes are always preferred; the redundant `registered_at` and `health_score` columns were also removed from the `GROUP BY` clause since they are functionally dependent on the `node_id` primary key.

### Security & Infrastructure
- 🔐 **JWT Middleware Authentication**: Global JWT enforcement at middleware layer (not handler extractors)
- 🛡️ **Rate Limiting**: Per-endpoint tier-based rate limiting (Auth: 10rpm, Nodes: 20rpm, Tasks: 30rpm, Proofs: 15rpm)
- 🔄 **Refresh Tokens**: JWT token rotation with 30-day refresh tokens and automatic revocation
- 🔒 **CORS Hardening**: Configurable origin-based CORS (no wildcards in production)
- 📊 **Prometheus Metrics**: `/metrics` endpoint with per-route latency and error tracking
- 📝 **Audit Logging**: Comprehensive audit trail for security events
- 🔍 **ZK Proof Verification**: Cryptographic verification (Groth16/BN254) with strict payload validation
- 🔑 **P2P Message Integrity**: Ed25519 signature verification for offline peer policy sync messages; signer public key validated against the local trusted key set
- 🛡️ **Middleware Hardening**: Explicit state injection for reliable authentication flow
- 🌐 **Security Headers**: HSTS, X-Content-Type-Options, X-Frame-Options, Referrer-Policy
- 📊 **Request Tracing**: Structured logging with request IDs for all API calls
- 💾 **Enhanced Persistence**: Migrations for task_runs, proof_artifacts, api_keys, audit_log, node_heartbeat_history

### Security Policy: Node Registry + Task Intake
- **Capability whitelist (registration)**:
  - `bandwidth_mbps`: `10..=100_000`
  - `cpu_cores`: `1..=256`
  - `memory_gb`: `1..=2_048`
- **Task-type registry (submission)**:
  - Canonical task types: `federated_learning`, `zk_proof`, `wasm_execution`, `computation`
  - Per-type policies: max execution time, max payload size, and WASM allow/deny
- **Node registry enforcement (admission control)**:
  - Task creation checks for enough eligible online nodes that meet the task policy before insert.

📖 See [`docs/NODE_SECURITY.md`](./docs/NODE_SECURITY.md) for the full security model, threat boundaries, and operator guidance.

---

## 🏗️ Architecture

![Ambient AI + VCP architecture diagram placeholder](docs/images/architecture-diagram-placeholder.png)

> **Diagram placeholder:** A versioned visual of the control plane, peer mesh, execution runtimes, and proof-verification path will be published here. The component map below remains the normative architecture summary.

### System Components

```
┌─────────────────────────────────────────────────────────────┐
│                     REST API Server                         │
│            (Axum + OpenAPI/Swagger UI)                     │
└──────────────┬──────────────────────────────────┬───────────┘
               │                                  │
       ┌───────▼────────┐                ┌──────▼───────┐
       │ Mesh Coordinator│                │ Node Registry│
       │  (Orchestration)│                │  (Health Mgmt)│
       └───────┬─────────┘                └──────┬───────┘
               │                                  │
    ┌──────────▼──────────────────────────────────▼─────────┐
    │           Ambient Node Network (P2P Mesh)             │
    └──┬────────┬────────┬────────┬────────┬────────┬───────┘
       │        │        │        │        │        │
    ┌──▼──┐  ┌─▼──┐  ┌─▼──┐  ┌─▼──┐  ┌─▼──┐  ┌─▼──┐
    │Node │  │Node│  │Node│  │Node│  │Node│  │Node│
    │(GPU)│  │(CPU)│  │(Edge)  │(IoT)│  │(Cloud) │(Mobile)
    └─────┘  └────┘  └────┘  └────┘  └────┘  └────┘
       │        │        │        │        │        │
    ┌──▼────────▼────────▼────────▼────────▼────────▼───────┐
    │     WASM Execution Engine + ZK Proof System           │
    │   (Sandboxed, Resource-Limited, Traceable)           │
    └───────────────────────────────────────────────────────┘
```

### 1. **Ambient Node** (`ambient-node`)
**Purpose**: Individual compute nodes in the distributed network

- ⚡ Real-time telemetry collection (energy, compute, privacy budgets)
- 📊 Multi-factor health scoring (bandwidth 40%, latency 30%, compute 20%, reputation 10%)
- 🛡️ Safety circuit breakers (temperature > 85°C, latency > 100ms, error count > 25)
- 🏆 Reputation tracking with success rate calculation
- 🔄 Dynamic health score updates

### 2. **WASM Execution Engine** (`wasm-engine`)
**Purpose**: Secure, sandboxed code execution

- 🔒 WasmEdge runtime integration for secure execution
- 📏 Resource limits: Memory (512MB), Timeout (30s), Gas metering
- 📝 Execution trace recording for ZK proof generation
- 🔁 Determinism verification for reproducibility
- ⚠️ Comprehensive error handling and validation

### 3. **ZK Proof System** (`zk-prover`)
**Purpose**: Cryptographic verification of computations

- 🔐 Production Groth16 implementation on BN254 curve
- ✓ Universal verifier for WASM program execution
- 🎯 Real cryptographic proofs with sub-second verification
- 📦 Compact proof size (~128-256 bytes)
- 🚀 Fast proof generation (<10s) and verification (<1s)

### 4. **Mesh Coordinator** (`mesh-coordinator`)
**Purpose**: Task orchestration and node management

- 📋 Centralized node registry with real-time health tracking
- 🎯 Multiple task assignment strategies:
  - **Weighted**: Health score-based selection
  - **Round-robin**: Fair distribution
  - **Least-loaded**: Load balancing
  - **Latency-aware**: Geographic optimization
- 🛣️ **`PeerRouter`**: Classifies each node's internet reachability (`Online`/`Offline`/`Unknown`) and resolves forwarding paths — direct for online nodes, one-hop relay via `Universal` or `Open` nodes otherwise
- ✅ Proof verification pipeline
- 💰 Reward distribution (future)

### 5. **Federated Learning** (`federated-learning`)
**Purpose**: Privacy-preserving distributed ML

- 📊 **FedAvg Algorithm**: Weighted model aggregation
- 🔒 **Differential Privacy**: Configurable ε (epsilon) and δ (delta)
- ✂️ **Gradient Clipping**: Bounded sensitivity for DP
- 🧮 **Noise Injection**: Gaussian and Laplacian mechanisms
- 🔄 **Multi-round Training**: Iterative model improvement

### 6. **REST API Server** (`api-server`) ⭐ **ENHANCED**
**Purpose**: Public-facing HTTP API with comprehensive validation and security

**Security Features:**
- ✅ **Node Ownership**: Nodes linked to user accounts with ownership verification
- ✅ **JWT Authentication**: Protected endpoints require authentication
- ✅ **Authorization**: Users can only manage their own nodes
- ✅ **Heartbeat Mechanism**: Track node availability and detect offline nodes
- ✅ **Soft Delete**: Maintain audit trail when nodes are deregistered
- ✅ **Capability Whitelist Enforcement**: Node capability claims are validated at registration (`bandwidth_mbps`, `cpu_cores`, `memory_gb`)
- ✅ **Task-Type Registry Enforcement**: Task intake checks canonical task types, runtime limits, WASM policy, and minimum capability requirements
- ✅ **Node Eligibility Gate**: Task submission is rejected when the online registry cannot satisfy `min_nodes` for the task policy
- ℹ️ **Current Visibility Model**: Node/task list endpoints are authenticated (JWT required) and visible to authenticated users; node ownership controls mutation (delete/heartbeat)

**Endpoints:**
- `GET /api/v1/health` - Health check ✅
- `POST /api/v1/auth/register` - Register user account ✅
- `POST /api/v1/auth/login` - Login and get JWT token ✅
- `POST /api/v1/nodes` - Register node (requires auth) ✅
- `GET /api/v1/nodes` - List all nodes ✅
- `GET /api/v1/nodes/{id}` - Get specific node ✅
- `DELETE /api/v1/nodes/{id}` - Delete node (requires ownership) ✅
- `PUT /api/v1/nodes/{id}/heartbeat` - Update heartbeat; returns `health_score`, `node_status`, `assigned_tasks` with `task_type`+`execution_status` ✅
- `GET /api/v1/nodes/{id}/heartbeat/activity` - Task connect/disconnect events for a node ✅
- `GET /api/v1/nodes/{id}/gateway-sessions` - Active relay sessions for gateway nodes (cleartext token included) ✅ **NEW**
- `POST /api/v1/tasks` - Submit task (requires auth) ✅
- `GET /api/v1/tasks` - List all tasks ✅
- `GET /api/v1/tasks/{id}` - Get specific task ✅
- `POST /api/v1/tasks/{id}/result` - Submit node execution result with optional ZK proof ✅ **NEW**
- `POST /api/v1/proofs/verify` - Verify ZK proof (requires auth) ✅
- `GET /api/v1/cluster/stats` - Cluster statistics ✅

**Validation Rules:**
- Node IDs: 1-64 chars, alphanumeric + hyphens/underscores
- Node types: `compute`, `gateway`, `storage`, `validator`, `open_internet`, `any`
- Bandwidth: 10-100,000 Mbps
- CPU cores: 1-256
- Memory: 1-2,048 GB
- Task types: `federated_learning`, `zk_proof`, `wasm_execution`, `computation`, `connect_only`
- Min nodes: 1-1000
- Execution time: 1-3600 seconds

### 7. **CLI Tool** (`cli`)
**Purpose**: Command-line interface for system management

```bash
# Start a compute node
ambient-vcp node --id node-001 --region us-west --node-type compute

# Start connect_only data-plane gateway on an open_internet node
ambient-vcp gateway --listen 0.0.0.0:7000 --sessions-file ./gateway-sessions.json

# Start a coordinator
ambient-vcp coordinator --cluster-id cluster-001 --strategy weighted

# Check node health
ambient-vcp health
```

Gateway sessions file format (`gateway-sessions.json`):

```json
[
  {
    "session_id": "sess_123",
    "session_token": "cs_your_ephemeral_token",
    "egress_profile": "allowlist_domains",
    "destination_policy_id": "policy_web_basic_v1",
    "allowed_destinations": ["*.example.com", "1.1.1.1"],
    "expires_at_epoch_seconds": 1735689600
  }
]
```

### 8. **Local Node Observability** (`ambient-node/observability`) 🆕
**Purpose**: Privacy-preserving, operator-only node inspection

**🔒 Privacy & Security Design:**
- ✅ **Local-only access**: Binds strictly to `127.0.0.1` (no external network access)
- ✅ **Operator-only**: Only the node owner can access this interface
- ✅ **Read-only**: No mutation or control of execution state
- ✅ **Privacy-preserving**: Does NOT expose private payloads, secrets, or sensitive data
- ✅ **No telemetry**: Does NOT send data to centralized systems or enable cross-node visibility
- ✅ **Optional**: Disabled by default, must be explicitly enabled

**Usage:**

```bash
# Start a node with local observability enabled
ambient-vcp node --id node-001 --region us-west --node-type compute \
  --observability --observability-port 9090

# The node will print a curl command on startup:
# curl http://127.0.0.1:9090/node/status | jq

# Inspect your node (example output):
curl http://127.0.0.1:9090/node/status | jq
```

**Example Response:**

```json
{
  "node_region": "us-west",
  "node_type": "compute",
  "uptime_seconds": 3600,
  "current_workload": "generation",
  "resources": {
    "cpu_percent": 45.2,
    "memory_percent": 62.8,
    "temperature_c": 68.0
  },
  "trust_summary": {
    "trust_threshold": 0.7,
    "last_trust_score": 0.85,
    "lineage_hash": "abc123...",
    "models_used": 2
  },
  "health_score": 0.78,
  "safe_mode": false,
  "timestamp": 1771453109
}
```

**Architecture:**
- Strict separation: observability MAY read execution state, but execution MUST NEVER depend on observability
- No blocking operations that could affect node performance
- Exposes only high-level, non-sensitive metrics (uptime, resource usage, trust scores)
- Trust decision metadata (scores, thresholds, hashes) - no payloads or model inputs

### 9. **AILEE ∆v Metric** (`ailee-trust-layer/metric`) 🆕
**Purpose**: Time-integrated efficiency monitoring based on the [AILEE paper](https://github.com/dfeen87/AILEE-Trust-Layer)

The AILEE framework introduces an *energy-weighted optimization gain functional* ∆v that accumulates performance gain over time while penalising inertia and off-resonant operation:

```
∆v = Isp · η · e^(−α·v₀²) · ∫ P_input(t) · e^(−α·w(t)²) · e^(2α·v₀·v(t)) / M(t) dt
```

- 📐 **`AileeMetric`**: Accumulates successive telemetry samples via `integrate()` and exposes `delta_v()` at any point in time
- 📋 **`AileeSample`**: Per-interval telemetry snapshot — compute/power input `P_input`, workload `w`, adaptation velocity `v`, and model inertia `M`
- 🎛️ **`AileeParams`**: Configurable resonance sensitivity `α`, efficiency coefficient `η`, specific factor `Isp`, and reference state `v₀`
- 🔒 Overflow-safe: both exponential resonance gates are clamped to prevent `f64` overflow for large telemetry values

**Usage:**
```rust
use ailee_trust_layer::metric::{AileeMetric, AileeSample};

let mut metric = AileeMetric::default();
metric.integrate(&AileeSample::new(100.0, 0.5, 1.2, 10.0, 1.0)); // P, w, v, M, dt
let gain = metric.delta_v(); // dimensionless efficiency gain
```

### 10. **Peer-to-Peer Policy Sync** (`ambient-node/offline`) 🆕
**Purpose**: Keep nodes operational and internet-capable even when disconnected from the API endpoint

> **Answer to "Can we connect nodes and power internet while disconnected from the API?"**  
> **Yes.** The `LocalSessionManager` runs in `OfflineControlPlane` mode when the WAN is up but the API is unreachable. Nodes can now *share verified policy snapshots directly with each other* — no central server needed.

- 🔗 **`PeerPolicySyncMessage`**: A serialisable, SHA3-256-integrity-protected snapshot of a node's egress policies and verification keys — covers full policy content (IDs *and* destinations) so tampering with allowed destinations also invalidates the hash
- 📤 **`LocalSessionManager::export_peer_sync()`**: Snapshot the current policy cache for distribution to peers
- 📥 **`LocalSessionManager::import_peer_sync()`**: Non-destructively merge policies from a peer — existing local entries are *never* overwritten, preventing a compromised peer from downgrading local policies
- 📋 Every import is appended to the local audit queue with event type `peer_sync_applied`
- ✅ Works in `OfflineControlPlane`, `NoUpstream`, and `OnlineControlPlane` states

**Node states:**

| State | API reachable | WAN up | Internet egress | Peer sync |
|-------|:---:|:---:|:---:|:---:|
| `OnlineControlPlane` | ✅ | ✅ | ✅ | ✅ |
| `OfflineControlPlane` | ❌ | ✅ | ✅ (cached policies) | ✅ |
| `NoUpstream` | ❌ | ❌ | ❌ | ✅ (receive only) |

**Usage:**
```rust
// Node A (has fresh policies) → exports a snapshot
let msg = node_a_mgr.export_peer_sync("node-A");

// Node B (API offline, stale cache) → imports non-destructively
let added = node_b_mgr.import_peer_sync(&msg)?;
// node-B can now activate sessions and route traffic using the synced policies
```

### 11. **Web Dashboard** (`api-server/assets`)
**Purpose**: Real-time monitoring interface

- 📊 Real-time cluster metrics visualization
- 🖥️ Interactive node registration
- 📈 Health score monitoring
- 🔄 Auto-refresh every 5 seconds
- 🎨 Modern gradient UI design
- 👁️ **Owner-only node observability** (v2.1.0): "View" button for local node status inspection

---

### 12. **FEEN Physics Engine Integration** (`ambient-node/feen`) 🆕
**Purpose**: Local wave-native physics simulation powering the `feen_resonator` node type and `feen_connectivity` task type

> **FEEN repository**: [https://github.com/dfeen87/FEEN](https://github.com/dfeen87/FEEN)

FEEN is a Duffing-resonator physics engine — VCP acts as the orchestrator while FEEN remains a self-contained, local physics backend.  The integration exposes three minimal REST endpoints (`/api/v1/simulate`, `/api/v1/coupling`, `/api/v1/delta_v`) and keeps FEEN internals fully hidden behind a clean Rust trait boundary.

**New VCP primitives introduced:**

| Primitive | Kind | Description |
|-----------|------|-------------|
| `feen_resonator` | Node type | Wraps a FEEN resonator's physical state `(x, v)`, coupling config, and accumulated ∆v |
| `feen_connectivity` | Task type | Uses FEEN to compute resonance, interference, stability, and ∆v across a set of nodes |

**Core Rust types** (`crates/ambient-node/src/feen.rs`):

- 🔧 **`ResonatorConfig`** — resonator parameters: `frequency_hz`, `q_factor` (damping), `beta` (nonlinearity)
- 📍 **`ResonatorState`** — physical state snapshot: displacement `x`, velocity `v`, `energy`, `phase`
- 🔗 **`CouplingConfig`** — directed coupling between two resonators: `source_id`, `target_id`, `strength`, `phase_shift`
- ⚡ **`Excitation`** — drive signal: `amplitude`, `frequency_hz`, `phase`
- 🧩 **`FeenEngine` trait** — async interface (`simulate_resonator`, `update_coupling`) that allows both the live HTTP client and test mocks to be used interchangeably
- 🌐 **`FeenClient`** — HTTP client that posts to the FEEN REST API (`/api/v1/simulate`, `/api/v1/coupling`)
- 🏗️ **`FeenNode`** — stateful VCP node that wraps a `FeenClient`, owns the current `ResonatorState`, and accumulates the AILEE ∆v metric across ticks

**FEEN-side REST API** (`feen-changes/`):

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/simulate` | POST | Stateless single-step simulation of a resonator |
| `/api/v1/coupling` | POST | Apply a coupling update between two resonators |
| `/api/v1/delta_v` | POST | Compute the AILEE ∆v metric for a sequence of telemetry samples |

**Usage:**
```rust
use ambient_node::feen::{FeenClient, FeenNode, ResonatorConfig, Excitation};

// Connect to a locally-running FEEN instance
let client = FeenClient::new("http://localhost:8080".to_string());
let config = ResonatorConfig { frequency_hz: 440.0, q_factor: 10.0, beta: 0.0 };
let mut node = FeenNode::new(client, config);

// Drive the resonator one time step (dt = 1 ms)
let excitation = Excitation { amplitude: 1.0, frequency_hz: 440.0, phase: 0.0 };
node.tick(&excitation, 0.001).await?;

// Read the accumulated efficiency gain
println!("∆v = {}", node.delta_v());
```

**Architectural invariants:**
- 🔒 Each `/api/v1/simulate` call is **stateless** — state is owned by VCP, never by FEEN
- 🔒 No persistent identity, session, or user semantics are introduced in FEEN
- 🔒 `FeenNode` uses the **`FeenEngine` trait**, keeping the HTTP transport swappable for tests
- 🔒 ∆v is computed **locally** by VCP's `AileeMetric`, not delegated to FEEN

---

## 📚 Technology Stack

### Why Rust for v1.0?

✅ **Performance**: Near-native execution speed  
✅ **Memory Safety**: Zero-cost abstractions with compile-time guarantees  
✅ **WASM Support**: First-class support via WasmEdge  
✅ **Concurrency**: Tokio async runtime for high-throughput systems  
✅ **Production-Ready**: Strong type system prevents bugs  

### Dependencies

- **Runtime**: Tokio (async/await)
- **Web Framework**: Axum 0.7
- **Serialization**: Serde + JSON
- **Cryptography**: SHA3, Ring
- **WASM**: WasmEdge SDK
- **API Docs**: OpenAPI/Swagger (utoipa)
- **Testing**: Tokio Test + Integration Tests

---

## 🎁 Why Clone This Repository?

**Get a production-ready distributed AI platform in 5 minutes!**

When you clone this repo, you immediately get:
- ✅ **REST API Server** with OpenAPI/Swagger docs
- ✅ **Federated Learning** with differential privacy
- ✅ **Zero-Knowledge Proofs** (Groth16, sub-second verification)
- ✅ **WASM Execution Engine** with sandboxing
- ✅ **Web Dashboard** for real-time monitoring
- ✅ **AILEE ∆v Metric** for continuous efficiency monitoring (new)
- ✅ **Offline-First + Peer Policy Sync** — nodes keep working and routing internet traffic even without the API endpoint (new)
- ✅ **HTTP CONNECT Proxy** — browsers on offline nodes tunnel HTTPS through a connected relay node, bypassing `ERR_INTERNET_DISCONNECTED` (new)
- ✅ **274 Passing Tests** + Zero compiler warnings
- ✅ **Complete Documentation** (15+ guides)
- ✅ **MIT License** - Fully open-source for personal, research, non-profit, and commercial use

👉 **[See Full Benefits Guide](./docs/USER_BENEFITS.md)** - Learn who benefits and how to use it

---

## 🚀 Quick Start

### Prerequisites

- **Rust**: 1.75 or later
- **WasmEdge**: (Optional, for WASM execution features)
- **Tools**: curl, jq (for demo script)

### Installation

```bash
# Clone the repository
git clone https://github.com/dfeen87/Ambient-AI-VCP-System.git
cd Ambient-AI-VCP-System

# Build the project (zero warnings!)
cargo build --release

# Run all tests (274 tests)
cargo test
```

### CLI-to-Result Quickstart

The CLI runs nodes and coordinators locally; authenticated registration and task intake use the REST control plane. With PostgreSQL available and the API server running, the following sequence installs the CLI, registers a node, submits one minimal computation, and reads its state and result:

```bash
# Install the workspace CLI and start the API in another terminal.
cargo install --path crates/cli
JWT_SECRET="$(openssl rand -base64 32)" DATABASE_URL="$DATABASE_URL" \
  cargo run --bin api-server

export VCP_API=http://localhost:3000
curl -sS -X POST "$VCP_API/api/v1/auth/register" \
  -H 'Content-Type: application/json' \
  -d '{"username":"quickstart","password":"change-me-now"}' >/dev/null
export VCP_TOKEN="$(curl -sS -X POST "$VCP_API/api/v1/auth/login" \
  -H 'Content-Type: application/json' \
  -d '{"username":"quickstart","password":"change-me-now"}' | jq -r .access_token)"

# Register control-plane capacity, then run the corresponding local node process.
curl -sS -X POST "$VCP_API/api/v1/nodes" \
  -H "Authorization: Bearer $VCP_TOKEN" -H 'Content-Type: application/json' \
  -d '{"node_id":"quick-node","region":"local","node_type":"compute","capabilities":{"bandwidth_mbps":100,"cpu_cores":4,"memory_gb":8,"gpu_available":false}}' | jq
ambient-vcp node --id quick-node --region local --node-type compute

# In another terminal with VCP_API and VCP_TOKEN exported:
export TASK_ID="$(curl -sS -X POST "$VCP_API/api/v1/tasks" \
  -H "Authorization: Bearer $VCP_TOKEN" -H 'Content-Type: application/json' \
  -d '{"task_type":"computation","wasm_module":null,"inputs":{"operation":"sum","values":[1,2,3]},"requirements":{"min_nodes":1,"max_execution_time_sec":30,"require_gpu":false,"require_proof":false}}' | jq -r .task_id)"
curl -sS -H "Authorization: Bearer $VCP_TOKEN" \
  "$VCP_API/api/v1/tasks/$TASK_ID" | jq '{task_id,status,result}'
```

Task execution is asynchronous. Poll the final command until `status` is `completed` or `failed`; a real worker may submit a result earlier, while the configured fallback respects `max_execution_time_sec`.

### Running the API Server

```bash
# Start the REST API server
cargo run --bin api-server

# Server starts on http://localhost:3000
# Swagger UI: http://localhost:3000/swagger-ui
```

### Running the Demo

```bash
# Run the complete multi-node demo
./demo/run-demo.sh

# This will:
# 1. Start the API server (if not running)
# 2. Register 3 nodes across different regions
# 3. Submit federated learning task
# 4. Submit ZK proof task
# 5. Verify proofs
# 6. Display cluster statistics
```

### Accessing the Dashboard

The dashboard is served by the API server itself:

```bash
# Start API server first
cargo run --bin api-server

# Open dashboard
open http://localhost:3000/
```

---

## 🧩 Minimal Examples

These payloads intentionally show the smallest useful shapes. Replace `$VCP_TOKEN`, task IDs, and cryptographic material with values issued by your deployment.

### Tiny WASM Execution

Compile a deterministic exported function, encode the module, and submit it through the same authenticated task endpoint:

```rust
// src/lib.rs in a cdylib crate targeting wasm32-wasip1
#[no_mangle]
pub extern "C" fn answer() -> i32 { 42 }
```

```bash
WASM_B64="$(base64 < target/wasm32-wasip1/release/tiny.wasm | tr -d '\n')"
jq -n --arg module "$WASM_B64" '{
  task_type:"wasm_execution", wasm_module:$module,
  inputs:{function_name:"answer",args:[]},
  requirements:{min_nodes:1,max_execution_time_sec:30,require_gpu:false,require_proof:false}
}' | curl -sS -X POST "$VCP_API/api/v1/tasks" \
  -H "Authorization: Bearer $VCP_TOKEN" -H 'Content-Type: application/json' -d @- | jq
```

### One Federated Learning Round

`FederatedAggregator` applies sample-weighted FedAvg and increments the global model version after each successful round:

```rust
use federated_learning::{FederatedAggregator, LayerWeights, ModelWeights};

let model = |weights| ModelWeights {
    layers: vec![LayerWeights { name: "dense".into(), weights, shape: vec![2] }],
    version: 0,
};
let mut round = FederatedAggregator::new(model(vec![0.0, 0.0]));
round.add_client_update("node-a".into(), model(vec![1.0, 3.0]), 1)?;
round.add_client_update("node-b".into(), model(vec![3.0, 5.0]), 1)?;
let global = round.aggregate()?; // weights = [2.0, 4.0], version = 1
# Ok::<(), anyhow::Error>(())
```

### ZK Proof Verification Payload

The verifier accepts base64-encoded Groth16 proof bytes and serialized public inputs. Payload validation is not proof generation: both fields must come from the matching BN254 circuit and verification key.

```json
{
  "task_id": "550e8400-e29b-41d4-a716-446655440000",
  "proof_data": "<base64-groth16-proof>",
  "public_inputs": "<base64-serialized-public-inputs>",
  "circuit_id": "wasm-execution-v1"
}
```

```bash
curl -sS -X POST "$VCP_API/api/v1/proofs/verify" \
  -H "Authorization: Bearer $VCP_TOKEN" -H 'Content-Type: application/json' \
  --data @proof-request.json | jq
```

---

## 🧪 Testing

### Test Coverage

| Component | Unit Tests | Integration Tests | Load Tests | Total |
|-----------|-----------|-------------------|------------|-------|
| ambient-node | 91 | 17 | - | 108 |
| ailee-trust-layer | 38 | - | - | 38 |
| api-server | 36 | 24 | 2 | 62 |
| federated-learning | 8 | - | - | 8 |
| mesh-coordinator | 21 | - | - | 21 |
| wasm-engine | 6 | - | - | 6 |
| zk-prover | 8 | - | - | 8 |
| **TOTAL** | **208** | **41** | **2** | **254** |

### Running Tests

```bash
# Run all tests
cargo test

# Run specific crate tests
cargo test -p api-server
cargo test -p ambient-node

# Run with logging
RUST_LOG=info cargo test

# Run integration tests only
cargo test --test integration_test
```

### Test Examples

**Input Validation Tests:**
```rust
# Test invalid node_id (empty string) - FAILS ✅
# Test invalid node_type (not in allowed list) - FAILS ✅
# Test invalid bandwidth (negative value) - FAILS ✅
# Test valid node registration - PASSES ✅
```

---

## 🔒 Security & Validation

### Authentication & Authorization ⭐ **NEW**

**Node Ownership & Lifecycle:**
- ✅ **JWT Authentication**: All node operations require valid JWT tokens
- ✅ **User Registration**: Secure account creation with bcrypt password hashing
- ✅ **Node Ownership**: Nodes linked to user accounts via foreign key constraint
- ✅ **Authorization**: Users can only manage their own nodes
- ✅ **Soft Delete**: Nodes can be deregistered with audit trail (deleted_at timestamp)
- ✅ **Heartbeat Tracking**: Detect stale/offline nodes via last_heartbeat timestamp
- ℹ️ **Read Visibility Emphasis**: `GET /nodes` and `GET /tasks` are authenticated endpoints and currently return shared authenticated views; ownership checks apply to node management actions

**Security Best Practices:**
- ✅ Parameterized SQL queries prevent injection attacks
- ✅ Error messages sanitized to prevent information leakage
- ✅ 404 responses for both missing and unauthorized resources
- ✅ Foreign key constraints ensure referential integrity
- ✅ Production mode enforces strong JWT secrets (min 32 characters)

**Protected Endpoints:**
```
POST   /api/v1/nodes                           - Register node (requires JWT)
POST   /api/v1/nodes/{id}/reject               - Reject node (requires ownership)
DELETE /api/v1/nodes/{id}                      - Delete node (requires ownership)
PUT    /api/v1/nodes/{id}/heartbeat            - Update heartbeat (requires ownership)
GET    /api/v1/nodes/{id}/heartbeat/activity   - Task activity events (requires ownership)
GET    /api/v1/nodes/{id}/gateway-sessions     - Active relay sessions (requires ownership)
POST   /api/v1/tasks                           - Submit task (requires JWT)
POST   /api/v1/tasks/{id}/result               - Submit node result + optional ZK proof (requires node ownership)
DELETE /api/v1/tasks/{id}                      - Delete task (requires owner/admin)
POST   /api/v1/proofs/verify                   - Verify proof (requires JWT)
GET    /metrics                                - Prometheus metrics (admin JWT required)
GET    /api/v1/admin/users                     - Admin users endpoint (admin JWT required)
POST   /api/v1/admin/throttle-overrides        - Admin throttle override endpoint
GET    /api/v1/admin/audit-log                 - Admin audit endpoint (admin JWT required)
GET    /api/v1/auth/api-key/validate           - API-key validation endpoint (API key required)
```

**Public Endpoints:**
```
GET  /api/v1/health               - Health check
POST /api/v1/auth/register        - Register account
POST /api/v1/auth/login           - Login and get JWT
POST /api/v1/auth/refresh         - Rotate refresh token / issue new access token
```

**Authenticated JWT Endpoints (non-admin):**
```
GET  /api/v1/nodes                - List nodes
GET  /api/v1/nodes/{id}           - Get node details
GET  /api/v1/tasks                - List tasks
GET  /api/v1/tasks/{id}           - Get task details
GET  /api/v1/cluster/stats        - Cluster statistics
```

### Input Validation

All API endpoints validate input data before processing:

**Node Registration:**
- ✅ Node ID length and character validation
- ✅ Region name validation
- ✅ Node type whitelist enforcement
- ✅ Capability range validation
- ✅ User authentication required

**Task Submission:**
- ✅ Task type whitelist enforcement
- ✅ WASM module size limits (10MB)
- ✅ Min/max node count validation
- ✅ Execution time limits
- ✅ User authentication required

**User Registration:**
- ✅ Username: 3-32 characters, alphanumeric + underscores
- ✅ Password: Minimum 8 characters
- ✅ Unique username enforcement
- ✅ Password strength requirements

**Error Responses:**
```json
{
  "error": "bad_request",
  "message": "node_id cannot exceed 64 characters"
}
```

### Sandbox Security

WASM execution is restricted by:
- 🔒 Memory: 512MB default (configurable)
- ⏱️ Timeout: 30 seconds
- 🔢 Max instructions: 10 billion
- 🚫 No filesystem access
- 🚫 No network access
- ✅ Cryptographic operations allowed

### Circuit Breakers

Nodes enter safe mode when:
- 🌡️ Temperature > 85°C
- ⏱️ Latency > 100ms
- ⚠️ Error count > 25 consecutive failures

---

## 🛡️ Threat Model

### Trust Boundaries

- **Public clients → API control plane:** All client input is untrusted. JWTs establish an authenticated principal; ownership and admin checks establish authorization.
- **Control plane → node mesh:** Registration claims, heartbeats, results, and relay requests cross a machine boundary. A registered node is not assumed honest merely because it is online.
- **Task payload → execution runtime:** Submitted modules and inputs are attacker-controlled. WASM isolation, deterministic execution, resource limits, and task-type policy checks constrain their authority.
- **Peer → offline node:** Policy snapshots and session leases can arrive without the coordinator. Their signatures, integrity hashes, expiry, and trusted-key membership must be verified before use.
- **Proof producer → verifier:** A prover may be malicious. Verification establishes only the statement encoded by the selected circuit and public inputs; it does not make an incorrect circuit specification trustworthy.

### Attack Surfaces and Mitigations

| Surface | Representative risk | Required mitigation |
|---------|---------------------|---------------------|
| Authentication API | Credential stuffing, token theft, replay | bcrypt password hashing, short-lived JWT access tokens, refresh-token rotation/revocation, TLS, and per-tier rate limiting |
| Node and task intake | Capability inflation, oversized payloads, scheduler abuse | Capability whitelists, canonical task registry, input/depth limits, ownership checks, and admission control |
| Worker execution | Malicious WASM, resource exhaustion, nondeterministic output | No filesystem/network access, memory/time/gas ceilings, deterministic modules, circuit breakers, and isolated worker identities |
| Results and proofs | Forged output or proof substitution | Bind task ID, circuit ID, and public inputs; verify Groth16 proofs server-side; reject malformed encodings and unknown circuits |
| Offline policy sync | Forged leases, stale policy, malicious peer downgrade | Ed25519 signatures, trusted signer allowlists, expiry enforcement, full-content hashing, non-destructive imports, and chained audit records |
| Gateway/data plane | Open proxying, token disclosure, destination escape | Ephemeral session tokens, immediate revocation, destination capability allowlists, bounded sessions, authenticated ownership, and least-privilege egress |

**Security roles:** ZK proofs provide computation integrity and privacy for statements supported by a circuit; JWTs authenticate control-plane requests; rate limits reduce online abuse and resource exhaustion; capability whitelists constrain what nodes and tasks may claim. These controls are complementary, not interchangeable. Operators must still protect signing keys, JWT secrets, databases, hosts, and TLS termination, and must monitor audit logs for anomalous behavior.

---

## 📊 Health Scoring Formula

```
Score = (bandwidth × 0.4) + (latency × 0.3) + (compute × 0.2) + (reputation × 0.1)
```

**Components:**
- **Bandwidth** (40%): Max 1000 Mbps
- **Latency** (30%): Lower is better, max 100ms
- **Compute** (20%): CPU + Memory availability
- **Reputation** (10%): Task success rate

---

## 🚀 Deployment Guide

### Environment Variables

Start from `.env.example`, keep secrets in a platform secret store, and validate the effective configuration before exposing the service:

| Area | Variables | Production guidance |
|------|-----------|---------------------|
| Runtime | `ENVIRONMENT`, `PORT`, `RUST_LOG` | Set `ENVIRONMENT=production`; log structured operational data without payloads or credentials |
| Persistence | `DATABASE_URL`, `DB_MAX_CONNECTIONS`, `DB_MIN_CONNECTIONS` | Require PostgreSQL TLS, a least-privilege role, bounded pools, migrations, backups, and restore tests |
| Authentication | `JWT_SECRET`, `JWT_EXPIRATION_HOURS`, `AUTH_HASH_PEPPER`, token-specific pepper overrides, `BCRYPT_COST` | Generate independent high-entropy secrets, inject at runtime, and rotate under an incident-tested procedure |
| Browser/API boundary | `CORS_ALLOWED_ORIGINS` | Enumerate HTTPS origins; wildcards are rejected in production |
| Abuse controls | `RATE_LIMIT_*_RPM`, `RATE_LIMIT_*_BURST` | Size each endpoint tier from measured traffic and alert on sustained rejection rates |
| Node lifecycle | `NODE_HEARTBEAT_TIMEOUT_MINUTES`, `NODE_OFFLINE_SWEEP_INTERVAL_SECONDS` | Choose a timeout above normal network jitter but below the maximum acceptable failover interval |

```bash
cp .env.example .env
export ENVIRONMENT=production
export JWT_SECRET="$(openssl rand -base64 48)"
export AUTH_HASH_PEPPER="$(openssl rand -base64 48)"
export DATABASE_URL='postgres://vcp@db.internal/vcp?sslmode=require'
export CORS_ALLOWED_ORIGINS='https://console.example.com'
cargo run --release --bin api-server
```

### Production Hardening

- Terminate TLS at a hardened reverse proxy or load balancer; restrict database, metrics, observability, and coordinator ports to private networks.
- Run the API and workers as distinct, non-root identities with read-only filesystems, minimal Linux capabilities, CPU/memory limits, and separate credentials.
- Pin the Rust lockfile and container digests, scan dependencies and images, protect CI signing material, and stage migrations before rollout.
- Centralize immutable audit logs and metrics; alert on authentication failures, proof rejections, node churn, task backlog, and offline-policy imports.
- Back up PostgreSQL and policy/signing keys separately, test restoration, and document JWT/key rotation and node-revocation procedures.

### Offline-First Configuration

Prime each node with the minimum required egress policies, verification keys, and signed session leases while the control plane is reachable. Pin trusted Ed25519 peer keys out of band, bound lease lifetimes, retain the tamper-evident audit queue, and test transitions through `OnlineControlPlane`, `OfflineControlPlane`, and `NoUpstream`. Offline operation must fail closed when a signature, expiry, destination, or capability check cannot be satisfied; it does not bypass task or egress policy.

### Gateway Mode

Use a dedicated `open_internet` or `any` node and expose only the configured relay listener. The gateway consumes coordinator-issued sessions and should never be deployed as an unauthenticated general proxy:

```bash
ambient-vcp gateway --listen 0.0.0.0:7000 \
  --sessions-file /run/ambient-vcp/gateway-sessions.json \
  --connect-timeout-seconds 5 --idle-timeout-seconds 600
```

Keep the session file owner-readable only, enforce destination allowlists and expiry, revoke sessions immediately when tasks end, leave backhaul routing in monitor-only mode until deliberately enabled, and apply host firewall plus relay QoS rules to the selected WAN interface.

### Recommended Security Settings

- Use unique 48-byte-or-stronger JWT, refresh-token, API-key, and connect-session secrets; never reuse development values.
- Keep bcrypt at cost 12 or benchmark a stronger tolerable value; use short JWT lifetimes and rotate refresh tokens.
- Retain the default endpoint-specific rate limits as a floor, restrict CORS to explicit HTTPS origins, and protect `/metrics` with admin authentication and network policy.
- Permit only registered task/node types and bounded capabilities; enable proof requirements for workloads whose result integrity crosses an untrusted-node boundary.
- Disable public local observability, database access, and peer-sync listeners; allow only authenticated peers and required ingress paths.

For platform-specific rollout and rollback procedures, see [`docs/DEPLOYMENT.md`](./docs/DEPLOYMENT.md) and [`docs/NODE_SECURITY.md`](./docs/NODE_SECURITY.md).

---

## 🌐 Deployment Options

### Docker (Recommended)

```bash
# Quick start with Docker Compose
docker-compose up -d

# Access the API
curl http://localhost:3000/api/v1/health
```

### Render.com (One-Click Deploy)

```bash
# Deploy to Render.com
render blueprint apply

# Your API will be at:
# https://ambient-ai-vcp-system.onrender.com
```

### Kubernetes

```bash
# Build and push image
docker build -t registry/ambient-vcp:latest .
docker push registry/ambient-vcp:latest

# Deploy to Kubernetes
kubectl apply -f k8s/deployment.yaml
kubectl apply -f k8s/service.yaml
```

### Production Security Checklist

**Before deploying to production:**

- [ ] Set `ENVIRONMENT=production` environment variable
- [ ] Generate secure `JWT_SECRET` (min 32 chars): `openssl rand -base64 32`
- [ ] Configure `DATABASE_URL` with PostgreSQL connection string
- [ ] Use managed PostgreSQL with SSL/TLS enabled
- [ ] Enable HTTPS (automatic with Render.com, configure for self-hosted)
- [ ] Configure proper CORS origins (not `*` in production)
- [ ] Set appropriate rate limits for your traffic
- [ ] Configure database backups
- [ ] Monitor logs for security events
- [ ] Never commit `.env` or secrets to git
- [ ] Review and run database migrations
- [ ] Test authentication flow in production environment

**Environment Variables Required:**
```bash
# Authentication (REQUIRED)
JWT_SECRET=<generate-with-openssl-rand-base64-32>
JWT_EXPIRATION_HOURS=24

# Database (REQUIRED)
DATABASE_URL=postgres://user:password@host:5432/dbname
DB_MAX_CONNECTIONS=10
DB_MIN_CONNECTIONS=2

# Environment
ENVIRONMENT=production

# Optional
PORT=3000
HOST=0.0.0.0
```

**First-Time Setup:**
```bash
# 1. Run database migrations
cargo run --bin api-server

# 2. Create admin user
curl -X POST https://your-api.com/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"secure-password"}'

# 3. Test authentication
curl -X POST https://your-api.com/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"secure-password"}'

# 4. Access dashboard
# Visit https://your-api.com and login
```

---

## 📊 Performance Targets

| Metric | Target | Actual Performance | Status |
|--------|--------|-------------------|--------|
| Task Assignment Latency | < 100ms | **< 0.003ms** (2.75µs avg) | ✅ **Exceeds by 33,333x** |
| WASM Execution | < 2x native slowdown | ~1.5x slowdown | ✅ Achieved |
| Proof Generation | < 10s | **~1-2s** | ✅ **5-10x faster** |
| Proof Verification | < 1s | **< 100ms** | ✅ **10x faster** |
| Concurrent Tasks | 1000+ | **171,204 tasks/sec** | ✅ **171x capacity** |
| Node Capacity | 10,000+ | **343,573 nodes/sec**, 10,000+ stored | ✅ **Validated at scale** |

**Load Test Results:**
- ✅ Successfully handled 1,000 concurrent task submissions in 6ms
- ✅ Successfully registered 10,000 nodes in 29ms  
- ✅ Stress tested with 1,000 nodes + 1,000 tasks simultaneously
- ✅ Average task assignment latency: 2.75 microseconds

---

## 🧪 Executable Specification (Living Contract)

The test files included in this repository are not just unit tests — they serve as a **living, runnable specification** of the Ambient AI VCP System.

Each test encodes the exact return contract for:

- **AILEE Trust Layer** (`GenerationResult`)
- **AmbientNode health, safety, and reputation**
- **MeshCoordinator routing, selection, and reward flow**
- **FederatedAggregator FedAvg rounds and versioning**

These tests ensure that:

- the system’s behavior is deterministic  
- return structures remain stable across versions  
- contributors can understand the architecture by *running* it  
- regressions are caught immediately  
- the repo doubles as documentation and verification  

If you are extending or modifying the system, update the tests to reflect the new contract.  
If you are integrating the system, use these tests as the authoritative reference for expected behavior.

---

## 🛣️ Roadmap

The roadmap prioritizes verifiable execution and operational safety over surface-area growth. Near-term work completes observability, Byzantine-resilient coordination, libp2p transport, independent security review, and MFA. Planned improvements then deepen scheduling, mobile/edge support, reproducible proof circuits, policy/key rotation, and failure-injection coverage. The long-term goal is an interoperable, decentralized compute fabric with auditable governance and privacy-preserving execution across heterogeneous nodes.

### ✅ Phase 1 - Core Infrastructure (COMPLETED)
- ✅ Ambient node implementation
- ✅ WASM execution engine
- ✅ Mesh coordinator
- ✅ ZK proof placeholder
- ✅ CLI tool
- ✅ Basic documentation

### ✅ Phase 2 - Production Features (COMPLETED)
- ✅ Federated learning (FedAvg + Differential Privacy)
- ✅ Multi-node demo application
- ✅ Web dashboard (Real-time monitoring)
- ✅ REST API server (Axum + OpenAPI/Swagger)
- ✅ Render.com deployment configuration
- ✅ Production ZK proofs (Groth16 on BN254)

### ⭐ Phase 2.5 - Robustness Enhancements (COMPLETED)
- ✅ **Zero compiler warnings**
- ✅ **Comprehensive input validation**
- ✅ **Integration test suite (13 tests)**
- ✅ **Improved error handling**
- ✅ **Enhanced documentation**
- ✅ **Production ZK proofs with Groth16**

### ⭐ Phase 2.6 - Security & Authentication (COMPLETED) **NEW**
- ✅ **JWT Authentication** - Secure token-based auth with configurable expiration
- ✅ **User Registration & Login** - Account creation with bcrypt password hashing
- ✅ **Node Ownership** - Foreign key linking nodes to user accounts
- ✅ **Authorization** - Users can only manage their own nodes
- ✅ **Node Lifecycle Management** - Delete nodes with ownership verification
- ✅ **Heartbeat Mechanism** - Track node availability and detect offline nodes
- ✅ **Dashboard Authentication** - Integrated login/logout with JWT storage
- ✅ **Security Documentation** - Comprehensive guides and best practices
- ✅ **Data Persistence** - PostgreSQL with migrations

### ⭐ Phase 2.7 - Offline-First Node Connectivity & AILEE Metric (COMPLETED) 🆕
- ✅ **AILEE ∆v Metric** — energy-weighted optimization gain functional from the AILEE paper; accumulates telemetry samples and produces a dimensionless efficiency score for comparative diagnostics
- ✅ **Overflow-safe resonance gates** — exponential terms in ∆v are clamped to `[-700, 700]` before evaluation to prevent `f64` overflow under extreme telemetry values
- ✅ **Peer-to-Peer Policy Sync** — nodes share cryptographically-verified policy snapshots directly without the control plane, keeping the mesh operational and internet-capable in `OfflineControlPlane` and `NoUpstream` states
- ✅ **Full-content integrity hashing** — `PeerPolicySyncMessage` hashes policy IDs *and* allowed destinations *and* full verification-key bytes, ensuring that modifications to any component (policy IDs, destinations, or keys) invalidate the hash
- ✅ **Persistent chained audit log** — every `import_peer_sync` call appends a `peer_sync_applied` record to a SHA3 hash-chained audit queue, providing a tamper-evident history of all policy imports
- ✅ **Ed25519 session lease signing** — `SessionLease` payloads are signed with Ed25519 and verified fully offline, enabling a node to authenticate new sessions without ever contacting the control plane
- ✅ **Three-state node model** — `LocalSessionManager` tracks `OnlineControlPlane`, `OfflineControlPlane`, and `NoUpstream` states, enforcing appropriate policy restrictions at each tier
- ✅ **Mesh connectivity analysis & peer routing** — `PeerRouter` classifies each node's reachability and resolves forwarding paths; Universal nodes are preferred over Open nodes to minimise relay depth
- ✅ **Real-time session revocation** — `DataPlaneGateway::revoke_session()` removes a session from the live store instantly, stopping traffic relay the moment a connect session ends
- ✅ **70 new tests** across `ailee-trust-layer` and `ambient-node` crates

### ⭐ Phase 2.8 - Routing, Auth Hardening & Operational Reliability (COMPLETED) 🆕
- ✅ **Internet Path Routing** — `PeerRouter` added to `mesh-coordinator`; classifies node reachability and resolves direct or relay forwarding paths through `Universal`/`Open` nodes
- ✅ **Gateway Session Lifecycle** — `DataPlaneGateway` gains `add_session()` and `revoke_session()` for runtime session management; relaying stops the moment a session is revoked
- ✅ **Non-Blocking Password Hashing** — `hash_password_async()` offloads bcrypt to `spawn_blocking`; cost configurable via `BCRYPT_COST` env var (default 12)
- ✅ **Pepper Config Polish** — all pepper env vars pre-configured in `docker-compose.yml` and `.env.example`; missing-pepper warnings downgraded to `debug` in development
- ✅ **Heartbeat-Triggered Task Assignment** — `update_node_heartbeat` now calls `assign_pending_tasks_for_node`, closing the gap where live nodes only received tasks at registration time
- ✅ **Self-Hosted Dashboard Fonts** — Syne and JetBrains Mono bundled as woff2 assets; no CDN dependency, dashboard works fully offline and in air-gapped deployments
- ✅ **Safe-Default Backhaul Routing** — `monitor_only = true` default prevents unintended kernel routing changes; `ip rule` entries scoped to source IP; health probes bound to the interface under test for accurate per-interface metrics

### ⭐ Phase 2.9 - Relay QoS for connect_only Tasks (COMPLETED)
- ✅ **WAN-side Relay QoS** — `RelayQosManager` installs Linux `tc` HTB + FQ-CoDel rules on the active WAN backhaul interface when a `connect_only` session starts on an `open_internet` or `any` node; relay traffic receives a guaranteed minimum bandwidth and a burst ceiling while node-internal traffic is protected by a separate reserved floor — eliminating congestion between relay streams and node control traffic
- ✅ **DSCP/TOS Classification** — egress packets already marked with DSCP EF (value 46) are steered into the high-priority relay HTB class via a `u32` filter; the HTB default class is also set to the relay class so unmarked relay TCP connections benefit without requiring end-to-end DSCP support
- ✅ **Bufferbloat Reduction** — an FQ-CoDel qdisc is attached to the relay class by default, providing active queue management and per-flow fairness that keeps relay session latency low even under sustained throughput
- ✅ **`BackhaulManager` integration** — new `activate_relay_qos()` and `deactivate_relay_qos()` methods apply or remove the WAN QoS rules against the currently active interface; `RelayQosConfig` is part of `BackhaulConfig` with safe production defaults (10 Mbps guaranteed, 1 Gbps ceiling, 1 Mbps node floor)
- ✅ **10 new tests** across `relay_qos` unit tests and `BackhaulManager` integration tests

### ⭐ Phase 2.10 - Hardware Keepalive & Node Heartbeat Tracking (COMPLETED) 🆕
- ✅ **Hardware Keepalive** — `BackhaulManager::hardware_keepalive_tick(now_secs)` emits periodic low-level keepalive probes at a configurable interval (`HardwareKeepaliveConfig`), preventing NAT and stateful-firewall session expiry on idle `connect_only` relay links
- ✅ **Node Heartbeat Tracking** — `NodeRegistry::record_heartbeat(id, now_secs)` and `is_node_alive(id, now_secs, timeout_secs)` give the mesh coordinator a lightweight, no-network-round-trip liveness signal for each registered node
- ✅ **`internet_required()` on `LocalSessionManager`** — returns `true` when any active local session needs outbound internet, enabling `BackhaulManager` to prioritise WAN interface selection for relay tasks

### ⭐ Phase 2.11 - FEEN Physics Engine Integration (COMPLETED) 🆕
- ✅ **`feen_resonator` node type** — new VCP node wrapping a FEEN Duffing resonator; owns physical state `(x, v)`, coupling configuration, and an `AileeMetric` accumulator that tracks ∆v across ticks
- ✅ **`feen_connectivity` task type** — new VCP task that uses FEEN to compute resonance, interference, stability, and ∆v across a group of resonator nodes
- ✅ **`FeenClient` Rust HTTP client** — posts to the local FEEN REST API (`/api/v1/simulate`, `/api/v1/coupling`) using `reqwest`; error handling propagates FEEN API status codes as typed `Result` errors
- ✅ **`FeenEngine` trait boundary** — clean async trait separates VCP logic from the FEEN transport, keeping the HTTP client swappable with in-process mocks for unit tests
- ✅ **Stateless simulation contract** — resonator state lives entirely in VCP; each `/api/v1/simulate` call is a pure function `(config, state, input, dt) → state′` with no server-side persistence
- ✅ **FEEN-side minimal REST surface** — three endpoints added under `feen-changes/`: `/simulate`, `/coupling`, `/delta_v`; no FEEN internals exposed beyond what VCP needs
- ✅ **12 new unit tests** covering construction, physics mock, error propagation, coupling updates, and JSON serialisation round-trips for all FEEN VCP types
- 🔗 **FEEN repository**: [https://github.com/dfeen87/FEEN](https://github.com/dfeen87/FEEN)

### 🔄 Phase 3 - Advanced Features (IN PROGRESS)
- [x] Authentication & authorization (JWT/API keys) ✅ **COMPLETED**
- [x] Data persistence (PostgreSQL) ✅ **COMPLETED**
- [x] Rate limiting (tiered endpoint limits) ✅ **COMPLETED**
- [ ] Metrics & monitoring (Prometheus)
- [ ] Byzantine fault tolerance
- [ ] P2P networking layer (libp2p)
- [ ] Production security audit
- [x] Token refresh mechanism ✅ **COMPLETED**
- [ ] Multi-factor authentication

### 🔮 Future Phases
- [ ] Mobile node support
- [ ] Advanced orchestration algorithms
- [ ] Cross-chain integration
- [ ] Decentralized governance

---

## 📁 Project Structure

```
ambient-vcp/
├── Cargo.toml                      # Workspace configuration
├── Cargo.lock                      # Dependency lock file
├── README.md                       # This file
├── CITATION.cff                    # Citation metadata for research
├── LICENSE                         
├── Dockerfile                      # Docker container configuration
├── docker-compose.yml              # Multi-container orchestration
├── render.yaml                     # Render.com deployment config
├── .env.example                    # Environment variables template
│
├── crates/                         # Rust workspace crates
│   ├── ambient-node/               # Node implementation + 110 tests
│   │   ├── src/offline.rs          #   LocalSessionManager + PeerPolicySyncMessage
│   │   └── src/connectivity/       #   Multi-backhaul, hotspot, tether subsystems
│   ├── ailee-trust-layer/          # AILEE Trust Layer + 38 tests
│   │   └── src/metric.rs           #   AileeMetric (∆v), AileeSample, AileeParams
│   ├── wasm-engine/                # WASM execution runtime + 6 tests
│   ├── zk-prover/                  # ZK proof generation (Groth16) + 8 tests
│   ├── mesh-coordinator/           # Task orchestration + peer routing + 21 tests
│   ├── federated-learning/         # FL protocol + 8 tests
│   ├── api-server/                 # REST API server + 62 tests (36 unit + 24 integration + 2 load/smoke)
│   └── cli/                        # Command-line interface
│
├── docs/                           # Documentation
│   ├── API_REFERENCE.md            # API endpoint documentation
│   ├── ARCHITECTURE.md             # System architecture details
│   ├── CONTRIBUTING.md             # Contribution guidelines
│   ├── DEPLOYMENT.md               # Deployment instructions
│   ├── GLOBAL_NODE_DEPLOYMENT.md   # Global node setup guide
│   ├── LANGUAGE_DECISION.md        # Technology stack rationale
│   ├── IMPLEMENTATION_SUMMARY.md   # Implementation overview
│   ├── PHASE1_SUMMARY.md           # Phase 1 development summary
│   ├── PHASE2_SUMMARY.md           # Phase 2 development summary
│   ├── PHASE2.md                   # Phase 2 planning document
│   ├── TESTING_SUMMARY.md          # Testing strategy and results
│   └── whitepapers/                # Research whitepapers
│       ├── AMBIENT_AI.md           # Ambient AI whitepaper
│       └── VCP.md                  # VCP protocol whitepaper
│
├── .github/                        # GitHub configurations
│   └── workflows/                  # CI/CD pipelines
│       └── ci.yml                  # Main CI workflow (tests, lint, build)
│
├── demo/                           # Demonstration scripts
│   ├── README.md                   # Demo documentation
│   └── run-demo.sh                 # Multi-node demo script
│
├── scripts/                        # Utility scripts
│   └── deploy-global-node.sh       # Global node deployment automation
│
├── examples/                       # Example implementations
│   └── hello-compute/              # Simple WASM compute example
│
├── wasm-modules/                   # WASM module storage
│   └── README.md                   # WASM modules documentation
│
├── v0.3-reference/                 # Legacy reference implementation
│   ├── README.md                   # v0.3 documentation
│   ├── package.json                # Node.js dependencies (legacy)
│   └── *.js                        # JavaScript implementation files
│
└── archive/                        # Archived files
    └── README_OLD.md               # Previous README version
```

**Key Directories:**
- `crates/` - Core Rust implementation with 246 passing tests
- `docs/` - Comprehensive documentation and whitepapers
- `.github/workflows/` - Automated CI/CD with tests, linting, and builds
- `crates/api-server/assets/` - Embedded dashboard + custom Swagger UI assets
- `scripts/` - Deployment and utility scripts

---

## 🤝 Contributing

Contributions are welcome! Please read our contributing guidelines before submitting PRs.

### Local Development

```bash
# Format, lint, and exercise the complete workspace.
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# Database-backed API tests use an isolated PostgreSQL database.
TEST_DATABASE_URL=postgres://postgres:postgres@localhost/ambient_vcp_test \
  cargo test -p api-server --test integration_test
```

### Adding a Node Type

1. Add the canonical node type to API registration validation and document its bounded capabilities.
2. Implement behavior in `ambient-node` behind a narrow trait boundary; keep transport, policy, and execution state separable.
3. Extend coordinator eligibility/routing rules and add unit tests for selection, heartbeat, offline, and rejection paths.
4. Add API integration tests and update OpenAPI-facing models and operator documentation. Never accept arbitrary capability names as executable authority.

### Adding a Task Type

1. Add one entry to `TASK_TYPE_REGISTRY` with its preferred node type, minimum capabilities, payload ceiling, execution deadline, and WASM policy.
2. Validate the input schema before persistence, define completion/disconnection semantics, and bind any proof to an explicit circuit and public-input format.
3. Implement worker execution and coordinator assignment without weakening sandbox, ownership, or admission-control guarantees.
4. Test valid intake, malformed and oversized inputs, insufficient capacity, reassignment, result submission, and proof-required failure cases.

### Working With the Coordinator Locally

```bash
# Terminal 1: standalone in-memory coordinator
cargo run -p ambient-vcp-cli -- coordinator \
  --cluster-id local-dev --strategy weighted

# Terminal 2: a local worker with operator-only observability
cargo run -p ambient-vcp-cli --features observability -- node \
  --id dev-node --region local --node-type compute \
  --observability --observability-port 9090
```

The standalone CLI is useful for coordinator and node logic. Use the API server plus `TEST_DATABASE_URL` when changing authenticated registration, durable assignment, heartbeat, or result lifecycle behavior. Keep tests deterministic, avoid real network dependencies in unit tests, and update the executable specification when a public contract changes.

### Development Workflow

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run tests (`cargo test`)
5. Ensure zero warnings (`cargo build --release`)
6. Commit your changes (`git commit -m 'Add amazing feature'`)
7. Push to the branch (`git push origin feature/amazing-feature`)
8. Open a Pull Request

---

## 📄 License

This project is fully open-source and licensed under the terms of the [MIT License](LICENSE) in the included LICENSE file.

---

## 🙏 Acknowledgments

- **WasmEdge** for WASM runtime
- **arkworks** for production ZK proof libraries (Groth16)
- **Axum** for the web framework
- The decentralized computing community for verifiable computation research

This project was developed with a combination of original ideas, hands‑on coding, and support from advanced AI systems. I would like to acknowledge **Microsoft Copilot**, **Anthropic Claude**, **Google Jules**, and **OpenAI ChatGPT** for their meaningful assistance in refining concepts, improving clarity, and strengthening the overall quality of this work.

---

## Enterprise Consulting & Integration
This architecture is fully open-source under the MIT License. If your organization requires custom scaling, proprietary integration, or dedicated technical consulting to deploy these models at an enterprise level, please reach out at: dfeen87@gmail.com

## 📧 Support & Contact

- 📖 **Documentation**: See `/docs` directory
- 🐛 **Issues**: [GitHub Issues](https://github.com/dfeen87/Ambient-AI-VCP-System/issues)
- 💬 **Discussions**: [GitHub Discussions](https://github.com/dfeen87/Ambient-AI-VCP-System/discussions)

---

<div align="center">

**Built with ❤️ for decentralized AI compute**

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![WebAssembly](https://img.shields.io/badge/WebAssembly-654FF0?style=for-the-badge&logo=WebAssembly&logoColor=white)](https://webassembly.org/)

**Status**: Production-Ready | **Version**: 3.2.0 | **Tests**: 274 Passing ✅

</div>
