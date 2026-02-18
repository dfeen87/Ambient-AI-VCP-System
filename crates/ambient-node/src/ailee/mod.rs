//! AILEE Trust Layer Integration
//!
//! This module integrates with the external AILEE Trust Layer to provide
//! trust-scored, multi-model generative execution within Ambient AI VCP nodes.
//!
//! The AILEE Trust Layer is treated as an external dependency and logical authority
//! for all generative logic, trust scoring, consensus, and lineage tracking.

pub mod adapters;
pub mod consensus;
pub mod generation;
pub mod trust;

pub use adapters::{ModelAdapter, ModelLocality, ModelOutput};
pub use consensus::ConsensusEngine;
pub use generation::{ExecutionMode, GenerationRequest, GenerationResult, TaskType};
pub use trust::{ConsistencyScore, TrustScores};
