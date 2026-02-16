use api_server::models::*;
use api_server::state::AppState;
use sqlx::PgPool;
use uuid::Uuid;

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

/// Test node validation - bandwidth below whitelist minimum
#[test]
fn test_node_validation_bandwidth_below_whitelist_min() {
    let node_reg = NodeRegistration {
        node_id: "test-node".to_string(),
        region: "us-west".to_string(),
        node_type: "compute".to_string(),
        capabilities: NodeCapabilities {
            bandwidth_mbps: 5.0,
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

/// Test task validation - wasm module is rejected for non-wasm task types
#[test]
fn test_task_validation_rejects_wasm_for_non_wasm_type() {
    let task_sub = TaskSubmission {
        task_type: "computation".to_string(),
        wasm_module: Some("AA==".to_string()),
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

/// Test task validation - runtime is capped by task-type registry
#[test]
fn test_task_validation_enforces_task_registry_runtime_limit() {
    let task_sub = TaskSubmission {
        task_type: "wasm_execution".to_string(),
        wasm_module: Some("AA==".to_string()),
        inputs: serde_json::json!({}),
        requirements: TaskRequirements {
            min_nodes: 1,
            max_execution_time_sec: 1200,
            require_gpu: false,
            require_proof: false,
        },
    };

    assert!(task_sub.validate().is_err());
}

#[tokio::test]
async fn test_pending_task_captures_newly_registered_node() {
    let db_url = match std::env::var("TEST_DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            println!("Skipping test_pending_task_captures_newly_registered_node — no TEST_DATABASE_URL set");
            return;
        }
    };

    let pool = PgPool::connect(&db_url)
        .await
        .expect("connect to postgres for integration test");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations should apply successfully for integration test");

    sqlx::query("TRUNCATE TABLE task_assignments, tasks, nodes, users CASCADE")
        .execute(&pool)
        .await
        .expect("cleanup tables before integration test");

    let state = AppState::new(pool.clone());
    let task = TaskSubmission {
        task_type: "computation".to_string(),
        wasm_module: None,
        inputs: serde_json::json!({"job": "pending-capture"}),
        requirements: TaskRequirements {
            min_nodes: 1,
            max_execution_time_sec: 300,
            require_gpu: false,
            require_proof: false,
        },
    };

    let creator_id = Uuid::new_v4();
    let submitted_task = state
        .submit_task(task, creator_id)
        .await
        .expect("task should be accepted as pending even when no eligible nodes are available yet");
    assert_eq!(submitted_task.status, TaskStatus::Pending);
    assert!(submitted_task.assigned_nodes.is_empty());

    let node_id = format!("pending-capture-node-{}", Uuid::new_v4());
    let registration = NodeRegistration {
        node_id: node_id.clone(),
        region: "us-west".to_string(),
        node_type: "compute".to_string(),
        capabilities: NodeCapabilities {
            bandwidth_mbps: 100.0,
            cpu_cores: 8,
            memory_gb: 16.0,
            gpu_available: false,
        },
    };

    state
        .register_node(registration, Uuid::new_v4())
        .await
        .expect("node registration should succeed and trigger pending task assignment");

    let updated_task = state
        .get_task(&submitted_task.task_id, creator_id)
        .await
        .expect("submitted task should still exist");

    assert_eq!(updated_task.status, TaskStatus::Running);
    assert!(updated_task.assigned_nodes.contains(&node_id));

    sqlx::query("TRUNCATE TABLE task_assignments, tasks, nodes, users CASCADE")
        .execute(&pool)
        .await
        .expect("cleanup tables after integration test");
}

#[tokio::test]
async fn test_task_assignment_uses_universal_node_type_any() {
    let db_url = match std::env::var("TEST_DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            println!("Skipping test_task_assignment_uses_universal_node_type_any — no TEST_DATABASE_URL set");
            return;
        }
    };

    let pool = PgPool::connect(&db_url)
        .await
        .expect("connect to postgres for integration test");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations should apply successfully for integration test");

    sqlx::query("TRUNCATE TABLE task_assignments, tasks, nodes, users CASCADE")
        .execute(&pool)
        .await
        .expect("cleanup tables before integration test");

    let state = AppState::new(pool.clone());
    let universal_node_id = format!("universal-any-node-{}", Uuid::new_v4());

    state
        .register_node(
            NodeRegistration {
                node_id: universal_node_id.clone(),
                region: "us-west".to_string(),
                node_type: "any".to_string(),
                capabilities: NodeCapabilities {
                    bandwidth_mbps: 1000.0,
                    cpu_cores: 16,
                    memory_gb: 64.0,
                    gpu_available: false,
                },
            },
            Uuid::new_v4(),
        )
        .await
        .expect("universal any node should register");

    let creator_id = Uuid::new_v4();
    let submitted_task = state
        .submit_task(
            TaskSubmission {
                task_type: "computation".to_string(),
                wasm_module: None,
                inputs: serde_json::json!({"job": "universal-should-match"}),
                requirements: TaskRequirements {
                    min_nodes: 1,
                    max_execution_time_sec: 300,
                    require_gpu: false,
                    require_proof: false,
                },
            },
            creator_id,
        )
        .await
        .expect("task submission should succeed");

    let task = state
        .get_task(&submitted_task.task_id, creator_id)
        .await
        .expect("submitted task should exist");

    assert_eq!(task.status, TaskStatus::Running);
    assert!(task.assigned_nodes.contains(&universal_node_id));

    sqlx::query("TRUNCATE TABLE task_assignments, tasks, nodes, users CASCADE")
        .execute(&pool)
        .await
        .expect("cleanup tables after integration test");
}

#[tokio::test]
async fn test_task_assignment_excludes_non_matching_node_types() {
    let db_url = match std::env::var("TEST_DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            println!("Skipping test_task_assignment_excludes_non_matching_node_types — no TEST_DATABASE_URL set");
            return;
        }
    };

    let pool = PgPool::connect(&db_url)
        .await
        .expect("connect to postgres for integration test");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations should apply successfully for integration test");

    sqlx::query("TRUNCATE TABLE task_assignments, tasks, nodes, users CASCADE")
        .execute(&pool)
        .await
        .expect("cleanup tables before integration test");

    let state = AppState::new(pool.clone());
    let gateway_node_id = format!("gateway-node-{}", Uuid::new_v4());

    state
        .register_node(
            NodeRegistration {
                node_id: gateway_node_id.clone(),
                region: "us-west".to_string(),
                node_type: "gateway".to_string(),
                capabilities: NodeCapabilities {
                    bandwidth_mbps: 1000.0,
                    cpu_cores: 16,
                    memory_gb: 64.0,
                    gpu_available: false,
                },
            },
            Uuid::new_v4(),
        )
        .await
        .expect("gateway node should register");

    let creator_id = Uuid::new_v4();
    let submitted_task = state
        .submit_task(
            TaskSubmission {
                task_type: "computation".to_string(),
                wasm_module: None,
                inputs: serde_json::json!({"job": "gateway-should-not-match"}),
                requirements: TaskRequirements {
                    min_nodes: 1,
                    max_execution_time_sec: 300,
                    require_gpu: false,
                    require_proof: false,
                },
            },
            creator_id,
        )
        .await
        .expect("task submission should succeed");

    let task = state
        .get_task(&submitted_task.task_id, creator_id)
        .await
        .expect("submitted task should exist");

    assert_eq!(task.status, TaskStatus::Pending);
    assert!(!task.assigned_nodes.contains(&gateway_node_id));

    sqlx::query("TRUNCATE TABLE task_assignments, tasks, nodes, users CASCADE")
        .execute(&pool)
        .await
        .expect("cleanup tables after integration test");
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
