//! Interface scoring system
//!
//! Implements a weight-based scoring function for backhaul interfaces:
//! score = weight_latency + weight_loss + weight_success + policy_bias
//!
//! Supports configurable weights and policy bias (e.g., prefer Ethernet > Wi-Fi > LTE > tether)

use crate::connectivity::backhaul::discovery::InterfaceInfo;
use crate::connectivity::backhaul::health::HealthStats;
use serde::{Deserialize, Serialize};

/// Scoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringConfig {
    /// Weight for latency component (lower is better)
    pub weight_latency: f64,
    
    /// Weight for packet loss component (lower is better)
    pub weight_loss: f64,
    
    /// Weight for success rate component (higher is better)
    pub weight_success: f64,
    
    /// Enable policy bias based on interface type
    pub enable_policy_bias: bool,
    
    /// Multiplier for policy bias
    pub policy_bias_multiplier: f64,
    
    /// Maximum acceptable RTT in milliseconds
    pub max_rtt_ms: f64,
    
    /// Maximum acceptable packet loss percentage
    pub max_loss_percent: f64,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        Self {
            weight_latency: 300.0,
            weight_loss: 200.0,
            weight_success: 500.0,
            enable_policy_bias: true,
            policy_bias_multiplier: 1.0,
            max_rtt_ms: 200.0,
            max_loss_percent: 10.0,
        }
    }
}

/// Scoring engine for interfaces
pub struct InterfaceScorer {
    config: ScoringConfig,
}

impl InterfaceScorer {
    pub fn new(config: ScoringConfig) -> Self {
        Self { config }
    }

    /// Calculate score for an interface
    ///
    /// Returns a score where higher is better.
    /// Score is composed of:
    /// - Latency component: lower RTT gives higher score
    /// - Loss component: lower packet loss gives higher score
    /// - Success component: higher success rate gives higher score
    /// - Policy bias: interface type preference
    pub fn score(
        &self,
        interface: &InterfaceInfo,
        health_stats: &HealthStats,
    ) -> InterfaceScore {
        let latency_score = self.score_latency(health_stats);
        let loss_score = self.score_loss(health_stats);
        let success_score = self.score_success(health_stats);
        let policy_bias = self.score_policy_bias(interface);
        
        let total_score = latency_score + loss_score + success_score + policy_bias;
        
        InterfaceScore {
            interface: interface.name.clone(),
            total: total_score as u32,
            latency_component: latency_score as u32,
            loss_component: loss_score as u32,
            success_component: success_score as u32,
            policy_bias: policy_bias as u32,
        }
    }

    /// Score based on latency (lower RTT is better)
    fn score_latency(&self, stats: &HealthStats) -> f64 {
        if stats.avg_rtt_ms == 0.0 {
            return 0.0;
        }
        
        // Normalize RTT: score decreases as RTT approaches max_rtt_ms
        let normalized = 1.0 - (stats.avg_rtt_ms / self.config.max_rtt_ms).min(1.0);
        normalized * self.config.weight_latency
    }

    /// Score based on packet loss (lower loss is better)
    fn score_loss(&self, stats: &HealthStats) -> f64 {
        // Normalize loss: score decreases as loss approaches max_loss_percent
        let normalized = 1.0 - (stats.packet_loss_percent / self.config.max_loss_percent).min(1.0);
        normalized * self.config.weight_loss
    }

    /// Score based on success rate (higher success is better)
    fn score_success(&self, stats: &HealthStats) -> f64 {
        if stats.total_probes == 0 {
            return 0.0;
        }
        
        let success_rate = stats.successful_probes as f64 / stats.total_probes as f64;
        success_rate * self.config.weight_success
    }

    /// Score based on policy bias (interface type preference)
    fn score_policy_bias(&self, interface: &InterfaceInfo) -> f64 {
        if !self.config.enable_policy_bias {
            return 0.0;
        }
        
        let bias = interface.iface_type.default_bias() as f64;
        bias * self.config.policy_bias_multiplier
    }
}

impl Default for InterfaceScorer {
    fn default() -> Self {
        Self::new(ScoringConfig::default())
    }
}

/// Interface score breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceScore {
    pub interface: String,
    pub total: u32,
    pub latency_component: u32,
    pub loss_component: u32,
    pub success_component: u32,
    pub policy_bias: u32,
}

impl InterfaceScore {
    /// Compare scores (for sorting)
    pub fn is_better_than(&self, other: &InterfaceScore) -> bool {
        self.total > other.total
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectivity::backhaul::discovery::InterfaceType;

    fn mock_interface(name: &str, iface_type: InterfaceType) -> InterfaceInfo {
        InterfaceInfo {
            name: name.to_string(),
            iface_type,
            is_up: true,
            has_carrier: true,
            has_address: true,
            mtu: 1500,
            mac_address: None,
            ipv4_addresses: vec![],
            ipv6_addresses: vec![],
        }
    }

    fn mock_health_stats(avg_rtt: f64, loss_percent: f64, success_rate: f64) -> HealthStats {
        let mut stats = HealthStats::new("test".to_string());
        stats.avg_rtt_ms = avg_rtt;
        stats.packet_loss_percent = loss_percent;
        
        let total = 100;
        let successful = (total as f64 * success_rate) as usize;
        stats.total_probes = total;
        stats.successful_probes = successful;
        stats.failed_probes = total - successful;
        
        stats
    }

    #[test]
    fn test_latency_scoring() {
        let scorer = InterfaceScorer::default();
        
        let good_latency = mock_health_stats(10.0, 0.0, 1.0);
        let bad_latency = mock_health_stats(200.0, 0.0, 1.0);
        
        let good_score = scorer.score_latency(&good_latency);
        let bad_score = scorer.score_latency(&bad_latency);
        
        assert!(good_score > bad_score);
    }

    #[test]
    fn test_loss_scoring() {
        let scorer = InterfaceScorer::default();
        
        let no_loss = mock_health_stats(50.0, 0.0, 1.0);
        let high_loss = mock_health_stats(50.0, 10.0, 1.0);
        
        let good_score = scorer.score_loss(&no_loss);
        let bad_score = scorer.score_loss(&high_loss);
        
        assert!(good_score > bad_score);
    }

    #[test]
    fn test_success_scoring() {
        let scorer = InterfaceScorer::default();
        
        let high_success = mock_health_stats(50.0, 0.0, 1.0);
        let low_success = mock_health_stats(50.0, 0.0, 0.5);
        
        let good_score = scorer.score_success(&high_success);
        let bad_score = scorer.score_success(&low_success);
        
        assert!(good_score > bad_score);
    }

    #[test]
    fn test_policy_bias() {
        let scorer = InterfaceScorer::default();
        
        let ethernet = mock_interface("eth0", InterfaceType::Ethernet);
        let wifi = mock_interface("wlan0", InterfaceType::WiFi);
        let lte = mock_interface("wwan0", InterfaceType::LteModem);
        
        let eth_bias = scorer.score_policy_bias(&ethernet);
        let wifi_bias = scorer.score_policy_bias(&wifi);
        let lte_bias = scorer.score_policy_bias(&lte);
        
        assert!(eth_bias > wifi_bias);
        assert!(wifi_bias > lte_bias);
    }

    #[test]
    fn test_total_score_comparison() {
        let scorer = InterfaceScorer::default();
        
        let ethernet = mock_interface("eth0", InterfaceType::Ethernet);
        let wifi = mock_interface("wlan0", InterfaceType::WiFi);
        
        let good_health = mock_health_stats(20.0, 0.0, 1.0);
        let ok_health = mock_health_stats(50.0, 2.0, 0.95);
        
        let eth_score = scorer.score(&ethernet, &good_health);
        let wifi_score = scorer.score(&wifi, &ok_health);
        
        // Ethernet with good health should score higher than WiFi with ok health
        assert!(eth_score.is_better_than(&wifi_score));
    }

    #[test]
    fn test_score_without_policy_bias() {
        let mut config = ScoringConfig::default();
        config.enable_policy_bias = false;
        let scorer = InterfaceScorer::new(config);
        
        let ethernet = mock_interface("eth0", InterfaceType::Ethernet);
        let wifi = mock_interface("wlan0", InterfaceType::WiFi);
        
        let same_health = mock_health_stats(50.0, 1.0, 0.98);
        
        let eth_score = scorer.score(&ethernet, &same_health);
        let wifi_score = scorer.score(&wifi, &same_health);
        
        // Without policy bias and same health, scores should be equal
        assert_eq!(eth_score.total, wifi_score.total);
    }
}
