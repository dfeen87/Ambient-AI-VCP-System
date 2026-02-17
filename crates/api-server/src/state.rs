/// Application state with PostgreSQL persistence
///
/// This module provides CRUD operations for nodes and tasks using a PostgreSQL database.
use crate::error::{ApiError, ApiResult};
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

        // Attempt to attach the newly registered node to pending tasks that still
        // need additional workers.
        self.assign_pending_tasks_for_node(&registration.node_id)
            .await?;

        // Return the created node
        let node_info = NodeInfo {
            node_id: registration.node_id,
            region: registration.region,
            node_type: registration.node_type,
            capabilities: registration.capabilities,
            health_score: 100.0,
            status: "online".to_string(),
            owner_id: owner_id.to_string(),
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
                node_id, region, node_type, owner_id, bandwidth_mbps, cpu_cores,
                memory_gb, gpu_available, health_score, status,
                registered_at, last_seen
            FROM nodes
            WHERE deleted_at IS NULL
              AND status != 'rejected'
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
                    owner_id: row.get::<Uuid, _>("owner_id").to_string(),
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
                node_id, region, node_type, owner_id, bandwidth_mbps, cpu_cores,
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
                owner_id: row.get::<Uuid, _>("owner_id").to_string(),
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
    pub async fn submit_task(&self, task: TaskSubmission, creator_id: Uuid) -> ApiResult<TaskInfo> {
        let task_id = Uuid::new_v4();
        let now = chrono::Utc::now();
        let task_type = task.task_type.clone();
        let task_inputs = task.inputs.clone();

        let task_registry_entry = task_type_registry_entry(&task.task_type)
            .ok_or_else(|| crate::error::ApiError::bad_request("Unsupported task_type"))?;

        // Insert task into database
        sqlx::query(
            r#"
            INSERT INTO tasks (
                task_id, task_type, status, wasm_module, inputs,
                min_nodes, max_execution_time_sec, require_gpu, require_proof, creator_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
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
        .bind(creator_id)
        .execute(&self.db)
        .await?;

        self.assign_available_nodes_for_task(
            task_id,
            task_registry_entry,
            task.requirements.min_nodes,
            task.requirements.require_gpu,
        )
        .await?;

        let task_status = self.get_task_status(task_id).await?;

        let (status, result) = if task_status == "running" {
            let result = analyze_task_payload(&task_type, &task_inputs);

            let mut tx = self.db.begin().await?;

            sqlx::query(
                r#"
                UPDATE tasks
                SET status = 'completed', result = $1, updated_at = NOW()
                WHERE task_id = $2
                "#,
            )
            .bind(&result)
            .bind(task_id)
            .execute(&mut *tx)
            .await?;

            self.disconnect_task_assignments(task_id, &mut tx).await?;

            tx.commit().await?;

            (TaskStatus::Completed, Some(result))
        } else {
            (parse_task_status(&task_status), None)
        };

        let assigned_nodes = self.get_assigned_nodes(task_id).await?;

        let task_info = TaskInfo {
            task_id: task_id.to_string(),
            task_type,
            status,
            assigned_nodes,
            created_at: now.to_rfc3339(),
            updated_at: now.to_rfc3339(),
            result,
            proof_id: None,
        };

        Ok(task_info)
    }

    async fn get_assigned_nodes(&self, task_id: Uuid) -> ApiResult<Vec<String>> {
        let assigned_nodes = sqlx::query_scalar::<_, String>(
            r#"
            SELECT node_id
            FROM task_assignments
            WHERE task_id = $1
              AND disconnected_at IS NULL
            ORDER BY node_id ASC
            "#,
        )
        .bind(task_id)
        .fetch_all(&self.db)
        .await?;

        Ok(assigned_nodes)
    }

    async fn get_task_status(&self, task_id: Uuid) -> ApiResult<String> {
        let status = sqlx::query_scalar::<_, String>(
            r#"
            SELECT status
            FROM tasks
            WHERE task_id = $1
            "#,
        )
        .bind(task_id)
        .fetch_one(&self.db)
        .await?;

        Ok(status)
    }

    async fn disconnect_task_assignments(
        &self,
        task_id: Uuid,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> ApiResult<()> {
        sqlx::query(
            r#"
            UPDATE task_assignments
            SET disconnected_at = NOW()
            WHERE task_id = $1
              AND disconnected_at IS NULL
            "#,
        )
        .bind(task_id)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn assign_available_nodes_for_task(
        &self,
        task_id: Uuid,
        task_registry_entry: &TaskTypeRegistryEntry,
        min_nodes: u32,
        require_gpu: bool,
    ) -> ApiResult<()> {
        let node_ids = sqlx::query_scalar::<_, String>(
            r#"
            SELECT node_id
            FROM nodes
            WHERE deleted_at IS NULL
              AND status = 'online'
              AND (node_type = $1 OR node_type = 'any')
              AND cpu_cores >= $2
              AND memory_gb >= $3
              AND bandwidth_mbps >= $4
              AND ($5 = FALSE OR gpu_available = TRUE)
            ORDER BY registered_at ASC
            "#,
        )
        .bind(task_registry_entry.preferred_node_type)
        .bind(task_registry_entry.minimum_capabilities.cpu_cores as i32)
        .bind(task_registry_entry.minimum_capabilities.memory_gb)
        .bind(task_registry_entry.minimum_capabilities.bandwidth_mbps)
        .bind(require_gpu || task_registry_entry.minimum_capabilities.gpu_available)
        .fetch_all(&self.db)
        .await?;

        for node_id in node_ids {
            sqlx::query(
                r#"
                INSERT INTO task_assignments (task_id, node_id)
                VALUES ($1, $2)
                ON CONFLICT (task_id, node_id)
                DO UPDATE SET assigned_at = NOW(), disconnected_at = NULL
                "#,
            )
            .bind(task_id)
            .bind(node_id)
            .execute(&self.db)
            .await?;
        }

        self.update_task_status_from_assignments(task_id, min_nodes)
            .await
    }

    async fn assign_pending_tasks_for_node(&self, node_id: &str) -> ApiResult<()> {
        let pending_tasks = sqlx::query(
            r#"
            SELECT
                t.task_id,
                t.task_type,
                t.min_nodes,
                t.require_gpu,
                COALESCE(COUNT(ta.node_id), 0) AS assigned_nodes
            FROM tasks t
            LEFT JOIN task_assignments ta ON ta.task_id = t.task_id AND ta.disconnected_at IS NULL
            WHERE t.status = 'pending'
            GROUP BY t.task_id, t.task_type, t.min_nodes, t.require_gpu
            HAVING COALESCE(COUNT(ta.node_id), 0) < t.min_nodes
            ORDER BY t.created_at ASC
            "#,
        )
        .fetch_all(&self.db)
        .await?;

        for task in pending_tasks {
            let task_id: Uuid = task.get("task_id");
            let task_type: String = task.get("task_type");
            let min_nodes: i32 = task.get("min_nodes");
            let require_gpu: bool = task.get("require_gpu");

            let Some(task_registry_entry) = task_type_registry_entry(&task_type) else {
                continue;
            };

            let node_is_eligible = sqlx::query_scalar::<_, bool>(
                r#"
                SELECT EXISTS (
                    SELECT 1
                    FROM nodes n
                    WHERE n.node_id = $1
                      AND n.deleted_at IS NULL
                      AND n.status = 'online'
                      AND (n.node_type = $2 OR n.node_type = 'any')
                      AND n.cpu_cores >= $3
                      AND n.memory_gb >= $4
                      AND n.bandwidth_mbps >= $5
                      AND ($6 = FALSE OR n.gpu_available = TRUE)
                )
                "#,
            )
            .bind(node_id)
            .bind(task_registry_entry.preferred_node_type)
            .bind(task_registry_entry.minimum_capabilities.cpu_cores as i32)
            .bind(task_registry_entry.minimum_capabilities.memory_gb)
            .bind(task_registry_entry.minimum_capabilities.bandwidth_mbps)
            .bind(require_gpu || task_registry_entry.minimum_capabilities.gpu_available)
            .fetch_one(&self.db)
            .await?;

            if !node_is_eligible {
                continue;
            }

            sqlx::query(
                r#"
                INSERT INTO task_assignments (task_id, node_id)
                VALUES ($1, $2)
                ON CONFLICT (task_id, node_id)
                DO UPDATE SET assigned_at = NOW(), disconnected_at = NULL
                "#,
            )
            .bind(task_id)
            .bind(node_id)
            .execute(&self.db)
            .await?;

            self.update_task_status_from_assignments(task_id, min_nodes as u32)
                .await?;
        }

        Ok(())
    }

    async fn update_task_status_from_assignments(
        &self,
        task_id: Uuid,
        min_nodes: u32,
    ) -> ApiResult<()> {
        let assigned_nodes: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM task_assignments
            WHERE task_id = $1
              AND disconnected_at IS NULL
            "#,
        )
        .bind(task_id)
        .fetch_one(&self.db)
        .await?;

        let next_status = if assigned_nodes >= min_nodes as i64 {
            "running"
        } else {
            "pending"
        };

        sqlx::query(
            r#"
            UPDATE tasks
            SET status = $1, updated_at = NOW()
            WHERE task_id = $2
            "#,
        )
        .bind(next_status)
        .bind(task_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Get a specific task from the database
    pub async fn get_task(&self, task_id: &str, requester_id: Uuid) -> Option<TaskInfo> {
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
                          AND ta.disconnected_at IS NULL
                    ),
                    ARRAY[]::VARCHAR[]
                ) as assigned_nodes
            FROM tasks t
            WHERE t.task_id = $1
              AND t.creator_id = $2
            "#,
        )
        .bind(task_uuid)
        .bind(requester_id)
        .fetch_optional(&self.db)
        .await;

        match result {
            Ok(Some(row)) => {
                let task_id_uuid: Uuid = row.get("task_id");
                let status_text: String = row.get("status");
                if let Err(e) = self
                    .notify_if_task_completed(task_id_uuid, &status_text)
                    .await
                {
                    tracing::warn!("Failed to send completion email: {:?}", e);
                }

                Some(TaskInfo {
                    task_id: task_id_uuid.to_string(),
                    task_type: row.get("task_type"),
                    status: parse_task_status(&status_text),
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
            }
            Ok(None) => None,
            Err(e) => {
                tracing::error!("Failed to get task {}: {:?}", task_id, e);
                None
            }
        }
    }

    /// List all tasks from the database
    pub async fn list_tasks(&self, requester_id: Uuid) -> Vec<TaskInfo> {
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
                          AND ta.disconnected_at IS NULL
                    ),
                    ARRAY[]::VARCHAR[]
                ) as assigned_nodes
            FROM tasks t
            WHERE t.creator_id = $1
            ORDER BY t.created_at DESC
            "#,
        )
        .bind(requester_id)
        .fetch_all(&self.db)
        .await;

        let tasks: Vec<TaskInfo> = match result {
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
        };

        for task in &tasks {
            if task.status == TaskStatus::Completed {
                if let Ok(task_uuid) = Uuid::parse_str(&task.task_id) {
                    if let Err(e) = self.notify_if_task_completed(task_uuid, "completed").await {
                        tracing::warn!("Failed to send completion email: {:?}", e);
                    }
                }
            }
        }

        tasks
    }

    async fn notify_if_task_completed(&self, task_id: Uuid, status: &str) -> ApiResult<()> {
        if status != "completed" {
            return Ok(());
        }

        let row = sqlx::query(
            r#"
            SELECT t.result, u.email
            FROM tasks t
            LEFT JOIN users u ON u.user_id = t.creator_id
            WHERE t.task_id = $1
              AND t.status = 'completed'
              AND t.completion_email_sent_at IS NULL
            "#,
        )
        .bind(task_id)
        .fetch_optional(&self.db)
        .await?;

        let Some(row) = row else {
            return Ok(());
        };

        let email = row.try_get::<Option<String>, _>("email").ok().flatten();
        let Some(email) = email.filter(|v| !v.trim().is_empty()) else {
            return Ok(());
        };

        let result_payload = row
            .try_get::<Option<serde_json::Value>, _>("result")
            .ok()
            .flatten()
            .unwrap_or(serde_json::json!({"message": "Task completed"}));

        self.send_completion_email(&email, task_id, &result_payload)
            .await?;

        sqlx::query(
            r#"
            UPDATE tasks
            SET completion_email_sent_at = NOW()
            WHERE task_id = $1
            "#,
        )
        .bind(task_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    async fn send_completion_email(
        &self,
        recipient: &str,
        task_id: Uuid,
        result: &serde_json::Value,
    ) -> ApiResult<()> {
        use tokio::io::AsyncWriteExt;
        use tokio::process::Command;

        let sender = std::env::var("EMAIL_FROM")
            .map_err(|_| ApiError::internal_error("EMAIL_FROM not configured"))?;
        let subject = format!("Task Completed: {task_id}");
        let body = format!(
            "Your task {task_id} has completed.

Result:
{}",
            serde_json::to_string_pretty(result)
                .unwrap_or_else(|_| "<failed to serialize task result>".to_string())
        );

        let message = format!(
            "From: {sender}
To: {recipient}
Subject: {subject}
Content-Type: text/plain; charset=\"utf-8\"

{body}
"
        );

        let mut child = Command::new("sendmail")
            .arg("-t")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|_| ApiError::internal_error("sendmail command is not available"))?;

        if let Some(stdin) = child.stdin.as_mut() {
            stdin
                .write_all(message.as_bytes())
                .await
                .map_err(|_| ApiError::internal_error("Failed writing message to sendmail"))?;
        }

        let output = child
            .wait_with_output()
            .await
            .map_err(|_| ApiError::internal_error("Failed waiting for sendmail completion"))?;

        if !output.status.success() {
            return Err(ApiError::internal_error(format!(
                "sendmail failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(())
    }

    /// Verify a ZK proof using actual cryptographic verification
    pub async fn verify_proof(
        &self,
        request: ProofVerificationRequest,
    ) -> ApiResult<ProofVerificationResponse> {
        use std::time::Instant;
        use zk_prover::{ZKProof, ZKVerifier};

        // Validate the request
        request.validate()?;

        let start = Instant::now();

        // Decode proof data and public inputs
        let proof_data = request.decode_proof_data()?;
        let public_inputs_data = request.decode_public_inputs()?;

        // Create ZK proof object
        let circuit_id = request.circuit_id.unwrap_or_else(|| "default".to_string());
        let proof = ZKProof::new(proof_data, public_inputs_data.clone(), circuit_id);

        // Verify proof size constraints
        if proof.size() > 75_000 {
            // 75KB max decoded proof size
            return Ok(ProofVerificationResponse {
                valid: false,
                task_id: request.task_id,
                verified_at: chrono::Utc::now().to_rfc3339(),
                verification_time_ms: start.elapsed().as_millis() as u64,
                error_message: Some("Proof size exceeds maximum allowed size".to_string()),
            });
        }

        // Initialize verifier with default verification key
        // In production, you would load the appropriate verification key based on circuit_id
        let verifier = ZKVerifier::default();

        // Perform cryptographic verification off the async runtime worker.
        let public_inputs_for_verify = public_inputs_data.clone();
        let valid = tokio::task::spawn_blocking(move || {
            verifier.verify_proof(&proof, &public_inputs_for_verify)
        })
        .await
        .map_err(|_| crate::error::ApiError::internal_error("Proof verification task failed"))?;

        let verification_time_ms = start.elapsed().as_millis() as u64;

        Ok(ProofVerificationResponse {
            valid,
            task_id: request.task_id,
            verified_at: chrono::Utc::now().to_rfc3339(),
            verification_time_ms,
            error_message: if !valid {
                Some("Proof verification failed: invalid proof or public inputs".to_string())
            } else {
                None
            },
        })
    }

    /// Get cluster statistics from the database
    pub async fn get_cluster_stats(&self) -> ClusterStats {
        // Get node statistics
        let node_stats = sqlx::query(
            r#"
            SELECT 
                COUNT(*) FILTER (WHERE status != 'rejected') as total_nodes,
                COUNT(*) FILTER (WHERE status = 'online' AND health_score >= 70.0) as healthy_nodes,
                COALESCE(AVG(health_score) FILTER (WHERE status != 'rejected'), 0.0) as avg_health_score,
                COALESCE(SUM(cpu_cores * memory_gb) FILTER (WHERE status != 'rejected'), 0.0) as total_compute_capacity
            FROM nodes
            WHERE deleted_at IS NULL
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

    /// Reject a node owned by the requesting user
    pub async fn reject_node(&self, node_id: &str, owner_id: Uuid) -> ApiResult<bool> {
        // Verify ownership
        if !self.check_node_ownership(node_id, owner_id).await? {
            return Ok(false);
        }

        let now = chrono::Utc::now();
        let result = sqlx::query(
            r#"
            UPDATE nodes
            SET status = 'rejected', updated_at = $1
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
                node_id, region, node_type, owner_id, bandwidth_mbps, cpu_cores,
                memory_gb, gpu_available, health_score, status,
                registered_at, last_seen
            FROM nodes
            WHERE owner_id = $1 AND deleted_at IS NULL
              AND status != 'rejected'
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
                    owner_id: row.get::<Uuid, _>("owner_id").to_string(),
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

fn analyze_task_payload(task_type: &str, inputs: &serde_json::Value) -> serde_json::Value {
    match inputs {
        serde_json::Value::Object(map) => {
            if task_type == "computation" {
                if let Some(expression) = map.get("expression").and_then(|v| v.as_str()) {
                    if let Some(result) = evaluate_arithmetic_expression(expression) {
                        return serde_json::json!({
                            "task_type": task_type,
                            "analysis_mode": "computation",
                            "summary": "Arithmetic expression evaluated successfully.",
                            "expression": expression,
                            "result": result
                        });
                    }
                }
            }

            if let Some(prompt) = map.get("prompt").and_then(|v| v.as_str()) {
                serde_json::json!({
                    "task_type": task_type,
                    "analysis_mode": "plain_text",
                    "summary": summarize_prompt(prompt),
                    "input_characters": prompt.chars().count(),
                    "keyword_signals": extract_keywords(prompt),
                    "recommendation": "Use JSON object inputs for richer structured outputs when possible."
                })
            } else {
                serde_json::json!({
                    "task_type": task_type,
                    "analysis_mode": "json",
                    "summary": "Structured JSON payload analyzed successfully.",
                    "top_level_keys": map.keys().cloned().collect::<Vec<String>>(),
                    "object_size": map.len(),
                    "recommendation": "Include an `expression` field for arithmetic tasks or a `prompt` field for natural-language summarization."
                })
            }
        }
        serde_json::Value::Array(values) => serde_json::json!({
            "task_type": task_type,
            "analysis_mode": "json_array",
            "summary": "Array payload analyzed successfully.",
            "item_count": values.len(),
            "preview": values.iter().take(3).cloned().collect::<Vec<serde_json::Value>>()
        }),
        serde_json::Value::String(value) => serde_json::json!({
            "task_type": task_type,
            "analysis_mode": "string",
            "summary": summarize_prompt(value),
            "input_characters": value.chars().count(),
            "keyword_signals": extract_keywords(value)
        }),
        primitive => serde_json::json!({
            "task_type": task_type,
            "analysis_mode": "primitive",
            "summary": "Primitive payload analyzed successfully.",
            "value": primitive
        }),
    }
}

fn evaluate_arithmetic_expression(expression: &str) -> Option<f64> {
    let normalized = expression
        .trim()
        .trim_end_matches(|c: char| c.is_ascii_punctuation() && c != '.')
        .replace(['X', 'x'], "*")
        .replace('÷', "/");

    let mut numbers = Vec::new();
    let mut operators = Vec::new();
    let mut current = String::new();

    for ch in normalized.chars() {
        if ch.is_ascii_digit() || ch == '.' {
            current.push(ch);
            continue;
        }

        if matches!(ch, '+' | '-' | '*' | '/') {
            if current.is_empty() {
                return None;
            }
            numbers.push(current.parse::<f64>().ok()?);
            current.clear();
            operators.push(ch);
            continue;
        }

        if !ch.is_whitespace() {
            return None;
        }
    }

    if current.is_empty() {
        return None;
    }
    numbers.push(current.parse::<f64>().ok()?);

    if numbers.len() != operators.len() + 1 {
        return None;
    }

    let mut folded_numbers = vec![numbers[0]];
    let mut folded_operators = Vec::new();

    for (idx, operator) in operators.iter().enumerate() {
        let rhs = numbers[idx + 1];
        if *operator == '*' {
            let last = folded_numbers.pop()?;
            folded_numbers.push(last * rhs);
        } else if *operator == '/' {
            if rhs == 0.0 {
                return None;
            }
            let last = folded_numbers.pop()?;
            folded_numbers.push(last / rhs);
        } else {
            folded_operators.push(*operator);
            folded_numbers.push(rhs);
        }
    }

    let mut result = folded_numbers[0];
    for (idx, operator) in folded_operators.iter().enumerate() {
        let rhs = folded_numbers[idx + 1];
        if *operator == '+' {
            result += rhs;
        } else {
            result -= rhs;
        }
    }

    Some(result)
}

fn summarize_prompt(prompt: &str) -> String {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return "No text content provided for analysis.".to_string();
    }

    let words: Vec<&str> = trimmed.split_whitespace().collect();
    let preview = words
        .iter()
        .take(18)
        .copied()
        .collect::<Vec<&str>>()
        .join(" ");

    if words.len() > 18 {
        format!("{}... ({} words total)", preview, words.len())
    } else {
        format!("{} ({} words total)", preview, words.len())
    }
}

fn extract_keywords(prompt: &str) -> Vec<String> {
    use std::collections::BTreeSet;

    let mut unique = BTreeSet::new();
    for raw in prompt.split(|c: char| !c.is_alphanumeric()) {
        let token = raw.to_lowercase();
        if token.len() >= 4 {
            unique.insert(token);
        }
    }

    unique.into_iter().take(8).collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyzes_plain_text_prompt_payload() {
        let value =
            serde_json::json!({"prompt": "Summarize this medical timeline and flag anomalies"});
        let result = analyze_task_payload("computation", &value);

        assert_eq!(result["analysis_mode"], "plain_text");
        assert!(result["summary"]
            .as_str()
            .unwrap_or_default()
            .contains("words total"));
    }

    #[test]
    fn analyzes_structured_json_payload() {
        let value = serde_json::json!({"job": "quick-compute", "priority": "high"});
        let result = analyze_task_payload("computation", &value);

        assert_eq!(result["analysis_mode"], "json");
        assert_eq!(result["object_size"], 2);
    }

    #[test]
    fn evaluates_computation_expression_payload() {
        let value = serde_json::json!({"expression": "10x10?", "operation": "multiply"});
        let result = analyze_task_payload("computation", &value);

        assert_eq!(result["analysis_mode"], "computation");
        assert_eq!(result["result"], 100.0);
    }

    #[test]
    fn returns_json_analysis_for_invalid_expression_payload() {
        let value = serde_json::json!({"expression": "10 apples", "operation": "multiply"});
        let result = analyze_task_payload("computation", &value);

        assert_eq!(result["analysis_mode"], "json");
    }
}
