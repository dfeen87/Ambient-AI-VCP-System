use serde::{Deserialize, Serialize};

/// Reward distribution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardDistribution {
    pub task_id: String,
    pub node_id: String,
    pub amount: f64,
    pub timestamp: u64,
}

impl RewardDistribution {
    pub fn new(task_id: String, node_id: String, amount: f64) -> Self {
        Self {
            task_id,
            node_id,
            amount,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

/// Settlement manager for tracking rewards
pub struct SettlementManager {
    distributions: Vec<RewardDistribution>,
}

impl SettlementManager {
    pub fn new() -> Self {
        Self {
            distributions: Vec::new(),
        }
    }

    pub fn record_reward(&mut self, distribution: RewardDistribution) {
        self.distributions.push(distribution);
    }

    pub fn get_node_rewards(&self, node_id: &str) -> Vec<&RewardDistribution> {
        self.distributions
            .iter()
            .filter(|d| d.node_id == node_id)
            .collect()
    }

    pub fn total_rewards_for_node(&self, node_id: &str) -> f64 {
        self.distributions
            .iter()
            .filter(|d| d.node_id == node_id)
            .map(|d| d.amount)
            .sum()
    }
}

impl Default for SettlementManager {
    fn default() -> Self {
        Self::new()
    }
}
