/// Application state with PostgreSQL persistence
///
/// This module provides CRUD operations for nodes and tasks using a PostgreSQL database.
use crate::error::ApiResult;
use crate::models::*;
use sqlx::{PgPool, Row};
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

    /// Register a new node in the database with owner tracking
    pub async fn register_node(
        &self,
        registration: NodeRegistration,
        owner_id: Uuid,
    ) -> ApiResult<NodeInfo> {
        let now = chrono::Utc::now();

        // Insert node into database with owner_id
        sqlx::query(
            r#"
            INSERT INTO nodes (
                node_id, region, node_type, bandwidth_mbps, cpu_cores, 
                memory_gb, gpu_available, health_score, status, 
                registered_at, last_seen, owner_id, last_heartbeat
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
        )
        .bind(&registration.node_id)
        .bind(&registration.region)
        .bind(&registration.node_type)
        .bind(registration.capabilities.bandwidth_mbps)
        .bind(registration.capabilities.cpu_cores as i32)
        .bind(registration.capabilities.memory_gb)
        .bind(registration.capabilities.gpu_available)
        .bind(100.0_f64)
        .bind("online")
        .bind(now)
        .bind(now)
        .bind(owner_id)
        .bind(now)
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

    /// List all nodes from the database (excludes soft-deleted nodes)
    pub async fn list_nodes(&self) -> Vec<NodeInfo> {
        let result = sqlx::query(
            r#"
            SELECT 
                node_id, region, node_type, bandwidth_mbps, cpu_cores,
                memory_gb, gpu_available, health_score, status,
                registered_at, last_seen
            FROM nodes
            WHERE deleted_at IS NULL
            ORDER BY registered_at DESC
            "#,
        )
        .fetch_all(&self.db)
        .await;

        match result {
            Ok(rows) => rows
                .into_iter()
                .map(|row| NodeInfo {
                    node_id: row.get("node_id"),
                    region: row.get("region"),
                    node_type: row.get("node_type"),
                    capabilities: NodeCapabilities {
                        bandwidth_mbps: row.get("bandwidth_mbps"),
                        cpu_cores: row.get::<i32, _>("cpu_cores") as u32,
                        memory_gb: row.get("memory_gb"),
                        gpu_available: row.get("gpu_available"),
                    },
                    health_score: row.get("health_score"),
                    status: row.get("status"),
                    registered_at: row
                        .get::<chrono::DateTime<chrono::Utc>, _>("registered_at")
                        .to_rfc3339(),
                    last_seen: row
                        .get::<chrono::DateTime<chrono::Utc>, _>("last_seen")
                        .to_rfc3339(),
                })
                .collect(),
            Err(e) => {
                tracing::error!("Failed to list nodes: {:?}", e);
                vec![]
            }
        }
    }

    /// Get a specific node from the database (excludes soft-deleted nodes)
    pub async fn get_node(&self, node_id: &str) -> Option<NodeInfo> {
        let result = sqlx::query(
            r#"
            SELECT 
                node_id, region, node_type, bandwidth_mbps, cpu_cores,
                memory_gb, gpu_available, health_score, status,
                registered_at, last_seen
            FROM nodes
            WHERE node_id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(node_id)
        .fetch_optional(&self.db)
        .await;

        match result {
            Ok(Some(row)) => Some(NodeInfo {
                node_id: row.get("node_id"),
                region: row.get("region"),
                node_type: row.get("node_type"),
                capabilities: NodeCapabilities {
                    bandwidth_mbps: row.get("bandwidth_mbps"),
                    cpu_cores: row.get::<i32, _>("cpu_cores") as u32,
                    memory_gb: row.get("memory_gb"),
                    gpu_available: row.get("gpu_available"),
                },
                health_score: row.get("health_score"),
                status: row.get("status"),
                registered_at: row
                    .get::<chrono::DateTime<chrono::Utc>, _>("registered_at")
                    .to_rfc3339(),
                last_seen: row
                    .get::<chrono::DateTime<chrono::Utc>, _>("last_seen")
                    .to_rfc3339(),
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
        sqlx::query(
            r#"
            INSERT INTO tasks (
                task_id, task_type, status, wasm_module, inputs,
                min_nodes, max_execution_time_sec, require_gpu, require_proof
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(task_id)
        .bind(&task.task_type)
        .bind("pending")
        .bind(task.wasm_module.as_deref())
        .bind(&task.inputs)
        .bind(task.requirements.min_nodes as i32)
        .bind(task.requirements.max_execution_time_sec as i64)
        .bind(task.requirements.require_gpu)
        .bind(task.requirements.require_proof)
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

        let result = sqlx::query(
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
                ) as assigned_nodes
            FROM tasks t
            WHERE t.task_id = $1
            "#,
        )
        .bind(task_uuid)
        .fetch_optional(&self.db)
        .await;

        match result {
            Ok(Some(row)) => Some(TaskInfo {
                task_id: row.get::<Uuid, _>("task_id").to_string(),
                task_type: row.get("task_type"),
                status: parse_task_status(&row.get::<String, _>("status")),
                assigned_nodes: row.get::<Vec<String>, _>("assigned_nodes"),
                created_at: row
                    .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                    .to_rfc3339(),
                updated_at: row
                    .get::<chrono::DateTime<chrono::Utc>, _>("updated_at")
                    .to_rfc3339(),
                result: row.try_get("result").ok(),
                proof_id: row.try_get("proof_id").ok(),
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
        let result = sqlx::query(
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
                ) as assigned_nodes
            FROM tasks t
            ORDER BY t.created_at DESC
            "#,
        )
        .fetch_all(&self.db)
        .await;

        match result {
            Ok(rows) => rows
                .into_iter()
                .map(|row| TaskInfo {
                    task_id: row.get::<Uuid, _>("task_id").to_string(),
                    task_type: row.get("task_type"),
                    status: parse_task_status(&row.get::<String, _>("status")),
                    assigned_nodes: row.get::<Vec<String>, _>("assigned_nodes"),
                    created_at: row
                        .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                        .to_rfc3339(),
                    updated_at: row
                        .get::<chrono::DateTime<chrono::Utc>, _>("updated_at")
                        .to_rfc3339(),
                    result: row.try_get("result").ok(),
                    proof_id: row.try_get("proof_id").ok(),
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
        let node_stats = sqlx::query(
            r#"
            SELECT 
                COUNT(*) as total_nodes,
                COUNT(*) FILTER (WHERE status = 'online' AND health_score >= 70.0) as healthy_nodes,
                COALESCE(AVG(health_score), 0.0) as avg_health_score,
                COALESCE(SUM(cpu_cores * memory_gb), 0.0) as total_compute_capacity
            FROM nodes
            "#,
        )
        .fetch_one(&self.db)
        .await;

        // Get task statistics
        let task_stats = sqlx::query(
            r#"
            SELECT 
                COUNT(*) as total_tasks,
                COUNT(*) FILTER (WHERE status = 'completed') as completed_tasks,
                COUNT(*) FILTER (WHERE status = 'failed') as failed_tasks
            FROM tasks
            "#,
        )
        .fetch_one(&self.db)
        .await;

        match (node_stats, task_stats) {
            (Ok(nodes), Ok(tasks)) => ClusterStats {
                total_nodes: nodes.get::<i64, _>("total_nodes") as usize,
                healthy_nodes: nodes.get::<i64, _>("healthy_nodes") as usize,
                total_tasks: tasks.get::<i64, _>("total_tasks") as usize,
                completed_tasks: tasks.get::<i64, _>("completed_tasks") as usize,
                failed_tasks: tasks.get::<i64, _>("failed_tasks") as usize,
                avg_health_score: nodes.get("avg_health_score"),
                total_compute_capacity: nodes.get("total_compute_capacity"),
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

    /// Check if a user owns a specific node
    pub async fn check_node_ownership(&self, node_id: &str, user_id: Uuid) -> ApiResult<bool> {
        let result = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM nodes
            WHERE node_id = $1 AND owner_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(node_id)
        .bind(user_id)
        .fetch_one(&self.db)
        .await?;

        Ok(result > 0)
    }

    /// Soft delete a node (sets deleted_at timestamp)
    pub async fn delete_node(&self, node_id: &str, owner_id: Uuid) -> ApiResult<bool> {
        // Verify ownership
        if !self.check_node_ownership(node_id, owner_id).await? {
            return Ok(false);
        }

        let now = chrono::Utc::now();
        let result = sqlx::query(
            r#"
            UPDATE nodes
            SET deleted_at = $1, status = 'offline', updated_at = $1
            WHERE node_id = $2 AND owner_id = $3 AND deleted_at IS NULL
            "#,
        )
        .bind(now)
        .bind(node_id)
        .bind(owner_id)
        .execute(&self.db)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Update node heartbeat timestamp
    pub async fn update_node_heartbeat(&self, node_id: &str, owner_id: Uuid) -> ApiResult<bool> {
        // Verify ownership
        if !self.check_node_ownership(node_id, owner_id).await? {
            return Ok(false);
        }

        let now = chrono::Utc::now();
        let result = sqlx::query(
            r#"
            UPDATE nodes
            SET last_heartbeat = $1, last_seen = $1, updated_at = $1
            WHERE node_id = $2 AND owner_id = $3 AND deleted_at IS NULL
            "#,
        )
        .bind(now)
        .bind(node_id)
        .bind(owner_id)
        .execute(&self.db)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// List nodes owned by a specific user
    pub async fn list_user_nodes(&self, owner_id: Uuid) -> Vec<NodeInfo> {
        let result = sqlx::query(
            r#"
            SELECT 
                node_id, region, node_type, bandwidth_mbps, cpu_cores,
                memory_gb, gpu_available, health_score, status,
                registered_at, last_seen
            FROM nodes
            WHERE owner_id = $1 AND deleted_at IS NULL
            ORDER BY registered_at DESC
            "#,
        )
        .bind(owner_id)
        .fetch_all(&self.db)
        .await;

        match result {
            Ok(rows) => rows
                .into_iter()
                .map(|row| NodeInfo {
                    node_id: row.get("node_id"),
                    region: row.get("region"),
                    node_type: row.get("node_type"),
                    capabilities: NodeCapabilities {
                        bandwidth_mbps: row.get("bandwidth_mbps"),
                        cpu_cores: row.get::<i32, _>("cpu_cores") as u32,
                        memory_gb: row.get("memory_gb"),
                        gpu_available: row.get("gpu_available"),
                    },
                    health_score: row.get("health_score"),
                    status: row.get("status"),
                    registered_at: row
                        .get::<chrono::DateTime<chrono::Utc>, _>("registered_at")
                        .to_rfc3339(),
                    last_seen: row
                        .get::<chrono::DateTime<chrono::Utc>, _>("last_seen")
                        .to_rfc3339(),
                })
                .collect(),
            Err(e) => {
                tracing::error!("Failed to list user nodes: {:?}", e);
                vec![]
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
