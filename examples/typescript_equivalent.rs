use phala_tee_deploy_rs::{DeploymentConfig, Error, Result, TeeClient};
use serde_json::json;
use std::collections::HashMap;
use std::env;

/// This example demonstrates a step-by-step deployment process that mirrors
/// the TypeScript workflow, providing full control over each phase.
#[tokio::main]
async fn main() -> Result<()> {
    // ===== SETUP =====
    dotenv::dotenv().ok();

    // Create a minimal client with just the API credentials
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

    // ===== PHASE 1: SELECT TEEPOD =====
    println!("1. Discovering available TEEPods...");
    let teepods = client.get_available_teepods().await?;

    // Ensure we have TEEPods available
    if teepods.nodes.is_empty() {
        return Err(Error::Api {
            status_code: 400,
            message: "No available TEEPods found".into(),
        });
    }

    // Select the first available TEEPod and image
    let node = &teepods.nodes[0];
    let teepod_id = node.teepod_id;
    let image = node.images[0].name.clone();
    println!("   Selected TEEPod ID: {}, Image: {}", teepod_id, image);

    // ===== PHASE 2: PREPARE CONFIGURATION =====
    println!("2. Preparing VM configuration...");

    // Define Docker compose content
    let docker_compose = r#"
services:
  app:
    image: leechael/phala-cloud-nextjs-starter:latest
    ports:
      - "3000:3000"
    volumes:
      - /var/run/tappd.sock:/var/run/tappd.sock
"#;

    // Create VM configuration using JSON for flexibility
    let vm_config = json!({
        "name": "test-deployment",
        "compose_manifest": {
            "docker_compose_file": docker_compose,
            "name": "test-deployment",
            "features": ["kms", "tproxy-net"]
        },
        "vcpu": 1,
        "memory": 1024,
        "disk_size": 10,
        "teepod_id": teepod_id,
        "image": image,
        "advanced_features": {
            "tproxy": true,
            "kms": true,
            "public_sys_info": true,
            "public_logs": true,
            "docker_config": {
                "username": "",
                "password": "",
                "registry": null
            },
            "listed": false
        }
    });

    // ===== PHASE 3: OBTAIN ENCRYPTION KEYS =====
    println!("3. Obtaining encryption public key...");
    let pubkey_response = client.get_pubkey_for_config(&vm_config).await?;

    // Access the strongly typed response
    let pubkey = &pubkey_response.app_env_encrypt_pubkey;
    let salt = &pubkey_response.app_id_salt;
    let app_id = pubkey_response.app_id.clone();

    // Construct the full application identifier with the required "app_" prefix
    let full_app_id = format!("app_{}", app_id);

    println!("   Received public key: {}", pubkey);
    println!("   App ID: {}", app_id);
    println!("   Full Application Identifier: {}", full_app_id);

    // ===== PHASE 4: PREPARE AND ENCRYPT ENVIRONMENT =====
    println!("4. Preparing environment variables...");
    let env_vars = vec![
        ("API_KEY".to_string(), "secret-value".to_string()),
        ("DEBUG".to_string(), "true".to_string()),
    ];

    // ===== PHASE 5: DEPLOY =====
    println!("5. Deploying to TEE environment...");
    let deployment = client
        .deploy_with_config_do_encrypt(vm_config, &env_vars, pubkey, salt)
        .await?;

    // ===== RESULT =====
    println!("\n✅ Deployment successful!");
    println!("   ID: {}", deployment.id);
    println!("   Status: {}", deployment.status);

    if let Some(details) = &deployment.details {
        if let Some(app_id_value) = details.get("app_id") {
            println!("   App ID: {}", app_id_value);
            println!("   Full Application Identifier: app_{}", app_id_value);
            println!("\n✨ You can check the network information using:");
            println!("   cargo run --example network_info app_{}", app_id_value);
        }
    } else {
        println!("   App ID: {}", app_id);
        println!("   Full Application Identifier: {}", full_app_id);
        println!("\n✨ You can check the network information using:");
        println!("   cargo run --example network_info {}", full_app_id);
    }

    Ok(())
}
