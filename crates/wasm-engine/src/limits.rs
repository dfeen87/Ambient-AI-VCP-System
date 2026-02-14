use serde::{Deserialize, Serialize};

/// Resource limits for WASM sandbox
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxLimits {
    pub memory_mb: u32,
    pub timeout_seconds: u32,
    pub max_instructions: u64,
    pub gas_metering_enabled: bool,
}

impl Default for SandboxLimits {
    fn default() -> Self {
        Self {
            memory_mb: 512,
            timeout_seconds: 30,
            max_instructions: 10_000_000_000, // 10 billion
            gas_metering_enabled: true,
        }
    }
}

impl SandboxLimits {
    pub fn new(memory_mb: u32, timeout_seconds: u32, max_instructions: u64) -> Self {
        Self {
            memory_mb,
            timeout_seconds,
            max_instructions,
            gas_metering_enabled: true,
        }
    }

    pub fn strict() -> Self {
        Self {
            memory_mb: 256,
            timeout_seconds: 10,
            max_instructions: 1_000_000_000,
            gas_metering_enabled: true,
        }
    }

    pub fn relaxed() -> Self {
        Self {
            memory_mb: 1024,
            timeout_seconds: 60,
            max_instructions: 50_000_000_000,
            gas_metering_enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_limits() {
        let limits = SandboxLimits::default();
        assert_eq!(limits.memory_mb, 512);
        assert_eq!(limits.timeout_seconds, 30);
        assert!(limits.gas_metering_enabled);
    }

    #[test]
    fn test_strict_limits() {
        let limits = SandboxLimits::strict();
        assert_eq!(limits.memory_mb, 256);
        assert_eq!(limits.timeout_seconds, 10);
    }
}
