use serde::{Deserialize, Serialize};

/// Task assignment strategy
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TaskAssignmentStrategy {
    /// Select based on weighted health scores
    Weighted,
    /// Round-robin selection
    RoundRobin,
    /// Select least loaded node
    LeastLoaded,
    /// Select lowest latency node
    LatencyAware,
}

impl Default for TaskAssignmentStrategy {
    fn default() -> Self {
        Self::Weighted
    }
}
