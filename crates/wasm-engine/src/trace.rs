use serde::{Deserialize, Serialize};

/// Execution trace for ZK proof generation
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

impl ExecutionTrace {
    /// Create a new execution trace
    pub fn new(
        module_hash: String,
        function_name: String,
        inputs: Vec<u8>,
        outputs: Vec<u8>,
        execution_time_ms: u64,
        gas_used: u64,
    ) -> Self {
        Self {
            module_hash,
            function_name,
            inputs,
            outputs,
            execution_time_ms,
            gas_used,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    /// Get trace hash for verification
    pub fn hash(&self) -> String {
        use sha3::{Digest, Sha3_256};
        let mut hasher = Sha3_256::new();
        hasher.update(&self.module_hash);
        hasher.update(&self.function_name);
        hasher.update(&self.inputs);
        hasher.update(&self.outputs);
        format!("{:x}", hasher.finalize())
    }
}
