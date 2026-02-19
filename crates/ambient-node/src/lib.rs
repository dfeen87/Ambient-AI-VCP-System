use serde::{Deserialize, Serialize};
use uuid::Uuid;

// VCP modules
pub mod connectivity;
pub mod gateway;
pub mod health;
pub mod offline;
pub mod reputation;
pub mod telemetry;

// Local observability (operator-only, privacy-preserving)
#[cfg(feature = "observability")]
pub mod observability;

// AILEE Integration - Clean boundary to external trust layer
pub mod ailee_integration;

// Re-export AILEE types from external crate (not re-implemented)
pub use ailee_trust_layer::{
    AileeMetric, AileeParams, AileeSample, ConsensusEngine, ExecutionMode, GenerationRequest,
    GenerationResult, LocalModelAdapter, ModelAdapter, ModelLocality, ModelOutput,
    RemoteModelAdapter, TaskType, TrustScores,
};

// Re-export VCP integration adapter
pub use ailee_integration::{AileeEngineAdapter, VcpExecutionContext};

// Re-export VCP types
pub use connectivity::*;
pub use gateway::*;
pub use health::*;
pub use offline::*;
pub use reputation::*;
pub use telemetry::*;

// Re-export observability types when feature is enabled
#[cfg(feature = "observability")]
pub use observability::*;

/// Unique identifier for an ambient node
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct NodeId {
    pub id: String,
    pub region: String,
    pub node_type: String,
}

impl NodeId {
    pub fn new(
        id: impl Into<String>,
        region: impl Into<String>,
        node_type: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            region: region.into(),
            node_type: node_type.into(),
        }
    }

    pub fn generate(region: impl Into<String>, node_type: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            region: region.into(),
            node_type: node_type.into(),
        }
    }
}

/// Safety policy configuration for circuit breakers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyPolicy {
    pub max_temperature_c: f64,
    pub max_latency_ms: f64,
    pub max_block_mb: f64,
    pub max_error_count: u32,
}

impl Default for SafetyPolicy {
    fn default() -> Self {
        Self {
            max_temperature_c: 85.0,
            max_latency_ms: 100.0,
            max_block_mb: 8.0,
            max_error_count: 25,
        }
    }
}

/// Main ambient node structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmbientNode {
    pub id: NodeId,
    pub telemetry: TelemetrySample,
    pub reputation: Reputation,
    safety_policy: SafetyPolicy,
    error_count: u32,
}

impl AmbientNode {
    pub fn new(id: NodeId, safety_policy: SafetyPolicy) -> Self {
        Self {
            id,
            telemetry: TelemetrySample::default(),
            reputation: Reputation::default(),
            safety_policy,
            error_count: 0,
        }
    }

    /// Ingest new telemetry data
    pub fn ingest_telemetry(&mut self, sample: TelemetrySample) {
        self.telemetry = sample;
    }

    /// Calculate overall health score (0.0 - 1.0)
    /// Weights: Bandwidth 40%, Latency 30%, Compute 20%, Reputation 10%
    pub fn health_score(&self) -> f64 {
        let bandwidth_score = self.telemetry.bandwidth_score();
        let latency_score = self.telemetry.latency_score();
        let compute_score = self.telemetry.compute_score();
        let reputation_score = self.reputation.score();

        (bandwidth_score * 0.4)
            + (latency_score * 0.3)
            + (compute_score * 0.2)
            + (reputation_score * 0.1)
    }

    /// Check if node is in safe mode (circuit breaker triggered)
    pub fn is_safe_mode(&self) -> bool {
        self.telemetry.temperature_c > self.safety_policy.max_temperature_c
            || self.telemetry.avg_latency_ms > self.safety_policy.max_latency_ms
            || self.error_count >= self.safety_policy.max_error_count
    }

    /// Update reputation based on task completion
    pub fn update_reputation(&mut self, success: bool, delta: f64) {
        if success {
            self.reputation.record_success(delta);
            self.error_count = 0;
        } else {
            self.reputation.record_failure(delta);
            self.error_count += 1;
        }
    }

    /// Get current safety policy
    pub fn safety_policy(&self) -> &SafetyPolicy {
        &self.safety_policy
    }

    /// Reset error count
    pub fn reset_errors(&mut self) {
        self.error_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_node_creation() {
        let node_id = NodeId::new("node-001", "us-west", "gateway");
        let policy = SafetyPolicy::default();
        let node = AmbientNode::new(node_id.clone(), policy);

        assert_eq!(node.id, node_id);
        assert!(!node.is_safe_mode());
    }

    #[test]
    fn test_health_score_calculation() {
        let node_id = NodeId::new("node-001", "us-west", "gateway");
        let mut node = AmbientNode::new(node_id, SafetyPolicy::default());

        let telemetry = TelemetrySample {
            bandwidth_mbps: 100.0,
            avg_latency_ms: 20.0,
            cpu_usage_percent: 50.0,
            memory_usage_percent: 60.0,
            temperature_c: 65.0,
            power_watts: 150.0,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        node.ingest_telemetry(telemetry);
        let score = node.health_score();

        assert!((0.0..=1.0).contains(&score));
    }

    #[test]
    fn test_safe_mode_temperature() {
        let node_id = NodeId::new("node-001", "us-west", "gateway");
        let mut node = AmbientNode::new(node_id, SafetyPolicy::default());

        let telemetry = TelemetrySample {
            temperature_c: 90.0, // Above threshold
            ..Default::default()
        };

        node.ingest_telemetry(telemetry);
        assert!(node.is_safe_mode());
    }

    #[test]
    fn test_reputation_update() {
        let node_id = NodeId::new("node-001", "us-west", "gateway");
        let mut node = AmbientNode::new(node_id, SafetyPolicy::default());

        // New node starts with 0.5 score
        assert_eq!(node.reputation.score(), 0.5);

        // After success, score should be 1.0
        node.update_reputation(true, 0.1);
        assert_eq!(node.reputation.score(), 1.0);

        // After one failure, score should drop
        node.update_reputation(false, 0.1);
        assert_eq!(node.reputation.score(), 0.5);

        // After more failures, score should be lower
        node.update_reputation(false, 0.1);
        assert!(node.reputation.score() < 0.5);
    }
}
