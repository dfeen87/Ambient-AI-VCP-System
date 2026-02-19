//! Integration tests for AILEE Trust Layer
//!
//! These tests verify:
//! 1. Direct AILEE API usage (substrate-agnostic)
//! 2. VCP integration adapter (VCP-specific context passing)

use ambient_node::{
    AileeEngineAdapter, ConsensusEngine, ExecutionMode, GenerationRequest, LocalModelAdapter,
    ModelAdapter, RemoteModelAdapter, TaskType, VcpExecutionContext,
};

// ============================================================================
// Direct AILEE API Tests (substrate-agnostic)
// ============================================================================

#[tokio::test]
async fn test_end_to_end_generation_local_only() {
    // Create a consensus engine requiring at least 2 models
    let engine = ConsensusEngine::new(2);

    // Create local adapters
    let adapters: Vec<Box<dyn ModelAdapter>> = vec![
        Box::new(LocalModelAdapter::new("local-gpt-small")),
        Box::new(LocalModelAdapter::new("local-llama")),
    ];

    // Create a generation request
    let request = GenerationRequest::new(
        "Explain quantum computing in simple terms",
        TaskType::Chat,
        0.7, // 70% trust threshold
        ExecutionMode::Local,
        true, // Allow offline
    );

    // Execute the request
    let result = engine.execute(&request, adapters).await.unwrap();

    // Verify the result
    assert!(!result.final_output.is_empty());
    assert!(result.trust_score >= 0.0 && result.trust_score <= 1.0);
    assert_eq!(result.model_lineage.len(), 2);
    assert!(result
        .model_lineage
        .contains(&"local-gpt-small".to_string()));
    assert!(result.model_lineage.contains(&"local-llama".to_string()));

    // Verify metadata
    assert_eq!(result.execution_metadata.models_consulted, 2);
    assert_eq!(result.execution_metadata.models_succeeded, 2);
    assert!(result.execution_metadata.was_offline);

    // Verify hashing
    assert!(result.verify_hash());
    assert_eq!(result.input_hash, request.hash());
}

#[tokio::test]
async fn test_end_to_end_generation_hybrid_mode() {
    let engine = ConsensusEngine::new(1);

    // Create mix of local and remote adapters
    let adapters: Vec<Box<dyn ModelAdapter>> = vec![
        Box::new(LocalModelAdapter::new("local-gpt-small")),
        Box::new(RemoteModelAdapter::new("remote-gpt4", true)), // Online
    ];

    let request = GenerationRequest::new(
        "Write a Python function to calculate fibonacci",
        TaskType::Code,
        0.5,
        ExecutionMode::Hybrid,
        true,
    );

    let result = engine.execute(&request, adapters).await.unwrap();

    assert!(!result.final_output.is_empty());
    assert_eq!(result.model_lineage.len(), 2);
    assert!(!result.execution_metadata.was_offline); // Has remote model
}

#[tokio::test]
async fn test_graceful_degradation_to_local() {
    let engine = ConsensusEngine::new(1);

    // Create adapters with offline remote
    let adapters: Vec<Box<dyn ModelAdapter>> = vec![
        Box::new(LocalModelAdapter::new("local-model")),
        Box::new(RemoteModelAdapter::new("remote-model", false)), // Offline
    ];

    let request = GenerationRequest::new(
        "Analyze this data",
        TaskType::Analysis,
        0.5,
        ExecutionMode::Hybrid,
        true, // Allow offline
    );

    let result = engine.execute(&request, adapters).await.unwrap();

    // Should still succeed with only local model
    assert!(!result.final_output.is_empty());
    assert_eq!(result.model_lineage.len(), 1);
    assert_eq!(result.model_lineage[0], "local-model");
    assert!(result.execution_metadata.was_offline);
}

#[tokio::test]
async fn test_high_trust_threshold() {
    let engine = ConsensusEngine::new(2);

    let adapters: Vec<Box<dyn ModelAdapter>> = vec![
        Box::new(LocalModelAdapter::new("model-1")),
        Box::new(LocalModelAdapter::new("model-2")),
    ];

    let request = GenerationRequest::new(
        "Critical medical advice",
        TaskType::Analysis,
        0.95, // Very high threshold
        ExecutionMode::Local,
        true,
    );

    // This should still succeed but might warn about threshold
    let result = engine.execute(&request, adapters).await;

    // The consensus engine will use best available even if below threshold
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_insufficient_models() {
    let engine = ConsensusEngine::new(3); // Require 3 models

    let adapters: Vec<Box<dyn ModelAdapter>> = vec![Box::new(LocalModelAdapter::new("only-one"))];

    let request = GenerationRequest::new(
        "Test prompt",
        TaskType::Chat,
        0.5,
        ExecutionMode::Local,
        true,
    );

    let result = engine.execute(&request, adapters).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_request_hash_consistency() {
    let req1 = GenerationRequest::new(
        "identical prompt",
        TaskType::Code,
        0.8,
        ExecutionMode::Hybrid,
        true,
    );

    let req2 = GenerationRequest::new(
        "identical prompt",
        TaskType::Code,
        0.8,
        ExecutionMode::Hybrid,
        true,
    );

    // Same inputs should produce same hash
    assert_eq!(req1.hash(), req2.hash());
}

#[tokio::test]
async fn test_trust_score_calculation() {
    let engine = ConsensusEngine::new(1);

    let adapters: Vec<Box<dyn ModelAdapter>> = vec![
        Box::new(LocalModelAdapter::new("model-a")),
        Box::new(LocalModelAdapter::new("model-b")),
    ];

    let request = GenerationRequest::new(
        "Safe and reasonable request",
        TaskType::Chat,
        0.5,
        ExecutionMode::Local,
        true,
    );

    let result = engine.execute(&request, adapters).await.unwrap();

    // Trust score should be reasonable
    assert!(result.trust_score >= 0.0);
    assert!(result.trust_score <= 1.0);
}

#[tokio::test]
async fn test_execution_mode_local_only() {
    let engine = ConsensusEngine::new(1);

    let adapters: Vec<Box<dyn ModelAdapter>> = vec![
        Box::new(LocalModelAdapter::new("local-1")),
        Box::new(RemoteModelAdapter::new("remote-1", true)),
    ];

    let request = GenerationRequest::new(
        "Test",
        TaskType::Chat,
        0.5,
        ExecutionMode::Local, // Local only
        true,
    );

    let result = engine.execute(&request, adapters).await.unwrap();

    // Should only use local model
    assert_eq!(result.model_lineage.len(), 1);
    assert_eq!(result.model_lineage[0], "local-1");
}

#[tokio::test]
async fn test_execution_mode_remote_only() {
    let engine = ConsensusEngine::new(1);

    let adapters: Vec<Box<dyn ModelAdapter>> = vec![
        Box::new(LocalModelAdapter::new("local-1")),
        Box::new(RemoteModelAdapter::new("remote-1", true)),
    ];

    let request = GenerationRequest::new(
        "Test",
        TaskType::Code,
        0.5,
        ExecutionMode::Remote, // Remote only
        false,
    );

    let result = engine.execute(&request, adapters).await.unwrap();

    // Should only use remote model
    assert_eq!(result.model_lineage.len(), 1);
    assert_eq!(result.model_lineage[0], "remote-1");
}

#[tokio::test]
async fn test_multiple_task_types() {
    let engine = ConsensusEngine::new(1);

    for task_type in [TaskType::Chat, TaskType::Code, TaskType::Analysis] {
        let adapters: Vec<Box<dyn ModelAdapter>> =
            vec![Box::new(LocalModelAdapter::new("versatile-model"))];

        let request = GenerationRequest::new(
            "Test prompt for different task types",
            task_type,
            0.5,
            ExecutionMode::Local,
            true,
        );

        let result = engine.execute(&request, adapters).await.unwrap();
        assert!(!result.final_output.is_empty());
    }
}

// ============================================================================
// VCP Integration Adapter Tests (VCP-specific context passing)
// ============================================================================

#[tokio::test]
async fn test_vcp_adapter_online_execution() {
    let adapter = AileeEngineAdapter::new(2);

    let context = VcpExecutionContext::new(
        true, // Online
        "us-west-1",
        "gateway",
        5000, // 5 second execution budget
        true, // Allow offline fallback
    );

    let result = adapter
        .execute_with_context(
            "Explain distributed consensus",
            TaskType::Analysis,
            0.7,
            &context,
        )
        .await
        .unwrap();

    // Verify execution succeeded
    assert!(!result.final_output.is_empty());
    assert!(result.trust_score >= 0.0 && result.trust_score <= 1.0);
    assert!(result.model_lineage.len() >= 2);

    // Verify deterministic hashing
    assert!(result.verify_hash());
}

#[tokio::test]
async fn test_vcp_adapter_offline_resilience() {
    let adapter = AileeEngineAdapter::new(2);

    let context = VcpExecutionContext::new(
        false, // Offline
        "eu-central-1",
        "compute",
        3000,
        true, // Allow offline execution
    );

    let result = adapter
        .execute_with_context(
            "Generate Python fibonacci function",
            TaskType::Code,
            0.6,
            &context,
        )
        .await
        .unwrap();

    // Should succeed using local models only
    assert!(!result.final_output.is_empty());
    assert!(result.execution_metadata.was_offline);

    // Should only use local models (no remote when offline)
    assert_eq!(result.model_lineage.len(), 2);
}

#[tokio::test]
async fn test_vcp_adapter_deterministic_replay() {
    let adapter = AileeEngineAdapter::new(1);

    let context = VcpExecutionContext::new(
        false, // Offline for determinism
        "ap-south-1",
        "validator",
        2000,
        true,
    );

    let prompt = "Deterministic test prompt";

    // Execute twice with same inputs
    let result1 = adapter
        .execute_with_context(prompt, TaskType::Chat, 0.5, &context)
        .await
        .unwrap();

    let result2 = adapter
        .execute_with_context(prompt, TaskType::Chat, 0.5, &context)
        .await
        .unwrap();

    // Same inputs should produce same input hash (deterministic request)
    assert_eq!(result1.input_hash, result2.input_hash);

    // Output verification
    assert!(result1.verify_hash());
    assert!(result2.verify_hash());
}

#[tokio::test]
async fn test_vcp_adapter_trust_threshold_not_met() {
    let adapter = AileeEngineAdapter::new(1);

    let context = VcpExecutionContext::new(true, "us-east-1", "storage", 4000, true);

    // Very high trust threshold
    let result = adapter
        .execute_with_context(
            "Critical security analysis",
            TaskType::Analysis,
            0.98, // 98% trust threshold (very high)
            &context,
        )
        .await;

    // Should still succeed (consensus engine uses best available)
    assert!(result.is_ok());

    let result = result.unwrap();
    assert!(!result.final_output.is_empty());
}

#[tokio::test]
async fn test_vcp_adapter_connectivity_aware() {
    let adapter = AileeEngineAdapter::new(2);

    // Test 1: Online execution should use hybrid mode
    let online_context = VcpExecutionContext::new(true, "us-west", "gateway", 5000, true);

    let online_result = adapter
        .execute_with_context("Test online", TaskType::Chat, 0.5, &online_context)
        .await
        .unwrap();

    // When online with hybrid mode, could use remote adapters
    assert!(online_result.model_lineage.len() >= 2);

    // Test 2: Offline execution should use local-only mode
    let offline_context = VcpExecutionContext::new(false, "us-west", "gateway", 5000, true);

    let offline_result = adapter
        .execute_with_context("Test offline", TaskType::Chat, 0.5, &offline_context)
        .await
        .unwrap();

    // When offline, should only use local adapters
    assert_eq!(offline_result.model_lineage.len(), 2);
    assert!(offline_result.execution_metadata.was_offline);
}

#[tokio::test]
async fn test_vcp_adapter_multiple_task_types() {
    let adapter = AileeEngineAdapter::new(1);

    let context = VcpExecutionContext::new(false, "eu-west-1", "compute", 3000, true);

    // Test all task types
    for task_type in [TaskType::Chat, TaskType::Code, TaskType::Analysis] {
        let result = adapter
            .execute_with_context("Test prompt for task type", task_type, 0.5, &context)
            .await
            .unwrap();

        assert!(!result.final_output.is_empty());
        assert!(result.trust_score >= 0.0 && result.trust_score <= 1.0);
    }
}

#[tokio::test]
async fn test_vcp_adapter_lineage_tracking() {
    let adapter = AileeEngineAdapter::new(2);

    let context = VcpExecutionContext::new(false, "ap-northeast-1", "validator", 4000, true);

    let result = adapter
        .execute_with_context("Track model lineage", TaskType::Analysis, 0.6, &context)
        .await
        .unwrap();

    // Verify lineage tracking
    assert!(!result.model_lineage.is_empty());
    assert!(result.model_lineage.len() >= 2);

    // All models should have non-empty IDs
    for model_id in &result.model_lineage {
        assert!(!model_id.is_empty());
    }
}

#[tokio::test]
async fn test_vcp_adapter_open_internet_node() {
    let adapter = AileeEngineAdapter::new(2);

    let context = VcpExecutionContext::new(
        true, // Open internet nodes are online
        "us-west-1",
        "open_internet",
        5000,
        true,
    );

    let result = adapter
        .execute_with_context("Test open internet node", TaskType::Chat, 0.5, &context)
        .await
        .unwrap();

    assert!(!result.final_output.is_empty());
    assert!(result.trust_score >= 0.0 && result.trust_score <= 1.0);
    assert!(result.verify_hash());
}

#[tokio::test]
async fn test_vcp_adapter_any_node_type() {
    let adapter = AileeEngineAdapter::new(2);

    let context = VcpExecutionContext::new(
        true, // Universal nodes are online
        "eu-central-1",
        "any",
        5000,
        true,
    );

    let result = adapter
        .execute_with_context("Test universal any node", TaskType::Chat, 0.5, &context)
        .await
        .unwrap();

    assert!(!result.final_output.is_empty());
    assert!(result.trust_score >= 0.0 && result.trust_score <= 1.0);
    assert!(result.verify_hash());
}

#[tokio::test]
async fn test_vcp_adapter_all_node_types() {
    // Verify AILEE is connected to every supported node type
    let node_types = [
        "compute",
        "gateway",
        "storage",
        "validator",
        "open_internet",
        "any",
    ];

    for node_type in node_types {
        let adapter = AileeEngineAdapter::new(1);
        let context =
            VcpExecutionContext::new(false, "us-east-1", node_type, 3000, true);

        let result = adapter
            .execute_with_context(
                "Test for node type",
                TaskType::Chat,
                0.5,
                &context,
            )
            .await
            .unwrap_or_else(|e| panic!("AILEE execution failed for node_type='{node_type}': {e}"));

        assert!(
            !result.final_output.is_empty(),
            "Empty output for node_type='{node_type}'"
        );
    }
}
