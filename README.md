# Phala TEE Deployment Toolkit

A Rust library for deploying Docker Compose applications to the Phala TEE (Trusted Execution Environment) Cloud with secure environment variable handling.

## Overview

Phala TEE Cloud runs applications in hardware-enforced isolated enclaves, providing enhanced security guarantees. This toolkit simplifies deployments to the Phala Cloud with:

- **Environment Variable Encryption** - Secure handling of sensitive data
- **Docker Compose Support** - Deploy multi-container applications
- **Flexible Deployment Patterns** - From simple one-step to advanced privilege separation

## Getting Started

### Prerequisites

- Phala Cloud account with API access
- A TEE pod ID from your Phala account
- Docker Compose configuration for your application

### Environment Setup

Create a `.env` file with:

```
PHALA_CLOUD_API_KEY=your-api-key
PHALA_CLOUD_API_ENDPOINT=https://cloud-api.phala.network/api/v1
PHALA_TEEPOD_ID=your-teepod-id
PHALA_APP_ID=your-app-id  # For updating existing deployments
```

## Deployment Patterns

### Pattern 1: Simple Deployment

Deploy with a single function call. Best for straightforward applications where simplicity is key:

```rust
let config = DeploymentConfig::new(
    std::env::var("PHALA_CLOUD_API_KEY")?,
    docker_compose_content,
    environment_variables,
    std::env::var("PHALA_TEEPOD_ID")?.parse()?,
    "phala-worker:latest".to_string(),
);

let client = TeeClient::new(config)?;
let deployment = client.deploy().await?;
println!("Deployed: {}", deployment.id);
```

### Pattern 2: Step-by-Step Deployment

For when you need full control over the deployment process:

```rust
// 1. Initialize client
let client = TeeClient::new(minimal_config)?;

// 2. Get available TEEPods
let teepods = client.get_available_teepods().await?;
let teepod_id = teepods["nodes"][0]["teepod_id"].as_u64()?;
let image = teepods["nodes"][0]["images"][0]["name"].as_str()?;

// 3. Prepare VM configuration
let vm_config = json!({
    "name": "my-app",
    "compose_manifest": { "docker_compose_file": docker_compose },
    "teepod_id": teepod_id,
    "image": image
    // Additional VM configuration...
});

// 4. Get encryption keys
let pubkey_response = client.get_pubkey_for_config(&vm_config).await?;

// 5. Deploy with environment variables
let deployment = client.deploy_with_config_do_encrypt(
    vm_config,
    &env_vars,
    pubkey_response["app_env_encrypt_pubkey"].as_str()?,
    pubkey_response["app_id_salt"].as_str()?
).await?;
```

### Pattern 3: Privilege Separation

Security-focused approach where operators handle infrastructure while users manage secrets:

```rust
// OPERATOR: Has API access, doesn't see secrets
// 1. Get available TEEPods
let teepods = client.get_available_teepods().await?;

// 2. Prepare VM configuration
let vm_config = json!({ /* configuration */ });

// 3. Get encryption key
let pubkey = client.get_pubkey_for_config(&vm_config).await?;

// 4. Send pubkey to user through secure channel
send_to_user(pubkey);

// 5. Receive encrypted variables from user
let encrypted_env = receive_from_user();

// 6. Deploy with encrypted environment
client.deploy_with_config_encrypted_env(
    vm_config, encrypted_env, pubkey, salt
).await?;

// USER: Has secrets, doesn't need API access
// 1. Receives pubkey from operator
// 2. Encrypts environment variables
let encrypted = Encryptor::encrypt_env_vars(&secrets, pubkey)?;
// 3. Sends encrypted data to operator
send_to_operator(encrypted);
```

### Pattern 4: Updating Deployments

Update existing deployments with new configurations or environment variables:

```rust
// 1. Get current configuration
let compose = client.get_compose(&app_id).await?;

// 2. Modify configuration and environment variables
let mut compose_file = compose.compose_file;
compose_file["services"]["app"]["image"] = json!("new-image:tag");

// 3. Apply update
client.update_compose(
    &app_id,
    compose_file,
    Some(new_env_vars),
    compose.env_pubkey
).await?;
```

## API Reference

- `DeploymentConfig` - Core configuration for deployments
- `TeeClient` - API client with methods for deployment operations
- `Encryptor` - Handles secure encryption of environment variables

## Examples

See the `examples/` directory for full working examples:

- `typescript_equivalent.rs` - Step-by-step deployment matching TypeScript workflow
- `pubkey_workflow.rs` - Public key encryption workflow
- `operator_user_flow.rs` - Privilege separation pattern
- `update_deployment.rs` - Updating existing deployments

## License

MIT License
