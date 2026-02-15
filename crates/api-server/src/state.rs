/// Application state with PostgreSQL persistence
///
/// This module provides CRUD operations for nodes and tasks using a PostgreSQL database.

use crate::error::{ApiError, ApiResult};
use crate::models::*;
use sqlx::PgPool;
use uuid::Uuid;

/// Application state with database connection pool
pub struct AppState {
    /// PostgreSQL connection pool
    pub db: PgPool,
}

impl AppState {
    /// Create new application state with database pool
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Register a new node in the database
    pub async fn register_node(&self, registration: NodeRegistration) -> ApiResult<NodeInfo> {
        let now = chrono::Utc::now();

        // Insert node into database
        sqlx::query!(
            r#"
            INSERT INTO nodes (
                node_id, region, node_type, bandwidth_mbps, cpu_cores, 
                memory_gb, gpu_available, health_score, status, 
                registered_at, last_seen
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
            registration.node_id,
            registration.region,
            registration.node_type,
            registration.capabilities.bandwidth_mbps,
            registration.capabilities.cpu_cores as i32,
            registration.capabilities.memory_gb,
            registration.capabilities.gpu_available,
            100.0_f64,
            "online",
            now,
            now,
        )
        .execute(&self.db)
        .await?;

        // Return the created node
        let node_info = NodeInfo {
            node_id: registration.node_id,
            region: registration.region,
            node_type: registration.node_type,
            capabilities: registration.capabilities,
            health_score: 100.0,
            status: "online".to_string(),
            registered_at: now.to_rfc3339(),
            last_seen: now.to_rfc3339(),
        };

        Ok(node_info)
    }

    /// List all nodes from the database
    pub async fn list_nodes(&self) -> Vec<NodeInfo> {
        let result = sqlx::query!(
            r#"
            SELECT 
                node_id, region, node_type, bandwidth_mbps, cpu_cores,
                memory_gb, gpu_available, health_score, status,
                registered_at, last_seen
            FROM nodes
            ORDER BY registered_at DESC
            "#
        )
        .fetch_all(&self.db)
        .await;

        match result {
            Ok(rows) => rows
                .into_iter()
                .map(|row| NodeInfo {
                    node_id: row.node_id,
                    region: row.region,
                    node_type: row.node_type,
                    capabilities: NodeCapabilities {
                        bandwidth_mbps: row.bandwidth_mbps,
                        cpu_cores: row.cpu_cores as u32,
                        memory_gb: row.memory_gb,
                        gpu_available: row.gpu_available,
                    },
                    health_score: row.health_score,
                    status: row.status,
                    registered_at: row.registered_at.to_rfc3339(),
                    last_seen: row.last_seen.to_rfc3339(),
                })
                .collect(),
            Err(e) => {
                tracing::error!("Failed to list nodes: {:?}", e);
                vec![]
            }
        }
    }

    /// Get a specific node from the database
    pub async fn get_node(&self, node_id: &str) -> Option<NodeInfo> {
        let result = sqlx::query!(
            r#"
            SELECT 
                node_id, region, node_type, bandwidth_mbps, cpu_cores,
                memory_gb, gpu_available, health_score, status,
                registered_at, last_seen
            FROM nodes
            WHERE node_id = $1
            "#,
            node_id
        )
        .fetch_optional(&self.db)
        .await;

        match result {
            Ok(Some(row)) => Some(NodeInfo {
                node_id: row.node_id,
                region: row.region,
                node_type: row.node_type,
                capabilities: NodeCapabilities {
                    bandwidth_mbps: row.bandwidth_mbps,
                    cpu_cores: row.cpu_cores as u32,
                    memory_gb: row.memory_gb,
                    gpu_available: row.gpu_available,
                },
                health_score: row.health_score,
                status: row.status,
                registered_at: row.registered_at.to_rfc3339(),
                last_seen: row.last_seen.to_rfc3339(),
            }),
            Ok(None) => None,
            Err(e) => {
                tracing::error!("Failed to get node {}: {:?}", node_id, e);
                None
            }
        }
    }

    /// Submit a task to the database
    pub async fn submit_task(&self, task: TaskSubmission) -> ApiResult<TaskInfo> {
        let task_id = Uuid::new_v4();
        let now = chrono::Utc::now();

        // Insert task into database
        sqlx::query!(
            r#"
            INSERT INTO tasks (
                task_id, task_type, status, wasm_module, inputs,
                min_nodes, max_execution_time_sec, require_gpu, require_proof
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            task_id,
            task.task_type,
            "pending",
            task.wasm_module,
            task.inputs,
            task.requirements.min_nodes as i32,
            task.requirements.max_execution_time_sec as i64,
            task.requirements.require_gpu,
            task.requirements.require_proof,
        )
        .execute(&self.db)
        .await?;

        let task_info = TaskInfo {
            task_id: task_id.to_string(),
            task_type: task.task_type,
            status: TaskStatus::Pending,
            assigned_nodes: vec![],
            created_at: now.to_rfc3339(),
            updated_at: now.to_rfc3339(),
            result: None,
            proof_id: None,
        };

        Ok(task_info)
    }

    /// Get a specific task from the database
    pub async fn get_task(&self, task_id: &str) -> Option<TaskInfo> {
        let task_uuid = match Uuid::parse_str(task_id) {
            Ok(uuid) => uuid,
            Err(_) => return None,
        };

        let result = sqlx::query!(
            r#"
            SELECT 
                t.task_id, t.task_type, t.status, t.result, t.proof_id,
                t.created_at, t.updated_at,
                COALESCE(
                    (
                        SELECT ARRAY_AGG(ta.node_id)
                        FROM task_assignments ta
                        WHERE ta.task_id = t.task_id
                    ),
                    ARRAY[]::VARCHAR[]
                ) as "assigned_nodes!"
            FROM tasks t
            WHERE t.task_id = $1
            "#,
            task_uuid
        )
        .fetch_optional(&self.db)
        .await;

        match result {
            Ok(Some(row)) => Some(TaskInfo {
                task_id: row.task_id.to_string(),
                task_type: row.task_type,
                status: parse_task_status(&row.status),
                assigned_nodes: row.assigned_nodes,
                created_at: row.created_at.to_rfc3339(),
                updated_at: row.updated_at.to_rfc3339(),
                result: row.result,
                proof_id: row.proof_id,
            }),
            Ok(None) => None,
            Err(e) => {
                tracing::error!("Failed to get task {}: {:?}", task_id, e);
                None
            }
        }
    }

    /// List all tasks from the database
    pub async fn list_tasks(&self) -> Vec<TaskInfo> {
        let result = sqlx::query!(
            r#"
            SELECT 
                t.task_id, t.task_type, t.status, t.result, t.proof_id,
                t.created_at, t.updated_at,
                COALESCE(
                    (
                        SELECT ARRAY_AGG(ta.node_id)
                        FROM task_assignments ta
                        WHERE ta.task_id = t.task_id
                    ),
                    ARRAY[]::VARCHAR[]
                ) as "assigned_nodes!"
            FROM tasks t
            ORDER BY t.created_at DESC
            "#
        )
        .fetch_all(&self.db)
        .await;

        match result {
            Ok(rows) => rows
                .into_iter()
                .map(|row| TaskInfo {
                    task_id: row.task_id.to_string(),
                    task_type: row.task_type,
                    status: parse_task_status(&row.status),
                    assigned_nodes: row.assigned_nodes,
                    created_at: row.created_at.to_rfc3339(),
                    updated_at: row.updated_at.to_rfc3339(),
                    result: row.result,
                    proof_id: row.proof_id,
                })
                .collect(),
            Err(e) => {
                tracing::error!("Failed to list tasks: {:?}", e);
                vec![]
            }
        }
    }

    /// Verify a ZK proof
    pub async fn verify_proof(
        &self,
        request: ProofVerificationRequest,
    ) -> ApiResult<ProofVerificationResponse> {
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

    /// Get cluster statistics from the database
    pub async fn get_cluster_stats(&self) -> ClusterStats {
        // Get node statistics
        let node_stats = sqlx::query!(
            r#"
            SELECT 
                COUNT(*) as total_nodes,
                COUNT(*) FILTER (WHERE status = 'online' AND health_score >= 70.0) as healthy_nodes,
                COALESCE(AVG(health_score), 0.0) as avg_health_score,
                COALESCE(SUM(cpu_cores * memory_gb), 0.0) as total_compute_capacity
            FROM nodes
            "#
        )
        .fetch_one(&self.db)
        .await;

        // Get task statistics
        let task_stats = sqlx::query!(
            r#"
            SELECT 
                COUNT(*) as total_tasks,
                COUNT(*) FILTER (WHERE status = 'completed') as completed_tasks,
                COUNT(*) FILTER (WHERE status = 'failed') as failed_tasks
            FROM tasks
            "#
        )
        .fetch_one(&self.db)
        .await;

        match (node_stats, task_stats) {
            (Ok(nodes), Ok(tasks)) => ClusterStats {
                total_nodes: nodes.total_nodes.unwrap_or(0) as usize,
                healthy_nodes: nodes.healthy_nodes.unwrap_or(0) as usize,
                total_tasks: tasks.total_tasks.unwrap_or(0) as usize,
                completed_tasks: tasks.completed_tasks.unwrap_or(0) as usize,
                failed_tasks: tasks.failed_tasks.unwrap_or(0) as usize,
                avg_health_score: nodes.avg_health_score.unwrap_or(0.0),
                total_compute_capacity: nodes.total_compute_capacity.unwrap_or(0.0),
            },
            _ => {
                tracing::error!("Failed to get cluster stats");
                ClusterStats {
                    total_nodes: 0,
                    healthy_nodes: 0,
                    total_tasks: 0,
                    completed_tasks: 0,
                    failed_tasks: 0,
                    avg_health_score: 0.0,
                    total_compute_capacity: 0.0,
                }
            }
        }
    }
}

/// Helper function to parse task status from string
fn parse_task_status(status: &str) -> TaskStatus {
    match status.to_lowercase().as_str() {
        "pending" => TaskStatus::Pending,
        "running" => TaskStatus::Running,
        "completed" => TaskStatus::Completed,
        "failed" => TaskStatus::Failed,
        _ => TaskStatus::Pending,
    }
}
