use serde::{Deserialize, Serialize};

pub mod prover;
pub mod verifier;

pub use prover::*;
pub use verifier::*;

/// ZK Proof representation (Production Groth16 implementation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZKProof {
    pub proof_data: Vec<u8>,
    pub public_inputs: Vec<u8>,
    pub circuit_id: String,
    pub proof_system: String,
}

impl ZKProof {
    pub fn new(proof_data: Vec<u8>, public_inputs: Vec<u8>, circuit_id: String) -> Self {
        Self {
            proof_data,
            public_inputs,
            circuit_id,
            proof_system: "groth16-bn254".to_string(),
        }
    }

    /// Get proof size in bytes
    pub fn size(&self) -> usize {
        self.proof_data.len()
    }
}

/// Proving key for ZK circuit
#[derive(Debug, Clone)]
pub struct ProvingKey {
    pub key_data: Vec<u8>,
}

/// Verification key for ZK circuit
#[derive(Debug, Clone)]
pub struct VerificationKey {
    pub key_data: Vec<u8>,
}

/// Execution trace from WASM engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTrace {
    pub module_hash: String,
    pub function_name: String,
    pub inputs: Vec<u8>,
    pub outputs: Vec<u8>,
    pub execution_time_ms: u64,
    pub gas_used: u64,
    pub timestamp: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proof_creation() {
        let proof = ZKProof::new(
            vec![1, 2, 3, 4],
            vec![5, 6, 7, 8],
            "test-circuit".to_string(),
        );
        assert_eq!(proof.size(), 4);
        assert_eq!(proof.circuit_id, "test-circuit");
    }
}
