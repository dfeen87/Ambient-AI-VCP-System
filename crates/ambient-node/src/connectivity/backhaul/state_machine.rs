//! State machine for interface lifecycle
//!
//! State transitions: UP → DEGRADED → DOWN → PROBING → UP
//!
//! Uses hold-down timers to avoid flapping and ensures transitions
//! are debounced and time-based.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::info;

/// Interface state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterfaceState {
    /// Interface is up and healthy
    Up,
    /// Interface is experiencing degraded performance
    Degraded,
    /// Interface is down or unreachable
    Down,
    /// Interface is being probed to determine state
    Probing,
}

impl InterfaceState {
    pub fn as_str(&self) -> &'static str {
        match self {
            InterfaceState::Up => "UP",
            InterfaceState::Degraded => "DEGRADED",
            InterfaceState::Down => "DOWN",
            InterfaceState::Probing => "PROBING",
        }
    }
}

/// State transition event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateEvent {
    /// Health check passed
    HealthyProbe,
    /// Health check showed degradation
    DegradedProbe,
    /// Health check failed
    FailedProbe,
    /// Interface physically went down
    PhysicalDown,
    /// Interface physically came up
    PhysicalUp,
}

/// State machine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMachineConfig {
    /// Time to wait before transitioning from UP to DEGRADED
    pub up_to_degraded_holddown_secs: u64,

    /// Time to wait before transitioning from DEGRADED to DOWN
    pub degraded_to_down_holddown_secs: u64,

    /// Time to wait before transitioning from DOWN to PROBING
    pub down_to_probing_holddown_secs: u64,

    /// Time to wait before transitioning from PROBING to UP
    pub probing_to_up_holddown_secs: u64,

    /// Minimum time in a state before allowing transition
    pub min_state_duration_secs: u64,
}

impl Default for StateMachineConfig {
    fn default() -> Self {
        Self {
            up_to_degraded_holddown_secs: 15,
            degraded_to_down_holddown_secs: 30,
            down_to_probing_holddown_secs: 60,
            probing_to_up_holddown_secs: 10,
            min_state_duration_secs: 5,
        }
    }
}

/// Interface state machine
pub struct InterfaceStateMachine {
    interface: String,
    current_state: InterfaceState,
    state_entered_at: Instant,
    pending_event: Option<(StateEvent, Instant)>,
    config: StateMachineConfig,
}

impl InterfaceStateMachine {
    pub fn new(interface: String, config: StateMachineConfig) -> Self {
        Self {
            interface,
            current_state: InterfaceState::Probing,
            state_entered_at: Instant::now(),
            pending_event: None,
            config,
        }
    }

    /// Get current state
    pub fn state(&self) -> InterfaceState {
        self.current_state
    }

    /// Get time spent in current state
    pub fn time_in_state(&self) -> Duration {
        self.state_entered_at.elapsed()
    }

    /// Process a state event
    ///
    /// Returns true if state changed, false otherwise
    pub fn process_event(&mut self, event: StateEvent) -> bool {
        // Check if we've been in current state long enough
        if self.time_in_state().as_secs() < self.config.min_state_duration_secs {
            // Queue event for later processing
            self.pending_event = Some((event, Instant::now()));
            return false;
        }

        // Process immediate transitions (physical state changes)
        match event {
            StateEvent::PhysicalDown => {
                return self.transition_to(InterfaceState::Down);
            }
            StateEvent::PhysicalUp if self.current_state == InterfaceState::Down => {
                return self.transition_to(InterfaceState::Probing);
            }
            _ => {}
        }

        // Check if we have a pending event that should now be processed
        if let Some((pending_event, queued_at)) = self.pending_event {
            if queued_at.elapsed().as_secs() >= self.get_holddown_for_transition(pending_event) {
                self.pending_event = None;
                return self.process_transition(pending_event);
            }
        }

        // Process new event
        if event == StateEvent::PhysicalDown || event == StateEvent::PhysicalUp {
            // Already handled above
            false
        } else {
            self.queue_or_process_event(event)
        }
    }

    /// Queue event or process if holddown expired
    fn queue_or_process_event(&mut self, event: StateEvent) -> bool {
        let holddown = self.get_holddown_for_transition(event);

        // If holddown is 0, process immediately
        if holddown == 0 {
            return self.process_transition(event);
        }

        // Check if we already have a pending event for this transition
        if let Some((pending_event, queued_at)) = self.pending_event {
            if pending_event == event {
                // Same event - check if holddown expired
                if queued_at.elapsed().as_secs() >= holddown {
                    self.pending_event = None;
                    return self.process_transition(event);
                }
                return false;
            }
        }

        // Queue new event
        self.pending_event = Some((event, Instant::now()));
        false
    }

    /// Get holddown duration for a transition
    fn get_holddown_for_transition(&self, event: StateEvent) -> u64 {
        match (self.current_state, event) {
            (InterfaceState::Up, StateEvent::DegradedProbe) => {
                self.config.up_to_degraded_holddown_secs
            }
            (InterfaceState::Degraded, StateEvent::FailedProbe) => {
                self.config.degraded_to_down_holddown_secs
            }
            (InterfaceState::Down, StateEvent::HealthyProbe) => {
                self.config.down_to_probing_holddown_secs
            }
            (InterfaceState::Probing, StateEvent::HealthyProbe) => {
                self.config.probing_to_up_holddown_secs
            }
            _ => 0,
        }
    }

    /// Process state transition based on event
    fn process_transition(&mut self, event: StateEvent) -> bool {
        let next_state = match (self.current_state, event) {
            // UP state transitions
            (InterfaceState::Up, StateEvent::DegradedProbe) => Some(InterfaceState::Degraded),
            (InterfaceState::Up, StateEvent::FailedProbe) => Some(InterfaceState::Degraded),
            (InterfaceState::Up, StateEvent::HealthyProbe) => None, // Stay in UP

            // DEGRADED state transitions
            (InterfaceState::Degraded, StateEvent::HealthyProbe) => Some(InterfaceState::Up),
            (InterfaceState::Degraded, StateEvent::FailedProbe) => Some(InterfaceState::Down),
            (InterfaceState::Degraded, StateEvent::DegradedProbe) => None, // Stay in DEGRADED

            // DOWN state transitions
            (InterfaceState::Down, StateEvent::HealthyProbe) => Some(InterfaceState::Probing),
            (InterfaceState::Down, StateEvent::DegradedProbe) => Some(InterfaceState::Probing),
            (InterfaceState::Down, StateEvent::FailedProbe) => None, // Stay in DOWN

            // PROBING state transitions
            (InterfaceState::Probing, StateEvent::HealthyProbe) => Some(InterfaceState::Up),
            (InterfaceState::Probing, StateEvent::DegradedProbe) => Some(InterfaceState::Degraded),
            (InterfaceState::Probing, StateEvent::FailedProbe) => Some(InterfaceState::Down),

            _ => None,
        };

        if let Some(new_state) = next_state {
            self.transition_to(new_state)
        } else {
            false
        }
    }

    /// Transition to a new state
    fn transition_to(&mut self, new_state: InterfaceState) -> bool {
        if new_state != self.current_state {
            info!(
                interface = %self.interface,
                from_state = %self.current_state.as_str(),
                to_state = %new_state.as_str(),
                duration_secs = self.time_in_state().as_secs(),
                "State transition"
            );

            self.current_state = new_state;
            self.state_entered_at = Instant::now();
            self.pending_event = None;
            true
        } else {
            false
        }
    }

    /// Force state change (for testing or emergency)
    #[allow(dead_code)]
    pub fn force_state(&mut self, new_state: InterfaceState) {
        self.transition_to(new_state);
        // For tests, we want to allow immediate transitions after force_state
        // So we set the timestamp to an earlier time
        self.state_entered_at = Instant::now() - std::time::Duration::from_secs(100);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let config = StateMachineConfig::default();
        let sm = InterfaceStateMachine::new("eth0".to_string(), config);

        assert_eq!(sm.state(), InterfaceState::Probing);
    }

    #[test]
    fn test_probing_to_up() {
        let config = StateMachineConfig {
            min_state_duration_secs: 0,
            probing_to_up_holddown_secs: 0,
            ..Default::default()
        };

        let mut sm = InterfaceStateMachine::new("eth0".to_string(), config);

        let changed = sm.process_event(StateEvent::HealthyProbe);
        assert!(changed);
        assert_eq!(sm.state(), InterfaceState::Up);
    }

    #[test]
    fn test_up_to_degraded() {
        let config = StateMachineConfig {
            min_state_duration_secs: 0,
            up_to_degraded_holddown_secs: 0,
            ..Default::default()
        };

        let mut sm = InterfaceStateMachine::new("eth0".to_string(), config);
        sm.force_state(InterfaceState::Up);

        let changed = sm.process_event(StateEvent::DegradedProbe);
        assert!(changed);
        assert_eq!(sm.state(), InterfaceState::Degraded);
    }

    #[test]
    fn test_degraded_to_down() {
        let config = StateMachineConfig {
            min_state_duration_secs: 0,
            degraded_to_down_holddown_secs: 0,
            ..Default::default()
        };

        let mut sm = InterfaceStateMachine::new("eth0".to_string(), config);
        sm.force_state(InterfaceState::Degraded);

        let changed = sm.process_event(StateEvent::FailedProbe);
        assert!(changed);
        assert_eq!(sm.state(), InterfaceState::Down);
    }

    #[test]
    fn test_physical_down_immediate() {
        let config = StateMachineConfig::default();
        let mut sm = InterfaceStateMachine::new("eth0".to_string(), config);
        sm.force_state(InterfaceState::Up);

        let changed = sm.process_event(StateEvent::PhysicalDown);
        assert!(changed);
        assert_eq!(sm.state(), InterfaceState::Down);
    }

    #[test]
    fn test_min_state_duration() {
        let config = StateMachineConfig {
            min_state_duration_secs: 100, // Long duration
            ..Default::default()
        };

        let mut sm = InterfaceStateMachine::new("eth0".to_string(), config);
        sm.force_state(InterfaceState::Up);

        // Try to transition immediately - should be queued
        let changed = sm.process_event(StateEvent::DegradedProbe);
        assert!(!changed);
        assert_eq!(sm.state(), InterfaceState::Up);

        // Event should be queued
        assert!(sm.pending_event.is_some());
    }
}
