//! Mesh/peer routing for internet connectivity
//!
//! This module analyzes node connectivity and implements peer routing logic
//! so that nodes without a direct internet path can connect through relay
//! peers that are designated as "universal" or "open" network nodes.
//!
//! Routing is *connection-only*: it resolves a forwarding path to the
//! internet but does not schedule or execute application workloads.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Internet connectivity status of a node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeConnectivityStatus {
    /// Node has a confirmed internet path
    Online,
    /// Node has no confirmed internet path
    Offline,
    /// Connectivity has not been probed yet
    Unknown,
}

/// Network role classification derived from `NodeId::node_type`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeKind {
    /// Universal node – participates in all roles; always eligible to relay
    Universal,
    /// Open node – publicly reachable; eligible to relay traffic for peers
    Open,
    /// Standard node – participates in tasks but does not relay
    Standard,
}

impl NodeKind {
    /// Derive the node kind from the string stored in `NodeId::node_type`.
    pub fn from_node_type(node_type: &str) -> Self {
        match node_type.to_lowercase().as_str() {
            "universal" => NodeKind::Universal,
            "open" | "gateway" => NodeKind::Open,
            _ => NodeKind::Standard,
        }
    }

    /// Whether this node kind is eligible to relay internet traffic for peers.
    pub fn can_relay(&self) -> bool {
        matches!(self, NodeKind::Universal | NodeKind::Open)
    }
}

/// A single hop in a resolved routing path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingHop {
    pub node_id: String,
    pub kind: NodeKind,
}

/// A resolved peer routing path from a source node to the internet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerRoute {
    pub source_node_id: String,
    /// Ordered list of relay hops; empty means direct internet access.
    pub hops: Vec<RoutingHop>,
}

impl PeerRoute {
    /// True when the source already has a direct internet connection.
    pub fn is_direct(&self) -> bool {
        self.hops.is_empty()
    }
}

/// Peer router: tracks node connectivity and resolves routing paths
///
/// `PeerRouter` is maintained by the `MeshCoordinator` and updated whenever
/// a node's backhaul state changes.  Call [`PeerRouter::find_route`] to
/// resolve the best path to the internet for any registered node.
pub struct PeerRouter {
    connectivity: HashMap<String, NodeConnectivityStatus>,
    kinds: HashMap<String, NodeKind>,
}

impl PeerRouter {
    pub fn new() -> Self {
        Self {
            connectivity: HashMap::new(),
            kinds: HashMap::new(),
        }
    }

    /// Register or update a node's connectivity status and role.
    pub fn update_node(
        &mut self,
        node_id: &str,
        node_type: &str,
        status: NodeConnectivityStatus,
    ) {
        self.connectivity.insert(node_id.to_string(), status);
        self.kinds
            .insert(node_id.to_string(), NodeKind::from_node_type(node_type));
    }

    /// Remove a node from the router (e.g. on deregistration).
    pub fn remove_node(&mut self, node_id: &str) {
        self.connectivity.remove(node_id);
        self.kinds.remove(node_id);
    }

    /// Return the known connectivity status for a node.
    pub fn connectivity_status(&self, node_id: &str) -> NodeConnectivityStatus {
        self.connectivity
            .get(node_id)
            .copied()
            .unwrap_or(NodeConnectivityStatus::Unknown)
    }

    /// IDs of all nodes currently known to be online.
    pub fn online_nodes(&self) -> Vec<String> {
        self.connectivity
            .iter()
            .filter(|(_, s)| **s == NodeConnectivityStatus::Online)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Find the best routing path for `source_node_id` to reach the internet.
    ///
    /// Returns:
    /// - `Some(PeerRoute { hops: [] })` – node is directly online.
    /// - `Some(PeerRoute { hops: [relay] })` – node must hop through a relay.
    ///   Relay selection prefers `Universal` over `Open` nodes.
    /// - `None` – no internet path is available (source is offline and no
    ///   suitable relay exists).
    pub fn find_route(&self, source_node_id: &str) -> Option<PeerRoute> {
        let source_status = self.connectivity_status(source_node_id);

        // Direct connection: no relay needed.
        if source_status == NodeConnectivityStatus::Online {
            return Some(PeerRoute {
                source_node_id: source_node_id.to_string(),
                hops: vec![],
            });
        }

        // Collect online relay candidates (Universal and Open nodes only).
        let mut candidates: Vec<(String, NodeKind)> = self
            .connectivity
            .iter()
            .filter(|(id, status)| {
                id.as_str() != source_node_id
                    && **status == NodeConnectivityStatus::Online
            })
            .filter_map(|(id, _)| {
                let kind = self
                    .kinds
                    .get(id.as_str())
                    .copied()
                    .unwrap_or(NodeKind::Standard);
                kind.can_relay().then_some((id.clone(), kind))
            })
            .collect();

        if candidates.is_empty() {
            return None;
        }

        // Prefer Universal relays over Open relays; break ties by node ID for
        // deterministic selection without requiring additional state.
        candidates.sort_by(|(id_a, kind_a), (id_b, kind_b)| {
            let rank = |k: &NodeKind| match k {
                NodeKind::Universal => 0u8,
                NodeKind::Open => 1,
                NodeKind::Standard => 2,
            };
            rank(kind_a).cmp(&rank(kind_b)).then(id_a.cmp(id_b))
        });

        let (relay_id, relay_kind) = candidates.remove(0);
        Some(PeerRoute {
            source_node_id: source_node_id.to_string(),
            hops: vec![RoutingHop {
                node_id: relay_id,
                kind: relay_kind,
            }],
        })
    }
}

impl Default for PeerRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_router() -> PeerRouter {
        let mut r = PeerRouter::new();
        r.update_node("node-open", "open", NodeConnectivityStatus::Online);
        r.update_node(
            "node-universal",
            "universal",
            NodeConnectivityStatus::Online,
        );
        r.update_node("node-standard", "standard", NodeConnectivityStatus::Online);
        r.update_node("node-offline", "open", NodeConnectivityStatus::Offline);
        r.update_node("node-unknown", "open", NodeConnectivityStatus::Unknown);
        r
    }

    // --- NodeKind ---

    #[test]
    fn test_node_kind_from_node_type() {
        assert_eq!(NodeKind::from_node_type("universal"), NodeKind::Universal);
        assert_eq!(NodeKind::from_node_type("UNIVERSAL"), NodeKind::Universal);
        assert_eq!(NodeKind::from_node_type("open"), NodeKind::Open);
        assert_eq!(NodeKind::from_node_type("gateway"), NodeKind::Open);
        assert_eq!(NodeKind::from_node_type("worker"), NodeKind::Standard);
        assert_eq!(NodeKind::from_node_type(""), NodeKind::Standard);
    }

    #[test]
    fn test_node_kind_can_relay() {
        assert!(NodeKind::Universal.can_relay());
        assert!(NodeKind::Open.can_relay());
        assert!(!NodeKind::Standard.can_relay());
    }

    // --- PeerRoute ---

    #[test]
    fn test_peer_route_is_direct() {
        let direct = PeerRoute {
            source_node_id: "a".to_string(),
            hops: vec![],
        };
        assert!(direct.is_direct());

        let relayed = PeerRoute {
            source_node_id: "a".to_string(),
            hops: vec![RoutingHop {
                node_id: "b".to_string(),
                kind: NodeKind::Open,
            }],
        };
        assert!(!relayed.is_direct());
    }

    // --- PeerRouter ---

    #[test]
    fn test_connectivity_status_defaults_to_unknown() {
        let r = PeerRouter::new();
        assert_eq!(
            r.connectivity_status("nonexistent"),
            NodeConnectivityStatus::Unknown
        );
    }

    #[test]
    fn test_update_and_remove_node() {
        let mut r = PeerRouter::new();
        r.update_node("n1", "open", NodeConnectivityStatus::Online);
        assert_eq!(r.connectivity_status("n1"), NodeConnectivityStatus::Online);

        r.update_node("n1", "open", NodeConnectivityStatus::Offline);
        assert_eq!(r.connectivity_status("n1"), NodeConnectivityStatus::Offline);

        r.remove_node("n1");
        assert_eq!(
            r.connectivity_status("n1"),
            NodeConnectivityStatus::Unknown
        );
    }

    #[test]
    fn test_online_nodes_lists_only_online() {
        let r = make_router();
        let online = r.online_nodes();
        // node-open, node-universal, node-standard are Online
        assert_eq!(online.len(), 3);
        assert!(online.contains(&"node-open".to_string()));
        assert!(online.contains(&"node-universal".to_string()));
        assert!(online.contains(&"node-standard".to_string()));
    }

    #[test]
    fn test_find_route_direct_for_online_node() {
        let r = make_router();
        let route = r.find_route("node-open");
        assert!(route.is_some());
        let route = route.unwrap();
        assert!(route.is_direct());
        assert_eq!(route.source_node_id, "node-open");
    }

    #[test]
    fn test_find_route_prefers_universal_relay() {
        let r = make_router();
        // node-offline needs a relay; both node-open (Open) and node-universal
        // (Universal) are online and can relay → Universal should be chosen.
        let route = r.find_route("node-offline").expect("expected a route");
        assert!(!route.is_direct());
        assert_eq!(route.hops.len(), 1);
        assert_eq!(route.hops[0].kind, NodeKind::Universal);
        assert_eq!(route.hops[0].node_id, "node-universal");
    }

    #[test]
    fn test_find_route_falls_back_to_open_relay() {
        let mut r = PeerRouter::new();
        r.update_node("node-open", "open", NodeConnectivityStatus::Online);
        r.update_node("node-offline", "standard", NodeConnectivityStatus::Offline);

        let route = r.find_route("node-offline").expect("expected a route");
        assert_eq!(route.hops[0].kind, NodeKind::Open);
        assert_eq!(route.hops[0].node_id, "node-open");
    }

    #[test]
    fn test_find_route_none_when_no_relay_available() {
        let mut r = PeerRouter::new();
        // Only standard nodes online – none can relay.
        r.update_node("node-std", "standard", NodeConnectivityStatus::Online);
        r.update_node("node-offline", "open", NodeConnectivityStatus::Offline);

        assert!(r.find_route("node-offline").is_none());
    }

    #[test]
    fn test_find_route_none_for_unknown_node() {
        let r = make_router();
        // An unregistered node has no direct path and triggers relay search.
        // node-universal and node-open are available relays, so a route exists.
        let route = r.find_route("brand-new-node");
        assert!(route.is_some());
        assert!(!route.unwrap().is_direct());
    }

    #[test]
    fn test_find_route_no_relay_to_self() {
        let mut r = PeerRouter::new();
        // Single universal node looking for its own route → direct.
        r.update_node("solo", "universal", NodeConnectivityStatus::Online);
        let route = r.find_route("solo").expect("expected direct route");
        assert!(route.is_direct());
    }

    #[test]
    fn test_find_route_offline_node_cannot_relay_to_itself() {
        let mut r = PeerRouter::new();
        r.update_node("solo", "universal", NodeConnectivityStatus::Offline);
        assert!(r.find_route("solo").is_none());
    }
}
