/// Application state with PostgreSQL persistence
///
/// This module provides CRUD operations for nodes and tasks using a PostgreSQL database.
use crate::error::{ApiError, ApiResult};
use crate::models::*;
use federated_learning::{FederatedAggregator, LayerWeights, ModelWeights, PrivacyBudget};
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Application state with database connection pool
pub struct AppState {
    /// PostgreSQL connection pool
    pub db: PgPool,
}

impl AppState {
    fn parse_max_concurrent_tasks_per_node(value: Option<&str>) -> i64 {
        value
            .and_then(|raw| raw.parse::<i64>().ok())
            .filter(|parsed| *parsed > 0)
            .unwrap_or(50)
    }

    fn parse_max_active_task_attachments_per_node(value: Option<&str>) -> i64 {
        Self::parse_max_concurrent_tasks_per_node(value)
    }

    fn max_active_task_attachments_per_node() -> i64 {
        let configured = std::env::var("MAX_CONCURRENT_TASKS_PER_NODE")
            .ok()
            .or_else(|| std::env::var("MAX_ACTIVE_TASK_ATTACHMENTS_PER_NODE").ok());

        Self::parse_max_active_task_attachments_per_node(configured.as_deref())
    }

    fn parse_node_heartbeat_stale_timeout_seconds(value: Option<&str>) -> i64 {
        value
            .and_then(|raw| raw.parse::<i64>().ok())
            .filter(|parsed| *parsed > 0)
            .unwrap_or(90)
    }

    fn node_heartbeat_stale_timeout_seconds() -> i64 {
        let configured = std::env::var("NODE_HEARTBEAT_STALE_TIMEOUT_SECONDS").ok();
        Self::parse_node_heartbeat_stale_timeout_seconds(configured.as_deref())
    }

    fn parse_connect_session_monitor_interval_seconds(value: Option<&str>) -> u64 {
        value
            .and_then(|raw| raw.parse::<u64>().ok())
            .filter(|parsed| *parsed > 0)
            .unwrap_or(15)
    }

    pub fn connect_session_monitor_interval_seconds() -> u64 {
        let configured = std::env::var("CONNECT_SESSION_MONITOR_INTERVAL_SECONDS").ok();
        Self::parse_connect_session_monitor_interval_seconds(configured.as_deref())
    }

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
            &task.task_type,
            task_registry_entry,
            task.requirements.min_nodes,
            task.requirements.require_gpu,
        )
        .await?;

        let task_status = self.get_task_status(task_id).await?;

        let status = parse_task_status(&task_status);

        let assigned_nodes = self.get_assigned_nodes(task_id).await?;

        let task_info = TaskInfo {
            task_id: task_id.to_string(),
            task_type,
            status,
            assigned_nodes,
            created_at: now.to_rfc3339(),
            updated_at: now.to_rfc3339(),
            result: None,
            proof_id: None,
        };

        Ok(task_info)
    }

    pub async fn complete_task_if_running(
        &self,
        task_id: Uuid,
        task_type: String,
        task_inputs: serde_json::Value,
    ) -> ApiResult<()> {
        let task_status = self.get_task_status(task_id).await?;
        if task_status != "running" {
            return Ok(());
        }

        let result = analyze_task_payload(&task_type, &task_inputs);

        let mut tx = self.db.begin().await?;

        let assigned_nodes_for_completed_task = sqlx::query_scalar::<_, String>(
            r#"
                SELECT node_id
                FROM task_assignments
                WHERE task_id = $1
                  AND disconnected_at IS NULL
                "#,
        )
        .bind(task_id)
        .fetch_all(&mut *tx)
        .await?;

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

        let should_disconnect_assignments = should_disconnect_assignments_on_completion(&task_type);
        if should_disconnect_assignments {
            self.disconnect_task_assignments(task_id, &mut tx).await?;
        }

        tx.commit().await?;

        if should_disconnect_assignments {
            for node_id in assigned_nodes_for_completed_task {
                self.assign_pending_tasks_for_node(&node_id).await?;
            }
        }

        Ok(())
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
        task_type: &str,
        task_registry_entry: &TaskTypeRegistryEntry,
        min_nodes: u32,
        require_gpu: bool,
    ) -> ApiResult<()> {
        let max_attachments = Self::max_active_task_attachments_per_node();
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

        if assigned_nodes >= min_nodes as i64 {
            return self
                .update_task_status_from_assignments(task_id, min_nodes)
                .await;
        }

        let additional_nodes_needed = min_nodes as i64 - assigned_nodes;
        let heartbeat_stale_timeout_seconds = Self::node_heartbeat_stale_timeout_seconds();
        let forbid_active_connect_session = task_type == "connect_only";

        let node_ids = sqlx::query_scalar::<_, String>(
            r#"
            SELECT n.node_id
            FROM nodes n
            LEFT JOIN task_assignments ta
              ON ta.node_id = n.node_id
             AND ta.disconnected_at IS NULL
            WHERE n.deleted_at IS NULL
              AND n.status = 'online'
              AND (
                    n.last_heartbeat >= NOW() - ($8::bigint * INTERVAL '1 second')
                    OR EXISTS (
                        SELECT 1
                        FROM connect_sessions cs
                        WHERE cs.node_id = n.node_id
                          AND cs.status = 'active'
                          AND cs.expires_at > NOW()
                    )
                  )
              AND (n.node_type = $1 OR n.node_type = 'any')
              AND n.cpu_cores >= $2
              AND n.memory_gb >= $3
              AND n.bandwidth_mbps >= $4
              AND ($5 = FALSE OR n.gpu_available = TRUE)
              AND (
                    $10 = FALSE
                    OR NOT EXISTS (
                        SELECT 1
                        FROM connect_sessions cs_busy
                        WHERE cs_busy.node_id = n.node_id
                          AND cs_busy.status = 'active'
                          AND cs_busy.expires_at > NOW()
                    )
                  )
              AND NOT EXISTS (
                  SELECT 1
                  FROM task_assignments existing
                  WHERE existing.task_id = $6
                    AND existing.node_id = n.node_id
                    AND existing.disconnected_at IS NULL
              )
            GROUP BY n.node_id, n.registered_at
            HAVING COUNT(ta.task_id) < $7
            ORDER BY n.registered_at ASC
            LIMIT $9
            "#,
        )
        .bind(task_registry_entry.preferred_node_type)
        .bind(task_registry_entry.minimum_capabilities.cpu_cores as i32)
        .bind(task_registry_entry.minimum_capabilities.memory_gb)
        .bind(task_registry_entry.minimum_capabilities.bandwidth_mbps)
        .bind(require_gpu || task_registry_entry.minimum_capabilities.gpu_available)
        .bind(task_id)
        .bind(max_attachments)
        .bind(heartbeat_stale_timeout_seconds)
        .bind(additional_nodes_needed)
        .bind(forbid_active_connect_session)
        .fetch_all(&self.db)
        .await?;

        for node_id in node_ids {
            sqlx::query(
                r#"
                INSERT INTO task_assignments (task_id, node_id)
                VALUES ($1, $2)
                ON CONFLICT (task_id, node_id)
                DO UPDATE SET assigned_at = NOW(), disconnected_at = NULL
                WHERE task_assignments.disconnected_at IS NOT NULL
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

    async fn active_attachment_count_for_node(&self, node_id: &str) -> ApiResult<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM task_assignments
            WHERE node_id = $1
              AND disconnected_at IS NULL
            "#,
        )
        .bind(node_id)
        .fetch_one(&self.db)
        .await?;

        Ok(count)
    }

    async fn assign_pending_tasks_for_node(&self, node_id: &str) -> ApiResult<()> {
        let max_attachments = Self::max_active_task_attachments_per_node();
        let mut current_attachments = self.active_attachment_count_for_node(node_id).await?;

        if current_attachments >= max_attachments {
            tracing::warn!(
                node_id,
                max_attachments,
                "Node reached active task attachment limit; skipping pending task attachment"
            );
            return Ok(());
        }

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
            if current_attachments >= max_attachments {
                tracing::info!(
                    node_id,
                    max_attachments,
                    "Stopped attaching node to pending tasks after reaching active task attachment limit"
                );
                break;
            }

            let task_id: Uuid = task.get("task_id");
            let task_type: String = task.get("task_type");
            let min_nodes: i32 = task.get("min_nodes");
            let require_gpu: bool = task.get("require_gpu");

            let Some(task_registry_entry) = task_type_registry_entry(&task_type) else {
                continue;
            };

            let heartbeat_stale_timeout_seconds = Self::node_heartbeat_stale_timeout_seconds();
            let forbid_active_connect_session = task_type == "connect_only";

            let node_is_eligible = sqlx::query_scalar::<_, bool>(
                r#"
                SELECT EXISTS (
                    SELECT 1
                    FROM nodes n
                    WHERE n.node_id = $1
                      AND n.deleted_at IS NULL
                      AND n.status = 'online'
                      AND (
                            n.last_heartbeat >= NOW() - ($7::bigint * INTERVAL '1 second')
                            OR EXISTS (
                                SELECT 1
                                FROM connect_sessions cs
                                WHERE cs.node_id = n.node_id
                                  AND cs.status = 'active'
                                  AND cs.expires_at > NOW()
                            )
                          )
                      AND (n.node_type = $2 OR n.node_type = 'any')
                      AND n.cpu_cores >= $3
                      AND n.memory_gb >= $4
                      AND n.bandwidth_mbps >= $5
                      AND ($6 = FALSE OR n.gpu_available = TRUE)
                      AND (
                            $8 = FALSE
                            OR NOT EXISTS (
                                SELECT 1
                                FROM connect_sessions cs_busy
                                WHERE cs_busy.node_id = n.node_id
                                  AND cs_busy.status = 'active'
                                  AND cs_busy.expires_at > NOW()
                            )
                          )
                )
                "#,
            )
            .bind(node_id)
            .bind(task_registry_entry.preferred_node_type)
            .bind(task_registry_entry.minimum_capabilities.cpu_cores as i32)
            .bind(task_registry_entry.minimum_capabilities.memory_gb)
            .bind(task_registry_entry.minimum_capabilities.bandwidth_mbps)
            .bind(require_gpu || task_registry_entry.minimum_capabilities.gpu_available)
            .bind(heartbeat_stale_timeout_seconds)
            .bind(forbid_active_connect_session)
            .fetch_one(&self.db)
            .await?;

            if !node_is_eligible {
                continue;
            }

            let rows = sqlx::query(
                r#"
                INSERT INTO task_assignments (task_id, node_id)
                VALUES ($1, $2)
                ON CONFLICT (task_id, node_id)
                DO UPDATE SET assigned_at = NOW(), disconnected_at = NULL
                WHERE task_assignments.disconnected_at IS NOT NULL
                "#,
            )
            .bind(task_id)
            .bind(node_id)
            .execute(&self.db)
            .await?;

            if rows.rows_affected() > 0 {
                current_attachments += 1;
            }

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

    pub async fn start_connect_session(
        &self,
        request: ConnectSessionStartRequest,
        requester_id: Uuid,
    ) -> ApiResult<ConnectSessionStartResponse> {
        let task_uuid = Uuid::parse_str(&request.task_id)
            .map_err(|_| ApiError::bad_request("task_id must be a valid UUID"))?;

        let task_row = sqlx::query(
            r#"
            SELECT t.inputs, t.status
            FROM tasks t
            WHERE t.task_id = $1
              AND t.creator_id = $2
              AND t.task_type = 'connect_only'
            "#,
        )
        .bind(task_uuid)
        .bind(requester_id)
        .fetch_optional(&self.db)
        .await?;

        let Some(task_row) = task_row else {
            return Err(ApiError::not_found_or_forbidden(
                "connect_only task not found for requester",
            ));
        };

        let status: String = task_row.get("status");
        if status != "running" {
            return Err(ApiError::bad_request(
                "connect session can only start when task status is running",
            ));
        }

        let inputs: serde_json::Value = task_row.get("inputs");
        let input_obj = inputs
            .as_object()
            .ok_or_else(|| ApiError::bad_request("task inputs must be an object"))?;

        let session_id = input_obj
            .get("session_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ApiError::bad_request("connect_only task missing session_id"))?
            .to_string();

        let egress_profile = input_obj
            .get("egress_profile")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ApiError::bad_request("connect_only task missing egress_profile"))?
            .to_string();

        let destination_policy_id = input_obj
            .get("destination_policy_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ApiError::bad_request("connect_only task missing destination_policy_id")
            })?
            .to_string();

        let duration_seconds = input_obj
            .get("duration_seconds")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| ApiError::bad_request("connect_only task missing duration_seconds"))?
            .clamp(1, 3600);

        let bandwidth_limit_mbps = input_obj
            .get("bandwidth_limit_mbps")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| {
                ApiError::bad_request("connect_only task missing bandwidth_limit_mbps")
            })?;
        let heartbeat_stale_timeout_seconds = Self::node_heartbeat_stale_timeout_seconds();

        let node_id = sqlx::query_scalar::<_, String>(
            r#"
            SELECT ta.node_id
            FROM task_assignments ta
            JOIN nodes n ON n.node_id = ta.node_id
            WHERE ta.task_id = $1
              AND ta.disconnected_at IS NULL
              AND n.deleted_at IS NULL
              AND n.status = 'online'
              AND n.last_heartbeat >= NOW() - ($2::bigint * INTERVAL '1 second')
              AND (n.node_type = 'open_internet' OR n.node_type = 'any')
            ORDER BY ta.assigned_at ASC
            LIMIT 1
            "#,
        )
        .bind(task_uuid)
        .bind(heartbeat_stale_timeout_seconds)
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| ApiError::service_unavailable("No eligible open_internet node assigned"))?;

        let protocol = request
            .tunnel_protocol
            .unwrap_or_else(|| "mtls".to_string());
        let now = chrono::Utc::now();
        let expires_at = now + chrono::Duration::seconds(duration_seconds);
        let session_token = crate::auth::generate_connect_session_token();
        let session_token_hash = crate::auth::hash_connect_session_token(&session_token);

        sqlx::query(
            r#"
            INSERT INTO connect_sessions (
                session_id, task_id, requester_id, node_id, tunnel_protocol,
                egress_profile, destination_policy_id, bandwidth_limit_mbps,
                session_token_hash, status, created_at, expires_at, last_heartbeat_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'active', $10, $11, $10)
            ON CONFLICT (session_id)
            DO UPDATE SET
                node_id = EXCLUDED.node_id,
                tunnel_protocol = EXCLUDED.tunnel_protocol,
                egress_profile = EXCLUDED.egress_profile,
                destination_policy_id = EXCLUDED.destination_policy_id,
                bandwidth_limit_mbps = EXCLUDED.bandwidth_limit_mbps,
                session_token_hash = EXCLUDED.session_token_hash,
                status = 'active',
                created_at = EXCLUDED.created_at,
                expires_at = EXCLUDED.expires_at,
                ended_at = NULL,
                last_heartbeat_at = EXCLUDED.last_heartbeat_at
            "#,
        )
        .bind(&session_id)
        .bind(task_uuid)
        .bind(requester_id)
        .bind(&node_id)
        .bind(&protocol)
        .bind(&egress_profile)
        .bind(&destination_policy_id)
        .bind(bandwidth_limit_mbps)
        .bind(&session_token_hash)
        .bind(now)
        .bind(expires_at)
        .execute(&self.db)
        .await?;

        let session = ConnectSessionInfo {
            session_id,
            task_id: task_uuid.to_string(),
            node_id,
            requester_id: requester_id.to_string(),
            tunnel_protocol: protocol,
            egress_profile,
            destination_policy_id,
            bandwidth_limit_mbps,
            status: ConnectSessionStatus::Active,
            created_at: now.to_rfc3339(),
            expires_at: expires_at.to_rfc3339(),
            last_heartbeat_at: Some(now.to_rfc3339()),
            ended_at: None,
        };

        Ok(ConnectSessionStartResponse {
            session,
            session_token,
        })
    }

    pub async fn get_connect_session(
        &self,
        session_id: &str,
        requester_id: Uuid,
    ) -> ApiResult<Option<ConnectSessionInfo>> {
        let row = sqlx::query(
            r#"
            SELECT session_id, task_id, requester_id, node_id, tunnel_protocol,
                   egress_profile, destination_policy_id, bandwidth_limit_mbps,
                   status, created_at, expires_at, last_heartbeat_at, ended_at
            FROM connect_sessions
            WHERE session_id = $1
              AND requester_id = $2
            "#,
        )
        .bind(session_id)
        .bind(requester_id)
        .fetch_optional(&self.db)
        .await?;

        Ok(row.map(map_connect_session_row))
    }

    pub async fn heartbeat_connect_session(
        &self,
        session_id: &str,
        requester_id: Uuid,
    ) -> ApiResult<Option<ConnectSessionInfo>> {
        let row = sqlx::query(
            r#"
            UPDATE connect_sessions
            SET last_heartbeat_at = NOW(),
                status = CASE WHEN expires_at < NOW() THEN 'expired' ELSE status END
            WHERE session_id = $1
              AND requester_id = $2
              AND status = 'active'
            RETURNING session_id, task_id, requester_id, node_id, tunnel_protocol,
                   egress_profile, destination_policy_id, bandwidth_limit_mbps,
                   status, created_at, expires_at, last_heartbeat_at, ended_at
            "#,
        )
        .bind(session_id)
        .bind(requester_id)
        .fetch_optional(&self.db)
        .await?;

        let Some(session_row) = row else {
            return Ok(None);
        };

        let session = map_connect_session_row(session_row);
        if !matches!(session.status, ConnectSessionStatus::Active) {
            return Ok(Some(session));
        }

        let heartbeat_stale_timeout_seconds = Self::node_heartbeat_stale_timeout_seconds();
        let node_is_healthy = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM nodes n
                WHERE n.node_id = $1
                  AND n.deleted_at IS NULL
                  AND n.status = 'online'
                  AND n.last_heartbeat >= NOW() - ($2::bigint * INTERVAL '1 second')
            )
            "#,
        )
        .bind(&session.node_id)
        .bind(heartbeat_stale_timeout_seconds)
        .fetch_one(&self.db)
        .await?;

        if node_is_healthy {
            return Ok(Some(session));
        }

        sqlx::query(
            r#"
            UPDATE connect_sessions
            SET status = 'ended', ended_at = NOW(), updated_at = NOW()
            WHERE session_id = $1
              AND requester_id = $2
              AND status = 'active'
            "#,
        )
        .bind(session_id)
        .bind(requester_id)
        .execute(&self.db)
        .await?;

        let task_uuid = Uuid::parse_str(&session.task_id)
            .map_err(|_| ApiError::internal_error("Invalid task ID format"))?;

        sqlx::query(
            r#"
            UPDATE task_assignments
            SET disconnected_at = NOW()
            WHERE task_id = $1
              AND node_id = $2
              AND disconnected_at IS NULL
            "#,
        )
        .bind(task_uuid)
        .bind(&session.node_id)
        .execute(&self.db)
        .await?;

        let task_meta = sqlx::query(
            r#"
            SELECT task_type, min_nodes, require_gpu
            FROM tasks
            WHERE task_id = $1
              AND status IN ('pending', 'running')
            "#,
        )
        .bind(task_uuid)
        .fetch_optional(&self.db)
        .await?;

        if let Some(task_meta) = task_meta {
            let task_type: String = task_meta.get("task_type");
            let min_nodes: i32 = task_meta.get("min_nodes");
            let require_gpu: bool = task_meta.get("require_gpu");

            if let Some(task_registry_entry) = task_type_registry_entry(&task_type) {
                self.assign_available_nodes_for_task(
                    task_uuid,
                    &task_type,
                    task_registry_entry,
                    min_nodes as u32,
                    require_gpu,
                )
                .await?;
            }
        }

        self.get_connect_session(session_id, requester_id).await
    }

    pub async fn stop_connect_session(
        &self,
        session_id: &str,
        requester_id: Uuid,
    ) -> ApiResult<Option<ConnectSessionInfo>> {
        let row = sqlx::query(
            r#"
            UPDATE connect_sessions
            SET status = 'ended', ended_at = NOW(), updated_at = NOW()
            WHERE session_id = $1
              AND requester_id = $2
              AND status = 'active'
            RETURNING session_id, task_id, requester_id, node_id, tunnel_protocol,
                   egress_profile, destination_policy_id, bandwidth_limit_mbps,
                   status, created_at, expires_at, last_heartbeat_at, ended_at
            "#,
        )
        .bind(session_id)
        .bind(requester_id)
        .fetch_optional(&self.db)
        .await?;

        Ok(row.map(map_connect_session_row))
    }

    /// Sweep active connect sessions and terminate sessions bound to stale/offline nodes.
    pub async fn sweep_connect_sessions(&self) -> ApiResult<usize> {
        let heartbeat_stale_timeout_seconds = Self::node_heartbeat_stale_timeout_seconds();
        let affected_task_ids = sqlx::query_scalar::<_, Uuid>(
            r#"
            WITH stale_sessions AS (
                SELECT
                    cs.session_id,
                    cs.task_id,
                    cs.node_id,
                    CASE
                        WHEN cs.expires_at < NOW() THEN 'expired'
                        ELSE 'ended'
                    END AS next_status
                FROM connect_sessions cs
                LEFT JOIN nodes n
                    ON n.node_id = cs.node_id
                    AND n.deleted_at IS NULL
                WHERE cs.status = 'active'
                  AND (
                        cs.expires_at < NOW()
                        OR n.node_id IS NULL
                        OR n.status <> 'online'
                        OR n.last_heartbeat < NOW() - ($1::bigint * INTERVAL '1 second')
                      )
            ),
            updated_sessions AS (
                UPDATE connect_sessions cs
                SET status = stale_sessions.next_status::connect_session_status,
                    ended_at = CASE
                        WHEN stale_sessions.next_status = 'ended' THEN NOW()
                        ELSE cs.ended_at
                    END,
                    updated_at = NOW()
                FROM stale_sessions
                WHERE cs.session_id = stale_sessions.session_id
                RETURNING stale_sessions.task_id, stale_sessions.node_id
            ),
            disconnected_assignments AS (
                UPDATE task_assignments ta
                SET disconnected_at = NOW()
                FROM updated_sessions us
                WHERE ta.task_id = us.task_id
                  AND ta.node_id = us.node_id
                  AND ta.disconnected_at IS NULL
                RETURNING ta.task_id
            )
            SELECT DISTINCT task_id
            FROM disconnected_assignments
            "#,
        )
        .bind(heartbeat_stale_timeout_seconds)
        .fetch_all(&self.db)
        .await?;

        for task_id in &affected_task_ids {
            let task_meta = sqlx::query(
                r#"
                SELECT task_type, min_nodes, require_gpu
                FROM tasks
                WHERE task_id = $1
                  AND status IN ('pending', 'running')
                "#,
            )
            .bind(task_id)
            .fetch_optional(&self.db)
            .await?;

            if let Some(task_meta) = task_meta {
                let task_type: String = task_meta.get("task_type");
                let min_nodes: i32 = task_meta.get("min_nodes");
                let require_gpu: bool = task_meta.get("require_gpu");

                if let Some(task_registry_entry) = task_type_registry_entry(&task_type) {
                    self.assign_available_nodes_for_task(
                        *task_id,
                        &task_type,
                        task_registry_entry,
                        min_nodes as u32,
                        require_gpu,
                    )
                    .await?;
                }
            }
        }

        Ok(affected_task_ids.len())
    }

    /// Delete a task created by the requesting user
    pub async fn delete_task(&self, task_id: &str, requester_id: Uuid) -> ApiResult<bool> {
        let task_uuid = match Uuid::parse_str(task_id) {
            Ok(id) => id,
            Err(_) => return Ok(false),
        };

        let assigned_nodes = sqlx::query_scalar::<_, String>(
            r#"
            SELECT node_id
            FROM task_assignments
            WHERE task_id = $1
              AND disconnected_at IS NULL
            "#,
        )
        .bind(task_uuid)
        .fetch_all(&self.db)
        .await?;

        let result = sqlx::query(
            r#"
            DELETE FROM tasks
            WHERE task_id = $1
              AND creator_id = $2
            "#,
        )
        .bind(task_uuid)
        .bind(requester_id)
        .execute(&self.db)
        .await?;

        if result.rows_affected() > 0 {
            for node_id in assigned_nodes {
                self.assign_pending_tasks_for_node(&node_id).await?;
            }
            return Ok(true);
        }

        Ok(false)
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
        use lettre::message::Mailbox;
        use lettre::Message;
        use tokio::io::AsyncWriteExt;
        use tokio::process::Command;

        let sender = std::env::var("EMAIL_FROM")
            .map_err(|_| ApiError::internal_error("EMAIL_FROM not configured"))?;

        if sender.contains('\r') || sender.contains('\n') {
            return Err(ApiError::internal_error(
                "EMAIL_FROM cannot contain carriage return or newline characters",
            ));
        }

        if recipient.contains('\r') || recipient.contains('\n') {
            return Err(ApiError::validation_error(
                "Email cannot contain carriage return or newline characters",
            ));
        }

        let sender_mailbox = sender
            .parse::<Mailbox>()
            .map_err(|_| ApiError::internal_error("EMAIL_FROM must be a valid email address"))?;
        let recipient_mailbox = recipient
            .parse::<Mailbox>()
            .map_err(|_| ApiError::validation_error("Email must be a valid address"))?;

        let subject = format!("Task Completed: {task_id}");
        let body = format!(
            "Your task {task_id} has completed.\n\nResult:\n{}",
            serde_json::to_string_pretty(result)
                .unwrap_or_else(|_| "<failed to serialize task result>".to_string())
        );

        let message = Message::builder()
            .from(sender_mailbox)
            .to(recipient_mailbox)
            .subject(subject)
            .body(body)
            .map_err(|_| ApiError::internal_error("Failed to build completion email message"))?;

        let mut child = Command::new("sendmail")
            .arg("-t")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|_| ApiError::internal_error("sendmail command is not available"))?;

        if let Some(stdin) = child.stdin.as_mut() {
            stdin
                .write_all(&message.formatted())
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

fn should_disconnect_assignments_on_completion(task_type: &str) -> bool {
    let _ = task_type;
    true
}

fn analyze_task_payload(task_type: &str, inputs: &serde_json::Value) -> serde_json::Value {
    if task_type == "connect_only" {
        return analyze_connect_only_payload(inputs);
    }

    match inputs {
        serde_json::Value::Object(map) => {
            match task_type {
                "federated_learning" => return analyze_federated_learning_payload(map),
                "zk_proof" => return analyze_zk_proof_payload(map),
                "wasm_execution" => return analyze_wasm_execution_payload(map),
                _ => {}
            }

            if task_type == "computation" {
                if let Some(expression) = map.get("expression").and_then(|v| v.as_str()) {
                    if let Some(result) = evaluate_arithmetic_expression(expression) {
                        tracing::info!(
                            task_type,
                            compute_path = "expression",
                            "Computation task routed to arithmetic expression executor"
                        );
                        return serde_json::json!({
                            "task_type": task_type,
                            "analysis_mode": "computation",
                            "compute_path": "expression",
                            "summary": "Arithmetic expression evaluated successfully.",
                            "expression": expression,
                            "result": result
                        });
                    }
                }
            }

            if let Some(prompt) = map.get("prompt").and_then(|v| v.as_str()) {
                if task_type == "computation" {
                    if let Some(expression) = infer_expression_from_prompt(prompt) {
                        if let Some(result) = evaluate_arithmetic_expression(&expression) {
                            tracing::info!(
                                task_type,
                                compute_path = "expression",
                                "Computation task routed to inferred arithmetic expression executor"
                            );
                            return serde_json::json!({
                                "task_type": task_type,
                                "analysis_mode": "computation",
                                "compute_path": "expression",
                                "summary": "Arithmetic expression inferred from prompt and evaluated successfully.",
                                "prompt": prompt,
                                "expression": expression,
                                "result": result
                            });
                        }
                    }

                    if let Some(operation) = parse_compute_operation_from_prompt(prompt) {
                        tracing::info!(
                            task_type,
                            compute_path = "algorithmic",
                            operation = ?operation,
                            "Computation task routed to algorithmic compute executor"
                        );
                        return execute_compute_operation(task_type, prompt, &operation);
                    }
                }

                tracing::info!(
                    task_type,
                    compute_path = "analysis",
                    "Computation task routed to plain-text analysis"
                );
                analyze_plain_text_prompt(task_type, prompt)
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
            "keyword_signals": extract_keywords(value),
            "recommendation": "Use an object payload with a `prompt` key for deeper text analysis."
        }),
        primitive => serde_json::json!({
            "task_type": task_type,
            "analysis_mode": "primitive",
            "summary": "Primitive payload analyzed successfully.",
            "value": primitive
        }),
    }
}

fn analyze_connect_only_payload(inputs: &serde_json::Value) -> serde_json::Value {
    let Some(map) = inputs.as_object() else {
        return serde_json::json!({
            "task_type": "connect_only",
            "analysis_mode": "connect_only",
            "status": "rejected",
            "summary": "connect_only payload must be a JSON object."
        });
    };

    serde_json::json!({
        "task_type": "connect_only",
        "analysis_mode": "connect_only",
        "status": "accepted",
        "summary": "Connectivity-only session requested. No arbitrary compute payload will be executed.",
        "session_id": map.get("session_id").cloned().unwrap_or(serde_json::Value::Null),
        "requester_id": map.get("requester_id").cloned().unwrap_or(serde_json::Value::Null),
        "duration_seconds": map.get("duration_seconds").cloned().unwrap_or(serde_json::Value::Null),
        "bandwidth_limit_mbps": map
            .get("bandwidth_limit_mbps")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "egress_profile": map.get("egress_profile").cloned().unwrap_or(serde_json::Value::Null),
        "destination_policy_id": map
            .get("destination_policy_id")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "enforcement": {
            "task_description_allowed": false,
            "wasm_module_allowed": false,
            "gpu_execution_allowed": false,
            "proof_generation_allowed": false,
            "policy_validation_required": true
        }
    })
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
enum ComputeOperation {
    ArithmeticExpression(String),
    MonteCarloPi { samples: u64 },
    MlInference,
    ZkProof,
    WasmExecution,
}

fn parse_compute_operation_from_prompt(prompt: &str) -> Option<ComputeOperation> {
    if let Some(expression) = infer_expression_from_prompt(prompt) {
        return Some(ComputeOperation::ArithmeticExpression(expression));
    }

    let lower = prompt.to_lowercase();
    let mentions_monte_carlo = lower.contains("monte carlo");
    let mentions_pi = lower.contains('π') || lower.contains("pi");
    let mentions_simulation = lower.contains("simulation") || lower.contains("estimate");

    if mentions_monte_carlo && mentions_pi && mentions_simulation {
        let samples = extract_sample_count_from_prompt(prompt).unwrap_or(100_000);
        return Some(ComputeOperation::MonteCarloPi { samples });
    }

    None
}

fn extract_sample_count_from_prompt(prompt: &str) -> Option<u64> {
    let sanitized = prompt.replace(',', "");
    let mut previous: Option<String> = None;

    for token in sanitized.split_whitespace() {
        let lowered = token.to_lowercase();
        let normalized = lowered.trim_matches(|c: char| !c.is_ascii_alphanumeric());

        if normalized == "samples" || normalized == "sample" {
            if let Some(prev) = previous.as_deref() {
                if let Some(value) = parse_u64_token(prev) {
                    return Some(value.max(1));
                }
            }
        }

        if let Some(rest) = normalized.strip_prefix("samples=") {
            if let Some(value) = parse_u64_token(rest) {
                return Some(value.max(1));
            }
        }

        previous = Some(normalized.to_string());
    }

    sanitized
        .split(|c: char| !c.is_ascii_digit())
        .find_map(parse_u64_token)
        .map(|v| v.max(1))
}

fn parse_u64_token(token: &str) -> Option<u64> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return None;
    }

    trimmed.parse::<u64>().ok()
}

fn execute_compute_operation(
    task_type: &str,
    prompt: &str,
    operation: &ComputeOperation,
) -> serde_json::Value {
    match operation {
        ComputeOperation::ArithmeticExpression(expression) => {
            if let Some(result) = evaluate_arithmetic_expression(expression) {
                serde_json::json!({
                    "task_type": task_type,
                    "analysis_mode": "computation",
                    "compute_path": "expression",
                    "summary": "Arithmetic expression inferred from prompt and evaluated successfully.",
                    "prompt": prompt,
                    "expression": expression,
                    "result": result
                })
            } else {
                analyze_plain_text_prompt(task_type, prompt)
            }
        }
        ComputeOperation::MonteCarloPi { samples } => {
            let simulation = execute_monte_carlo_pi(*samples, monte_carlo_seed());
            serde_json::json!({
                "task_type": task_type,
                "analysis_mode": "computation",
                "compute_path": "algorithmic",
                "operation": "monte_carlo_pi",
                "summary": "Monte Carlo π simulation executed successfully.",
                "prompt": prompt,
                "estimated_pi": simulation.estimated_pi,
                "samples": simulation.samples,
                "duration_ms": simulation.duration_ms,
                "seed": simulation.seed
            })
        }
        _ => analyze_plain_text_prompt(task_type, prompt),
    }
}

fn monte_carlo_seed() -> Option<u64> {
    std::env::var("COMPUTE_MONTE_CARLO_SEED")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
}

struct MonteCarloPiResult {
    estimated_pi: f64,
    samples: u64,
    duration_ms: u128,
    seed: Option<u64>,
}

fn execute_monte_carlo_pi(samples: u64, seed: Option<u64>) -> MonteCarloPiResult {
    use rand::{rngs::StdRng, Rng, SeedableRng};

    let started = std::time::Instant::now();
    let mut inside_circle: u64 = 0;

    let mut rng = match seed {
        Some(seed_value) => StdRng::seed_from_u64(seed_value),
        None => StdRng::from_entropy(),
    };

    for _ in 0..samples {
        let x: f64 = rng.gen_range(0.0..1.0);
        let y: f64 = rng.gen_range(0.0..1.0);
        if x * x + y * y <= 1.0 {
            inside_circle += 1;
        }
    }

    let estimated_pi = if samples == 0 {
        0.0
    } else {
        4.0 * inside_circle as f64 / samples as f64
    };

    MonteCarloPiResult {
        estimated_pi,
        samples,
        duration_ms: started.elapsed().as_millis(),
        seed,
    }
}

fn analyze_federated_learning_payload(
    map: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Value {
    let participant_count = map
        .get("participant_count")
        .and_then(|v| v.as_u64())
        .or_else(|| {
            map.get("clients")
                .and_then(|v| v.as_array())
                .map(|v| v.len() as u64)
        });

    let round_count = map
        .get("rounds")
        .and_then(|v| v.as_u64())
        .or_else(|| map.get("num_rounds").and_then(|v| v.as_u64()));

    let aggregation_strategy = map
        .get("aggregation_strategy")
        .or_else(|| map.get("aggregation"))
        .and_then(|value| value.as_str())
        .unwrap_or("fedavg")
        .to_lowercase();

    let privacy_budget = map
        .get("privacy_budget")
        .and_then(parse_privacy_budget)
        .map(|budget| {
            serde_json::json!({
                "epsilon": budget.epsilon,
                "delta": budget.delta
            })
        });

    let aggregation_preview = if aggregation_strategy == "fedavg" {
        aggregate_fedavg_preview(map)
    } else {
        None
    };

    let wiring_status = if aggregation_strategy != "fedavg" {
        "unsupported_strategy"
    } else if map.contains_key("global_model") && map.contains_key("client_updates") {
        if aggregation_preview.is_some() {
            "wired"
        } else {
            "invalid_updates"
        }
    } else {
        "awaiting_client_updates"
    };

    serde_json::json!({
        "task_type": "federated_learning",
        "analysis_mode": "federated_learning",
        "summary": "Federated learning payload analyzed successfully.",
        "participant_count": participant_count,
        "round_count": round_count,
        "aggregation_strategy": aggregation_strategy,
        "wiring_status": wiring_status,
        "privacy_budget": privacy_budget,
        "aggregation_preview": aggregation_preview,
        "has_model_config": map.contains_key("model") || map.contains_key("model_config"),
        "has_aggregation_strategy": map.contains_key("aggregation") || map.contains_key("aggregation_strategy"),
        "top_level_keys": map.keys().cloned().collect::<Vec<String>>()
    })
}

fn parse_privacy_budget(value: &serde_json::Value) -> Option<PrivacyBudget> {
    let map = value.as_object()?;
    let epsilon = map.get("epsilon")?.as_f64()?;
    let delta = map.get("delta")?.as_f64()?;

    if epsilon <= 0.0 || delta <= 0.0 {
        return None;
    }

    Some(PrivacyBudget::new(epsilon, delta))
}

fn aggregate_fedavg_preview(
    map: &serde_json::Map<String, serde_json::Value>,
) -> Option<serde_json::Value> {
    let global_model = parse_model_weights(map.get("global_model")?)?;
    let updates = map.get("client_updates")?.as_array()?;

    let mut aggregator = FederatedAggregator::new(global_model);

    for update in updates {
        let update_map = update.as_object()?;
        let client_id = update_map.get("client_id")?.as_str()?.to_string();
        let num_samples = update_map
            .get("num_samples")
            .and_then(|value| value.as_u64())? as usize;
        if num_samples == 0 {
            return None;
        }

        let model = parse_model_weights(update_map.get("model")?)?;
        aggregator
            .add_client_update(client_id, model, num_samples)
            .ok()?;
    }

    let aggregated_model = aggregator.aggregate().ok()?;

    Some(serde_json::json!({
        "round": aggregator.current_round(),
        "version": aggregated_model.version,
        "num_layers": aggregated_model.layers.len(),
        "num_parameters": aggregated_model.num_parameters(),
        "layers": aggregated_model.layers.iter().map(|layer| {
            serde_json::json!({
                "name": layer.name,
                "shape": layer.shape,
                "weights": layer.weights
            })
        }).collect::<Vec<_>>()
    }))
}

fn parse_model_weights(value: &serde_json::Value) -> Option<ModelWeights> {
    let map = value.as_object()?;
    let layers = map.get("layers")?.as_array()?;

    let parsed_layers = layers
        .iter()
        .map(parse_layer_weights)
        .collect::<Option<Vec<_>>>()?;

    let version = map.get("version").and_then(|v| v.as_u64()).unwrap_or(0);

    Some(ModelWeights {
        layers: parsed_layers,
        version,
    })
}

fn parse_layer_weights(value: &serde_json::Value) -> Option<LayerWeights> {
    let map = value.as_object()?;
    let name = map.get("name")?.as_str()?.to_string();

    let weights = map
        .get("weights")?
        .as_array()?
        .iter()
        .map(|weight| weight.as_f64())
        .collect::<Option<Vec<_>>>()?;

    let shape = map
        .get("shape")?
        .as_array()?
        .iter()
        .map(|dimension| dimension.as_u64().map(|value| value as usize))
        .collect::<Option<Vec<_>>>()?;

    Some(LayerWeights {
        name,
        weights,
        shape,
    })
}

fn analyze_plain_text_prompt(task_type: &str, prompt: &str) -> serde_json::Value {
    let words: Vec<&str> = prompt.split_whitespace().collect();
    let sentence_count = prompt
        .split(['.', '!', '?'])
        .filter(|s| !s.trim().is_empty())
        .count();
    let estimated_reading_time_sec = ((words.len() as f64 / 3.0).ceil() as u64).max(1);

    let extracted_numbers = extract_number_literals(prompt);
    let looks_like_instruction = contains_instructional_language(prompt);
    let has_question = prompt.contains('?');

    serde_json::json!({
        "task_type": task_type,
        "analysis_mode": "plain_text",
        "summary": summarize_prompt(prompt),
        "input_characters": prompt.chars().count(),
        "word_count": words.len(),
        "sentence_count": sentence_count,
        "has_question": has_question,
        "looks_like_instruction": looks_like_instruction,
        "estimated_reading_time_sec": estimated_reading_time_sec,
        "keyword_signals": extract_keywords(prompt),
        "extracted_numbers": extracted_numbers,
        "suggested_json_shape": suggested_json_shape_for_task(task_type, prompt),
        "recommendation": task_specific_plain_text_recommendation(task_type)
    })
}

fn analyze_zk_proof_payload(map: &serde_json::Map<String, serde_json::Value>) -> serde_json::Value {
    let circuit_name = map
        .get("circuit")
        .and_then(|v| v.as_str())
        .or_else(|| map.get("circuit_name").and_then(|v| v.as_str()));

    let public_input_count = map
        .get("public_inputs")
        .and_then(|v| v.as_array())
        .map(|v| v.len())
        .or_else(|| {
            map.get("public_input_count")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
        });

    serde_json::json!({
        "task_type": "zk_proof",
        "analysis_mode": "zk_proof",
        "summary": "Zero-knowledge proof payload analyzed successfully.",
        "circuit": circuit_name,
        "public_input_count": public_input_count,
        "has_witness": map.contains_key("witness") || map.contains_key("private_inputs"),
        "has_proof_system": map.contains_key("proof_system") || map.contains_key("protocol"),
        "top_level_keys": map.keys().cloned().collect::<Vec<String>>()
    })
}

fn analyze_wasm_execution_payload(
    map: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Value {
    let module_size_bytes = map
        .get("module_size_bytes")
        .and_then(|v| v.as_u64())
        .or_else(|| map.get("wasm_size_bytes").and_then(|v| v.as_u64()));

    let function_name = map
        .get("entrypoint")
        .and_then(|v| v.as_str())
        .or_else(|| map.get("function").and_then(|v| v.as_str()));

    serde_json::json!({
        "task_type": "wasm_execution",
        "analysis_mode": "wasm_execution",
        "summary": "WASM execution payload analyzed successfully.",
        "entrypoint": function_name,
        "module_size_bytes": module_size_bytes,
        "has_wasm_module": map.contains_key("wasm_module") || map.contains_key("module_bytes") || map.contains_key("module"),
        "has_runtime_limits": map.contains_key("limits") || map.contains_key("timeout_ms") || map.contains_key("memory_limit_mb"),
        "top_level_keys": map.keys().cloned().collect::<Vec<String>>()
    })
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

fn infer_expression_from_prompt(prompt: &str) -> Option<String> {
    let mut seen_digit = false;
    let mut expression = String::new();

    for ch in prompt.chars() {
        if !seen_digit {
            if ch.is_ascii_digit() {
                seen_digit = true;
                expression.push(ch);
            }
            continue;
        }

        if ch.is_ascii_digit() || matches!(ch, '+' | '-' | '*' | '/' | '.' | 'x' | 'X' | '÷') {
            expression.push(ch);
            continue;
        }

        if ch.is_whitespace() {
            expression.push(ch);
            continue;
        }

        break;
    }

    if seen_digit
        && expression
            .chars()
            .any(|c| matches!(c, '+' | '-' | '*' | '/' | 'x' | 'X' | '÷'))
    {
        Some(expression.trim().to_string())
    } else {
        None
    }
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
        format!("{}... ({} words)", preview, words.len())
    } else {
        format!("{} ({} words)", preview, words.len())
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

fn extract_number_literals(prompt: &str) -> Vec<f64> {
    prompt
        .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
        .filter_map(|token| {
            let trimmed = token.trim();
            if trimmed.is_empty() || trimmed == "-" || trimmed == "." {
                return None;
            }
            trimmed.parse::<f64>().ok()
        })
        .take(8)
        .collect()
}

fn contains_instructional_language(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    [
        "compute",
        "calculate",
        "analyze",
        "summarize",
        "classify",
        "extract",
        "generate",
        "predict",
        "verify",
    ]
    .iter()
    .any(|kw| lower.contains(kw))
}

fn task_specific_plain_text_recommendation(task_type: &str) -> &'static str {
    match task_type {
        "computation" => {
            "Provide JSON with `expression` for deterministic math or include `prompt` plus `operation`/`target` fields for richer compute analysis."
        }
        "federated_learning" => {
            "Provide JSON with `participant_count`, `rounds`, and `aggregation_strategy` to get federated-learning specific analysis."
        }
        "zk_proof" => {
            "Provide JSON with `circuit`, `public_inputs`, and `proof_system` for precise proof-task analysis."
        }
        "wasm_execution" => {
            "Provide JSON with `entrypoint`, module details, and runtime limits for actionable WASM execution analysis."
        }
        _ => "Use JSON object inputs for richer structured outputs when possible.",
    }
}

fn suggested_json_shape_for_task(task_type: &str, prompt: &str) -> serde_json::Value {
    match task_type {
        "computation" => {
            if let Some(expression) = infer_expression_from_prompt(prompt) {
                serde_json::json!({
                    "expression": expression,
                    "operation": "evaluate"
                })
            } else {
                serde_json::json!({
                    "prompt": prompt,
                    "operation": "analyze"
                })
            }
        }
        "federated_learning" => serde_json::json!({
            "prompt": prompt,
            "participant_count": null,
            "rounds": null,
            "aggregation_strategy": "fedavg"
        }),
        "zk_proof" => serde_json::json!({
            "prompt": prompt,
            "circuit": null,
            "public_inputs": [],
            "proof_system": null
        }),
        "wasm_execution" => serde_json::json!({
            "prompt": prompt,
            "entrypoint": null,
            "module_size_bytes": null,
            "timeout_ms": null
        }),
        _ => serde_json::json!({ "prompt": prompt }),
    }
}

fn parse_connect_session_status(status: &str) -> ConnectSessionStatus {
    match status.to_lowercase().as_str() {
        "active" => ConnectSessionStatus::Active,
        "ended" => ConnectSessionStatus::Ended,
        "expired" => ConnectSessionStatus::Expired,
        _ => ConnectSessionStatus::Expired,
    }
}

fn map_connect_session_row(row: sqlx::postgres::PgRow) -> ConnectSessionInfo {
    ConnectSessionInfo {
        session_id: row.get("session_id"),
        task_id: row.get::<Uuid, _>("task_id").to_string(),
        requester_id: row.get::<Uuid, _>("requester_id").to_string(),
        node_id: row.get("node_id"),
        tunnel_protocol: row.get("tunnel_protocol"),
        egress_profile: row.get("egress_profile"),
        destination_policy_id: row.get("destination_policy_id"),
        bandwidth_limit_mbps: row.get("bandwidth_limit_mbps"),
        status: parse_connect_session_status(&row.get::<String, _>("status")),
        created_at: row
            .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
            .to_rfc3339(),
        expires_at: row
            .get::<chrono::DateTime<chrono::Utc>, _>("expires_at")
            .to_rfc3339(),
        last_heartbeat_at: row
            .get::<Option<chrono::DateTime<chrono::Utc>>, _>("last_heartbeat_at")
            .map(|v| v.to_rfc3339()),
        ended_at: row
            .get::<Option<chrono::DateTime<chrono::Utc>>, _>("ended_at")
            .map(|v| v.to_rfc3339()),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_max_active_task_attachments_env_value() {
        assert_eq!(
            AppState::parse_max_active_task_attachments_per_node(Some("75")),
            75
        );
    }

    #[test]
    fn falls_back_to_default_on_invalid_or_non_positive_attachment_limits() {
        assert_eq!(
            AppState::parse_max_active_task_attachments_per_node(Some("0")),
            50
        );
        assert_eq!(
            AppState::parse_max_active_task_attachments_per_node(Some("-12")),
            50
        );
        assert_eq!(
            AppState::parse_max_active_task_attachments_per_node(Some("abc")),
            50
        );
        assert_eq!(
            AppState::parse_max_active_task_attachments_per_node(None),
            50
        );
    }

    #[test]
    fn parses_heartbeat_stale_timeout_seconds() {
        assert_eq!(
            AppState::parse_connect_session_monitor_interval_seconds(Some("30")),
            30
        );
        assert_eq!(
            AppState::parse_connect_session_monitor_interval_seconds(Some("0")),
            15
        );
        assert_eq!(
            AppState::parse_connect_session_monitor_interval_seconds(Some("-5")),
            15
        );
        assert_eq!(
            AppState::parse_connect_session_monitor_interval_seconds(Some("bogus")),
            15
        );
        assert_eq!(
            AppState::parse_connect_session_monitor_interval_seconds(None),
            15
        );
    }

    #[test]
    fn analyzes_plain_text_prompt_payload() {
        let value =
            serde_json::json!({"prompt": "Summarize this medical timeline and flag anomalies"});
        let result = analyze_task_payload("computation", &value);

        assert_eq!(result["analysis_mode"], "plain_text");
        assert!(result["summary"]
            .as_str()
            .unwrap_or_default()
            .contains("words"));
        assert!(result["word_count"].as_u64().unwrap_or_default() > 0);
        assert!(result["suggested_json_shape"].is_object());
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
    fn evaluates_inferred_expression_from_prompt_payload() {
        let value = serde_json::json!({"prompt": "need to figure out what 5 + 5 is"});
        let result = analyze_task_payload("computation", &value);

        assert_eq!(result["analysis_mode"], "computation");
        assert_eq!(result["expression"], "5 + 5");
        assert_eq!(result["result"], 10.0);
    }

    #[test]
    fn returns_json_analysis_for_invalid_expression_payload() {
        let value = serde_json::json!({"expression": "10 apples", "operation": "multiply"});
        let result = analyze_task_payload("computation", &value);

        assert_eq!(result["analysis_mode"], "json");
    }

    #[test]
    fn executes_monte_carlo_pi_from_prompt_payload() {
        std::env::set_var("COMPUTE_MONTE_CARLO_SEED", "42");
        let value = serde_json::json!({
            "prompt": "compute a Monte Carlo simulation estimating π using 1,000,000 random samples"
        });
        let result = analyze_task_payload("computation", &value);
        std::env::remove_var("COMPUTE_MONTE_CARLO_SEED");

        assert_eq!(result["analysis_mode"], "computation");
        assert_eq!(result["compute_path"], "algorithmic");
        assert_eq!(result["operation"], "monte_carlo_pi");
        assert_eq!(result["samples"], 1_000_000);
        let estimated_pi = result["estimated_pi"].as_f64().unwrap_or_default();
        assert!(estimated_pi > 3.10 && estimated_pi < 3.18);
    }

    #[test]
    fn unknown_compute_prompt_falls_back_to_analysis() {
        let value =
            serde_json::json!({"prompt": "compute a deep causal graph about patient trajectories"});
        let result = analyze_task_payload("computation", &value);

        assert_eq!(result["analysis_mode"], "plain_text");
        assert_eq!(result["task_type"], "computation");
    }
    #[test]
    fn analyzes_federated_learning_payload() {
        let value = serde_json::json!({
            "participant_count": 12,
            "rounds": 20,
            "aggregation_strategy": "fedavg"
        });
        let result = analyze_task_payload("federated_learning", &value);

        assert_eq!(result["analysis_mode"], "federated_learning");
        assert_eq!(result["participant_count"], 12);
        assert_eq!(result["aggregation_strategy"], "fedavg");
        assert_eq!(result["wiring_status"], "awaiting_client_updates");
    }

    #[test]
    fn federated_learning_wiring_aggregates_client_updates() {
        let value = serde_json::json!({
            "participant_count": 2,
            "rounds": 1,
            "aggregation_strategy": "fedavg",
            "privacy_budget": {
                "epsilon": 1.0,
                "delta": 1e-5
            },
            "global_model": {
                "version": 0,
                "layers": [
                    {"name": "layer1", "weights": [1.0, 2.0], "shape": [2]}
                ]
            },
            "client_updates": [
                {
                    "client_id": "client-1",
                    "num_samples": 2,
                    "model": {
                        "version": 0,
                        "layers": [
                            {"name": "layer1", "weights": [2.0, 4.0], "shape": [2]}
                        ]
                    }
                },
                {
                    "client_id": "client-2",
                    "num_samples": 1,
                    "model": {
                        "version": 0,
                        "layers": [
                            {"name": "layer1", "weights": [5.0, 7.0], "shape": [2]}
                        ]
                    }
                }
            ]
        });

        let result = analyze_task_payload("federated_learning", &value);

        assert_eq!(result["wiring_status"], "wired");
        assert_eq!(result["privacy_budget"]["epsilon"], 1.0);
        assert_eq!(result["privacy_budget"]["delta"], 1e-5);

        let weights = result["aggregation_preview"]["layers"][0]["weights"]
            .as_array()
            .expect("weights to be present");
        assert_eq!(weights[0], 3.0);
        assert_eq!(weights[1], 5.0);
    }

    #[test]
    fn analyzes_zk_proof_payload() {
        let value = serde_json::json!({
            "circuit": "range_check",
            "public_inputs": [1, 2, 3],
            "proof_system": "groth16"
        });
        let result = analyze_task_payload("zk_proof", &value);

        assert_eq!(result["analysis_mode"], "zk_proof");
        assert_eq!(result["public_input_count"], 3);
    }

    #[test]
    fn analyzes_wasm_execution_payload() {
        let value = serde_json::json!({
            "entrypoint": "run",
            "module_size_bytes": 2048,
            "timeout_ms": 5000
        });
        let result = analyze_task_payload("wasm_execution", &value);

        assert_eq!(result["analysis_mode"], "wasm_execution");
        assert_eq!(result["entrypoint"], "run");
    }

    #[test]
    fn connect_only_payload_reports_isolation_enforcement_flags() {
        let value = serde_json::json!({
            "session_id": "sess_123",
            "requester_id": "user_abc",
            "duration_seconds": 120,
            "bandwidth_limit_mbps": 50,
            "egress_profile": "allowlist_domains",
            "destination_policy_id": "policy_web_basic_v1"
        });

        let result = analyze_task_payload("connect_only", &value);

        assert_eq!(result["analysis_mode"], "connect_only");
        assert_eq!(result["status"], "accepted");
        assert_eq!(result["enforcement"]["task_description_allowed"], false);
        assert_eq!(result["enforcement"]["wasm_module_allowed"], false);
        assert_eq!(result["enforcement"]["gpu_execution_allowed"], false);
        assert_eq!(result["enforcement"]["proof_generation_allowed"], false);
        assert_eq!(result["enforcement"]["policy_validation_required"], true);
    }

    #[test]
    fn task_completion_disconnects_assignments_for_all_task_types() {
        assert!(should_disconnect_assignments_on_completion("connect_only"));
        assert!(should_disconnect_assignments_on_completion("computation"));
        assert!(should_disconnect_assignments_on_completion(
            "federated_learning"
        ));
        assert!(should_disconnect_assignments_on_completion("zk_proof"));
        assert!(should_disconnect_assignments_on_completion(
            "wasm_execution"
        ));
    }
}
