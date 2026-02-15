use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Health check response
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub timestamp: String,
}

/// Node registration request
#[derive(Debug, Deserialize, ToSchema)]
pub struct NodeRegistration {
    pub node_id: String,
    pub region: String,
    pub node_type: String,
    pub capabilities: NodeCapabilities,
}

/// Node capabilities
#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct NodeCapabilities {
    pub bandwidth_mbps: f64,
    pub cpu_cores: u32,
    pub memory_gb: f64,
    pub gpu_available: bool,
}

/// Node information
#[derive(Debug, Serialize, ToSchema, Clone)]
pub struct NodeInfo {
    pub node_id: String,
    pub region: String,
    pub node_type: String,
    pub capabilities: NodeCapabilities,
    pub health_score: f64,
    pub status: String,
    pub registered_at: String,
    pub last_seen: String,
}

/// Task submission request
#[derive(Debug, Deserialize, ToSchema)]
pub struct TaskSubmission {
    pub task_type: String,
    pub wasm_module: Option<String>, // Base64 encoded WASM module
    pub inputs: serde_json::Value,
    pub requirements: TaskRequirements,
}

/// Task requirements
#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct TaskRequirements {
    pub min_nodes: u32,
    pub max_execution_time_sec: u64,
    pub require_gpu: bool,
    pub require_proof: bool,
}

/// Task information
#[derive(Debug, Serialize, ToSchema, Clone)]
pub struct TaskInfo {
    pub task_id: String,
    pub task_type: String,
    pub status: TaskStatus,
    pub assigned_nodes: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub result: Option<serde_json::Value>,
    pub proof_id: Option<String>,
}

/// Task status
#[derive(Debug, Serialize, Deserialize, ToSchema, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Proof verification request
#[derive(Debug, Deserialize, ToSchema)]
pub struct ProofVerificationRequest {
    pub task_id: String,
    pub proof_data: String, // Base64 encoded proof
    pub public_inputs: String, // Base64 encoded public inputs
}

/// Proof verification response
#[derive(Debug, Serialize, ToSchema)]
pub struct ProofVerificationResponse {
    pub valid: bool,
    pub task_id: String,
    pub verified_at: String,
    pub verification_time_ms: u64,
}

/// Cluster statistics
#[derive(Debug, Serialize, ToSchema)]
pub struct ClusterStats {
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub avg_health_score: f64,
    pub total_compute_capacity: f64,
}

/// API Error
#[derive(Debug, Serialize, ToSchema)]
pub struct ApiError {
    pub error: String,
    pub message: String,
}

impl ApiError {
    pub fn new(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new("not_found", message)
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new("bad_request", message)
    }

    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new("internal_error", message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.error.as_str() {
            "not_found" => StatusCode::NOT_FOUND,
            "bad_request" => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, Json(self)).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::internal_error(err.to_string())
    }
}
