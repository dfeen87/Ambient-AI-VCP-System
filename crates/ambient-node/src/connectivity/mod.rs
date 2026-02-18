//! Connectivity layer for universal/open-access nodes
//!
//! This module provides multi-backhaul orchestration for distributed systems,
//! managing WAN connectivity through multiple interface types:
//! - Ethernet
//! - Wi-Fi client mode
//! - LTE/5G modem
//! - USB tethering (usb0, enx*)
//! - Bluetooth PAN (bnep0)
//! - (Phase 2) Wi-Fi AP mode for hotspot fallback
//!
//! The connectivity layer is responsible ONLY for:
//! - Maintaining WAN connectivity
//! - Relaying traffic
//! - Staying reachable under changing network conditions
//!
//! It does NOT handle application tasks, policy logic, or WASM workloads.

use thiserror::Error;

pub mod backhaul;
pub mod hotspot;
pub mod tether;

pub use backhaul::{ActiveBackhaul, BackhaulManager, BackhaulState};

/// Connectivity layer errors
#[derive(Debug, Error)]
pub enum ConnectivityError {
    #[error("Interface discovery failed: {0}")]
    DiscoveryError(String),

    #[error("Health probe failed: {0}")]
    ProbeError(String),

    #[error("Routing operation failed: {0}")]
    RoutingError(String),

    #[error("Hotspot operation failed: {0}")]
    HotspotError(String),

    #[error("Tether operation failed: {0}")]
    TetherError(String),

    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Netlink error: {0}")]
    NetlinkError(String),

    #[error("State machine error: {0}")]
    StateMachineError(String),
}

pub type Result<T> = std::result::Result<T, ConnectivityError>;
