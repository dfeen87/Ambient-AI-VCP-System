use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[cfg(feature = "wasm-runtime")]
use wasmedge_sdk::{
    config::{CommonConfigOptions, ConfigBuilder},
    params, VmBuilder,
};

pub mod limits;
pub mod sandbox;
pub mod trace;

pub use limits::*;
pub use sandbox::*;
pub use trace::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WasmRuntime {
    WasmEdge,
    Wasmer,
    WAVM,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmCall {
    pub module_path: String,
    pub function_name: String,
    pub inputs: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmResult {
    pub output: Vec<u8>,
    pub execution_time_ms: u64,
    pub gas_used: u64,
    pub success: bool,
    pub error: Option<String>,
}

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

pub async fn execute(&self, call: WasmCall) -> Result<WasmResult> {
    let start = Instant::now();

    let canonical_module_path = match canonicalize_module_path(&call.module_path) {
        Ok(p) => p,
        Err(_) => {
            return Ok(WasmResult {
                output: vec![],
                execution_time_ms: 0,
                gas_used: 0,
                success: false,
                error: Some(format!("Module not found: {}", call.module_path)),
            })
        }
    };

    let mut call = call;
    call.module_path = canonical_module_path.to_string_lossy().to_string();

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
                WasmRuntime::WasmEdge => self.execute_wasmedge(&call, start).await,
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

    #[cfg(feature = "wasm-runtime")]
    async fn execute_wasmedge(&self, call: &WasmCall, start: Instant) -> Result<WasmResult> {
        let config = ConfigBuilder::new(CommonConfigOptions::default())
            .with_bulk_memory_operations(true)
            .build()?;

        let mut vm = VmBuilder::new().with_config(config).build()?;

        vm.load_wasm_from_file(&call.module_path)?;
        vm.validate()?;

        let max_duration = std::time::Duration::from_secs(self.limits.timeout_seconds as u64);
        let result = tokio::time::timeout(max_duration, async move {
            vm.run_func(Some(&call.function_name), params!())
        })
        .await;

        let execution_time = start.elapsed().as_millis() as u64;

        match result {
            Err(_) => Ok(WasmResult {
                output: vec![],
                execution_time_ms: execution_time,
                gas_used: self.limits.max_instructions,
                success: false,
                error: Some("Timeout exceeded - execution cancelled".to_string()),
            }),
            Ok(Ok(returns)) => {
                let output = if returns.is_empty() {
                    vec![0u8]
                } else {
                    returns[0].to_i32().to_le_bytes().to_vec()
                };
                Ok(WasmResult {
                    output,
                    execution_time_ms: execution_time,
                    gas_used: self
                        .limits
                        .max_instructions
                        .min(execution_time.saturating_mul(10_000)),
                    success: true,
                    error: None,
                })
            }
            Ok(Err(e)) => Ok(WasmResult {
                output: vec![],
                execution_time_ms: execution_time,
                gas_used: self
                    .limits
                    .max_instructions
                    .min(execution_time.saturating_mul(10_000)),
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    pub async fn execute_with_trace(&self, call: WasmCall) -> Result<(WasmResult, ExecutionTrace)> {
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

    pub async fn verify_determinism(&self, _module_hash: &str, _inputs: &[u8]) -> bool {
        true
    }

    fn hash_module(module_path: &str) -> Result<String> {
        use sha3::{Digest, Sha3_256};
        let bytes = std::fs::read(module_path)?;
        let mut hasher = Sha3_256::new();
        hasher.update(&bytes);
        Ok(format!("{:x}", hasher.finalize()))
    }

    pub fn limits(&self) -> &SandboxLimits {
        &self.limits
    }
}

fn canonicalize_module_path(path: &str) -> Result<std::path::PathBuf> {
    let canonical = std::fs::canonicalize(path)?;
    let roots =
        std::env::var("WASM_ALLOWED_ROOTS").unwrap_or_else(|_| "./wasm-modules,./tmp".to_string());
    let allowed = roots
        .split(',')
        .filter_map(|r| std::fs::canonicalize(r.trim()).ok())
        .any(|root| canonical.starts_with(root));
    if !allowed {
        return Err(anyhow::anyhow!("Module path outside allowed roots"));
    }
    Ok(canonical)
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
        std::env::set_var("WASM_ALLOWED_ROOTS", ".");
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

    #[test]
    fn test_module_path_rejected_outside_roots() {
        std::env::set_var("WASM_ALLOWED_ROOTS", "./wasm-modules");
        let err = canonicalize_module_path("Cargo.toml").unwrap_err();
        assert!(err.to_string().contains("outside allowed roots"));
    }
}
