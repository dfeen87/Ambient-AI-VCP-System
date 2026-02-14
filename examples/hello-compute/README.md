# Hello Compute Example

This is a simple example demonstrating how to run a basic computation task in the Ambient AI + VCP System.

## Prerequisites

- Rust 1.75 or later
- Ambient VCP System built and installed

## Running the Example

### 1. Start a coordinator

```bash
cargo run --bin ambient-vcp -- coordinator --cluster-id demo-cluster --strategy weighted
```

### 2. Start a node (in a separate terminal)

```bash
cargo run --bin ambient-vcp -- node --id node-001 --region us-west --node-type compute
```

### 3. (Optional) View node health

```bash
cargo run --bin ambient-vcp -- health
```

## Expected Output

The coordinator should show:
```
Cluster ID: demo-cluster
Strategy: Weighted
Nodes: 0
```

Each node should show:
```
Node ID: node-001
Region: us-west
Type: compute
Health Score: 0.XX
Safe Mode: false
```

## Next Steps

In a production system, you would:
1. Submit tasks to the coordinator
2. The coordinator selects the best node
3. The node executes the WASM task
4. Results are verified with ZK proofs
5. Rewards are distributed

This basic example demonstrates the infrastructure is working correctly.
