//! AILEE Trust Layer
//!
//! External generative intelligence engine providing trust-scored, multi-model
//! generative execution with consensus, lineage tracking, and deterministic behavior.
//!
//! This crate is substrate-agnostic and contains no knowledge of VCP internals.
//! It provides clean interfaces for:
//! - Trust scoring
//! - Consensus across multiple model outputs
//! - Lineage tracking
//! - Deterministic execution and replay
//!
//! ## Architecture
//!
//! The AILEE Trust Layer operates as a pure function: given inputs (request, adapters),
//! it produces deterministic outputs (result with trust scores and lineage).
//!
//! ### Core Components
//!
//! - **Generation**: Request/response structures and execution metadata
//! - **Adapters**: Model abstraction layer (local/remote)
//! - **Trust**: Trust scoring algorithms (confidence, safety, consistency)
//! - **Consensus**: Multi-model output selection and scoring
//!
//! ### Usage
//!
//! ```rust,no_run
//! use ailee_trust_layer::{
//!     ConsensusEngine, GenerationRequest, TaskType, ExecutionMode,
//!     LocalModelAdapter, ModelAdapter,
//! };
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Create the AILEE consensus engine
//! let engine = ConsensusEngine::new(2); // Require 2 models
//!
//! // Set up model adapters
//! let adapters: Vec<Box<dyn ModelAdapter>> = vec![
//!     Box::new(LocalModelAdapter::new("model-1")),
//!     Box::new(LocalModelAdapter::new("model-2")),
//! ];
//!
//! // Create a generation request
//! let request = GenerationRequest::new(
//!     "Explain quantum computing",
//!     TaskType::Chat,
//!     0.7, // Trust threshold
//!     ExecutionMode::Local,
//!     true, // Allow offline
//! );
//!
//! // Execute and get result
//! let result = engine.execute(&request, adapters).await?;
//!
//! println!("Output: {}", result.final_output);
//! println!("Trust score: {}", result.trust_score);
//! println!("Model lineage: {:?}", result.model_lineage);
//! # Ok(())
//! # }
//! ```

pub mod adapters;
pub mod consensus;
pub mod generation;
pub mod metric;
pub mod trust;

// Re-export commonly used types
pub use adapters::{
    LocalModelAdapter, ModelAdapter, ModelLocality, ModelOutput, RemoteModelAdapter,
};
pub use consensus::ConsensusEngine;
pub use generation::{
    ExecutionMetadata, ExecutionMode, GenerationRequest, GenerationResult, TaskType,
};
pub use metric::{AileeMetric, AileeParams, AileeSample};
pub use trust::{compute_trust_scores, ConsistencyScore, SafetyChecker, TrustScores};
