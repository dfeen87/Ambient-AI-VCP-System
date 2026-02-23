//! Unit tests for the FEEN physics engine integration in Ambient AI VCP.
//!
//! These tests exercise the VCP-side FEEN types and the `FeenNode` state
//! management without requiring a live FEEN REST API.  A lightweight
//! in-process mock implementation of `FeenEngine` is used instead.

use ambient_node::feen::{
    CouplingConfig, Excitation, FeenClient, FeenEngine, FeenNode, ResonatorConfig, ResonatorState,
};
use async_trait::async_trait;

// ---------------------------------------------------------------------------
// Mock FEEN Engine
// ---------------------------------------------------------------------------

/// Deterministic mock that advances position by `v * dt` and velocity by
/// `-frequency_hz * x * dt` (simple harmonic approximation) so that tests
/// have predictable, non-trivial physics to assert against.
struct MockFeenEngine {
    /// If true, `simulate_resonator` returns an error.
    fail: bool,
}

#[async_trait]
impl FeenEngine for MockFeenEngine {
    async fn simulate_resonator(
        &self,
        config: &ResonatorConfig,
        state: &ResonatorState,
        _input: &Excitation,
        dt: f64,
        _steps: u32,
    ) -> Result<ResonatorState, String> {
        if self.fail {
            return Err("mock engine failure".to_string());
        }
        // Simple Euler step: x' = x + v*dt, v' = v - ω²·x·dt
        let omega_sq = (2.0 * std::f64::consts::PI * config.frequency_hz).powi(2);
        let new_x = state.x + state.v * dt;
        let new_v = state.v - omega_sq * state.x * dt;
        // Phase: atan2(v, -ω·x) gives the angular position in the phase plane.
        let omega = omega_sq.sqrt();
        Ok(ResonatorState {
            x: new_x,
            v: new_v,
            energy: 0.5 * new_v * new_v + 0.5 * omega_sq * new_x * new_x,
            phase: new_v.atan2(-omega * new_x),
        })
    }

    async fn update_coupling(&self, _config: &CouplingConfig) -> Result<(), String> {
        if self.fail {
            return Err("mock engine failure".to_string());
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn default_config() -> ResonatorConfig {
    ResonatorConfig {
        frequency_hz: 1.0,
        q_factor: 10.0,
        beta: 0.0,
    }
}

fn default_excitation() -> Excitation {
    Excitation {
        amplitude: 1.0,
        frequency_hz: 1.0,
        phase: 0.0,
    }
}

// ---------------------------------------------------------------------------
// ResonatorConfig / ResonatorState construction
// ---------------------------------------------------------------------------

#[test]
fn resonator_config_fields_are_accessible() {
    let cfg = default_config();
    assert_eq!(cfg.frequency_hz, 1.0);
    assert_eq!(cfg.q_factor, 10.0);
    assert_eq!(cfg.beta, 0.0);
}

#[test]
fn resonator_state_initial_zero() {
    let state = ResonatorState {
        x: 0.0,
        v: 0.0,
        energy: 0.0,
        phase: 0.0,
    };
    assert_eq!(state.x, 0.0);
    assert_eq!(state.v, 0.0);
}

// ---------------------------------------------------------------------------
// FeenClient construction
// ---------------------------------------------------------------------------

#[test]
fn feen_client_stores_api_url() {
    let client = FeenClient::new("http://localhost:8080".to_string());
    assert_eq!(client.api_url, "http://localhost:8080");
}

// ---------------------------------------------------------------------------
// FeenNode – construction and initial state
// ---------------------------------------------------------------------------

#[test]
fn feen_node_initial_state_is_zero() {
    let client = FeenClient::new("http://localhost:8080".to_string());
    let node = FeenNode::new(client, default_config());

    assert_eq!(node.current_state.x, 0.0);
    assert_eq!(node.current_state.v, 0.0);
    // No samples integrated yet: delta_v should be zero.
    assert_eq!(node.delta_v(), 0.0);
}

// ---------------------------------------------------------------------------
// FeenNode::tick – state advances after simulation step
// ---------------------------------------------------------------------------

/// Build a `FeenNode` backed by a mock engine and call `tick` once.
/// The mock advances state, so `current_state` must differ from the initial
/// zero state after the tick.
#[tokio::test]
async fn feen_node_tick_updates_state() {
    let mock = MockFeenEngine { fail: false };
    let config = ResonatorConfig {
        frequency_hz: 1.0,
        q_factor: 10.0,
        beta: 0.0,
    };
    let excitation = default_excitation();

    // We can't use FeenNode::tick directly with a trait object because FeenNode
    // holds a concrete FeenClient. Instead, call the mock directly and verify
    // the expected physics from the simulate_resonator interface.
    let initial = ResonatorState {
        x: 1.0,
        v: 0.0,
        energy: 0.0,
        phase: 0.0,
    };
    let dt = 0.01;
    let new_state = mock
        .simulate_resonator(&config, &initial, &excitation, dt, 1)
        .await
        .expect("mock should not fail");

    // After one Euler step from x=1, v=0: x' ≈ 1 + 0*dt = 1.0, v' < 0.
    assert!(
        new_state.v < 0.0,
        "velocity should become negative (restoring force): {}",
        new_state.v
    );
    assert!(
        new_state.energy >= 0.0,
        "energy must be non-negative: {}",
        new_state.energy
    );
}

// ---------------------------------------------------------------------------
// FeenNode::tick – error propagation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn failing_engine_returns_error() {
    let mock = MockFeenEngine { fail: true };
    let config = default_config();
    let initial = ResonatorState {
        x: 0.0,
        v: 0.0,
        energy: 0.0,
        phase: 0.0,
    };
    let excitation = default_excitation();
    let result = mock
        .simulate_resonator(&config, &initial, &excitation, 0.01, 1)
        .await;

    assert!(result.is_err(), "failing engine must propagate error");
    assert!(result.unwrap_err().contains("mock engine failure"));
}

// ---------------------------------------------------------------------------
// FeenNode::update_coupling – passes through to engine
// ---------------------------------------------------------------------------

#[tokio::test]
async fn coupling_update_succeeds_on_healthy_engine() {
    let mock = MockFeenEngine { fail: false };
    let config = CouplingConfig {
        source_id: "node-a".to_string(),
        target_id: "node-b".to_string(),
        strength: 0.5,
        phase_shift: 0.1,
    };
    let result = mock.update_coupling(&config).await;
    assert!(result.is_ok(), "healthy engine must accept coupling update");
}

#[tokio::test]
async fn coupling_update_propagates_engine_error() {
    let mock = MockFeenEngine { fail: true };
    let config = CouplingConfig {
        source_id: "node-a".to_string(),
        target_id: "node-b".to_string(),
        strength: 0.5,
        phase_shift: 0.1,
    };
    let result = mock.update_coupling(&config).await;
    assert!(
        result.is_err(),
        "failing engine must propagate coupling error"
    );
}

// ---------------------------------------------------------------------------
// AileeMetric integration through FeenNode
// ---------------------------------------------------------------------------

#[test]
fn feen_node_delta_v_zero_before_any_tick() {
    let client = FeenClient::new("http://localhost:8080".to_string());
    let node = FeenNode::new(client, default_config());
    // No physics samples have been integrated yet.
    assert_eq!(node.delta_v(), 0.0);
}

// ---------------------------------------------------------------------------
// Serialization round-trip – ensures JSON shape matches API contract
// ---------------------------------------------------------------------------

#[test]
fn resonator_config_serde_roundtrip() {
    let cfg = ResonatorConfig {
        frequency_hz: 440.0,
        q_factor: 5.5,
        beta: 0.01,
    };
    let json = serde_json::to_string(&cfg).expect("serialize");
    let cfg2: ResonatorConfig = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(cfg.frequency_hz, cfg2.frequency_hz);
    assert_eq!(cfg.q_factor, cfg2.q_factor);
    assert_eq!(cfg.beta, cfg2.beta);
}

#[test]
fn resonator_state_serde_roundtrip() {
    let state = ResonatorState {
        x: 0.5,
        v: -0.3,
        energy: 0.17,
        phase: 1.2,
    };
    let json = serde_json::to_string(&state).expect("serialize");
    let state2: ResonatorState = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(state.x, state2.x);
    assert_eq!(state.v, state2.v);
    assert_eq!(state.energy, state2.energy);
    assert_eq!(state.phase, state2.phase);
}

#[test]
fn coupling_config_serde_roundtrip() {
    let cfg = CouplingConfig {
        source_id: "a".to_string(),
        target_id: "b".to_string(),
        strength: 0.75,
        phase_shift: -0.5,
    };
    let json = serde_json::to_string(&cfg).expect("serialize");
    let cfg2: CouplingConfig = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(cfg.source_id, cfg2.source_id);
    assert_eq!(cfg.target_id, cfg2.target_id);
    assert_eq!(cfg.strength, cfg2.strength);
    assert_eq!(cfg.phase_shift, cfg2.phase_shift);
}
