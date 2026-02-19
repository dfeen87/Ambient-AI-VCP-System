use crate::{VerificationKey, ZKProof};
use ark_bn254::{Bn254, Fr};
use ark_groth16::{Groth16, Proof, VerifyingKey as ArkVerifyingKey};
use ark_serialize::CanonicalDeserialize;
use ark_snark::SNARK;
use std::time::Instant;

/// ZK Proof Verifier with Groth16
pub struct ZKVerifier {
    verification_key: ArkVerifyingKey<Bn254>,
}

impl ZKVerifier {
    pub fn new(verification_key: VerificationKey) -> Result<Self, String> {
        // Deserialize the verification key; surface an error instead of panicking
        // so callers can handle invalid or corrupted key material gracefully.
        let ark_vk =
            ArkVerifyingKey::<Bn254>::deserialize_compressed(&verification_key.key_data[..])
                .map_err(|e| format!("Failed to deserialize verification key: {}", e))?;

        Ok(Self {
            verification_key: ark_vk,
        })
    }

    /// Verify a ZK proof
    pub fn verify_proof(&self, proof: &ZKProof, public_inputs: &[u8]) -> bool {
        let start = Instant::now();

        // Deserialize proof
        let ark_proof = match Proof::<Bn254>::deserialize_compressed(&proof.proof_data[..]) {
            Ok(p) => p,
            Err(_) => return false,
        };

        // Deserialize public inputs (module_hash and input_hash)
        let mut cursor = public_inputs;
        let public_inputs_fe: Vec<Fr> = (0..2)
            .filter_map(|_| Fr::deserialize_compressed(&mut cursor).ok())
            .collect();

        if public_inputs_fe.len() != 2 {
            return false;
        }

        // Verify the proof
        let result =
            Groth16::<Bn254>::verify(&self.verification_key, &public_inputs_fe, &ark_proof)
                .unwrap_or(false);

        let elapsed = start.elapsed();
        tracing::info!(
            "Proof verification took {:?} (target: <1s), result: {}",
            elapsed,
            result
        );

        result
    }

    /// Get proof size in bytes
    pub fn proof_size(&self, proof: &ZKProof) -> usize {
        proof.size()
    }
}

impl Default for ZKVerifier {
    fn default() -> Self {
        use crate::prover::ZKProver;

        // Get verification key from default prover
        let prover = ZKProver::default();
        let vk_data = prover.verification_key().key_data.clone();

        Self::new(VerificationKey { key_data: vk_data })
            .expect("Default ZKProver produces a valid verification key")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{prover::ZKProver, ExecutionTrace};

    #[test]
    fn test_proof_verification() {
        let prover = ZKProver::default();
        let verifier = ZKVerifier::default();

        let trace = ExecutionTrace {
            module_hash: "test_hash".to_string(),
            function_name: "test_fn".to_string(),
            inputs: vec![1, 2, 3],
            outputs: vec![4, 5, 6],
            execution_time_ms: 100,
            gas_used: 1000,
            timestamp: 42,
        };

        let proof = prover.generate_proof(trace).unwrap();

        // Verify with correct public inputs
        assert!(verifier.verify_proof(&proof, &proof.public_inputs));

        // Verify with incorrect public inputs should fail
        let wrong_inputs = vec![0u8; proof.public_inputs.len()];
        assert!(!verifier.verify_proof(&proof, &wrong_inputs));
    }

    #[test]
    fn test_proof_verification_performance() {
        let prover = ZKProver::default();
        let verifier = ZKVerifier::default();

        let trace = ExecutionTrace {
            module_hash: "performance_test".to_string(),
            function_name: "compute".to_string(),
            inputs: vec![1; 1000],
            outputs: vec![2; 1000],
            execution_time_ms: 500,
            gas_used: 50000,
            timestamp: 12345,
        };

        let proof = prover.generate_proof(trace).unwrap();

        let start = Instant::now();
        let result = verifier.verify_proof(&proof, &proof.public_inputs);
        let elapsed = start.elapsed();

        assert!(result, "Proof verification should succeed");
        assert!(
            elapsed.as_secs() < 1,
            "Proof verification took {:?}, should be < 1s",
            elapsed
        );
    }

    #[test]
    fn test_proof_size() {
        let prover = ZKProver::default();
        let verifier = ZKVerifier::default();

        let trace = ExecutionTrace {
            module_hash: "test".to_string(),
            function_name: "fn".to_string(),
            inputs: vec![1, 2, 3, 4],
            outputs: vec![5, 6, 7, 8],
            execution_time_ms: 100,
            gas_used: 1000,
            timestamp: 0,
        };

        let proof = prover.generate_proof(trace).unwrap();
        let size = verifier.proof_size(&proof);

        // Groth16 proofs are typically around 128-256 bytes
        assert!(
            size > 0 && size < 1024,
            "Proof size should be reasonable: {} bytes",
            size
        );
    }

    #[test]
    fn test_new_returns_error_on_invalid_key_data() {
        let bad_vk = crate::VerificationKey {
            key_data: vec![0xFF; 32],
        };
        let result = ZKVerifier::new(bad_vk);
        assert!(
            result.is_err(),
            "ZKVerifier::new should fail on invalid key data"
        );
    }
}
