# Understanding Nodes and Tasks in Ambient AI VCP System

## 📋 Table of Contents
- [The Two-Sided Marketplace](#the-two-sided-marketplace)
- [What is a Node?](#what-is-a-node)
- [Node Types Explained](#node-types-explained)
- [What is a Task?](#what-is-a-task)
- [Who Creates Tasks?](#who-creates-tasks)
- [Task Types Explained](#task-types-explained)
- [How to Register a Node](#how-to-register-a-node)
- [How to Submit a Task](#how-to-submit-a-task)
- [Complete Examples](#complete-examples)

---

## 🔄 The Two-Sided Marketplace

Ambient AI VCP System is a **two-sided marketplace for computation**:

### Supply Side: **Node Operators** (YOU!)
- People who have computing power to share
- Run nodes on laptops, servers, GPUs, edge devices
- Contribute idle compute capacity
- Earn reputation for completing tasks
- Examples: Home users, universities, data centers

### Demand Side: **Task Submitters**
- People who need computing power
- Submit computational work via API
- Developers, data scientists, researchers, businesses
- Pay for usage (future: token-based economy)
- Examples: App developers, ML engineers, research labs

### The System Connects Both:
```
Task Submitters → (Submit work) → Ambient AI VCP → (Distribute) → Node Operators
       ↓                                                                ↓
Need computation                                              Have computing power
       ↓                                                                ↓
       └────────────← (Results returned) ←──────────────────────────────┘
```

**👉 For a deep dive on who creates tasks and why, see:** [WHO_CREATES_TASKS.md](./WHO_CREATES_TASKS.md)

---

## 🖥️ What is a Node?

A **node** is any device (computer, server, edge device, etc.) that joins the Ambient AI VCP network to contribute computing power. Think of it like a worker in a distributed factory—each node has specific capabilities and can perform different types of work.

### Key Characteristics of a Node:

- **Unique Identity**: Each node has a unique ID (e.g., `gpu-server-001`, `laptop-42`)
- **Geographic Location**: Nodes specify their region (e.g., `us-west`, `eu-central`)
- **Type/Role**: Nodes have specific roles in the network (compute, gateway, storage, validator, any)
- **Capabilities**: Defined by hardware specs (CPU, memory, bandwidth, GPU availability)
- **Health Score**: A dynamic score (0-100) based on performance and reliability

---

## 🔧 Node Types Explained

The system supports **four types of nodes**, each with a specific role in the network:

### 1. **Compute Node** 🧮
**Purpose**: Execute AI workloads, run computations, and process tasks

**Ideal for:**
- Running WASM modules
- Executing machine learning models
- Performing federated learning
- General computational tasks

**Typical Hardware:**
- Desktop computers
- Servers with GPUs
- High-performance workstations
- Cloud instances (AWS EC2, GCP, Azure VMs)

**Example Use Cases:**
- Training neural networks
- Running inference
- Processing large datasets
- Cryptographic computations

**When to choose Compute:**
> If your machine has good CPU/GPU and you want to contribute computing power for AI tasks, register as a **Compute** node.

---

### 2. **Gateway Node** 🌐
**Purpose**: Route tasks, coordinate communication, and serve as entry points to the network

**Ideal for:**
- Task distribution and routing
- Load balancing across compute nodes
- Network orchestration
- API endpoints for external clients

**Typical Hardware:**
- Servers with high bandwidth
- Cloud instances with public IPs
- Edge routers
- Dedicated network appliances

**Example Use Cases:**
- Receiving tasks from clients
- Distributing work to compute nodes
- Aggregating results
- Managing network topology

**When to choose Gateway:**
> If your machine has excellent network connectivity and you want to help route and distribute work across the network, register as a **Gateway** node.

---

### 3. **Storage Node** 💾
**Purpose**: Store datasets, models, intermediate results, and task outputs

**Ideal for:**
- Distributed file storage
- Dataset hosting
- Model checkpointing
- Result caching

**Typical Hardware:**
- Servers with large storage capacity
- NAS (Network Attached Storage) devices
- Cloud storage instances (S3-backed, etc.)
- High-capacity HDDs/SSDs

**Example Use Cases:**
- Storing training datasets for federated learning
- Caching WASM modules
- Persisting model weights
- Archiving computation results

**When to choose Storage:**
> If your machine has large storage capacity and reliable uptime, register as a **Storage** node to help store datasets and results.

---

### 4. **Validator Node** ✅
**Purpose**: Verify zero-knowledge proofs, validate computation correctness, and ensure integrity

**Ideal for:**
- Cryptographic proof verification
- Computation validation
- Consensus participation
- Trust establishment

**Typical Hardware:**
- Any reliable machine (verification is lightweight)
- Nodes with good uptime
- Trusted infrastructure
- Secure environments

**Example Use Cases:**
- Verifying ZK proofs from compute nodes
- Checking computation correctness
- Detecting Byzantine/malicious nodes
- Maintaining network integrity

**When to choose Validator:**
> If you want to help ensure network security and integrity by verifying proofs and validating computations, register as a **Validator** node.

---

## 📊 Node Type Comparison

| Feature | Compute | Gateway | Storage | Validator |
|---------|---------|---------|---------|-----------|
| **Primary Role** | Execute tasks | Route traffic | Store data | Verify proofs |
| **CPU Requirement** | High | Medium | Low | Low |
| **GPU Requirement** | Optional | Not needed | Not needed | Not needed |
| **Bandwidth Requirement** | Medium | **High** | Medium | Low |
| **Storage Requirement** | Low | Low | **High** | Low |
| **Trust Requirement** | Medium | Medium | Medium | **High** |
| **Typical Reward** | High | Medium | Medium | Low-Medium |

---

## ⚙️ What is a Task?

A **task** is a unit of work submitted to the network for execution. Tasks are distributed to appropriate nodes based on their requirements and node capabilities.

### Task Lifecycle:

1. **Submission**: Client submits task to API
2. **Validation**: System validates task parameters
3. **Scheduling**: Mesh coordinator assigns task to suitable nodes
4. **Execution**: Selected nodes execute the task
5. **Verification**: (Optional) Validators verify execution proofs
6. **Completion**: Results are returned to the client

---

## 👥 Who Creates Tasks?

Tasks are created by **anyone who needs computational work done**:

- 💻 **Application Developers** - Building AI-powered apps
- 🧪 **Data Scientists** - Training models, running experiments
- 🔬 **Researchers** - Large-scale simulations, data processing
- 🏢 **Businesses** - Fraud detection, risk analysis, ML pipelines
- 🎨 **Individual Users** - Personal projects, learning, experimentation

### Why Submit Tasks?
Instead of buying and maintaining expensive servers:
- ✅ Pay only for computation you use
- ✅ Zero infrastructure maintenance
- ✅ Access to distributed compute power
- ✅ Privacy-preserving options (federated learning)
- ✅ Global scalability

**👉 See complete guide:** [WHO_CREATES_TASKS.md](./WHO_CREATES_TASKS.md) - Deep dive into task submitters, use cases, and the complete ecosystem.

---

## 🎯 Task Types Explained

The system supports **four types of tasks**:

### 1. **Federated Learning** (`federated_learning`) 🤖

**What it does**: Trains machine learning models across multiple nodes without sharing raw data

**How it works:**
- Each node trains on local data
- Nodes share model updates (not data)
- Coordinator aggregates updates using FedAvg
- Privacy preserved via differential privacy

**Example Use Case:**
```
Hospital A, B, and C want to train a diagnostic model without sharing 
patient data. Each hospital's node trains locally, then shares only 
encrypted model gradients for aggregation.
```

**Requirements:**
- Multiple compute nodes (min_nodes ≥ 2)
- Dataset on each node
- Privacy budget (epsilon, delta)

**Best For:**
- Healthcare: Multi-hospital model training
- Finance: Fraud detection across banks
- Mobile: Keyboard prediction without uploading keystrokes

---

### 2. **ZK Proof** (`zk_proof`) 🔐

**What it does**: Generates zero-knowledge proofs for computations

**How it works:**
- Node executes a computation
- Generates cryptographic proof (Groth16)
- Proof demonstrates correct execution without revealing inputs
- Validators verify proof

**Example Use Case:**
```
Prove you executed a complex calculation correctly without revealing 
the input data or intermediate steps. Useful for privacy-preserving 
verification.
```

**Requirements:**
- Compute node with ZK prover
- Computation circuit definition
- May require GPU for faster proving

**Best For:**
- Privacy-preserving authentication
- Confidential transactions
- Verifiable outsourced computation

---

### 3. **WASM Execution** (`wasm_execution`) 📦

**What it does**: Executes WebAssembly modules in a sandboxed environment

**How it works:**
- Upload WASM binary (base64 encoded)
- Node loads WASM in sandboxed runtime
- Execute with resource limits (memory, time, gas)
- Return results securely

**Example Use Case:**
```
Run a custom image processing algorithm, data transformation function, 
or any portable computation across different nodes' architectures.
```

**Requirements:**
- WASM module (max 10MB)
- Input data
- Resource specifications

**Best For:**
- Portable custom algorithms
- Secure sandboxed execution
- Cross-platform computation
- Untrusted code execution

---

### 4. **General Computation** (`computation`) ⚡

**What it does**: Executes general-purpose computational tasks

**How it works:**
- Submit task with input parameters
- Node performs computation
- Returns results
- Optional proof generation

**Example Use Case:**
```
Mathematical simulations, data processing, optimization problems, 
or any computation that doesn't fit specific categories above.
```

**Requirements:**
- Compute node
- Input data
- Computation specification

**Best For:**
- Scientific simulations
- Data analytics
- Batch processing
- Custom workloads

---

## 🚀 How to Register a Node

### Via Web Dashboard (Easiest)

1. **Open the Dashboard**
   - Go to: https://ambient-ai-vcp-system.onrender.com
   - Or run locally: http://localhost:3000

2. **Scroll to "Register New Node" section** at the bottom

3. **Fill in the form:**
   - **Node ID**: Unique identifier (e.g., `my-laptop-01`)
   - **Region**: Select your geographic region
   - **Node Type**: Choose based on your hardware:
     - `Compute` - For CPU/GPU-heavy machines
     - `Gateway` - For high-bandwidth network nodes
     - `Storage` - For high-capacity storage
     - `Validator` - For trusted verification nodes
   - **Bandwidth (Mbps)**: Your internet speed
   - **CPU Cores**: Number of CPU cores
   - **Memory (GB)**: Available RAM

4. **Click "Register Node"**

5. **Confirmation**: You'll see a success message and the node will appear in the "Registered Nodes" table

---

### Via API (Advanced)

**Endpoint:** `POST /api/v1/nodes`

**Example:**
```bash
curl -X POST https://ambient-ai-vcp-system.onrender.com/api/v1/nodes \
  -H "Content-Type: application/json" \
  -d '{
    "node_id": "gpu-server-001",
    "region": "us-west",
    "node_type": "compute",
    "capabilities": {
      "bandwidth_mbps": 1000,
      "cpu_cores": 16,
      "memory_gb": 64,
      "gpu_available": true
    }
  }'
```

**Response:**
```json
{
  "node_id": "gpu-server-001",
  "region": "us-west",
  "node_type": "compute",
  "health_score": 85.5,
  "status": "online",
  "registered_at": "2024-01-15T10:30:00Z"
}
```

---

### Via CLI (Command Line)

```bash
# Build the CLI
cargo build --release --bin ambient-vcp-cli

# Run a compute node
./target/release/ambient-vcp-cli node \
  --id gpu-node-001 \
  --region us-west \
  --node-type compute
```

---

## 📤 How to Submit a Task

### Via API

**Endpoint:** `POST /api/v1/tasks`

**Example - Federated Learning Task:**
```bash
curl -X POST https://ambient-ai-vcp-system.onrender.com/api/v1/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "task_type": "federated_learning",
    "inputs": {
      "model_name": "fraud_detection",
      "rounds": 5,
      "epsilon": 1.0
    },
    "requirements": {
      "min_nodes": 3,
      "max_execution_time_sec": 600,
      "require_gpu": false,
      "require_proof": true
    }
  }'
```

**Example - WASM Execution Task:**
```bash
curl -X POST https://ambient-ai-vcp-system.onrender.com/api/v1/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "task_type": "wasm_execution",
    "wasm_module": "<base64-encoded-wasm-binary>",
    "inputs": {
      "arg1": "value1",
      "arg2": 42
    },
    "requirements": {
      "min_nodes": 1,
      "max_execution_time_sec": 30,
      "require_gpu": false,
      "require_proof": true
    }
  }'
```

---

## 📝 Complete Examples

### Example 1: Research Lab with GPU Cluster

**Scenario**: University research lab with 10 GPU servers

**Registration:**
```bash
# Register each server as a compute node
for i in {1..10}; do
  curl -X POST https://ambient-ai-vcp-system.onrender.com/api/v1/nodes \
    -H "Content-Type: application/json" \
    -d "{
      \"node_id\": \"gpu-server-$(printf %03d $i)\",
      \"region\": \"us-east\",
      \"node_type\": \"compute\",
      \"capabilities\": {
        \"bandwidth_mbps\": 10000,
        \"cpu_cores\": 32,
        \"memory_gb\": 128,
        \"gpu_available\": true
      }
    }"
done
```

---

### Example 2: Edge Device Network

**Scenario**: IoT gateway coordinating 100 edge sensors

**Registration:**
```bash
# Register gateway node
curl -X POST https://ambient-ai-vcp-system.onrender.com/api/v1/nodes \
  -H "Content-Type: application/json" \
  -d '{
    "node_id": "edge-gateway-main",
    "region": "us-west",
    "node_type": "gateway",
    "capabilities": {
      "bandwidth_mbps": 1000,
      "cpu_cores": 8,
      "memory_gb": 16,
      "gpu_available": false
    }
  }'
```

---

### Example 3: Home User Contributing

**Scenario**: Individual with a gaming PC wants to contribute idle compute

**Steps:**
1. Open dashboard: https://ambient-ai-vcp-system.onrender.com
2. Fill registration form:
   - Node ID: `my-gaming-pc`
   - Region: `us-west`
   - Node Type: `compute`
   - Bandwidth: `300` Mbps
   - CPU Cores: `8`
   - Memory: `32` GB
3. Click "Register Node"
4. Your PC is now part of the network!

---

## 🤔 FAQ

### Q: Can I register multiple node types from the same machine?
**A:** No. Each physical machine should register as one node with the most appropriate type. However, you can run multiple nodes if you have multiple machines.

### Q: What happens if my node goes offline?
**A:** Your node's health score will decrease, and it will stop receiving new tasks. When it comes back online, it can re-register or resume participation.

### Q: Do I need all four node types to run the system?
**A:** No. The system works with any combination of node types. A single compute node is enough to start processing tasks.

### Q: Can I change my node type after registration?
**A:** Currently, you need to re-register with a new node ID. Future updates will support type changes.

### Q: What's the minimum hardware required?
**A:** 
- CPU: 2+ cores
- Memory: 2+ GB RAM
- Bandwidth: 10+ Mbps
- Storage: Varies by node type

### Q: Why is there a "Register New Node" section at the top of the dashboard?
**A:** This allows you to easily add your machine to the network. It's the primary way users contribute computing resources to the decentralized system. Think of it as "joining the cluster" or "volunteering your computer's idle time."

### Q: What are tasks used for?
**A:** Tasks are the actual work executed by the network. When someone needs computation done (train a model, run a simulation, process data), they submit a task. The system then routes that task to appropriate nodes based on their capabilities and the task requirements.

---

## 🎓 Best Practices

### For Node Operators:

1. **Choose the Right Type**: Match your hardware to the node type
2. **Accurate Specs**: Report honest capabilities for better task matching
3. **Stable Connectivity**: Maintain good uptime for higher health scores
4. **Monitor Performance**: Check the dashboard regularly
5. **Start Small**: Begin with one node before scaling up

### For Task Submitters:

1. **Right Task Type**: Choose the correct task type for your workload
2. **Realistic Requirements**: Set appropriate min_nodes and execution time
3. **Proof When Needed**: Only require proofs for security-critical tasks
4. **Optimize WASM**: Keep WASM modules small and efficient
5. **Test Locally**: Validate your task works before submitting to the network

---

## 📚 Related Documentation

- [API Reference](./API_REFERENCE.md) - Complete API documentation
- [Architecture](./ARCHITECTURE.md) - System architecture deep dive
- [Getting Started](./GETTING_STARTED.md) - Quick start guide
- [Deployment](./DEPLOYMENT.md) - Production deployment guide

---

## 🆘 Need Help?

- **Documentation**: See `/docs` directory
- **Issues**: [GitHub Issues](https://github.com/dfeen87/Ambient-AI-VCP-System/issues)
- **Discussions**: [GitHub Discussions](https://github.com/dfeen87/Ambient-AI-VCP-System/discussions)
- **API Docs**: https://ambient-ai-vcp-system.onrender.com/swagger-ui

---

**Last Updated**: February 2024  
**Version**: 1.0.0
