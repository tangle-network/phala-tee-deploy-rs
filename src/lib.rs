//! # Phala TEE Deploy - Rust Client
//!
//! A Rust client library for deploying Docker containers to the Phala TEE Cloud platform.
//! This library provides secure, efficient tools for managing containerized applications
//! within Trusted Execution Environments.
//!
//! ## Key Features
//!
//! - **Secure Deployment**: Environment variables are encrypted using industry-standard cryptography
//! - **Flexible API**: Both high-level (TeeDeployer) and low-level (TeeClient) interfaces
//! - **Docker Compose Integration**: Direct integration with Docker Compose configurations
//! - **TEEPod Management**: Discovery and selection of available TEE environments
//! - **Robust Error Handling**: Comprehensive error types with detailed diagnostics
//! - **Secure Workflows**: Support for separated operator/user deployment patterns
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use phala_tee_deploy_rs::{Result, TeeDeployerBuilder};
//! use std::collections::HashMap;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Create a deployer with your API key
//!     let mut deployer = TeeDeployerBuilder::new()
//!         .with_api_key("your-api-key")
//!         .build()?;
//!
//!     // Discover available TEEPods
//!     deployer.discover_teepod().await?;
//!
//!     // Define environment variables (will be encrypted)
//!     let mut env_vars = HashMap::new();
//!     env_vars.insert("PORT".to_string(), "8080".to_string());
//!     env_vars.insert("NODE_ENV".to_string(), "production".to_string());
//!
//!     // Deploy a simple NGINX service
//!     let result = deployer.deploy_simple_service(
//!         "nginx:latest",
//!         "web",
//!         "my-webapp",
//!         env_vars,
//!         Some(vec!["80:80".to_string()]),
//!         None,
//!         None,
//!         None,
//!         None,
//!         None,
//!     ).await?;
//!
//!     println!("Deployment successful: {:?}", result);
//!     Ok(())
//! }
//! ```
//!
//! ## Security
//!
//! This library implements several security best practices:
//!
//! - X25519 key exchange for secure key distribution
//! - AES-GCM for authenticated encryption of sensitive data
//! - TLS for all API communications
//! - Sensitive data is never logged in plaintext
//!
//! ## Secure Operator/User Workflow
//!
//! This library supports a secure workflow pattern that separates infrastructure management from sensitive data:
//!
//! ```rust,no_run
//! use phala_tee_deploy_rs::{Encryptor, Result, TeeDeployerBuilder};
//!
//! // OPERATOR PHASE 1: Setup infrastructure and get public key
//! async fn operator_setup() -> Result<(serde_json::Value, String, String)> {
//!     let mut deployer = TeeDeployerBuilder::new()
//!         .with_api_key("operator-api-key")
//!         .build()?;
//!
//!     deployer.discover_teepod().await?;
//!     
//!     // Create VM configuration
//!     let vm_config = deployer.create_vm_config_from_string(
//!         "version: '3'\nservices:\n  app:\n    image: nginx:alpine",
//!         "secure-app",
//!         None, None, None
//!     )?;
//!     
//!     // Get encryption public key
//!     let pubkey_response = deployer.get_pubkey_for_config(&vm_config).await?;
//!     let pubkey = pubkey_response["app_env_encrypt_pubkey"].as_str().unwrap().to_string();
//!     let salt = pubkey_response["app_id_salt"].as_str().unwrap().to_string();
//!     
//!     // Return VM config and encryption keys (to be sent to user)
//!     Ok((vm_config, pubkey, salt))
//! }
//!
//! // USER: Encrypt sensitive data
//! fn user_encrypt_secrets(pubkey: &str) -> Result<String> {
//!     let secrets = vec![
//!         ("DB_PASSWORD".to_string(), "super-secret-password".to_string()),
//!         ("API_KEY".to_string(), "secret-api-key".to_string()),
//!     ];
//!     
//!     // Encrypt with public key
//!     let encrypted_env = Encryptor::encrypt_env_vars(&secrets, pubkey)?;
//!     
//!     // Return encrypted data (to be sent back to operator)
//!     Ok(encrypted_env)
//! }
//!
//! // OPERATOR PHASE 2: Deploy with encrypted environment variables
//! async fn operator_deploy(
//!     vm_config: serde_json::Value,
//!     encrypted_env: String,
//!     pubkey: &str,
//!     salt: &str
//! ) -> Result<()> {
//!     let mut deployer = TeeDeployerBuilder::new()
//!         .with_api_key("operator-api-key")
//!         .build()?;
//!
//!     // Deploy with encrypted environment variables
//!     let deployment = deployer.deploy_with_encrypted_env(
//!         vm_config, encrypted_env, pubkey, salt
//!     ).await?;
//!     
//!     println!("Deployed successfully: {}", deployment["id"]);
//!     Ok(())
//! }
//! ```
//!
//! ## Documentation
//!
//! For more advanced usage, see:
//!
//! - [`TeeDeployer`]: High-level API for most deployment scenarios
//! - [`TeeClient`]: Low-level API for direct control over deployment details
//! - [`DeploymentConfig`]: Configuration options for the deployment process
//!
//! ## Error Handling
//!
//! The library uses a comprehensive [`Error`] type with variants for different
//! failure scenarios, making error diagnosis and handling straightforward.

mod client;
mod config;
mod crypto;
mod deployer;
mod error;
mod types;

#[cfg(test)]
mod tests;

pub use client::TeeClient;
pub use config::DeploymentConfig;
pub use crypto::Encryptor;
pub use deployer::{TeeDeployer, TeeDeployerBuilder};
pub use error::Error;
pub use types::*;

/// Result type for Phala TEE deployment operations.
///
/// This is a convenience alias for `std::result::Result<T, Error>` that simplifies
/// error handling throughout the library.
pub type Result<T> = std::result::Result<T, Error>;
