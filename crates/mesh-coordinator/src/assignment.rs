use serde::{Deserialize, Serialize};

/// Task assignment strategy
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum TaskAssignmentStrategy {
    /// Select based on weighted health scores
    #[default]
    Weighted,
    /// Round-robin selection
    RoundRobin,
    /// Select least loaded node
    LeastLoaded,
    /// Select lowest latency node
    LatencyAware,
}
