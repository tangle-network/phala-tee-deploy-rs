use phala_tee_deploy_rs::{DeploymentConfig, Result, TeeClient};
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

    // Get app_id from command line or environment
    let app_id = env::args()
        .nth(1)
        .or_else(|| env::var("PHALA_APP_ID").ok())
        .expect("App ID must be provided as argument or PHALA_APP_ID env var");

    // Create a minimal config for API access
    let config = DeploymentConfig {
        api_key,
        api_url: api_endpoint,
        docker_compose: String::new(), // Not needed for updates
        env_vars: HashMap::new(),      // Not needed for updates
        teepod_id: 0,                  // Not needed for updates
        image: String::new(),          // Not needed for updates
        vm_config: None,               // Not needed for updates
    };

    let client = TeeClient::new(config)?;

    // Step 1: Get current compose manifest
    println!("Fetching current deployment configuration...");
    let compose_response = client.get_compose(&app_id).await?;
    println!("Current compose manifest retrieved.");

    // Step 2: Adjust the compose file
    let mut compose_file = compose_response.compose_file;
    compose_file["pre_launch_script"] = serde_json::json!(
        r#"
#!/bin/bash
echo "--------------------------------"
echo "Hello again, DSTACK!"
echo "--------------------------------"
echo
env
echo "--------------------------------"
    "#
    );

    // Step 3 (optional): Prepare environment variables to encrypt
    let mut env_vars = HashMap::new();
    env_vars.insert("FOO".to_string(), "BAR".to_string());

    // Step 4: Update the compose file
    println!("Updating deployment configuration...");
    let update_response = client
        .update_compose(
            &app_id,
            compose_file,
            Some(env_vars),
            compose_response.env_pubkey,
        )
        .await?;

    println!("Deployment updated successfully!");
    println!("Response: {:?}", update_response);

    Ok(())
}
