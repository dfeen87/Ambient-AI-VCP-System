//! Bluetooth PAN tethering detection and management
//!
//! Detects bnep* interfaces and integrates with backhaul scoring

use crate::connectivity::{ConnectivityError, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;
use tracing::{debug, info, warn};

/// Bluetooth tether configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BluetoothTetherConfig {
    /// Enable Bluetooth tether detection
    pub enabled: bool,

    /// Interface pattern (typically bnep*)
    pub interface_pattern: String,

    /// Enable network configuration
    pub enable_network_config: bool,

    /// Validate route before promotion
    pub validate_route: bool,

    /// Route validation target
    pub validation_target: String,
}

impl Default for BluetoothTetherConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interface_pattern: "bnep".to_string(),
            enable_network_config: true,
            validate_route: true,
            validation_target: "8.8.8.8".to_string(),
        }
    }
}

/// Bluetooth tether manager
pub struct BluetoothTether {
    config: BluetoothTetherConfig,
    execute_commands: bool,
    active_interfaces: Vec<String>,
}

impl BluetoothTether {
    pub fn new(config: BluetoothTetherConfig, execute_commands: bool) -> Self {
        Self {
            config,
            execute_commands,
            active_interfaces: Vec::new(),
        }
    }

    /// Detect Bluetooth PAN interfaces
    pub async fn detect_interfaces(&self) -> Result<Vec<String>> {
        if !self.config.enabled {
            return Ok(Vec::new());
        }

        debug!("Detecting Bluetooth PAN interfaces");

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

            // Check if interface matches pattern
            if name.starts_with(&self.config.interface_pattern) {
                info!(interface = %name, "Detected Bluetooth PAN interface");
                detected.push(name);
            }
        }

        Ok(detected)
    }

    /// Initialize Bluetooth PAN interface
    pub async fn initialize_interface(&mut self, interface: &str) -> Result<()> {
        info!(interface = %interface, "Initializing Bluetooth PAN interface");

        // Bring interface up
        self.execute(&["ip", "link", "set", interface, "up"])?;

        // Configure network if enabled
        if self.config.enable_network_config {
            self.configure_network(interface).await?;
        }

        // Validate route if enabled
        if self.config.validate_route {
            self.validate_connectivity(interface).await?;
        }

        self.active_interfaces.push(interface.to_string());

        info!(interface = %interface, "Bluetooth PAN interface initialized");

        Ok(())
    }

    /// Configure network for interface
    async fn configure_network(&self, interface: &str) -> Result<()> {
        debug!(interface = %interface, "Configuring network");

        // In production, would:
        // 1. Request DHCP via dhclient or systemd-networkd
        // 2. Or configure IPv6 SLAAC
        // 3. Add routing rules

        // Simulate DHCP request
        self.execute(&["dhclient", "-1", "-timeout", "30", interface])?;

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

    /// Release Bluetooth PAN interface
    pub async fn release_interface(&mut self, interface: &str) -> Result<()> {
        info!(interface = %interface, "Releasing Bluetooth PAN interface");

        // Release network configuration
        if self.config.enable_network_config {
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

    #[tokio::test]
    async fn test_detect_interfaces() {
        let config = BluetoothTetherConfig::default();
        let tether = BluetoothTether::new(config, false);

        let result = tether.detect_interfaces().await;
        assert!(result.is_ok());

        // May or may not find interfaces depending on system
        let interfaces = result.unwrap();
        debug!("Detected {} Bluetooth PAN interfaces", interfaces.len());
    }

    #[tokio::test]
    async fn test_interface_lifecycle() {
        let config = BluetoothTetherConfig::default();
        let mut tether = BluetoothTether::new(config, false);

        assert_eq!(tether.active_interfaces().len(), 0);

        tether.initialize_interface("bnep0").await.unwrap();
        assert_eq!(tether.active_interfaces().len(), 1);
        assert_eq!(tether.active_interfaces()[0], "bnep0");

        tether.release_interface("bnep0").await.unwrap();
        assert_eq!(tether.active_interfaces().len(), 0);
    }

    #[test]
    fn test_interface_pattern() {
        let config = BluetoothTetherConfig::default();
        let tether = BluetoothTether::new(config, false);

        // Check pattern matching
        assert!("bnep0".starts_with(&tether.config.interface_pattern));
        assert!("bnep1".starts_with(&tether.config.interface_pattern));
        assert!(!"eth0".starts_with(&tether.config.interface_pattern));
    }
}
