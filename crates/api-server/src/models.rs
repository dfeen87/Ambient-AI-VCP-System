use crate::error::ApiError;
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

impl NodeRegistration {
    /// Validate node registration data
    pub fn validate(&self) -> Result<(), ApiError> {
        // Validate node_id
        if self.node_id.is_empty() {
            return Err(ApiError::bad_request("node_id cannot be empty"));
        }
        if self.node_id.len() > 64 {
            return Err(ApiError::bad_request("node_id cannot exceed 64 characters"));
        }
        if !self
            .node_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(ApiError::bad_request(
                "node_id can only contain alphanumeric characters, hyphens, and underscores",
            ));
        }

        // Validate region
        if self.region.is_empty() {
            return Err(ApiError::bad_request("region cannot be empty"));
        }
        if self.region.len() > 32 {
            return Err(ApiError::bad_request("region cannot exceed 32 characters"));
        }

        // Validate node_type
        const VALID_NODE_TYPES: &[&str] = &["compute", "gateway", "storage", "validator", "any"];
        if !VALID_NODE_TYPES.contains(&self.node_type.as_str()) {
            return Err(ApiError::bad_request(format!(
                "node_type must be one of: {}",
                VALID_NODE_TYPES.join(", ")
            )));
        }

        // Validate capabilities
        self.capabilities.validate()?;

        Ok(())
    }
}

/// Node capabilities
#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct NodeCapabilities {
    pub bandwidth_mbps: f64,
    pub cpu_cores: u32,
    pub memory_gb: f64,
    pub gpu_available: bool,
}

impl NodeCapabilities {
    /// Validate node capabilities against capability whitelist constraints
    pub fn validate(&self) -> Result<(), ApiError> {
        // Validate bandwidth (10-100,000 Mbps)
        if !(10.0..=100_000.0).contains(&self.bandwidth_mbps) {
            return Err(ApiError::bad_request(
                "bandwidth_mbps must be between 10 and 100,000",
            ));
        }

        // Validate CPU cores (1-256 cores)
        if !(1..=256).contains(&self.cpu_cores) {
            return Err(ApiError::bad_request("cpu_cores must be between 1 and 256"));
        }

        // Validate memory (1-2,048 GB)
        if !(1.0..=2_048.0).contains(&self.memory_gb) {
            return Err(ApiError::bad_request(
                "memory_gb must be between 1 and 2,048",
            ));
        }

        Ok(())
    }
}

/// Task type registry entry used to validate task submissions and scheduling feasibility.
#[derive(Debug, Clone)]
pub struct TaskTypeRegistryEntry {
    pub task_type: &'static str,
    pub minimum_capabilities: NodeCapabilities,
    pub max_execution_time_sec: u64,
    pub max_input_size_mb: usize,
    pub allow_wasm_module: bool,
}

pub const TASK_TYPE_REGISTRY: [TaskTypeRegistryEntry; 4] = [
    TaskTypeRegistryEntry {
        task_type: "federated_learning",
        minimum_capabilities: NodeCapabilities {
            bandwidth_mbps: 500.0,
            cpu_cores: 8,
            memory_gb: 32.0,
            gpu_available: false,
        },
        max_execution_time_sec: 3600,
        max_input_size_mb: 50,
        allow_wasm_module: false,
    },
    TaskTypeRegistryEntry {
        task_type: "zk_proof",
        minimum_capabilities: NodeCapabilities {
            bandwidth_mbps: 100.0,
            cpu_cores: 8,
            memory_gb: 16.0,
            gpu_available: false,
        },
        max_execution_time_sec: 1800,
        max_input_size_mb: 25,
        allow_wasm_module: false,
    },
    TaskTypeRegistryEntry {
        task_type: "wasm_execution",
        minimum_capabilities: NodeCapabilities {
            bandwidth_mbps: 100.0,
            cpu_cores: 4,
            memory_gb: 8.0,
            gpu_available: false,
        },
        max_execution_time_sec: 900,
        max_input_size_mb: 10,
        allow_wasm_module: true,
    },
    TaskTypeRegistryEntry {
        task_type: "computation",
        minimum_capabilities: NodeCapabilities {
            bandwidth_mbps: 50.0,
            cpu_cores: 4,
            memory_gb: 8.0,
            gpu_available: false,
        },
        max_execution_time_sec: 1800,
        max_input_size_mb: 20,
        allow_wasm_module: false,
    },
];

pub fn task_type_registry_entry(task_type: &str) -> Option<&'static TaskTypeRegistryEntry> {
    TASK_TYPE_REGISTRY
        .iter()
        .find(|entry| entry.task_type == task_type)
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

impl TaskSubmission {
    /// Validate task submission data
    pub fn validate(&self) -> Result<(), ApiError> {
        // Validate task_type against centralized registry
        let task_type_entry = task_type_registry_entry(&self.task_type).ok_or_else(|| {
            ApiError::bad_request(format!(
                "task_type must be one of: {}",
                TASK_TYPE_REGISTRY
                    .iter()
                    .map(|entry| entry.task_type)
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        })?;

        // Validate WASM module policy and size if provided
        if let Some(ref module) = self.wasm_module {
            if !task_type_entry.allow_wasm_module {
                return Err(ApiError::bad_request(format!(
                    "wasm_module is not allowed for task_type {}",
                    self.task_type
                )));
            }

            if module.len() > task_type_entry.max_input_size_mb * 1024 * 1024 {
                return Err(ApiError::bad_request(format!(
                    "wasm_module cannot exceed {}MB for task_type {}",
                    task_type_entry.max_input_size_mb, self.task_type
                )));
            }
        }

        if self.requirements.max_execution_time_sec > task_type_entry.max_execution_time_sec {
            return Err(ApiError::bad_request(format!(
                "max_execution_time_sec cannot exceed {} for task_type {}",
                task_type_entry.max_execution_time_sec, self.task_type
            )));
        }

        // Deep validate arbitrary JSON payloads.
        validate_json_depth(&self.inputs, 0)?;

        // Validate requirements
        self.requirements.validate()?;

        Ok(())
    }
}

fn validate_json_depth(value: &serde_json::Value, depth: usize) -> Result<(), ApiError> {
    const MAX_DEPTH: usize = 16;
    const MAX_ARRAY_ITEMS: usize = 1_000;
    const MAX_OBJECT_KEYS: usize = 1_000;

    if depth > MAX_DEPTH {
        return Err(ApiError::bad_request(
            "inputs JSON exceeds maximum nesting depth",
        ));
    }

    match value {
        serde_json::Value::Array(items) => {
            if items.len() > MAX_ARRAY_ITEMS {
                return Err(ApiError::bad_request("inputs JSON array too large"));
            }
            for item in items {
                validate_json_depth(item, depth + 1)?;
            }
        }
        serde_json::Value::Object(map) => {
            if map.len() > MAX_OBJECT_KEYS {
                return Err(ApiError::bad_request("inputs JSON object too large"));
            }
            for (k, v) in map {
                if k.len() > 256 {
                    return Err(ApiError::bad_request("inputs JSON key too long"));
                }
                validate_json_depth(v, depth + 1)?;
            }
        }
        serde_json::Value::String(s) if s.len() > 10_000 => {
            return Err(ApiError::bad_request("inputs JSON string value too large"));
        }
        _ => {}
    }

    Ok(())
}

/// Task requirements
#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct TaskRequirements {
    pub min_nodes: u32,
    pub max_execution_time_sec: u64,
    pub require_gpu: bool,
    pub require_proof: bool,
}

impl TaskRequirements {
    /// Validate task requirements
    pub fn validate(&self) -> Result<(), ApiError> {
        // Validate min_nodes (1-1000)
        if self.min_nodes == 0 || self.min_nodes > 1000 {
            return Err(ApiError::bad_request(
                "min_nodes must be between 1 and 1000",
            ));
        }

        // Validate max_execution_time (1 second to 1 hour)
        if self.max_execution_time_sec == 0 || self.max_execution_time_sec > 3600 {
            return Err(ApiError::bad_request(
                "max_execution_time_sec must be between 1 and 3600",
            ));
        }

        Ok(())
    }
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
    pub proof_data: String,         // Base64 encoded proof
    pub public_inputs: String,      // Base64 encoded public inputs
    pub circuit_id: Option<String>, // Optional circuit identifier
}

impl ProofVerificationRequest {
    /// Validate proof verification request
    pub fn validate(&self) -> Result<(), ApiError> {
        // Validate task_id
        if self.task_id.is_empty() {
            return Err(ApiError::bad_request("task_id cannot be empty"));
        }

        // Validate proof_data size (base64 encoded)
        if self.proof_data.len() > 100_000 {
            // ~75KB max proof size
            return Err(ApiError::bad_request(
                "proof_data exceeds maximum size of 100KB (base64 encoded)",
            ));
        }

        // Validate public_inputs size (base64 encoded)
        if self.public_inputs.len() > 10_000 {
            // ~7.5KB max public inputs
            return Err(ApiError::bad_request(
                "public_inputs exceeds maximum size of 10KB (base64 encoded)",
            ));
        }

        // Validate base64 encoding
        if base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &self.proof_data)
            .is_err()
        {
            return Err(ApiError::bad_request("proof_data is not valid base64"));
        }

        if base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &self.public_inputs,
        )
        .is_err()
        {
            return Err(ApiError::bad_request("public_inputs is not valid base64"));
        }

        Ok(())
    }

    /// Decode proof data from base64
    pub fn decode_proof_data(&self) -> Result<Vec<u8>, ApiError> {
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &self.proof_data)
            .map_err(|_| ApiError::bad_request("Failed to decode proof_data"))
    }

    /// Decode public inputs from base64
    pub fn decode_public_inputs(&self) -> Result<Vec<u8>, ApiError> {
        base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &self.public_inputs,
        )
        .map_err(|_| ApiError::bad_request("Failed to decode public_inputs"))
    }
}

/// Proof verification response
#[derive(Debug, Serialize, ToSchema)]
pub struct ProofVerificationResponse {
    pub valid: bool,
    pub task_id: String,
    pub verified_at: String,
    pub verification_time_ms: u64,
    pub error_message: Option<String>,
}

/// Cluster statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ClusterStats {
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub avg_health_score: f64,
    pub total_compute_capacity: f64,
}
