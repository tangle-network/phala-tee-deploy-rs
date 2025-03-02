# Phala TEE Deployment Toolkit

This repository contains a Rust library and tools for deploying Docker Compose configurations to the Phala TEE (Trusted Execution Environment) Cloud. The toolkit enables secure deployment of any containerized application to run in a trusted environment with enhanced security.

## Overview

Phala TEE Cloud provides a secure computing environment where your applications run in hardware-enforced isolated enclaves. This toolkit simplifies the process of deploying containerized applications to this environment, handling encryption of sensitive data and communication with the Phala Cloud API.

## Prerequisites

Before deploying, ensure you have the following:

1. A Phala Cloud account with API access
2. A TEE pod ID from your Phala account
3. Any API keys or secrets required by your application

## Core Features

- **Secure Environment Variable Handling**: All sensitive data is encrypted before transmission
- **Docker Compose Support**: Deploy multi-container applications using standard Docker Compose files
- **Rust Library Integration**: Programmatically deploy applications using the Rust library
- **Bash Script Alternative**: Simple deployment script for those who prefer bash
- **TEE-Specific Optimizations**: Configurations optimized for the Phala TEE environment

## Environment Variables

The deployment requires the following environment variables:

- `PHALA_CLOUD_API_KEY`: Your Phala Cloud API key
- `PHALA_TEEPOD_ID`: Your TEE pod ID from Phala
- Application-specific environment variables as needed

You can set these variables in a `.env` file or export them in your terminal.

## Deployment Examples

### Step by Step Deployment

This approach mirrors the TypeScript workflow, providing more granular control:

```rust
use phala_tee_deploy_rs::{DeploymentConfig, TeeClient};
use serde_json::json;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup client
    let config = DeploymentConfig {
        api_key: std::env::var("PHALA_CLOUD_API_KEY").expect("API key not set"),
        api_url: std::env::var("PHALA_CLOUD_API_ENDPOINT")
            .unwrap_or_else(|_| "https://cloud-api.phala.network/api/v1".to_string()),
        docker_compose: String::new(), // Not needed for this method
        env_vars: HashMap::new(),      // Not needed for this method
        teepod_id: 0,                  // Not needed for this method
        image: String::new(),          // Not needed for this method
        vm_config: None,               // Not needed for this method
    };

    let client = TeeClient::new(config)?;

    // Step 1: Get available TEEPods
    let teepods = client.get_available_teepods().await?;
    if teepods["nodes"].as_array().map_or(true, |nodes| nodes.is_empty()) {
        println!("No available TEEPods found");
        return Ok(());
    }

    // Extract the first available TEEPod and image
    let node = &teepods["nodes"][0];
    let teepod_id = node["teepod_id"].as_u64().expect("Invalid TEEPod ID");
    let image = node["images"][0]["name"].as_str().expect("Invalid image name");

    // Step 2: Create VM configuration
    let vm_config = json!({
        "name": "my-app",
        "compose_manifest": {
            "docker_compose_file": "services:\n  app:\n    image: my-image:latest\n    ports:\n      - '3000:3000'",
            "name": "my-app"
        },
        "vcpu": 1,
        "memory": 1024,
        "disk_size": 10,
        "teepod_id": teepod_id,
        "image": image
    });

    // Step 3: Get encryption public key
    let pubkey_response = client.get_pubkey_for_config(&vm_config).await?;

    // Step 4: Prepare environment variables to encrypt
    let env_vars = vec![
        ("API_KEY".to_string(), "secret-value".to_string()),
        ("DEBUG".to_string(), "true".to_string())
    ];

    // Step 5: Deploy with encrypted environment variables
    let deployment = client.deploy_with_config(
        vm_config,
        &env_vars,
        pubkey_response["app_env_encrypt_pubkey"].as_str().unwrap(),
        pubkey_response["app_id_salt"].as_str().unwrap()
    ).await?;

    println!("Deployment ID: {}", deployment.id);

    Ok(())
}
```

### Operator-User Workflow (Separation of Privileges)

This workflow demonstrates a separation of concerns pattern where:

1. An **operator** is responsible for infrastructure (has API access to deploy)
2. A **user** is responsible for application secrets (but doesn't need API access)

```rust
// OPERATOR ACTIONS
// The operator has API access but doesn't need to see the secrets
async fn operator_workflow() -> Result<(), Error> {
    // Set up client
    let client = TeeClient::new(/* config */)?;

    // 1. Fetch available TEEPods
    let teepods = client.get_available_teepods().await?;
    let teepod_id = teepods["nodes"][0]["teepod_id"].as_u64().unwrap();
    let image = teepods["nodes"][0]["images"][0]["name"].as_str().unwrap();

    // 2. Create VM configuration
    let vm_config = json!({
        "name": "secure-application",
        "compose_manifest": {
            "docker_compose_file": "version: '3'...",
            "name": "app"
        },
        "teepod_id": teepod_id,
        "image": image
    });

    // 3. Get encryption public key
    let pubkey_response = client.get_pubkey_for_config(&vm_config).await?;
    let pubkey = pubkey_response["app_env_encrypt_pubkey"].as_str().unwrap();
    let salt = pubkey_response["app_id_salt"].as_str().unwrap();

    // 4. Send public key to user through secure channel
    // ... (external communication happens here) ...

    // 5. Receive encrypted environment variables from user
    let encrypted_env = receive_from_user();

    // 6. Deploy with encrypted environment
    let deployment = client
        .deploy_with_config(vm_config, &user_secrets, pubkey, salt)
        .await?;

    println!("Deployed: {}", deployment.id);
    Ok(())
}

// USER ACTIONS
// The user has the secrets but doesn't need API access
fn user_workflow(pubkey: &str, salt: &str) -> String {
    // 1. Prepare sensitive environment variables
    let secrets = vec![
        ("API_KEY".to_string(), "secret-api-key".to_string()),
        ("DATABASE_PASSWORD".to_string(), "db-password".to_string()),
    ];

    // 2. Encrypt environment variables using public key
    let encryptor = create_encryptor(pubkey, salt);
    let encrypted_data = encrypt_env_vars(encryptor, &secrets);

    // 3. Send encrypted data back to operator
    // ... (external communication happens here) ...

    encrypted_data
}
```

### Simplified Deployment

The library provides a simplified approach which handles many steps internally:

```rust
use phala_tee_deploy_rs::{DeploymentConfig, TeeClient};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv::dotenv().ok();

    // Set up your Docker Compose content
    let docker_compose = r#"
services:
  app:
    image: your-image:tag
    # Your Docker Compose configuration
    environment:
      - ENV_VAR=${ENV_VAR}
    # Other configuration...
"#;

    // Set up environment variables for the deployment
    let mut env_vars = HashMap::new();
    env_vars.insert("ENV_VAR".to_string(),
                     std::env::var("ENV_VAR").expect("ENV_VAR must be set"));
    // Add other environment variables...

    // Create deployment configuration
    let config = DeploymentConfig::new(
        std::env::var("PHALA_CLOUD_API_KEY").expect("PHALA_CLOUD_API_KEY must be set"),
        docker_compose.to_string(),
        env_vars,
        std::env::var("PHALA_TEEPOD_ID")
            .expect("PHALA_TEEPOD_ID must be set")
            .parse::<u64>()?,
        "your-image:tag".to_string(),
    );

    // Deploy
    let client = TeeClient::new(config)?;
    let deployment = client.deploy().await?;

    println!("Deployment ID: {}", deployment.id);
    println!("Status: {}", deployment.status);

    Ok(())
}
```

### Updating Existing Deployments

You can update an existing deployment with new configurations or environment variables:

```rust
use phala_tee_deploy_rs::{DeploymentConfig, TeeClient};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv::dotenv().ok();

    // Get app_id from command line or environment
    let app_id = std::env::args().nth(1)
        .or_else(|| std::env::var("PHALA_APP_ID").ok())
        .expect("App ID must be provided");

    // Initialize client with minimal config
    let config = DeploymentConfig {
        api_key: std::env::var("PHALA_CLOUD_API_KEY").expect("API key not set"),
        api_url: std::env::var("PHALA_CLOUD_API_ENDPOINT")
            .unwrap_or_else(|_| "https://cloud-api.phala.network/api/v1".to_string()),
        docker_compose: String::new(), // Not needed for updates
        env_vars: HashMap::new(),      // Not needed for updates
        teepod_id: 0,                 // Not needed for updates
        image: String::new(),         // Not needed for updates
        vm_config: None,              // Not needed for updates
    };

    let client = TeeClient::new(config)?;

    // Get current configuration
    let compose = client.get_compose(&app_id).await?;

    // Modify the configuration
    let mut compose_file = compose.compose_file;
    compose_file["your_setting_to_change"] = serde_json::json!("new_value");

    // Update with new environment variables (optional)
    let mut env_vars = HashMap::new();
    env_vars.insert("NEW_ENV_VAR".to_string(), "value".to_string());

    // Apply the update
    let update_response = client.update_compose(
        &app_id,
        compose_file,
        Some(env_vars),
        compose.env_pubkey,
    ).await?;

    println!("Deployment updated successfully!");

    Ok(())
}
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
