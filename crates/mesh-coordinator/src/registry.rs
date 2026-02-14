use ambient_node::AmbientNode;
use std::collections::HashMap;

/// Node registry for tracking active nodes
pub struct NodeRegistry {
    nodes: HashMap<String, AmbientNode>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn register(&mut self, node: AmbientNode) {
        let node_id = node.id.id.clone();
        self.nodes.insert(node_id, node);
    }

    pub fn unregister(&mut self, node_id: &str) -> Option<AmbientNode> {
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
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
