# Global Node Deployment Guide

This guide explains how to deploy the Ambient AI VCP system as a **global online API** that anyone can connect to and use.

## 🌐 Overview

The Ambient AI VCP system can be deployed as a public service with:

- **Global API Server** - REST API for task submission and node management
- **Mesh Coordinator** - Orchestrates tasks across registered nodes
- **Compute Nodes** - Distributed workers across multiple regions
- **Web Dashboard** - Real-time monitoring interface

## 🚀 Quick Deploy with Docker Compose

The easiest way to run a global node is with Docker Compose:

```bash
# Start the entire global network
docker-compose up -d

# View logs
docker-compose logs -f api-server

# Check status
curl http://localhost:3000/api/v1/health
```

This starts:
- **API Server** on `http://localhost:3000`
- **Coordinator** managing the cluster
- **4 Compute Nodes** across different regions (US West, US East, EU, APAC)

### Access Points

- **API Endpoint**: `http://localhost:3000`
- **Swagger UI**: `http://localhost:3000/swagger-ui`
- **Health Check**: `http://localhost:3000/api/v1/health`
- **Web Dashboard**: Open `dashboard/index.html` and configure API URL

## ☁️ Deploy to Cloud (Render.com)

### Prerequisites

1. Create account at [render.com](https://render.com)
2. Fork this repository to your GitHub
3. Connect your GitHub to Render

### Deploy

1. **Via Render Dashboard**:
   - Click "New +" → "Blueprint"
   - Connect your repository
   - Render will auto-detect `render.yaml`
   - Click "Apply"

2. **Via CLI**:
   ```bash
   # Install Render CLI
   brew install render
   
   # Login
   render login
   
   # Deploy
   render blueprint apply
   ```

### Configuration

The `render.yaml` configures:
- Service name: `ambient-ai-vcp-api`
- Region: Oregon (can be changed)
- Port: 10000
- Health check: `/api/v1/health`

Your API will be available at:
```
https://ambient-ai-vcp-api.onrender.com
```

## 🐳 Deploy to Any Cloud with Docker

### Build and Run

```bash
# Build the Docker image
docker build -t ambient-vcp-api .

# Run the API server
docker run -d \
  -p 3000:3000 \
  -e RUST_LOG=info \
  -e API_HOST=0.0.0.0 \
  -e API_PORT=3000 \
  --name ambient-api \
  ambient-vcp-api
```

### Deploy to AWS ECS

```bash
# Tag for ECR
docker tag ambient-vcp-api:latest <your-account>.dkr.ecr.us-west-2.amazonaws.com/ambient-vcp-api:latest

# Push to ECR
docker push <your-account>.dkr.ecr.us-west-2.amazonaws.com/ambient-vcp-api:latest

# Create ECS task definition and service
aws ecs create-service \
  --cluster ambient-vcp-cluster \
  --service-name ambient-api \
  --task-definition ambient-vcp-api:1 \
  --desired-count 1 \
  --launch-type FARGATE
```

### Deploy to Google Cloud Run

```bash
# Tag for GCR
docker tag ambient-vcp-api gcr.io/<project-id>/ambient-vcp-api

# Push to GCR
docker push gcr.io/<project-id>/ambient-vcp-api

# Deploy to Cloud Run
gcloud run deploy ambient-vcp-api \
  --image gcr.io/<project-id>/ambient-vcp-api \
  --platform managed \
  --region us-central1 \
  --allow-unauthenticated
```

### Deploy to Azure Container Instances

```bash
# Tag for ACR
docker tag ambient-vcp-api <registry>.azurecr.io/ambient-vcp-api

# Push to ACR
docker push <registry>.azurecr.io/ambient-vcp-api

# Deploy to ACI
az container create \
  --resource-group ambient-vcp \
  --name ambient-api \
  --image <registry>.azurecr.io/ambient-vcp-api \
  --dns-name-label ambient-vcp-api \
  --ports 3000
```

## 🔧 Environment Variables

### API Server

| Variable | Default | Description |
|----------|---------|-------------|
| `API_HOST` | `127.0.0.1` | Host to bind to (use `0.0.0.0` for public) |
| `API_PORT` | `3000` | Port to listen on |
| `RUST_LOG` | `info` | Log level (`debug`, `info`, `warn`, `error`) |
| `CORS_ALLOWED_ORIGINS` | `*` | CORS origins (comma-separated) |
| `MAX_WORKERS` | `4` | Number of worker threads |

### Coordinator

| Variable | Default | Description |
|----------|---------|-------------|
| `CLUSTER_ID` | `default-cluster` | Cluster identifier |
| `STRATEGY` | `weighted` | Task assignment strategy |
| `MAX_NODES` | `10000` | Maximum nodes in cluster |

### Compute Nodes

| Variable | Default | Description |
|----------|---------|-------------|
| `NODE_ID` | Auto-generated | Unique node identifier |
| `REGION` | `us-west` | Geographic region |
| `NODE_TYPE` | `compute` | Node type (`gateway`, `compute`, `storage`) |
| `COORDINATOR_URL` | `http://localhost:8080` | Coordinator endpoint |

## 📡 API Endpoints

### Health & Status

```bash
# Health check
GET /api/v1/health

# Cluster statistics
GET /api/v1/stats
```

### Node Management

```bash
# Register a node
POST /api/v1/nodes
{
  "id": "node-001",
  "region": "us-west",
  "node_type": "compute",
  "telemetry": { ... }
}

# List nodes
GET /api/v1/nodes

# Get node details
GET /api/v1/nodes/{node_id}

# Deregister node
DELETE /api/v1/nodes/{node_id}
```

### Task Submission

```bash
# Submit compute task
POST /api/v1/tasks
{
  "task_type": "compute",
  "wasm_module": "<base64-encoded-wasm>",
  "inputs": { ... }
}

# Get task status
GET /api/v1/tasks/{task_id}

# List tasks
GET /api/v1/tasks
```

### Proof Verification

```bash
# Submit proof
POST /api/v1/proofs/verify
{
  "task_id": "...",
  "proof": { ... },
  "result": { ... }
}
```

## 🔒 Security Considerations

### For Public Deployments

1. **Enable Authentication**:
   ```bash
   # Use API keys or JWT tokens
   export API_KEY_SECRET="your-secret-key"
   ```

2. **Rate Limiting**:
   - Configure reverse proxy (nginx, Caddy)
   - Use cloud provider rate limiting

3. **HTTPS/TLS**:
   - Always use HTTPS in production
   - Render.com provides auto-SSL
   - Use Let's Encrypt for self-hosted

4. **CORS Configuration**:
   ```bash
   # Restrict origins in production
   export CORS_ALLOWED_ORIGINS="https://yourdomain.com,https://app.yourdomain.com"
   ```

5. **Resource Limits**:
   ```yaml
   # docker-compose.yml
   deploy:
     resources:
       limits:
         cpus: '2'
         memory: 4G
   ```

## 🌍 Global Node Architecture

### Recommended Setup

For a production global deployment:

```
┌─────────────────────────────────────────┐
│         Load Balancer (Global)          │
│         (Cloudflare, AWS ALB)           │
└──────────────┬──────────────────────────┘
               │
       ┌───────┴────────┐
       │                │
   ┌───▼────┐      ┌───▼────┐
   │ Region │      │ Region │
   │ US-West│      │ EU-West│
   └───┬────┘      └───┬────┘
       │                │
   ┌───▼──────────┐ ┌──▼────────────┐
   │ API Server   │ │ API Server    │
   │ + Coordinator│ │ + Coordinator │
   └───┬──────────┘ └──┬────────────┘
       │                │
   ┌───▼───┐        ┌──▼───┐
   │Nodes  │        │Nodes │
   │(x10)  │        │(x10) │
   └───────┘        └──────┘
```

### Multi-Region Deployment

1. **Deploy API servers in multiple regions**
2. **Use DNS-based load balancing** (GeoDNS)
3. **Sync coordinator state** (Redis, etcd)
4. **Distribute nodes** across regions

## 📊 Monitoring

### Prometheus Metrics

The API server exposes Prometheus metrics at `/metrics`:

```bash
# Scrape metrics
curl http://localhost:3000/metrics
```

Key metrics:
- `vcp_tasks_total` - Total tasks submitted
- `vcp_nodes_registered` - Active nodes
- `vcp_task_duration_seconds` - Task execution time
- `vcp_proof_verification_total` - Proofs verified

### Grafana Dashboard

Import the provided dashboard:
```bash
grafana-cli dashboards import docs/grafana-dashboard.json
```

## 🧪 Testing the Global API

### Register a Node

```bash
curl -X POST http://localhost:3000/api/v1/nodes \
  -H "Content-Type: application/json" \
  -d '{
    "id": "my-node-001",
    "region": "us-west",
    "node_type": "compute",
    "telemetry": {
      "bandwidth_mbps": 1000,
      "latency_ms": 10,
      "cpu_percent": 20,
      "memory_percent": 30
    }
  }'
```

### Submit a Task

```bash
curl -X POST http://localhost:3000/api/v1/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "task_type": "compute",
    "wasm_module": "AGFzbQEAAAA...",
    "inputs": {"x": 42}
  }'
```

### Check Cluster Health

```bash
curl http://localhost:3000/api/v1/stats | jq
```

## 🤝 Connecting to a Global Node

### From the CLI

```bash
# Configure global endpoint
export VCP_API_URL=https://ambient-vcp-api.onrender.com

# Register your node
cargo run --bin ambient-vcp -- node \
  --id my-local-node \
  --region us-west \
  --coordinator-url $VCP_API_URL
```

### From the Dashboard

1. Open `dashboard/index.html`
2. Configure API URL: `https://ambient-vcp-api.onrender.com`
3. View global cluster metrics

### From v0.3 JavaScript Clients

```javascript
// Update the API endpoint
const API_ENDPOINT = 'https://ambient-vcp-api.onrender.com';

// Submit task via REST instead of P2P
await fetch(`${API_ENDPOINT}/api/v1/tasks`, {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ task_type: 'compute', ... })
});
```

## 🆘 Troubleshooting

### API Server Won't Start

```bash
# Check logs
docker-compose logs api-server

# Common issues:
# 1. Port already in use
lsof -i :3000

# 2. Missing environment variables
docker-compose config

# 3. Build errors
docker-compose build --no-cache
```

### Nodes Not Registering

```bash
# Check coordinator logs
docker-compose logs coordinator

# Verify network connectivity
docker-compose exec node-1 ping coordinator

# Check API connectivity
curl http://localhost:3000/api/v1/health
```

### High Latency

- Use CDN for static assets
- Deploy API servers closer to users
- Enable HTTP/2 and compression
- Optimize Docker image size

## 📚 Additional Resources

- [API Reference](./API_REFERENCE.md)
- [Architecture Overview](./ARCHITECTURE.md)
- [White Papers](./whitepapers/)
- [Contributing Guide](./CONTRIBUTING.md)

---

**Need help?** Open an issue on [GitHub](https://github.com/dfeen87/Ambient-AI-VCP-System/issues)
