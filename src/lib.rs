//! # TEE Deploy
//!
//! A secure deployment library for managing Docker containers in Trusted Execution Environments (TEEs).
//! This library provides functionality to deploy and manage containerized applications with encrypted
//! environment variables and secure communication.
//!
//! ## Features
//!
//! - Secure environment variable handling with x25519 key exchange
//! - AES-GCM encryption for sensitive data
//! - Docker compose configuration management
//! - TEE-compatible deployment pipeline
//! - Comprehensive error handling
//!
//! ## Example
//!
//! ```rust,no_run
//! use std::collections::HashMap;
//! use phala_tee_deploy_rs::{DeploymentConfig, TeeClient};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Set up environment variables
//!     let mut env_vars = HashMap::new();
//!     env_vars.insert("API_KEY".to_string(), "secret".to_string());
//!
//!     // Create configuration
//!     let config = DeploymentConfig::new(
//!         "your-api-key".to_string(),
//!         "version: '3'".to_string(),
//!         env_vars,
//!         1,
//!         "your-image:latest".to_string(),
//!     );
//!
//!     // Deploy
//!     let client = TeeClient::new(config)?;
//!     let deployment = client.deploy().await?;
//!     
//!     println!("Deployed: {}", deployment.id);
//!     Ok(())
//! }
//! ```
//!
//! ## Security Considerations
//!
//! This library implements several security measures:
//!
//! 1. Environment variables are encrypted using x25519 key exchange
//! 2. AES-GCM provides authenticated encryption
//! 3. All API communication uses HTTPS
//! 4. Sensitive data is never logged
//!
//! ## Error Handling
//!
//! The library provides a comprehensive error type that covers:
//!
//! - API communication errors
//! - Configuration validation
//! - Encryption/decryption issues
//! - Missing environment variables
//!
//! ## Testing
//!
//! The library includes extensive tests covering:
//!
//! - Successful deployment flows
//! - Error handling
//! - Timeout scenarios
//! - Input validation
//!
//! Use `cargo test` to run the test suite.

mod client;
mod config;
mod crypto;
mod error;
mod types;

#[cfg(test)]
mod tests;

pub use client::TeeClient;
pub use config::DeploymentConfig;
pub use crypto::Encryptor;
pub use error::Error;
pub use types::*;

/// Result type for TEE deployment operations
pub type Result<T> = std::result::Result<T, Error>;
