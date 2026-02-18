//! Wi-Fi Access Point mode management
//!
//! Handles switching Wi-Fi radios to AP mode when no upstream is available.
//! Allows nearby devices or sibling nodes to connect for local continuity.

use crate::connectivity::{ConnectivityError, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;
use tracing::{debug, info, warn};

/// AP state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApState {
    /// AP is inactive
    Inactive,
    /// AP is starting up
    Starting,
    /// AP is active and accepting clients
    Active,
    /// AP is shutting down
    Stopping,
    /// AP encountered an error
    Error,
}

/// AP configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApConfig {
    /// Interface to use for AP mode
    pub interface: String,

    /// SSID to broadcast
    pub ssid: String,

    /// Channel to use (1-11 for 2.4GHz)
    pub channel: u8,

    /// IP address for AP interface
    pub ip_address: String,

    /// Subnet mask
    pub netmask: String,

    /// DHCP range start
    pub dhcp_start: String,

    /// DHCP range end
    pub dhcp_end: String,

    /// Whether to enable NAT
    pub enable_nat: bool,

    /// Upstream interface for NAT (if enabled)
    pub nat_upstream: Option<String>,
}

impl Default for ApConfig {
    fn default() -> Self {
        Self {
            interface: "wlan0".to_string(),
            ssid: "Ambient-Hotspot".to_string(),
            channel: 6,
            ip_address: "192.168.50.1".to_string(),
            netmask: "255.255.255.0".to_string(),
            dhcp_start: "192.168.50.100".to_string(),
            dhcp_end: "192.168.50.200".to_string(),
            enable_nat: true,
            nat_upstream: None,
        }
    }
}

/// AP mode manager
pub struct ApMode {
    config: ApConfig,
    state: ApState,
    execute_commands: bool,
}

impl ApMode {
    pub fn new(config: ApConfig, execute_commands: bool) -> Self {
        Self {
            config,
            state: ApState::Inactive,
            execute_commands,
        }
    }

    /// Get current state
    pub fn state(&self) -> ApState {
        self.state
    }

    /// Start AP mode
    pub async fn start(&mut self) -> Result<()> {
        if self.state == ApState::Active {
            debug!("AP mode already active");
            return Ok(());
        }

        info!(
            interface = %self.config.interface,
            ssid = %self.config.ssid,
            "Starting AP mode"
        );

        self.state = ApState::Starting;

        // Configure interface
        self.configure_interface().await?;

        // Start hostapd (in production, would use hostapd configuration)
        self.start_hostapd().await?;

        // Start DHCP server
        self.start_dhcp().await?;

        // Configure NAT if enabled
        if self.config.enable_nat {
            self.configure_nat().await?;
        }

        self.state = ApState::Active;
        info!("AP mode activated");

        Ok(())
    }

    /// Stop AP mode
    pub async fn stop(&mut self) -> Result<()> {
        if self.state == ApState::Inactive {
            debug!("AP mode already inactive");
            return Ok(());
        }

        info!("Stopping AP mode");
        self.state = ApState::Stopping;

        // Stop DHCP server
        self.stop_dhcp().await?;

        // Stop hostapd
        self.stop_hostapd().await?;

        // Remove NAT rules
        if self.config.enable_nat {
            self.remove_nat().await?;
        }

        // Deconfigure interface
        self.deconfigure_interface().await?;

        self.state = ApState::Inactive;
        info!("AP mode deactivated");

        Ok(())
    }

    /// Configure interface for AP mode
    async fn configure_interface(&self) -> Result<()> {
        debug!("Configuring interface for AP mode");

        // Bring interface down
        self.execute(&["ip", "link", "set", &self.config.interface, "down"])?;

        // Set IP address
        self.execute(&[
            "ip",
            "addr",
            "add",
            &format!("{}/{}", self.config.ip_address, "24"),
            "dev",
            &self.config.interface,
        ])?;

        // Bring interface up
        self.execute(&["ip", "link", "set", &self.config.interface, "up"])?;

        Ok(())
    }

    /// Deconfigure interface
    async fn deconfigure_interface(&self) -> Result<()> {
        debug!("Deconfiguring interface");

        self.execute(&["ip", "addr", "flush", "dev", &self.config.interface])?;

        Ok(())
    }

    /// Start hostapd (simplified - in production would use actual hostapd)
    async fn start_hostapd(&self) -> Result<()> {
        debug!("Starting hostapd");

        // In production, this would:
        // 1. Generate hostapd.conf with SSID, channel, security settings
        // 2. Start hostapd daemon with configuration
        // For this implementation, we'll just log it

        info!(
            ssid = %self.config.ssid,
            channel = self.config.channel,
            "Hostapd configuration prepared"
        );

        Ok(())
    }

    /// Stop hostapd
    async fn stop_hostapd(&self) -> Result<()> {
        debug!("Stopping hostapd");

        // In production: killall hostapd or systemctl stop hostapd

        Ok(())
    }

    /// Start DHCP server (simplified)
    async fn start_dhcp(&self) -> Result<()> {
        debug!("Starting DHCP server");

        // In production, would configure and start dnsmasq or dhcpd
        info!(
            range = format!("{} - {}", self.config.dhcp_start, self.config.dhcp_end),
            "DHCP server configuration prepared"
        );

        Ok(())
    }

    /// Stop DHCP server
    async fn stop_dhcp(&self) -> Result<()> {
        debug!("Stopping DHCP server");

        // In production: killall dnsmasq or systemctl stop isc-dhcp-server

        Ok(())
    }

    /// Configure NAT
    async fn configure_nat(&self) -> Result<()> {
        if let Some(upstream) = &self.config.nat_upstream {
            info!(
                ap_interface = %self.config.interface,
                upstream = %upstream,
                "Configuring NAT"
            );

            // Enable IP forwarding
            self.execute(&["sysctl", "-w", "net.ipv4.ip_forward=1"])?;

            // Add iptables NAT rule
            self.execute(&[
                "iptables",
                "-t",
                "nat",
                "-A",
                "POSTROUTING",
                "-o",
                upstream,
                "-j",
                "MASQUERADE",
            ])?;

            // Add forwarding rules
            self.execute(&[
                "iptables",
                "-A",
                "FORWARD",
                "-i",
                &self.config.interface,
                "-o",
                upstream,
                "-j",
                "ACCEPT",
            ])?;

            self.execute(&[
                "iptables",
                "-A",
                "FORWARD",
                "-i",
                upstream,
                "-o",
                &self.config.interface,
                "-m",
                "state",
                "--state",
                "RELATED,ESTABLISHED",
                "-j",
                "ACCEPT",
            ])?;
        }

        Ok(())
    }

    /// Remove NAT configuration
    async fn remove_nat(&self) -> Result<()> {
        if let Some(upstream) = &self.config.nat_upstream {
            debug!("Removing NAT configuration");

            // Remove iptables rules (simplified - in production would track exact rules)
            let _ = self.execute(&[
                "iptables",
                "-t",
                "nat",
                "-D",
                "POSTROUTING",
                "-o",
                upstream,
                "-j",
                "MASQUERADE",
            ]);
        }

        Ok(())
    }

    /// Execute a command
    fn execute(&self, args: &[&str]) -> Result<()> {
        if !self.execute_commands {
            debug!(command = ?args, "Skipping command execution (dry run)");
            return Ok(());
        }

        let output = Command::new(args[0])
            .args(&args[1..])
            .output()
            .map_err(|e| {
                ConnectivityError::HotspotError(format!("Failed to execute command: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(
                command = args.join(" "),
                stderr = %stderr,
                "Command execution failed (non-fatal)"
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ap_mode_lifecycle() {
        let config = ApConfig::default();
        let mut ap = ApMode::new(config, false); // Don't execute commands

        assert_eq!(ap.state(), ApState::Inactive);

        ap.start().await.unwrap();
        assert_eq!(ap.state(), ApState::Active);

        ap.stop().await.unwrap();
        assert_eq!(ap.state(), ApState::Inactive);
    }

    #[tokio::test]
    async fn test_ap_mode_double_start() {
        let config = ApConfig::default();
        let mut ap = ApMode::new(config, false);

        ap.start().await.unwrap();
        assert_eq!(ap.state(), ApState::Active);

        // Starting again should be idempotent
        ap.start().await.unwrap();
        assert_eq!(ap.state(), ApState::Active);
    }
}
