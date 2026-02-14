use serde::{Deserialize, Serialize};

/// Sandbox capabilities configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxCapabilities {
    pub filesystem_access: bool,
    pub network_access: bool,
    pub crypto_allowed: bool,
}

impl Default for SandboxCapabilities {
    fn default() -> Self {
        Self {
            filesystem_access: false,
            network_access: false,
            crypto_allowed: true,
        }
    }
}

impl SandboxCapabilities {
    pub fn new(filesystem: bool, network: bool, crypto: bool) -> Self {
        Self {
            filesystem_access: filesystem,
            network_access: network,
            crypto_allowed: crypto,
        }
    }

    pub fn no_access() -> Self {
        Self {
            filesystem_access: false,
            network_access: false,
            crypto_allowed: false,
        }
    }
}
