//! Multi-backhaul orchestration module
//!
//! This module ties together interface discovery, health probing, scoring,
//! state machines, and routing to provide a unified backhaul management system.

use crate::connectivity::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

pub mod discovery;
pub mod health;
pub mod routing;
pub mod scoring;
pub mod state_machine;

pub use discovery::{InterfaceDiscovery, InterfaceInfo, InterfaceRegistry, InterfaceType};
pub use health::{HealthProber, HealthStats, ProbeConfig};
pub use routing::{RoutingConfig, RoutingManager};
pub use scoring::{InterfaceScore, InterfaceScorer, ScoringConfig};
pub use state_machine::{
    InterfaceState as StateMachineState, InterfaceStateMachine, StateEvent, StateMachineConfig,
};

/// Backhaul state for public API
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackhaulState {
    Up,
    Degraded,
    Down,
}

impl From<StateMachineState> for BackhaulState {
    fn from(state: StateMachineState) -> Self {
        match state {
            StateMachineState::Up => BackhaulState::Up,
            StateMachineState::Degraded => BackhaulState::Degraded,
            StateMachineState::Down | StateMachineState::Probing => BackhaulState::Down,
        }
    }
}

/// Active backhaul information for public API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveBackhaul {
    pub iface: String,
    pub state: BackhaulState,
    pub score: u32,
}

/// Complete backhaul manager configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BackhaulConfig {
    pub probe_config: ProbeConfig,
    pub scoring_config: ScoringConfig,
    pub state_machine_config: StateMachineConfig,
    pub routing_config: RoutingConfig,
}

/// Per-interface state tracking
struct InterfaceState {
    info: InterfaceInfo,
    prober: HealthProber,
    state_machine: InterfaceStateMachine,
    last_score: u32,
}

/// Main backhaul manager
pub struct BackhaulManager {
    config: BackhaulConfig,
    registry: Arc<InterfaceRegistry>,
    discovery: InterfaceDiscovery,
    scorer: InterfaceScorer,
    routing: Arc<RwLock<RoutingManager>>,
    interface_states: Arc<RwLock<HashMap<String, InterfaceState>>>,
    active_interface: Arc<RwLock<Option<String>>>,
}

impl BackhaulManager {
    /// Create a new backhaul manager
    pub fn new(config: BackhaulConfig) -> Self {
        let registry = Arc::new(InterfaceRegistry::new());
        let discovery = InterfaceDiscovery::new(registry.clone());
        let scorer = InterfaceScorer::new(config.scoring_config.clone());
        let routing = Arc::new(RwLock::new(RoutingManager::new(
            config.routing_config.clone(),
        )));

        Self {
            config,
            registry,
            discovery,
            scorer,
            routing,
            interface_states: Arc::new(RwLock::new(HashMap::new())),
            active_interface: Arc::new(RwLock::new(None)),
        }
    }

    /// Start the backhaul manager
    ///
    /// This spawns background tasks for:
    /// - Interface discovery
    /// - Health probing
    /// - State machine updates
    /// - Routing updates
    pub async fn start(&self) -> Result<()> {
        info!("Starting backhaul manager");

        // Start interface discovery
        self.discovery.start_monitoring().await?;

        // Spawn main management loop
        let manager = self.clone();
        tokio::spawn(async move {
            if let Err(e) = manager.management_loop().await {
                warn!(error = %e, "Management loop terminated");
            }
        });

        Ok(())
    }

    /// Main management loop
    async fn management_loop(&self) -> Result<()> {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(
            self.config.probe_config.interval_secs,
        ));

        loop {
            interval.tick().await;

            if let Err(e) = self.management_iteration().await {
                warn!(error = %e, "Management iteration failed");
            }
        }
    }

    /// Single iteration of management loop
    async fn management_iteration(&self) -> Result<()> {
        // Get current WAN candidates
        let candidates = self.registry.get_wan_candidates().await;

        if candidates.is_empty() {
            debug!("No WAN candidates available");
            return Ok(());
        }

        let mut states = self.interface_states.write().await;

        // Update or create state for each candidate
        for candidate in &candidates {
            if !states.contains_key(&candidate.name) {
                // New interface - create state; bind probes to the interface's
                // own IPv4 address so each probe travels through that interface.
                let local_ip = candidate.ipv4_addresses.first().cloned();
                let prober = {
                    let p =
                        HealthProber::new(candidate.name.clone(), self.config.probe_config.clone());
                    if let Some(ip) = local_ip {
                        p.with_local_addr(ip)
                    } else {
                        p
                    }
                };
                let state_machine = InterfaceStateMachine::new(
                    candidate.name.clone(),
                    self.config.state_machine_config.clone(),
                );

                states.insert(
                    candidate.name.clone(),
                    InterfaceState {
                        info: candidate.clone(),
                        prober,
                        state_machine,
                        last_score: 0,
                    },
                );

                info!(interface = %candidate.name, "Registered new WAN candidate");
            }
        }

        // Probe all interfaces and update state machines
        for (name, iface_state) in states.iter_mut() {
            // Perform health probes
            let _results = iface_state.prober.probe_once().await;

            // Determine state event based on health
            let event = if iface_state.prober.is_healthy() {
                StateEvent::HealthyProbe
            } else if iface_state.prober.is_degraded() {
                StateEvent::DegradedProbe
            } else {
                StateEvent::FailedProbe
            };

            // Update state machine
            iface_state.state_machine.process_event(event);

            // Update score
            let score = self
                .scorer
                .score(&iface_state.info, iface_state.prober.stats());
            iface_state.last_score = score.total;

            debug!(
                interface = %name,
                state = ?iface_state.state_machine.state(),
                score = score.total,
                "Interface status"
            );
        }

        // Select best interface
        self.select_best_interface(&states).await?;

        Ok(())
    }

    /// Select and activate the best available interface
    async fn select_best_interface(&self, states: &HashMap<String, InterfaceState>) -> Result<()> {
        // Filter to UP interfaces only
        let up_interfaces: Vec<_> = states
            .iter()
            .filter(|(_, state)| state.state_machine.state() == StateMachineState::Up)
            .collect();

        if up_interfaces.is_empty() {
            debug!("No UP interfaces available");
            return Ok(());
        }

        // Find highest scoring interface
        let best = up_interfaces
            .iter()
            .max_by_key(|(_, state)| state.last_score)
            .map(|(name, _)| name.as_str());

        if let Some(best_interface) = best {
            let current_active = self.active_interface.read().await;

            // Only switch if different
            if current_active.as_deref() != Some(best_interface) {
                let old_interface = current_active.as_deref().map(|s| s.to_string());
                drop(current_active); // Release read lock

                info!(
                    old = ?old_interface,
                    new = best_interface,
                    "Switching active backhaul interface"
                );

                // Update routing
                let mut routing = self.routing.write().await;
                let source_ip = if let Some(state) = states.get(best_interface) {
                    state.info.ipv4_addresses.first().cloned()
                } else {
                    None
                };
                routing.switch_active_interface(best_interface, None, source_ip)?;
                drop(routing);

                // Update active interface
                let mut active = self.active_interface.write().await;
                *active = Some(best_interface.to_string());
            }
        }

        Ok(())
    }

    /// Get current active backhaul (public API)
    pub async fn current_backhaul(&self) -> Option<ActiveBackhaul> {
        let active = self.active_interface.read().await;
        let active_name = active.as_ref()?;

        let states = self.interface_states.read().await;
        let iface_state = states.get(active_name)?;

        Some(ActiveBackhaul {
            iface: active_name.clone(),
            state: BackhaulState::from(iface_state.state_machine.state()),
            score: iface_state.last_score,
        })
    }

    /// Get all interface states (for debugging/monitoring)
    pub async fn get_all_interface_states(&self) -> Vec<(String, BackhaulState, u32)> {
        let states = self.interface_states.read().await;
        states
            .iter()
            .map(|(name, state)| {
                (
                    name.clone(),
                    BackhaulState::from(state.state_machine.state()),
                    state.last_score,
                )
            })
            .collect()
    }

    /// Shutdown the backhaul manager
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down backhaul manager");

        let mut routing = self.routing.write().await;
        routing.cleanup_all()?;

        Ok(())
    }
}

impl Clone for BackhaulManager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            registry: self.registry.clone(),
            discovery: InterfaceDiscovery::new(self.registry.clone()),
            scorer: InterfaceScorer::new(self.config.scoring_config.clone()),
            routing: self.routing.clone(),
            interface_states: self.interface_states.clone(),
            active_interface: self.active_interface.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_backhaul_manager_creation() {
        let mut config = BackhaulConfig::default();
        config.routing_config.execute_commands = false;

        let manager = BackhaulManager::new(config);

        let backhaul = manager.current_backhaul().await;
        assert!(backhaul.is_none()); // No interfaces yet
    }

    #[tokio::test]
    async fn test_backhaul_manager_start() {
        let mut config = BackhaulConfig::default();
        config.routing_config.execute_commands = false;

        let manager = BackhaulManager::new(config);

        let result = manager.start().await;
        assert!(result.is_ok());

        // Give it time to discover interfaces
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Should have some interfaces discovered (mock or real)
        let all_states = manager.get_all_interface_states().await;
        debug!("Discovered {} interfaces", all_states.len());
    }
}
