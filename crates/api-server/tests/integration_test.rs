use api_server::models::*;

// Note: Database-dependent tests are commented out because they require TEST_DATABASE_URL
// To run full integration tests with a real database:
// TEST_DATABASE_URL=postgres://user:pass@localhost/test_db cargo test --test integration_test

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
fn test_node_validation_invalid_cpu_cores() {
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

/// Test node validation - invalid memory (too small)
#[test]
fn test_node_validation_invalid_memory() {
    let node_reg = NodeRegistration {
        node_id: "test-node".to_string(),
        region: "us-west".to_string(),
        node_type: "compute".to_string(),
        capabilities: NodeCapabilities {
            bandwidth_mbps: 500.0,
            cpu_cores: 8,
            memory_gb: 0.05,
            gpu_available: false,
        },
    };

    assert!(node_reg.validate().is_err());
}

/// Test node validation - valid node
#[test]
fn test_node_validation_valid() {
    let node_reg = NodeRegistration {
        node_id: "test-node".to_string(),
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

/// Test node validation - "any" node type is valid for universal nodes
#[test]
fn test_node_validation_any_type() {
    let node_reg = NodeRegistration {
        node_id: "universal-node".to_string(),
        region: "us-west".to_string(),
        node_type: "any".to_string(),
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
            require_proof: false,
        },
    };

    assert!(task_sub.validate().is_err());
}

/// Test task validation - invalid min_nodes (zero)
#[test]
fn test_task_validation_invalid_min_nodes() {
    let task_sub = TaskSubmission {
        task_type: "computation".to_string(),
        wasm_module: None,
        inputs: serde_json::json!({}),
        requirements: TaskRequirements {
            min_nodes: 0,
            max_execution_time_sec: 300,
            require_gpu: false,
            require_proof: false,
        },
    };

    assert!(task_sub.validate().is_err());
}

/// Test task validation - invalid execution time (zero)
#[test]
fn test_task_validation_invalid_time() {
    let task_sub = TaskSubmission {
        task_type: "computation".to_string(),
        wasm_module: None,
        inputs: serde_json::json!({}),
        requirements: TaskRequirements {
            min_nodes: 1,
            max_execution_time_sec: 0,
            require_gpu: false,
            require_proof: false,
        },
    };

    assert!(task_sub.validate().is_err());
}

/// Test task validation - valid task
#[test]
fn test_task_validation_valid() {
    let task_sub = TaskSubmission {
        task_type: "computation".to_string(),
        wasm_module: None,
        inputs: serde_json::json!({"key": "value"}),
        requirements: TaskRequirements {
            min_nodes: 1,
            max_execution_time_sec: 300,
            require_gpu: false,
            require_proof: false,
        },
    };

    assert!(task_sub.validate().is_ok());
}

// Database-dependent tests are commented out
// Uncomment and run with TEST_DATABASE_URL set to test with real database

/*
mod common;
use api_server::state::AppState;
use std::sync::Arc;

/// Test state - register node
#[tokio::test]
#[ignore] // Requires TEST_DATABASE_URL
async fn test_state_register_node() {
    let pool = common::create_test_pool().await;
    let state = Arc::new(AppState::new(pool.clone()));

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

    common::cleanup_test_db(&pool).await;
}
*/
