use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub mod models;
pub mod state;

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
) -> Result<(StatusCode, Json<NodeInfo>), ApiError> {
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
) -> Result<Json<NodeInfo>, ApiError> {
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
) -> Result<(StatusCode, Json<TaskInfo>), ApiError> {
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
) -> Result<Json<TaskInfo>, ApiError> {
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
) -> Result<Json<ProofVerificationResponse>, ApiError> {
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

/// Build the API router
pub fn create_router(state: Arc<AppState>) -> Router {
    // Create API routes
    let api_routes = Router::new()
        .route("/health", get(health_check))
        .route("/nodes", post(register_node).get(list_nodes))
        .route("/nodes/:node_id", get(get_node))
        .route("/tasks", post(submit_task).get(list_tasks))
        .route("/tasks/:task_id", get(get_task))
        .route("/proofs/verify", post(verify_proof))
        .route("/cluster/stats", get(get_cluster_stats));

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
