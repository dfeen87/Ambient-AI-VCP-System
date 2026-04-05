# Getting Started - 5 Minute Quick Start

## ✅ Prerequisites Checklist

Before you begin, ensure you have:
- [ ] **Rust 1.75+** installed ([Install Rust](https://rustup.rs/))
- [ ] **Git** installed
- [ ] **curl** and **jq** installed (for demo script)
- [ ] A terminal/command line interface

---

## 🚀 Quick Start (5 Minutes)

### Step 1: Clone the Repository (30 seconds)

```bash
git clone https://github.com/dfeen87/Ambient-AI-VCP-System.git
cd Ambient-AI-VCP-System
```

### Step 2: Build the Project (2-3 minutes)

```bash
cargo build --release
```

**Expected output:** 
```
   Compiling ambient-node v0.1.0
   Compiling wasm-engine v0.1.0
   ...
   Finished release [optimized] target(s) in 2m 15s
```

### Step 3: Run Tests (1 minute)

```bash
cargo test
```

**Expected output:**
```
running X tests
...
test result: ok. X passed; 0 failed; 0 ignored; 0 measured
```

### Step 4: Start the API Server (10 seconds)

```bash
cargo run --bin api-server
```

**Expected output:**
```
Starting Ambient AI VCP API Server...
Server running on http://0.0.0.0:3000
Swagger UI: http://localhost:3000/swagger-ui
```

### Step 5: Verify It's Working (30 seconds)

Open a new terminal and run:

```bash
curl http://localhost:3000/api/v1/health
```

**Expected output:**
```json
{
  "status": "healthy",
  "timestamp": "2026-02-15T21:35:00Z"
}
```

---

## 🎉 Success! What Now?

### Option A: Explore the Web Dashboard

1. Open your browser to [http://localhost:3000](http://localhost:3000)
2. You'll see the real-time monitoring interface

### Option B: Run the Demo Script

```bash
# In a new terminal (keep API server running)
./demo/run-demo.sh
```

This will:
- ✅ Register 3 compute nodes across different regions
- ✅ Submit a federated learning task
- ✅ Submit a ZK proof task
- ✅ Verify cryptographic proofs
- ✅ Display cluster statistics

### Option C: Explore the API with Swagger

1. Open your browser to: http://localhost:3000/swagger-ui
2. Try out the interactive API documentation
3. Test endpoints directly from your browser

### Option D: Try Manual API Calls

Register a compute node:
```bash
curl -X POST http://localhost:3000/api/v1/nodes \
  -H "Content-Type: application/json" \
  -d '{
    "node_id": "my-first-node",
    "region": "us-west",
    "node_type": "compute",
    "capabilities": {
      "bandwidth_mbps": 100.0,
      "cpu_cores": 8,
      "memory_gb": 16.0,
      "gpu_available": false
    }
  }'
```

List all nodes:
```bash
curl http://localhost:3000/api/v1/nodes | jq
```

---

## 📚 Next Steps: Choose Your Path

### For AI/ML Developers
👉 Read: [Federated Learning Guide](./docs/whitepapers/VCP.md#federated-learning)  
👉 Try: Modify the FL task in `demo/run-demo.sh`  
👉 Explore: `crates/federated-learning/`

### For Blockchain Developers
👉 Read: [ZK Proofs Documentation](./docs/ZK_PROOFS.md)  
👉 Try: Submit a ZK proof task via API  
👉 Explore: `crates/zk-prover/`

### For Backend Developers
👉 Read: [API Reference](./docs/API_REFERENCE.md)  
👉 Try: Add a new endpoint in `crates/api-server/`  
👉 Explore: Rust async patterns in the codebase

### For System Architects
👉 Read: [Architecture Overview](./docs/ARCHITECTURE.md)  
👉 Try: Deploy with Docker Compose  
👉 Explore: System design patterns

### For Students/Learners
👉 Read: [Implementation Summary](./docs/IMPLEMENTATION_SUMMARY.md)  
👉 Try: Run tests with `RUST_LOG=debug cargo test`  
👉 Explore: All the well-documented Rust code

---

## 🛠️ Common Next Actions

### Customize the System

1. **Change the API port:**
   ```bash
   PORT=8080 cargo run --bin api-server
   ```

2. **Run in development mode with hot reload:**
   ```bash
   cargo watch -x 'run --bin api-server'
   ```

3. **Run specific crate tests:**
   ```bash
   cargo test -p ambient-node
   cargo test -p zk-prover
   ```

### Deploy to Production

1. **Docker Compose:**
   ```bash
   docker-compose up -d
   ```

2. **Render.com (one-click):**
   ```bash
   render blueprint apply
   ```

3. **Manual deployment:**
   ```bash
   cargo build --release
   ./target/release/api-server
   ```

---

## 🐛 Troubleshooting

### API Server Won't Start

**Problem:** Port 3000 already in use  
**Solution:** 
```bash
lsof -i :3000  # Find what's using the port
kill <PID>     # Kill that process
# Or use a different port
PORT=8080 cargo run --bin api-server
```

### Build Fails

**Problem:** Rust version too old  
**Solution:**
```bash
rustup update
cargo clean
cargo build --release
```

**Problem:** Missing system dependencies  
**Solution:**
```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install build-essential pkg-config libssl-dev

# macOS
brew install openssl pkg-config
```

### Tests Fail

**Problem:** Some tests are timing out  
**Solution:**
```bash
# Run tests with more time
cargo test -- --test-threads=1
```

### Demo Script Fails

**Problem:** `jq` not found  
**Solution:**
```bash
# Ubuntu/Debian
sudo apt-get install jq

# macOS
brew install jq
```

---

## 📖 Documentation Map

**Start Here:**
- `README.md` - Overview and features
- `GETTING_STARTED.md` - This file
- `docs/USER_BENEFITS.md` - Why use this system?

**Architecture & Design:**
- `docs/ARCHITECTURE.md` - System design
- `docs/whitepapers/VCP.md` - Protocol specification
- `docs/whitepapers/AMBIENT_AI.md` - Research paper

**Implementation:**
- `docs/API_REFERENCE.md` - API endpoints
- `docs/TESTING_SUMMARY.md` - Test strategy
- `docs/DEPLOYMENT.md` - Production deployment

**Contributing:**
- `docs/CONTRIBUTING.md` - How to contribute
- `docs/LANGUAGE_DECISION.md` - Why Rust?

---

## 💡 Pro Tips

1. **Use Rust Analyzer** - Install the rust-analyzer extension in VS Code for better IDE support

2. **Enable logging** - See what's happening:
   ```bash
   RUST_LOG=info cargo run --bin api-server
   ```

3. **Watch mode** - Auto-rebuild on changes:
   ```bash
   cargo install cargo-watch
   cargo watch -x test
   ```

4. **Performance profiling** - Find bottlenecks:
   ```bash
   cargo build --release
   RUST_LOG=trace ./target/release/api-server
   ```

5. **Clippy for better code** - Rust's linter:
   ```bash
   cargo clippy
   ```

---

## 🎯 Your First Task

**Challenge:** Register a node and submit a task programmatically

```bash
# 1. Start the server
cargo run --bin api-server &

# 2. Wait for startup
sleep 2

# 3. Register a node
NODE_ID=$(curl -s -X POST http://localhost:3000/api/v1/nodes \
  -H "Content-Type: application/json" \
  -d '{
    "node_id": "challenge-node",
    "region": "us-east",
    "node_type": "compute",
    "capabilities": {
      "bandwidth_mbps": 1000.0,
      "cpu_cores": 16,
      "memory_gb": 32.0,
      "gpu_available": true
    }
  }' | jq -r '.id')

echo "Registered node: $NODE_ID"

# 4. Submit a task
TASK_ID=$(curl -s -X POST http://localhost:3000/api/v1/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "task_type": "computation",
    "inputs": {"x": 42},
    "requirements": {
      "min_nodes": 1,
      "max_execution_time_sec": 60,
      "require_gpu": true,
      "require_proof": true
    }
  }' | jq -r '.id')

echo "Submitted task: $TASK_ID"

# 5. Check task status
curl http://localhost:3000/api/v1/tasks/$TASK_ID | jq
```

---

## 🤝 Get Help

- 🐛 **Found a bug?** [Open an issue](https://github.com/dfeen87/Ambient-AI-VCP-System/issues)
- 💬 **Have questions?** [Start a discussion](https://github.com/dfeen87/Ambient-AI-VCP-System/discussions)
- 📖 **Need docs?** Check the `/docs` directory
- 🔍 **Want examples?** Look in `/examples` and `/demo`

---

## ✅ Checklist: You're Ready When...

- [ ] API server starts successfully
- [ ] All 274 tests pass
- [ ] You can register a node via API
- [ ] You can submit a task via API
- [ ] Dashboard shows real-time data
- [ ] Demo script runs without errors

**Got all checkmarks?** 🎉 You're ready to build something amazing!

---

**Next:** Check out [docs/USER_BENEFITS.md](./docs/USER_BENEFITS.md) to see what you can build with this platform!
