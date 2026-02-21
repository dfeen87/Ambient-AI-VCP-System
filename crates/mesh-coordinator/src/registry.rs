use ambient_node::AmbientNode;
use std::collections::HashMap;

/// Node registry for tracking active nodes
pub struct NodeRegistry {
    nodes: HashMap<String, AmbientNode>,
    /// Unix-epoch seconds of the last recorded heartbeat for each node.
    heartbeats: HashMap<String, u64>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            heartbeats: HashMap::new(),
        }
    }

    pub fn register(&mut self, node: AmbientNode) {
        let node_id = node.id.id.clone();
        self.nodes.insert(node_id, node);
    }

    pub fn unregister(&mut self, node_id: &str) -> Option<AmbientNode> {
        self.heartbeats.remove(node_id);
        self.nodes.remove(node_id)
    }

    pub fn get(&self, node_id: &str) -> Option<&AmbientNode> {
        self.nodes.get(node_id)
    }

    pub fn get_mut(&mut self, node_id: &str) -> Option<&mut AmbientNode> {
        self.nodes.get_mut(node_id)
    }

    pub fn all_nodes(&self) -> Vec<&AmbientNode> {
        self.nodes.values().collect()
    }

    pub fn count(&self) -> usize {
        self.nodes.len()
    }

    /// Record a heartbeat for `node_id` at the given Unix-epoch timestamp.
    ///
    /// Call this whenever a registered node reports liveness (e.g. after a
    /// successful hardware keepalive probe or an application-level ping).
    /// Nodes that have not sent a heartbeat within the timeout window can be
    /// removed with [`NodeRegistry::eject_stale_nodes`].
    ///
    /// # Returns
    /// `true` if `node_id` is registered, `false` if it is unknown.
    pub fn record_heartbeat(&mut self, node_id: &str, now: u64) -> bool {
        if self.nodes.contains_key(node_id) {
            self.heartbeats.insert(node_id.to_string(), now);
            true
        } else {
            false
        }
    }

    /// Return the Unix-epoch seconds of the last heartbeat for `node_id`, or
    /// `None` if no heartbeat has been recorded yet.
    pub fn last_heartbeat(&self, node_id: &str) -> Option<u64> {
        self.heartbeats.get(node_id).copied()
    }

    /// Remove nodes whose last heartbeat is older than `timeout_secs` seconds
    /// relative to `now`.
    ///
    /// Nodes that have *never* sent a heartbeat are also ejected, since they
    /// have been unreachable since registration.
    ///
    /// Returns the IDs of every ejected node so the caller can update dependent
    /// state (e.g. peer routing tables).
    pub fn eject_stale_nodes(&mut self, now: u64, timeout_secs: u64) -> Vec<String> {
        let stale: Vec<String> = self
            .nodes
            .keys()
            .filter(|id| {
                match self.heartbeats.get(*id) {
                    Some(&last) => now.saturating_sub(last) > timeout_secs,
                    // Never sent a heartbeat → treat as stale.
                    None => true,
                }
            })
            .cloned()
            .collect();

        for id in &stale {
            self.nodes.remove(id);
            self.heartbeats.remove(id);
        }

        stale
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambient_node::{NodeId, SafetyPolicy};

    fn make_node(id: &str) -> AmbientNode {
        AmbientNode::new(
            NodeId::new(id, "us-west", "worker"),
            SafetyPolicy::default(),
        )
    }

    #[test]
    fn test_record_heartbeat_known_node() {
        let mut registry = NodeRegistry::new();
        registry.register(make_node("n1"));

        assert!(registry.record_heartbeat("n1", 1000));
        assert_eq!(registry.last_heartbeat("n1"), Some(1000));
    }

    #[test]
    fn test_record_heartbeat_unknown_node() {
        let mut registry = NodeRegistry::new();
        // "n-ghost" was never registered.
        assert!(!registry.record_heartbeat("n-ghost", 1000));
        assert_eq!(registry.last_heartbeat("n-ghost"), None);
    }

    #[test]
    fn test_eject_stale_nodes_removes_timed_out() {
        let mut registry = NodeRegistry::new();
        registry.register(make_node("n-stale-1"));
        registry.register(make_node("n-stale-2"));

        let t0 = 1_000u64;
        registry.record_heartbeat("n-stale-1", t0);
        registry.record_heartbeat("n-stale-2", t0);

        // Advance time by 61 seconds; timeout is 60 s.  Both nodes are stale.
        let now = t0 + 61;
        let ejected = registry.eject_stale_nodes(now, 60);

        assert_eq!(ejected.len(), 2, "both nodes exceeded the timeout");
        assert!(registry.nodes.is_empty());
    }

    #[test]
    fn test_eject_stale_nodes_keeps_fresh() {
        let mut registry = NodeRegistry::new();
        registry.register(make_node("n-fresh"));
        registry.register(make_node("n-stale"));

        let t0 = 1_000u64;
        registry.record_heartbeat("n-stale", t0);
        // n-fresh gets a heartbeat right at "now".
        let now = t0 + 61;
        registry.record_heartbeat("n-fresh", now);

        let ejected = registry.eject_stale_nodes(now, 60);

        assert_eq!(ejected, vec!["n-stale".to_string()]);
        assert!(registry.nodes.contains_key("n-fresh"));
        assert!(!registry.nodes.contains_key("n-stale"));
    }

    #[test]
    fn test_eject_stale_nodes_never_heartbeated() {
        let mut registry = NodeRegistry::new();
        registry.register(make_node("n-silent"));

        // Node was registered but never sent a heartbeat.
        let ejected = registry.eject_stale_nodes(9999, 60);

        assert_eq!(ejected, vec!["n-silent".to_string()]);
        assert!(registry.nodes.is_empty());
    }

    #[test]
    fn test_unregister_clears_heartbeat() {
        let mut registry = NodeRegistry::new();
        registry.register(make_node("n1"));
        registry.record_heartbeat("n1", 500);
        registry.unregister("n1");

        assert!(registry.last_heartbeat("n1").is_none());
        assert!(registry.nodes.is_empty());
    }
}
