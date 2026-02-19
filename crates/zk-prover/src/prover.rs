use crate::{ExecutionTrace, ProvingKey, VerificationKey, ZKProof};
use anyhow::Result;
use ark_bn254::{Bn254, Fr};
use ark_ff::{PrimeField, Zero};
use ark_groth16::{Groth16, ProvingKey as ArkProvingKey};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_snark::SNARK;
use ark_std::rand::SeedableRng;
use blake2::{Blake2s256, Digest};
use std::time::Instant;

/// Circuit for verifying execution trace
#[derive(Clone)]
struct ExecutionTraceCircuit {
    /// Hash of the WASM module (public input)
    module_hash: Option<Fr>,
    /// Hash of the inputs (public input)
    input_hash: Option<Fr>,
    /// Hash of the outputs (witness)
    output_hash: Option<Fr>,
    /// Execution time in ms (witness)
    execution_time: Option<Fr>,
    /// Gas used (witness)
    gas_used: Option<Fr>,
}

impl ConstraintSynthesizer<Fr> for ExecutionTraceCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        use ark_r1cs_std::fields::fp::FpVar;
        use ark_r1cs_std::prelude::*;

        // Allocate public inputs
        let module_hash_var = FpVar::new_input(cs.clone(), || {
            self.module_hash.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let input_hash_var = FpVar::new_input(cs.clone(), || {
            self.input_hash.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Allocate witnesses
        let output_hash_var = FpVar::new_witness(cs.clone(), || {
            self.output_hash.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let _execution_time_var = FpVar::new_witness(cs.clone(), || {
            self.execution_time.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let _gas_used_var = FpVar::new_witness(cs, || {
            self.gas_used.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Simple constraint: output hash should be derived from inputs (placeholder - just check they're allocated)
        // In a real system, you'd have actual computation constraints here
        let _ = &module_hash_var + &input_hash_var + &output_hash_var;

        Ok(())
    }
}

/// ZK Prover with Groth16
pub struct ZKProver {
    proving_key: ArkProvingKey<Bn254>,
    verification_key: VerificationKey,
}

impl ZKProver {
    /// Create a new `ZKProver` from serialised key material.
    ///
    /// Returns `Err(anyhow::Error)` if `proving_key.key_data` cannot be
    /// deserialised as a Groth16 proving key for the BN254 curve.  Callers
    /// must not rely on a silent fallback to any default key — invalid key
    /// material is always surfaced as an error.
    pub fn new(proving_key: ProvingKey, verification_key: VerificationKey) -> Result<Self> {
        // Deserialize the proving key; return an error if the key data is invalid
        // rather than silently falling back to a predictable seed-0 key.
        let ark_pk =
            ArkProvingKey::<Bn254>::deserialize_compressed(&proving_key.key_data[..])
                .map_err(|e| anyhow::anyhow!("Failed to deserialize proving key: {}", e))?;

        Ok(Self {
            proving_key: ark_pk,
            verification_key,
        })
    }

    /// Generate a ZK proof from execution trace
    pub fn generate_proof(&self, trace: ExecutionTrace) -> Result<ZKProof> {
        let start = Instant::now();

        // Hash the module, inputs, and outputs to field elements
        let module_hash_fe = hash_to_field(trace.module_hash.as_bytes());
        let input_hash_fe = hash_to_field(&trace.inputs);
        let output_hash_fe = hash_to_field(&trace.outputs);

        // Create circuit with actual values
        let circuit = ExecutionTraceCircuit {
            module_hash: Some(module_hash_fe),
            input_hash: Some(input_hash_fe),
            output_hash: Some(output_hash_fe),
            execution_time: Some(Fr::from(trace.execution_time_ms)),
            gas_used: Some(Fr::from(trace.gas_used)),
        };

        // Generate proof
        let rng = &mut ark_std::rand::rngs::StdRng::seed_from_u64(trace.timestamp);
        let proof = Groth16::<Bn254>::prove(&self.proving_key, circuit, rng)?;

        // Serialize proof
        let mut proof_bytes = Vec::new();
        proof.serialize_compressed(&mut proof_bytes)?;

        // Prepare public inputs (module hash and input hash)
        let mut public_inputs = Vec::new();
        module_hash_fe.serialize_compressed(&mut public_inputs)?;
        input_hash_fe.serialize_compressed(&mut public_inputs)?;

        let elapsed = start.elapsed();
        tracing::info!("Proof generation took {:?} (target: <10s)", elapsed);

        Ok(ZKProof::new(proof_bytes, public_inputs, trace.module_hash))
    }

    /// Get verification key
    pub fn verification_key(&self) -> &VerificationKey {
        &self.verification_key
    }
}

impl Default for ZKProver {
    fn default() -> Self {
        // Generate default keys for testing
        let rng = &mut ark_std::rand::rngs::StdRng::seed_from_u64(0);
        let circuit = ExecutionTraceCircuit {
            module_hash: Some(Fr::from(1u64)),
            input_hash: Some(Fr::from(1u64)),
            output_hash: Some(Fr::from(1u64)),
            execution_time: Some(Fr::from(100u64)),
            gas_used: Some(Fr::from(1000u64)),
        };

        let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(circuit, rng).unwrap();

        let mut vk_bytes = Vec::new();
        vk.serialize_compressed(&mut vk_bytes).unwrap();

        Self {
            proving_key: pk,
            verification_key: VerificationKey { key_data: vk_bytes },
        }
    }
}

/// Hash arbitrary bytes to a field element
fn hash_to_field(data: &[u8]) -> Fr {
    let mut hasher = Blake2s256::new();
    hasher.update(data);
    let hash = hasher.finalize();

    // Convert hash to field element, ensuring it's non-zero
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&hash[0..32]);
    let mut fe = Fr::from_be_bytes_mod_order(&bytes);

    // If we got zero (very unlikely), add 1
    if fe.is_zero() {
        fe = Fr::from(1u64);
    }

    fe
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proof_generation() {
        let prover = ZKProver::default();
        let trace = ExecutionTrace {
            module_hash: "test_hash".to_string(),
            function_name: "test_fn".to_string(),
            inputs: vec![1, 2, 3],
            outputs: vec![4, 5, 6],
            execution_time_ms: 100,
            gas_used: 1000,
            timestamp: 0,
        };

        let proof = prover.generate_proof(trace).unwrap();
        assert!(!proof.proof_data.is_empty());
        assert!(!proof.public_inputs.is_empty());
    }

    #[test]
    fn test_proof_generation_performance() {
        let prover = ZKProver::default();
        let trace = ExecutionTrace {
            module_hash: "performance_test".to_string(),
            function_name: "test_fn".to_string(),
            inputs: vec![1; 1000],
            outputs: vec![2; 1000],
            execution_time_ms: 500,
            gas_used: 50000,
            timestamp: 12345,
        };

        let start = Instant::now();
        let proof = prover.generate_proof(trace).unwrap();
        let elapsed = start.elapsed();

        assert!(!proof.proof_data.is_empty());
        assert!(
            elapsed.as_secs() < 10,
            "Proof generation took {:?}, should be < 10s",
            elapsed
        );
    }

    #[test]
    fn test_new_returns_error_on_invalid_key_data() {
        let bad_pk = crate::ProvingKey {
            key_data: vec![0xFF; 32],
        };
        let bad_vk = crate::VerificationKey {
            key_data: vec![0xFF; 32],
        };
        let result = ZKProver::new(bad_pk, bad_vk);
        assert!(result.is_err(), "ZKProver::new should fail on invalid key data");
    }
}
