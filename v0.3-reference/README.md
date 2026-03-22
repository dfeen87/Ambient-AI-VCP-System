# v0.3-alpha Reference Implementation

This directory contains the **v0.3-alpha reference implementation** of the Verifiable Computation Protocol (VCP), as described in the VCP white paper. This is a **JavaScript/Node.js** implementation that demonstrates the foundational "Proof-of-Compute" architecture.

## ⚠️ Purpose

This is a **reference implementation** for educational and documentation purposes. It demonstrates the core concepts:

- Decentralized P2P mesh networking (libp2p)
- Zero-knowledge proof generation and verification (snarkjs + Circom)
- Trustless economic settlement (automated escrow and payment)

**For production use, see the v1.0 Rust implementation in the main crates/ directory.**

## 🏗️ Architecture

The v0.3 system consists of three actors:

1. **ambient-node.js** - The Worker Node
   - Listens for compute tasks on the P2P mesh
   - Performs computation (calculates square root)
   - Generates ZK-SNARK proofs using snarkjs
   - Publishes proofs to the network

2. **ambient-client.js** - The Job Requester
   - Posts compute tasks to the network
   - Offers rewards for completed work
   - Waits for ledger confirmation of payment

3. **ambient-ledger.js** - The Autonomous Verifier
   - Monitors the network for tasks and proofs
   - Holds client funds in escrow
   - Verifies ZK-proofs cryptographically
   - Automatically releases payments to workers upon valid proof

## 🚀 Quick Start

### Prerequisites

- Node.js v18 or later
- npm
- Circom compiler (`npm install -g circom`)

### Installation

```bash
cd v0.3-reference
npm install
```

### One-Time Setup

Generate the ZK-proof keys (only run once):

```bash
node setup.js
```

This will:
- Compile the `circuit.circom` file
- Perform the "Powers of Tau" ceremony
- Generate `proving_key.zkey` for workers
- Generate `verification_key.json` for verifiers

### Running the Network

Open **three separate terminals** in the `v0.3-reference` directory:

**Terminal 1: Start the Ledger (Smart Contract)**
```bash
node ambient-ledger.js
```

**Terminal 2: Start a Worker Node**
```bash
node ambient-node.js
```

**Terminal 3: Start the Client**
```bash
node ambient-client.js
```

### Watch the Magic ✨

You'll see the complete trustless economic loop:

1. **[Client]** publishes: `TASK:compute:y=49` (find x where x²=49)
2. **[Ledger]** escrows the reward
3. **[Worker]** receives task and computes the answer (x=7)
4. **[Worker]** generates ZK-proof and publishes it
5. **[Ledger]** verifies the proof cryptographically
6. **[Ledger]** automatically pays the worker
7. **[Client]** receives payment confirmation

## 📋 Files

- **ambient-node.js** - Worker node implementation
- **ambient-client.js** - Job requester implementation  
- **ambient-ledger.js** - Autonomous verifier and settlement node
- **circuit.circom** - ZK circuit proving x² = y
- **setup.js** - One-time key generation ceremony
- **package.json** - Dependencies and scripts
- **README.md** - This file

## 🔑 Key Technologies

- **libp2p** - Decentralized P2P networking (mesh, gossipsub, mDNS)
- **snarkjs** - Zero-knowledge proof generation and verification
- **Circom** - ZK circuit definition language
- **Node.js** - Runtime environment

## 🎯 What This Demonstrates

### The "Architecture of Truth"

This implementation proves that a **trustless economic loop** is possible:

```
Task → Escrow → Compute → Prove → Verify → Settle
```

Key principles:
- **No central authority** - All nodes are peers
- **Cryptographic verification** - Proofs replace trust
- **Automated settlement** - Smart contract logic via the Ledger node
- **Privacy preserving** - Worker's private input (x) never revealed

### Limitations (Addressed in v1.0)

The v0.3 implementation has intentional limitations:

1. **Fixed Computation** - Hard-coded to compute square roots (x²=y)
2. **Simple Circuit** - Cannot verify arbitrary computations or AI workloads
3. **Basic Networking** - Uses broadcast pubsub instead of intelligent routing

These are systematically addressed in the **v1.0 Rust implementation**.

## 📖 Evolution to v1.0

The v1.0 implementation (in the main repository) upgrades this foundation:

| Component | v0.3 (JavaScript) | v1.0 (Rust) |
|-----------|-------------------|-------------|
| **Compute Sandbox** | Hard-coded sqrt | WASM runtime (arbitrary code) |
| **ZK System** | Simple x²=y circuit | Universal execution trace proofs |
| **Networking** | libp2p gossipsub | Reputation-based orchestration |
| **AI Support** | None | TensorFlow.js models → Full WASM AI |
| **Language** | JavaScript/Node.js | Rust (performance + safety) |

## 🤔 Why JavaScript for v0.3?

JavaScript/Node.js was chosen for v0.3 because:

1. **Rapid prototyping** - Faster to prove the concept
2. **libp2p maturity** - Excellent JavaScript implementation
3. **snarkjs** - Production-ready ZK library for JS
4. **Accessibility** - Easy for researchers to understand and modify

## 🦀 Why Rust for v1.0?

Rust was chosen for v1.0 because:

1. **Performance** - Near-native speed for compute-intensive workloads
2. **Safety** - Memory safety guarantees critical for secure execution
3. **WASM support** - First-class support via WasmEdge/Wasmer
4. **Concurrency** - Excellent async runtime (Tokio) for distributed systems
5. **Production-ready** - Better suited for high-throughput, secure deployments

## 📚 Learn More

See the white papers in `/docs/whitepapers/`:
- **VCP.md** - Detailed technical specification
- **AMBIENT_AI.md** - Vision and architecture

## 📄 License

Non-Commercial License - see parent LICENSE file
