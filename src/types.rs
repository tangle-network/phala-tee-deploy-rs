use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Docker registry authentication configuration.
///
/// Used to access private Docker registries when deploying containers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerConfig {
    /// Docker registry username
    pub username: String,

    /// Docker registry password
    pub password: String,

    /// Optional custom registry URL
    pub registry: Option<String>,
}

/// Advanced features configuration for TEE deployments.
///
/// Controls security and visibility settings for deployed applications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedFeatures {
    /// Enable transparent proxy support
    pub tproxy: bool,

    /// Enable Key Management System integration
    pub kms: bool,

    /// Make system information publicly accessible
    pub public_sys_info: bool,

    /// Make application logs publicly accessible
    pub public_logs: bool,

    /// Docker registry authentication settings
    pub docker_config: DockerConfig,

    /// List this deployment in public directories
    pub listed: bool,
}

/// Docker Compose manifest configuration.
///
/// Defines the application structure using Docker Compose format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeManifest {
    /// Name of the application
    pub name: String,

    /// Enabled features for this deployment
    pub features: Vec<String>,

    /// Docker Compose file content
    pub docker_compose_file: String,
}

/// Virtual Machine configuration for a TEE deployment.
///
/// Defines the resources and settings for the VM that will run the containerized application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    /// Name of the deployment
    pub name: String,

    /// Docker Compose manifest configuration
    pub compose_manifest: ComposeManifest,

    /// Number of virtual CPU cores
    pub vcpu: u32,

    /// Memory allocation in MB
    pub memory: u32,

    /// Disk size allocation in GB
    pub disk_size: u32,

    /// ID of the TEEPod to deploy to
    pub teepod_id: u64,

    /// Container image to use
    pub image: String,

    /// Advanced features configuration
    pub advanced_features: AdvancedFeatures,
}

/// Encrypted environment variable entry.
///
/// Used for secure transmission of sensitive environment variables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedEnv {
    /// Environment variable name
    pub key: String,

    /// Encrypted environment variable value
    pub value: String,
}

/// Response from a deployment operation.
///
/// Contains information about the created deployment including its ID and status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentResponse {
    /// Unique identifier for the deployment
    pub id: u64,

    /// Current status of the deployment (e.g., "pending", "running")
    pub status: String,

    /// Additional deployment details as key-value pairs
    pub details: Option<HashMap<String, serde_json::Value>>,
}

/// Response when retrieving a compose configuration.
///
/// Contains both the compose configuration and the public key needed for
/// encrypting environment variables for updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeResponse {
    /// The compose file configuration
    pub compose_file: serde_json::Value,

    /// Public key for encrypting environment variables
    pub env_pubkey: String,
}
