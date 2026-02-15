use crate::models::*;
use anyhow::Result;
use std::collections::HashMap;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Application state
pub struct AppState {
    nodes: RwLock<HashMap<String, NodeInfo>>,
    tasks: RwLock<HashMap<String, TaskInfo>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            nodes: RwLock::new(HashMap::new()),
            tasks: RwLock::new(HashMap::new()),
        }
    }

    /// Register a new node
    pub async fn register_node(&self, registration: NodeRegistration) -> Result<NodeInfo> {
        let now = chrono::Utc::now().to_rfc3339();

        let node_info = NodeInfo {
            node_id: registration.node_id.clone(),
            region: registration.region,
            node_type: registration.node_type,
            capabilities: registration.capabilities,
            health_score: 100.0,
            status: "online".to_string(),
            registered_at: now.clone(),
            last_seen: now,
        };

        let mut nodes = self.nodes.write().await;
        nodes.insert(registration.node_id, node_info.clone());

        Ok(node_info)
    }

    /// List all nodes
    pub async fn list_nodes(&self) -> Vec<NodeInfo> {
        let nodes = self.nodes.read().await;
        nodes.values().cloned().collect()
    }

    /// Get a specific node
    pub async fn get_node(&self, node_id: &str) -> Option<NodeInfo> {
        let nodes = self.nodes.read().await;
        nodes.get(node_id).cloned()
    }

    /// Submit a task
    pub async fn submit_task(&self, task: TaskSubmission) -> Result<TaskInfo> {
        let task_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        let task_info = TaskInfo {
            task_id: task_id.clone(),
            task_type: task.task_type,
            status: TaskStatus::Pending,
            assigned_nodes: vec![],
            created_at: now.clone(),
            updated_at: now,
            result: None,
            proof_id: None,
        };

        let mut tasks = self.tasks.write().await;
        tasks.insert(task_id, task_info.clone());

        Ok(task_info)
    }

    /// Get a specific task
    pub async fn get_task(&self, task_id: &str) -> Option<TaskInfo> {
        let tasks = self.tasks.read().await;
        tasks.get(task_id).cloned()
    }

    /// List all tasks
    pub async fn list_tasks(&self) -> Vec<TaskInfo> {
        let tasks = self.tasks.read().await;
        tasks.values().cloned().collect()
    }

    /// Verify a ZK proof
    pub async fn verify_proof(
        &self,
        request: ProofVerificationRequest,
    ) -> Result<ProofVerificationResponse> {
        use std::time::Instant;

        let start = Instant::now();

        // In a real implementation, this would verify the proof using the ZK verifier
        let valid = true; // Placeholder

        let verification_time_ms = start.elapsed().as_millis() as u64;

        Ok(ProofVerificationResponse {
            valid,
            task_id: request.task_id,
            verified_at: chrono::Utc::now().to_rfc3339(),
            verification_time_ms,
        })
    }

    /// Get cluster statistics
    pub async fn get_cluster_stats(&self) -> ClusterStats {
        let nodes = self.nodes.read().await;
        let tasks = self.tasks.read().await;

        let total_nodes = nodes.len();
        let healthy_nodes = nodes
            .values()
            .filter(|n| n.status == "online" && n.health_score >= 70.0)
            .count();

        let total_tasks = tasks.len();
        let completed_tasks = tasks
            .values()
            .filter(|t| t.status == TaskStatus::Completed)
            .count();
        let failed_tasks = tasks
            .values()
            .filter(|t| t.status == TaskStatus::Failed)
            .count();

        let avg_health_score = if total_nodes > 0 {
            nodes.values().map(|n| n.health_score).sum::<f64>() / total_nodes as f64
        } else {
            0.0
        };

        let total_compute_capacity = nodes
            .values()
            .map(|n| n.capabilities.cpu_cores as f64 * n.capabilities.memory_gb)
            .sum();

        ClusterStats {
            total_nodes,
            healthy_nodes,
            total_tasks,
            completed_tasks,
            failed_tasks,
            avg_health_score,
            total_compute_capacity,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
