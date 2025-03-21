use crate::{
    AdvancedFeatures, ComposeManifest, DeploymentConfig, DeploymentResponse, DockerConfig, Error,
    NetworkInfoResponse, PubkeyResponse, Result, SystemStatsResponse, TeeClient,
    TeePodDiscoveryResponse, VmConfig,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;

/// `TeeDeployer` provides a high-level interface for deploying Docker Compose applications
/// to the Phala TEE Cloud platform.
///
/// This struct provides a simplified API for deploying applications compared to the
/// lower-level `TeeClient`, with methods for common deployment scenarios and
/// configuration management using standard JSON.
///
/// # Features
///
/// * TEEPod discovery and selection
/// * Docker Compose application deployment from various sources
/// * Environment variable encryption
/// * Deployment updates
///
/// # Examples
///
/// ```no_run
/// use phala_tee_deploy_rs::{Result, TeeDeployerBuilder};
/// use std::collections::HashMap;
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     // Create deployer using builder pattern
///     let mut deployer = TeeDeployerBuilder::new()
///         .with_api_key("your_api_key")
///         .build()?;
///
///     // Discover available TEEPods
///     deployer.discover_teepod().await?;
///
///     // Deploy a simple service
///     let mut env_vars = HashMap::new();
///     env_vars.insert("PORT".to_string(), "3000".to_string());
///
///     let result = deployer.deploy_simple_service(
///         "nginx:latest",
///         "web",
///         "my-webapp",
///         env_vars,
///         Some(vec!["80:80".to_string()]),
///         None,
///         None,
///         None,
///         None,
///         None,
///     ).await?;
///
///     println!("Deployment successful: {:?}", result);
///     Ok(())
/// }
/// ```
pub struct TeeDeployer {
    client: TeeClient,
    selected_teepod: Option<(u64, String)>,
}

impl TeeDeployer {
    /// Creates a new `TeeDeployer` with the specified API credentials.
    ///
    /// # Parameters
    ///
    /// * `api_key` - The API key for authenticating with the Phala Cloud API
    /// * `api_endpoint` - Optional custom API endpoint URL. If `None`, uses the default endpoint
    ///
    /// # Returns
    ///
    /// A new `TeeDeployer` instance or an error if initialization fails
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying HTTP client cannot be created
    pub fn new(api_key: String, api_endpoint: Option<String>) -> Result<Self> {
        let config = DeploymentConfig {
            api_key,
            api_url: api_endpoint
                .unwrap_or_else(|| "https://cloud-api.phala.network/api/v1".to_string()),
            docker_compose: String::new(),
            env_vars: HashMap::new(),
            teepod_id: 0,
            image: String::new(),
            vm_config: None,
        };

        Ok(Self {
            client: TeeClient::new(config)?,
            selected_teepod: None,
        })
    }

    /// Discovers and selects the first available TEEPod automatically.
    ///
    /// This method queries the Phala Cloud API for available TEEPods and selects
    /// the first one from the response. It's a convenient way to get started without
    /// needing to choose a specific TEEPod.
    ///
    /// # Returns
    ///
    /// `Ok(())` if a TEEPod was successfully discovered and selected
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The API request fails
    /// * No TEEPods are available
    /// * The API response has an unexpected format
    pub async fn discover_teepod(&mut self) -> Result<TeePodDiscoveryResponse> {
        eprintln!("🔍 Discovering available TEEPods...");

        let teepods = match self.client.get_available_teepods().await {
            Ok(result) => result,
            Err(err) => {
                // Log error details for diagnosis
                eprintln!("❌ Error connecting to TEEPod service: {}", err);
                if let Error::Api {
                    status_code,
                    ref message,
                } = err
                {
                    if status_code >= 500 {
                        eprintln!("   Server error ({}): This may be a temporary issue with the Phala Cloud API", status_code);
                        if message.contains("502") && message.contains("Bad Gateway") {
                            eprintln!("   Cloudflare 502 Bad Gateway detected - the API server may be unreachable");
                            eprintln!("   You may want to check your network connection or try again later");
                        }
                    }
                }
                return Err(err);
            }
        };

        let nodes = teepods.nodes.clone();

        if nodes.is_empty() {
            return Err(Error::Api {
                status_code: 400,
                message: "No available TEEPods found".into(),
            });
        }

        let node = &nodes[0];
        let teepod_id = node.teepod_id;

        let image = node.images[0].name.clone();

        eprintln!(
            "✅ TEEPod discovered: ID {} with image {}",
            teepod_id, image
        );
        self.selected_teepod = Some((teepod_id, image));
        Ok(teepods)
    }

    /// Selects a specific TEEPod by ID and verifies its availability.
    ///
    /// This method allows you to choose a particular TEEPod for deployment instead
    /// of using the first available one.
    ///
    /// # Parameters
    ///
    /// * `teepod_id` - The ID of the TEEPod to select
    ///
    /// # Returns
    ///
    /// `Ok(())` if the TEEPod was found and selected successfully
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The API request fails
    /// * The specified TEEPod is not found or not available
    /// * The API response has an unexpected format
    pub async fn select_teepod(&mut self, teepod_id: u64) -> Result<()> {
        eprintln!("🔍 Selecting TEEPod with ID: {}", teepod_id);

        let teepods = match self.client.get_available_teepods().await {
            Ok(result) => result,
            Err(err) => {
                // Log error details for diagnosis
                eprintln!("❌ Error connecting to TEEPod service: {}", err);
                if let Error::Api {
                    status_code,
                    ref message,
                } = err
                {
                    if status_code >= 500 {
                        eprintln!("   Server error ({}): This may be a temporary issue with the Phala Cloud API", status_code);
                        if message.contains("502") && message.contains("Bad Gateway") {
                            eprintln!("   Cloudflare 502 Bad Gateway detected - the API server may be unreachable");
                            eprintln!("   You may want to check your network connection or try again later");
                        }
                    }
                }
                return Err(err);
            }
        };

        let nodes = teepods.nodes.clone();

        for node in nodes {
            if node.teepod_id == teepod_id {
                let image = node.images[0].name.clone();

                eprintln!("✅ TEEPod selected: ID {} with image {}", teepod_id, image);
                self.selected_teepod = Some((teepod_id, image));
                return Ok(());
            }
        }

        eprintln!("❌ TEEPod with ID {} not found or not available", teepod_id);
        Err(Error::Api {
            status_code: 404,
            message: format!("TEEPod with ID {} not found or not available", teepod_id),
        })
    }

    /// Deploys a Docker Compose application using a YAML string configuration.
    ///
    /// This method takes a Docker Compose configuration as a string and deploys it to the
    /// selected TEEPod with the specified options.
    ///
    /// # Parameters
    ///
    /// * `docker_compose_file` - The Docker Compose configuration as a YAML string
    /// * `app_name` - Name for the deployed application
    /// * `env_vars` - Environment variables for the application (will be securely encrypted)
    /// * `vcpu` - Optional vCPU cores for the VM (defaults to 1)
    /// * `memory` - Optional memory in MB for the VM (defaults to 1024)
    /// * `disk_size` - Optional disk size in GB for the VM (defaults to 10)
    ///
    /// # Returns
    ///
    /// A `DeploymentResponse` containing deployment details including ID, status, and TEEPod information
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * No TEEPod has been selected
    /// * The API request fails
    /// * Environment variable encryption fails
    pub async fn deploy_compose(
        &self,
        docker_compose_file: &str,
        app_name: &str,
        env_vars: HashMap<String, String>,
        vcpu: Option<u64>,
        memory: Option<u64>,
        disk_size: Option<u64>,
    ) -> Result<DeploymentResponse> {
        // Ensure we have a selected TEEPod
        let (teepod_id, image) = self.selected_teepod.as_ref().ok_or_else(|| Error::Api {
            status_code: 400,
            message: "No TEEPod selected. Call discover_teepod() or select_teepod() first".into(),
        })?;

        // Create VM configuration
        let vm_config = json!({
            "name": app_name,
            "compose_manifest": {
                "docker_compose_file": docker_compose_file,
                "name": app_name,
                "features": ["kms", "tproxy-net"]
            },
            "vcpu": vcpu.unwrap_or(1),
            "memory": memory.unwrap_or(1024),
            "disk_size": disk_size.unwrap_or(10),
            "teepod_id": teepod_id,
            "image": image
        });

        // Deploy the application with automatic encryption
        let env_vars_vec: Vec<(String, String)> = env_vars.into_iter().collect();

        // Get encryption keys
        let pubkey_response = self.client.get_pubkey_for_config(&vm_config).await?;
        let pubkey = pubkey_response.app_env_encrypt_pubkey;
        let salt = pubkey_response.app_id_salt;

        // Deploy with encrypted environment variables
        let deployment = self
            .client
            .deploy_with_config_do_encrypt(vm_config, &env_vars_vec, &pubkey, &salt)
            .await?;

        // Add extra details if needed
        if let Some(mut details) = deployment.details.clone() {
            details.insert(
                "teepod_id".to_string(),
                serde_json::Value::Number(serde_json::Number::from(*teepod_id)),
            );
            details.insert(
                "image".to_string(),
                serde_json::Value::String(image.clone()),
            );

            let mut deployment_with_details = deployment.clone();
            deployment_with_details.details = Some(details);
            Ok(deployment_with_details)
        } else {
            // Create new details if none exist
            let mut details = HashMap::new();
            details.insert(
                "teepod_id".to_string(),
                serde_json::Value::Number(serde_json::Number::from(*teepod_id)),
            );
            details.insert(
                "image".to_string(),
                serde_json::Value::String(image.clone()),
            );

            let mut deployment_with_details = deployment.clone();
            deployment_with_details.details = Some(details);
            Ok(deployment_with_details)
        }
    }

    /// Deploys a Docker Compose application from a file path.
    ///
    /// Reads a Docker Compose file from the specified path and deploys it to the selected TEEPod.
    ///
    /// # Parameters
    ///
    /// * `compose_path` - Path to the Docker Compose file
    /// * `app_name` - Name for the deployed application
    /// * `env_vars` - Environment variables for the application
    /// * `vcpu` - Optional vCPU cores for the VM
    /// * `memory` - Optional memory in MB for the VM
    /// * `disk_size` - Optional disk size in GB for the VM
    ///
    /// # Returns
    ///
    /// A `DeploymentResponse` containing deployment details
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The file cannot be read
    /// * The underlying `deploy_compose` call fails
    pub async fn deploy_compose_from_file<P: AsRef<Path>>(
        &self,
        compose_path: P,
        app_name: &str,
        env_vars: HashMap<String, String>,
        vcpu: Option<u64>,
        memory: Option<u64>,
        disk_size: Option<u64>,
    ) -> Result<DeploymentResponse> {
        // Read compose file
        let content = std::fs::read_to_string(compose_path)
            .map_err(|e| Error::Configuration(format!("Failed to read compose file: {}", e)))?;

        self.deploy_compose(&content, app_name, env_vars, vcpu, memory, disk_size)
            .await
    }

    /// Deploys a Docker Compose application from a YAML string.
    ///
    /// Takes a Docker Compose YAML string and deploys it to the selected TEEPod.
    ///
    /// # Parameters
    ///
    /// * `yaml_content` - Docker Compose YAML content as a string
    /// * `app_name` - Name for the deployed application
    /// * `env_vars` - Environment variables for the application
    /// * `vcpu` - Optional vCPU cores for the VM
    /// * `memory` - Optional memory in MB for the VM
    /// * `disk_size` - Optional disk size in GB for the VM
    ///
    /// # Returns
    ///
    /// A `DeploymentResponse` containing deployment details
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying `deploy_compose` call fails
    pub async fn deploy_compose_from_string(
        &self,
        yaml_content: &str,
        app_name: &str,
        env_vars: HashMap<String, String>,
        vcpu: Option<u64>,
        memory: Option<u64>,
        disk_size: Option<u64>,
    ) -> Result<DeploymentResponse> {
        self.deploy_compose(yaml_content, app_name, env_vars, vcpu, memory, disk_size)
            .await
    }

    /// Deploys a simple service using just an image name and basic configuration.
    ///
    /// This is a convenience method for quickly deploying a single service
    /// without needing to create a full Docker Compose configuration.
    ///
    /// # Parameters
    ///
    /// * `image` - Docker image name (e.g. "nginx:latest")
    /// * `service_name` - Name for the service
    /// * `app_name` - Name for the deployed application
    /// * `env_vars` - Environment variables for the service
    /// * `ports` - Optional port mappings (e.g. ["80:80"])
    /// * `volumes` - Optional volume mappings
    /// * `command` - Optional command override for the container
    /// * `vcpu` - Optional vCPU cores for the VM
    /// * `memory` - Optional memory in MB for the VM
    /// * `disk_size` - Optional disk size in GB for the VM
    ///
    /// # Returns
    ///
    /// A `DeploymentResponse` containing deployment details
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying `deploy_compose` call fails
    pub async fn deploy_simple_service(
        &self,
        image: &str,
        service_name: &str,
        app_name: &str,
        env_vars: HashMap<String, String>,
        ports: Option<Vec<String>>,
        volumes: Option<Vec<String>>,
        command: Option<Vec<String>>,
        vcpu: Option<u64>,
        memory: Option<u64>,
        disk_size: Option<u64>,
    ) -> Result<DeploymentResponse> {
        // Create a simple Docker Compose YAML configuration
        let mut yaml = String::from("services:\n");
        yaml.push_str(&format!("  {}:\n", service_name));
        yaml.push_str(&format!("    image: {}\n", image));

        if let Some(ports) = &ports {
            yaml.push_str("    ports:\n");
            for port in ports {
                yaml.push_str(&format!("      - \"{}\"\n", port));
            }
        }

        if let Some(volumes) = &volumes {
            yaml.push_str("    volumes:\n");
            for volume in volumes {
                yaml.push_str(&format!("      - {}\n", volume));
            }
        }

        if let Some(command) = &command {
            yaml.push_str("    command: [");
            for (i, cmd) in command.iter().enumerate() {
                if i > 0 {
                    yaml.push_str(", ");
                }
                yaml.push_str(&format!("\"{}\"", cmd));
            }
            yaml.push_str("]\n");
        }

        if !env_vars.is_empty() {
            yaml.push_str("    environment:\n");
            for (key, value) in &env_vars {
                yaml.push_str(&format!("      {}: {}\n", key, value));
            }
        }

        // Deploy
        self.deploy_compose(&yaml, app_name, env_vars, vcpu, memory, disk_size)
            .await
    }

    /// Updates an existing deployment with new configuration and/or environment variables.
    ///
    /// # Parameters
    ///
    /// * `app_id` - The ID of the application to update
    /// * `compose_content` - Optional new Docker Compose configuration as a YAML string
    /// * `env_vars` - Optional new environment variables
    ///
    /// # Returns
    ///
    /// A `Value` containing details about the update operation
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The application cannot be found
    /// * The API request fails
    pub async fn update_deployment(
        &self,
        app_id: &str,
        compose_content: Option<&str>,
        env_vars: Option<HashMap<String, String>>,
    ) -> Result<Value> {
        // Get the current compose configuration
        let compose_response = self.client.get_compose(app_id).await?;
        let mut compose_file = compose_response.compose_file;

        // Update compose file if provided
        if let Some(new_config) = compose_content {
            if let Some(manifest) = compose_file.get_mut("compose_manifest") {
                if let Some(obj) = manifest.as_object_mut() {
                    obj.insert("docker_compose_file".to_string(), json!(new_config));
                }
            }
        }

        // Apply the update
        let response = self
            .client
            .update_compose(app_id, compose_file, env_vars, compose_response.env_pubkey)
            .await?;

        Ok(json!({
            "status": "updated",
            "app_id": app_id,
            "details": response
        }))
    }

    /// Creates a VM configuration for a Docker Compose application.
    ///
    /// This method creates a VM configuration without actually deploying it,
    /// which can be used to request encryption keys or for later deployment.
    ///
    /// # Parameters
    ///
    /// * `docker_compose_file` - The Docker Compose configuration as a YAML string
    /// * `app_name` - Name for the application
    /// * `vcpu` - Optional vCPU cores for the VM (defaults to 1)
    /// * `memory` - Optional memory in MB for the VM (defaults to 1024)
    /// * `disk_size` - Optional disk size in GB for the VM (defaults to 10)
    ///
    /// # Returns
    ///
    /// A VM configuration object
    ///
    /// # Errors
    ///
    /// Returns an error if no TEEPod has been selected
    pub fn create_vm_config(
        &self,
        docker_compose_file: &str,
        app_name: &str,
        vcpu: Option<u64>,
        memory: Option<u64>,
        disk_size: Option<u64>,
    ) -> Result<VmConfig> {
        // Ensure we have a selected TEEPod
        let (teepod_id, image) = self.selected_teepod.as_ref().ok_or_else(|| Error::Api {
            status_code: 400,
            message: "No TEEPod selected. Call discover_teepod() or select_teepod() first".into(),
        })?;

        // Create VM configuration
        let vm_config = VmConfig {
            name: app_name.to_string(),
            compose_manifest: ComposeManifest {
                name: app_name.to_string(),
                features: vec!["kms".to_string(), "tproxy-net".to_string()],
                docker_compose_file: docker_compose_file.to_string(),
            },
            vcpu: vcpu.unwrap_or(1) as u32,
            memory: memory.unwrap_or(1024) as u32,
            disk_size: disk_size.unwrap_or(10) as u32,
            teepod_id: *teepod_id,
            image: image.to_string(),
            advanced_features: AdvancedFeatures {
                tproxy: true,
                kms: true,
                public_sys_info: true,
                public_logs: true,
                docker_config: DockerConfig {
                    username: String::new(),
                    password: String::new(),
                    registry: None,
                },
                listed: true,
            },
        };

        Ok(vm_config)
    }

    /// Creates a VM configuration from a Docker Compose file path.
    ///
    /// This is a convenience method that reads a Docker Compose file
    /// and creates a VM configuration for it.
    ///
    /// # Parameters
    ///
    /// * `compose_path` - Path to the Docker Compose file
    /// * `app_name` - Name for the application
    /// * `vcpu` - Optional vCPU cores for the VM
    /// * `memory` - Optional memory in MB for the VM
    /// * `disk_size` - Optional disk size in GB for the VM
    ///
    /// # Returns
    ///
    /// A VM configuration object
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The file cannot be read
    /// * The `create_vm_config` call fails
    pub fn create_vm_config_from_file<P: AsRef<Path>>(
        &self,
        compose_path: P,
        app_name: &str,
        vcpu: Option<u64>,
        memory: Option<u64>,
        disk_size: Option<u64>,
    ) -> Result<VmConfig> {
        // Read compose file
        let content = std::fs::read_to_string(compose_path)
            .map_err(|e| Error::Configuration(format!("Failed to read compose file: {}", e)))?;

        self.create_vm_config(&content, app_name, vcpu, memory, disk_size)
    }

    /// Retrieves the encryption public key for a VM configuration.
    ///
    /// This method requests a public key from the API that can be used to
    /// encrypt environment variables for a specific VM configuration.
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
    /// Returns an error if the API request fails
    pub async fn get_pubkey_for_config(&self, vm_config: &Value) -> Result<PubkeyResponse> {
        self.client
            .get_pubkey_for_config(vm_config)
            .await
            .map_err(|e| e)
    }

    /// Deploys a VM configuration with pre-encrypted environment variables.
    ///
    /// This method is useful for workflows where the user encrypts their own
    /// environment variables and the operator deploys the application. It allows
    /// for a separation of concerns between compute management and sensitive data.
    ///
    /// # Parameters
    ///
    /// * `vm_config` - The VM configuration as a JSON value
    /// * `encrypted_env` - Pre-encrypted environment variables as a hex string
    /// * `app_env_encrypt_pubkey` - The public key used for encryption
    /// * `app_id_salt` - The salt value used for encryption
    ///
    /// # Returns
    ///
    /// A JSON value containing the deployment details
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails
    pub async fn deploy_with_encrypted_env(
        &self,
        vm_config: Value,
        encrypted_env: String,
        app_env_encrypt_pubkey: &str,
        app_id_salt: &str,
    ) -> Result<DeploymentResponse> {
        let response = self
            .client
            .deploy_with_config_encrypted_env(
                vm_config,
                encrypted_env,
                app_env_encrypt_pubkey,
                app_id_salt,
            )
            .await?;

        Ok(response)
    }

    /// Provisions a new ELIZA chatbot deployment.
    ///
    /// This method initiates the ELIZA deployment process by requesting an app_id
    /// and encryption key from the API. This is the first step in the two-step
    /// deployment process.
    ///
    /// # Parameters
    ///
    /// * `name` - Name for the ELIZA deployment
    /// * `character_file` - Character configuration file content
    /// * `env_keys` - List of environment variable keys to include
    /// * `image` - Docker image to use for the deployment
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// * `app_id` - The ID of the provisioned application
    /// * `app_env_encrypt_pubkey` - The public key for encrypting environment variables
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The API request fails
    /// * Invalid configuration is provided
    /// * The response cannot be parsed
    pub async fn provision_eliza(
        &self,
        name: String,
        character_file: String,
        env_keys: Vec<String>,
        image: String,
    ) -> Result<(String, String)> {
        self.client
            .provision_eliza(name, character_file, env_keys, image)
            .await
    }

    /// Creates a VM for an ELIZA deployment with encrypted environment variables.
    ///
    /// This method is the second step in the ELIZA deployment process, creating
    /// the actual VM with the provided encrypted environment variables.
    ///
    /// # Parameters
    ///
    /// * `app_id` - The ID of the provisioned application
    /// * `encrypted_env` - Pre-encrypted environment variables
    ///
    /// # Returns
    ///
    /// A `DeploymentResponse` containing the deployment details and status
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The API request fails
    /// * The deployment cannot be created
    /// * The response cannot be parsed
    pub async fn create_eliza_vm(
        &self,
        app_id: &str,
        encrypted_env: &str,
    ) -> Result<DeploymentResponse> {
        self.client.create_eliza_vm(app_id, encrypted_env).await
    }

    /// Retrieves network information for a deployed application.
    ///
    /// This method fetches network connectivity details, status, and public URLs
    /// for accessing a deployed application.
    ///
    /// # Parameters
    ///
    /// * `app_id` - The ID of the application to get network information for
    ///
    /// # Returns
    ///
    /// A `NetworkInfoResponse` containing network details including status and URLs
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The API request fails
    /// * The application is not found
    /// * The network information cannot be retrieved
    pub async fn get_network_info(&self, app_id: &str) -> Result<NetworkInfoResponse> {
        self.client.get_network_info(app_id).await
    }

    /// Retrieves system statistics for a deployed application.
    ///
    /// This method fetches detailed system information including OS details,
    /// CPU, memory, disk usage, and load averages for a deployed application.
    ///
    /// # Parameters
    ///
    /// * `app_id` - The ID of the application to get system statistics for
    ///
    /// # Returns
    ///
    /// A `SystemStatsResponse` containing system information including resource usage
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The API request fails
    /// * The application is not found
    /// * The system statistics cannot be retrieved
    pub async fn get_system_stats(&self, app_id: &str) -> Result<SystemStatsResponse> {
        self.client.get_system_stats(app_id).await
    }

    /// Returns a reference to the underlying `TeeClient` for direct access to lower-level operations.
    ///
    /// # Returns
    ///
    /// A reference to the internal `TeeClient` instance
    pub fn get_client(&self) -> &TeeClient {
        &self.client
    }
}

/// Builder for creating a `TeeDeployer` with a fluent interface.
///
/// This builder pattern allows for a more ergonomic API when constructing
/// a `TeeDeployer` instance with various optional parameters.
///
/// # Examples
///
/// ```no_run
/// use phala_tee_deploy_rs::TeeDeployerBuilder;
///
/// let deployer = TeeDeployerBuilder::new()
///     .with_api_key("your_api_key_here")
///     .with_api_endpoint("https://custom-endpoint.example.com/api/v1")
///     .build()
///     .expect("Failed to create deployer");
/// ```
pub struct TeeDeployerBuilder {
    api_key: Option<String>,
    api_endpoint: Option<String>,
}

impl TeeDeployerBuilder {
    /// Creates a new empty `TeeDeployerBuilder`.
    ///
    /// # Returns
    ///
    /// A new `TeeDeployerBuilder` instance with no parameters set
    pub fn new() -> Self {
        Self {
            api_key: None,
            api_endpoint: None,
        }
    }

    /// Sets the API key for authenticating with the Phala Cloud API.
    ///
    /// # Parameters
    ///
    /// * `api_key` - The API key to use
    ///
    /// # Returns
    ///
    /// The builder instance for method chaining
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Sets a custom API endpoint URL.
    ///
    /// # Parameters
    ///
    /// * `endpoint` - The custom API endpoint URL
    ///
    /// # Returns
    ///
    /// The builder instance for method chaining
    pub fn with_api_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.api_endpoint = Some(endpoint.into());
        self
    }

    /// Builds a `TeeDeployer` instance with the configured parameters.
    ///
    /// # Returns
    ///
    /// A new `TeeDeployer` instance if successful
    ///
    /// # Errors
    ///
    /// Returns an error if the API key is not set or if the `TeeDeployer` creation fails
    pub fn build(self) -> Result<TeeDeployer> {
        let api_key = self
            .api_key
            .ok_or_else(|| Error::Configuration("API key is required".into()))?;

        TeeDeployer::new(api_key, self.api_endpoint)
    }
}
