//! Integration tests for AILEE Trust Layer

use ambient_node::ailee::{
    adapters::{LocalModelAdapter, ModelAdapter, RemoteModelAdapter},
    consensus::ConsensusEngine,
    generation::{ExecutionMode, GenerationRequest, TaskType},
};

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
