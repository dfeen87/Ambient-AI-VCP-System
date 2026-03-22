# Ambient AI + VCP System

A production-ready implementation of a **Verifiable Computation Protocol (VCP)** that orchestrates AI workloads across heterogeneous edge devices with cryptographic verification and zero-knowledge proofs.

## 🌟 Features

- **Ambient Node Mesh**: Self-organizing network of heterogeneous devices
- **Intelligent Orchestration**: Health-based task assignment with reputation scoring
- **WASM Execution Engine**: Secure sandboxed computation with resource limits
- **Zero-Knowledge Proofs**: Cryptographic verification of execution correctness
- **Federated Learning**: Privacy-preserving multi-node model training with FedAvg and differential privacy
- **Verifiable Computation**: Proof-of-Execution for trustless distributed computing
- **Energy Telemetry**: Verifiable sustainability metrics

## 🏗️ Architecture

### Core Components

1. **Ambient Node** (`ambient-node`)
   - Telemetry collection (energy, compute, privacy budgets)
   - Health scoring based on bandwidth, latency, compute efficiency, and reputation
   - Safety circuit breakers (temperature, latency thresholds)
   - Reputation tracking (completed tasks, disputes)

2. **WASM Execution Engine** (`wasm-engine`)
   - Secure sandboxed execution with WasmEdge runtime
   - Resource limits: memory (512MB default), timeout (30s), gas metering
   - Execution trace recording for ZK proofs
   - Determinism checking for reproducibility

3. **ZK Proof System** (`zk-prover`)
   - Execution trace to proof conversion (placeholder)
   - Universal verifier for WASM programs
   - Designed for RISC Zero zkVM or Plonky2 integration

4. **Mesh Coordinator** (`mesh-coordinator`)
   - Node registry with health tracking
   - Task assignment strategies: Weighted, Round-robin, Least-loaded, Latency-aware
   - Proof verification and reward distribution

5. **Federated Learning** (`federated-learning`)
   - FedAvg aggregation algorithm
   - Differential privacy with configurable ε and δ
   - Gradient clipping and noise injection
   - Client-side model training interface

6. **REST API Server** (`api-server`)
   - Node registration and management
   - Task submission and tracking
   - Proof verification endpoints
   - OpenAPI/Swagger documentation
   - Real-time cluster statistics

7. **CLI Tool** (`cli`)
   - Start and manage nodes
   - Start mesh coordinators
   - Health monitoring

8. **Web Dashboard** (`dashboard`)
   - Real-time node monitoring
   - Task management interface
   - Health metrics visualization
   - Interactive node registration

## 📚 Language & Technology Stack

This is the **v1.0 production implementation** built in **Rust** for performance, safety, and scalability.

### Why Rust for v1.0?

- **Performance**: Near-native execution speed for compute-intensive workloads
- **Memory Safety**: Zero-cost abstractions with compile-time guarantees
- **WASM Support**: First-class support via WasmEdge for secure sandboxed execution
- **Concurrency**: Tokio async runtime for high-throughput distributed systems
- **Production-Ready**: Strong type system and error handling for reliable deployments

### v0.3-alpha Reference Implementation

A **JavaScript/Node.js reference implementation** demonstrating the foundational "Proof-of-Compute" architecture is available in `/v0.3-reference/`. This implementation uses:

- **libp2p** for decentralized P2P networking
- **snarkjs** for zero-knowledge proof generation
- **Circom** for ZK circuit definitions

See [`/v0.3-reference/README.md`](./v0.3-reference/README.md) for details.

### Language Decision

For a comprehensive analysis of language choices, technology stack decisions, and the evolution from v0.3 to v1.0, see:

📖 **[Language & Technology Decision Document](./docs/LANGUAGE_DECISION.md)**

## 🚀 Quick Start

### Prerequisites

- Rust 1.75 or later
- WasmEdge SDK (for WASM execution)

### Installation

```bash
# Clone the repository
git clone https://github.com/dfeen87/Ambient-AI-VCP-System.git
cd Ambient-AI-VCP-System

# Build the project
cargo build --release

# Run tests
cargo test
```

### Running a Node

```bash
# Start a compute node
cargo run --bin ambient-vcp -- node --id node-001 --region us-west --node-type compute

# Start with custom configuration
cargo run --bin ambient-vcp -- node --id node-002 --region eu-central --node-type gateway
```

### Running a Coordinator

```bash
# Start a mesh coordinator
cargo run --bin ambient-vcp -- coordinator --cluster-id cluster-001 --strategy weighted

# Use different assignment strategy
cargo run --bin ambient-vcp -- coordinator --cluster-id cluster-002 --strategy latency-aware
```

### Running the API Server (Phase 2)

```bash
# Start the REST API server
cargo run --bin api-server

# Server starts on http://localhost:3000
# Swagger UI: http://localhost:3000/swagger-ui
# Health check: http://localhost:3000/api/v1/health
```

### Running the Multi-Node Demo (Phase 2)

```bash
# Run the complete demo
./demo/run-demo.sh

# This will:
# 1. Start the API server (if not running)
# 2. Register 3 nodes across different regions
# 3. Submit federated learning task
# 4. Submit ZK proof task
# 5. Verify proofs
# 6. Display cluster statistics
```

### Accessing the Dashboard (Phase 2)

```bash
# Open the web dashboard
open dashboard/index.html

# Or navigate to it in your browser
# Configure API URL to http://localhost:3000
# View real-time cluster metrics and manage nodes
```

## 🌐 Global Node Deployment

The VCP system can be deployed as a **global online API** that anyone can connect to:

```bash
# Quick start with Docker Compose
docker-compose up -d

# Access the API
curl http://localhost:3000/api/v1/health

# View Swagger docs
open http://localhost:3000/swagger-ui
```

This starts a complete global network with:
- **API Server** - Public REST API on port 3000
- **Mesh Coordinator** - Task orchestration
- **Multi-Region Nodes** - Distributed compute across US, EU, and APAC

### Deploy to Cloud

Deploy to Render.com with one click:
```bash
render blueprint apply
```

Your API will be available at: `https://ambient-ai-vcp-api.onrender.com`

**📖 Full deployment guide**: [Global Node Deployment](./docs/GLOBAL_NODE_DEPLOYMENT.md)

### Connect to a Global Node

```bash
# Register your local node to a global coordinator
export VCP_API_URL=https://ambient-vcp-api.onrender.com

cargo run --bin ambient-vcp -- node \
  --id my-node \
  --coordinator-url $VCP_API_URL
```

### Health Check

```bash
cargo run --bin ambient-vcp -- health
```

## 📊 Health Scoring

Node health is calculated using a weighted formula:

- **Bandwidth**: 40% weight (max 1000 Mbps)
- **Latency**: 30% weight (lower is better, max 100ms)
- **Compute Efficiency**: 20% weight (CPU + Memory availability)
- **Reputation**: 10% weight (task success rate)

Score = (bandwidth × 0.4) + (latency × 0.3) + (compute × 0.2) + (reputation × 0.1)

## 🔒 Safety & Security

### Circuit Breakers

Nodes automatically enter safe mode when:
- Temperature exceeds 85°C
- Latency exceeds 100ms
- Error count exceeds 25 consecutive failures

### Sandbox Limits

WASM execution is restricted by:
- Memory: 512MB default (configurable)
- Timeout: 30 seconds
- Max instructions: 10 billion
- No filesystem access
- No network access
- Cryptographic operations allowed

## 📁 Project Structure

```
ambient-vcp/
├── Cargo.toml              # Workspace configuration
├── README.md               # This file
├── crates/
│   ├── ambient-node/       # Node implementation
│   ├── wasm-engine/        # WASM execution runtime
│   ├── zk-prover/          # ZK proof generation
│   ├── mesh-coordinator/   # Task orchestration
│   ├── federated-learning/ # FL protocol
│   ├── api-server/         # REST API server
│   └── cli/                # Command-line interface
└── docs/                   # Documentation
```

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p ambient-node

# Run with logging
RUST_LOG=info cargo test
```

## 📈 Performance Targets

- **Task Assignment Latency**: < 100ms
- **WASM Execution**: Native performance (< 2x slowdown)
- **Proof Generation**: < 10s for most tasks (placeholder)
- **Proof Verification**: < 1s (placeholder)
- **Throughput**: 1000+ concurrent tasks (planned)
- **Node Capacity**: 10,000+ nodes per cluster (planned)

## 🛣️ Roadmap

### Phase 1 (Completed) ✅
- ✅ Core infrastructure (ambient-node, wasm-engine, mesh-coordinator)
- ✅ WASM execution with resource limits
- ✅ Execution trace recording
- ✅ ZK proof placeholder
- ✅ CLI tool
- ✅ Basic documentation

### Phase 2 (Completed) ✅
- ✅ Federated learning implementation (FedAvg + Differential Privacy)
- ✅ Multi-node demo application
- ✅ Web dashboard (Real-time monitoring)
- ✅ REST API server (Axum with OpenAPI/Swagger)
- ✅ Render.com deployment configuration
- ⚠️ Real ZK proof generation (Placeholder - RISC Zero integration pending)

### Phase 3 (Future)
- [ ] Byzantine consensus
- [ ] Network P2P layer (libp2p)
- [ ] Production-grade security audit
- [ ] Advanced metrics and monitoring
- [ ] Mobile node support

## 🤝 Contributing

Contributions are welcome! Please read our contributing guidelines before submitting PRs.

## 📄 License

Non-Commercial License - see LICENSE file for details

## 🙏 Acknowledgments

- WasmEdge for WASM runtime
- RISC Zero for ZK VM inspiration
- The decentralized computing community for verifiable computation research

## 📧 Contact

For questions or support, please open an issue on GitHub.

---

Built with ❤️ for decentralized AI compute
