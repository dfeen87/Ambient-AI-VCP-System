use anyhow::Result;
use bitcoin::{
    absolute::LockTime, transaction::Version, Amount, Network, ScriptBuf, Transaction, TxOut,
};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

/// Bitcoin transaction commitment for ZK proofs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofCommitment {
    pub proof_hash: String,
    pub task_id: String,
    pub timestamp: u64,
    pub merkle_root: String,
}

impl ProofCommitment {
    pub fn new(proof_hash: String, task_id: String, timestamp: u64) -> Self {
        Self {
            proof_hash: proof_hash.clone(),
            task_id,
            timestamp,
            merkle_root: Self::compute_merkle_root(&proof_hash),
        }
    }

    /// Compute Merkle root for commitment
    fn compute_merkle_root(proof_hash: &str) -> String {
        let mut hasher = Sha3_256::new();
        hasher.update(proof_hash.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Create OP_RETURN script with commitment data
    pub fn to_op_return_script(&self) -> ScriptBuf {
        let commitment_data = format!("AMBIENT:{}", self.merkle_root);
        let bytes = commitment_data.into_bytes();
        
        // Create a simple OP_RETURN script manually
        let mut script_bytes = vec![bitcoin::opcodes::all::OP_RETURN.to_u8()];
        
        // Add push opcode based on data length
        if bytes.len() <= 75 {
            script_bytes.push(bytes.len() as u8);
        } else {
            // For longer data, use OP_PUSHDATA1
            script_bytes.push(bitcoin::opcodes::all::OP_PUSHDATA1.to_u8());
            script_bytes.push(bytes.len() as u8);
        }
        
        script_bytes.extend_from_slice(&bytes);
        ScriptBuf::from(script_bytes)
    }

    /// Verify commitment matches proof
    pub fn verify(&self, proof_hash: &str) -> bool {
        let expected_root = Self::compute_merkle_root(proof_hash);
        self.merkle_root == expected_root
    }
}

/// Bitcoin commitment transaction builder
pub struct CommitmentTxBuilder {
    network: Network,
}

impl CommitmentTxBuilder {
    pub fn new(network: Network) -> Self {
        Self { network }
    }

    /// Create a commitment transaction
    pub fn build_commitment_tx(
        &self,
        commitment: &ProofCommitment,
        _fee_sats: u64,
    ) -> Result<Transaction> {
        // Create OP_RETURN output with commitment
        let op_return_output = TxOut {
            value: Amount::ZERO,
            script_pubkey: commitment.to_op_return_script(),
        };

        // Build transaction
        let tx = Transaction {
            version: Version::TWO,
            lock_time: LockTime::ZERO,
            input: vec![],  // Would be populated with actual UTXOs
            output: vec![op_return_output],
        };

        Ok(tx)
    }

    /// Extract commitment from transaction
    pub fn extract_commitment(&self, tx: &Transaction) -> Option<String> {
        for output in &tx.output {
            if output.script_pubkey.is_op_return() {
                // Extract commitment data
                let script_bytes = output.script_pubkey.as_bytes();
                if script_bytes.len() > 2 {
                    // Skip OP_RETURN (0x6a) and length byte
                    let data = &script_bytes[2..];
                    if let Ok(s) = std::str::from_utf8(data) {
                        if s.starts_with("AMBIENT:") {
                            return Some(s[8..].to_string());
                        }
                    }
                }
            }
        }
        None
    }
}

impl Default for CommitmentTxBuilder {
    fn default() -> Self {
        Self::new(Network::Testnet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proof_commitment() {
        let proof_hash = "abc123".to_string();
        let commitment = ProofCommitment::new(proof_hash.clone(), "task-1".to_string(), 1234567890);

        assert!(commitment.verify(&proof_hash));
        assert!(!commitment.verify("different_hash"));
    }

    #[test]
    fn test_commitment_tx_builder() {
        let builder = CommitmentTxBuilder::default();
        let commitment = ProofCommitment::new(
            "test_proof_hash".to_string(),
            "task-1".to_string(),
            1234567890,
        );

        let tx = builder.build_commitment_tx(&commitment, 1000).unwrap();

        assert_eq!(tx.output.len(), 1);
        assert!(tx.output[0].script_pubkey.is_op_return());

        // Extract and verify commitment
        let extracted = builder.extract_commitment(&tx);
        assert!(extracted.is_some());
        assert_eq!(extracted.unwrap(), commitment.merkle_root);
    }
}
