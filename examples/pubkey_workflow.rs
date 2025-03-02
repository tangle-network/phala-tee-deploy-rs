use dotenv::dotenv;
use phala_tee_deploy_rs::{DeploymentConfig, Encryptor, Error, TeeClient};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;

/// This example demonstrates the precise 3-step workflow:
/// 1. Operator calls /cvms/pubkey/from_cvm_configuration to retrieve the public key
/// 2. Operator sends the public key to a user (simulated), who encrypts the secret and sends back
/// 3. Operator calls /cvms/from_cvm_configuration with the same payload and encrypted environment
#[tokio::main]
async fn main() -> Result<(), Error> {
    // Load environment variables
    dotenv().ok();

    // Set up the API client configuration
    let config = DeploymentConfig {
        api_key: env::var("PHALA_API_KEY").expect("PHALA_API_KEY must be set"),
        api_url: env::var("PHALA_API_ENDPOINT")
            .unwrap_or_else(|_| "https://cloud-api.phala.network/api/v1".to_string()),
        docker_compose: String::new(),
        env_vars: HashMap::new(),
        teepod_id: 0,
        image: String::new(),
        vm_config: None,
    };

    let client = TeeClient::new(config)?;

    // STEP 1: Create a VM configuration to be deployed
    // In a real scenario this would be your custom configuration
    println!("Step 1: Creating VM configuration");

    // For simplicity we'll use a minimal example
    let vm_config = json!({
        "name": "pubkey-workflow-example",
        "compose_manifest": {
            "docker_compose_file": r#"
version: '3'
services:
  app:
    image: nginx:alpine
    ports:
      - "8080:80"
    environment:
      - SECRET_TOKEN
"#,
            "name": "nginx-app"
        },
        "teepod_id": 123,  // This would be a real TEEPod ID in production
        "image": "phala/teepod:latest"  // This would be a real image in production
    });

    // STEP 2: Call /cvms/pubkey/from_cvm_configuration to get the public key
    println!("Step 2: Retrieving public key for encryption");
    let pubkey_response = client.get_pubkey_for_config(&vm_config).await?;

    let public_key = pubkey_response["app_env_encrypt_pubkey"].as_str().unwrap();
    let salt = pubkey_response["app_id_salt"].as_str().unwrap();

    println!("Retrieved public key: {}", public_key);
    println!("Retrieved salt: {}", salt);

    // STEP 3: In a real scenario, the operator would send this public key to the user
    // who would encrypt their secrets and send back the encrypted data
    println!("\nStep 3: [SIMULATION] Sending public key to user");
    println!("In a real scenario, this would be sent through a secure channel.");
    println!("The user would then encrypt their secrets with this key.\n");

    // For this example, we'll simulate the user's environment variables
    let user_secrets = vec![
        (
            "SECRET_TOKEN".to_string(),
            "user-secret-value-123".to_string(),
        ),
        ("API_KEY".to_string(), "user-api-key-456".to_string()),
    ];
    // Encrypt the environment variables
    let encrypted_env = Encryptor::encrypt_env_vars(&user_secrets, public_key)?;

    // STEP 4: Call /cvms/from_cvm_configuration with the VM config and encrypted env
    println!("Step 4: Deploying with the VM configuration and encrypted environment");
    let deployment = client
        .deploy_with_config_encrypted_env(vm_config, encrypted_env, public_key, salt)
        .await?;

    println!("\nDeployment successful!");
    println!("Deployment ID: {}", deployment.id);
    println!("Status: {}", deployment.status);

    println!("\nWorkflow complete. The deployment is now running in the TEE environment.");

    Ok(())
}
