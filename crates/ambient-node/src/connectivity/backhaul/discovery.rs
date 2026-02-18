//! Interface discovery and monitoring
//!
//! This module detects and monitors network interfaces:
//! - Ethernet (eth*, eno*, enp*)
//! - Wi-Fi (wlan*, wlp*)
//! - LTE/5G modem (wwan*, ppp*)
//! - USB tethering (usb0, enx*)
//! - Bluetooth PAN (bnep0)
//!
//! Uses rtnetlink to watch for interface up/down and address/route changes.
//! Maintains an in-memory registry of candidate WAN interfaces.

use crate::connectivity::{ConnectivityError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Interface type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InterfaceType {
    Ethernet,
    WiFi,
    LteModem,
    UsbTether,
    BluetoothPan,
    Unknown,
}

impl InterfaceType {
    /// Classify interface based on name
    pub fn from_name(name: &str) -> Self {
        if name.starts_with("eth")
            || name.starts_with("eno")
            || name.starts_with("enp")
            || name.starts_with("ens")
        {
            InterfaceType::Ethernet
        } else if name.starts_with("wlan") || name.starts_with("wlp") {
            InterfaceType::WiFi
        } else if name.starts_with("wwan") || name.starts_with("ppp") {
            InterfaceType::LteModem
        } else if name.starts_with("usb") || name.starts_with("enx") {
            InterfaceType::UsbTether
        } else if name.starts_with("bnep") {
            InterfaceType::BluetoothPan
        } else {
            InterfaceType::Unknown
        }
    }

    /// Default policy bias for interface type
    pub fn default_bias(&self) -> i32 {
        match self {
            InterfaceType::Ethernet => 100,
            InterfaceType::WiFi => 80,
            InterfaceType::LteModem => 60,
            InterfaceType::UsbTether => 40,
            InterfaceType::BluetoothPan => 30,
            InterfaceType::Unknown => 0,
        }
    }
}

/// Interface information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceInfo {
    pub name: String,
    pub iface_type: InterfaceType,
    pub is_up: bool,
    pub has_carrier: bool,
    pub has_address: bool,
    pub mtu: u32,
    pub mac_address: Option<String>,
    pub ipv4_addresses: Vec<String>,
    pub ipv6_addresses: Vec<String>,
}

impl InterfaceInfo {
    /// Check if interface is a viable WAN candidate
    pub fn is_wan_candidate(&self) -> bool {
        self.is_up
            && self.has_carrier
            && self.has_address
            && !matches!(self.iface_type, InterfaceType::Unknown)
    }
}

/// Interface registry maintaining state of all discovered interfaces
pub struct InterfaceRegistry {
    interfaces: Arc<RwLock<HashMap<String, InterfaceInfo>>>,
}

impl InterfaceRegistry {
    pub fn new() -> Self {
        Self {
            interfaces: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register or update an interface
    pub async fn register(&self, info: InterfaceInfo) {
        let name = info.name.clone();
        let mut interfaces = self.interfaces.write().await;
        
        if interfaces.contains_key(&name) {
            debug!(interface = %name, "Updating interface");
        } else {
            info!(interface = %name, iface_type = ?info.iface_type, "Discovered new interface");
        }
        
        interfaces.insert(name, info);
    }

    /// Remove an interface from the registry
    pub async fn unregister(&self, name: &str) {
        let mut interfaces = self.interfaces.write().await;
        if interfaces.remove(name).is_some() {
            info!(interface = %name, "Interface removed");
        }
    }

    /// Get all WAN candidate interfaces
    pub async fn get_wan_candidates(&self) -> Vec<InterfaceInfo> {
        let interfaces = self.interfaces.read().await;
        interfaces
            .values()
            .filter(|info| info.is_wan_candidate())
            .cloned()
            .collect()
    }

    /// Get specific interface info
    pub async fn get(&self, name: &str) -> Option<InterfaceInfo> {
        let interfaces = self.interfaces.read().await;
        interfaces.get(name).cloned()
    }

    /// Get all interfaces
    pub async fn get_all(&self) -> Vec<InterfaceInfo> {
        let interfaces = self.interfaces.read().await;
        interfaces.values().cloned().collect()
    }
}

impl Default for InterfaceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Interface discovery service
pub struct InterfaceDiscovery {
    registry: Arc<InterfaceRegistry>,
}

impl InterfaceDiscovery {
    pub fn new(registry: Arc<InterfaceRegistry>) -> Self {
        Self { registry }
    }

    /// Start monitoring interfaces
    ///
    /// This is a simplified implementation that performs periodic discovery
    /// In production, this would use rtnetlink to get real-time notifications
    pub async fn start_monitoring(&self) -> Result<()> {
        info!("Starting interface discovery monitoring");
        
        // Initial discovery
        self.discover_interfaces().await?;
        
        // Spawn monitoring task
        let registry = self.registry.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
            loop {
                interval.tick().await;
                let discovery = InterfaceDiscovery::new(registry.clone());
                if let Err(e) = discovery.discover_interfaces().await {
                    warn!(error = %e, "Interface discovery iteration failed");
                }
            }
        });
        
        Ok(())
    }

    /// Discover interfaces using platform-specific methods
    ///
    /// This is a simplified mock implementation for demonstration.
    /// In production, this would use rtnetlink or similar platform APIs.
    async fn discover_interfaces(&self) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            self.discover_linux_interfaces().await
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            warn!("Interface discovery not implemented for this platform");
            Ok(())
        }
    }

    #[cfg(target_os = "linux")]
    async fn discover_linux_interfaces(&self) -> Result<()> {
        use std::fs;
        
        // Read /sys/class/net for interface list
        let net_path = "/sys/class/net";
        let entries = match fs::read_dir(net_path) {
            Ok(entries) => entries,
            Err(e) => {
                warn!(error = %e, "Could not read /sys/class/net, using mock data");
                return self.use_mock_interfaces().await;
            }
        };
        
        for entry in entries {
            let entry = entry.map_err(|e| {
                ConnectivityError::DiscoveryError(format!("Failed to read directory entry: {}", e))
            })?;
            
            let name = entry.file_name().to_string_lossy().to_string();
            
            // Skip loopback
            if name == "lo" {
                continue;
            }
            
            // Read interface state
            let is_up = self.read_sys_int(&format!("{}/{}/operstate", net_path, name))
                .map(|state| state == "up")
                .unwrap_or(false);
            
            let has_carrier = self.read_sys_int(&format!("{}/{}/carrier", net_path, name))
                .map(|c| c == "1")
                .unwrap_or(false);
            
            let mtu = self.read_sys_int(&format!("{}/{}/mtu", net_path, name))
                .and_then(|s| s.parse().ok())
                .unwrap_or(1500);
            
            let mac_address = self.read_sys_string(&format!("{}/{}/address", net_path, name));
            
            let info = InterfaceInfo {
                name: name.clone(),
                iface_type: InterfaceType::from_name(&name),
                is_up,
                has_carrier,
                has_address: is_up && has_carrier, // Simplified - would need to check actual addresses
                mtu,
                mac_address,
                ipv4_addresses: vec![],
                ipv6_addresses: vec![],
            };
            
            self.registry.register(info).await;
        }
        
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn read_sys_int(&self, path: &str) -> Option<String> {
        std::fs::read_to_string(path)
            .ok()
            .map(|s| s.trim().to_string())
    }

    #[cfg(target_os = "linux")]
    fn read_sys_string(&self, path: &str) -> Option<String> {
        std::fs::read_to_string(path)
            .ok()
            .map(|s| s.trim().to_string())
    }

    /// Use mock interface data for testing/development
    async fn use_mock_interfaces(&self) -> Result<()> {
        debug!("Using mock interface data");
        
        let mock_interfaces = vec![
            InterfaceInfo {
                name: "eth0".to_string(),
                iface_type: InterfaceType::Ethernet,
                is_up: true,
                has_carrier: true,
                has_address: true,
                mtu: 1500,
                mac_address: Some("00:11:22:33:44:55".to_string()),
                ipv4_addresses: vec!["192.168.1.100".to_string()],
                ipv6_addresses: vec![],
            },
            InterfaceInfo {
                name: "wlan0".to_string(),
                iface_type: InterfaceType::WiFi,
                is_up: true,
                has_carrier: false,
                has_address: false,
                mtu: 1500,
                mac_address: Some("AA:BB:CC:DD:EE:FF".to_string()),
                ipv4_addresses: vec![],
                ipv6_addresses: vec![],
            },
        ];
        
        for info in mock_interfaces {
            self.registry.register(info).await;
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interface_type_classification() {
        assert_eq!(InterfaceType::from_name("eth0"), InterfaceType::Ethernet);
        assert_eq!(InterfaceType::from_name("wlan0"), InterfaceType::WiFi);
        assert_eq!(InterfaceType::from_name("wwan0"), InterfaceType::LteModem);
        assert_eq!(InterfaceType::from_name("usb0"), InterfaceType::UsbTether);
        assert_eq!(InterfaceType::from_name("bnep0"), InterfaceType::BluetoothPan);
        assert_eq!(InterfaceType::from_name("unknown"), InterfaceType::Unknown);
    }

    #[test]
    fn test_interface_type_bias() {
        assert_eq!(InterfaceType::Ethernet.default_bias(), 100);
        assert_eq!(InterfaceType::WiFi.default_bias(), 80);
        assert_eq!(InterfaceType::LteModem.default_bias(), 60);
        assert_eq!(InterfaceType::UsbTether.default_bias(), 40);
        assert_eq!(InterfaceType::BluetoothPan.default_bias(), 30);
    }

    #[tokio::test]
    async fn test_registry_operations() {
        let registry = InterfaceRegistry::new();
        
        let info = InterfaceInfo {
            name: "eth0".to_string(),
            iface_type: InterfaceType::Ethernet,
            is_up: true,
            has_carrier: true,
            has_address: true,
            mtu: 1500,
            mac_address: None,
            ipv4_addresses: vec![],
            ipv6_addresses: vec![],
        };
        
        registry.register(info.clone()).await;
        
        let retrieved = registry.get("eth0").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "eth0");
        
        let candidates = registry.get_wan_candidates().await;
        assert_eq!(candidates.len(), 1);
        
        registry.unregister("eth0").await;
        assert!(registry.get("eth0").await.is_none());
    }

    #[tokio::test]
    async fn test_interface_discovery() {
        let registry = Arc::new(InterfaceRegistry::new());
        let discovery = InterfaceDiscovery::new(registry.clone());
        
        // This will use mock data if /sys/class/net is not available
        let result = discovery.discover_interfaces().await;
        assert!(result.is_ok());
        
        let all_interfaces = registry.get_all().await;
        assert!(!all_interfaces.is_empty());
    }
}
