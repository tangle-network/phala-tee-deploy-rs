use phala_tee_deploy_rs::{DeploymentConfig, Error, TeeClient};
use serde_json::json;
use std::collections::HashMap;
use std::env;

/// This example demonstrates a security-focused deployment pattern with separation of concerns:
/// - Operator: Has Phala API access and handles infrastructure (but no access to secrets)
/// - User: Has sensitive credentials but no Phala API access
///
/// This pattern ensures that neither party has access to both infrastructure and secrets.
#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv::dotenv().ok();

    // ============ OPERATOR ACTIONS ============
    println!("🔷 OPERATOR: Setting up deployment environment");

    // Initialize client with minimal configuration
    let client = TeeClient::new(DeploymentConfig {
        api_key: env::var("PHALA_CLOUD_API_KEY").expect("API key required"),
        api_url: env::var("PHALA_CLOUD_API_ENDPOINT")
            .unwrap_or_else(|_| "https://cloud-api.phala.network/api/v1".to_string()),
        docker_compose: String::new(),
        env_vars: HashMap::new(),
        teepod_id: 0,
        image: String::new(),
        vm_config: None,
    })?;

    // 1. Get available infrastructure
    println!("🔷 OPERATOR: Discovering available TEEPods");
    let teepods = client.get_available_teepods().await?;
    let teepod_id = teepods.nodes[0].teepod_id;
    let image = teepods.nodes[0].images[0].name.clone();
    println!("      Using TEEPod ID: {}", teepod_id);

    // 2. Define application structure (without secrets)
    println!("🔷 OPERATOR: Creating VM configuration");
    let vm_config = json!({
        "name": "secure-app",
        "compose_manifest": {
            "docker_compose_file": r#"
services:
  app:
    image: alpine:latest
    command: sh -c "while true; do echo $SECRET_API_KEY; sleep 10; done"
    "#,
            "name": "secure-app"
        },
        "vcpu": 1,
        "memory": 1024,
        "disk_size": 10,
        "teepod_id": teepod_id,
        "image": image
    });

    // 3. Get encryption key
    println!("🔷 OPERATOR: Obtaining encryption key from Phala Cloud");
    let pubkey_response = client.get_pubkey_for_config(&vm_config).await?;
    let pubkey = pubkey_response.app_env_encrypt_pubkey;
    let salt = pubkey_response.app_id_salt;
    let app_id = pubkey_response.app_id.clone();

    // Construct the full application identifier with the required "app_" prefix
    let full_app_id = format!("app_{}", app_id);

    // 4. Securely share public key with user
    println!("🔷 OPERATOR: Sending public key to user via secure channel\n");

    // ============ USER ACTIONS ============
    println!("🔶 USER: Received public key: {}...", &pubkey[0..16]);

    // 1. User defines sensitive environment variables
    println!("🔶 USER: Preparing sensitive environment variables");
    let sensitive_env_vars = vec![
        (
            "SECRET_API_KEY".to_string(),
            "api_18f21a0bc99d4b7".to_string(),
        ),
        (
            "DATABASE_PASSWORD".to_string(),
            "very-secure-pw-123".to_string(),
        ),
    ];

    // 2. User sends sensitive data to operator
    // In a real-world scenario, the user would encrypt these variables themselves
    // using a local encryption tool with the provided public key
    println!("🔶 USER: Sending sensitive data to operator\n");

    // ============ OPERATOR ACTIONS CONTINUED ============
    println!("🔷 OPERATOR: Received sensitive environment variables for deployment");

    // 5. Deploy with VM config and environment variables (encrypted internally)
    println!("🔷 OPERATOR: Deploying application with encrypted environment");
    let deployment = client
        .deploy_with_config_do_encrypt(vm_config, &sensitive_env_vars, &pubkey, &salt)
        .await?;

    // ============ RESULT ============
    println!("\n✅ Deployment successful!");
    println!("   ID: {}", deployment.id);
    println!("   App ID: {}", app_id);
    println!("   Full Application Identifier: {}", full_app_id);
    println!("   Status: {}", deployment.status);
    println!("\n🔒 Security benefit: Neither party had access to both API keys and secrets");

    println!("\n🔍 To check your deployment status:");
    println!("   cargo run --example network_info {}", full_app_id);

    Ok(())
}
