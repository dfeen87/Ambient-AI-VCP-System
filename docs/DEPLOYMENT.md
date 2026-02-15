# Deployment Guide

This guide covers deploying the Ambient AI + VCP System in various environments.

## Local Development

### Prerequisites

- Rust 1.75 or later
- (Optional) WasmEdge for WASM execution
- (Optional) Docker for containerized deployment

### Quick Start

```bash
# Clone the repository
git clone https://github.com/dfeen87/Ambient-AI-VCP-System.git
cd Ambient-AI-VCP-System

# Build the project
cargo build --release

# Run health check
cargo run --bin ambient-vcp -- health
```

### Running a Single Node

```bash
# Start a node
cargo run --bin ambient-vcp -- node \
    --id node-001 \
    --region us-west \
    --node-type compute
```

### Running a Coordinator

```bash
# Start a coordinator
cargo run --bin ambient-vcp -- coordinator \
    --cluster-id my-cluster \
    --strategy weighted
```

## Docker Deployment

### Single Container

```bash
# Build the Docker image
docker build -t ambient-vcp:latest .

# Run a node
docker run -it ambient-vcp:latest node \
    --id node-001 \
    --region us-west \
    --node-type compute
```

### Multi-Node with Docker Compose

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f

# Stop all services
docker-compose down
```

## Production Deployment

### System Requirements

#### Coordinator Node
- CPU: 4+ cores
- RAM: 8GB minimum
- Storage: 100GB SSD
- Network: 1Gbps+

#### Compute Node
- CPU: 2+ cores
- RAM: 4GB minimum
- Storage: 50GB SSD
- Network: 100Mbps+

### Security Considerations

1. **Network Security**
   - Use TLS/SSL for all communications
   - Implement firewall rules
   - Enable rate limiting

2. **Node Authentication**
   - Use cryptographic identities
   - Implement node registration process
   - Regular key rotation

3. **Resource Limits**
   - Enforce WASM sandbox limits
   - Monitor resource usage
   - Implement circuit breakers

### Monitoring

```bash
# Check node health
cargo run --bin ambient-vcp -- health

# View node info
cargo run --bin ambient-vcp -- info --id node-001
```

## Kubernetes Deployment

### Prerequisites
- Kubernetes cluster (1.20+)
- kubectl configured
- Docker registry access

### Create Kubernetes Manifests

#### Deployment

```yaml
# k8s/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ambient-vcp-api
  labels:
    app: ambient-vcp
spec:
  replicas: 3
  selector:
    matchLabels:
      app: ambient-vcp
  template:
    metadata:
      labels:
        app: ambient-vcp
    spec:
      containers:
      - name: api-server
        image: your-registry/ambient-vcp:latest
        ports:
        - containerPort: 3000
        env:
        - name: PORT
          value: "3000"
        - name: RUST_LOG
          value: "info"
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "1Gi"
            cpu: "1000m"
        livenessProbe:
          httpGet:
            path: /api/v1/health
            port: 3000
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /api/v1/health
            port: 3000
          initialDelaySeconds: 5
          periodSeconds: 5
```

#### Service

```yaml
# k8s/service.yaml
apiVersion: v1
kind: Service
metadata:
  name: ambient-vcp-service
spec:
  selector:
    app: ambient-vcp
  ports:
  - protocol: TCP
    port: 80
    targetPort: 3000
  type: LoadBalancer
```

### Deploy to Kubernetes

```bash
# Build and push Docker image
docker build -t your-registry/ambient-vcp:latest .
docker push your-registry/ambient-vcp:latest

# Apply manifests
kubectl apply -f k8s/deployment.yaml
kubectl apply -f k8s/service.yaml

# Check status
kubectl get pods
kubectl logs -f deployment/ambient-vcp-api
```

## Render.com Deployment

### Steps

1. Fork or clone this repository
2. Go to [Render.com](https://render.com) and click "New +" → "Blueprint"
3. Connect your GitHub repository
4. Render will automatically detect `render.yaml`
5. Click "Apply" to deploy

The API will be available at: `https://your-app-name.onrender.com`

Access points:
- Health: `/api/v1/health`
- Swagger UI: `/swagger-ui`
- API Docs: `/api-docs/openapi.json`

## Environment Variables

- `RUST_LOG`: Set logging level (debug, info, warn, error)
- `AMBIENT_NODE_ID`: Override node ID
- `AMBIENT_REGION`: Override region
- `AMBIENT_CLUSTER_ID`: Cluster identifier

## Troubleshooting

### Build Errors

If you encounter WASM-related build errors:
```bash
# Build without WASM runtime
cargo build --no-default-features
```

### Connection Issues

- Verify network connectivity
- Check firewall rules
- Ensure correct ports are open

### Performance Issues

- Monitor system resources
- Check health scores
- Review circuit breaker thresholds

## Upgrading

```bash
# Pull latest changes
git pull origin main

# Rebuild
cargo build --release

# Restart services
docker-compose restart
```

## Backup and Recovery

### Data to Backup

- Node configurations
- Reputation data
- Task history
- Proofs and traces

### Recovery Process

1. Stop services
2. Restore backup data
3. Restart services
4. Verify health

## Support

For deployment issues:
- Check the documentation
- Search existing issues
- Open a new issue with details
