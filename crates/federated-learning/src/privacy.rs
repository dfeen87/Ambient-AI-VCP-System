use rand::Rng;
use serde::{Deserialize, Serialize};

/// Differential Privacy parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyBudget {
    pub epsilon: f64,
    pub delta: f64,
}

impl PrivacyBudget {
    pub fn new(epsilon: f64, delta: f64) -> Self {
        Self { epsilon, delta }
    }

    /// Conservative privacy budget
    pub fn conservative() -> Self {
        Self {
            epsilon: 0.1,
            delta: 1e-5,
        }
    }

    /// Standard privacy budget
    pub fn standard() -> Self {
        Self {
            epsilon: 1.0,
            delta: 1e-5,
        }
    }

    /// Relaxed privacy budget
    pub fn relaxed() -> Self {
        Self {
            epsilon: 10.0,
            delta: 1e-4,
        }
    }
}

/// Privacy-preserving mechanisms
pub struct PrivacyMechanism {
    budget: PrivacyBudget,
}

impl PrivacyMechanism {
    pub fn new(budget: PrivacyBudget) -> Self {
        Self { budget }
    }

    /// Add Gaussian noise to a value for differential privacy
    pub fn add_gaussian_noise(&self, value: f64, sensitivity: f64) -> f64 {
        let mut rng = rand::thread_rng();

        // Calculate noise scale using Gaussian mechanism
        // sigma = sqrt(2 * ln(1.25/delta)) * sensitivity / epsilon
        let sigma =
            (2.0 * (1.25 / self.budget.delta).ln()).sqrt() * sensitivity / self.budget.epsilon;

        // Guard against degenerate budgets (epsilon=0 → sigma=∞, delta=0 → NaN, etc.).
        // sigma must be strictly positive and finite for rand_distr::Normal to accept it;
        // sigma=0 means "no noise at all", which the caller can achieve by not calling this
        // function, and is not a meaningful DP noise configuration.
        if !sigma.is_finite() || sigma <= 0.0 {
            return value;
        }

        if let Ok(dist) = rand_distr::Normal::new(0.0, sigma) {
            value + rng.sample::<f64, _>(dist)
        } else {
            value
        }
    }

    /// Add Laplacian noise to a value for differential privacy
    pub fn add_laplacian_noise(&self, value: f64, sensitivity: f64) -> f64 {
        let mut rng = rand::thread_rng();

        // Scale for Laplace distribution
        let scale = sensitivity / self.budget.epsilon;

        // Guard against degenerate budgets (epsilon=0 → scale=∞, etc.).
        // scale must be strictly positive and finite for rand_distr::Exp to accept it;
        // scale=0 is not a valid Laplace distribution parameter.
        if !scale.is_finite() || scale <= 0.0 {
            return value;
        }

        // Generate Laplace noise manually using exponential distribution
        // Laplace(0, b) = sign * Exponential(1/b) where sign is ±1 with equal probability
        let sign = if rng.gen::<bool>() { 1.0 } else { -1.0 };
        if let Ok(dist) = rand_distr::Exp::new(1.0 / scale) {
            let exp_sample: f64 = rng.sample(dist);
            value + sign * exp_sample
        } else {
            value
        }
    }

    /// Apply gradient clipping to bound sensitivity
    pub fn clip_gradient(&self, gradient: &mut [f64], clip_norm: f64) {
        let norm: f64 = gradient.iter().map(|x| x * x).sum::<f64>().sqrt();

        if norm > clip_norm {
            let scale = clip_norm / norm;
            for g in gradient.iter_mut() {
                *g *= scale;
            }
        }
    }

    /// Add noise to gradients for DP-SGD
    pub fn add_dp_noise_to_gradients(&self, gradients: &mut [f64], clip_norm: f64) {
        // First clip gradients
        self.clip_gradient(gradients, clip_norm);

        // Then add noise
        for g in gradients.iter_mut() {
            *g = self.add_gaussian_noise(*g, clip_norm);
        }
    }
}

impl Default for PrivacyMechanism {
    fn default() -> Self {
        Self::new(PrivacyBudget::standard())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privacy_budget() {
        let budget = PrivacyBudget::conservative();
        assert_eq!(budget.epsilon, 0.1);
        assert_eq!(budget.delta, 1e-5);
    }

    #[test]
    fn test_gradient_clipping() {
        let mechanism = PrivacyMechanism::default();
        let mut gradients = vec![3.0, 4.0]; // Norm = 5.0

        mechanism.clip_gradient(&mut gradients, 1.0);

        let norm: f64 = gradients.iter().map(|x| x * x).sum::<f64>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_noise_addition() {
        let mechanism = PrivacyMechanism::new(PrivacyBudget::standard());
        let value = 100.0;
        let sensitivity = 1.0;

        // Test multiple samples to check statistical properties
        let mut differences = Vec::new();
        for _ in 0..100 {
            let noisy_value = mechanism.add_gaussian_noise(value, sensitivity);
            differences.push((noisy_value - value).abs());
        }

        // At least some values should have noise
        assert!(differences.iter().any(|&d| d > 0.1));

        // Most values should be within reasonable range (99.7% within 3 sigma)
        // sigma ≈ 4.85, so 3*sigma ≈ 14.5
        let within_range = differences.iter().filter(|&&d| d < 15.0).count();
        assert!(
            within_range >= 95,
            "Expected at least 95/100 samples within 3 sigma, got {}",
            within_range
        );
    }

    #[test]
    fn test_noise_does_not_panic_with_degenerate_budget() {
        // epsilon=0 would cause division by zero or NaN in the noise formulas;
        // the implementation must return the original value unperturbed rather
        // than panicking.
        let mechanism = PrivacyMechanism::new(PrivacyBudget::new(0.0, 1e-5));
        let value = 42.0;
        let result = mechanism.add_gaussian_noise(value, 1.0);
        // With epsilon=0 the sigma is infinite; distribution creation fails gracefully
        assert!(result.is_finite(), "result should be finite: {}", result);

        let result2 = mechanism.add_laplacian_noise(value, 1.0);
        assert!(result2.is_finite(), "result should be finite: {}", result2);
    }
}
