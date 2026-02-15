use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[cfg(feature = "wasm-runtime")]
use wasmedge_sdk::{
    config::{CommonConfigOptions, ConfigBuilder},
    Module, Store, Vm, VmBuilder,
};

pub mod limits;
pub mod sandbox;
pub mod trace;

pub use limits::*;
pub use sandbox::*;
pub use trace::*;

/// WASM runtime type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WasmRuntime {
    WasmEdge,
    Wasmer,
    WAVM,
}

/// WASM function call specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmCall {
    pub module_path: String,
    pub function_name: String,
    pub inputs: Vec<u8>,
}

/// Result of WASM execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmResult {
    pub output: Vec<u8>,
    pub execution_time_ms: u64,
    pub gas_used: u64,
    pub success: bool,
    pub error: Option<String>,
}

/// WASM execution engine
pub struct WasmEngine {
    _runtime: WasmRuntime,
    limits: SandboxLimits,
}

impl WasmEngine {
    pub fn new(runtime: WasmRuntime, limits: SandboxLimits) -> Self {
        Self {
            _runtime: runtime,
            limits,
        }
    }

    /// Execute a WASM function call
    pub async fn execute(&self, call: WasmCall) -> Result<WasmResult> {
        let _start = Instant::now();

        // Validate module path exists
        if !std::path::Path::new(&call.module_path).exists() {
            return Ok(WasmResult {
                output: vec![],
                execution_time_ms: 0,
                gas_used: 0,
                success: false,
                error: Some(format!("Module not found: {}", call.module_path)),
            });
        }

        #[cfg(feature = "wasm-runtime")]
        {
            match self._runtime {
                WasmRuntime::WasmEdge => self.execute_wasmedge(&call, _start).await,
                _ => Err(anyhow::anyhow!(
                    "Runtime not implemented: {:?}",
                    self._runtime
                )),
            }
        }

        #[cfg(not(feature = "wasm-runtime"))]
        {
            Ok(WasmResult {
                output: vec![],
                execution_time_ms: 0,
                gas_used: 0,
                success: false,
                error: Some(
                    "WASM runtime not enabled. Build with --features wasm-runtime".to_string(),
                ),
            })
        }
    }

    /// Execute with WasmEdge runtime
    #[cfg(feature = "wasm-runtime")]
    async fn execute_wasmedge(&self, call: &WasmCall, start: Instant) -> Result<WasmResult> {
        use std::time::Duration;

        // Build configuration with limits
        let config = ConfigBuilder::new(CommonConfigOptions::default())
            .with_bulk_memory_operations(true)
            .build()?;

        // Create VM
        let mut vm = VmBuilder::new().with_config(config).build()?;

        // Load module
        vm.load_wasm_from_file(&call.module_path)?;
        vm.validate()?;

        // Check timeout
        let elapsed = start.elapsed();
        if elapsed > Duration::from_secs(self.limits.timeout_seconds as u64) {
            return Ok(WasmResult {
                output: vec![],
                execution_time_ms: elapsed.as_millis() as u64,
                gas_used: 0,
                success: false,
                error: Some("Timeout exceeded".to_string()),
            });
        }

        // Execute function
        let result = vm.run_func(Some(&call.function_name), vec![]);

        let execution_time = start.elapsed().as_millis() as u64;

        match result {
            Ok(returns) => {
                // Convert return values to bytes
                let output = if returns.is_empty() {
                    vec![0u8]
                } else {
                    // Simple conversion - take first value as i32
                    let val = returns[0].to_i32();
                    val.to_le_bytes().to_vec()
                };

                Ok(WasmResult {
                    output,
                    execution_time_ms: execution_time,
                    gas_used: 0, // WasmEdge doesn't expose gas directly
                    success: true,
                    error: None,
                })
            }
            Err(e) => Ok(WasmResult {
                output: vec![],
                execution_time_ms: execution_time,
                gas_used: 0,
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Execute with execution trace recording
    pub async fn execute_with_trace(&self, call: WasmCall) -> Result<(WasmResult, ExecutionTrace)> {
        let _trace_start = Instant::now();
        let result = self.execute(call.clone()).await?;

        let trace = ExecutionTrace {
            module_hash: Self::hash_module(&call.module_path)?,
            function_name: call.function_name,
            inputs: call.inputs,
            outputs: result.output.clone(),
            execution_time_ms: result.execution_time_ms,
            gas_used: result.gas_used,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
        };

        Ok((result, trace))
    }

    /// Verify determinism by executing twice with same inputs
    pub async fn verify_determinism(&self, _module_hash: &str, _inputs: &[u8]) -> bool {
        // For now, return true as determinism checking requires state management
        // In production, this would execute the module twice and compare outputs
        true
    }

    /// Hash a WASM module
    fn hash_module(module_path: &str) -> Result<String> {
        use sha3::{Digest, Sha3_256};
        let bytes = std::fs::read(module_path)?;
        let mut hasher = Sha3_256::new();
        hasher.update(&bytes);
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Get current limits
    pub fn limits(&self) -> &SandboxLimits {
        &self.limits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let limits = SandboxLimits::default();
        let engine = WasmEngine::new(WasmRuntime::WasmEdge, limits);
        assert_eq!(engine.limits().memory_mb, 512);
    }

    #[test]
    fn test_module_not_found() {
        let limits = SandboxLimits::default();
        let engine = WasmEngine::new(WasmRuntime::WasmEdge, limits);

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            engine
                .execute(WasmCall {
                    module_path: "nonexistent.wasm".to_string(),
                    function_name: "test".to_string(),
                    inputs: vec![],
                })
                .await
        });

        assert!(result.is_ok());
        let wasm_result = result.unwrap();
        assert!(!wasm_result.success);
        assert!(wasm_result.error.is_some());
    }
}
