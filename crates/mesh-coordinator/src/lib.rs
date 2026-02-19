use ambient_node::AmbientNode;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wasm_engine::WasmCall;
use zk_prover::{ZKProof, ZKVerifier};

pub mod assignment;
pub mod peer_routing;
pub mod registry;
pub mod settlement;

pub use assignment::*;
pub use peer_routing::*;
pub use registry::*;
pub use settlement::*;

/// Task requirements specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequirements {
    pub min_health_score: f64,
    pub min_bandwidth_mbps: f64,
    pub max_latency_ms: f64,
    pub required_compute_mb: u32,
}

impl Default for TaskRequirements {
    fn default() -> Self {
        Self {
            min_health_score: 0.5,
            min_bandwidth_mbps: 10.0,
            max_latency_ms: 100.0,
            required_compute_mb: 256,
        }
    }
}

/// Task to be executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub wasm_call: WasmCall,
    pub requirements: TaskRequirements,
    pub reward_amount: f64,
}

/// Task execution result with proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub node_id: String,
    pub output: Vec<u8>,
    pub execution_time_ms: u64,
    pub proof: Option<Vec<u8>>,
}

/// Mesh coordinator for task orchestration
pub struct MeshCoordinator {
    cluster_id: String,
    nodes: HashMap<String, AmbientNode>,
    strategy: TaskAssignmentStrategy,
    verifier: ZKVerifier,
    peer_router: PeerRouter,
}

impl MeshCoordinator {
    pub fn new(cluster_id: String, strategy: TaskAssignmentStrategy) -> Self {
        Self {
            cluster_id,
            nodes: HashMap::new(),
            strategy,
            verifier: ZKVerifier::default(),
            peer_router: PeerRouter::new(),
        }
    }

    /// Register a new node in the mesh
    pub fn register_node(&mut self, node: AmbientNode) {
        let node_id = node.id.id.clone();
        let node_type = node.id.node_type.clone();
        // New nodes start with Unknown connectivity until explicitly updated.
        self.peer_router
            .update_node(&node_id, &node_type, NodeConnectivityStatus::Unknown);
        self.nodes.insert(node_id, node);
    }

    /// Unregister a node
    pub fn unregister_node(&mut self, node_id: &str) {
        self.peer_router.remove_node(node_id);
        self.nodes.remove(node_id);
    }

    /// Update the internet connectivity status of a registered node.
    ///
    /// Call this whenever the node's backhaul state changes so that
    /// [`MeshCoordinator::find_peer_route`] always reflects current
    /// network conditions.
    pub fn sync_connectivity(&mut self, node_id: &str, status: NodeConnectivityStatus) {
        if let Some(node) = self.nodes.get(node_id) {
            let node_type = node.id.node_type.clone();
            self.peer_router.update_node(node_id, &node_type, status);
        }
    }

    /// Find the best peer route for `node_id` to reach the internet.
    ///
    /// Returns `None` if the node is not registered or no internet path
    /// exists (direct or via relay).
    pub fn find_peer_route(&self, node_id: &str) -> Option<PeerRoute> {
        self.peer_router.find_route(node_id)
    }

    /// Select best node for a task based on requirements and strategy
    pub fn select_node_for_task(&self, requirements: TaskRequirements) -> Option<&AmbientNode> {
        // Filter nodes that meet requirements
        let eligible_nodes: Vec<&AmbientNode> = self
            .nodes
            .values()
            .filter(|node| {
                !node.is_safe_mode()
                    && node.health_score() >= requirements.min_health_score
                    && node.telemetry.bandwidth_mbps >= requirements.min_bandwidth_mbps
                    && node.telemetry.avg_latency_ms <= requirements.max_latency_ms
            })
            .collect();

        if eligible_nodes.is_empty() {
            return None;
        }

        // Apply selection strategy
        match self.strategy {
            TaskAssignmentStrategy::Weighted => {
                // Select node with highest health score
                eligible_nodes
                    .iter()
                    .max_by(|a, b| {
                        a.health_score()
                            .partial_cmp(&b.health_score())
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .copied()
            }
            TaskAssignmentStrategy::RoundRobin => {
                // Simple: select first eligible node
                // In production, would track last selected and rotate
                eligible_nodes.first().copied()
            }
            TaskAssignmentStrategy::LeastLoaded => {
                // Select node with lowest CPU usage
                eligible_nodes
                    .iter()
                    .min_by(|a, b| {
                        a.telemetry
                            .cpu_usage_percent
                            .partial_cmp(&b.telemetry.cpu_usage_percent)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .copied()
            }
            TaskAssignmentStrategy::LatencyAware => {
                // Select node with lowest latency
                eligible_nodes
                    .iter()
                    .min_by(|a, b| {
                        a.telemetry
                            .avg_latency_ms
                            .partial_cmp(&b.telemetry.avg_latency_ms)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .copied()
            }
        }
    }

    /// Dispatch task and verify result with proof
    pub async fn dispatch_and_reward(&mut self, task: Task) -> Result<TaskResult> {
        // Select node
        let node = self
            .select_node_for_task(task.requirements.clone())
            .ok_or_else(|| anyhow!("No eligible nodes found"))?;

        let node_id = node.id.id.clone();

        // In production, would actually dispatch to the node
        // For now, return a mock result
        Ok(TaskResult {
            task_id: task.id,
            node_id,
            output: vec![],
            execution_time_ms: 0,
            proof: None,
        })
    }

    /// Verify a task result proof
    pub fn verify_result(&self, result: &TaskResult, proof: &ZKProof) -> bool {
        if let Some(raw_proof) = &result.proof {
            if raw_proof != &proof.proof_data {
                return false;
            }
        }

        self.verifier.verify_proof(proof, &proof.public_inputs)
    }

    /// Get cluster statistics
    pub fn cluster_stats(&self) -> ClusterStats {
        let total_nodes = self.nodes.len();
        let healthy_nodes = self.nodes.values().filter(|n| !n.is_safe_mode()).count();
        let avg_health = if total_nodes > 0 {
            self.nodes.values().map(|n| n.health_score()).sum::<f64>() / total_nodes as f64
        } else {
            0.0
        };

        ClusterStats {
            total_nodes,
            healthy_nodes,
            avg_health_score: avg_health,
        }
    }

    /// Get cluster ID
    pub fn cluster_id(&self) -> &str {
        &self.cluster_id
    }

    /// Get number of registered nodes
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

/// Cluster statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStats {
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub avg_health_score: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambient_node::{NodeId, SafetyPolicy};
    use zk_prover::{ExecutionTrace, ZKProver};

    #[test]
    fn test_coordinator_creation() {
        let coordinator =
            MeshCoordinator::new("test-cluster".to_string(), TaskAssignmentStrategy::Weighted);
        assert_eq!(coordinator.cluster_id(), "test-cluster");
        assert_eq!(coordinator.node_count(), 0);
    }

    #[test]
    fn test_node_registration() {
        let mut coordinator =
            MeshCoordinator::new("test-cluster".to_string(), TaskAssignmentStrategy::Weighted);

        let node_id = NodeId::new("node-001", "us-west", "gateway");
        let node = AmbientNode::new(node_id, SafetyPolicy::default());

        coordinator.register_node(node);
        assert_eq!(coordinator.node_count(), 1);
    }

    #[test]
    fn test_cluster_stats() {
        let mut coordinator =
            MeshCoordinator::new("test-cluster".to_string(), TaskAssignmentStrategy::Weighted);

        let node_id = NodeId::new("node-001", "us-west", "gateway");
        let node = AmbientNode::new(node_id, SafetyPolicy::default());
        coordinator.register_node(node);

        let stats = coordinator.cluster_stats();
        assert_eq!(stats.total_nodes, 1);
    }

    #[test]
    fn test_verify_result_uses_proof_public_inputs() {
        let coordinator =
            MeshCoordinator::new("test-cluster".to_string(), TaskAssignmentStrategy::Weighted);

        let prover = ZKProver::default();
        let trace = ExecutionTrace {
            module_hash: "wiring-test".to_string(),
            function_name: "compute".to_string(),
            inputs: vec![1, 2, 3],
            outputs: vec![4, 5, 6],
            execution_time_ms: 50,
            gas_used: 500,
            timestamp: 42,
        };

        let proof = prover.generate_proof(trace).unwrap();
        let result = TaskResult {
            task_id: "task-1".to_string(),
            node_id: "node-1".to_string(),
            output: vec![9, 9, 9],
            execution_time_ms: 50,
            proof: Some(proof.proof_data.clone()),
        };

        assert!(coordinator.verify_result(&result, &proof));
    }

    #[test]
    fn test_verify_result_rejects_mismatched_embedded_proof() {
        let coordinator =
            MeshCoordinator::new("test-cluster".to_string(), TaskAssignmentStrategy::Weighted);

        let prover = ZKProver::default();
        let trace = ExecutionTrace {
            module_hash: "wiring-test".to_string(),
            function_name: "compute".to_string(),
            inputs: vec![1, 2, 3],
            outputs: vec![4, 5, 6],
            execution_time_ms: 50,
            gas_used: 500,
            timestamp: 42,
        };

        let proof = prover.generate_proof(trace).unwrap();
        let result = TaskResult {
            task_id: "task-1".to_string(),
            node_id: "node-1".to_string(),
            output: vec![9, 9, 9],
            execution_time_ms: 50,
            proof: Some(vec![0, 1, 2]),
        };

        assert!(!coordinator.verify_result(&result, &proof));
    }

    #[test]
    fn test_sync_connectivity_and_find_peer_route_direct() {
        let mut coordinator =
            MeshCoordinator::new("test-cluster".to_string(), TaskAssignmentStrategy::Weighted);

        let node_id = NodeId::new("node-gw", "us-east", "gateway");
        let node = AmbientNode::new(node_id, SafetyPolicy::default());
        coordinator.register_node(node);

        // Mark node as online → direct route with no hops.
        coordinator.sync_connectivity("node-gw", NodeConnectivityStatus::Online);

        let route = coordinator
            .find_peer_route("node-gw")
            .expect("route expected");
        assert!(route.is_direct());
        assert_eq!(route.source_node_id, "node-gw");
    }

    #[test]
    fn test_sync_connectivity_and_find_peer_route_relayed() {
        let mut coordinator =
            MeshCoordinator::new("test-cluster".to_string(), TaskAssignmentStrategy::Weighted);

        // A universal relay that is online.
        let relay_id = NodeId::new("relay-uni", "us-east", "universal");
        let relay = AmbientNode::new(relay_id, SafetyPolicy::default());
        coordinator.register_node(relay);
        coordinator.sync_connectivity("relay-uni", NodeConnectivityStatus::Online);

        // An offline worker that needs to reach the internet.
        let worker_id = NodeId::new("worker-1", "us-east", "worker");
        let worker = AmbientNode::new(worker_id, SafetyPolicy::default());
        coordinator.register_node(worker);
        coordinator.sync_connectivity("worker-1", NodeConnectivityStatus::Offline);

        let route = coordinator
            .find_peer_route("worker-1")
            .expect("relay route expected");
        assert!(!route.is_direct());
        assert_eq!(route.hops.len(), 1);
        assert_eq!(route.hops[0].node_id, "relay-uni");
        assert_eq!(route.hops[0].kind, NodeKind::Universal);
    }

    #[test]
    fn test_unregister_removes_from_peer_router() {
        let mut coordinator =
            MeshCoordinator::new("test-cluster".to_string(), TaskAssignmentStrategy::Weighted);

        let node_id = NodeId::new("node-x", "eu-west", "open");
        let node = AmbientNode::new(node_id, SafetyPolicy::default());
        coordinator.register_node(node);
        coordinator.sync_connectivity("node-x", NodeConnectivityStatus::Online);

        // Removing the node should leave no route for other offline nodes
        // (only node was the potential relay).
        let offline_id = NodeId::new("node-y", "eu-west", "worker");
        let offline = AmbientNode::new(offline_id, SafetyPolicy::default());
        coordinator.register_node(offline);
        coordinator.sync_connectivity("node-y", NodeConnectivityStatus::Offline);

        // Before removal: relay is available.
        assert!(coordinator.find_peer_route("node-y").is_some());

        coordinator.unregister_node("node-x");

        // After removal: no relay → no route.
        assert!(coordinator.find_peer_route("node-y").is_none());
    }
}
