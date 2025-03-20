# Phala TEE Deployment Toolkit

A Rust library for deploying Docker Compose applications to the Phala TEE (Trusted Execution Environment) Cloud with secure environment variable handling.

## Overview

Phala TEE Cloud runs applications in hardware-enforced isolated enclaves, providing enhanced security guarantees. This toolkit simplifies deployments to the Phala Cloud with:

- **Environment Variable Encryption** - Secure handling of sensitive data
- **Docker Compose Support** - Deploy multi-container applications
- **ELIZA Deployment** - Simplified deployment of ELIZA chatbots
- **Flexible Deployment Patterns** - From simple one-step to advanced privilege separation

## Getting Started

### Prerequisites

- Phala Cloud account with API access
- An API key from your Phala account

### Environment Setup

Create a `.env` file with:

```
PHALA_CLOUD_API_KEY=your-api-key
```

## Deployment APIs

### High-Level API with TeeDeployer (Recommended)

The `TeeDeployer` provides a streamlined interface for deploying applications to Phala TEE Cloud.

```rust
use phala_tee_deploy_rs::{TeeDeployerBuilder};
use std::collections::HashMap;

// Create deployer with builder pattern
let mut deployer = TeeDeployerBuilder::new()
    .with_api_key(std::env::var("PHALA_CLOUD_API_KEY")?)
    .build()?;

// Discover available TEEPod
deployer.discover_teepod().await?;
```

### Deployment Options

#### 1. Deploy from Docker Compose YAML

```rust
// Deploy from YAML string
let yaml = r#"
version: '3'
services:
  app:
    image: nginx:latest
    ports:
      - "80:80"
"#;

let mut env_vars = HashMap::new();
env_vars.insert("PORT".to_string(), "80".to_string());

let result = deployer.deploy_compose_from_string(
    yaml,
    "my-app",
    env_vars,
    Some(1),    // vCPUs
    Some(1024), // Memory (MB)
    Some(10),   // Disk size (GB)
).await?;

// Access the response
println!("Deployment ID: {}", result.id);
println!("Status: {}", result.status);
```

#### 2. Deploy a Simple Service

```rust
// Deploy a simple service with just an image
let mut env_vars = HashMap::new();
env_vars.insert("PORT".to_string(), "3000".to_string());

let result = deployer.deploy_simple_service(
    "nginx:latest",                  // Docker image
    "web",                           // Service name
    "my-webapp",                     // App name
    env_vars,                        // Environment variables
    Some(vec!["80:80".to_string()]), // Port mappings
    None,                            // Volumes
    None,                            // Command
    None,                            // vCPUs (default)
    None,                            // Memory (default)
    None,                            // Disk size (default)
).await?;
```

#### 3. Deploy ELIZA (Two-Step Process)

```rust
// Step 1: Provision ELIZA to get app_id and encryption key
let deployment_name = format!("eliza-demo-{}", uuid::Uuid::new_v4());
let (app_id, app_env_encrypt_pubkey) = deployer
    .provision_eliza(
        deployment_name.clone(),
        character_file,               // ELIZA character configuration
        vec!["OPENAI_API_KEY".to_string()], // Environment variables to include
        "phalanetwork/eliza:v0.1.8-alpha.1".to_string(),
    )
    .await?;

// Step 2: Encrypt environment variables
let mut env_vars = Vec::new();
env_vars.push(("CHARACTER_DATA".to_string(), character_file));
if let Ok(key) = std::env::var("OPENAI_API_KEY") {
    env_vars.push(("OPENAI_API_KEY".to_string(), key));
}
let encrypted_env = Encryptor::encrypt_env_vars(&env_vars, &app_env_encrypt_pubkey)?;

// Step 3: Create VM with encrypted environment variables
let result = deployer.create_eliza_vm(&app_id, &encrypted_env).await?;
```

### Getting Deployment Information

#### Network Information

```rust
// Get network info for a deployed application
let network_info = deployer.get_network_info(&app_id).await?;

if network_info.is_online {
    println!("Application URL: {}", network_info.public_urls.app);
    println!("Instance URL: {}", network_info.public_urls.instance);
}
```

#### System Statistics

```rust
// Get system statistics for a deployed application
let stats = deployer.get_system_stats(&app_id).await?;

println!("OS: {} {}", stats.sysinfo.os_name, stats.sysinfo.os_version);
println!("Memory: {:.2} GB used / {:.2} GB total",
    stats.sysinfo.used_memory as f64 / 1024.0 / 1024.0 / 1024.0,
    stats.sysinfo.total_memory as f64 / 1024.0 / 1024.0 / 1024.0
);
```

### Updating Deployments

```rust
// Update an existing deployment
let app_id = format!("app_{}", deployment_id);
let mut new_env_vars = HashMap::new();
new_env_vars.insert("DEBUG".to_string(), "true".to_string());

let update_result = deployer.update_deployment(
    &app_id,
    Some(new_docker_compose),  // New Docker Compose configuration (optional)
    Some(new_env_vars)         // New environment variables (optional)
).await?;
```

## Advanced Deployment Patterns

For more advanced use cases such as privilege separation (where operators handle infrastructure while users manage secrets), see the examples directory or refer to the API documentation.

## Examples

Check out the `examples` directory for complete working examples:

- `simple_deployment.rs` - Basic deployment example
- `eliza_deployment.rs` - ELIZA chatbot deployment example
- `advanced_deployment.rs` - Step-by-step deployment with privilege separation
