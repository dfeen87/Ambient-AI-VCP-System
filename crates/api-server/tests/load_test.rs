use api_server::models::*;
use api_server::state::AppState;
use std::sync::Arc;
use std::time::Instant;
use tokio::task::JoinSet;

/// Load test for concurrent task submissions
#[tokio::test]
async fn load_test_concurrent_tasks() {
    let state = Arc::new(AppState::new());

    // Register some nodes first
    for i in 0..10 {
        let registration = NodeRegistration {
            node_id: format!("node-{}", i),
            region: "us-west".to_string(),
            node_type: "compute".to_string(),
            capabilities: NodeCapabilities {
                bandwidth_mbps: 1000.0,
                cpu_cores: 16,
                memory_gb: 32.0,
                gpu_available: true,
            },
        };
        state.register_node(registration).await.unwrap();
    }

    // Test concurrent task submissions
    let num_tasks = 1000;
    let start = Instant::now();

    let mut tasks = JoinSet::new();
    for i in 0..num_tasks {
        let state_clone = state.clone();
        tasks.spawn(async move {
            let task = TaskSubmission {
                task_type: "computation".to_string(),
                wasm_module: None,
                inputs: serde_json::json!({"task_id": i}),
                requirements: TaskRequirements {
                    min_nodes: 1,
                    max_execution_time_sec: 300,
                    require_gpu: false,
                    require_proof: false,
                },
            };
            state_clone.submit_task(task).await
        });
    }

    // Wait for all tasks to complete
    let mut success_count = 0;
    while let Some(result) = tasks.join_next().await {
        if result.unwrap().is_ok() {
            success_count += 1;
        }
    }

    let elapsed = start.elapsed();

    println!("✅ Load Test Results:");
    println!("   Tasks submitted: {}", num_tasks);
    println!("   Successful: {}", success_count);
    println!("   Failed: {}", num_tasks - success_count);
    println!("   Time elapsed: {:?}", elapsed);
    println!(
        "   Throughput: {:.2} tasks/sec",
        num_tasks as f64 / elapsed.as_secs_f64()
    );

    assert_eq!(success_count, num_tasks, "All tasks should succeed");
    assert!(elapsed.as_secs() < 10, "Should complete within 10 seconds");
}

/// Load test for node capacity
#[tokio::test]
async fn load_test_node_capacity() {
    let state = Arc::new(AppState::new());

    let num_nodes = 10_000;
    let start = Instant::now();

    let mut tasks = JoinSet::new();
    for i in 0..num_nodes {
        let state_clone = state.clone();
        tasks.spawn(async move {
            let registration = NodeRegistration {
                node_id: format!("node-{:06}", i),
                region: format!("region-{}", i % 10),
                node_type: if i % 4 == 0 {
                    "compute"
                } else if i % 4 == 1 {
                    "storage"
                } else if i % 4 == 2 {
                    "gateway"
                } else {
                    "validator"
                }
                .to_string(),
                capabilities: NodeCapabilities {
                    bandwidth_mbps: 1000.0,
                    cpu_cores: 8,
                    memory_gb: 16.0,
                    gpu_available: i % 10 == 0,
                },
            };
            state_clone.register_node(registration).await
        });
    }

    // Wait for all registrations to complete
    let mut success_count = 0;
    while let Some(result) = tasks.join_next().await {
        if result.unwrap().is_ok() {
            success_count += 1;
        }
    }

    let elapsed = start.elapsed();

    // Verify all nodes are registered
    let nodes = state.list_nodes().await;

    println!("✅ Node Capacity Test Results:");
    println!("   Nodes registered: {}", num_nodes);
    println!("   Successful: {}", success_count);
    println!("   Failed: {}", num_nodes - success_count);
    println!("   Stored in state: {}", nodes.len());
    println!("   Time elapsed: {:?}", elapsed);
    println!(
        "   Throughput: {:.2} nodes/sec",
        num_nodes as f64 / elapsed.as_secs_f64()
    );

    assert_eq!(success_count, num_nodes, "All nodes should register");
    assert_eq!(nodes.len(), num_nodes, "All nodes should be stored");
    assert!(elapsed.as_secs() < 30, "Should complete within 30 seconds");
}

/// Stress test: Mixed operations under load
#[tokio::test]
async fn stress_test_mixed_operations() {
    let state = Arc::new(AppState::new());

    // Register 1000 nodes
    for i in 0..1000 {
        let registration = NodeRegistration {
            node_id: format!("node-{:04}", i),
            region: "us-east".to_string(),
            node_type: "compute".to_string(),
            capabilities: NodeCapabilities {
                bandwidth_mbps: 1000.0,
                cpu_cores: 16,
                memory_gb: 32.0,
                gpu_available: true,
            },
        };
        state.register_node(registration).await.unwrap();
    }

    let start = Instant::now();
    let mut tasks = JoinSet::new();

    // Submit 1000 tasks concurrently
    for i in 0..1000 {
        let state_clone = state.clone();
        tasks.spawn(async move {
            let task = TaskSubmission {
                task_type: "wasm_execution".to_string(),
                wasm_module: None,
                inputs: serde_json::json!({"id": i}),
                requirements: TaskRequirements {
                    min_nodes: 1,
                    max_execution_time_sec: 600,
                    require_gpu: i % 2 == 0,
                    require_proof: i % 3 == 0,
                },
            };
            state_clone.submit_task(task).await
        });
    }

    let mut task_count = 0;
    while let Some(result) = tasks.join_next().await {
        if result.unwrap().is_ok() {
            task_count += 1;
        }
    }

    // Read cluster stats concurrently
    let stats = state.get_cluster_stats().await;

    let elapsed = start.elapsed();

    println!("✅ Stress Test Results:");
    println!("   Nodes: {}", stats.total_nodes);
    println!("   Tasks submitted: {}", task_count);
    println!("   Time elapsed: {:?}", elapsed);
    println!(
        "   Operations/sec: {:.2}",
        (stats.total_nodes + task_count) as f64 / elapsed.as_secs_f64()
    );

    assert_eq!(stats.total_nodes, 1000);
    assert_eq!(task_count, 1000);
}

/// Performance benchmark: Task submission latency
#[tokio::test]
async fn benchmark_task_assignment_latency() {
    let state = Arc::new(AppState::new());

    // Register nodes
    for i in 0..10 {
        let registration = NodeRegistration {
            node_id: format!("node-{}", i),
            region: "us-west".to_string(),
            node_type: "compute".to_string(),
            capabilities: NodeCapabilities {
                bandwidth_mbps: 1000.0,
                cpu_cores: 8,
                memory_gb: 16.0,
                gpu_available: true,
            },
        };
        state.register_node(registration).await.unwrap();
    }

    // Measure latency for single task submission
    let mut latencies = Vec::new();
    for i in 0..100 {
        let start = Instant::now();

        let task = TaskSubmission {
            task_type: "computation".to_string(),
            wasm_module: None,
            inputs: serde_json::json!({"id": i}),
            requirements: TaskRequirements {
                min_nodes: 1,
                max_execution_time_sec: 300,
                require_gpu: false,
                require_proof: false,
            },
        };

        state.submit_task(task).await.unwrap();
        latencies.push(start.elapsed());
    }

    let avg_latency = latencies.iter().sum::<std::time::Duration>() / latencies.len() as u32;
    let max_latency = latencies.iter().max().unwrap();

    println!("✅ Task Assignment Latency:");
    println!("   Average: {:?}", avg_latency);
    println!("   Maximum: {:?}", max_latency);
    println!("   Target: < 100ms");

    assert!(
        avg_latency.as_millis() < 100,
        "Average latency should be < 100ms, got {:?}",
        avg_latency
    );
}
