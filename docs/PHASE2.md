# Phase 2 Quick Reference

## New Features

### 1. REST API Server
- **Package**: `api-server`
- **Binary**: `cargo run --bin api-server`
- **Port**: 3000 (configurable via `PORT` env var)
- **Endpoints**:
  - `GET /api/v1/health` - Health check
  - `POST /api/v1/nodes` - Register node
  - `GET /api/v1/nodes` - List nodes
  - `GET /api/v1/nodes/{id}` - Get node
  - `POST /api/v1/tasks` - Submit task
  - `GET /api/v1/tasks` - List tasks
  - `GET /api/v1/tasks/{id}` - Get task
  - `POST /api/v1/proofs/verify` - Verify proof
  - `GET /api/v1/cluster/stats` - Cluster stats
  - `GET /swagger-ui` - Swagger documentation

### 2. Federated Learning
- **Package**: `federated-learning`
- **Features**:
  - FedAvg aggregation algorithm
  - Differential privacy (Gaussian/Laplacian noise)
  - Gradient clipping
  - Privacy budgets (ε, δ)
  - Client model aggregation

**Example Usage**:
```rust
use federated_learning::{FederatedAggregator, ModelWeights, PrivacyMechanism, PrivacyBudget};

// Create aggregator
let initial_model = ModelWeights::new();
let mut aggregator = FederatedAggregator::new(initial_model);

// Add client updates
aggregator.add_client_update("client1".to_string(), model1, 100)?;
aggregator.add_client_update("client2".to_string(), model2, 200)?;

// Aggregate with privacy
let global_model = aggregator.aggregate()?;

// Apply differential privacy
let privacy = PrivacyMechanism::new(PrivacyBudget::standard());
let mut gradients = vec![1.0, 2.0, 3.0];
privacy.add_dp_noise_to_gradients(&mut gradients, 1.0);
```

### 3. Web Dashboard
- **Location**: `dashboard/index.html`
- **Features**:
  - Real-time node monitoring
  - Task management
  - Health metrics visualization
  - Interactive node registration
  - Auto-refresh every 5 seconds

**Access**: Open `dashboard/index.html` in browser, configure API URL

### 5. Multi-Node Demo
- **Location**: `demo/run-demo.sh`
- **Requirements**: curl, jq
- **Features**:
  - Automatic API server startup
  - Register 3 nodes
  - Submit FL and ZK tasks
  - Verify proofs
  - Display statistics

**Run**: `./demo/run-demo.sh`

## Deployment

### Local Development
```bash
# Start API server
cargo run --bin api-server

# In another terminal, run demo
./demo/run-demo.sh
```

### Docker
```bash
# Build image
docker build -t ambient-vcp:latest .

# Run container
docker run -p 3000:3000 ambient-vcp:latest
```

### Render.com
1. Push to GitHub
2. Connect repository to Render
3. Render detects `render.yaml`
4. Click "Apply" to deploy

### Kubernetes
```bash
# Build and push
docker build -t registry/ambient-vcp:latest .
docker push registry/ambient-vcp:latest

# Deploy
kubectl apply -f k8s/deployment.yaml
kubectl apply -f k8s/service.yaml
```

## API Examples

### Register Node
```bash
curl -X POST http://localhost:3000/api/v1/nodes \
  -H "Content-Type: application/json" \
  -d '{
    "node_id": "node-001",
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

### Submit FL Task
```bash
curl -X POST http://localhost:3000/api/v1/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "task_type": "federated_learning",
    "inputs": {
      "model_type": "neural_network",
      "rounds": 10,
      "aggregation": "fedavg"
    },
    "requirements": {
      "min_nodes": 3,
      "max_execution_time_sec": 300,
      "require_gpu": false,
      "require_proof": true
    }
  }'
```

### Verify Proof
```bash
curl -X POST http://localhost:3000/api/v1/proofs/verify \
  -H "Content-Type: application/json" \
  -d '{
    "task_id": "task-id-here",
    "proof_data": "base64-proof",
    "public_inputs": "base64-inputs"
  }'
```

### Get Stats
```bash
curl http://localhost:3000/api/v1/cluster/stats
```

## Testing

### Unit Tests
```bash
# All tests
cargo test --workspace

# Specific package
cargo test -p federated-learning
cargo test -p api-server
```

### Integration Test
```bash
# Run the demo
./demo/run-demo.sh

# Should output:
# ✓ All nodes registered successfully
# ✓ Federated Learning task submitted
# ✓ ZK Proof task submitted
# ✓ Proof verification complete
# ✓ Demo completed successfully!
```

## Architecture

```
┌─────────────┐
│  Dashboard  │ (HTML/JS)
└──────┬──────┘
       │ HTTP
┌──────▼──────┐
│  API Server │ (Axum)
│  Port 3000  │
└──────┬──────┘
       │
   ┌───┴───┐
   │       │
┌──▼──┐ ┌──▼────┐
│Node │ │Coord  │
└──┬──┘ └───┬───┘
   │        │
   └────┬───┘
        │
   ┌────┼────┐
   │    │    │
┌──▼┐ ┌─▼┐ ┌─▼────┐
│FL │ │ZK│ │WASM  │
└───┘ └──┘ └──────┘
```

## Privacy Budgets

| Level | Epsilon (ε) | Delta (δ) | Use Case |
|-------|-------------|-----------|----------|
| Conservative | 0.1 | 1e-5 | High privacy |
| Standard | 1.0 | 1e-5 | Balanced |
| Relaxed | 10.0 | 1e-4 | Low privacy |

## Performance Targets (Phase 2)

- API Latency: < 100ms (p95)
- FL Aggregation: < 5s for 10 nodes
- Proof Verification: < 1s
- Dashboard Refresh: 5s intervals
- Concurrent Tasks: 100+
- Node Capacity: 1,000+ per cluster

## Known Limitations

1. **ZK Proofs**: Currently using placeholder implementation
   - Real RISC Zero integration pending
   - Proof generation is simulated

2. **Persistence**: In-memory storage only
   - Node registrations lost on restart
   - Consider adding database in production

3. **Authentication**: No auth implemented
   - Add API keys or JWT for production

4. **P2P Networking**: Not yet implemented
   - Nodes communicate via API server
   - Phase 3 will add libp2p

## Troubleshooting

### API Server Won't Start
```bash
# Check port availability
lsof -i :3000

# Try different port
PORT=8080 cargo run --bin api-server
```

### Demo Script Fails
```bash
# Install jq
sudo apt-get install jq  # Debian/Ubuntu
brew install jq          # macOS

# Check API server
curl http://localhost:3000/api/v1/health
```

### Dashboard Shows No Data
1. Check API URL in dashboard
2. Verify API server is running
3. Check browser console for errors
4. Ensure CORS is enabled (default)

## Next Steps

1. **Production Deployment**: Use Render.com or Kubernetes
2. **Add Persistence**: Integrate PostgreSQL/Redis
3. **Real ZK Proofs**: Complete RISC Zero integration
4. **Authentication**: Add JWT or API keys
5. **Monitoring**: Add Prometheus/Grafana
6. **P2P Layer**: Implement libp2p networking

## Resources

- [API Documentation](http://localhost:3000/swagger-ui)
- [Demo Guide](demo/README.md)
- [Deployment Guide](docs/DEPLOYMENT.md)
- [Main README](README.md)
