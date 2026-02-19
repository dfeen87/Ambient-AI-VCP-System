//! AILEE optimization metric (∆v)
//!
//! Implements the energy-weighted gain functional from the AILEE paper:
//!
//! ```text
//! ∆v = Isp · η · e^(−α·v₀²) · ∫₀^tf  P_input(t) · e^(−α·w(t)²) · e^(2α·v₀·v(t)) / M(t)  dt
//! ```
//!
//! This is an **operational performance functional**, not a physical law. It
//! aggregates measurable system signals into a time-integrated efficiency
//! metric that can be used for comparative diagnostics across schedulers or
//! hardware configurations.
//!
//! ## Variables
//!
//! | Symbol       | Description                                               |
//! |--------------|-----------------------------------------------------------|
//! | ∆v           | Dimensionless effective optimization gain                 |
//! | `Isp`        | Specific efficiency factor (gain per unit resource)       |
//! | `η` (eta)    | System efficiency coefficient                             |
//! | `α` (alpha)  | Resonance sensitivity / damping coefficient               |
//! | `v₀`         | Reference learning / velocity state                       |
//! | `P_input(t)` | Compute or power input at time *t*                        |
//! | `w(t)`       | Workload intensity at time *t*                            |
//! | `v(t)`       | Instantaneous learning / adaptation state at time *t*     |
//! | `M(t)`       | Model inertia (effective system mass) at time *t*         |

use serde::{Deserialize, Serialize};

/// Parameters for the AILEE ∆v metric.
///
/// # Calibration guidance
///
/// | Parameter | Typical range | Effect                                         |
/// |-----------|---------------|------------------------------------------------|
/// | `isp`     | 0.1 – 10.0    | Scales the overall magnitude of ∆v.            |
/// | `eta`     | 0.1 – 1.0     | Architectural efficiency factor (≤ 1 for real systems). |
/// | `alpha`   | 0.001 – 1.0   | Resonance sensitivity. Large values narrow the resonance band; keep it small enough that `alpha * velocity²` and `2 * alpha * v0 * velocity` stay well below ~700 to avoid `f64` overflow in the exponential gates. |
/// | `v0`      | Any real      | Reference operating point. Set to the expected nominal `velocity` value. |
///
/// `P_input`, `workload`, `velocity`, and `inertia` in [`AileeSample`] are
/// dimensionless or in user-defined units — the metric is a *relative*
/// diagnostic functional, not an absolute physical quantity.  Consistent units
/// within a single deployment are sufficient for comparative analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AileeParams {
    /// Specific efficiency factor (gain per unit resource).
    pub isp: f64,
    /// System efficiency coefficient (algorithmic / architectural).
    pub eta: f64,
    /// Resonance sensitivity coefficient (damping / amplification).
    pub alpha: f64,
    /// Reference learning / velocity state `v₀`.
    pub v0: f64,
}

impl Default for AileeParams {
    fn default() -> Self {
        Self {
            isp: 1.0,
            eta: 1.0,
            alpha: 0.1,
            v0: 1.0,
        }
    }
}

/// A single telemetry observation used to drive the AILEE ∆v integral.
///
/// Collect one `AileeSample` per measurement interval and pass it to
/// [`AileeMetric::integrate`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AileeSample {
    /// Compute or power input `P_input(t)` (watts or normalised units).
    pub p_input: f64,
    /// Workload intensity `w(t)`.
    pub workload: f64,
    /// Instantaneous learning / adaptation state `v(t)` (e.g. loss-gradient norm).
    pub velocity: f64,
    /// Model inertia / effective system mass `M(t)`.
    /// Must be strictly positive; values ≤ 0 are clamped to a small epsilon.
    pub inertia: f64,
    /// Duration of this measurement interval in seconds (`dt`).
    pub dt: f64,
}

impl AileeSample {
    /// Create a new telemetry sample, clamping `inertia` and `dt` to valid ranges.
    pub fn new(p_input: f64, workload: f64, velocity: f64, inertia: f64, dt: f64) -> Self {
        Self {
            p_input: p_input.max(0.0),
            workload,
            velocity,
            inertia: inertia.max(f64::EPSILON),
            dt: dt.max(0.0),
        }
    }
}

/// AILEE ∆v metric accumulator.
///
/// Computes the time-integrated efficiency gain by accumulating successive
/// [`AileeSample`]s and applying the outer scaling factors on demand.
///
/// ```text
/// ∆v = Isp · η · e^(−α·v₀²) · Σ  P_input · e^(−α·w²) · e^(2α·v₀·v) / M  · dt
/// ```
///
/// # Example
///
/// ```rust
/// use ailee_trust_layer::metric::{AileeMetric, AileeParams, AileeSample};
///
/// let mut metric = AileeMetric::default();
///
/// // Feed a sequence of telemetry observations.
/// metric.integrate(&AileeSample::new(100.0, 0.5, 1.2, 10.0, 1.0));
/// metric.integrate(&AileeSample::new( 90.0, 0.6, 1.1,  9.0, 1.0));
///
/// let gain = metric.delta_v();
/// assert!(gain >= 0.0, "gain should be non-negative");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AileeMetric {
    params: AileeParams,
    /// Running sum of the weighted integrand (before outer scaling).
    accumulated: f64,
    /// Number of samples that have been integrated.
    sample_count: u64,
}

impl AileeMetric {
    /// Create a new accumulator with the given parameters.
    pub fn new(params: AileeParams) -> Self {
        Self {
            params,
            accumulated: 0.0,
            sample_count: 0,
        }
    }

    /// Integrate a single telemetry sample.
    ///
    /// The per-step contribution is:
    ///
    /// ```text
    /// P_input(t) · e^(−α·w(t)²) · e^(2α·v₀·v(t)) / M(t) · dt
    /// ```
    ///
    /// The two exponential terms act as nonlinear resonance gates: deviations
    /// from the optimal operating point suppress the contribution, while
    /// coherent operation enhances it. Division by `M(t)` penalises brute-force
    /// scaling and rewards efficient adaptation.
    ///
    /// Both exponents are clamped to `[-700, 700]` before evaluation to
    /// prevent `f64` overflow / underflow when parameters or telemetry values
    /// are very large.
    pub fn integrate(&mut self, sample: &AileeSample) {
        let alpha = self.params.alpha;
        let v0 = self.params.v0;

        // Workload resonance gate: e^(-α·w²) — suppresses off-workload operation.
        let workload_exp = (-alpha * sample.workload.powi(2)).clamp(-700.0, 700.0);
        let workload_gate = workload_exp.exp();

        // Velocity resonance gate: e^(2α·v₀·v) — rewards coherent adaptation.
        // Clamped to avoid overflow when velocity is large.
        let velocity_exp = (2.0 * alpha * v0 * sample.velocity).clamp(-700.0, 700.0);
        let velocity_gate = velocity_exp.exp();

        // Integrand contribution for this time step.
        let integrand = sample.p_input * workload_gate * velocity_gate / sample.inertia;
        self.accumulated += integrand * sample.dt;
        self.sample_count += 1;
    }

    /// Compute the current ∆v value.
    ///
    /// Applies the outer scaling `Isp · η · e^(-α·v₀²)` to the accumulated integral.
    pub fn delta_v(&self) -> f64 {
        let alpha = self.params.alpha;
        let v0 = self.params.v0;
        let outer = self.params.isp * self.params.eta * (-alpha * v0.powi(2)).exp();
        outer * self.accumulated
    }

    /// Reset the accumulator to zero while keeping the parameters.
    pub fn reset(&mut self) {
        self.accumulated = 0.0;
        self.sample_count = 0;
    }

    /// Number of samples integrated since the last [`reset`](AileeMetric::reset).
    pub fn sample_count(&self) -> u64 {
        self.sample_count
    }

    /// Access the current parameters.
    pub fn params(&self) -> &AileeParams {
        &self.params
    }
}

impl Default for AileeMetric {
    fn default() -> Self {
        Self::new(AileeParams::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_sample(p: f64, w: f64, v: f64) -> AileeSample {
        AileeSample::new(p, w, v, 10.0, 1.0)
    }

    #[test]
    fn delta_v_zero_with_no_samples() {
        let m = AileeMetric::default();
        assert_eq!(m.delta_v(), 0.0);
        assert_eq!(m.sample_count(), 0);
    }

    #[test]
    fn delta_v_positive_after_integration() {
        let mut m = AileeMetric::default();
        m.integrate(&default_sample(100.0, 0.5, 1.0));
        assert!(m.delta_v() > 0.0);
        assert_eq!(m.sample_count(), 1);
    }

    #[test]
    fn higher_power_input_yields_higher_gain() {
        let params = AileeParams::default();
        let mut low = AileeMetric::new(params.clone());
        let mut high = AileeMetric::new(params);

        low.integrate(&default_sample(50.0, 0.0, 1.0));
        high.integrate(&default_sample(100.0, 0.0, 1.0));

        assert!(high.delta_v() > low.delta_v());
    }

    #[test]
    fn larger_workload_deviation_suppresses_gain() {
        let params = AileeParams::default();
        let mut optimal = AileeMetric::new(params.clone());
        let mut off_resonant = AileeMetric::new(params);

        // w=0 is optimal (no suppression); w=5 is far off-resonant.
        optimal.integrate(&default_sample(100.0, 0.0, 1.0));
        off_resonant.integrate(&default_sample(100.0, 5.0, 1.0));

        assert!(optimal.delta_v() > off_resonant.delta_v());
    }

    #[test]
    fn higher_inertia_reduces_gain() {
        let params = AileeParams::default();
        let mut low_inertia = AileeMetric::new(params.clone());
        let mut high_inertia = AileeMetric::new(params);

        low_inertia.integrate(&AileeSample::new(100.0, 0.0, 1.0, 1.0, 1.0));
        high_inertia.integrate(&AileeSample::new(100.0, 0.0, 1.0, 100.0, 1.0));

        assert!(low_inertia.delta_v() > high_inertia.delta_v());
    }

    #[test]
    fn reset_clears_accumulator() {
        let mut m = AileeMetric::default();
        m.integrate(&default_sample(100.0, 0.5, 1.0));
        assert!(m.delta_v() > 0.0);

        m.reset();
        assert_eq!(m.delta_v(), 0.0);
        assert_eq!(m.sample_count(), 0);
    }

    #[test]
    fn samples_accumulate_additively() {
        let params = AileeParams::default();
        let mut two_steps = AileeMetric::new(params.clone());

        // Integrate same sample twice.
        two_steps.integrate(&default_sample(100.0, 0.0, 1.0));
        two_steps.integrate(&default_sample(100.0, 0.0, 1.0));

        // Integrate same sample once with dt=2 (should give same result).
        let mut one_step = AileeMetric::new(params);
        one_step.integrate(&AileeSample::new(100.0, 0.0, 1.0, 10.0, 2.0));

        let diff = (two_steps.delta_v() - one_step.delta_v()).abs();
        assert!(diff < 1e-10, "accumulation mismatch: {diff}");
    }

    #[test]
    fn large_velocity_does_not_overflow() {
        // Without the clamp, 2·α·v₀·v = 2·0.1·1.0·1e6 = 2e5, which overflows f64::exp.
        let mut m = AileeMetric::default();
        m.integrate(&AileeSample::new(100.0, 0.0, 1e6, 1.0, 1.0));
        assert!(m.delta_v().is_finite(), "delta_v should be finite even for very large velocity");
    }

    #[test]
    fn ailee_sample_clamps_invalid_inertia() {
        let s = AileeSample::new(1.0, 0.0, 0.0, -5.0, 1.0);
        assert!(s.inertia > 0.0);
    }

    #[test]
    fn ailee_sample_clamps_negative_dt() {
        let s = AileeSample::new(1.0, 0.0, 0.0, 1.0, -3.0);
        assert_eq!(s.dt, 0.0);
    }
}
