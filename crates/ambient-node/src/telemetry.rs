use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Telemetry data sample from a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySample {
    pub bandwidth_mbps: f64,
    pub avg_latency_ms: f64,
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub temperature_c: f64,
    pub power_watts: f64,
    pub timestamp: u64,
}

impl Default for TelemetrySample {
    fn default() -> Self {
        Self {
            bandwidth_mbps: 0.0,
            avg_latency_ms: 0.0,
            cpu_usage_percent: 0.0,
            memory_usage_percent: 0.0,
            temperature_c: 0.0,
            power_watts: 0.0,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

impl TelemetrySample {
    /// Calculate bandwidth score (0.0 - 1.0)
    /// Assumes max bandwidth of 1000 Mbps
    pub fn bandwidth_score(&self) -> f64 {
        (self.bandwidth_mbps / 1000.0).min(1.0).max(0.0)
    }

    /// Calculate latency score (0.0 - 1.0)
    /// Lower latency = higher score
    /// Assumes max acceptable latency of 100ms
    pub fn latency_score(&self) -> f64 {
        if self.avg_latency_ms <= 0.0 {
            return 1.0;
        }
        (1.0 - (self.avg_latency_ms / 100.0)).min(1.0).max(0.0)
    }

    /// Calculate compute efficiency score (0.0 - 1.0)
    /// Based on CPU and memory usage
    pub fn compute_score(&self) -> f64 {
        let cpu_available = 100.0 - self.cpu_usage_percent;
        let memory_available = 100.0 - self.memory_usage_percent;
        
        ((cpu_available + memory_available) / 200.0).min(1.0).max(0.0)
    }

    /// Check if telemetry indicates healthy state
    pub fn is_healthy(&self) -> bool {
        self.temperature_c < 85.0
            && self.cpu_usage_percent < 95.0
            && self.memory_usage_percent < 95.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bandwidth_score() {
        let mut sample = TelemetrySample::default();
        sample.bandwidth_mbps = 500.0;
        assert_eq!(sample.bandwidth_score(), 0.5);

        sample.bandwidth_mbps = 1000.0;
        assert_eq!(sample.bandwidth_score(), 1.0);

        sample.bandwidth_mbps = 2000.0;
        assert_eq!(sample.bandwidth_score(), 1.0); // Capped at 1.0
    }

    #[test]
    fn test_latency_score() {
        let mut sample = TelemetrySample::default();
        sample.avg_latency_ms = 50.0;
        assert_eq!(sample.latency_score(), 0.5);

        sample.avg_latency_ms = 10.0;
        assert_eq!(sample.latency_score(), 0.9);

        sample.avg_latency_ms = 0.0;
        assert_eq!(sample.latency_score(), 1.0);
    }

    #[test]
    fn test_compute_score() {
        let mut sample = TelemetrySample::default();
        sample.cpu_usage_percent = 50.0;
        sample.memory_usage_percent = 50.0;
        assert_eq!(sample.compute_score(), 0.5);

        sample.cpu_usage_percent = 0.0;
        sample.memory_usage_percent = 0.0;
        assert_eq!(sample.compute_score(), 1.0);
    }

    #[test]
    fn test_is_healthy() {
        let mut sample = TelemetrySample::default();
        sample.temperature_c = 70.0;
        sample.cpu_usage_percent = 60.0;
        sample.memory_usage_percent = 70.0;
        assert!(sample.is_healthy());

        sample.temperature_c = 90.0;
        assert!(!sample.is_healthy());
    }
}
