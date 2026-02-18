//! Consensus engine for selecting final output from multiple models

use super::adapters::{ModelAdapter, ModelOutput};
use super::generation::{ExecutionMetadata, GenerationRequest, GenerationResult};
use super::trust::{compute_trust_scores, TrustScores};

/// Consensus engine for multi-model output selection
#[derive(Debug, Clone)]
pub struct ConsensusEngine {
    /// Minimum number of successful models required
    min_models: usize,
}

impl ConsensusEngine {
    /// Create new consensus engine
    pub fn new(min_models: usize) -> Self {
        Self {
            min_models: min_models.max(1),
        }
    }

    /// Execute generation request across multiple adapters
    pub async fn execute(
        &self,
        request: &GenerationRequest,
        adapters: Vec<Box<dyn ModelAdapter>>,
    ) -> anyhow::Result<GenerationResult> {
        let start = std::time::Instant::now();

        // Filter adapters based on availability and execution mode
        let available_adapters = self.filter_adapters(adapters, request).await;

        if available_adapters.is_empty() {
            anyhow::bail!("No adapters available for execution");
        }

        // Execute generation across all available adapters
        let outputs = self.generate_all(&available_adapters, request).await;

        if outputs.is_empty() {
            anyhow::bail!("All models failed to generate output");
        }

        // Check if we meet minimum model requirement
        if outputs.len() < self.min_models {
            anyhow::bail!(
                "Insufficient models succeeded ({} < {})",
                outputs.len(),
                self.min_models
            );
        }

        // Compute trust scores for each output
        let scored_outputs = self.score_outputs(&outputs);

        // Select final output based on consensus
        let (final_output, trust_score) =
            self.select_output(&scored_outputs, request.trust_threshold)?;

        // Build result metadata
        let elapsed = start.elapsed().as_millis() as u64;
        let was_offline = outputs.iter().all(|o| {
            available_adapters
                .iter()
                .find(|a| a.model_id() == o.model_id)
                .map(|a| !matches!(a.locality(), super::adapters::ModelLocality::Remote))
                .unwrap_or(true)
        });

        let model_lineage = outputs.iter().map(|o| o.model_id.clone()).collect();

        let metadata = ExecutionMetadata::new(
            available_adapters.len(),
            outputs.len(),
            was_offline,
            elapsed,
        );

        Ok(GenerationResult::new(
            final_output.text.clone(),
            trust_score,
            model_lineage,
            metadata,
            request.hash(),
        ))
    }

    /// Filter adapters based on availability and execution mode
    async fn filter_adapters(
        &self,
        adapters: Vec<Box<dyn ModelAdapter>>,
        request: &GenerationRequest,
    ) -> Vec<Box<dyn ModelAdapter>> {
        let mut filtered = Vec::new();

        for adapter in adapters {
            if !adapter.is_available().await {
                continue;
            }

            let locality = adapter.locality();

            let should_include = match request.execution_mode {
                super::generation::ExecutionMode::Local => {
                    matches!(locality, super::adapters::ModelLocality::Local)
                }
                super::generation::ExecutionMode::Remote => {
                    matches!(locality, super::adapters::ModelLocality::Remote)
                }
                super::generation::ExecutionMode::Hybrid => true,
            };

            if should_include {
                filtered.push(adapter);
            }
        }

        filtered
    }

    /// Generate outputs from all adapters
    async fn generate_all(
        &self,
        adapters: &[Box<dyn ModelAdapter>],
        request: &GenerationRequest,
    ) -> Vec<ModelOutput> {
        let mut outputs = Vec::new();

        for adapter in adapters {
            match adapter.generate(&request.prompt, request.task_type).await {
                Ok(output) => outputs.push(output),
                Err(_) => {
                    // Gracefully handle individual adapter failures
                    continue;
                }
            }
        }

        outputs
    }

    /// Compute trust scores for all outputs
    fn score_outputs(&self, outputs: &[ModelOutput]) -> Vec<(ModelOutput, TrustScores)> {
        outputs
            .iter()
            .map(|output| {
                // Get all peers (other outputs)
                let peers: Vec<ModelOutput> = outputs
                    .iter()
                    .filter(|o| o.model_id != output.model_id)
                    .cloned()
                    .collect();

                let scores = compute_trust_scores(output, &peers);
                (output.clone(), scores)
            })
            .collect()
    }

    /// Select final output based on trust scores
    fn select_output(
        &self,
        scored_outputs: &[(ModelOutput, TrustScores)],
        trust_threshold: f64,
    ) -> anyhow::Result<(ModelOutput, f64)> {
        // Filter outputs that meet trust threshold
        let mut valid_outputs: Vec<(ModelOutput, f64)> = scored_outputs
            .iter()
            .map(|(output, scores)| (output.clone(), scores.overall_score()))
            .filter(|(_, score)| *score >= trust_threshold)
            .collect();

        if valid_outputs.is_empty() {
            // If no outputs meet threshold, select best available but warn
            if let Some((output, scores)) = scored_outputs.iter().max_by(|a, b| {
                a.1.overall_score()
                    .partial_cmp(&b.1.overall_score())
                    .unwrap_or(std::cmp::Ordering::Equal)
            }) {
                let score = scores.overall_score();
                tracing::warn!(
                    "No outputs met trust threshold {}, using best available with score {}",
                    trust_threshold,
                    score
                );
                return Ok((output.clone(), score));
            }

            anyhow::bail!("No valid outputs available");
        }

        // Sort by score descending
        valid_outputs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Return highest scoring output
        Ok(valid_outputs[0].clone())
    }
}

impl Default for ConsensusEngine {
    fn default() -> Self {
        Self::new(1)
    }
}

#[cfg(test)]
mod tests {
    use super::super::adapters::{LocalModelAdapter, RemoteModelAdapter};
    use super::super::generation::{ExecutionMode, TaskType};
    use super::*;

    #[tokio::test]
    async fn test_consensus_with_local_adapters() {
        let engine = ConsensusEngine::new(2);

        let adapters: Vec<Box<dyn ModelAdapter>> = vec![
            Box::new(LocalModelAdapter::new("local-1")),
            Box::new(LocalModelAdapter::new("local-2")),
        ];

        let request = GenerationRequest::new(
            "test prompt",
            TaskType::Chat,
            0.5,
            ExecutionMode::Local,
            true,
        );

        let result = engine.execute(&request, adapters).await.unwrap();

        assert!(!result.final_output.is_empty());
        assert_eq!(result.model_lineage.len(), 2);
        assert!(result.trust_score >= 0.0 && result.trust_score <= 1.0);
        assert!(result.verify_hash());
    }

    #[tokio::test]
    async fn test_consensus_hybrid_mode() {
        let engine = ConsensusEngine::new(1);

        let adapters: Vec<Box<dyn ModelAdapter>> = vec![
            Box::new(LocalModelAdapter::new("local-1")),
            Box::new(RemoteModelAdapter::new("remote-1", true)),
        ];

        let request = GenerationRequest::new(
            "test prompt",
            TaskType::Code,
            0.5,
            ExecutionMode::Hybrid,
            true,
        );

        let result = engine.execute(&request, adapters).await.unwrap();
        assert_eq!(result.model_lineage.len(), 2);
    }

    #[tokio::test]
    async fn test_consensus_offline_mode() {
        let engine = ConsensusEngine::new(1);

        let adapters: Vec<Box<dyn ModelAdapter>> = vec![
            Box::new(LocalModelAdapter::new("local-1")),
            Box::new(RemoteModelAdapter::new("remote-1", false)), // Offline
        ];

        let request = GenerationRequest::new(
            "test prompt",
            TaskType::Chat,
            0.5,
            ExecutionMode::Hybrid,
            true,
        );

        let result = engine.execute(&request, adapters).await.unwrap();
        // Should only use local adapter
        assert_eq!(result.model_lineage.len(), 1);
        assert_eq!(result.model_lineage[0], "local-1");
    }

    #[tokio::test]
    async fn test_consensus_insufficient_models() {
        let engine = ConsensusEngine::new(3); // Require 3 models

        let adapters: Vec<Box<dyn ModelAdapter>> =
            vec![Box::new(LocalModelAdapter::new("local-1"))];

        let request = GenerationRequest::new(
            "test prompt",
            TaskType::Chat,
            0.5,
            ExecutionMode::Local,
            true,
        );

        let result = engine.execute(&request, adapters).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_consensus_no_adapters_available() {
        let engine = ConsensusEngine::new(1);

        let adapters: Vec<Box<dyn ModelAdapter>> = vec![
            Box::new(RemoteModelAdapter::new("remote-1", false)), // Offline
        ];

        let request = GenerationRequest::new(
            "test prompt",
            TaskType::Chat,
            0.5,
            ExecutionMode::Remote,
            false, // Don't allow offline
        );

        let result = engine.execute(&request, adapters).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execution_metadata() {
        let engine = ConsensusEngine::new(1);

        let adapters: Vec<Box<dyn ModelAdapter>> = vec![
            Box::new(LocalModelAdapter::new("local-1")),
            Box::new(LocalModelAdapter::new("local-2")),
        ];

        let request = GenerationRequest::new(
            "test prompt",
            TaskType::Analysis,
            0.5,
            ExecutionMode::Local,
            true,
        );

        let result = engine.execute(&request, adapters).await.unwrap();

        assert_eq!(result.execution_metadata.models_consulted, 2);
        assert_eq!(result.execution_metadata.models_succeeded, 2);
        assert!(result.execution_metadata.was_offline);
    }
}
