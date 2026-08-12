// Copyright (c) Don Michael Feeney Jr.
// Licensed under the MIT License.

/// Health scoring system for nodes
pub trait HealthScorer {
    fn health_score(&self) -> f64;
    fn is_healthy(&self) -> bool {
        self.health_score() > 0.5
    }
}
