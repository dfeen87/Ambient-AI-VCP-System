//! Health probing for backhaul interfaces
//!
//! Performs periodic async probes to multiple targets:
//! - Control-plane FQDN
//! - Gateway
//! - 1-2 neutral internet hosts
//!
//! Collects metrics:
//! - RTT (Round-Trip Time)
//! - Success/failure rate
//! - Basic packet loss estimate

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use tokio::net::TcpSocket;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::debug;

/// Health probe configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeConfig {
    /// Probe interval
    pub interval_secs: u64,

    /// Probe timeout
    pub timeout_secs: u64,

    /// Targets to probe
    pub targets: Vec<ProbeTarget>,

    /// Failure threshold before marking degraded
    pub degraded_threshold: usize,

    /// Failure threshold before marking down
    pub down_threshold: usize,
}

impl Default for ProbeConfig {
    fn default() -> Self {
        Self {
            interval_secs: 5,
            timeout_secs: 3,
            targets: vec![
                ProbeTarget {
                    name: "cloudflare-dns".to_string(),
                    address: "1.1.1.1".to_string(),
                    port: 53,
                    probe_type: ProbeType::TcpConnect,
                },
                ProbeTarget {
                    name: "google-dns".to_string(),
                    address: "8.8.8.8".to_string(),
                    port: 53,
                    probe_type: ProbeType::TcpConnect,
                },
            ],
            degraded_threshold: 1,
            down_threshold: 2,
        }
    }
}

/// Probe target specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeTarget {
    pub name: String,
    pub address: String,
    pub port: u16,
    pub probe_type: ProbeType,
}

/// Type of probe to perform
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ProbeType {
    /// TCP connection test
    TcpConnect,
    /// ICMP ping (requires privileges)
    #[allow(dead_code)]
    IcmpPing,
}

/// Result of a single probe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    pub target_name: String,
    pub success: bool,
    pub rtt_ms: Option<u64>,
    pub error: Option<String>,
    pub timestamp: u64,
}

/// Health probe statistics for an interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStats {
    pub interface: String,
    pub total_probes: usize,
    pub successful_probes: usize,
    pub failed_probes: usize,
    pub avg_rtt_ms: f64,
    pub min_rtt_ms: u64,
    pub max_rtt_ms: u64,
    pub packet_loss_percent: f64,
    pub last_success: Option<u64>,
    pub last_failure: Option<u64>,
    pub consecutive_failures: usize,
}

impl HealthStats {
    pub fn new(interface: String) -> Self {
        Self {
            interface,
            total_probes: 0,
            successful_probes: 0,
            failed_probes: 0,
            avg_rtt_ms: 0.0,
            min_rtt_ms: u64::MAX,
            max_rtt_ms: 0,
            packet_loss_percent: 0.0,
            last_success: None,
            last_failure: None,
            consecutive_failures: 0,
        }
    }

    /// Update statistics with a new probe result
    pub fn update(&mut self, result: &ProbeResult) {
        self.total_probes += 1;

        if result.success {
            self.successful_probes += 1;
            self.consecutive_failures = 0;
            self.last_success = Some(result.timestamp);

            if let Some(rtt) = result.rtt_ms {
                self.min_rtt_ms = self.min_rtt_ms.min(rtt);
                self.max_rtt_ms = self.max_rtt_ms.max(rtt);

                // Running average
                let total_rtt = self.avg_rtt_ms * (self.successful_probes - 1) as f64;
                self.avg_rtt_ms = (total_rtt + rtt as f64) / self.successful_probes as f64;
            }
        } else {
            self.failed_probes += 1;
            self.consecutive_failures += 1;
            self.last_failure = Some(result.timestamp);
        }

        // Calculate packet loss percentage
        if self.total_probes > 0 {
            self.packet_loss_percent =
                (self.failed_probes as f64 / self.total_probes as f64) * 100.0;
        }
    }

    /// Check if interface is healthy
    pub fn is_healthy(&self, config: &ProbeConfig) -> bool {
        self.consecutive_failures < config.degraded_threshold
    }

    /// Check if interface is degraded
    pub fn is_degraded(&self, config: &ProbeConfig) -> bool {
        self.consecutive_failures >= config.degraded_threshold
            && self.consecutive_failures < config.down_threshold
    }

    /// Check if interface is down
    pub fn is_down(&self, config: &ProbeConfig) -> bool {
        self.consecutive_failures >= config.down_threshold
    }
}

/// Health prober for an interface
pub struct HealthProber {
    interface: String,
    config: ProbeConfig,
    stats: HealthStats,
    /// Local IPv4 address of the interface, used to bind outgoing probes so
    /// that each probe travels through the correct interface rather than
    /// following the OS default route.
    local_addr: Option<String>,
}

impl HealthProber {
    pub fn new(interface: String, config: ProbeConfig) -> Self {
        Self {
            stats: HealthStats::new(interface.clone()),
            interface,
            config,
            local_addr: None,
        }
    }

    /// Bind outgoing probes to `addr` (the interface's IPv4 address).
    ///
    /// This ensures each probe is routed through the correct interface instead
    /// of following the OS default route, which would give false-healthy
    /// readings when another interface is currently preferred.
    pub fn with_local_addr(mut self, addr: String) -> Self {
        self.local_addr = Some(addr);
        self
    }

    /// Perform a single probe cycle
    pub async fn probe_once(&mut self) -> Vec<ProbeResult> {
        let mut results = Vec::new();

        for target in &self.config.targets {
            let result = self.probe_target(target).await;
            self.stats.update(&result);
            results.push(result);
        }

        results
    }

    /// Probe a single target
    async fn probe_target(&self, target: &ProbeTarget) -> ProbeResult {
        let start = Instant::now();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        match target.probe_type {
            ProbeType::TcpConnect => self.tcp_probe(target, start, timestamp).await,
            ProbeType::IcmpPing => {
                // ICMP ping requires raw sockets / elevated privileges
                // For this implementation, we'll fall back to TCP
                debug!(target = %target.name, "ICMP ping not implemented, using TCP");
                self.tcp_probe(target, start, timestamp).await
            }
        }
    }

    /// Perform TCP connection probe
    ///
    /// When a `local_addr` is configured the socket is bound to that address
    /// before connecting.  This forces the probe to travel through the
    /// interface that owns that address, giving an accurate health signal even
    /// when policy routing rules are active.
    async fn tcp_probe(&self, target: &ProbeTarget, start: Instant, timestamp: u64) -> ProbeResult {
        let timeout_duration = Duration::from_secs(self.config.timeout_secs);
        let addr = format!("{}:{}", target.address, target.port);

        let connect_result: std::io::Result<TcpStream> = if let Some(local_ip) = &self.local_addr {
            // Bind to the interface's local IP so the probe is routed through
            // that interface regardless of the current policy routing state.
            let local: SocketAddr = match format!("{}:0", local_ip).parse() {
                Ok(a) => a,
                Err(e) => {
                    return ProbeResult {
                        target_name: target.name.clone(),
                        success: false,
                        rtt_ms: None,
                        error: Some(format!("invalid local address {}: {}", local_ip, e)),
                        timestamp,
                    };
                }
            };
            let remote: SocketAddr = match addr.parse() {
                Ok(a) => a,
                Err(e) => {
                    return ProbeResult {
                        target_name: target.name.clone(),
                        success: false,
                        rtt_ms: None,
                        error: Some(format!("invalid remote address {}: {}", addr, e)),
                        timestamp,
                    };
                }
            };
            let socket_result = TcpSocket::new_v4().and_then(|s| s.bind(local).map(|_| s));
            match socket_result {
                Ok(socket) => {
                    timeout(timeout_duration, socket.connect(remote))
                        .await
                        .unwrap_or_else(|_| {
                            Err(std::io::Error::new(
                                std::io::ErrorKind::TimedOut,
                                "timeout",
                            ))
                        })
                }
                Err(e) => Err(e),
            }
        } else {
            timeout(timeout_duration, TcpStream::connect(&addr))
                .await
                .unwrap_or_else(|_| {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "timeout",
                    ))
                })
        };

        let elapsed = start.elapsed();

        match connect_result {
            Ok(_stream) => {
                debug!(
                    interface = %self.interface,
                    target = %target.name,
                    rtt_ms = elapsed.as_millis(),
                    "Probe successful"
                );
                ProbeResult {
                    target_name: target.name.clone(),
                    success: true,
                    rtt_ms: Some(elapsed.as_millis() as u64),
                    error: None,
                    timestamp,
                }
            }
            Err(e) => {
                debug!(
                    interface = %self.interface,
                    target = %target.name,
                    error = %e,
                    "Probe failed"
                );
                ProbeResult {
                    target_name: target.name.clone(),
                    success: false,
                    rtt_ms: None,
                    error: Some(e.to_string()),
                    timestamp,
                }
            }
        }
    }

    /// Get current health statistics
    pub fn stats(&self) -> &HealthStats {
        &self.stats
    }

    /// Check if interface is healthy based on current stats
    pub fn is_healthy(&self) -> bool {
        self.stats.is_healthy(&self.config)
    }

    /// Check if interface is degraded
    pub fn is_degraded(&self) -> bool {
        self.stats.is_degraded(&self.config)
    }

    /// Check if interface is down
    pub fn is_down(&self) -> bool {
        self.stats.is_down(&self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_stats_update() {
        let mut stats = HealthStats::new("eth0".to_string());

        let success_result = ProbeResult {
            target_name: "test".to_string(),
            success: true,
            rtt_ms: Some(50),
            error: None,
            timestamp: 1000,
        };

        stats.update(&success_result);
        assert_eq!(stats.successful_probes, 1);
        assert_eq!(stats.consecutive_failures, 0);
        assert_eq!(stats.avg_rtt_ms, 50.0);

        let failure_result = ProbeResult {
            target_name: "test".to_string(),
            success: false,
            rtt_ms: None,
            error: Some("timeout".to_string()),
            timestamp: 1005,
        };

        stats.update(&failure_result);
        assert_eq!(stats.failed_probes, 1);
        assert_eq!(stats.consecutive_failures, 1);
        assert_eq!(stats.packet_loss_percent, 50.0);
    }

    #[test]
    fn test_health_thresholds() {
        let config = ProbeConfig::default();
        let mut stats = HealthStats::new("eth0".to_string());

        assert!(stats.is_healthy(&config));
        assert!(!stats.is_degraded(&config));
        assert!(!stats.is_down(&config));

        // Add failure to reach degraded threshold
        stats.consecutive_failures = config.degraded_threshold;
        assert!(!stats.is_healthy(&config));
        assert!(stats.is_degraded(&config));
        assert!(!stats.is_down(&config));

        // Add failures to reach down threshold
        stats.consecutive_failures = config.down_threshold;
        assert!(!stats.is_healthy(&config));
        assert!(!stats.is_degraded(&config));
        assert!(stats.is_down(&config));
    }

    #[tokio::test]
    async fn test_health_prober_tcp() {
        let config = ProbeConfig {
            interval_secs: 5,
            timeout_secs: 1,
            targets: vec![ProbeTarget {
                name: "localhost".to_string(),
                address: "127.0.0.1".to_string(),
                port: 1, // Usually unreachable, but test will complete
                probe_type: ProbeType::TcpConnect,
            }],
            degraded_threshold: 1,
            down_threshold: 2,
        };

        let mut prober = HealthProber::new("eth0".to_string(), config);
        let results = prober.probe_once().await;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].target_name, "localhost");
        // Result may be success or failure depending on system state
        assert_eq!(prober.stats().total_probes, 1);
    }

    #[test]
    fn test_with_local_addr_builder() {
        let config = ProbeConfig::default();
        let prober = HealthProber::new("eth0".to_string(), config)
            .with_local_addr("192.168.1.100".to_string());
        assert_eq!(prober.local_addr.as_deref(), Some("192.168.1.100"));
    }

    #[tokio::test]
    async fn test_health_prober_with_local_addr_unreachable() {
        // Bind to loopback and attempt an unreachable port — the probe must
        // complete (with a failure result) rather than panic or hang.
        let config = ProbeConfig {
            interval_secs: 5,
            timeout_secs: 1,
            targets: vec![ProbeTarget {
                name: "local-unreachable".to_string(),
                address: "127.0.0.1".to_string(),
                port: 1,
                probe_type: ProbeType::TcpConnect,
            }],
            degraded_threshold: 1,
            down_threshold: 2,
        };

        let mut prober = HealthProber::new("lo".to_string(), config)
            .with_local_addr("127.0.0.1".to_string());
        let results = prober.probe_once().await;

        assert_eq!(results.len(), 1);
        assert_eq!(prober.stats().total_probes, 1);
    }
}
