# Multi-Node Demo Application

This demo showcases the Phase 2 features of the Ambient AI VCP System:

## Features Demonstrated

1. **Multi-Node Registration**: Register compute nodes across different regions
2. **Federated Learning**: Submit FL tasks with differential privacy
3. **ZK Proof Generation**: Submit and verify zero-knowledge proofs
4. **Verifiable Computation**: Cryptographic proof verification
5. **REST API**: Interact with the system via HTTP endpoints
6. **Web Dashboard**: Monitor cluster status in real-time

## Prerequisites

- Rust 1.75+
- curl
- jq (for JSON processing)
- A web browser (for dashboard)

## Quick Start

### 1. Start the API Server

```bash
# From the project root
cargo run --bin api-server
```

The API server will start on `http://localhost:3000`

### 2. Run the Demo Script

In a new terminal:

```bash
./demo/run-demo.sh
```

This will:
- Check if the API server is running (start it if not)
- Register 3 compute nodes in different regions
- Submit a federated learning task
- Submit a ZK proof task
- Verify the proof
- Display cluster statistics

### 3. Open the Dashboard

Open `dashboard/index.html` in your browser to see:
- Real-time node status
- Task monitoring
- Cluster health metrics
- Node registration form

## Manual Testing

### Register a Node

```bash
curl -X POST http://localhost:3000/api/v1/nodes \
  -H "Content-Type: application/json" \
  -d '{
    "node_id": "my-node",
    "region": "us-west",
    "node_type": "compute",
    "capabilities": {
      "bandwidth_mbps": 100.0,
      "cpu_cores": 4,
      "memory_gb": 8.0,
      "gpu_available": false
    }
  }'
```

### Submit a Task

```bash
curl -X POST http://localhost:3000/api/v1/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "task_type": "federated_learning",
    "inputs": {
      "model_type": "neural_network",
      "rounds": 5
    },
    "requirements": {
      "min_nodes": 2,
      "max_execution_time_sec": 120,
      "require_gpu": false,
      "require_proof": true
    }
  }'
```

### Verify a Proof

```bash
curl -X POST http://localhost:3000/api/v1/proofs/verify \
  -H "Content-Type: application/json" \
  -d '{
    "task_id": "your-task-id",
    "proof_data": "base64-encoded-proof",
    "public_inputs": "base64-encoded-inputs"
  }'
```

### Get Cluster Stats

```bash
curl http://localhost:3000/api/v1/cluster/stats
```

## API Documentation

Once the server is running, visit:
- Swagger UI: http://localhost:3000/swagger-ui
- OpenAPI spec: http://localhost:3000/api-docs/openapi.json

## Architecture

```
┌─────────────────┐
│   Web Dashboard │
└────────┬────────┘
         │ HTTP
         ▼
┌─────────────────┐
│   REST API      │
│   (Port 3000)   │
└────────┬────────┘
         │
    ┌────┴────┐
    │         │
    ▼         ▼
┌────────┐ ┌──────────────┐
│ Nodes  │ │ Coordinators │
└────────┘ └──────────────┘
    │           │
    └─────┬─────┘
          │
    ┌─────┴──────┐
    │            │
    ▼            ▼
┌────────┐  ┌─────────┐
│ FL Agg │  │ ZK Prov │
└────────┘  └─────────┘
    │            │
    └─────┬──────┘
          │
          ▼
    ┌─────────────┐
    │ Verifier    │
    └─────────────┘
```

## Example Workflows

### Workflow 1: Federated Learning

1. Register 3+ nodes with GPU capabilities
2. Submit FL task with privacy budget (ε=1.0, δ=1e-5)
3. Nodes train model locally
4. Aggregator combines updates using FedAvg
5. Differential privacy applied to gradients
6. Final model verified and stored

### Workflow 2: ZK Proof Computation

1. Register compute node
2. Submit WASM computation task
3. Node executes in sandbox
4. Generate ZK proof of correct execution
5. Verify proof cryptographically
6. Store verified computation result

### Workflow 3: Multi-Region Coordination

1. Register nodes in us-west, us-east, eu-central
2. Coordinator assigns tasks based on latency
3. Monitor health scores via dashboard
4. Automatic failover on node degradation

## Troubleshooting

**API server won't start:**
- Check if port 3000 is already in use: `lsof -i :3000`
- Try a different port: `PORT=8080 cargo run --bin api-server`

**Demo script fails:**
- Ensure `jq` is installed: `sudo apt-get install jq`
- Check API server is running: `curl http://localhost:3000/api/v1/health`

**Dashboard shows no data:**
- Verify API URL in dashboard matches server address
- Check browser console for CORS errors
- Ensure API server has CORS enabled (it should by default)

## Next Steps

- Deploy to Render.com using `render.yaml`
- Scale to multiple coordinators
- Add production authentication
- Implement real ZK proof generation with RISC Zero
- Add P2P networking layer
