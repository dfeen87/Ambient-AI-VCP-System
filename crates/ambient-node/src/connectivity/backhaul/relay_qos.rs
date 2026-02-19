//! WAN-side QoS for connect_only relay sessions
//!
//! When an `open_internet` or global-`any` node runs a `connect_only` task it
//! acts as an internet relay: client traffic flows **through** the node to the
//! WAN backhaul interface and on to the destination.  Without QoS, node-local
//! traffic (health probes, control-plane messages, other workloads) can compete
//! with the relay stream and increase latency or reduce throughput.
//!
//! This module installs lightweight Linux `tc` rules on the **active WAN
//! backhaul interface** whenever a relay session is active:
//!
//! 1. An **HTB root qdisc** with two leaf classes:
//!    - `1:10` – relay traffic (high priority, guaranteed minimum bandwidth,
//!      burst ceiling limited only by interface speed)
//!    - `1:20` – all other node traffic (lower priority, best-effort)
//! 2. An optional **FQ-CoDel** qdisc on the relay class to actively manage
//!    queue depth and keep per-flow latency low (reduces bufferbloat).
//! 3. A **DSCP/TOS u32 filter** that steers packets already marked with the
//!    configured DSCP value (default EF = 46) into the relay class.  The HTB
//!    default class is also set to `1:10` so that unmarked relay TCP connections
//!    benefit from the same prioritisation without requiring end-to-end DSCP
//!    support.
//!
//! When no relay session is active the rules are torn down, restoring the
//! interface to its default queuing behaviour.

use crate::connectivity::{ConnectivityError, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;
use tracing::{debug, info};

/// QoS configuration for relay sessions on the WAN backhaul interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayQosConfig {
    /// Enable WAN-side relay QoS.  When `false` all methods are no-ops.
    pub enabled: bool,

    /// Minimum bandwidth guaranteed for relay session traffic (kbps).
    ///
    /// This is the HTB `rate` for the relay class.  Relay traffic will always
    /// receive at least this much bandwidth even when node-local traffic is
    /// competing.
    pub relay_min_bandwidth_kbps: u32,

    /// Maximum bandwidth ceiling for relay session traffic (kbps).
    ///
    /// This is the HTB `ceil` for the relay class.  Relay traffic can burst up
    /// to this limit when the interface has spare capacity.  Set to a very high
    /// value (default 1 Gbps) to let the relay use all available bandwidth.
    pub relay_max_bandwidth_kbps: u32,

    /// Bandwidth reserved for node-internal traffic (kbps).
    ///
    /// This is the HTB `rate` for the node-internal class.  The node always
    /// keeps at least this much bandwidth for its own control-plane and health
    /// traffic so relay sessions cannot completely starve the node.
    pub node_min_bandwidth_kbps: u32,

    /// Attach an FQ-CoDel qdisc to the relay class.
    ///
    /// FQ-CoDel (Fair Queuing Controlled Delay) provides active queue
    /// management that keeps per-flow queue depth short and minimises induced
    /// latency.  Strongly recommended when the relay carries latency-sensitive
    /// traffic such as HTTPS or interactive sessions.
    pub use_fq_codel: bool,

    /// DSCP value used to classify egress packets as relay traffic.
    ///
    /// Packets with this DSCP value in their IP TOS field are steered into the
    /// high-priority relay HTB class.  The default is `46` (Expedited
    /// Forwarding / EF), which is also what most VPN and proxy clients mark
    /// for interactive/latency-sensitive traffic.
    ///
    /// Even without DSCP marking, relay connections benefit because the HTB
    /// default class is set to the relay class (`1:10`).
    pub relay_dscp: u8,
}

impl Default for RelayQosConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            relay_min_bandwidth_kbps: 10_000, // 10 Mbps guaranteed
            relay_max_bandwidth_kbps: 1_000_000, // 1 Gbps ceiling
            node_min_bandwidth_kbps: 1_000,   // 1 Mbps reserved for node traffic
            use_fq_codel: true,
            relay_dscp: 46, // EF – Expedited Forwarding
        }
    }
}

/// Manages WAN-side QoS prioritisation for connect_only relay sessions.
///
/// Callers should call [`RelayQosManager::activate_on_interface`] when a
/// relay session starts and [`RelayQosManager::deactivate_from_interface`]
/// when it ends.  Both operations are idempotent and safe to call multiple
/// times.
pub struct RelayQosManager {
    config: RelayQosConfig,
    /// When `false`, `tc` commands are logged but never executed (dry-run /
    /// test mode).
    execute_commands: bool,
}

impl RelayQosManager {
    /// Create a new manager.
    ///
    /// Set `execute_commands` to `false` for tests or dry-run environments
    /// where actual `tc` invocations must be skipped.
    pub fn new(config: RelayQosConfig, execute_commands: bool) -> Self {
        Self {
            config,
            execute_commands,
        }
    }

    /// Apply relay QoS rules on a WAN backhaul interface.
    ///
    /// Installs an HTB qdisc with relay-priority leaf classes and an optional
    /// FQ-CoDel qdisc on the relay class.  Any existing root qdisc on the
    /// interface is removed first so the operation is idempotent.
    ///
    /// # Arguments
    ///
    /// * `interface` – name of the active WAN backhaul interface (e.g.
    ///   `"eth0"`, `"wwan0"`).
    pub async fn activate_on_interface(&self, interface: &str) -> Result<()> {
        if !self.config.enabled {
            debug!("Relay QoS disabled, skipping activation");
            return Ok(());
        }

        info!(
            interface = %interface,
            relay_min_kbps = self.config.relay_min_bandwidth_kbps,
            relay_max_kbps = self.config.relay_max_bandwidth_kbps,
            use_fq_codel = self.config.use_fq_codel,
            "Activating relay QoS on WAN backhaul interface"
        );

        // Remove any pre-existing root qdisc – failure is expected when none
        // exists, so we ignore the result.
        let _ = self.execute(&["tc", "qdisc", "del", "dev", interface, "root"]);

        let total_kbps = self
            .config
            .relay_min_bandwidth_kbps
            .saturating_add(self.config.node_min_bandwidth_kbps);

        let total_str = format!("{}kbit", total_kbps);
        let relay_min_str = format!("{}kbit", self.config.relay_min_bandwidth_kbps);
        let relay_max_str = format!("{}kbit", self.config.relay_max_bandwidth_kbps);
        let node_min_str = format!("{}kbit", self.config.node_min_bandwidth_kbps);

        // Root HTB qdisc.  Default class `10` means unclassified packets land
        // in the relay class – this handles unmarked relay TCP connections.
        self.execute(&[
            "tc", "qdisc", "add", "dev", interface, "root", "handle", "1:", "htb", "default", "10",
        ])?;

        // Root class – total guaranteed rate (floor for both leaf classes).
        self.execute(&[
            "tc", "class", "add", "dev", interface, "parent", "1:", "classid", "1:1", "htb",
            "rate", &total_str,
        ])?;

        // Relay class 1:10 – high priority, guaranteed minimum, can burst to
        // the ceiling (effectively the full interface speed).
        self.execute(&[
            "tc",
            "class",
            "add",
            "dev",
            interface,
            "parent",
            "1:1",
            "classid",
            "1:10",
            "htb",
            "rate",
            &relay_min_str,
            "ceil",
            &relay_max_str,
            "prio",
            "1",
        ])?;

        // Node-internal class 1:20 – lower priority, best-effort.
        self.execute(&[
            "tc",
            "class",
            "add",
            "dev",
            interface,
            "parent",
            "1:1",
            "classid",
            "1:20",
            "htb",
            "rate",
            &node_min_str,
            "ceil",
            &total_str,
            "prio",
            "2",
        ])?;

        // Attach FQ-CoDel to the relay leaf class for active queue management
        // and per-flow fairness, which reduces bufferbloat-induced latency.
        if self.config.use_fq_codel {
            self.execute(&[
                "tc", "qdisc", "add", "dev", interface, "parent", "1:10", "handle", "10:",
                "fq_codel",
            ])?;
        }

        // DSCP/TOS filter: steer DSCP-EF-marked packets into the relay class.
        // The TOS byte carries DSCP in its upper 6 bits (DSCP << 2).
        let dscp_tos = format!("0x{:02x}", self.config.relay_dscp << 2);
        let dscp_mask = "0xfc"; // mask for upper 6 bits

        self.execute(&[
            "tc", "filter", "add", "dev", interface, "protocol", "ip", "parent", "1:", "prio", "1",
            "u32", "match", "ip", "tos", &dscp_tos, dscp_mask, "flowid", "1:10",
        ])?;

        info!(interface = %interface, "Relay QoS activated");
        Ok(())
    }

    /// Remove relay QoS rules from a WAN backhaul interface.
    ///
    /// Deletes the root qdisc installed by [`activate_on_interface`], which
    /// implicitly removes all child classes and filters.  If no qdisc is
    /// present the operation succeeds silently.
    ///
    /// # Arguments
    ///
    /// * `interface` – name of the WAN backhaul interface.
    pub async fn deactivate_from_interface(&self, interface: &str) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        info!(interface = %interface, "Deactivating relay QoS from WAN backhaul interface");

        // Best-effort: ignore errors if the qdisc was already removed.
        let _ = self.execute(&["tc", "qdisc", "del", "dev", interface, "root"]);

        info!(interface = %interface, "Relay QoS deactivated");
        Ok(())
    }

    /// Execute a `tc` command.
    ///
    /// Failures are logged at DEBUG level and treated as non-fatal because some
    /// operations (e.g., deleting a non-existent qdisc) are expected to fail
    /// during normal operation.  Callers that need strict error handling should
    /// check the returned `Result` and handle it explicitly.
    fn execute(&self, args: &[&str]) -> Result<()> {
        if !self.execute_commands {
            debug!(command = ?args, "Skipping command execution (dry run)");
            return Ok(());
        }

        let output = Command::new(args[0])
            .args(&args[1..])
            .output()
            .map_err(|e| {
                ConnectivityError::RoutingError(format!("Failed to execute tc command: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            debug!(
                command = args.join(" "),
                stderr = %stderr,
                "tc command returned non-zero (non-fatal)"
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dry_run_manager() -> RelayQosManager {
        RelayQosManager::new(RelayQosConfig::default(), false)
    }

    #[tokio::test]
    async fn test_activate_dry_run_succeeds() {
        let manager = dry_run_manager();
        assert!(manager.activate_on_interface("eth0").await.is_ok());
    }

    #[tokio::test]
    async fn test_deactivate_dry_run_succeeds() {
        let manager = dry_run_manager();
        assert!(manager.deactivate_from_interface("eth0").await.is_ok());
    }

    #[tokio::test]
    async fn test_activate_deactivate_cycle() {
        let manager = dry_run_manager();
        assert!(manager.activate_on_interface("wwan0").await.is_ok());
        assert!(manager.deactivate_from_interface("wwan0").await.is_ok());
    }

    #[tokio::test]
    async fn test_disabled_config_is_noop() {
        // Even with execute_commands=true (would fail on a system without tc),
        // a disabled config must succeed without executing any commands.
        let config = RelayQosConfig {
            enabled: false,
            ..Default::default()
        };
        let manager = RelayQosManager::new(config, true);
        assert!(manager.activate_on_interface("eth0").await.is_ok());
        assert!(manager.deactivate_from_interface("eth0").await.is_ok());
    }

    #[test]
    fn test_default_config_values() {
        let config = RelayQosConfig::default();

        assert!(config.enabled);
        assert!(config.use_fq_codel);
        assert_eq!(config.relay_dscp, 46, "Default DSCP should be EF (46)");
        assert!(
            config.relay_min_bandwidth_kbps > 0,
            "Relay min bandwidth must be positive"
        );
        assert!(
            config.relay_max_bandwidth_kbps > config.relay_min_bandwidth_kbps,
            "Relay max bandwidth must exceed min bandwidth"
        );
        assert!(
            config.node_min_bandwidth_kbps > 0,
            "Node min bandwidth must be positive"
        );
    }

    #[test]
    fn test_dscp_tos_encoding() {
        // EF DSCP value 46 should map to TOS byte 0xb8 (46 << 2 = 184 = 0xb8)
        let config = RelayQosConfig::default();
        let dscp_tos = config.relay_dscp << 2;
        assert_eq!(dscp_tos, 0xb8u8);
    }

    #[test]
    fn test_custom_bandwidth_config() {
        let config = RelayQosConfig {
            relay_min_bandwidth_kbps: 50_000,
            relay_max_bandwidth_kbps: 500_000,
            node_min_bandwidth_kbps: 5_000,
            ..Default::default()
        };
        let manager = RelayQosManager::new(config, false);
        assert_eq!(manager.config.relay_min_bandwidth_kbps, 50_000);
        assert_eq!(manager.config.relay_max_bandwidth_kbps, 500_000);
        assert_eq!(manager.config.node_min_bandwidth_kbps, 5_000);
    }
}
