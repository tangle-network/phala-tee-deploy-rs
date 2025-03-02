use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentConfig {
    /// Base URL for the TEE cloud API
    pub api_url: String,

    /// API key for authentication
    pub api_key: String,

    /// Docker compose configuration
    pub docker_compose: String,

    /// Environment variables to encrypt
    pub env_vars: HashMap<String, String>,

    /// TEE pod ID
    pub teepod_id: u64,

    /// Docker image to deploy
    pub image: String,

    /// VM configuration
    pub vm_config: Option<super::types::VmConfig>,
}

impl DeploymentConfig {
    pub fn new(
        api_key: String,
        docker_compose: String,
        env_vars: HashMap<String, String>,
        teepod_id: u64,
        image: String,
    ) -> Self {
        Self {
            api_url: "https://cloud-api.phala.network/api/v1".to_string(),
            api_key,
            docker_compose,
            env_vars,
            teepod_id,
            image,
            vm_config: None,
        }
    }

    pub fn with_api_url(mut self, api_url: String) -> Self {
        self.api_url = api_url;
        self
    }

    pub fn with_vm_config(mut self, vm_config: super::types::VmConfig) -> Self {
        self.vm_config = Some(vm_config);
        self
    }
}
