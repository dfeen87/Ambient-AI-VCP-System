# 📋 Ambient AI VCP System - Quick Reference Card

*One-page guide to nodes, tasks, and the system*

---

## 🔄 THE TWO-SIDED MARKETPLACE

```
┌──────────────────┐                      ┌──────────────────┐
│  TASK SUBMITTERS │  ───Submit work───>  │  NODE OPERATORS  │
│  (Demand Side)   │                      │  (Supply Side)   │
│                  │  <───Return results─ │                  │
└──────────────────┘                      └──────────────────┘
 • App developers                          • Home users with PCs
 • Data scientists                         • Universities
 • Researchers                             • Data centers
 • Businesses                              • Anyone with compute
 • Need computing                          • Provide computing
```

**See full guide:** [docs/WHO_CREATES_TASKS.md](./docs/WHO_CREATES_TASKS.md)

---

## 🖥️ NODE TYPES

| Type | Symbol | Purpose | Best For | Hardware |
|------|--------|---------|----------|----------|
| **Compute** | 🧮 | Execute AI workloads | Running models, training | High CPU/GPU |
| **Gateway** | 🌐 | Route traffic | Load balancing, coordination | High bandwidth |
| **Storage** | 💾 | Store data | Datasets, results, models | Large storage |
| **Validator** | ✅ | Verify proofs | Security, integrity | Reliable uptime |

### When to Use Each Type:

```
┌─────────────────────────────────────────────────────────┐
│  Got a gaming PC with GPU?        → Register: COMPUTE  │
│  Got fiber internet & public IP?  → Register: GATEWAY  │
│  Got terabytes of storage?        → Register: STORAGE  │
│  Got a reliable server 24/7?      → Register: VALIDATOR│
└─────────────────────────────────────────────────────────┘
```

---

## ⚙️ TASK TYPES

| Type | Purpose | Use Case | Nodes Required |
|------|---------|----------|----------------|
| **federated_learning** | Train ML models privately | Multi-hospital diagnostics | 2+ compute |
| **zk_proof** | Verify computation | Privacy-preserving auth | 1+ compute |
| **wasm_execution** | Run sandboxed code | Portable algorithms | 1+ compute |
| **computation** | General processing | Simulations, analytics | 1+ compute |

---

## 🚀 QUICK START

### 1️⃣ Register a Node (Web Dashboard)

1. Go to: **https://ambient-ai-vcp-system.onrender.com**
2. Scroll to **"Register New Node"**
3. Fill form:
   - Node ID: `my-node-01`
   - Region: `us-west`
   - Type: `compute` (or gateway/storage/validator)
   - Bandwidth: `100` Mbps
   - CPU: `4` cores
   - Memory: `8` GB
4. Click **"Register Node"**

### 2️⃣ Submit a Task (API)

```bash
curl -X POST https://ambient-ai-vcp-system.onrender.com/api/v1/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "task_type": "computation",
    "inputs": {"data": "test"},
    "requirements": {
      "min_nodes": 1,
      "max_execution_time_sec": 60,
      "require_gpu": false,
      "require_proof": false
    }
  }'
```

### 3️⃣ View Results

- Dashboard: https://ambient-ai-vcp-system.onrender.com
- API Docs: https://ambient-ai-vcp-system.onrender.com/swagger-ui

---

## 📊 VALIDATION RULES

### Node Registration:
- ✅ node_id: 1-64 chars, alphanumeric + `-` `_`
- ✅ node_type: `compute` | `gateway` | `storage` | `validator`
- ✅ bandwidth: 0-100,000 Mbps
- ✅ cpu_cores: 1-1024
- ✅ memory_gb: 0.1-10,000

### Task Submission:
- ✅ task_type: `federated_learning` | `zk_proof` | `wasm_execution` | `computation`
- ✅ min_nodes: 1-1000
- ✅ max_execution_time_sec: 1-3600
- ✅ wasm_module: max 10MB (base64)

---

## 🎯 HEALTH SCORE FORMULA

```
Health Score = (bandwidth × 40%) + (latency × 30%) + (compute × 20%) + (reputation × 10%)
```

**Circuit Breakers:**
- 🌡️ Temperature > 85°C → Safe mode
- ⏱️ Latency > 100ms → Reduced capacity
- ⚠️ Error count > 25 → Temporary suspension

---

## 🔗 ESSENTIAL URLS

| Resource | URL |
|----------|-----|
| **Live Dashboard** | https://ambient-ai-vcp-system.onrender.com |
| **API Docs** | https://ambient-ai-vcp-system.onrender.com/swagger-ui |
| **GitHub** | https://github.com/dfeen87/Ambient-AI-VCP-System |
| **Full Guide** | [NODES_AND_TASKS_GUIDE.md](./NODES_AND_TASKS_GUIDE.md) |

---

## 📝 API ENDPOINTS

```bash
# Health check
GET /api/v1/health

# Nodes
POST /api/v1/nodes              # Register node
GET  /api/v1/nodes              # List all nodes
GET  /api/v1/nodes/{id}         # Get specific node

# Tasks
POST /api/v1/tasks              # Submit task
GET  /api/v1/tasks              # List all tasks
GET  /api/v1/tasks/{id}         # Get specific task

# Proofs
POST /api/v1/proofs/verify      # Verify ZK proof

# Stats
GET  /api/v1/cluster/stats      # Cluster statistics
```

---

## 💡 EXAMPLES

### Example 1: Home User with Gaming PC

```json
{
  "node_id": "my-gaming-rig",
  "region": "us-west",
  "node_type": "compute",
  "capabilities": {
    "bandwidth_mbps": 300,
    "cpu_cores": 8,
    "memory_gb": 32,
    "gpu_available": true
  }
}
```

### Example 2: Data Center Server

```json
{
  "node_id": "datacenter-01",
  "region": "us-east",
  "node_type": "compute",
  "capabilities": {
    "bandwidth_mbps": 10000,
    "cpu_cores": 64,
    "memory_gb": 256,
    "gpu_available": true
  }
}
```

### Example 3: Edge Gateway

```json
{
  "node_id": "edge-gateway",
  "region": "ap-southeast",
  "node_type": "gateway",
  "capabilities": {
    "bandwidth_mbps": 1000,
    "cpu_cores": 4,
    "memory_gb": 16,
    "gpu_available": false
  }
}
```

---

## 🆘 TROUBLESHOOTING

| Issue | Solution |
|-------|----------|
| "node_type must be one of..." | Use: `compute`, `gateway`, `storage`, `validator`, or `any` |
| "node_id cannot exceed 64 chars" | Shorten your node ID |
| "bandwidth_mbps must be between 0-100,000" | Check your bandwidth value |
| Can't connect to API | Verify URL: https://ambient-ai-vcp-system.onrender.com |
| Dashboard not loading | Check browser console, try different browser |

---

## 📚 LEARN MORE

- 📖 **Full Documentation**: [docs/NODES_AND_TASKS_GUIDE.md](./NODES_AND_TASKS_GUIDE.md)
- 🚀 **Getting Started**: [GETTING_STARTED.md](./GETTING_STARTED.md)
- 🏗️ **Architecture**: [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md)
- 🤝 **Contributing**: [docs/CONTRIBUTING.md](./docs/CONTRIBUTING.md)

---

## 📞 GET HELP

- 🐛 Issues: https://github.com/dfeen87/Ambient-AI-VCP-System/issues
- 💬 Discussions: https://github.com/dfeen87/Ambient-AI-VCP-System/discussions
- 📧 API Support: See Swagger docs

---

**Version**: 1.0.0 | **License**: Non-Commercial | **Tests**: 48 Passing ✅

*Print this card for quick reference!*
