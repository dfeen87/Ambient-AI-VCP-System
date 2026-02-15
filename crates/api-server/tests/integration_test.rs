use api_server::{models::*, state::AppState};

/// Test node validation - empty node_id
#[test]
fn test_node_validation_empty_id() {
    let node_reg = NodeRegistration {
        node_id: "".to_string(),
        region: "us-west".to_string(),
        node_type: "compute".to_string(),
        capabilities: NodeCapabilities {
            bandwidth_mbps: 500.0,
            cpu_cores: 8,
            memory_gb: 16.0,
            gpu_available: false,
        },
    };

    assert!(node_reg.validate().is_err());
}

/// Test node validation - invalid node_type
#[test]
fn test_node_validation_invalid_type() {
    let node_reg = NodeRegistration {
        node_id: "test-node".to_string(),
        region: "us-west".to_string(),
        node_type: "invalid".to_string(),
        capabilities: NodeCapabilities {
            bandwidth_mbps: 500.0,
            cpu_cores: 8,
            memory_gb: 16.0,
            gpu_available: false,
        },
    };

    assert!(node_reg.validate().is_err());
}

/// Test node validation - invalid bandwidth
#[test]
fn test_node_validation_invalid_bandwidth() {
    let node_reg = NodeRegistration {
        node_id: "test-node".to_string(),
        region: "us-west".to_string(),
        node_type: "compute".to_string(),
        capabilities: NodeCapabilities {
            bandwidth_mbps: -100.0,
            cpu_cores: 8,
            memory_gb: 16.0,
            gpu_available: false,
        },
    };

    assert!(node_reg.validate().is_err());
}

/// Test node validation - invalid CPU cores (zero)
#[test]
fn test_node_validation_zero_cpu_cores() {
    let node_reg = NodeRegistration {
        node_id: "test-node".to_string(),
        region: "us-west".to_string(),
        node_type: "compute".to_string(),
        capabilities: NodeCapabilities {
            bandwidth_mbps: 500.0,
            cpu_cores: 0,
            memory_gb: 16.0,
            gpu_available: false,
        },
    };

    assert!(node_reg.validate().is_err());
}

/// Test node validation - valid node
#[test]
fn test_node_validation_valid() {
    let node_reg = NodeRegistration {
        node_id: "test-node-001".to_string(),
        region: "us-west".to_string(),
        node_type: "compute".to_string(),
        capabilities: NodeCapabilities {
            bandwidth_mbps: 500.0,
            cpu_cores: 8,
            memory_gb: 16.0,
            gpu_available: false,
        },
    };

    assert!(node_reg.validate().is_ok());
}

/// Test task validation - invalid task_type
#[test]
fn test_task_validation_invalid_type() {
    let task_sub = TaskSubmission {
        task_type: "invalid_type".to_string(),
        wasm_module: None,
        inputs: serde_json::json!({}),
        requirements: TaskRequirements {
            min_nodes: 1,
            max_execution_time_sec: 300,
            require_gpu: false,
            require_proof: true,
        },
    };

    assert!(task_sub.validate().is_err());
}

/// Test task validation - invalid min_nodes (zero)
#[test]
fn test_task_validation_zero_min_nodes() {
    let task_sub = TaskSubmission {
        task_type: "federated_learning".to_string(),
        wasm_module: None,
        inputs: serde_json::json!({}),
        requirements: TaskRequirements {
            min_nodes: 0,
            max_execution_time_sec: 300,
            require_gpu: false,
            require_proof: true,
        },
    };

    assert!(task_sub.validate().is_err());
}

/// Test task validation - invalid max_execution_time (too large)
#[test]
fn test_task_validation_invalid_execution_time() {
    let task_sub = TaskSubmission {
        task_type: "federated_learning".to_string(),
        wasm_module: None,
        inputs: serde_json::json!({}),
        requirements: TaskRequirements {
            min_nodes: 1,
            max_execution_time_sec: 10000, // Too large
            require_gpu: false,
            require_proof: true,
        },
    };

    assert!(task_sub.validate().is_err());
}

/// Test task validation - valid task
#[test]
fn test_task_validation_valid() {
    let task_sub = TaskSubmission {
        task_type: "federated_learning".to_string(),
        wasm_module: None,
        inputs: serde_json::json!({"model": "test"}),
        requirements: TaskRequirements {
            min_nodes: 1,
            max_execution_time_sec: 300,
            require_gpu: false,
            require_proof: true,
        },
    };

    assert!(task_sub.validate().is_ok());
}

/// Test state - register node
#[tokio::test]
async fn test_state_register_node() {
    let state = AppState::new();

    let node_reg = NodeRegistration {
        node_id: "test-node-001".to_string(),
        region: "us-west".to_string(),
        node_type: "compute".to_string(),
        capabilities: NodeCapabilities {
            bandwidth_mbps: 500.0,
            cpu_cores: 8,
            memory_gb: 16.0,
            gpu_available: false,
        },
    };

    let node_info = state.register_node(node_reg).await.unwrap();
    assert_eq!(node_info.node_id, "test-node-001");
    assert_eq!(node_info.region, "us-west");
    assert_eq!(node_info.health_score, 100.0);
}

/// Test state - list nodes
#[tokio::test]
async fn test_state_list_nodes() {
    let state = AppState::new();

    // Register a node
    let node_reg = NodeRegistration {
        node_id: "test-node-001".to_string(),
        region: "us-west".to_string(),
        node_type: "compute".to_string(),
        capabilities: NodeCapabilities {
            bandwidth_mbps: 500.0,
            cpu_cores: 8,
            memory_gb: 16.0,
            gpu_available: false,
        },
    };
    state.register_node(node_reg).await.unwrap();

    let nodes = state.list_nodes().await;
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].node_id, "test-node-001");
}

/// Test state - submit task
#[tokio::test]
async fn test_state_submit_task() {
    let state = AppState::new();

    let task_sub = TaskSubmission {
        task_type: "federated_learning".to_string(),
        wasm_module: None,
        inputs: serde_json::json!({"model": "test"}),
        requirements: TaskRequirements {
            min_nodes: 1,
            max_execution_time_sec: 300,
            require_gpu: false,
            require_proof: true,
        },
    };

    let task_info = state.submit_task(task_sub).await.unwrap();
    assert_eq!(task_info.task_type, "federated_learning");
    assert_eq!(task_info.status, TaskStatus::Pending);
}

/// Test state - cluster stats
#[tokio::test]
async fn test_state_cluster_stats() {
    let state = AppState::new();

    // Initially empty
    let stats = state.get_cluster_stats().await;
    assert_eq!(stats.total_nodes, 0);
    assert_eq!(stats.total_tasks, 0);

    // After registering a node
    let node_reg = NodeRegistration {
        node_id: "test-node-001".to_string(),
        region: "us-west".to_string(),
        node_type: "compute".to_string(),
        capabilities: NodeCapabilities {
            bandwidth_mbps: 500.0,
            cpu_cores: 8,
            memory_gb: 16.0,
            gpu_available: false,
        },
    };
    state.register_node(node_reg).await.unwrap();

    let stats = state.get_cluster_stats().await;
    assert_eq!(stats.total_nodes, 1);
    assert_eq!(stats.healthy_nodes, 1);
}
