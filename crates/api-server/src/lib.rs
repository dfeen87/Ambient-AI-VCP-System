use axum::{
    extract::{Path, State},
    http::StatusCode,
    middleware as axum_middleware,
    routing::{get, post, put},
    Json, Router,
};
use sqlx::Row;
use std::sync::Arc;
use tracing::info;
use utoipa::OpenApi;
use uuid::Uuid;

pub mod auth;
pub mod db;
pub mod error;
pub mod middleware;
pub mod models;
pub mod rate_limit;
pub mod state;

use error::{ApiError, ApiResult};
use models::*;
use state::AppState;

/// API Documentation
#[derive(OpenApi)]
#[openapi(
    paths(
        health_check,
        register_node,
        list_nodes,
        get_node,
        delete_node,
        reject_node,
        update_heartbeat,
        submit_task,
        get_task,
        list_tasks,
        verify_proof,
        get_cluster_stats,
        register_user,
        login,
        refresh_token,
    ),
    components(schemas(
        HealthResponse,
        NodeRegistration,
        NodeInfo,
        TaskSubmission,
        TaskInfo,
        TaskStatus,
        ProofVerificationRequest,
        ProofVerificationResponse,
        ClusterStats,
        ApiError,
        auth::RegisterRequest,
        auth::LoginRequest,
        auth::LoginResponse,
        auth::RefreshTokenRequest,
        auth::RefreshTokenResponse,
    ))
)]
struct ApiDoc;

/// Serve the dashboard
async fn dashboard() -> axum::response::Html<&'static str> {
    axum::response::Html(include_str!("../assets/index.html"))
}

/// Serve the custom Swagger UI
async fn swagger_ui() -> axum::response::Html<&'static str> {
    axum::response::Html(include_str!("../assets/swagger.html"))
}

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/api/v1/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    )
)]
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

/// Register a new node
#[utoipa::path(
    post,
    path = "/api/v1/nodes",
    request_body = NodeRegistration,
    responses(
        (status = 201, description = "Node registered successfully", body = NodeInfo),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
async fn register_node(
    State(state): State<Arc<AppState>>,
    auth_user: auth::AuthUser,
    Json(registration): Json<NodeRegistration>,
) -> ApiResult<(StatusCode, Json<NodeInfo>)> {
    registration.validate()?;

    info!(
        "Registering node: {} in region {} for user {}",
        registration.node_id, registration.region, auth_user.username
    );

    // Parse user_id
    let user_id = Uuid::parse_str(&auth_user.user_id)
        .map_err(|_| ApiError::internal_error("Invalid user ID format"))?;

    let node_info = state.register_node(registration, user_id).await?;

    Ok((StatusCode::CREATED, Json(node_info)))
}

/// List all nodes
#[utoipa::path(
    get,
    path = "/api/v1/nodes",
    responses(
        (status = 200, description = "List of nodes", body = Vec<NodeInfo>)
    )
)]
async fn list_nodes(State(state): State<Arc<AppState>>) -> Json<Vec<NodeInfo>> {
    let nodes = state.list_nodes().await;
    Json(nodes)
}

/// Get a specific node
#[utoipa::path(
    get,
    path = "/api/v1/nodes/{node_id}",
    params(
        ("node_id" = String, Path, description = "Node ID")
    ),
    responses(
        (status = 200, description = "Node information", body = NodeInfo),
        (status = 404, description = "Node not found", body = ApiError)
    )
)]
async fn get_node(
    State(state): State<Arc<AppState>>,
    Path(node_id): Path<String>,
) -> ApiResult<Json<NodeInfo>> {
    let node = state
        .get_node(&node_id)
        .await
        .ok_or_else(|| ApiError::not_found(format!("Node {} not found", node_id)))?;

    Ok(Json(node))
}

/// Delete a node (soft delete)
#[utoipa::path(
    delete,
    path = "/api/v1/nodes/{node_id}",
    params(
        ("node_id" = String, Path, description = "Node ID")
    ),
    responses(
        (status = 200, description = "Node deleted successfully"),
        (status = 404, description = "Node not found or you don't have permission to delete it", body = ApiError)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
async fn delete_node(
    State(state): State<Arc<AppState>>,
    auth_user: auth::AuthUser,
    Path(node_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    info!("Deleting node: {} for user {}", node_id, auth_user.username);

    // Parse user_id
    let user_id = Uuid::parse_str(&auth_user.user_id)
        .map_err(|_| ApiError::internal_error("Invalid user ID format"))?;

    let deleted = state.delete_node(&node_id, user_id).await?;

    if !deleted {
        return Err(ApiError::not_found_or_forbidden(format!(
            "Node {} not found or you don't have permission to delete it",
            node_id
        )));
    }

    Ok(Json(serde_json::json!({
        "message": "Node deleted successfully",
        "node_id": node_id
    })))
}

/// Reject a node (owner only)
#[utoipa::path(
    post,
    path = "/api/v1/nodes/{node_id}/reject",
    params(
        ("node_id" = String, Path, description = "Node ID")
    ),
    responses(
        (status = 200, description = "Node rejected successfully"),
        (status = 404, description = "Node not found or you don't have permission to reject it", body = ApiError)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
async fn reject_node(
    State(state): State<Arc<AppState>>,
    auth_user: auth::AuthUser,
    Path(node_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    info!(
        "Rejecting node: {} for user {}",
        node_id, auth_user.username
    );

    // Parse user_id
    let user_id = Uuid::parse_str(&auth_user.user_id)
        .map_err(|_| ApiError::internal_error("Invalid user ID format"))?;

    let rejected = state.reject_node(&node_id, user_id).await?;

    if !rejected {
        return Err(ApiError::not_found_or_forbidden(format!(
            "Node {} not found or you don't have permission to reject it",
            node_id
        )));
    }

    Ok(Json(serde_json::json!({
        "message": "Node rejected successfully",
        "node_id": node_id,
        "status": "rejected"
    })))
}

/// Update node heartbeat
#[utoipa::path(
    put,
    path = "/api/v1/nodes/{node_id}/heartbeat",
    params(
        ("node_id" = String, Path, description = "Node ID")
    ),
    responses(
        (status = 200, description = "Heartbeat updated successfully"),
        (status = 404, description = "Node not found or you don't have permission to update it", body = ApiError)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
async fn update_heartbeat(
    State(state): State<Arc<AppState>>,
    auth_user: auth::AuthUser,
    Path(node_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    // Parse user_id
    let user_id = Uuid::parse_str(&auth_user.user_id)
        .map_err(|_| ApiError::internal_error("Invalid user ID format"))?;

    let updated = state.update_node_heartbeat(&node_id, user_id).await?;

    if !updated {
        return Err(ApiError::not_found_or_forbidden(format!(
            "Node {} not found or you don't have permission to update it",
            node_id
        )));
    }

    Ok(Json(serde_json::json!({
        "message": "Heartbeat updated successfully",
        "node_id": node_id,
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

/// Submit a task
#[utoipa::path(
    post,
    path = "/api/v1/tasks",
    request_body = TaskSubmission,
    responses(
        (status = 201, description = "Task submitted successfully", body = TaskInfo),
        (status = 400, description = "Invalid request", body = ApiError)
    )
)]
async fn submit_task(
    State(state): State<Arc<AppState>>,
    auth_user: auth::AuthUser,
    Json(task): Json<TaskSubmission>,
) -> ApiResult<(StatusCode, Json<TaskInfo>)> {
    task.validate()?;

    info!("Submitting task: {}", task.task_type);

    let creator_id = Uuid::parse_str(&auth_user.user_id)
        .map_err(|_| ApiError::internal_error("Invalid user ID format"))?;

    let task_info = state.submit_task(task, creator_id).await?;

    Ok((StatusCode::CREATED, Json(task_info)))
}

/// Get task information
#[utoipa::path(
    get,
    path = "/api/v1/tasks/{task_id}",
    params(
        ("task_id" = String, Path, description = "Task ID")
    ),
    responses(
        (status = 200, description = "Task information", body = TaskInfo),
        (status = 404, description = "Task not found", body = ApiError)
    )
)]
async fn get_task(
    State(state): State<Arc<AppState>>,
    auth_user: auth::AuthUser,
    Path(task_id): Path<String>,
) -> ApiResult<Json<TaskInfo>> {
    let task = state
        .get_task(
            &task_id,
            Uuid::parse_str(&auth_user.user_id)
                .map_err(|_| ApiError::internal_error("Invalid user ID format"))?,
        )
        .await
        .ok_or_else(|| ApiError::not_found(format!("Task {} not found", task_id)))?;

    Ok(Json(task))
}

/// List all tasks
#[utoipa::path(
    get,
    path = "/api/v1/tasks",
    responses(
        (status = 200, description = "List of tasks", body = Vec<TaskInfo>)
    )
)]
async fn list_tasks(
    State(state): State<Arc<AppState>>,
    auth_user: auth::AuthUser,
) -> ApiResult<Json<Vec<TaskInfo>>> {
    let requester_id = Uuid::parse_str(&auth_user.user_id)
        .map_err(|_| ApiError::internal_error("Invalid user ID format"))?;
    let tasks = state.list_tasks(requester_id).await;
    Ok(Json(tasks))
}

/// Verify a ZK proof
#[utoipa::path(
    post,
    path = "/api/v1/proofs/verify",
    request_body = ProofVerificationRequest,
    responses(
        (status = 200, description = "Proof verification result", body = ProofVerificationResponse),
        (status = 400, description = "Invalid request", body = ApiError)
    )
)]
async fn verify_proof(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProofVerificationRequest>,
) -> ApiResult<Json<ProofVerificationResponse>> {
    // Validate request first
    request.validate()?;

    info!(
        "Verifying proof for task: {} (proof size: {} bytes)",
        request.task_id,
        request.proof_data.len()
    );

    let response = state.verify_proof(request).await?;

    if response.valid {
        info!(
            "Proof verified successfully for task: {} in {}ms",
            response.task_id, response.verification_time_ms
        );
    } else {
        info!(
            "Proof verification failed for task: {} - {}",
            response.task_id,
            response.error_message.as_deref().unwrap_or("unknown error")
        );
    }

    Ok(Json(response))
}

/// Get cluster statistics
#[utoipa::path(
    get,
    path = "/api/v1/cluster/stats",
    responses(
        (status = 200, description = "Cluster statistics", body = ClusterStats)
    )
)]
async fn get_cluster_stats(State(state): State<Arc<AppState>>) -> Json<ClusterStats> {
    let stats = state.get_cluster_stats().await;
    Json(stats)
}

/// Register a new user
#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered successfully"),
        (status = 400, description = "Invalid request", body = ApiError)
    )
)]
async fn register_user(
    State(state): State<Arc<AppState>>,
    Json(request): Json<auth::RegisterRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    request.validate()?;

    info!("Registering user: {}", request.username);

    let password_hash = auth::hash_password(&request.password)?;
    let api_key = auth::generate_api_key();
    let api_key_hash = auth::hash_api_key(&api_key);

    let user_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO users (username, password_hash, role, email)
        VALUES ($1, $2, $3, $4)
        RETURNING user_id
        "#,
    )
    .bind(&request.username)
    .bind(&password_hash)
    .bind("user")
    .bind(request.email.as_deref())
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.constraint().is_some() {
                return ApiError::conflict("Username already exists");
            }
        }
        ApiError::from(e)
    })?;

    sqlx::query(
        r#"
        INSERT INTO api_keys (user_id, key_hash, key_prefix, name, scopes)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(user_id)
    .bind(&api_key_hash)
    .bind(api_key.chars().take(8).collect::<String>())
    .bind("default")
    .bind(vec!["tasks:read".to_string(), "tasks:write".to_string()])
    .execute(&state.db)
    .await?;

    info!(
        "User registered successfully: {} ({})",
        request.username, user_id
    );

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "user_id": user_id.to_string(),
            "username": request.username,
            "email": request.email,
            "api_key": api_key,
            "message": "User registered successfully. Save your API key - it won't be shown again."
        })),
    ))
}

/// Login endpoint
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Invalid credentials", body = ApiError)
    )
)]
async fn login(
    State(state): State<Arc<AppState>>,
    Json(request): Json<auth::LoginRequest>,
) -> ApiResult<Json<auth::LoginResponse>> {
    // Validate login request
    request.validate()?;

    info!("Login attempt for user: {}", request.username);

    let user_row = sqlx::query(
        r#"
        SELECT user_id, username, password_hash, role
        FROM users
        WHERE username = $1
        "#,
    )
    .bind(&request.username)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::unauthorized("Invalid username or password"))?;

    let user_id: Uuid = user_row.get("user_id");
    let username: String = user_row.get("username");
    let password_hash: String = user_row.get("password_hash");
    let role: String = user_row.get("role");

    let password_valid =
        auth::verify_password_async(request.password.clone(), password_hash).await?;
    if !password_valid {
        return Err(ApiError::unauthorized("Invalid username or password"));
    }

    sqlx::query("UPDATE users SET last_login = NOW() WHERE user_id = $1")
        .bind(user_id)
        .execute(&state.db)
        .await?;

    let auth_config = auth::AuthConfig::from_env()?;
    let token = auth_config.generate_token(user_id.to_string(), username.clone(), role)?;

    // Generate refresh token
    let refresh_token = auth::generate_refresh_token();
    let refresh_token_hash = auth::hash_refresh_token(&refresh_token);
    let expires_at = chrono::Utc::now() + chrono::Duration::days(30); // 30 days

    // Store refresh token in database
    sqlx::query(
        r#"
        INSERT INTO refresh_tokens (user_id, token_hash, expires_at)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(user_id)
    .bind(&refresh_token_hash)
    .bind(expires_at)
    .execute(&state.db)
    .await?;

    info!("Login successful for user: {}", username);

    Ok(Json(auth::LoginResponse {
        access_token: token,
        refresh_token: Some(refresh_token),
        token_type: "Bearer".to_string(),
        expires_in: auth_config.jwt_expiration_hours * 3600,
    }))
}

/// Refresh token endpoint
#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "Token refreshed successfully", body = RefreshTokenResponse),
        (status = 401, description = "Invalid or expired refresh token", body = ApiError)
    )
)]
async fn refresh_token(
    State(state): State<Arc<AppState>>,
    Json(request): Json<auth::RefreshTokenRequest>,
) -> ApiResult<Json<auth::RefreshTokenResponse>> {
    let token_hash = auth::hash_refresh_token(&request.refresh_token);

    // Fetch refresh token from database
    let token_row = sqlx::query(
        r#"
        SELECT rt.user_id, rt.expires_at, rt.revoked_at, u.username, u.role
        FROM refresh_tokens rt
        JOIN users u ON rt.user_id = u.user_id
        WHERE rt.token_hash = $1
        "#,
    )
    .bind(&token_hash)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::unauthorized("Invalid refresh token"))?;

    let user_id: Uuid = token_row.get("user_id");
    let username: String = token_row.get("username");
    let role: String = token_row.get("role");
    let expires_at: chrono::DateTime<chrono::Utc> = token_row.get("expires_at");
    let revoked_at: Option<chrono::DateTime<chrono::Utc>> = token_row.get("revoked_at");

    // Check if token is revoked
    if revoked_at.is_some() {
        return Err(ApiError::unauthorized("Refresh token has been revoked"));
    }

    // Check if token is expired
    if expires_at < chrono::Utc::now() {
        return Err(ApiError::unauthorized("Refresh token has expired"));
    }

    // Revoke old refresh token
    sqlx::query(
        r#"
        UPDATE refresh_tokens 
        SET revoked_at = NOW(), revoked_reason = 'rotated'
        WHERE token_hash = $1
        "#,
    )
    .bind(&token_hash)
    .execute(&state.db)
    .await?;

    // Generate new JWT access token
    let auth_config = auth::AuthConfig::from_env()?;
    let access_token = auth_config.generate_token(user_id.to_string(), username, role)?;

    // Generate new refresh token
    let new_refresh_token = auth::generate_refresh_token();
    let new_token_hash = auth::hash_refresh_token(&new_refresh_token);
    let new_expires_at = chrono::Utc::now() + chrono::Duration::days(30);

    // Store new refresh token
    sqlx::query(
        r#"
        INSERT INTO refresh_tokens (user_id, token_hash, expires_at)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(user_id)
    .bind(&new_token_hash)
    .bind(new_expires_at)
    .execute(&state.db)
    .await?;

    info!("Token refreshed successfully for user_id: {}", user_id);

    Ok(Json(auth::RefreshTokenResponse {
        access_token,
        refresh_token: new_refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: auth_config.jwt_expiration_hours * 3600,
    }))
}

async fn validate_api_key(auth_user: auth::AuthUser) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "user_id": auth_user.user_id,
        "username": auth_user.username,
        "role": auth_user.role,
        "authenticated_via": "api_key"
    })))
}

async fn admin_users() -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(
        serde_json::json!({"message": "admin user management"}),
    ))
}

async fn admin_throttle_overrides() -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(
        serde_json::json!({"message": "admin global throttle overrides"}),
    ))
}

async fn admin_audit_log() -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"message": "admin audit logs"})))
}

/// Build the API router
pub fn create_router(state: Arc<AppState>) -> Router {
    let public_routes = Router::new()
        .route("/health", get(health_check))
        .route("/auth/register", post(register_user))
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh_token))
        .layer(axum_middleware::from_fn(
            middleware::auth::auth_config_middleware,
        ));

    let protected_routes = Router::new()
        .route("/nodes", post(register_node).get(list_nodes))
        .route("/nodes/:node_id", get(get_node).delete(delete_node))
        .route("/nodes/:node_id/reject", post(reject_node))
        .route("/nodes/:node_id/heartbeat", put(update_heartbeat))
        .route("/tasks", post(submit_task).get(list_tasks))
        .route("/tasks/:task_id", get(get_task))
        .route("/proofs/verify", post(verify_proof))
        .route("/cluster/stats", get(get_cluster_stats))
        .layer(axum_middleware::from_fn(
            middleware::auth::jwt_auth_middleware,
        ));

    let api_key_routes = Router::new()
        .route("/auth/api-key/validate", get(validate_api_key))
        .layer(axum_middleware::from_fn(
            middleware::auth::api_key_auth_middleware,
        ));

    let admin_routes = Router::new()
        .route("/admin/users", get(admin_users))
        .route("/admin/throttle-overrides", post(admin_throttle_overrides))
        .route("/admin/audit-log", get(admin_audit_log))
        .layer(axum_middleware::from_fn(
            middleware::auth::require_scope_middleware,
        ))
        .layer(axum_middleware::from_fn(
            middleware::auth::require_admin_middleware,
        ))
        .layer(axum_middleware::from_fn(
            middleware::auth::jwt_auth_middleware,
        ));

    let api_routes = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(api_key_routes)
        .merge(admin_routes);

    // Create OpenAPI JSON route (still using utoipa for spec generation)
    let openapi_json = utoipa::openapi::OpenApiBuilder::from(ApiDoc::openapi()).build();

    let docs_router = Router::new().route(
        "/api-docs/openapi.json",
        get(|| async { Json(openapi_json) }),
    );

    Router::new()
        .route("/", get(dashboard))
        .route("/swagger-ui", get(swagger_ui))
        .nest("/api/v1", api_routes)
        .merge(docs_router)
        .merge(middleware::metrics::create_metrics_router())
        .layer(axum_middleware::from_fn(
            middleware::metrics::metrics_middleware,
        ))
        .layer(axum_middleware::from_fn(
            middleware::logging::request_tracing_middleware,
        ))
        .layer(axum_middleware::from_fn(
            middleware::headers::security_headers_middleware,
        ))
        .layer(axum_middleware::from_fn(rate_limit::rate_limit_middleware))
        .layer(middleware::cors::create_cors_layer())
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        let response = health_check().await;
        assert_eq!(response.status, "healthy");
    }
}
