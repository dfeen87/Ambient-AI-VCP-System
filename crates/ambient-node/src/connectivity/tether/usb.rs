//! USB tethering detection and management
//!
//! Detects USB RNDIS/ECM/NCM interfaces (usb0, enx*)
//! Triggers DHCP/IPv6 SLAAC and validates route before promoting

use crate::connectivity::{ConnectivityError, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;
use tracing::{debug, info, warn};

/// USB tether configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbTetherConfig {
    /// Enable USB tether detection
    pub enabled: bool,

    /// Interfaces to monitor (patterns)
    pub interface_patterns: Vec<String>,

    /// Enable DHCP client
    pub enable_dhcp: bool,

    /// DHCP timeout in seconds
    pub dhcp_timeout_secs: u64,

    /// Validate route before promotion
    pub validate_route: bool,

    /// Route validation target
    pub validation_target: String,
}

impl Default for UsbTetherConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interface_patterns: vec!["usb0".to_string(), "enx".to_string()],
            enable_dhcp: true,
            dhcp_timeout_secs: 30,
            validate_route: true,
            validation_target: "8.8.8.8".to_string(),
        }
    }
}

/// USB tether manager
pub struct UsbTether {
    config: UsbTetherConfig,
    execute_commands: bool,
    active_interfaces: Vec<String>,
}

impl UsbTether {
    pub fn new(config: UsbTetherConfig, execute_commands: bool) -> Self {
        Self {
            config,
            execute_commands,
            active_interfaces: Vec::new(),
        }
    }

    /// Detect USB tethered interfaces
    pub async fn detect_interfaces(&self) -> Result<Vec<String>> {
        if !self.config.enabled {
            return Ok(Vec::new());
        }

        debug!("Detecting USB tethered interfaces");

        // Read /sys/class/net for interface list
        let net_path = "/sys/class/net";
        let entries = match std::fs::read_dir(net_path) {
            Ok(entries) => entries,
            Err(e) => {
                warn!(error = %e, "Could not read /sys/class/net");
                return Ok(Vec::new());
            }
        };

        let mut detected = Vec::new();

        for entry in entries {
            let entry = entry.map_err(|e| {
                ConnectivityError::TetherError(format!("Failed to read directory entry: {}", e))
            })?;

            let name = entry.file_name().to_string_lossy().to_string();

            // Check if interface matches any pattern
            if self.matches_pattern(&name) {
                info!(interface = %name, "Detected USB tethered interface");
                detected.push(name);
            }
        }

        Ok(detected)
    }

    /// Check if interface name matches configured patterns
    fn matches_pattern(&self, name: &str) -> bool {
        self.config
            .interface_patterns
            .iter()
            .any(|pattern| name.starts_with(pattern) || name == pattern)
    }

    /// Initialize USB tethered interface
    pub async fn initialize_interface(&mut self, interface: &str) -> Result<()> {
        info!(interface = %interface, "Initializing USB tethered interface");

        // Bring interface up
        self.execute(&["ip", "link", "set", interface, "up"])?;

        // Request DHCP if enabled
        if self.config.enable_dhcp {
            self.request_dhcp(interface).await?;
        }

        // Validate route if enabled
        if self.config.validate_route {
            self.validate_connectivity(interface).await?;
        }

        self.active_interfaces.push(interface.to_string());

        info!(interface = %interface, "USB tethered interface initialized");

        Ok(())
    }

    /// Request DHCP for interface
    async fn request_dhcp(&self, interface: &str) -> Result<()> {
        debug!(interface = %interface, "Requesting DHCP");

        // In production, would use dhclient or systemd-networkd
        // For this implementation, we'll simulate it

        self.execute(&[
            "dhclient",
            "-1",
            "-timeout",
            &self.config.dhcp_timeout_secs.to_string(),
            interface,
        ])?;

        Ok(())
    }

    /// Validate connectivity through interface
    async fn validate_connectivity(&self, interface: &str) -> Result<()> {
        debug!(
            interface = %interface,
            target = %self.config.validation_target,
            "Validating connectivity"
        );

        // Simple ping test
        let output = self.execute(&[
            "ping",
            "-c",
            "1",
            "-W",
            "3",
            "-I",
            interface,
            &self.config.validation_target,
        ])?;

        if output {
            info!(interface = %interface, "Connectivity validated");
            Ok(())
        } else {
            Err(ConnectivityError::TetherError(format!(
                "Connectivity validation failed for {}",
                interface
            )))
        }
    }

    /// Release USB tethered interface
    pub async fn release_interface(&mut self, interface: &str) -> Result<()> {
        info!(interface = %interface, "Releasing USB tethered interface");

        // Release DHCP lease
        if self.config.enable_dhcp {
            let _ = self.execute(&["dhclient", "-r", interface]);
        }

        // Bring interface down
        self.execute(&["ip", "link", "set", interface, "down"])?;

        self.active_interfaces.retain(|i| i != interface);

        Ok(())
    }

    /// Get active interfaces
    pub fn active_interfaces(&self) -> &[String] {
        &self.active_interfaces
    }

    /// Execute a command
    fn execute(&self, args: &[&str]) -> Result<bool> {
        if !self.execute_commands {
            debug!(command = ?args, "Skipping command execution (dry run)");
            return Ok(true);
        }

        let output = Command::new(args[0])
            .args(&args[1..])
            .output()
            .map_err(|e| {
                ConnectivityError::TetherError(format!("Failed to execute command: {}", e))
            })?;

        Ok(output.status.success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matching() {
        let config = UsbTetherConfig::default();
        let tether = UsbTether::new(config, false);

        assert!(tether.matches_pattern("usb0"));
        assert!(tether.matches_pattern("enx001122334455"));
        assert!(!tether.matches_pattern("eth0"));
        assert!(!tether.matches_pattern("wlan0"));
    }

    #[tokio::test]
    async fn test_interface_lifecycle() {
        let config = UsbTetherConfig::default();
        let mut tether = UsbTether::new(config, false);

        assert_eq!(tether.active_interfaces().len(), 0);

        tether.initialize_interface("usb0").await.unwrap();
        assert_eq!(tether.active_interfaces().len(), 1);
        assert_eq!(tether.active_interfaces()[0], "usb0");

        tether.release_interface("usb0").await.unwrap();
        assert_eq!(tether.active_interfaces().len(), 0);
    }

    #[tokio::test]
    async fn test_detect_interfaces() {
        let config = UsbTetherConfig::default();
        let tether = UsbTether::new(config, false);

        let result = tether.detect_interfaces().await;
        assert!(result.is_ok());

        // May or may not find interfaces depending on system
        let interfaces = result.unwrap();
        debug!("Detected {} USB interfaces", interfaces.len());
    }
}
