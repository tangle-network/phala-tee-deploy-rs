use base64;
use phala_tee_deploy_rs::{DeploymentConfig, TeeClient};
use std::collections::HashMap;
use std::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv::dotenv().ok();

    // Read the C-3PO character configuration file
    let character_data = fs::read_to_string("examples/c3po.character.json")?;
    #[allow(deprecated)]
    let character_data_base64 = base64::encode(character_data.as_bytes());

    // Create Docker compose configuration for Eliza with C-3PO character
    let docker_compose = r#"
services:
  eliza:
    image: phalanetwork/eliza:v0.1.7-alpha.2
    container_name: eliza
    command:
      - /bin/sh
      - -c
      - |
        cd /app
        echo $${CHARACTER_DATA} | base64 -d > characters/c3po.character.json
        pnpm run start --non-interactive --character=characters/c3po.character.json
    ports:
      - "3000:3000"
    volumes:
      - /var/run/tappd.sock:/var/run/tappd.sock
      - tee:/app/db.sqlite
    environment:
      - TEE_MODE=PRODUCTION
      - REDPILL_API_KEY=${REDPILL_API_KEY}
      - REDPILL_MODEL=gpt-4o-mini
      - TELEGRAM_BOT_TOKEN=${TELEGRAM_BOT_TOKEN}
      - WALLET_SECRET_SALT=${WALLET_SECRET_SALT}
      - CHARACTER_DATA=${CHARACTER_DATA}
    restart: always

volumes:
    tee:
"#;

    // Set up environment variables for the deployment
    let mut env_vars = HashMap::new();
    env_vars.insert(
        "REDPILL_API_KEY".to_string(),
        std::env::var("REDPILL_API_KEY").expect("REDPILL_API_KEY must be set"),
    );
    env_vars.insert(
        "TELEGRAM_BOT_TOKEN".to_string(),
        std::env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN must be set"),
    );
    env_vars.insert(
        "WALLET_SECRET_SALT".to_string(),
        std::env::var("WALLET_SECRET_SALT").expect("WALLET_SECRET_SALT must be set"),
    );
    env_vars.insert("CHARACTER_DATA".to_string(), character_data_base64);

    // Get TEE pod ID and API key from environment variables
    let teepod_id = std::env::var("PHALA_TEEPOD_ID")
        .expect("PHALA_TEEPOD_ID must be set")
        .parse::<u64>()?;

    let api_key = std::env::var("PHALA_CLOUD_API_KEY").expect("PHALA_CLOUD_API_KEY must be set");

    // Create deployment configuration
    let config = DeploymentConfig::new(
        api_key,
        docker_compose.to_string(),
        env_vars,
        teepod_id,
        "phalanetwork/eliza:v0.1.7-alpha.2".to_string(),
    );

    // Create client and deploy
    let client = TeeClient::new(config)?;
    let deployment = client.deploy().await?;

    println!("Eliza C-3PO Agent Deployment created successfully!");
    println!("ID: {}", deployment.id);
    println!("Status: {}", deployment.status);
    if let Some(details) = deployment.details {
        println!("Details: {:#?}", details);
    }

    println!(
        "\nDeployment complete. Your Eliza C-3PO agent is now running in the Phala TEE Cloud."
    );
    println!(
        "You can check its status in the Phala Cloud dashboard using the deployment ID above."
    );

    Ok(())
}
