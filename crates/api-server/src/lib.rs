use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use sqlx::Row;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;

pub mod auth;
pub mod db;
pub mod error;
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
        submit_task,
        get_task,
        list_tasks,
        verify_proof,
        get_cluster_stats,
        register_user,
        login,
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
    ))
)]
struct ApiDoc;

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
        (status = 400, description = "Invalid request", body = ApiError)
    )
)]
async fn register_node(
    State(state): State<Arc<AppState>>,
    Json(registration): Json<NodeRegistration>,
) -> ApiResult<(StatusCode, Json<NodeInfo>)> {
    // Validate input
    registration.validate()?;

    info!(
        "Registering node: {} in region {}",
        registration.node_id, registration.region
    );

    let node_info = state.register_node(registration).await?;

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
    Json(task): Json<TaskSubmission>,
) -> ApiResult<(StatusCode, Json<TaskInfo>)> {
    // Validate input
    task.validate()?;

    info!("Submitting task: {}", task.task_type);

    let task_info = state.submit_task(task).await?;

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
    Path(task_id): Path<String>,
) -> ApiResult<Json<TaskInfo>> {
    let task = state
        .get_task(&task_id)
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
async fn list_tasks(State(state): State<Arc<AppState>>) -> Json<Vec<TaskInfo>> {
    let tasks = state.list_tasks().await;
    Json(tasks)
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
    info!("Verifying proof for task: {}", request.task_id);

    let response = state.verify_proof(request).await?;

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
    // Validate request
    request.validate()?;

    info!("Registering user: {}", request.username);

    // Hash the password
    let password_hash = auth::hash_password(&request.password)?;

    // Generate API key
    let api_key = auth::generate_api_key();

    // Insert user into database
    let user_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO users (username, password_hash, api_key, role)
        VALUES ($1, $2, $3, $4)
        RETURNING user_id
        "#,
    )
    .bind(&request.username)
    .bind(&password_hash)
    .bind(&api_key)
    .bind("user")
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

    info!("User registered successfully: {} ({})", request.username, user_id);

    Ok((StatusCode::CREATED, Json(serde_json::json!({
        "user_id": user_id.to_string(),
        "username": request.username,
        "api_key": api_key,
        "message": "User registered successfully. Save your API key - it won't be shown again."
    }))))
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
    info!("Login attempt for user: {}", request.username);

    // Get user from database
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

    // Verify password
    let password_valid = auth::verify_password(&request.password, &password_hash)?;
    if !password_valid {
        return Err(ApiError::unauthorized("Invalid username or password"));
    }

    // Update last login
    sqlx::query("UPDATE users SET last_login = NOW() WHERE user_id = $1")
        .bind(user_id)
        .execute(&state.db)
        .await?;

    // Get auth config from environment
    let auth_config = auth::AuthConfig::from_env()?;

    // Generate JWT token
    let token = auth_config.generate_token(
        user_id.to_string(),
        username.clone(),
        role,
    )?;

    info!("Login successful for user: {}", username);

    Ok(Json(auth::LoginResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
        expires_in: auth_config.jwt_expiration_hours * 3600,
    }))
}

/// Build the API router
pub fn create_router(state: Arc<AppState>) -> Router {
    // Create public API routes (no authentication required)
    let public_routes = Router::new()
        .route("/health", get(health_check))
        .route("/auth/register", post(register_user))
        .route("/auth/login", post(login));

    // Create protected API routes (authentication required)
    let protected_routes = Router::new()
        .route("/nodes", post(register_node).get(list_nodes))
        .route("/nodes/:node_id", get(get_node))
        .route("/tasks", post(submit_task).get(list_tasks))
        .route("/tasks/:task_id", get(get_task))
        .route("/proofs/verify", post(verify_proof))
        .route("/cluster/stats", get(get_cluster_stats));

    // Combine routes
    let api_routes = Router::new()
        .merge(public_routes)
        .merge(protected_routes);

    // Create main router with API prefix
    Router::new()
        .nest("/api/v1", api_routes)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(CorsLayer::permissive())
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
