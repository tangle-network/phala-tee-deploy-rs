use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerConfig {
    pub username: String,
    pub password: String,
    pub registry: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedFeatures {
    pub tproxy: bool,
    pub kms: bool,
    pub public_sys_info: bool,
    pub public_logs: bool,
    pub docker_config: DockerConfig,
    pub listed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeManifest {
    pub name: String,
    pub features: Vec<String>,
    pub docker_compose_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    pub name: String,
    pub compose_manifest: ComposeManifest,
    pub vcpu: u32,
    pub memory: u32,
    pub disk_size: u32,
    pub teepod_id: u64,
    pub image: String,
    pub advanced_features: AdvancedFeatures,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedEnv {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentResponse {
    pub id: String,
    pub status: String,
    pub details: Option<HashMap<String, serde_json::Value>>,
}

/// Response when retrieving a compose configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeResponse {
    /// The compose file configuration
    pub compose_file: serde_json::Value,
    /// Public key for encrypting environment variables
    pub env_pubkey: String,
}
