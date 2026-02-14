use serde::{Deserialize, Serialize};

/// Reputation tracking for nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reputation {
    pub completed_tasks: u64,
    pub failed_tasks: u64,
    pub disputes: u64,
    pub total_compute_time_ms: u64,
}

impl Default for Reputation {
    fn default() -> Self {
        Self {
            completed_tasks: 0,
            failed_tasks: 0,
            disputes: 0,
            total_compute_time_ms: 0,
        }
    }
}

impl Reputation {
    /// Calculate reputation score (0.0 - 1.0)
    pub fn score(&self) -> f64 {
        let total_tasks = self.completed_tasks + self.failed_tasks;
        
        if total_tasks == 0 {
            return 0.5; // Neutral score for new nodes
        }

        let success_rate = self.completed_tasks as f64 / total_tasks as f64;
        let dispute_penalty = (self.disputes as f64 * 0.05).min(0.3);
        
        (success_rate - dispute_penalty).max(0.0).min(1.0)
    }

    /// Record a successful task completion
    pub fn record_success(&mut self, delta: f64) {
        self.completed_tasks += 1;
        self.total_compute_time_ms += (delta * 1000.0) as u64;
    }

    /// Record a failed task
    pub fn record_failure(&mut self, _delta: f64) {
        self.failed_tasks += 1;
    }

    /// Record a dispute
    pub fn record_dispute(&mut self) {
        self.disputes += 1;
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        let total_tasks = self.completed_tasks + self.failed_tasks;
        if total_tasks == 0 {
            return 1.0;
        }
        self.completed_tasks as f64 / total_tasks as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_reputation() {
        let rep = Reputation::default();
        assert_eq!(rep.score(), 0.5); // Neutral for new nodes
    }

    #[test]
    fn test_reputation_score() {
        let mut rep = Reputation::default();
        
        // Record 10 successful tasks
        for _ in 0..10 {
            rep.record_success(1.0);
        }
        assert_eq!(rep.score(), 1.0);

        // Record 5 failures
        for _ in 0..5 {
            rep.record_failure(1.0);
        }
        let score = rep.score();
        assert!(score > 0.6 && score < 0.7);
    }

    #[test]
    fn test_dispute_penalty() {
        let mut rep = Reputation::default();
        
        // Perfect success rate
        for _ in 0..10 {
            rep.record_success(1.0);
        }
        
        // Add disputes
        rep.record_dispute();
        rep.record_dispute();
        
        let score = rep.score();
        assert!(score < 1.0); // Score reduced due to disputes
        assert!(score > 0.85); // But still high
    }

    #[test]
    fn test_success_rate() {
        let mut rep = Reputation::default();
        
        rep.record_success(1.0);
        rep.record_success(1.0);
        rep.record_failure(1.0);
        
        assert!((rep.success_rate() - 0.6667).abs() < 0.001);
    }
}
