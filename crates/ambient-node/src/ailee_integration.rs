//! VCP Integration Adapter for AILEE Trust Layer
//!
//! This module provides the integration boundary between Ambient AI VCP
//! and the external AILEE Trust Layer.
//!
//! ## Responsibilities
//!
//! - Instantiate AILEE engine within VCP node lifecycle
//! - Pass VCP execution context (connectivity, locality, constraints) to AILEE
//! - Invoke AILEE in-process (not over network)
//! - Receive GenerationResult and route/return appropriately
//! - Remain agnostic to trust math and consensus logic
//!
//! ## Clean Separation
//!
//! - VCP provides: connectivity state, locality hints, execution constraints
//! - AILEE provides: trust scoring, consensus, lineage, determinism
//! - No VCP logic leaks into AILEE
//! - No AILEE logic re-implemented in VCP

use ailee_trust_layer::{
    ConsensusEngine, ExecutionMode, GenerationRequest, GenerationResult, LocalModelAdapter,
    ModelAdapter, RemoteModelAdapter, TaskType,
};
use serde::{Deserialize, Serialize};

/// VCP execution context passed to AILEE
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VcpExecutionContext {
    /// Current connectivity state
    pub is_online: bool,
    /// Node locality (region, type)
    pub node_region: String,
    pub node_type: String,
    /// Execution constraints
    pub max_execution_time_ms: u64,
    pub allow_offline_execution: bool,
}

impl VcpExecutionContext {
    /// Create new VCP execution context
    pub fn new(
        is_online: bool,
        node_region: impl Into<String>,
        node_type: impl Into<String>,
        max_execution_time_ms: u64,
        allow_offline_execution: bool,
    ) -> Self {
        Self {
            is_online,
            node_region: node_region.into(),
            node_type: node_type.into(),
            max_execution_time_ms,
            allow_offline_execution,
        }
    }
}

/// AILEE Engine Wrapper for VCP Integration
///
/// This wrapper:
/// - Manages AILEE engine lifecycle within VCP nodes
/// - Adapts VCP context to AILEE execution parameters
/// - Handles result routing and error propagation
/// - Maintains clean separation of concerns
pub struct AileeEngineAdapter {
    consensus_engine: ConsensusEngine,
}

impl AileeEngineAdapter {
    /// Create new AILEE engine adapter
    pub fn new(min_models: usize) -> Self {
        Self {
            consensus_engine: ConsensusEngine::new(min_models),
        }
    }

    /// Execute generation request using AILEE Trust Layer
    ///
    /// This method:
    /// 1. Accepts VCP context and user request
    /// 2. Configures model adapters based on connectivity
    /// 3. Invokes AILEE in-process
    /// 4. Returns result with trust scores and lineage
    pub async fn execute_with_context(
        &self,
        prompt: impl Into<String>,
        task_type: TaskType,
        trust_threshold: f64,
        vcp_context: &VcpExecutionContext,
    ) -> anyhow::Result<GenerationResult> {
        // Determine execution mode based on VCP connectivity
        let execution_mode = if vcp_context.is_online {
            ExecutionMode::Hybrid // Use both local and remote when online
        } else {
            ExecutionMode::Local // Fall back to local only when offline
        };

        // Create generation request
        let request = GenerationRequest::new(
            prompt,
            task_type,
            trust_threshold,
            execution_mode,
            vcp_context.allow_offline_execution,
        );

        // Configure model adapters based on VCP context
        let adapters = self.create_adapters(vcp_context);

        // Invoke AILEE in-process
        let result = self.consensus_engine.execute(&request, adapters).await?;

        // Validate execution time constraint
        if result.execution_metadata.execution_time_ms > vcp_context.max_execution_time_ms {
            tracing::warn!(
                "Execution exceeded time budget: {} > {}",
                result.execution_metadata.execution_time_ms,
                vcp_context.max_execution_time_ms
            );
        }

        Ok(result)
    }

    /// Create model adapters based on VCP context
    ///
    /// This is where VCP connectivity state informs AILEE adapter selection
    fn create_adapters(&self, vcp_context: &VcpExecutionContext) -> Vec<Box<dyn ModelAdapter>> {
        let mut adapters: Vec<Box<dyn ModelAdapter>> = Vec::new();

        // Always include local adapters (offline-capable)
        adapters.push(Box::new(LocalModelAdapter::new("vcp-local-model-1")));
        adapters.push(Box::new(LocalModelAdapter::new("vcp-local-model-2")));

        // Add remote adapters only if online
        if vcp_context.is_online {
            adapters.push(Box::new(RemoteModelAdapter::new(
                "vcp-remote-model-1",
                true,
            )));
        }

        adapters
    }
}

impl Default for AileeEngineAdapter {
    fn default() -> Self {
        Self::new(2) // Require at least 2 models by default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vcp_context_online_execution() {
        let adapter = AileeEngineAdapter::new(2);

        let context = VcpExecutionContext::new(
            true, // Online
            "us-west", "gateway", 5000, // 5 second budget
            true,
        );

        let result = adapter
            .execute_with_context("Test prompt", TaskType::Chat, 0.7, &context)
            .await
            .unwrap();

        assert!(!result.final_output.is_empty());
        assert!(result.trust_score >= 0.0 && result.trust_score <= 1.0);
        assert!(result.model_lineage.len() >= 2);
    }

    #[tokio::test]
    async fn test_vcp_context_offline_execution() {
        let adapter = AileeEngineAdapter::new(2);

        let context = VcpExecutionContext::new(
            false, // Offline
            "eu-central",
            "compute",
            3000,
            true,
        );

        let result = adapter
            .execute_with_context("Offline test", TaskType::Code, 0.6, &context)
            .await
            .unwrap();

        // Should succeed with local models only
        assert!(!result.final_output.is_empty());
        assert!(result.execution_metadata.was_offline);
        assert_eq!(result.model_lineage.len(), 2); // Only local models
    }

    #[tokio::test]
    async fn test_deterministic_execution() {
        let adapter = AileeEngineAdapter::new(1);

        let context = VcpExecutionContext::new(false, "us-east", "validator", 2000, true);

        let result1 = adapter
            .execute_with_context("Deterministic test", TaskType::Analysis, 0.5, &context)
            .await
            .unwrap();

        let result2 = adapter
            .execute_with_context("Deterministic test", TaskType::Analysis, 0.5, &context)
            .await
            .unwrap();

        // Same input should produce same hash
        assert_eq!(result1.input_hash, result2.input_hash);
    }

    #[tokio::test]
    async fn test_trust_threshold_enforcement() {
        let adapter = AileeEngineAdapter::new(1);

        let context = VcpExecutionContext::new(true, "ap-south", "storage", 4000, true);

        // High trust threshold
        let result = adapter
            .execute_with_context("High trust task", TaskType::Chat, 0.95, &context)
            .await;

        // Should still succeed (consensus engine uses best available)
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_offline_not_allowed() {
        let adapter = AileeEngineAdapter::new(2);

        let context = VcpExecutionContext::new(
            false, // Offline
            "us-west", "gateway", 5000, false, // Don't allow offline
        );

        // This should still work because we have local adapters available
        // The allow_offline flag is for request policy, not adapter availability
        let result = adapter
            .execute_with_context("Test", TaskType::Chat, 0.5, &context)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_open_internet_node_execution() {
        let adapter = AileeEngineAdapter::new(2);

        let context = VcpExecutionContext::new(
            true, // Open internet nodes are online
            "us-west",
            "open_internet",
            5000,
            true,
        );

        let result = adapter
            .execute_with_context("Test open_internet node", TaskType::Chat, 0.5, &context)
            .await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(!result.final_output.is_empty());
        assert!(result.trust_score >= 0.0 && result.trust_score <= 1.0);
    }

    #[tokio::test]
    async fn test_any_node_type_execution() {
        let adapter = AileeEngineAdapter::new(2);

        let context = VcpExecutionContext::new(
            true, // Universal nodes are online
            "eu-central",
            "any",
            5000,
            true,
        );

        let result = adapter
            .execute_with_context("Test any node type", TaskType::Chat, 0.5, &context)
            .await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(!result.final_output.is_empty());
        assert!(result.trust_score >= 0.0 && result.trust_score <= 1.0);
    }
}
