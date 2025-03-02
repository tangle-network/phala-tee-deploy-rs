use reqwest::Client;
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;

use crate::{
    config::DeploymentConfig,
    crypto::Encryptor,
    error::Error,
    types::{ComposeResponse, DeploymentResponse, VmConfig},
};

/// Client for interacting with the TEE deployment API
pub struct TeeClient {
    client: Client,
    config: DeploymentConfig,
}

impl TeeClient {
    /// Create a new TeeClient with the given configuration
    pub fn new(config: DeploymentConfig) -> Result<Self, Error> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(Error::HttpClient)?;

        Ok(Self { client, config })
    }

    /// Deploy a container to the TEE environment
    pub async fn deploy(&self) -> Result<DeploymentResponse, Error> {
        // Get or create VM configuration
        let vm_config = self.config.vm_config.clone().unwrap_or_else(|| VmConfig {
            name: format!("tee-deploy-{}", uuid::Uuid::new_v4()),
            compose_manifest: crate::types::ComposeManifest {
                name: "tee-deployment".to_string(),
                features: vec!["kms".to_string(), "tproxy-net".to_string()],
                docker_compose_file: self.config.docker_compose.clone(),
            },
            vcpu: 2,
            memory: 8192,
            disk_size: 40,
            teepod_id: self.config.teepod_id,
            image: self.config.image.clone(),
            advanced_features: crate::types::AdvancedFeatures {
                tproxy: true,
                kms: true,
                public_sys_info: true,
                public_logs: true,
                docker_config: crate::types::DockerConfig {
                    username: String::new(),
                    password: String::new(),
                    registry: None,
                },
                listed: false,
            },
        });

        // Get encryption public key
        let pubkey_response = self.get_pubkey(&vm_config).await?;

        // Encrypt environment variables
        let encryptor = Encryptor::new();
        let env_vars: Vec<_> = self
            .config
            .env_vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        let encrypted_env = encryptor.encrypt_env_vars(
            &env_vars,
            &pubkey_response["app_env_encrypt_pubkey"]
                .as_str()
                .ok_or_else(|| Error::Api {
                    status_code: 500,
                    message: "Missing encryption public key".into(),
                })?,
        )?;

        // Create deployment
        let response = self
            .client
            .post(format!(
                "{}/cvms/from_cvm_configuration",
                self.config.api_url
            ))
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.config.api_key)
            .json(&json!({
                "vm_config": vm_config,
                "encrypted_env": encrypted_env,
                "app_env_encrypt_pubkey": pubkey_response["app_env_encrypt_pubkey"],
                "app_id_salt": pubkey_response["app_id_salt"],
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Api {
                status_code: response.status().as_u16(),
                message: response.text().await?,
            });
        }

        response
            .json::<DeploymentResponse>()
            .await
            .map_err(Error::HttpClient)
    }

    async fn get_pubkey(&self, vm_config: &VmConfig) -> Result<serde_json::Value, Error> {
        let response = self
            .client
            .post(format!(
                "{}/cvms/pubkey/from_cvm_configuration",
                self.config.api_url
            ))
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.config.api_key)
            .json(&vm_config)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Api {
                status_code: response.status().as_u16(),
                message: response.text().await?,
            });
        }

        response.json().await.map_err(Error::HttpClient)
    }

    /// Get the current compose configuration for an app
    pub async fn get_compose(&self, app_id: &str) -> Result<ComposeResponse, Error> {
        let response = self
            .client
            .get(format!("{}/cvms/{}/compose", self.config.api_url, app_id))
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.config.api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Api {
                status_code: response.status().as_u16(),
                message: response.text().await?,
            });
        }

        response
            .json::<ComposeResponse>()
            .await
            .map_err(Error::HttpClient)
    }

    /// Update the compose configuration for an app
    pub async fn update_compose(
        &self,
        app_id: &str,
        compose_file: serde_json::Value,
        env_vars: Option<HashMap<String, String>>,
        env_pubkey: String,
    ) -> Result<serde_json::Value, Error> {
        let mut body = json!({
            "compose_manifest": compose_file
        });

        // Encrypt environment variables if provided
        if let Some(vars) = env_vars {
            let env_vars: Vec<_> = vars.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            let encryptor = Encryptor::new();
            let encrypted_env = encryptor.encrypt_env_vars(&env_vars, &env_pubkey)?;
            body["encrypted_env"] = json!(encrypted_env);
        }

        let response = self
            .client
            .put(format!("{}/cvms/{}/compose", self.config.api_url, app_id))
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.config.api_key)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Api {
                status_code: response.status().as_u16(),
                message: response.text().await?,
            });
        }

        response.json().await.map_err(Error::HttpClient)
    }

    /// Get available TEEPods
    pub async fn get_available_teepods(&self) -> Result<serde_json::Value, Error> {
        let response = self
            .client
            .get(format!("{}/teepods/available", self.config.api_url))
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.config.api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Api {
                status_code: response.status().as_u16(),
                message: response.text().await?,
            });
        }

        response.json().await.map_err(Error::HttpClient)
    }

    /// Get the public key for a VM configuration
    pub async fn get_pubkey_for_config(
        &self,
        vm_config: &serde_json::Value,
    ) -> Result<serde_json::Value, Error> {
        let response = self
            .client
            .post(format!(
                "{}/cvms/pubkey/from_cvm_configuration",
                self.config.api_url
            ))
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.config.api_key)
            .json(&vm_config)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Api {
                status_code: response.status().as_u16(),
                message: response.text().await?,
            });
        }

        response.json().await.map_err(Error::HttpClient)
    }

    /// Deploy a container with a custom VM configuration
    pub async fn deploy_with_config(
        &self,
        vm_config: serde_json::Value,
        env_vars: &[(String, String)],
        app_env_encrypt_pubkey: &str,
        app_id_salt: &str,
    ) -> Result<DeploymentResponse, Error> {
        // Encrypt environment variables
        let encryptor = Encryptor::new();
        let encrypted_env = encryptor.encrypt_env_vars(env_vars, app_env_encrypt_pubkey)?;

        // Create deployment
        let response = self
            .client
            .post(format!(
                "{}/cvms/from_cvm_configuration",
                self.config.api_url
            ))
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.config.api_key)
            .json(&json!({
                "vm_config": vm_config,
                "encrypted_env": encrypted_env,
                "app_env_encrypt_pubkey": app_env_encrypt_pubkey,
                "app_id_salt": app_id_salt,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Api {
                status_code: response.status().as_u16(),
                message: response.text().await?,
            });
        }

        response
            .json::<DeploymentResponse>()
            .await
            .map_err(Error::HttpClient)
    }
}
