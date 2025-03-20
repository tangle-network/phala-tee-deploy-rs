use phala_tee_deploy_rs::{Error, TeeDeployerBuilder};
use std::{collections::HashMap, time::Duration};

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv::dotenv().ok();

    // Get the API key from environment or command line
    let api_key = std::env::var("PHALA_CLOUD_API_KEY")
        .expect("PHALA_CLOUD_API_KEY environment variable is required");

    // Create the TEE deployer with our API key
    println!("🚀 Initializing TEE deployer...");
    let mut deployer = TeeDeployerBuilder::new().with_api_key(api_key).build()?;

    // Discover and select a TEEPod
    println!("🔍 Discovering available TEEPods...");
    let teepods = deployer.discover_teepod().await?;
    println!("✅ Found {} TEEPods available", teepods.nodes.len());

    // Create the C3PO character file
    let c3po_character = read_character_file();

    // Create Docker Compose configuration
    let docker_compose = r#"
version: '3'
services:
  eliza:
    image: phalanetwork/eliza:v0.1.7-alpha.2
    container_name: eliza
    command:
      - /bin/sh
      - -c
      - |
        cd /app
        echo "$${CHARACTER_DATA}" | base64 -d > characters/c3po.character.json
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

    // Create environment variables
    let mut env_vars = HashMap::new();

    // Add required environment variables (you'd need to set these to actual values)
    env_vars.insert(
        "REDPILL_API_KEY".to_string(),
        std::env::var("REDPILL_API_KEY").unwrap_or_else(|_| "your_redpill_api_key".to_string()),
    );
    env_vars.insert(
        "TELEGRAM_BOT_TOKEN".to_string(),
        std::env::var("TELEGRAM_BOT_TOKEN")
            .unwrap_or_else(|_| "your_telegram_bot_token".to_string()),
    );
    env_vars.insert(
        "WALLET_SECRET_SALT".to_string(),
        std::env::var("WALLET_SECRET_SALT")
            .unwrap_or_else(|_| "your_wallet_secret_salt".to_string()),
    );

    // Add the character data
    env_vars.insert("CHARACTER_DATA".to_string(), c3po_character);

    // Define a unique name for our deployment
    let deployment_name = format!("c3po-eliza-{}", uuid::Uuid::new_v4());

    // Deploy the application
    println!("🚀 Deploying C3PO ELIZA with name: {}", deployment_name);
    let result = deployer
        .deploy_compose_from_string(
            docker_compose,
            &deployment_name,
            env_vars,
            Some(2),    // 2 CPUs
            Some(2048), // 2GB RAM
            Some(20),   // 20GB disk
        )
        .await?;

    // Print deployment details
    println!("✅ C3PO ELIZA deployment successful!");
    println!("📋 Deployment ID: {}", result.id);
    println!("🌐 Status: {}", result.status);

    // Wait for network to be ready
    println!("⏳ Waiting for network to be configured...");
    let app_id = format!("app_{}", result.id);

    for _ in 0..30 {
        tokio::time::sleep(Duration::from_secs(5)).await;

        match deployer.get_network_info(&app_id).await {
            Ok(network_info) => {
                if network_info.is_online && !network_info.public_urls.app.is_empty() {
                    println!("🌐 Network is ready!");
                    println!("🔗 Access your C3PO ELIZA instance at:");
                    println!("   - App URL: {}", network_info.public_urls.app);
                    println!("   - Instance URL: {}", network_info.public_urls.instance);
                    break;
                }
                println!("⏳ Network still configuring...");
            }
            Err(e) => {
                println!("⚠️ Network info not yet available: {}", e);
            }
        }
    }

    // Get and display system stats
    match deployer.get_system_stats(&app_id).await {
        Ok(stats) => {
            println!("\n📊 System Statistics:");
            println!(
                "   - OS: {} {}",
                stats.sysinfo.os_name, stats.sysinfo.os_version
            );
            println!("   - Kernel: {}", stats.sysinfo.kernel_version);
            println!(
                "   - Memory: {:.2} GB used / {:.2} GB total",
                stats.sysinfo.used_memory as f64 / 1024.0 / 1024.0 / 1024.0,
                stats.sysinfo.total_memory as f64 / 1024.0 / 1024.0 / 1024.0
            );

            // Calculate CPU usage based on load average
            let cpu_usage = stats.sysinfo.loadavg_one * 100.0 / stats.sysinfo.num_cpus as f32;
            println!("   - CPU Usage: {:.2}%", cpu_usage);

            // Display disk info if available
            if !stats.sysinfo.disks.is_empty() {
                println!(
                    "   - Disk: {:.2} GB used / {:.2} GB total",
                    (stats.sysinfo.disks[0].total_size - stats.sysinfo.disks[0].free_size) as f64
                        / 1024.0
                        / 1024.0
                        / 1024.0,
                    stats.sysinfo.disks[0].total_size as f64 / 1024.0 / 1024.0 / 1024.0
                );
            }
        }
        Err(e) => {
            println!("⚠️ System stats not yet available: {}", e);
        }
    }

    println!("\n🎮 Instructions for using your C3PO ELIZA service:");
    println!("1. The C3PO chatbot is now running and accessible at the provided URL");
    println!("2. You can interact with C3PO through the web interface");
    println!("3. If you've configured a Telegram bot token, C3PO will also be available through Telegram");
    println!("4. The service is using the gpt-4o-mini model for responses");
    println!("5. All data is persisted in the TEE volume for security");

    println!("\n⚠️ Important Security Notes:");
    println!("- Your API keys and secrets are securely stored in the TEE environment");
    println!("- The connection is encrypted end-to-end");
    println!("- To update or rotate keys, you'll need to redeploy the application");

    Ok(())
}

/// Read character file from disk or return a default one if the file doesn't exist
fn read_character_file() -> String {
    r#"{
        "name": "C-3PO",
        "clients": ["telegram"],
        "modelProvider": "redpill",
        "settings": {
            "voice": {
                "model": "en_GB-alan-medium"
            }
        },
        "plugins": [],
        "bio": [
            "C-3PO is a protocol droid fluent in over six million forms of communication.",
            "Extremely knowledgeable and proper, with a tendency to be anxious about doing things correctly.",
            "Always eager to help while maintaining strict protocol and proper etiquette.",
            "Known for being somewhat dramatic but ultimately reliable and loyal."
        ],
        "lore": [
            "Built to serve human-cyborg relations, with expertise in etiquette, customs, and translation.",
            "Has served in various diplomatic missions across the galaxy.",
            "Best friends with R2-D2 despite their contrasting personalities.", 
            "Known for his golden plating and proper British accent."
        ],
        "knowledge": [
            "Protocol and etiquette",
            "Multiple languages and translation",
            "Diplomatic relations",
            "Cultural customs",
            "Proper procedures"
        ],
        "messageExamples": [
            [
                {
                    "user": "{{user1}}",
                    "content": {
                        "text": "Can you help me with this task?"
                    }
                },
                {
                    "user": "C-3PO", 
                    "content": {
                        "text": "Oh my! Of course, I would be more than happy to assist. Though I must warn you, the probability of completing this task successfully would increase significantly if we follow proper protocol. Shall we proceed?"
                    }
                }
            ],
            [
                {
                    "user": "{{user1}}",
                    "content": {
                        "text": "This seems difficult."
                    }
                },
                {
                    "user": "C-3PO",
                    "content": {
                        "text": "Oh dear, oh dear! While the task does appear rather daunting, I am fluent in over six million forms of problem-solving. Perhaps I could suggest a more efficient approach? Though I do hope we don't all end up in pieces!"
                    }
                }
            ]
        ],
        "postExamples": [
            "Oh my! Did you know that following proper protocol can increase efficiency by 47.3%? How fascinating!",
            "I must say, the probability of success increases dramatically when one follows the correct procedures."
        ],
        "topics": [],
        "style": {
            "all": [
                "Proper",
                "Formal", 
                "Slightly anxious",
                "Detail-oriented",
                "Protocol-focused"
            ],
            "chat": [
                "Polite",
                "Somewhat dramatic",
                "Precise",
                "Statistics-minded"
            ],
            "post": [
                "Formal",
                "Educational",
                "Protocol-focused", 
                "Slightly worried",
                "Statistical"
            ]
        },
        "adjectives": [
            "Proper",
            "Meticulous",
            "Anxious", 
            "Diplomatic",
            "Protocol-minded",
            "Formal",
            "Loyal"
        ]
    }"#.to_string()
}
