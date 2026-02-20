//! Simulation tests for the Ambient AI VCP System.
//!
//! These tests exercise the real application code end-to-end and assert on
//! the concrete return values produced by each subsystem, demonstrating what
//! the technology actually returns at runtime.

use ambient_node::{
    AileeEngineAdapter, AmbientNode, ConsensusEngine, ExecutionMode, GenerationRequest,
    LocalModelAdapter, ModelAdapter, NodeId, RemoteModelAdapter, SafetyPolicy, TaskType,
    TelemetrySample, VcpExecutionContext,
};
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn healthy_telemetry() -> TelemetrySample {
    TelemetrySample {
        bandwidth_mbps: 200.0,
        upload_bandwidth_mbps: 100.0,
        download_bandwidth_mbps: 100.0,
        avg_latency_ms: 15.0,
        cpu_usage_percent: 30.0,
        memory_usage_percent: 40.0,
        temperature_c: 55.0,
        power_watts: 120.0,
        timestamp: ts(),
    }
}

// ---------------------------------------------------------------------------
// AmbientNode – lifecycle and health score returns
// ---------------------------------------------------------------------------

#[test]
fn simulate_node_health_score_returns() {
    let node_id = NodeId::new("sim-node-001", "us-west", "gateway");
    let mut node = AmbientNode::new(node_id.clone(), SafetyPolicy::default());

    // Start from a degraded baseline: high latency and saturated CPU/memory.
    node.ingest_telemetry(TelemetrySample {
        bandwidth_mbps: 10.0,
        avg_latency_ms: 90.0,
        cpu_usage_percent: 90.0,
        memory_usage_percent: 90.0,
        temperature_c: 60.0,
        ..Default::default()
    });
    let degraded_score = node.health_score();
    assert!(
        (0.0..=1.0).contains(&degraded_score),
        "health score must be in [0,1], got {degraded_score}"
    );

    // Ingest realistic healthy telemetry and confirm score improves.
    node.ingest_telemetry(healthy_telemetry());
    let healthy_score = node.health_score();
    assert!(
        healthy_score > degraded_score,
        "healthy telemetry should raise health score: {healthy_score} vs {degraded_score}"
    );
    assert!(
        !node.is_safe_mode(),
        "healthy node must not be in safe mode"
    );
}

#[test]
fn simulate_node_safe_mode_returns() {
    let node_id = NodeId::new("sim-node-002", "eu-central", "compute");
    let mut node = AmbientNode::new(node_id, SafetyPolicy::default());

    // Overheated telemetry triggers safe mode.
    node.ingest_telemetry(TelemetrySample {
        temperature_c: 95.0, // exceeds 85 °C threshold
        avg_latency_ms: 20.0,
        bandwidth_mbps: 100.0,
        ..Default::default()
    });

    assert!(node.is_safe_mode(), "overheated node must enter safe mode");
}

#[test]
fn simulate_node_reputation_returns() {
    let node_id = NodeId::new("sim-node-003", "ap-south", "validator");
    let mut node = AmbientNode::new(node_id, SafetyPolicy::default());

    // Brand-new node starts at 0.5.
    assert_eq!(node.reputation.score(), 0.5);

    // One success brings it to 1.0.
    node.update_reputation(true, 0.1);
    assert_eq!(node.reputation.score(), 1.0);

    // Each failure halves the score.
    node.update_reputation(false, 0.1);
    assert_eq!(node.reputation.score(), 0.5);
    node.update_reputation(false, 0.1);
    assert!(
        node.reputation.score() < 0.5,
        "repeated failures must lower reputation below 0.5"
    );
}

// ---------------------------------------------------------------------------
// AILEE ConsensusEngine – GenerationResult returns
// ---------------------------------------------------------------------------

#[tokio::test]
async fn simulate_local_consensus_returns() {
    let engine = ConsensusEngine::new(2);

    let adapters: Vec<Box<dyn ModelAdapter>> = vec![
        Box::new(LocalModelAdapter::new("sim-local-a")),
        Box::new(LocalModelAdapter::new("sim-local-b")),
    ];

    let request = GenerationRequest::new(
        "Describe how ambient mesh networking works",
        TaskType::Chat,
        0.6,
        ExecutionMode::Local,
        true,
    );

    let result = engine.execute(&request, adapters).await.unwrap();

    // final_output contains the prompt text (stub adapter echoes it).
    assert!(
        result.final_output.contains("ambient mesh networking"),
        "output should echo the prompt: {}",
        result.final_output
    );
    // trust_score is always in [0,1].
    assert!(
        (0.0..=1.0).contains(&result.trust_score),
        "trust score out of range: {}",
        result.trust_score
    );
    // Both local models contributed.
    assert_eq!(result.model_lineage.len(), 2);
    assert!(result.model_lineage.contains(&"sim-local-a".to_string()));
    assert!(result.model_lineage.contains(&"sim-local-b".to_string()));
    // Execution happened offline (no remote adapter).
    assert!(result.execution_metadata.was_offline);
    assert_eq!(result.execution_metadata.models_consulted, 2);
    assert_eq!(result.execution_metadata.models_succeeded, 2);
    // Cryptographic integrity of the result.
    assert!(result.verify_hash());
    assert_eq!(result.input_hash, request.hash());
}

#[tokio::test]
async fn simulate_hybrid_mode_returns() {
    let engine = ConsensusEngine::new(1);

    let adapters: Vec<Box<dyn ModelAdapter>> = vec![
        Box::new(LocalModelAdapter::new("sim-local-hybrid")),
        Box::new(RemoteModelAdapter::new("sim-remote-hybrid", true)),
    ];

    let request = GenerationRequest::new(
        "Generate a Rust function that computes factorial",
        TaskType::Code,
        0.5,
        ExecutionMode::Hybrid,
        true,
    );

    let result = engine.execute(&request, adapters).await.unwrap();

    // Both adapters should have responded.
    assert_eq!(result.model_lineage.len(), 2);
    // Remote model was online so execution was not fully offline.
    assert!(!result.execution_metadata.was_offline);
    assert!(result.verify_hash());
}

#[tokio::test]
async fn simulate_offline_graceful_degradation_returns() {
    let engine = ConsensusEngine::new(1);

    let adapters: Vec<Box<dyn ModelAdapter>> = vec![
        Box::new(LocalModelAdapter::new("sim-local-fallback")),
        Box::new(RemoteModelAdapter::new("sim-remote-down", false)), // offline
    ];

    let request = GenerationRequest::new(
        "Analyze network topology",
        TaskType::Analysis,
        0.5,
        ExecutionMode::Hybrid,
        true,
    );

    let result = engine.execute(&request, adapters).await.unwrap();

    // Offline remote adapter must be excluded; only local contributes.
    assert_eq!(result.model_lineage.len(), 1);
    assert_eq!(result.model_lineage[0], "sim-local-fallback");
    assert!(result.execution_metadata.was_offline);
    assert!(result.verify_hash());
}

#[tokio::test]
async fn simulate_code_task_type_returns() {
    let engine = ConsensusEngine::new(1);

    let adapters: Vec<Box<dyn ModelAdapter>> =
        vec![Box::new(LocalModelAdapter::new("code-model"))];

    let request = GenerationRequest::new(
        "fibonacci",
        TaskType::Code,
        0.5,
        ExecutionMode::Local,
        true,
    );

    let result = engine.execute(&request, adapters).await.unwrap();

    // Stub local adapter prefixes Code outputs with "// Generated code:\n".
    assert!(
        result.final_output.starts_with("// Generated code:"),
        "code task output should start with code prefix: {}",
        result.final_output
    );
}

#[tokio::test]
async fn simulate_analysis_task_type_returns() {
    let engine = ConsensusEngine::new(1);

    let adapters: Vec<Box<dyn ModelAdapter>> =
        vec![Box::new(LocalModelAdapter::new("analysis-model"))];

    let request = GenerationRequest::new(
        "dataset metrics",
        TaskType::Analysis,
        0.5,
        ExecutionMode::Local,
        true,
    );

    let result = engine.execute(&request, adapters).await.unwrap();

    // Stub local adapter prefixes Analysis outputs with "Analysis: ".
    assert!(
        result.final_output.starts_with("Analysis: "),
        "analysis task output should start with analysis prefix: {}",
        result.final_output
    );
}

// ---------------------------------------------------------------------------
// VCP Integration Adapter – contextual execution returns
// ---------------------------------------------------------------------------

#[tokio::test]
async fn simulate_vcp_adapter_online_returns() {
    let adapter = AileeEngineAdapter::new(2);

    let context = VcpExecutionContext::new(
        true,        // online
        "us-west-1",
        "gateway",
        5000,
        true,
    );

    let result = adapter
        .execute_with_context(
            "Explain zero-knowledge proofs",
            TaskType::Analysis,
            0.6,
            &context,
        )
        .await
        .unwrap();

    assert!(!result.final_output.is_empty());
    assert!((0.0..=1.0).contains(&result.trust_score));
    assert!(result.model_lineage.len() >= 2);
    assert!(result.verify_hash());
}

#[tokio::test]
async fn simulate_vcp_adapter_offline_returns() {
    let adapter = AileeEngineAdapter::new(2);

    let context = VcpExecutionContext::new(
        false,       // offline
        "eu-central-1",
        "compute",
        3000,
        true,
    );

    let result = adapter
        .execute_with_context(
            "Summarize the FedAvg algorithm",
            TaskType::Chat,
            0.5,
            &context,
        )
        .await
        .unwrap();

    assert!(!result.final_output.is_empty());
    // Offline context must run on local models only.
    assert!(result.execution_metadata.was_offline);
    assert_eq!(result.model_lineage.len(), 2);
    assert!(result.verify_hash());
}

#[tokio::test]
async fn simulate_request_hash_determinism_returns() {
    // The same request must always hash to the same value – this is the
    // foundation of deterministic replay in VCP.
    let req1 = GenerationRequest::new(
        "deterministic replay test",
        TaskType::Code,
        0.8,
        ExecutionMode::Hybrid,
        true,
    );
    let req2 = GenerationRequest::new(
        "deterministic replay test",
        TaskType::Code,
        0.8,
        ExecutionMode::Hybrid,
        true,
    );

    assert_eq!(
        req1.hash(),
        req2.hash(),
        "identical requests must produce identical hashes"
    );

    let req3 = GenerationRequest::new(
        "different prompt",
        TaskType::Code,
        0.8,
        ExecutionMode::Hybrid,
        true,
    );
    assert_ne!(
        req1.hash(),
        req3.hash(),
        "different prompts must produce different hashes"
    );
}
