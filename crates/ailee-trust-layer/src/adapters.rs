//! Model adapters for generative execution

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::generation::TaskType;

/// Locality of a model (local or remote)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModelLocality {
    /// Model runs locally on this node
    Local,
    /// Model runs remotely (requires network)
    Remote,
}

/// Output from a model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelOutput {
    /// Generated text
    pub text: String,
    /// Model identifier
    pub model_id: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

impl ModelOutput {
    /// Create new model output
    pub fn new(
        text: impl Into<String>,
        model_id: impl Into<String>,
        confidence: f64,
        execution_time_ms: u64,
    ) -> Self {
        Self {
            text: text.into(),
            model_id: model_id.into(),
            confidence: confidence.clamp(0.0, 1.0),
            execution_time_ms,
        }
    }
}

/// Trait for model adapters
#[async_trait]
pub trait ModelAdapter: Send + Sync {
    /// Generate output from prompt
    async fn generate(&self, prompt: &str, task_type: TaskType) -> anyhow::Result<ModelOutput>;

    /// Get model identifier
    fn model_id(&self) -> &str;

    /// Get model locality
    fn locality(&self) -> ModelLocality;

    /// Check if adapter is available (e.g., network connectivity for remote)
    async fn is_available(&self) -> bool;
}

/// Local model adapter (stub implementation)
#[derive(Debug, Clone)]
pub struct LocalModelAdapter {
    model_id: String,
}

impl LocalModelAdapter {
    /// Create new local adapter
    pub fn new(model_id: impl Into<String>) -> Self {
        Self {
            model_id: model_id.into(),
        }
    }
}

#[async_trait]
impl ModelAdapter for LocalModelAdapter {
    async fn generate(&self, prompt: &str, task_type: TaskType) -> anyhow::Result<ModelOutput> {
        // Simulate local processing
        let start = std::time::Instant::now();

        // Simple stub: echo prompt with prefix based on task type
        let prefix = match task_type {
            TaskType::Chat => "Chat response: ",
            TaskType::Code => "// Generated code:\n",
            TaskType::Analysis => "Analysis: ",
        };

        let text = format!("{}{}", prefix, prompt);
        let elapsed = start.elapsed().as_millis() as u64;

        Ok(ModelOutput::new(
            text,
            self.model_id.clone(),
            0.85, // Stub confidence
            elapsed,
        ))
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn locality(&self) -> ModelLocality {
        ModelLocality::Local
    }

    async fn is_available(&self) -> bool {
        // Local models are always available
        true
    }
}

/// Remote model adapter (stub implementation with connectivity awareness)
#[derive(Debug, Clone)]
pub struct RemoteModelAdapter {
    model_id: String,
    is_online: bool,
}

impl RemoteModelAdapter {
    /// Create new remote adapter
    pub fn new(model_id: impl Into<String>, is_online: bool) -> Self {
        Self {
            model_id: model_id.into(),
            is_online,
        }
    }

    /// Update online status
    pub fn set_online(&mut self, is_online: bool) {
        self.is_online = is_online;
    }
}

#[async_trait]
impl ModelAdapter for RemoteModelAdapter {
    async fn generate(&self, prompt: &str, task_type: TaskType) -> anyhow::Result<ModelOutput> {
        if !self.is_online {
            anyhow::bail!("Remote model unavailable: offline");
        }

        // Simulate remote processing (would normally call external API)
        let start = std::time::Instant::now();

        let prefix = match task_type {
            TaskType::Chat => "Remote chat: ",
            TaskType::Code => "// Remote code:\n",
            TaskType::Analysis => "Remote analysis: ",
        };

        let text = format!("{}{}", prefix, prompt);
        let elapsed = start.elapsed().as_millis() as u64;

        Ok(ModelOutput::new(
            text,
            self.model_id.clone(),
            0.92, // Remote models typically have higher confidence
            elapsed,
        ))
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn locality(&self) -> ModelLocality {
        ModelLocality::Remote
    }

    async fn is_available(&self) -> bool {
        self.is_online
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_local_adapter_always_available() {
        let adapter = LocalModelAdapter::new("local-model-1");
        assert!(adapter.is_available().await);
        assert_eq!(adapter.model_id(), "local-model-1");
        assert_eq!(adapter.locality(), ModelLocality::Local);
    }

    #[tokio::test]
    async fn test_local_adapter_generation() {
        let adapter = LocalModelAdapter::new("local-model-1");
        let result = adapter
            .generate("test prompt", TaskType::Chat)
            .await
            .unwrap();
        assert!(result.text.contains("test prompt"));
        assert_eq!(result.model_id, "local-model-1");
        assert!(result.confidence > 0.0 && result.confidence <= 1.0);
    }

    #[tokio::test]
    async fn test_remote_adapter_online() {
        let adapter = RemoteModelAdapter::new("remote-model-1", true);
        assert!(adapter.is_available().await);
        assert_eq!(adapter.locality(), ModelLocality::Remote);

        let result = adapter
            .generate("test prompt", TaskType::Code)
            .await
            .unwrap();
        assert!(result.text.contains("test prompt"));
    }

    #[tokio::test]
    async fn test_remote_adapter_offline() {
        let adapter = RemoteModelAdapter::new("remote-model-1", false);
        assert!(!adapter.is_available().await);

        let result = adapter.generate("test prompt", TaskType::Chat).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_remote_adapter_status_change() {
        let mut adapter = RemoteModelAdapter::new("remote-model-1", false);
        assert!(!adapter.is_available().await);

        adapter.set_online(true);
        assert!(adapter.is_available().await);
    }

    #[test]
    fn test_model_output_confidence_clamping() {
        let output = ModelOutput::new("text", "model", 1.5, 100);
        assert_eq!(output.confidence, 1.0);

        let output2 = ModelOutput::new("text", "model", -0.5, 100);
        assert_eq!(output2.confidence, 0.0);
    }
}
