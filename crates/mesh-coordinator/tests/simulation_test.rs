//! Simulation tests for the Mesh Coordinator.
//!
//! Each test builds a realistic cluster, runs a coordinator operation, and
//! asserts on the concrete return values – demonstrating what the technology
//! actually returns at runtime.

use ambient_node::{AmbientNode, NodeId, SafetyPolicy, TelemetrySample};
use mesh_coordinator::{
    ClusterStats, MeshCoordinator, NodeConnectivityStatus, Task, TaskAssignmentStrategy,
    TaskRequirements,
};
use wasm_engine::WasmCall;
use std::time::{SystemTime, UNIX_EPOCH};

fn ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn make_node(id: &str, region: &str, node_type: &str, bandwidth: f64, latency: f64) -> AmbientNode {
    let node_id = NodeId::new(id, region, node_type);
    let mut node = AmbientNode::new(node_id, SafetyPolicy::default());
    node.ingest_telemetry(TelemetrySample {
        bandwidth_mbps: bandwidth,
        upload_bandwidth_mbps: bandwidth / 2.0,
        download_bandwidth_mbps: bandwidth / 2.0,
        avg_latency_ms: latency,
        cpu_usage_percent: 20.0,
        memory_usage_percent: 30.0,
        temperature_c: 55.0,
        power_watts: 100.0,
        timestamp: ts(),
    });
    node
}

// ---------------------------------------------------------------------------
// Cluster stats returns
// ---------------------------------------------------------------------------

#[test]
fn simulate_cluster_stats_empty_returns() {
    let coordinator =
        MeshCoordinator::new("sim-cluster".to_string(), TaskAssignmentStrategy::Weighted);

    let stats: ClusterStats = coordinator.cluster_stats();

    assert_eq!(stats.total_nodes, 0);
    assert_eq!(stats.healthy_nodes, 0);
    assert_eq!(stats.avg_health_score, 0.0);
}

#[test]
fn simulate_cluster_stats_with_nodes_returns() {
    let mut coordinator =
        MeshCoordinator::new("sim-cluster".to_string(), TaskAssignmentStrategy::Weighted);

    coordinator.register_node(make_node("n1", "us-west", "gateway", 200.0, 10.0));
    coordinator.register_node(make_node("n2", "us-east", "compute", 150.0, 20.0));
    coordinator.register_node(make_node("n3", "eu-west", "storage", 100.0, 30.0));

    let stats = coordinator.cluster_stats();

    assert_eq!(stats.total_nodes, 3);
    assert_eq!(stats.healthy_nodes, 3, "all nodes should be healthy");
    assert!(
        (0.0..=1.0).contains(&stats.avg_health_score),
        "avg health score must be in [0,1]: {}",
        stats.avg_health_score
    );
    assert!(
        stats.avg_health_score > 0.0,
        "healthy nodes must produce a positive avg health score"
    );
}

// ---------------------------------------------------------------------------
// Task assignment / selection returns
// ---------------------------------------------------------------------------

#[test]
fn simulate_weighted_task_selection_returns() {
    let mut coordinator =
        MeshCoordinator::new("sim-cluster".to_string(), TaskAssignmentStrategy::Weighted);

    // n1 has much higher bandwidth → higher health score → should be selected.
    coordinator.register_node(make_node("n1-fast", "us-west", "gateway", 1000.0, 5.0));
    coordinator.register_node(make_node("n2-slow", "us-east", "compute", 10.0, 90.0));

    let requirements = TaskRequirements {
        min_health_score: 0.1,
        min_bandwidth_mbps: 5.0,
        max_latency_ms: 100.0,
        required_compute_mb: 128,
    };

    let selected = coordinator
        .select_node_for_task(requirements)
        .expect("a node must be selected");

    assert_eq!(selected.id.id, "n1-fast");
}

#[test]
fn simulate_latency_aware_task_selection_returns() {
    let mut coordinator = MeshCoordinator::new(
        "sim-cluster".to_string(),
        TaskAssignmentStrategy::LatencyAware,
    );

    coordinator.register_node(make_node("n1-high-lat", "us-west", "gateway", 200.0, 80.0));
    coordinator.register_node(make_node("n2-low-lat", "us-east", "compute", 200.0, 5.0));

    let requirements = TaskRequirements::default();

    let selected = coordinator
        .select_node_for_task(requirements)
        .expect("a node must be selected");

    assert_eq!(
        selected.id.id, "n2-low-lat",
        "latency-aware strategy must pick the lowest-latency node"
    );
}

#[test]
fn simulate_no_eligible_node_returns_none() {
    let mut coordinator =
        MeshCoordinator::new("sim-cluster".to_string(), TaskAssignmentStrategy::Weighted);

    // Node with very low bandwidth – below the requirement.
    coordinator.register_node(make_node("n1-weak", "us-west", "compute", 1.0, 200.0));

    let requirements = TaskRequirements {
        min_health_score: 0.9,
        min_bandwidth_mbps: 500.0,
        max_latency_ms: 5.0,
        required_compute_mb: 512,
    };

    let selected = coordinator.select_node_for_task(requirements);
    assert!(
        selected.is_none(),
        "no node should qualify for the strict requirements"
    );
}

// ---------------------------------------------------------------------------
// Task dispatch (dispatch_and_reward) returns
// ---------------------------------------------------------------------------

#[tokio::test]
async fn simulate_task_dispatch_returns() {
    let mut coordinator =
        MeshCoordinator::new("sim-cluster".to_string(), TaskAssignmentStrategy::Weighted);

    coordinator.register_node(make_node("dispatch-node", "us-west", "gateway", 500.0, 10.0));

    let task = Task {
        id: "task-sim-001".to_string(),
        wasm_call: WasmCall {
            module_path: "inference.wasm".to_string(),
            function_name: "run".to_string(),
            inputs: vec![1, 2, 3],
        },
        requirements: TaskRequirements::default(),
        reward_amount: 0.05,
    };

    let result = coordinator.dispatch_and_reward(task).await.unwrap();

    assert_eq!(result.task_id, "task-sim-001");
    assert_eq!(result.node_id, "dispatch-node");
    // Stub dispatch returns empty output with no proof.
    assert!(result.output.is_empty());
    assert!(result.proof.is_none());
}

#[tokio::test]
async fn simulate_task_dispatch_no_nodes_returns_error() {
    let mut coordinator =
        MeshCoordinator::new("sim-cluster".to_string(), TaskAssignmentStrategy::Weighted);

    let task = Task {
        id: "task-sim-002".to_string(),
        wasm_call: WasmCall {
            module_path: "inference.wasm".to_string(),
            function_name: "run".to_string(),
            inputs: vec![],
        },
        requirements: TaskRequirements::default(),
        reward_amount: 0.01,
    };

    let result = coordinator.dispatch_and_reward(task).await;
    assert!(
        result.is_err(),
        "dispatch without any registered nodes must return an error"
    );
}

// ---------------------------------------------------------------------------
// Peer routing returns
// ---------------------------------------------------------------------------

#[test]
fn simulate_direct_peer_route_returns() {
    let mut coordinator =
        MeshCoordinator::new("sim-cluster".to_string(), TaskAssignmentStrategy::Weighted);

    coordinator.register_node(make_node("gw-online", "us-west", "gateway", 300.0, 10.0));
    coordinator.sync_connectivity("gw-online", NodeConnectivityStatus::Online);

    let route = coordinator
        .find_peer_route("gw-online")
        .expect("online node must have a direct route");

    assert!(route.is_direct(), "online node must have a direct route");
    assert_eq!(route.source_node_id, "gw-online");
    assert!(route.hops.is_empty(), "direct route has no relay hops");
}

#[test]
fn simulate_relayed_peer_route_returns() {
    let mut coordinator =
        MeshCoordinator::new("sim-cluster".to_string(), TaskAssignmentStrategy::Weighted);

    // Universal relay node that is online.
    coordinator.register_node(make_node("relay-uni", "us-west", "universal", 300.0, 10.0));
    coordinator.sync_connectivity("relay-uni", NodeConnectivityStatus::Online);

    // Worker node that is offline and needs to reach the internet via relay.
    coordinator.register_node(make_node("worker-offline", "us-west", "worker", 50.0, 30.0));
    coordinator.sync_connectivity("worker-offline", NodeConnectivityStatus::Offline);

    let route = coordinator
        .find_peer_route("worker-offline")
        .expect("offline node must get a relayed route");

    assert!(!route.is_direct(), "offline node must use a relay");
    assert_eq!(route.hops.len(), 1);
    assert_eq!(route.hops[0].node_id, "relay-uni");
}

#[test]
fn simulate_no_route_when_no_relay_returns_none() {
    let mut coordinator =
        MeshCoordinator::new("sim-cluster".to_string(), TaskAssignmentStrategy::Weighted);

    coordinator.register_node(make_node("isolated", "us-west", "worker", 50.0, 30.0));
    coordinator.sync_connectivity("isolated", NodeConnectivityStatus::Offline);

    let route = coordinator.find_peer_route("isolated");
    assert!(
        route.is_none(),
        "isolated offline node with no relay must return None"
    );
}
