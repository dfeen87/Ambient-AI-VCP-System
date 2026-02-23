use serde::{Deserialize, Serialize};
use crate::{AileeMetric, AileeSample};
use async_trait::async_trait;

/// Configuration for a FEEN Resonator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResonatorConfig {
    pub frequency_hz: f64,
    pub q_factor: f64,
    pub beta: f64, // Nonlinearity
}

/// State of a FEEN Resonator (x, v)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResonatorState {
    pub x: f64, // Displacement
    pub v: f64, // Velocity
    pub energy: f64,
    pub phase: f64,
}

/// Coupling configuration between resonators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouplingConfig {
    pub source_id: String,
    pub target_id: String,
    pub strength: f64,
    pub phase_shift: f64,
}

/// Excitation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Excitation {
    pub amplitude: f64,
    pub frequency_hz: f64,
    pub phase: f64,
}

/// Interface to the FEEN Physics Engine
///
/// In a real deployment, this would make HTTP calls to the FEEN REST API
/// or use FFI to call the C++ library.
#[async_trait]
pub trait FeenEngine: Send + Sync {
    async fn simulate_resonator(&self, config: &ResonatorConfig, state: &ResonatorState, input: &Excitation, dt: f64, steps: u32) -> Result<ResonatorState, String>;
    async fn update_coupling(&self, config: &CouplingConfig) -> Result<(), String>;
}

/// Implementation of FEEN Engine Client
#[derive(Debug, Clone)]
pub struct FeenClient {
    pub api_url: String,
    client: reqwest::Client,
}

impl FeenClient {
    pub fn new(api_url: String) -> Self {
        Self {
            api_url,
            client: reqwest::Client::new(),
        }
    }
}

#[derive(Serialize)]
struct SimulationRequest<'a> {
    config: &'a ResonatorConfig,
    state: &'a ResonatorState,
    input: &'a Excitation,
    dt: f64,
    steps: u32,
}

#[derive(Deserialize)]
struct SimulationResponse {
    state: ResonatorState,
    #[allow(dead_code)]
    delta_v: Option<f64>,
}

#[async_trait]
impl FeenEngine for FeenClient {
    async fn simulate_resonator(&self, config: &ResonatorConfig, state: &ResonatorState, input: &Excitation, dt: f64, steps: u32) -> Result<ResonatorState, String> {
        let request = SimulationRequest {
            config,
            state,
            input,
            dt,
            steps,
        };

        let url = format!("{}/api/v1/simulate", self.api_url);
        let response = self.client.post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Failed to send request: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("FEEN API error: {}", response.status()));
        }

        let sim_res: SimulationResponse = response.json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(sim_res.state)
    }

    async fn update_coupling(&self, config: &CouplingConfig) -> Result<(), String> {
        let url = format!("{}/api/v1/coupling", self.api_url);
        let response = self.client.post(&url)
            .json(config)
            .send()
            .await
            .map_err(|e| format!("Failed to send request: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("FEEN API error: {}", response.status()));
        }

        Ok(())
    }
}

/// FEEN Node representation
#[derive(Debug, Clone)]
pub struct FeenNode {
    // using the concrete client for now
    pub client: FeenClient,
    pub resonator_config: ResonatorConfig,
    pub current_state: ResonatorState,
    pub metric: AileeMetric,
}

impl FeenNode {
    pub fn new(client: FeenClient, config: ResonatorConfig) -> Self {
        Self {
            client,
            resonator_config: config,
            current_state: ResonatorState { x: 0.0, v: 0.0, energy: 0.0, phase: 0.0 },
            metric: AileeMetric::default(),
        }
    }

    pub async fn tick(&mut self, input: &Excitation, dt: f64) -> Result<(), String> {
        let new_state = self.client.simulate_resonator(&self.resonator_config, &self.current_state, input, dt, 1).await?;

        // Calculate physics values for AileeSample
        // P_input ~ Energy input (simplified as excitation amplitude squared)
        let p_input = input.amplitude.powi(2);

        // Workload w(t) ~ Deviation from natural frequency (simplified)
        let workload = (input.frequency_hz - self.resonator_config.frequency_hz).abs();

        // Velocity v(t) ~ Phase velocity or just velocity
        let velocity = new_state.v.abs();

        // Inertia M(t) ~ mass, assume 1.0 or related to Q factor
        let inertia = 1.0;

        let sample = AileeSample::new(p_input, workload, velocity, inertia, dt);
        self.metric.integrate(&sample);

        self.current_state = new_state;

        Ok(())
    }

    pub fn delta_v(&self) -> f64 {
        self.metric.delta_v()
    }

    pub async fn update_coupling(&self, config: &CouplingConfig) -> Result<(), String> {
        self.client.update_coupling(config).await
    }
}
