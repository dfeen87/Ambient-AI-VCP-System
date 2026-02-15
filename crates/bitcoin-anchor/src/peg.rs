use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// State peg for Layer-2 settlement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatePeg {
    pub state_root: String,
    pub block_height: u64,
    pub commitments: Vec<String>,
}

impl StatePeg {
    pub fn new(state_root: String, block_height: u64) -> Self {
        Self {
            state_root,
            block_height,
            commitments: Vec::new(),
        }
    }

    pub fn add_commitment(&mut self, commitment_hash: String) {
        self.commitments.push(commitment_hash);
    }
}

/// Settlement manager for Bitcoin Layer-2
pub struct SettlementManager {
    pegs: HashMap<u64, StatePeg>,
    current_height: u64,
}

impl SettlementManager {
    pub fn new() -> Self {
        Self {
            pegs: HashMap::new(),
            current_height: 0,
        }
    }

    /// Create a new state peg
    pub fn create_peg(&mut self, state_root: String) -> Result<StatePeg> {
        let peg = StatePeg::new(state_root, self.current_height);
        self.pegs.insert(self.current_height, peg.clone());
        self.current_height += 1;
        Ok(peg)
    }

    /// Get a state peg by height
    pub fn get_peg(&self, height: u64) -> Option<&StatePeg> {
        self.pegs.get(&height)
    }

    /// Add commitment to current peg
    pub fn add_commitment_to_current(&mut self, commitment_hash: String) -> Result<()> {
        if self.current_height == 0 {
            anyhow::bail!("No active peg");
        }

        let height = self.current_height - 1;
        if let Some(peg) = self.pegs.get_mut(&height) {
            peg.add_commitment(commitment_hash);
            Ok(())
        } else {
            anyhow::bail!("Peg not found")
        }
    }

    /// Verify a commitment exists in a peg
    pub fn verify_commitment(&self, height: u64, commitment_hash: &str) -> bool {
        self.pegs
            .get(&height)
            .map(|peg| peg.commitments.contains(&commitment_hash.to_string()))
            .unwrap_or(false)
    }
}

impl Default for SettlementManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settlement_manager() {
        let mut manager = SettlementManager::new();

        let peg = manager.create_peg("state_root_1".to_string()).unwrap();
        assert_eq!(peg.block_height, 0);

        manager.add_commitment_to_current("commitment_1".to_string()).unwrap();

        assert!(manager.verify_commitment(0, "commitment_1"));
        assert!(!manager.verify_commitment(0, "commitment_2"));
    }
}
