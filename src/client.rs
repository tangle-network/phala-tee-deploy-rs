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

/// Client for interacting with the Phala TEE Cloud API.
///
/// `TeeClient` provides low-level access to the Phala Cloud API for deploying
/// and managing containerized applications in a Trusted Execution Environment (TEE).
/// This client handles authentication, API requests, and encryption of sensitive data.
///
/// For most use cases, consider using the higher-level `TeeDeployer` API instead,
/// which provides a more ergonomic interface built on top of this client.
///
/// # Features
///
/// * Direct API access to the Phala TEE Cloud
/// * Secure environment variable encryption
/// * TEEPod discovery and selection
/// * Application deployment and management
pub struct TeeClient {
    client: Client,
    config: DeploymentConfig,
}

impl TeeClient {
    /// Creates a new `TeeClient` with the given configuration.
    ///
    /// # Parameters
    ///
    /// * `config` - The deployment configuration including API credentials and default settings
    ///
    /// # Returns
    ///
    /// A new `TeeClient` instance if successful
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be created
    pub fn new(config: DeploymentConfig) -> Result<Self, Error> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(Error::HttpClient)?;

        Ok(Self { client, config })
    }

    /// Deploys a container to the TEE environment using the client's configuration.
    ///
    /// This method uses the configuration set during client creation to deploy
    /// an application. It handles VM configuration, encryption, and API communication.
    ///
    /// # Returns
    ///
    /// A `DeploymentResponse` containing the deployment details if successful
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The API request fails
    /// * Environment variables cannot be encrypted
    /// * The API returns an error response
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
        let env_vars: Vec<_> = self
            .config
            .env_vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        let encrypted_env = Encryptor::encrypt_env_vars(
            &env_vars,
            &pubkey_response["app_env_encrypt_pubkey"]
                .as_str()
                .ok_or_else(|| Error::Api {
                    status_code: 500,
                    message: "Missing encryption public key".into(),
                })?,
        )?;

        // Create a mutable request body from vm_config
        let mut request_body = serde_json::to_value(&vm_config)
            .unwrap()
            .as_object()
            .cloned()
            .unwrap_or_default();

        // Add the additional fields
        request_body.insert(
            "encrypted_env".to_string(),
            serde_json::Value::String(encrypted_env),
        );
        request_body.insert(
            "app_env_encrypt_pubkey".to_string(),
            pubkey_response["app_env_encrypt_pubkey"].clone(),
        );
        request_body.insert(
            "app_id_salt".to_string(),
            pubkey_response["app_id_salt"].clone(),
        );

        // Create deployment
        let response = self
            .client
            .post(format!(
                "{}/cvms/from_cvm_configuration",
                self.config.api_url
            ))
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.config.api_key)
            .json(&request_body)
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

    /// Retrieves the encryption public key for a given VM configuration.
    ///
    /// This is a helper method used internally to get the public key needed
    /// for securely encrypting environment variables.
    ///
    /// # Parameters
    ///
    /// * `vm_config` - The VM configuration to get a public key for
    ///
    /// # Returns
    ///
    /// A JSON value containing the public key and salt if successful
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails or returns an error
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

    /// Retrieves the current Docker Compose configuration for an application.
    ///
    /// # Parameters
    ///
    /// * `app_id` - The ID of the application to get the configuration for
    ///
    /// # Returns
    ///
    /// A `ComposeResponse` containing the compose file and encryption public key
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails or the application is not found
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

    /// Updates the Docker Compose configuration for an existing application.
    ///
    /// This method can update both the application configuration and its
    /// environment variables.
    ///
    /// # Parameters
    ///
    /// * `app_id` - The ID of the application to update
    /// * `compose_file` - The new Docker Compose configuration
    /// * `env_vars` - Optional new environment variables
    /// * `env_pubkey` - The public key for encrypting environment variables
    ///
    /// # Returns
    ///
    /// A JSON value containing the update operation result
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The API request fails
    /// * The application is not found
    /// * Environment variables cannot be encrypted
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
            let encrypted_env = Encryptor::encrypt_env_vars(&env_vars, &env_pubkey)?;
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

    /// Retrieves a list of available TEEPods from the Phala Cloud API.
    ///
    /// This method queries the API for TEEPods that are available for deployment,
    /// providing detailed diagnostics for any connection issues.
    ///
    /// # Returns
    ///
    /// A JSON value containing the list of available TEEPods if successful
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The network request fails (timeout, connection issues, etc.)
    /// * The API returns an error response
    /// * The response cannot be parsed as valid JSON
    pub async fn get_available_teepods(&self) -> Result<serde_json::Value, Error> {
        // Construct request URL
        let request_url = format!("{}/teepods/available", self.config.api_url);

        // Build request with explicit timeouts
        let request = self
            .client
            .get(&request_url)
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.config.api_key)
            .timeout(std::time::Duration::from_secs(15)); // Extend timeout for slow networks

        // Log request and execute
        eprintln!("Requesting TEEPods from: {}", request_url);

        let response = match request.send().await {
            Ok(resp) => resp,
            Err(err) => {
                let err_msg = format!("Network error while fetching TEEPods: {}", err);
                eprintln!("{}", err_msg);

                if err.is_timeout() {
                    eprintln!("Request timed out - the server may be under high load or your network connection may be slow");
                } else if err.is_connect() {
                    eprintln!(
                        "Connection error - please check your network connection and API endpoint"
                    );
                }

                return Err(Error::HttpClient(err));
            }
        };

        // Check response status
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read error response".to_string());

            eprintln!("API error (status {}): {}", status.as_u16(), error_text);

            return Err(Error::Api {
                status_code: status.as_u16(),
                message: error_text,
            });
        }

        // Parse response
        match response.json().await {
            Ok(json) => Ok(json),
            Err(err) => {
                eprintln!("Failed to parse API response: {}", err);
                Err(Error::HttpClient(err))
            }
        }
    }

    /// Retrieves the encryption public key for a custom VM configuration.
    ///
    /// # Parameters
    ///
    /// * `vm_config` - The VM configuration as a JSON value
    ///
    /// # Returns
    ///
    /// A JSON value containing the public key and salt for encryption
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails or returns an error
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

    /// Deploys a container with a custom VM configuration and encrypts environment variables.
    ///
    /// This method handles the encryption of environment variables and then calls
    /// `deploy_with_config_encrypted_env` to perform the actual deployment.
    ///
    /// # Parameters
    ///
    /// * `vm_config` - The VM configuration as a JSON value
    /// * `env_vars` - Environment variables to encrypt and include in the deployment
    /// * `app_env_encrypt_pubkey` - The public key for encrypting environment variables
    /// * `app_id_salt` - The salt value for encryption
    ///
    /// # Returns
    ///
    /// A `DeploymentResponse` containing the deployment details if successful
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * Environment variable encryption fails
    /// * The API request fails
    /// * The API returns an error response
    pub async fn deploy_with_config_do_encrypt(
        &self,
        vm_config: serde_json::Value,
        env_vars: &[(String, String)],
        app_env_encrypt_pubkey: &str,
        app_id_salt: &str,
    ) -> Result<DeploymentResponse, Error> {
        // Encrypt environment variables
        let encrypted_env = Encryptor::encrypt_env_vars(env_vars, app_env_encrypt_pubkey)?;

        self.deploy_with_config_encrypted_env(
            vm_config,
            encrypted_env,
            app_env_encrypt_pubkey,
            app_id_salt,
        )
        .await
    }

    /// Deploys a container with a custom VM configuration and pre-encrypted environment variables.
    ///
    /// This method is the final step in the deployment process, sending the VM configuration
    /// and encrypted environment variables to the API.
    ///
    /// # Parameters
    ///
    /// * `vm_config` - The VM configuration as a JSON value
    /// * `encrypted_env` - Pre-encrypted environment variables as a string
    /// * `app_env_encrypt_pubkey` - The public key used for encryption
    /// * `app_id_salt` - The salt value used for encryption
    ///
    /// # Returns
    ///
    /// A `DeploymentResponse` containing the deployment details if successful
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails or returns an error
    pub async fn deploy_with_config_encrypted_env(
        &self,
        vm_config: serde_json::Value,
        encrypted_env: String,
        app_env_encrypt_pubkey: &str,
        app_id_salt: &str,
    ) -> Result<DeploymentResponse, Error> {
        // Create a mutable request body
        let mut request_body = vm_config.as_object().cloned().unwrap_or_default();

        // Add the additional fields
        request_body.insert(
            "encrypted_env".to_string(),
            serde_json::Value::String(encrypted_env),
        );
        request_body.insert(
            "app_env_encrypt_pubkey".to_string(),
            serde_json::Value::String(app_env_encrypt_pubkey.to_string()),
        );
        request_body.insert(
            "app_id_salt".to_string(),
            serde_json::Value::String(app_id_salt.to_string()),
        );

        println!("request_body: {:#?}", request_body);
        // Create deployment
        let response = self
            .client
            .post(format!(
                "{}/cvms/from_cvm_configuration",
                self.config.api_url
            ))
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.config.api_key)
            .json(&request_body)
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
