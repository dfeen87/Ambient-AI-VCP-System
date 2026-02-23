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
    pub observability_port: Option<u16>,
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
        const VALID_NODE_TYPES: &[&str] = &[
            "compute",
            "gateway",
            "storage",
            "validator",
            "open_internet",
            "any",
            "feen_resonator",
        ];
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
    pub preferred_node_type: &'static str,
    pub minimum_capabilities: NodeCapabilities,
    pub max_execution_time_sec: u64,
    pub max_input_size_mb: usize,
    pub allow_wasm_module: bool,
}

pub const TASK_TYPE_REGISTRY: [TaskTypeRegistryEntry; 6] = [
    TaskTypeRegistryEntry {
        task_type: "federated_learning",
        preferred_node_type: "compute",
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
        preferred_node_type: "compute",
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
        preferred_node_type: "compute",
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
        preferred_node_type: "compute",
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
    TaskTypeRegistryEntry {
        task_type: "connect_only",
        preferred_node_type: "open_internet",
        minimum_capabilities: NodeCapabilities {
            bandwidth_mbps: 50.0,
            cpu_cores: 1,
            memory_gb: 1.0,
            gpu_available: false,
        },
        max_execution_time_sec: 3600,
        max_input_size_mb: 1,
        allow_wasm_module: false,
    },
    TaskTypeRegistryEntry {
        task_type: "feen_connectivity",
        preferred_node_type: "feen_resonator",
        minimum_capabilities: NodeCapabilities {
            bandwidth_mbps: 100.0,
            cpu_cores: 4,
            memory_gb: 4.0,
            gpu_available: false,
        },
        max_execution_time_sec: 600,
        max_input_size_mb: 5,
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
    pub owner_id: String,
    pub capabilities: NodeCapabilities,
    pub health_score: f64,
    pub status: String,
    pub registered_at: String,
    pub last_seen: String,
    pub observability_port: Option<u16>,
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

        if self.task_type == "connect_only" {
            validate_connect_only_inputs(&self.inputs)?;

            if self.requirements.min_nodes != 1 {
                return Err(ApiError::bad_request(
                    "connect_only tasks must set requirements.min_nodes to 1",
                ));
            }

            if self.requirements.require_gpu {
                return Err(ApiError::bad_request(
                    "connect_only tasks cannot require GPU",
                ));
            }

            if self.requirements.require_proof {
                return Err(ApiError::bad_request(
                    "connect_only tasks cannot require proof",
                ));
            }
        }

        if self.task_type == "feen_connectivity" {
            validate_feen_connectivity_inputs(&self.inputs)?;
        }

        // Validate requirements
        self.requirements.validate()?;

        Ok(())
    }
}

fn validate_connect_only_inputs(inputs: &serde_json::Value) -> Result<(), ApiError> {
    let obj = inputs
        .as_object()
        .ok_or_else(|| ApiError::bad_request("connect_only inputs must be a JSON object"))?;

    let allowed_keys = [
        "session_id",
        "requester_id",
        "duration_seconds",
        "bandwidth_limit_mbps",
        "egress_profile",
        "destination_policy_id",
    ];

    for key in obj.keys() {
        if !allowed_keys.contains(&key.as_str()) {
            return Err(ApiError::bad_request(format!(
                "connect_only inputs contains unsupported key: {}",
                key
            )));
        }
    }

    let session_id = obj
        .get("session_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ApiError::bad_request("connect_only inputs requires string field session_id")
        })?;
    if session_id.is_empty() || session_id.len() > 128 {
        return Err(ApiError::bad_request(
            "session_id must be between 1 and 128 characters",
        ));
    }

    let requester_id = obj
        .get("requester_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ApiError::bad_request("connect_only inputs requires string field requester_id")
        })?;
    if requester_id.is_empty() || requester_id.len() > 128 {
        return Err(ApiError::bad_request(
            "requester_id must be between 1 and 128 characters",
        ));
    }

    let duration_seconds = obj
        .get("duration_seconds")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| {
            ApiError::bad_request("connect_only inputs requires numeric field duration_seconds")
        })?;
    if duration_seconds == 0 || duration_seconds > 3600 {
        return Err(ApiError::bad_request(
            "duration_seconds must be between 1 and 3600",
        ));
    }

    let bandwidth_limit_mbps = obj
        .get("bandwidth_limit_mbps")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| {
            ApiError::bad_request("connect_only inputs requires numeric field bandwidth_limit_mbps")
        })?;
    if !(1.0..=10_000.0).contains(&bandwidth_limit_mbps) {
        return Err(ApiError::bad_request(
            "bandwidth_limit_mbps must be between 1 and 10,000",
        ));
    }

    let egress_profile = obj
        .get("egress_profile")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ApiError::bad_request("connect_only inputs requires string field egress_profile")
        })?;
    const VALID_EGRESS_PROFILES: &[&str] = &[
        "allowlist_domains",
        "protocol_limited",
        "metered_general_egress",
    ];
    if !VALID_EGRESS_PROFILES.contains(&egress_profile) {
        return Err(ApiError::bad_request(format!(
            "egress_profile must be one of: {}",
            VALID_EGRESS_PROFILES.join(", ")
        )));
    }

    let destination_policy_id = obj
        .get("destination_policy_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ApiError::bad_request("connect_only inputs requires string field destination_policy_id")
        })?;
    if destination_policy_id.is_empty() || destination_policy_id.len() > 128 {
        return Err(ApiError::bad_request(
            "destination_policy_id must be between 1 and 128 characters",
        ));
    }

    Ok(())
}

fn validate_feen_connectivity_inputs(inputs: &serde_json::Value) -> Result<(), ApiError> {
    let obj = inputs
        .as_object()
        .ok_or_else(|| ApiError::bad_request("feen_connectivity inputs must be a JSON object"))?;

    // Check for network definition or similar.
    // The prompt says: "Inputs: excitation parameters, coupling updates, simulation control (start/stop/step)."
    // And for the Task Type: "accept a structured description of two or more VCP nodes and their connection parameters"

    // Let's require a "nodes" list and "connections" list.
    if !obj.contains_key("nodes") {
        return Err(ApiError::bad_request(
            "feen_connectivity inputs requires 'nodes' list",
        ));
    }
    if !obj.contains_key("connections") {
        return Err(ApiError::bad_request(
            "feen_connectivity inputs requires 'connections' list",
        ));
    }

    Ok(())
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
    pub former_assigned_nodes: Vec<String>,
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

#[derive(Debug, Deserialize, ToSchema)]
pub struct ConnectSessionStartRequest {
    pub task_id: String,
    pub tunnel_protocol: Option<String>,
}

impl ConnectSessionStartRequest {
    pub fn validate(&self) -> Result<(), ApiError> {
        if self.task_id.is_empty() {
            return Err(ApiError::bad_request("task_id cannot be empty"));
        }

        if let Some(protocol) = &self.tunnel_protocol {
            const VALID_PROTOCOLS: &[&str] = &["mtls", "wireguard", "quic"];
            if !VALID_PROTOCOLS.contains(&protocol.as_str()) {
                return Err(ApiError::bad_request(format!(
                    "tunnel_protocol must be one of: {}",
                    VALID_PROTOCOLS.join(", ")
                )));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ConnectSessionStatus {
    Active,
    Ended,
    Expired,
}

#[derive(Debug, Serialize, ToSchema, Clone)]
pub struct ConnectSessionInfo {
    pub session_id: String,
    pub task_id: String,
    pub node_id: String,
    pub requester_id: String,
    pub tunnel_protocol: String,
    pub egress_profile: String,
    pub destination_policy_id: String,
    pub bandwidth_limit_mbps: f64,
    pub status: ConnectSessionStatus,
    /// `true` while the session is active and internet relay is on.
    /// Browsers and phones should use this field to determine whether the
    /// internet connection provided by this connect-only task is currently live.
    pub internet_active: bool,
    pub created_at: String,
    pub expires_at: String,
    pub last_heartbeat_at: Option<String>,
    pub ended_at: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ConnectSessionStartResponse {
    pub session: ConnectSessionInfo,
    pub session_token: String,
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

/// Node task result submission — sent by a node owner after the node has
/// finished executing its portion of a task.
///
/// For tasks that require proof (`require_proof = true`) the caller should
/// include `proof_data` and `public_inputs` (both Base64-encoded).  The
/// server verifies the proof before marking the task as completed.
#[derive(Debug, Deserialize, ToSchema)]
pub struct NodeTaskResult {
    /// ID of the node that performed the work.
    pub node_id: String,
    /// Execution output produced by the node (arbitrary JSON).
    pub result: serde_json::Value,
    /// Optional wall-clock execution time reported by the node (milliseconds).
    pub execution_time_ms: Option<u64>,
    /// Base64-encoded ZK proof blob (required when the task has `require_proof`).
    pub proof_data: Option<String>,
    /// Base64-encoded public inputs for the proof circuit.
    pub public_inputs: Option<String>,
    /// Circuit identifier; defaults to `"default"` when omitted.
    pub circuit_id: Option<String>,
}

impl NodeTaskResult {
    pub fn validate(&self) -> Result<(), ApiError> {
        if self.node_id.is_empty() {
            return Err(ApiError::bad_request("node_id cannot be empty"));
        }
        if self.node_id.len() > 64 {
            return Err(ApiError::bad_request("node_id cannot exceed 64 characters"));
        }

        if let Some(ref proof_data) = self.proof_data {
            if proof_data.len() > 100_000 {
                return Err(ApiError::bad_request(
                    "proof_data exceeds maximum size of 100KB (base64 encoded)",
                ));
            }
            if base64::Engine::decode(&base64::engine::general_purpose::STANDARD, proof_data)
                .is_err()
            {
                return Err(ApiError::bad_request("proof_data is not valid base64"));
            }
        }

        if let Some(ref public_inputs) = self.public_inputs {
            if public_inputs.len() > 10_000 {
                return Err(ApiError::bad_request(
                    "public_inputs exceeds maximum size of 10KB (base64 encoded)",
                ));
            }
            if base64::Engine::decode(&base64::engine::general_purpose::STANDARD, public_inputs)
                .is_err()
            {
                return Err(ApiError::bad_request("public_inputs is not valid base64"));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_session_start_request_accepts_supported_protocols() {
        let request = ConnectSessionStartRequest {
            task_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            tunnel_protocol: Some("wireguard".to_string()),
        };

        assert!(request.validate().is_ok());
    }

    #[test]
    fn connect_session_start_request_rejects_unsupported_protocols() {
        let request = ConnectSessionStartRequest {
            task_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            tunnel_protocol: Some("ipsec".to_string()),
        };

        assert!(request.validate().is_err());
    }
}
