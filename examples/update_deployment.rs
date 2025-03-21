use phala_tee_deploy_rs::{DeploymentConfig, Result, TeeClient};
use serde_json::json;
use std::collections::HashMap;
use std::env;

/// This example demonstrates how to update an existing deployment
/// with new configuration and environment variables
#[tokio::main]
async fn main() -> Result<()> {
    // ===== SETUP =====
    dotenv::dotenv().ok();

    // Get application ID from arguments or environment
    let app_id = env::args()
        .nth(1)
        .or_else(|| env::var("PHALA_APP_ID").ok())
        .expect("App ID required: provide as argument or PHALA_APP_ID env var");

    // Add the "app_" prefix if it's not already present
    let prefixed_app_id = if !app_id.starts_with("app_") {
        format!("app_{}", app_id)
    } else {
        app_id.clone()
    };

    println!("Using application identifier: {}", prefixed_app_id);

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

    // ===== PHASE 1: RETRIEVE CURRENT CONFIGURATION =====
    println!("1. Retrieving current deployment configuration...");
    let compose = client.get_compose(&prefixed_app_id).await?;

    // ===== PHASE 2: MODIFY CONFIGURATION =====
    println!("2. Modifying deployment configuration...");
    let mut compose_file = compose.compose_file;

    // Example: Update container image
    if let Some(services) = compose_file["services"].as_object_mut() {
        if let Some(app_service) = services.get_mut("app") {
            if app_service.is_object() {
                app_service["image"] = json!("updated-image:latest");
                println!("   Updated service image to 'updated-image:latest'");
            }
        }
    }

    // Example: Add or update environment variables
    println!("3. Preparing new environment variables...");
    let mut env_vars = HashMap::new();
    env_vars.insert("API_KEY".to_string(), "updated-api-key".to_string());
    env_vars.insert("DEBUG_MODE".to_string(), "true".to_string());

    // ===== PHASE 3: APPLY UPDATES =====
    println!("4. Applying updates to deployment...");
    let update_response = client
        .update_compose(
            &prefixed_app_id,
            compose_file,
            Some(env_vars),
            compose.env_pubkey,
        )
        .await?;

    // ===== RESULT =====
    println!("\n✅ Deployment updated successfully!");
    println!("   New configuration applied to: {}", prefixed_app_id);

    // Access the strongly typed response if available, otherwise show raw JSON
    if let Some(status) = update_response.get("status") {
        println!("   Update status: {}", status);
    }

    if let Some(message) = update_response.get("message") {
        println!("   Message: {}", message);
    }

    Ok(())
}
