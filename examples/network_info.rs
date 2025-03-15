use phala_tee_deploy_rs::{DeploymentConfig, Error, Result, TeeClient};
use serde_json::json;
use std::collections::HashMap;
use std::env;
use std::time::Duration;

/// This example demonstrates deploying an application and retrieving its network information
/// in the Phala TEE Cloud.
#[tokio::main]
async fn main() -> Result<()> {
    // ===== SETUP =====
    dotenv::dotenv().ok();

    // Get API key from environment or fail with a helpful message
    let api_key = env::var("PHALA_CLOUD_API_KEY")
        .expect("PHALA_CLOUD_API_KEY environment variable is required");

    // Optional custom API endpoint
    let api_url = env::var("PHALA_CLOUD_API_ENDPOINT")
        .unwrap_or_else(|_| "https://cloud-api.phala.network/api/v1".to_string());

    println!("=== Phala TEE Network Info Example ===\n");
    println!("Using API endpoint: {}", api_url);

    // Initialize client with minimal configuration
    let client = TeeClient::new(DeploymentConfig {
        api_key,
        api_url,
        docker_compose: String::new(),
        env_vars: HashMap::new(),
        teepod_id: 0,
        image: String::new(),
        vm_config: None,
    })?;

    // ===== STEP 1: DISCOVER TEEPOD =====
    println!("\n🔍 Discovering available TEEPods...");
    let teepods = client.get_available_teepods().await?;

    if teepods.nodes.is_empty() {
        return Err(Error::Configuration(
            "No available TEEPods found. Please check your Phala Cloud account.".into(),
        ));
    }

    // Choose the first available TEEPod and its first available image
    let node = &teepods.nodes[0];
    let teepod_id = node.teepod_id;
    let image = node.images[0].name.clone();
    println!("✅ Selected TEEPod ID: {}, Image: {}", teepod_id, image);

    // ===== STEP 2: CREATE VM CONFIG =====
    println!("\n📝 Creating VM configuration...");

    // Use the Phala Cloud NextJS starter configuration
    let docker_compose = r#"
services:
  app:
    image: leechael/phala-cloud-nextjs-starter:latest
    ports:
      - "3000:3000"
    volumes:
      - /var/run/tappd.sock:/var/run/tappd.sock
"#;

    // Create VM configuration
    let vm_config = json!({
        "name": "test-app",
        "compose_manifest": {
            "docker_compose_file": docker_compose,
            "name": "test-app",
            "features": ["kms", "tproxy-net"]
        },
        "vcpu": 1,
        "memory": 1024,
        "disk_size": 10,
        "teepod_id": teepod_id,
        "image": image
    });

    // ===== STEP 3: GET ENCRYPTION KEY =====
    println!("🔑 Getting encryption public key...");
    let pubkey_response = client.get_pubkey_for_config(&vm_config).await?;

    // Extract the app_id from the pubkey response - this is the correct identifier for the deployment
    let app_id = pubkey_response.app_id.clone();

    // Construct the full application identifier with the required "app_" prefix
    let full_app_id = format!("app_{}", app_id);

    println!("✅ Received public key for encryption");
    println!("   App ID: {}", app_id);
    println!("   Full Application Identifier: {}", full_app_id);

    // Environment variables to deploy with
    let env_vars = vec![
        ("NGINX_HOST".to_string(), "example.com".to_string()),
        ("NGINX_PORT".to_string(), "80".to_string()),
    ];

    println!("   Using environment variables:");
    for (key, _) in &env_vars {
        println!("   - {}: [value encrypted]", key);
    }

    // ===== STEP 4: DEPLOY APPLICATION =====
    println!("\n🚀 Deploying test application...");
    let deployment = client
        .deploy_with_config_do_encrypt(
            vm_config,
            &env_vars,
            &pubkey_response.app_env_encrypt_pubkey,
            &pubkey_response.app_id_salt,
        )
        .await?;

    println!("✅ Application deployed successfully!");
    println!("   Deployment ID: {}", deployment.id);
    println!("   App ID: {}", app_id); // Use the app_id from pubkey_response
    println!("   Full Application Identifier: {}", full_app_id);
    println!("   Status: {}", deployment.status);

    // ===== STEP 5: WAIT FOR INITIALIZATION =====
    println!("\n⏳ Waiting for deployment to initialize...");
    println!("   (This may take up to 60 seconds)");
    println!("   The network info API will provide the URL to access your application.");

    // Implement a retry mechanism for network info with timeout
    let mut attempts = 0;
    let max_attempts = 12; // Try for up to 60 seconds (12 attempts * 5 seconds)
    let mut network_info = None;

    while attempts < max_attempts {
        attempts += 1;
        tokio::time::sleep(Duration::from_secs(5)).await;

        match client.get_network_info(&full_app_id).await {
            Ok(info) => {
                if info.is_online {
                    network_info = Some(info);
                    break;
                } else {
                    println!(
                        "   Deployment not yet online, waiting... (attempt {}/{})",
                        attempts, max_attempts
                    );
                }
            }
            Err(e) => {
                println!(
                    "   Error checking network status (attempt {}/{}): {}",
                    attempts, max_attempts, e
                );
                if attempts == max_attempts {
                    return Err(e);
                }
            }
        }
    }

    // ===== STEP 6: DISPLAY NETWORK INFO =====
    println!("\n📡 Network Information for {}:", full_app_id);

    match network_info {
        Some(info) => {
            println!(
                "   Status: {}",
                if info.is_online {
                    "🟢 Online"
                } else {
                    "🔴 Offline"
                }
            );
            println!("   Public: {}", if info.is_public { "Yes" } else { "No" });

            if let Some(error) = info.error {
                println!("⚠️ Error: {}", error);
            }

            println!("   Internal IP: {}", info.internal_ip);
            println!("   Latest Handshake: {}", info.latest_handshake);

            println!("\n🌐 Public URLs:");

            // Check if app URL is available before displaying
            if !info.public_urls.app.is_empty() {
                println!("   Application URL: {}", info.public_urls.app);
                println!(
                    "\n✅ Notice that the app_id '{}' is included in the URL",
                    app_id
                );
                println!("   This confirms that the app_id from the pubkey_response");
                println!("   is the correct identifier for your deployment.");

                println!("\n✅ You can access your application at:");
                println!("   {}", info.public_urls.app);
                println!("\n   Try opening this URL in your browser or run:");
                println!("   curl {}", info.public_urls.app);
            } else {
                println!("   Application URL not yet available");
                println!("   The deployment may still be initializing.");
                println!("   Try running this example again in a few minutes.");
            }

            if !info.public_urls.instance.is_empty() {
                println!("   Instance URL: {}", info.public_urls.instance);
            }

            println!("\n🔍 Key Takeaways:");
            println!(
                "   1. The app_id '{}' comes from the pubkey_response, not the deployment ID",
                app_id
            );
            println!("   2. You must use the network info API to get the correct URL for your application");
            println!("   3. The network info API is the only reliable way to get the correct URL");
        }
        None => {
            println!("⚠️ Could not retrieve network information after multiple attempts.");
            println!("   Your deployment may still be initializing.");
            println!("   Try running this example again in a few minutes.");

            println!("\n🔍 Even without network info, remember:");
            println!(
                "   1. The app_id '{}' from the pubkey_response is your application identifier",
                app_id
            );
            println!("   2. You must query the network info API to get the correct URL");
            println!("   3. It may take several minutes for the deployment to fully initialize");
        }
    }

    Ok(())
}
