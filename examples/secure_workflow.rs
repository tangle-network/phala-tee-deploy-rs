use phala_tee_deploy_rs::{Encryptor, Error, Result, TeeDeployerBuilder};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;

/// This example demonstrates a secure deployment workflow with separation of concerns:
///
/// 1. Operator prepares deployment and gets encryption public key
/// 2. User encrypts sensitive environment variables with the public key
/// 3. Operator deploys the application with the encrypted environment variables
///
/// This approach ensures that:
/// - The operator never sees the user's sensitive data
/// - The user doesn't need API credentials or access to TEE infrastructure
/// - The entire process remains secure with end-to-end encryption
#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file if present
    dotenv::dotenv().ok();

    // ======== OPERATOR ACTIONS (PART 1) ========
    println!("🔷 OPERATOR (Phase 1): Setting up infrastructure and obtaining public key");

    // Initialize deployer with API credentials
    let mut deployer = TeeDeployerBuilder::new()
        .with_api_key(env::var("PHALA_CLOUD_API_KEY").expect("PHALA_CLOUD_API_KEY must be set"))
        .with_api_endpoint(
            env::var("PHALA_CLOUD_API_ENDPOINT")
                .unwrap_or_else(|_| "https://cloud-api.phala.network/api/v1".to_string()),
        )
        .build()?;

    // Discover available TEEPods
    println!("🔍 Discovering available TEEPods...");
    deployer.discover_teepod().await?;

    // Create VM configuration from Docker Compose content
    println!("📄 Creating VM configuration...");
    let docker_compose = r#"
version: '3'
services:
  app:
    image: nginx:alpine
    ports:
      - "8080:80"
    environment:
      - DB_PASSWORD
      - API_SECRET
"#;

    let vm_config = deployer.create_vm_config_from_string(
        docker_compose,
        "secure-workflow-example",
        Some(1),    // 1 vCPU
        Some(1024), // 1 GB RAM
        Some(10),   // 10 GB disk
    )?;

    // Get the public key for this VM configuration
    println!("🔑 Requesting encryption public key...");
    let pubkey_response = deployer.get_pubkey_for_config(&vm_config).await?;

    let public_key = pubkey_response["app_env_encrypt_pubkey"]
        .as_str()
        .expect("Missing public key in response");

    let salt = pubkey_response["app_id_salt"]
        .as_str()
        .expect("Missing salt in response");

    println!("✅ Public key obtained: {}", public_key);
    println!("✅ Salt obtained: {}", salt);

    // At this point, the operator would securely send the public key to the user
    println!("\n======== SECURE CHANNEL ========");
    println!("Operator securely shares the public key with the user");
    println!("======== SECURE CHANNEL ========\n");

    // ======== USER ACTIONS ========
    println!("🔶 USER: Encrypting sensitive environment variables");

    // User's sensitive environment variables
    let user_env_vars = vec![
        (
            "DB_PASSWORD".to_string(),
            "super-secret-db-password".to_string(),
        ),
        ("API_SECRET".to_string(), "user-private-api-key".to_string()),
    ];

    // User encrypts their environment variables with the public key
    println!("🔐 Encrypting environment variables...");
    let encrypted_env = Encryptor::encrypt_env_vars(&user_env_vars, public_key)?;
    println!("✅ Environment variables encrypted successfully");

    // At this point, the user would securely send the encrypted env vars back to the operator
    println!("\n======== SECURE CHANNEL ========");
    println!("User securely shares the encrypted environment variables with the operator");
    println!("But never reveals the plaintext values");
    println!("======== SECURE CHANNEL ========\n");

    // ======== OPERATOR ACTIONS (PART 2) ========
    println!("🔷 OPERATOR (Phase 2): Deploying with encrypted environment variables");

    // Deploy with the VM configuration and encrypted environment variables
    println!("🚀 Deploying application...");
    let deployment = deployer
        .deploy_with_encrypted_env(vm_config, encrypted_env, public_key, salt)
        .await?;

    println!("\n✅ Deployment successful!");
    println!("Deployment ID: {}", deployment["id"]);
    println!("Status: {}", deployment["status"]);

    Ok(())
}
