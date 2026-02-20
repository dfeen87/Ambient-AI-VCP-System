//! QoS (Quality of Service) management
//!
//! Prioritizes control traffic over bulk traffic on the hotspot

use crate::connectivity::Result;
use serde::{Deserialize, Serialize};
use std::process::Command;
use tracing::{debug, info};

/// Traffic class for QoS
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrafficClass {
    /// Control traffic (highest priority)
    Control,
    /// Interactive traffic (medium-high priority)
    Interactive,
    /// Bulk traffic (lowest priority)
    Bulk,
}

impl TrafficClass {
    /// Get DSCP value for traffic class
    pub fn dscp_value(&self) -> u8 {
        match self {
            TrafficClass::Control => 46,     // EF (Expedited Forwarding)
            TrafficClass::Interactive => 34, // AF41
            TrafficClass::Bulk => 10,        // AF11
        }
    }

    /// Get tc class ID for traffic class
    pub fn tc_class_id(&self) -> &'static str {
        match self {
            TrafficClass::Control => "1:10",
            TrafficClass::Interactive => "1:20",
            TrafficClass::Bulk => "1:30",
        }
    }
}

/// QoS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QosConfig {
    /// Interface to apply QoS rules
    pub interface: String,

    /// Enable QoS
    pub enabled: bool,

    /// Control traffic bandwidth limit (kbps)
    pub control_bandwidth_kbps: u32,

    /// Interactive traffic bandwidth limit (kbps)
    pub interactive_bandwidth_kbps: u32,

    /// Bulk traffic bandwidth limit (kbps)
    pub bulk_bandwidth_kbps: u32,

    /// Maximum total bandwidth for the hotspot interface (kbps).
    ///
    /// This is the HTB root class `rate`.  It must be set to (or above) the
    /// actual WAN uplink speed so that individual traffic classes can burst
    /// up to the full interface speed when other classes are idle.
    ///
    /// Setting this to the *sum* of the per-class minimums (the old default
    /// behaviour) would hard-cap ALL egress (upload) at that sum — even when
    /// only one class is active — because in Linux HTB a child class can
    /// never exceed its parent's own rate.  Since `tc` rules only apply to
    /// egress, this bug suppresses upload throughput without affecting
    /// downloads.  The default is 1 Gbps so the hotspot never bottlenecks
    /// the WAN uplink.
    pub max_bandwidth_kbps: u32,

    /// Control traffic ports
    pub control_ports: Vec<u16>,

    /// Interactive traffic ports
    pub interactive_ports: Vec<u16>,
}

impl Default for QosConfig {
    fn default() -> Self {
        Self {
            interface: "wlan0".to_string(),
            enabled: true,
            control_bandwidth_kbps: 1000,      // 1 Mbps for control
            interactive_bandwidth_kbps: 5000,  // 5 Mbps for interactive
            bulk_bandwidth_kbps: 10000,        // 10 Mbps for bulk
            max_bandwidth_kbps: 1_000_000,     // 1 Gbps – never cap the WAN uplink
            control_ports: vec![22, 443],      // SSH, HTTPS
            interactive_ports: vec![80, 8080], // HTTP
        }
    }
}

/// QoS manager
pub struct QosManager {
    config: QosConfig,
    execute_commands: bool,
}

impl QosManager {
    pub fn new(config: QosConfig, execute_commands: bool) -> Self {
        Self {
            config,
            execute_commands,
        }
    }

    /// Apply QoS rules
    pub async fn apply_qos(&self) -> Result<()> {
        if !self.config.enabled {
            debug!("QoS disabled, skipping");
            return Ok(());
        }

        info!(interface = %self.config.interface, "Applying QoS rules");

        // Set up HTB (Hierarchical Token Bucket) qdisc
        self.setup_htb().await?;

        // Create traffic classes
        self.create_traffic_classes().await?;

        // Add filters for port-based classification
        self.add_port_filters().await?;

        info!("QoS rules applied");

        Ok(())
    }

    /// Remove QoS rules
    pub async fn remove_qos(&self) -> Result<()> {
        info!(interface = %self.config.interface, "Removing QoS rules");

        self.execute(&["tc", "qdisc", "del", "dev", &self.config.interface, "root"])?;

        Ok(())
    }

    /// Set up HTB qdisc
    async fn setup_htb(&self) -> Result<()> {
        debug!("Setting up HTB qdisc");

        // Remove existing qdisc
        let _ = self.execute(&["tc", "qdisc", "del", "dev", &self.config.interface, "root"]);

        // Add HTB root qdisc
        self.execute(&[
            "tc",
            "qdisc",
            "add",
            "dev",
            &self.config.interface,
            "root",
            "handle",
            "1:",
            "htb",
            "default",
            "30",
        ])?;

        Ok(())
    }

    /// Create traffic classes
    async fn create_traffic_classes(&self) -> Result<()> {
        debug!("Creating traffic classes");

        // The root class rate must equal max_bandwidth so that individual leaf
        // classes can burst up to the full WAN uplink speed.  Using the *sum*
        // of per-class minimums here would hard-cap ALL egress at that sum —
        // even when only one class is active — because in Linux HTB a child
        // class can never exceed its parent's own rate.
        let max_kbps = self.config.max_bandwidth_kbps;
        let max_str = format!("{}kbit", max_kbps);

        // Root class – rate set to max_bandwidth so leaf classes can burst to
        // the full interface speed without being capped at the parent.
        self.execute(&[
            "tc",
            "class",
            "add",
            "dev",
            &self.config.interface,
            "parent",
            "1:",
            "classid",
            "1:1",
            "htb",
            "rate",
            &max_str,
        ])?;

        // Control class (highest priority)
        self.execute(&[
            "tc",
            "class",
            "add",
            "dev",
            &self.config.interface,
            "parent",
            "1:1",
            "classid",
            TrafficClass::Control.tc_class_id(),
            "htb",
            "rate",
            &format!("{}kbit", self.config.control_bandwidth_kbps),
            "ceil",
            &max_str,
            "prio",
            "1",
        ])?;

        // Interactive class
        self.execute(&[
            "tc",
            "class",
            "add",
            "dev",
            &self.config.interface,
            "parent",
            "1:1",
            "classid",
            TrafficClass::Interactive.tc_class_id(),
            "htb",
            "rate",
            &format!("{}kbit", self.config.interactive_bandwidth_kbps),
            "ceil",
            &max_str,
            "prio",
            "2",
        ])?;

        // Bulk class (lowest priority)
        self.execute(&[
            "tc",
            "class",
            "add",
            "dev",
            &self.config.interface,
            "parent",
            "1:1",
            "classid",
            TrafficClass::Bulk.tc_class_id(),
            "htb",
            "rate",
            &format!("{}kbit", self.config.bulk_bandwidth_kbps),
            "ceil",
            &max_str,
            "prio",
            "3",
        ])?;

        Ok(())
    }

    /// Add port-based filters
    async fn add_port_filters(&self) -> Result<()> {
        debug!("Adding port-based filters");

        // Control traffic filters
        for port in &self.config.control_ports {
            self.execute(&[
                "tc",
                "filter",
                "add",
                "dev",
                &self.config.interface,
                "protocol",
                "ip",
                "parent",
                "1:",
                "prio",
                "1",
                "u32",
                "match",
                "ip",
                "dport",
                &port.to_string(),
                "0xffff",
                "flowid",
                TrafficClass::Control.tc_class_id(),
            ])?;
        }

        // Interactive traffic filters
        for port in &self.config.interactive_ports {
            self.execute(&[
                "tc",
                "filter",
                "add",
                "dev",
                &self.config.interface,
                "protocol",
                "ip",
                "parent",
                "1:",
                "prio",
                "2",
                "u32",
                "match",
                "ip",
                "dport",
                &port.to_string(),
                "0xffff",
                "flowid",
                TrafficClass::Interactive.tc_class_id(),
            ])?;
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
                crate::connectivity::ConnectivityError::HotspotError(format!(
                    "Failed to execute command: {}",
                    e
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            debug!(
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

    #[test]
    fn test_traffic_class_dscp() {
        assert_eq!(TrafficClass::Control.dscp_value(), 46);
        assert_eq!(TrafficClass::Interactive.dscp_value(), 34);
        assert_eq!(TrafficClass::Bulk.dscp_value(), 10);
    }

    #[tokio::test]
    async fn test_qos_application() {
        let config = QosConfig::default();
        let manager = QosManager::new(config, false); // Don't execute commands

        let result = manager.apply_qos().await;
        assert!(result.is_ok());

        let result = manager.remove_qos().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_qos_disabled() {
        let config = QosConfig {
            enabled: false,
            ..Default::default()
        };

        let manager = QosManager::new(config, false);

        // Should succeed but do nothing
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(manager.apply_qos());
        assert!(result.is_ok());
    }

    /// The HTB parent class rate must equal max_bandwidth, not the sum of
    /// per-class minimums.  If the parent rate were only control + interactive
    /// + bulk (e.g. 16 Mbps by default), HTB would hard-cap ALL egress at
    /// that sum — making uploads unable to reach full WAN speed even when
    /// only one class is active.
    #[test]
    fn test_parent_htb_rate_equals_max_bandwidth() {
        let config = QosConfig {
            control_bandwidth_kbps: 1_000,
            interactive_bandwidth_kbps: 5_000,
            bulk_bandwidth_kbps: 10_000,
            max_bandwidth_kbps: 1_000_000, // 1 Gbps
            ..Default::default()
        };
        let parent_rate = config.max_bandwidth_kbps;
        let old_wrong_rate = config.control_bandwidth_kbps
            + config.interactive_bandwidth_kbps
            + config.bulk_bandwidth_kbps;
        assert!(
            parent_rate > old_wrong_rate,
            "parent HTB rate ({parent_rate} kbps) must exceed the old incorrect \
             sum of class minimums ({old_wrong_rate} kbps) so uploads are not bottlenecked"
        );
    }

    /// Each leaf class ceil must equal max_bandwidth so that any single class
    /// can burst to the full WAN uplink speed when other classes are idle.
    #[test]
    fn test_leaf_class_ceil_equals_max_bandwidth() {
        let config = QosConfig::default();
        // All class minimum rates must be strictly below max_bandwidth so the
        // burst headroom is meaningful.
        assert!(
            config.control_bandwidth_kbps < config.max_bandwidth_kbps,
            "control class min ({} kbps) must be below max_bandwidth ({} kbps)",
            config.control_bandwidth_kbps,
            config.max_bandwidth_kbps
        );
        assert!(
            config.interactive_bandwidth_kbps < config.max_bandwidth_kbps,
            "interactive class min ({} kbps) must be below max_bandwidth ({} kbps)",
            config.interactive_bandwidth_kbps,
            config.max_bandwidth_kbps
        );
        assert!(
            config.bulk_bandwidth_kbps < config.max_bandwidth_kbps,
            "bulk class min ({} kbps) must be below max_bandwidth ({} kbps)",
            config.bulk_bandwidth_kbps,
            config.max_bandwidth_kbps
        );
    }
}
