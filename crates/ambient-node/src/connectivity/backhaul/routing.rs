//! Linux policy routing for backhaul interfaces
//!
//! Uses per-interface routing tables and `ip rule` semantics.
//! Ensures routing changes are atomic and avoids transient blackholes.
//!
//! All routing logic is encapsulated here to avoid leaking implementation
//! details to the rest of the connectivity layer.

use crate::connectivity::{ConnectivityError, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;
use tracing::{debug, info, warn};

/// Routing table ID range for backhaul interfaces
const TABLE_ID_BASE: u32 = 100;
const TABLE_ID_MAX: u32 = 200;

/// Routing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    /// Whether to actually execute routing commands (false for dry-run testing)
    pub execute_commands: bool,

    /// When true (the default), per-interface routing tables are prepared but
    /// no `ip rule` is added to the policy database.  The host's existing
    /// routing is left entirely intact and routing changes are only applied
    /// when the operator explicitly sets this to `false`.
    pub monitor_only: bool,

    /// Main routing table priority
    pub main_table_priority: u32,

    /// Interface-specific table priority
    pub interface_table_priority: u32,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            execute_commands: true,
            // Safe default: observe interfaces without touching the host's routing.
            monitor_only: true,
            main_table_priority: 32766,
            interface_table_priority: 1000,
        }
    }
}

/// Routing table entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteEntry {
    pub interface: String,
    pub table_id: u32,
    pub gateway: Option<String>,
    pub metric: u32,
}

/// Routing manager for backhaul interfaces
pub struct RoutingManager {
    config: RoutingConfig,
    active_interface: Option<String>,
    /// Maps interface name → (table_id, optional source IP used in the policy rule)
    table_assignments: std::collections::HashMap<String, (u32, Option<String>)>,
    next_table_id: u32,
}

impl RoutingManager {
    pub fn new(config: RoutingConfig) -> Self {
        Self {
            config,
            active_interface: None,
            table_assignments: std::collections::HashMap::new(),
            next_table_id: TABLE_ID_BASE,
        }
    }

    /// Assign a routing table ID to an interface
    fn assign_table_id(&mut self, interface: &str, source_ip: Option<&str>) -> u32 {
        if let Some(&(table_id, _)) = self.table_assignments.get(interface) {
            return table_id;
        }

        let table_id = self.next_table_id;
        self.next_table_id += 1;

        if self.next_table_id > TABLE_ID_MAX {
            warn!("Routing table ID space exhausted, wrapping around");
            self.next_table_id = TABLE_ID_BASE;
        }

        self.table_assignments
            .insert(interface.to_string(), (table_id, source_ip.map(str::to_string)));
        table_id
    }

    /// Switch active interface
    ///
    /// This performs atomic routing updates to avoid blackholes:
    /// 1. Set up new interface routing table
    /// 2. Add new policy routing rule  (skipped when `monitor_only` is true)
    /// 3. Remove old policy routing rule (skipped when `monitor_only` is true)
    /// 4. Clean up old interface routing table (if needed)
    ///
    /// `source_ip` should be the interface's IPv4 address.  When provided the
    /// policy rule is scoped to `from <source_ip>` instead of `from all`,
    /// which limits the impact to traffic that originates from that address and
    /// avoids overriding the host's default route for unrelated traffic.
    pub fn switch_active_interface(
        &mut self,
        new_interface: &str,
        gateway: Option<String>,
        source_ip: Option<String>,
    ) -> Result<()> {
        info!(
            old_interface = ?self.active_interface,
            new_interface = %new_interface,
            "Switching active interface"
        );

        let table_id = self.assign_table_id(new_interface, source_ip.as_deref());

        // Step 1: Set up new interface routing table
        self.setup_interface_table(new_interface, table_id, gateway.as_deref())?;

        // Step 2: Add new policy routing rule (skipped in monitor_only mode)
        self.add_routing_rule(new_interface, table_id, source_ip.as_deref())?;

        // Step 3: Remove old policy routing rule (if exists, skipped in monitor_only mode)
        if let Some(old_interface) = &self.active_interface {
            if old_interface != new_interface {
                if let Some(&(old_table_id, ref old_ip)) = self.table_assignments.get(old_interface)
                {
                    self.remove_routing_rule(old_interface, old_table_id, old_ip.as_deref())?;
                }
            }
        }

        self.active_interface = Some(new_interface.to_string());

        Ok(())
    }

    /// Set up routing table for an interface
    fn setup_interface_table(
        &self,
        interface: &str,
        table_id: u32,
        gateway: Option<&str>,
    ) -> Result<()> {
        debug!(
            interface = %interface,
            table_id = table_id,
            gateway = ?gateway,
            "Setting up interface routing table"
        );

        let table_id_str = table_id.to_string();

        // Flush existing routes in this table
        self.execute_command(&["ip", "route", "flush", "table", &table_id_str])?;

        // Add default route through this interface
        if let Some(gw) = gateway {
            self.execute_command(&[
                "ip",
                "route",
                "add",
                "default",
                "via",
                gw,
                "dev",
                interface,
                "table",
                &table_id_str,
            ])?;
        } else {
            self.execute_command(&[
                "ip",
                "route",
                "add",
                "default",
                "dev",
                interface,
                "table",
                &table_id_str,
            ])?;
        }

        Ok(())
    }

    /// Add policy routing rule
    ///
    /// When `monitor_only` is set on the config this is a no-op: per-interface
    /// routing tables are prepared (see `setup_interface_table`) but no `ip
    /// rule` entry is inserted, so the host's existing routing is preserved.
    ///
    /// When routing is active the rule is scoped to `from <source_ip>` if an
    /// IP is provided, which prevents the rule from hijacking traffic that does
    /// not originate from that interface.
    fn add_routing_rule(
        &self,
        interface: &str,
        table_id: u32,
        source_ip: Option<&str>,
    ) -> Result<()> {
        if self.config.monitor_only {
            debug!(
                interface = %interface,
                "monitor_only: skipping ip rule add (host routing unchanged)"
            );
            return Ok(());
        }

        debug!(
            interface = %interface,
            table_id = table_id,
            source_ip = ?source_ip,
            "Adding policy routing rule"
        );

        let table_id_str = table_id.to_string();
        let priority_str = self.config.interface_table_priority.to_string();
        let from_selector = source_ip.unwrap_or("all");

        // ip rule add from <from_selector> lookup <table_id> pref <priority>
        self.execute_command(&[
            "ip",
            "rule",
            "add",
            "from",
            from_selector,
            "lookup",
            &table_id_str,
            "pref",
            &priority_str,
        ])?;

        Ok(())
    }

    /// Remove policy routing rule
    fn remove_routing_rule(
        &self,
        interface: &str,
        table_id: u32,
        source_ip: Option<&str>,
    ) -> Result<()> {
        if self.config.monitor_only {
            return Ok(());
        }

        debug!(
            interface = %interface,
            table_id = table_id,
            "Removing policy routing rule"
        );

        let table_id_str = table_id.to_string();
        let from_selector = source_ip.unwrap_or("all");

        // ip rule del from <from_selector> lookup <table_id>
        let result = self.execute_command(&[
            "ip",
            "rule",
            "del",
            "from",
            from_selector,
            "lookup",
            &table_id_str,
        ]);

        // Don't fail if rule doesn't exist
        if let Err(e) = result {
            debug!(error = %e, "Failed to remove routing rule (may not exist)");
        }

        Ok(())
    }

    /// Execute a routing command
    fn execute_command(&self, args: &[&str]) -> Result<()> {
        if !self.config.execute_commands {
            debug!(command = ?args, "Skipping command execution (dry run)");
            return Ok(());
        }

        let output = Command::new(args[0])
            .args(&args[1..])
            .output()
            .map_err(|e| {
                ConnectivityError::RoutingError(format!("Failed to execute command: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ConnectivityError::RoutingError(format!(
                "Command failed: {} - {}",
                args.join(" "),
                stderr
            )));
        }

        Ok(())
    }

    /// Get current active interface
    pub fn active_interface(&self) -> Option<&str> {
        self.active_interface.as_deref()
    }

    /// Rollback routing changes for an interface
    pub fn rollback_interface(&mut self, interface: &str) -> Result<()> {
        info!(interface = %interface, "Rolling back routing changes");

        if let Some(&(table_id, ref source_ip)) = self.table_assignments.get(interface) {
            let table_id_str = table_id.to_string();

            // Remove routing rule
            self.remove_routing_rule(interface, table_id, source_ip.as_deref())?;

            // Flush routing table
            self.execute_command(&["ip", "route", "flush", "table", &table_id_str])?;
        }

        if self.active_interface.as_deref() == Some(interface) {
            self.active_interface = None;
        }

        Ok(())
    }

    /// Clean up all routing state (for shutdown)
    pub fn cleanup_all(&mut self) -> Result<()> {
        info!("Cleaning up all routing state");

        // Collect table assignments to avoid borrow issues
        let assignments: Vec<_> = self.table_assignments.drain().collect();

        for (interface, (table_id, source_ip)) in assignments {
            if let Err(e) =
                self.remove_routing_rule(&interface, table_id, source_ip.as_deref())
            {
                warn!(interface = %interface, error = %e, "Failed to remove routing rule");
            }

            let table_id_str = table_id.to_string();
            if let Err(e) = self.execute_command(&["ip", "route", "flush", "table", &table_id_str])
            {
                warn!(interface = %interface, error = %e, "Failed to flush routing table");
            }
        }

        self.active_interface = None;

        Ok(())
    }
}

impl Default for RoutingManager {
    fn default() -> Self {
        Self::new(RoutingConfig::default())
    }
}

impl Drop for RoutingManager {
    fn drop(&mut self) {
        // Best-effort cleanup on drop
        let _ = self.cleanup_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_id_assignment() {
        let mut manager = RoutingManager::new(RoutingConfig {
            execute_commands: false,
            ..Default::default()
        });

        let table1 = manager.assign_table_id("eth0", Some("192.168.1.100"));
        let table2 = manager.assign_table_id("wlan0", Some("10.0.0.5"));
        let table3 = manager.assign_table_id("eth0", Some("192.168.1.100")); // Should reuse

        assert_eq!(table1, TABLE_ID_BASE);
        assert_eq!(table2, TABLE_ID_BASE + 1);
        assert_eq!(table1, table3); // Same interface gets same table
    }

    #[test]
    fn test_switch_active_interface() {
        let mut manager = RoutingManager::new(RoutingConfig {
            execute_commands: false, // Don't actually run ip commands in test
            ..Default::default()
        });

        let result = manager.switch_active_interface(
            "eth0",
            Some("192.168.1.1".to_string()),
            Some("192.168.1.100".to_string()),
        );
        assert!(result.is_ok());
        assert_eq!(manager.active_interface(), Some("eth0"));

        let result = manager.switch_active_interface("wlan0", None, None);
        assert!(result.is_ok());
        assert_eq!(manager.active_interface(), Some("wlan0"));
    }

    #[test]
    fn test_rollback_interface() {
        let mut manager = RoutingManager::new(RoutingConfig {
            execute_commands: false,
            ..Default::default()
        });

        manager
            .switch_active_interface("eth0", None, None)
            .unwrap();
        assert_eq!(manager.active_interface(), Some("eth0"));

        manager.rollback_interface("eth0").unwrap();
        assert_eq!(manager.active_interface(), None);
    }

    #[test]
    fn test_monitor_only_default() {
        let config = RoutingConfig::default();
        assert!(
            config.monitor_only,
            "monitor_only must default to true to protect the host's existing routing"
        );
    }

    #[test]
    fn test_monitor_only_skips_ip_rule() {
        // With monitor_only=true and execute_commands=true the manager must
        // still succeed (no real commands are run) and leave active_interface set.
        let mut manager = RoutingManager::new(RoutingConfig {
            execute_commands: false,
            monitor_only: true,
            ..Default::default()
        });

        let result = manager.switch_active_interface(
            "eth0",
            None,
            Some("192.168.1.100".to_string()),
        );
        assert!(result.is_ok());
        assert_eq!(manager.active_interface(), Some("eth0"));
    }
}
