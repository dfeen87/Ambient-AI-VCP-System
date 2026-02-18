//! Generative execution pipeline for AILEE Trust Layer

use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

/// Type of generative task
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TaskType {
    /// Chat-based interaction
    Chat,
    /// Code generation or analysis
    Code,
    /// General analysis task
    Analysis,
}

/// Execution mode for generation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionMode {
    /// Use only local models
    Local,
    /// Use only remote models (when available)
    Remote,
    /// Use both local and remote models
    Hybrid,
}

/// Request for generative execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRequest {
    /// The prompt to generate from
    pub prompt: String,
    /// Type of task
    pub task_type: TaskType,
    /// Minimum trust threshold (0.0 - 1.0)
    pub trust_threshold: f64,
    /// Execution mode
    pub execution_mode: ExecutionMode,
    /// Whether to allow offline execution
    pub allow_offline: bool,
}

impl GenerationRequest {
    /// Create a new generation request
    pub fn new(
        prompt: impl Into<String>,
        task_type: TaskType,
        trust_threshold: f64,
        execution_mode: ExecutionMode,
        allow_offline: bool,
    ) -> Self {
        Self {
            prompt: prompt.into(),
            task_type,
            trust_threshold: trust_threshold.clamp(0.0, 1.0),
            execution_mode,
            allow_offline,
        }
    }

    /// Compute cryptographic hash of the request
    pub fn hash(&self) -> String {
        let mut hasher = Sha3_256::new();
        hasher.update(self.prompt.as_bytes());
        hasher.update(format!("{:?}", self.task_type).as_bytes());
        hasher.update(self.trust_threshold.to_le_bytes());
        hasher.update(format!("{:?}", self.execution_mode).as_bytes());
        hasher.update([self.allow_offline as u8]);
        format!("{:x}", hasher.finalize())
    }
}

/// Result of generative execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationResult {
    /// Final output selected by consensus
    pub final_output: String,
    /// Overall trust score (0.0 - 1.0)
    pub trust_score: f64,
    /// Lineage of models that contributed
    pub model_lineage: Vec<String>,
    /// Execution metadata
    pub execution_metadata: ExecutionMetadata,
    /// Cryptographic hash of input request
    pub input_hash: String,
    /// Cryptographic hash of output
    pub output_hash: String,
}

impl GenerationResult {
    /// Create a new generation result
    pub fn new(
        final_output: String,
        trust_score: f64,
        model_lineage: Vec<String>,
        execution_metadata: ExecutionMetadata,
        input_hash: String,
    ) -> Self {
        let output_hash = Self::compute_output_hash(&final_output);
        Self {
            final_output,
            trust_score: trust_score.clamp(0.0, 1.0),
            model_lineage,
            execution_metadata,
            input_hash,
            output_hash,
        }
    }

    /// Compute hash of output
    fn compute_output_hash(output: &str) -> String {
        let mut hasher = Sha3_256::new();
        hasher.update(output.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Verify hash matches output
    pub fn verify_hash(&self) -> bool {
        self.output_hash == Self::compute_output_hash(&self.final_output)
    }
}

/// Metadata about execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    /// Number of models consulted
    pub models_consulted: usize,
    /// Number of models that succeeded
    pub models_succeeded: usize,
    /// Whether execution was offline
    pub was_offline: bool,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Timestamp of execution (Unix epoch seconds)
    pub timestamp: u64,
}

impl ExecutionMetadata {
    /// Create new execution metadata
    pub fn new(
        models_consulted: usize,
        models_succeeded: usize,
        was_offline: bool,
        execution_time_ms: u64,
    ) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time should be after UNIX epoch")
            .as_secs();
        Self {
            models_consulted,
            models_succeeded,
            was_offline,
            execution_time_ms,
            timestamp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generation_request_creation() {
        let req = GenerationRequest::new(
            "test prompt",
            TaskType::Chat,
            0.8,
            ExecutionMode::Hybrid,
            true,
        );
        assert_eq!(req.prompt, "test prompt");
        assert_eq!(req.task_type, TaskType::Chat);
        assert_eq!(req.trust_threshold, 0.8);
        assert_eq!(req.execution_mode, ExecutionMode::Hybrid);
        assert!(req.allow_offline);
    }

    #[test]
    fn test_trust_threshold_clamping() {
        let req = GenerationRequest::new(
            "test",
            TaskType::Code,
            1.5, // Over limit
            ExecutionMode::Local,
            false,
        );
        assert_eq!(req.trust_threshold, 1.0);

        let req2 = GenerationRequest::new(
            "test",
            TaskType::Code,
            -0.5, // Under limit
            ExecutionMode::Local,
            false,
        );
        assert_eq!(req2.trust_threshold, 0.0);
    }

    #[test]
    fn test_request_hash_deterministic() {
        let req1 = GenerationRequest::new(
            "test prompt",
            TaskType::Chat,
            0.8,
            ExecutionMode::Hybrid,
            true,
        );
        let req2 = GenerationRequest::new(
            "test prompt",
            TaskType::Chat,
            0.8,
            ExecutionMode::Hybrid,
            true,
        );
        assert_eq!(req1.hash(), req2.hash());
    }

    #[test]
    fn test_request_hash_different() {
        let req1 = GenerationRequest::new(
            "test prompt 1",
            TaskType::Chat,
            0.8,
            ExecutionMode::Hybrid,
            true,
        );
        let req2 = GenerationRequest::new(
            "test prompt 2",
            TaskType::Chat,
            0.8,
            ExecutionMode::Hybrid,
            true,
        );
        assert_ne!(req1.hash(), req2.hash());
    }

    #[test]
    fn test_generation_result_hash_verification() {
        let metadata = ExecutionMetadata::new(2, 2, false, 100);
        let result = GenerationResult::new(
            "output text".to_string(),
            0.9,
            vec!["model1".to_string()],
            metadata,
            "input_hash".to_string(),
        );
        assert!(result.verify_hash());
    }

    #[test]
    fn test_generation_result_trust_score_clamping() {
        let metadata = ExecutionMetadata::new(1, 1, false, 50);
        let result = GenerationResult::new(
            "output".to_string(),
            1.5, // Over limit
            vec![],
            metadata,
            "hash".to_string(),
        );
        assert_eq!(result.trust_score, 1.0);
    }
}
