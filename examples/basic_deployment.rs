use phala_tee_deploy_rs::{DeploymentConfig, TeeClient};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv::dotenv().ok();

    // Create Docker compose configuration
    let docker_compose = r#"
services:
  app:
    image: ${IMAGE}
    environment:
      - API_KEY=${API_KEY}
      - DEBUG=${DEBUG}
    ports:
      - "8080:8080"
    volumes:
      - /data:/app/data
"#;

    // Set up environment variables for the deployment
    let mut env_vars = HashMap::new();
    env_vars.insert("IMAGE".to_string(), "my-app:latest".to_string());
    env_vars.insert("API_KEY".to_string(), "secret-key".to_string());
    env_vars.insert("DEBUG".to_string(), "true".to_string());

    // Create deployment configuration
    let config = DeploymentConfig::new(
        std::env::var("PHALA_CLOUD_API_KEY")?,
        docker_compose.to_string(),
        env_vars,
        1, // teepod_id
        "phala-node:latest".to_string(),
    );

    // Create client and deploy
    let client = TeeClient::new(config)?;
    let deployment = client.deploy().await?;

    println!("Deployment created successfully!");
    println!("ID: {}", deployment.id);
    println!("Status: {}", deployment.status);
    if let Some(details) = deployment.details {
        println!("Details: {:#?}", details);
    }

    Ok(())
}
