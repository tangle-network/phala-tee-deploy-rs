use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for deploying applications to the Phala TEE Cloud.
///
/// This struct contains all the parameters needed to create a deployment,
/// including API credentials, Docker Compose configuration, and environment variables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentConfig {
    /// Base URL for the Phala TEE Cloud API
    pub api_url: String,

    /// API key for authentication with the Phala Cloud API
    pub api_key: String,

    /// Docker Compose configuration as a string
    pub docker_compose: String,

    /// Environment variables to be securely encrypted and included in the deployment
    pub env_vars: HashMap<String, String>,

    /// ID of the TEEPod to deploy to
    pub teepod_id: u64,

    /// Docker image to deploy
    pub image: String,

    /// Optional custom VM configuration
    pub vm_config: Option<super::types::VmConfig>,
}

impl DeploymentConfig {
    /// Creates a new deployment configuration with default API URL.
    ///
    /// # Parameters
    ///
    /// * `api_key` - API key for authenticating with the Phala Cloud API
    /// * `docker_compose` - Docker Compose configuration as a string
    /// * `env_vars` - Environment variables to include in the deployment
    /// * `teepod_id` - ID of the TEEPod to deploy to
    /// * `image` - Docker image to deploy
    ///
    /// # Returns
    ///
    /// A new `DeploymentConfig` instance with the default API URL
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

    /// Sets a custom API URL for the Phala Cloud API.
    ///
    /// # Parameters
    ///
    /// * `api_url` - The custom API URL to use
    ///
    /// # Returns
    ///
    /// The updated `DeploymentConfig` instance for method chaining
    pub fn with_api_url(mut self, api_url: String) -> Self {
        self.api_url = api_url;
        self
    }

    /// Sets a custom VM configuration for the deployment.
    ///
    /// # Parameters
    ///
    /// * `vm_config` - The custom VM configuration to use
    ///
    /// # Returns
    ///
    /// The updated `DeploymentConfig` instance for method chaining
    pub fn with_vm_config(mut self, vm_config: super::types::VmConfig) -> Self {
        self.vm_config = Some(vm_config);
        self
    }
}
