use phala_tee_deploy_rs::{DeploymentConfig, Result, TeeClient};
use serde_json::json;
use std::collections::HashMap;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv::dotenv().ok();

    // Get API key and endpoint from environment
    let api_key = env::var("PHALA_CLOUD_API_KEY").expect("PHALA_CLOUD_API_KEY must be set");
    let api_endpoint = env::var("PHALA_CLOUD_API_ENDPOINT")
        .unwrap_or_else(|_| "https://cloud-api.phala.network/api/v1".to_string());

    // Create a minimal config for API access
    let config = DeploymentConfig {
        api_key,
        api_url: api_endpoint,
        docker_compose: String::new(),
        env_vars: HashMap::new(),
        teepod_id: 0,
        image: String::new(),
        vm_config: None,
    };

    let client = TeeClient::new(config)?;

    // Step 1: Get available TEEPods
    println!("Fetching available TEEPods...");
    let teepods_response = client.get_available_teepods().await?;

    if teepods_response["nodes"]
        .as_array()
        .map_or(true, |nodes| nodes.is_empty())
    {
        println!("No available TEEPods found");
        return Ok(());
    }

    let node = &teepods_response["nodes"][0];
    let teepod_id = node["teepod_id"].as_u64().expect("Invalid TEEPod ID");
    let image = node["images"][0]["name"]
        .as_str()
        .expect("Invalid image name")
        .to_string();

    println!("Using TEEPod ID: {}, Image: {}", teepod_id, image);

    // Define Docker compose content
    let docker_compose = r#"
services:
  demo:
    image: leechael/bun-webserver-demo:latest
    container_name: demo
    ports:
      - "3000:3000"
    volumes:
      - /var/run/tappd.sock:/var/run/tappd.sock
"#;

    // Optional pre-launch script
    let pre_launch_script = r#"
#!/bin/bash
echo "--------------------------------"
echo "Hello, DSTACK!"
echo "--------------------------------"
echo
env
echo
echo "--------------------------------"
"#;

    // Create VM configuration
    let vm_config = json!({
        "name": "test",
        "compose_manifest": {
            "docker_compose_file": docker_compose,
            "pre_launch_script": pre_launch_script,
            "name": "test"
        },
        "vcpu": 1,
        "memory": 1024,
        "disk_size": 10,
        "teepod_id": teepod_id,
        "image": image
    });

    // Step 2: Get encryption public key
    println!("Getting encryption public key...");
    let pubkey_response = client.get_pubkey_for_config(&vm_config).await?;

    // Step 3: Prepare environment variables to encrypt
    let env_vars = vec![("FOO".to_string(), "BAR".to_string())];

    // Step 4: Deploy with encrypted environment variables
    println!("Deploying VM...");
    let deployment = client
        .deploy_with_config(
            vm_config,
            &env_vars,
            pubkey_response["app_env_encrypt_pubkey"].as_str().unwrap(),
            pubkey_response["app_id_salt"].as_str().unwrap(),
        )
        .await?;

    println!("Deployment successful!");
    println!("ID: {}", deployment.id);
    println!("Status: {}", deployment.status);
    if let Some(details) = deployment.details {
        println!("Details: {:#?}", details);
    }

    Ok(())
}
