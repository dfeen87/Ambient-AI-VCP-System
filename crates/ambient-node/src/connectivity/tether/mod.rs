//! Tether detection and management
//!
//! Phase 2 functionality for USB and Bluetooth tethering

pub mod bluetooth;
pub mod usb;

pub use bluetooth::{BluetoothTether, BluetoothTetherConfig};
pub use usb::{UsbTether, UsbTetherConfig};

use serde::{Deserialize, Serialize};

/// Tether policy profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TetherPolicy {
    /// Use tether freely
    Unrestricted,
    /// Use tether only in emergency (when no other backhaul available)
    EmergencyOnly,
    /// Metered usage with data budget
    Metered { budget_mb: u64 },
}

/// Tether policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TetherPolicyConfig {
    /// Policy profile
    pub policy: TetherPolicy,

    /// Enable battery-aware mode
    pub battery_aware: bool,

    /// Probe frequency reduction factor for battery-aware mode
    pub battery_aware_probe_reduction: u32,

    /// Data usage tracking
    pub track_data_usage: bool,
}

impl Default for TetherPolicyConfig {
    fn default() -> Self {
        Self {
            policy: TetherPolicy::Metered { budget_mb: 1024 }, // 1 GB
            battery_aware: true,
            battery_aware_probe_reduction: 2,
            track_data_usage: true,
        }
    }
}

/// Data usage tracker
pub struct DataUsageTracker {
    total_bytes: u64,
    budget_bytes: Option<u64>,
}

impl DataUsageTracker {
    pub fn new(budget_mb: Option<u64>) -> Self {
        Self {
            total_bytes: 0,
            budget_bytes: budget_mb.map(|mb| mb * 1024 * 1024),
        }
    }

    /// Record data usage
    pub fn record_usage(&mut self, bytes: u64) {
        self.total_bytes += bytes;
    }

    /// Get total usage in MB
    pub fn total_usage_mb(&self) -> u64 {
        self.total_bytes / (1024 * 1024)
    }

    /// Check if budget exceeded
    pub fn is_budget_exceeded(&self) -> bool {
        if let Some(budget) = self.budget_bytes {
            self.total_bytes >= budget
        } else {
            false
        }
    }

    /// Get remaining budget in MB
    pub fn remaining_budget_mb(&self) -> Option<u64> {
        self.budget_bytes.map(|budget| {
            let remaining = budget.saturating_sub(self.total_bytes);
            remaining / (1024 * 1024)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_usage_tracker() {
        let mut tracker = DataUsageTracker::new(Some(100)); // 100 MB budget

        assert_eq!(tracker.total_usage_mb(), 0);
        assert!(!tracker.is_budget_exceeded());

        // Use 50 MB
        tracker.record_usage(50 * 1024 * 1024);
        assert_eq!(tracker.total_usage_mb(), 50);
        assert_eq!(tracker.remaining_budget_mb(), Some(50));
        assert!(!tracker.is_budget_exceeded());

        // Use another 60 MB (total 110 MB, exceeding budget)
        tracker.record_usage(60 * 1024 * 1024);
        assert_eq!(tracker.total_usage_mb(), 110);
        assert!(tracker.is_budget_exceeded());
    }

    #[test]
    fn test_unlimited_budget() {
        let mut tracker = DataUsageTracker::new(None);

        tracker.record_usage(1024 * 1024 * 1024); // 1 GB
        assert!(!tracker.is_budget_exceeded());
        assert!(tracker.remaining_budget_mb().is_none());
    }
}
