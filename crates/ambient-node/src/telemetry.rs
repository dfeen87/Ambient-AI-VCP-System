use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Telemetry data sample from a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySample {
    /// Aggregate / legacy bandwidth field (Mbps).  Used as a fallback when
    /// directional measurements are not yet available.
    pub bandwidth_mbps: f64,
    /// Measured WAN upload speed (Mbps).  When non-zero this is used by
    /// [`bandwidth_score`](TelemetrySample::bandwidth_score) instead of
    /// `bandwidth_mbps` so AILEE can see the true upload capability.
    pub upload_bandwidth_mbps: f64,
    /// Measured WAN download speed (Mbps).  When non-zero this is used by
    /// [`bandwidth_score`](TelemetrySample::bandwidth_score) instead of
    /// `bandwidth_mbps` so AILEE can see the true download capability.
    pub download_bandwidth_mbps: f64,
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
            upload_bandwidth_mbps: 0.0,
            download_bandwidth_mbps: 0.0,
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
    /// Calculate bandwidth score (0.0 - 1.0) for AILEE health scoring.
    ///
    /// When directional measurements (`upload_bandwidth_mbps` /
    /// `download_bandwidth_mbps`) are available the score reflects the
    /// **bottleneck direction** - i.e. the minimum of the two - so AILEE
    /// accurately ranks nodes by their true duplex capability rather than
    /// only one direction.  This prevents a node with fast downloads but
    /// throttled uploads (or vice-versa) from appearing healthier than it
    /// really is.
    ///
    /// Falls back to the aggregate `bandwidth_mbps` field when directional
    /// measurements have not yet been populated (both are zero).
    ///
    /// Assumes a reference maximum of 1,000 Mbps (1 Gbps).
    pub fn bandwidth_score(&self) -> f64 {
        let effective = match (
            self.upload_bandwidth_mbps > 0.0,
            self.download_bandwidth_mbps > 0.0,
        ) {
            // Both directions measured: use the bottleneck (minimum).
            (true, true) => self.upload_bandwidth_mbps.min(self.download_bandwidth_mbps),
            // Only upload measured.
            (true, false) => self.upload_bandwidth_mbps,
            // Only download measured.
            (false, true) => self.download_bandwidth_mbps,
            // Neither measured yet: fall back to legacy aggregate field.
            (false, false) => self.bandwidth_mbps,
        };
        (effective / 1000.0).clamp(0.0, 1.0)
    }

    /// Calculate latency score (0.0 - 1.0)
    /// Lower latency = higher score
    /// Assumes max acceptable latency of 100ms
    pub fn latency_score(&self) -> f64 {
        if self.avg_latency_ms <= 0.0 {
            return 1.0;
        }
        (1.0 - (self.avg_latency_ms / 100.0)).clamp(0.0, 1.0)
    }

    /// Calculate compute efficiency score (0.0 - 1.0)
    /// Based on CPU and memory usage
    pub fn compute_score(&self) -> f64 {
        let cpu_available = 100.0 - self.cpu_usage_percent;
        let memory_available = 100.0 - self.memory_usage_percent;

        ((cpu_available + memory_available) / 200.0).clamp(0.0, 1.0)
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
        let sample = TelemetrySample {
            bandwidth_mbps: 500.0,
            ..Default::default()
        };
        assert_eq!(sample.bandwidth_score(), 0.5);

        let sample = TelemetrySample {
            bandwidth_mbps: 1000.0,
            ..Default::default()
        };
        assert_eq!(sample.bandwidth_score(), 1.0);

        let sample = TelemetrySample {
            bandwidth_mbps: 2000.0,
            ..Default::default()
        };
        assert_eq!(sample.bandwidth_score(), 1.0); // Capped at 1.0
    }

    #[test]
    fn test_latency_score() {
        let sample = TelemetrySample {
            avg_latency_ms: 50.0,
            ..Default::default()
        };
        assert_eq!(sample.latency_score(), 0.5);

        let sample = TelemetrySample {
            avg_latency_ms: 10.0,
            ..Default::default()
        };
        assert_eq!(sample.latency_score(), 0.9);

        let sample = TelemetrySample {
            avg_latency_ms: 0.0,
            ..Default::default()
        };
        assert_eq!(sample.latency_score(), 1.0);
    }

    #[test]
    fn test_compute_score() {
        let sample = TelemetrySample {
            cpu_usage_percent: 50.0,
            memory_usage_percent: 50.0,
            ..Default::default()
        };
        assert_eq!(sample.compute_score(), 0.5);

        let sample = TelemetrySample {
            cpu_usage_percent: 0.0,
            memory_usage_percent: 0.0,
            ..Default::default()
        };
        assert_eq!(sample.compute_score(), 1.0);
    }

    #[test]
    fn test_is_healthy() {
        let sample = TelemetrySample {
            temperature_c: 70.0,
            cpu_usage_percent: 60.0,
            memory_usage_percent: 70.0,
            ..Default::default()
        };
        assert!(sample.is_healthy());

        let sample = TelemetrySample {
            temperature_c: 90.0,
            ..Default::default()
        };
        assert!(!sample.is_healthy());
    }

    /// bandwidth_score falls back to aggregate bandwidth_mbps when no
    /// directional measurements have been taken yet.
    #[test]
    fn test_bandwidth_score_legacy_fallback() {
        let sample = TelemetrySample {
            bandwidth_mbps: 400.0,
            upload_bandwidth_mbps: 0.0,
            download_bandwidth_mbps: 0.0,
            ..Default::default()
        };
        assert_eq!(sample.bandwidth_score(), 0.4);
    }

    /// When only upload is available AILEE scores on upload alone.
    #[test]
    fn test_bandwidth_score_upload_only() {
        let sample = TelemetrySample {
            upload_bandwidth_mbps: 200.0,
            ..Default::default()
        };
        assert_eq!(sample.bandwidth_score(), 0.2);
    }

    /// When only download is available AILEE scores on download alone.
    #[test]
    fn test_bandwidth_score_download_only() {
        let sample = TelemetrySample {
            download_bandwidth_mbps: 800.0,
            ..Default::default()
        };
        assert_eq!(sample.bandwidth_score(), 0.8);
    }

    /// When both directions are measured AILEE uses the bottleneck (minimum)
    /// so a fast download does not mask a slow upload.
    #[test]
    fn test_bandwidth_score_uses_bottleneck_direction() {
        let sample = TelemetrySample {
            upload_bandwidth_mbps: 20.0,   // slow upload
            download_bandwidth_mbps: 500.0, // fast download
            ..Default::default()
        };
        // Score must reflect the upload bottleneck, not the fast download.
        assert_eq!(
            sample.bandwidth_score(),
            0.02,
            "bandwidth_score should use the bottleneck (upload) direction"
        );
    }

    /// Symmetric case: equal upload and download should score the shared value.
    #[test]
    fn test_bandwidth_score_symmetric() {
        let sample = TelemetrySample {
            upload_bandwidth_mbps: 300.0,
            download_bandwidth_mbps: 300.0,
            ..Default::default()
        };
        assert_eq!(sample.bandwidth_score(), 0.3);
    }
}
