use dockworker::config::compose::{ComposeConfig, Service};
use dockworker::config::volume::Volume;
use phala_tee_deploy_rs::{TeeDeployer, TeeDeployerBuilder};
use std::collections::HashMap;
use std::env;

/// This example demonstrates how to use the TeeDeployer with dockworker
/// to simplify the deployment of Docker Compose applications to Phala TEE Cloud.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv::dotenv().ok();

    println!("=== Phala TEE Deployment with Dockworker Integration ===\n");

    // Create the deployer with builder pattern
    let mut deployer = TeeDeployerBuilder::new()
        .with_api_key(env::var("PHALA_CLOUD_API_KEY").expect("API key required"))
        .with_api_endpoint(
            env::var("PHALA_CLOUD_API_ENDPOINT")
                .unwrap_or_else(|_| "https://cloud-api.phala.network/api/v1".to_string()),
        )
        .build()?;

    // Discover an available TEEPod
    println!("🔍 Discovering available TEEPods...");
    deployer.discover_teepod().await?;

    // ===== APPROACH 1: Create a custom Docker Compose configuration =====
    println!("\nMethod 1: Creating and deploying a custom Docker Compose configuration");

    // Create a Docker Compose configuration with multiple services
    let mut compose_config = ComposeConfig::default();

    // Create the API service
    let mut api_service = Service::default();
    api_service.image = Some("node:18-alpine".to_string());
    api_service.command = Some(vec!["node".to_string(), "server.js".to_string()]);
    api_service.ports = Some(vec!["3000:3000".to_string()]);

    let mut api_env = HashMap::new();
    api_env.insert("NODE_ENV".to_string(), "production".to_string());
    api_env.insert("DB_HOST".to_string(), "db".to_string());
    api_service.environment = Some(api_env.into());

    // Create the database service
    let mut db_service = Service::default();
    db_service.image = Some("mongo:latest".to_string());
    db_service.volumes = Some(vec![Volume::Named("db-data:/data/db".to_string())]);

    let mut db_env = HashMap::new();
    db_env.insert(
        "MONGO_INITDB_ROOT_USERNAME".to_string(),
        "admin".to_string(),
    );
    db_env.insert(
        "MONGO_INITDB_ROOT_PASSWORD".to_string(),
        "secure_password".to_string(),
    );
    db_service.environment = Some(db_env.into());

    // Add services to the config
    compose_config
        .services
        .insert("api".to_string(), api_service);
    compose_config.services.insert("db".to_string(), db_service);

    // Add persistent volume configuration
    compose_config.volumes.insert(
        "db-data".to_string(),
        Volume::Config {
            name: "db-data".to_string(),
            driver: None,
            driver_opts: None,
        },
    );

    // Prepare deployment environment variables
    let mut deployment_env = HashMap::new();
    deployment_env.insert("API_KEY".to_string(), "secret_api_key_value".to_string());
    deployment_env.insert("JWT_SECRET".to_string(), "my_jwt_secret_value".to_string());

    // Deploy the compose configuration
    println!("🚀 Deploying custom Docker Compose configuration...");
    let result = deployer
        .deploy_compose(
            &compose_config,
            "node-mongo-app",
            deployment_env,
            Some(2),    // vCPUs
            Some(2048), // Memory (MB)
            Some(20),   // Disk size (GB)
        )
        .await?;

    println!("✅ Deployment successful!");
    println!("    ID: {}", result["id"]);
    println!("    Status: {}\n", result["status"]);

    // ===== APPROACH 2: Deploy from a Docker Compose file string =====
    println!("\nMethod 2: Deploying from a Docker Compose YAML string");

    // Create a Docker Compose YAML string
    let yaml_content = r#"
version: '3'
services:
  web:
    image: nginx:latest
    ports:
      - "80:80"
    volumes:
      - ./site:/usr/share/nginx/html
  
  redis:
    image: redis:alpine
    ports:
      - "6379:6379"
"#;

    // Prepare environment variables
    let mut env_vars = HashMap::new();
    env_vars.insert("NGINX_HOST".to_string(), "example.com".to_string());
    env_vars.insert("NGINX_PORT".to_string(), "80".to_string());

    // Deploy from the YAML string
    println!("🚀 Deploying from YAML string...");
    let result = deployer
        .deploy_compose_from_string(
            yaml_content,
            "nginx-redis-app",
            env_vars,
            Some(1),    // vCPUs
            Some(1024), // Memory (MB)
            Some(10),   // Disk size (GB)
        )
        .await?;

    println!("✅ Deployment successful!");
    println!("    ID: {}", result["id"]);
    println!("    Status: {}\n", result["status"]);

    println!("\n=== All deployments completed successfully ===");

    Ok(())
}
