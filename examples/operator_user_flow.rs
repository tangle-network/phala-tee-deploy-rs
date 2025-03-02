use dotenv::dotenv;
use phala_tee_deploy_rs::{DeploymentConfig, Error, TeeClient};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;

/// This example demonstrates a workflow where:
/// 1. An operator retrieves a public key from the Phala API
/// 2. The operator shares the public key with a user who encrypts sensitive data
/// 3. The operator deploys the application with the encrypted environment variables
#[tokio::main]
async fn main() -> Result<(), Error> {
    // Load environment variables from .env file
    dotenv().ok();

    // Set up the client configuration
    let config = DeploymentConfig {
        api_key: env::var("PHALA_API_KEY").expect("PHALA_API_KEY must be set"),
        api_url: env::var("PHALA_API_ENDPOINT")
            .unwrap_or_else(|_| "https://cloud-api.phala.network/api/v1".to_string()),
        docker_compose: String::new(), // Not used in this example
        env_vars: HashMap::new(),      // Not used in this example
        teepod_id: 0,                  // Will select from available TEEPods
        image: String::new(),          // Will select from available TEEPods
        vm_config: None,               // Will create custom VM config
    };

    let client = TeeClient::new(config)?;

    // STEP 1: Operator retrieves available TEEPods
    println!("Fetching available TEEPods...");
    let teepods = client.get_available_teepods().await?;

    // For this example, we'll just take the first available TEEPod
    let teepod_id = teepods["nodes"][0]["teepod_id"].as_u64().unwrap();
    let image = teepods["nodes"][0]["images"][0]["name"].as_str().unwrap();

    println!("Selected TEEPod ID: {}, Image: {}", teepod_id, image);

    // Create VM configuration
    let vm_config = json!({
        "name": "operator-user-example",
        "compose_manifest": {
            "docker_compose_file": r#"
version: '3'
services:
  app:
    image: busybox
    command: sh -c "while true; do echo $SECRET_MESSAGE; sleep 5; done"
"#,
            "name": "secure-app"
        },
        "teepod_id": teepod_id,
        "image": image
    });

    // STEP 2: Operator retrieves encryption public key
    println!("Retrieving public key for encryption...");
    let pubkey_response = client.get_pubkey_for_config(&vm_config).await?;

    let pubkey = pubkey_response["app_env_encrypt_pubkey"].as_str().unwrap();
    let salt = pubkey_response["app_id_salt"].as_str().unwrap();

    println!("Received public key: {}", pubkey);
    println!("Received salt: {}", salt);

    // -----------------------------------------------------------------
    // STEP 3: Operator sends public key to the user
    // This would happen outside of this code, e.g., via a secure channel
    println!("\n--- SIMULATING USER ACTION ---");
    println!("User received public key: {}", pubkey);

    // The user would encrypt their sensitive data with the public key
    // For this example, we'll simulate this by encrypting it ourselves

    // Normally, the user would do this part on their own machine:
    let user_secrets = vec![
        (
            "SECRET_MESSAGE".to_string(),
            "This is a secret message!".to_string(),
        ),
        ("API_KEY".to_string(), "user-secret-api-key".to_string()),
    ];

    // In a real scenario, the user would:
    // 1. Create an Encryptor instance
    // 2. Encrypt their environment variables using the public key
    // 3. Send the encrypted data back to the operator

    println!("User encrypts environment variables with the public key");
    println!("User sends back encrypted data to operator");
    println!("--- END USER SIMULATION ---\n");
    // -----------------------------------------------------------------

    // STEP 4: Operator receives the encrypted environment variables from the user
    // In a real scenario, the encrypted data would come from the user
    // For this example, we'll simulate by encrypting it here
    println!("Operator received encrypted environment from user");

    // STEP 5: Operator deploys with the VM configuration and encrypted environment
    println!("Deploying application with encrypted environment...");
    let deployment = client
        .deploy_with_config_do_encrypt(vm_config, &user_secrets, pubkey, salt)
        .await?;

    println!("Deployment successful!");
    println!("Deployment ID: {}", deployment.id);
    println!("Status: {}", deployment.status);

    Ok(())
}
