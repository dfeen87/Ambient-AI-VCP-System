//! Local Node Observability
//!
//! **Privacy & Decentralization Constraints**
//!
//! This module provides a local-only, read-only observability interface for node operators
//! to inspect their own node's activity. Key design principles:
//!
//! - **Local-only access**: Binds strictly to 127.0.0.1 (no external network access)
//! - **Operator-only**: Only the node owner can access this interface
//! - **Read-only**: No mutation or control of execution state
//! - **Privacy-preserving**: Does NOT expose:
//!   - Private payloads or model inputs
//!   - Secrets or credentials
//!   - Information about other nodes
//!   - Sensitive execution data
//! - **No telemetry**: Does NOT:
//!   - Send data to centralized systems
//!   - Report to external aggregators
//!   - Enable cross-node visibility
//!
//! ## Architecture
//!
//! The observability layer maintains strict separation from execution logic:
//! - Observability MAY depend on execution state (read-only)
//! - Execution MUST NEVER depend on observability
//! - No shared global state
//! - No blocking operations that could affect execution
//!
//! ## Usage
//!
//! When a node starts with observability enabled, it:
//! 1. Starts a local HTTP server on 127.0.0.1:<port>
//! 2. Prints a curl command to stdout for operator inspection
//! 3. Exposes /node/status endpoint with high-level, non-sensitive data
//!
//! The feature is **disabled by default** and must be explicitly enabled.

use crate::AmbientNode;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// High-level workload category (non-payload, non-sensitive)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadCategory {
    /// Generative AI inference
    Generation,
    /// Zero-knowledge proof computation
    ZkProof,
    /// Federated learning aggregation
    FederatedLearning,
    /// General computation
    Compute,
    /// Idle (no active workload)
    Idle,
}

/// Trust decision metadata (scores, thresholds, hashes - no payloads)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustDecisionSummary {
    /// Minimum trust threshold configured
    pub trust_threshold: f64,
    /// Last consensus trust score (0.0 - 1.0)
    pub last_trust_score: f64,
    /// Lineage hash (deterministic execution tracking)
    pub lineage_hash: String,
    /// Number of models used in last consensus
    pub models_used: usize,
}

/// Local resource usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU usage percentage (0.0 - 100.0)
    pub cpu_percent: f64,
    /// Memory usage percentage (0.0 - 100.0)
    pub memory_percent: f64,
    /// Temperature in Celsius
    pub temperature_c: f64,
}

/// Node status (high-level, non-sensitive data only)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    /// Node identifier (region, type - no private ID)
    pub node_region: String,
    pub node_type: String,
    /// Node uptime in seconds
    pub uptime_seconds: u64,
    /// Current workload category
    pub current_workload: WorkloadCategory,
    /// Local resource usage
    pub resources: ResourceUsage,
    /// Trust decision summary (no payloads)
    pub trust_summary: Option<TrustDecisionSummary>,
    /// Health score (0.0 - 1.0)
    pub health_score: f64,
    /// Safe mode active (circuit breaker)
    pub safe_mode: bool,
    /// Timestamp of this status snapshot
    pub timestamp: u64,
}

impl NodeStatus {
    /// Create status snapshot from node state
    pub fn from_node(node: &AmbientNode, start_time: SystemTime) -> Self {
        let now = SystemTime::now();
        let uptime = now.duration_since(start_time).unwrap_or_default().as_secs();

        Self {
            node_region: node.id.region.clone(),
            node_type: node.id.node_type.clone(),
            uptime_seconds: uptime,
            current_workload: WorkloadCategory::Idle, // TODO: Track actual workload
            resources: ResourceUsage {
                cpu_percent: node.telemetry.cpu_usage_percent,
                memory_percent: node.telemetry.memory_usage_percent,
                temperature_c: node.telemetry.temperature_c,
            },
            trust_summary: None, // TODO: Integrate with AILEE when available
            health_score: node.health_score(),
            safe_mode: node.is_safe_mode(),
            timestamp: now.duration_since(UNIX_EPOCH).unwrap().as_secs(),
        }
    }
}

/// Shared node state for observability (read-only access)
#[derive(Clone)]
pub struct ObservableNodeState {
    node: Arc<RwLock<AmbientNode>>,
    start_time: SystemTime,
}

impl ObservableNodeState {
    /// Create new observable state
    pub fn new(node: Arc<RwLock<AmbientNode>>) -> Self {
        Self {
            node,
            start_time: SystemTime::now(),
        }
    }

    /// Get current node status snapshot
    pub async fn status(&self) -> NodeStatus {
        let node = self.node.read().await;
        NodeStatus::from_node(&node, self.start_time)
    }
}

/// Local observability server (bound to 127.0.0.1 only)
pub struct LocalObservabilityServer {
    port: u16,
    state: ObservableNodeState,
}

impl LocalObservabilityServer {
    /// Create new local observability server
    ///
    /// # Security
    ///
    /// The server will bind to 127.0.0.1 only, ensuring it is accessible
    /// only by the local node operator, not from external networks.
    pub fn new(port: u16, node: Arc<RwLock<AmbientNode>>) -> Self {
        Self {
            port,
            state: ObservableNodeState::new(node),
        }
    }

    /// Print curl command for operator
    pub fn print_curl_command(&self) {
        println!("\n╭─────────────────────────────────────────────────╮");
        println!("│  Local Node Observability (Operator-Only)      │");
        println!("╰─────────────────────────────────────────────────╯");
        println!();
        println!("  Inspect your node status:");
        println!(
            "  \x1b[1;36mcurl http://127.0.0.1:{}/node/status | jq\x1b[0m",
            self.port
        );
        println!();
        println!("  (Access limited to localhost only)");
        println!();
    }

    /// Run the observability server
    ///
    /// This starts an HTTP server bound to 127.0.0.1 that serves the /node/status endpoint.
    /// The server runs until the process is terminated.
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use axum::{extract::State, routing::get, Json, Router};
        use std::net::SocketAddr;

        let state = self.state.clone();

        // Define the /node/status endpoint
        let app = Router::new()
            .route(
                "/node/status",
                get(|State(state): State<ObservableNodeState>| async move {
                    let status = state.status().await;
                    Json(status)
                }),
            )
            .with_state(state);

        // Bind STRICTLY to 127.0.0.1 (local-only access)
        let addr = SocketAddr::from(([127, 0, 0, 1], self.port));

        tracing::info!("Starting local observability server on {}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NodeId, SafetyPolicy};

    #[test]
    fn test_node_status_creation() {
        let node_id = NodeId::new("test-node", "us-west", "compute");
        let node = AmbientNode::new(node_id, SafetyPolicy::default());
        let start_time = SystemTime::now();

        let status = NodeStatus::from_node(&node, start_time);

        assert_eq!(status.node_region, "us-west");
        assert_eq!(status.node_type, "compute");
        assert!(status.health_score >= 0.0 && status.health_score <= 1.0);
        assert!(!status.safe_mode);
    }

    #[test]
    fn test_workload_category_serialization() {
        let category = WorkloadCategory::Generation;
        let json = serde_json::to_string(&category).unwrap();
        assert_eq!(json, "\"generation\"");
    }

    #[tokio::test]
    async fn test_observable_state() {
        let node_id = NodeId::new("test-node", "eu-central", "gateway");
        let node = AmbientNode::new(node_id, SafetyPolicy::default());
        let node_arc = Arc::new(RwLock::new(node));

        let state = ObservableNodeState::new(node_arc);
        let status = state.status().await;

        assert_eq!(status.node_region, "eu-central");
        assert_eq!(status.node_type, "gateway");
    }

    #[test]
    fn test_server_creation() {
        let node_id = NodeId::new("test-node", "ap-south", "compute");
        let node = AmbientNode::new(node_id, SafetyPolicy::default());
        let node_arc = Arc::new(RwLock::new(node));

        let server = LocalObservabilityServer::new(8080, node_arc);
        assert_eq!(server.port, 8080);
    }
}
