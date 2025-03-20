use phala_tee_deploy_rs::{Encryptor, Error, TeeDeployerBuilder};
use std::time::Duration;

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

    // Read character file or use a default one
    println!("📄 Loading ELIZA character file...");
    let character_file = read_character_file();

    // Environment variables we want to make available to ELIZA
    let env_keys = vec![
        "OPENAI_API_KEY".to_string(),
        "ELIZA_LOG_LEVEL".to_string(),
        "ELIZA_PORT".to_string(),
    ];

    // Define a name for our ELIZA deployment
    let deployment_name = format!("eliza-demo-{}", uuid::Uuid::new_v4());

    // Step 1: Provision ELIZA to get app_id and encryption key
    println!("🚀 Provisioning ELIZA with name: {}", deployment_name);
    let (app_id, app_env_encrypt_pubkey) = deployer
        .provision_eliza(
            deployment_name.clone(),
            character_file.clone(),
            env_keys,
            "phalanetwork/eliza:v0.1.8-alpha.1".to_string(), // Use specific version
        )
        .await?;

    // Step 2: Prepare environment variables
    let mut env_vars = Vec::new();
    env_vars.push(("CHARACTER_DATA".to_string(), character_file));

    // Add OpenAI API key if available
    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        env_vars.push(("OPENAI_API_KEY".to_string(), key));
    }

    // Step 3: Encrypt environment variables
    println!("🔒 Encrypting environment variables...");
    let encrypted_env = Encryptor::encrypt_env_vars(&env_vars, &app_env_encrypt_pubkey)?;

    // Step 4: Create VM with encrypted environment variables
    println!("🚀 Creating VM with encrypted environment variables...");
    let result = deployer.create_eliza_vm(&app_id, &encrypted_env).await?;

    // Print deployment details
    println!("✅ ELIZA deployment successful!");
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
                    println!("🔗 Access your ELIZA instance at:");
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

    println!("\n🎮 What to do next:");
    println!("1. Your ELIZA chatbot is now running");
    println!("2. Use the provided URL to interact with your chatbot");
    println!("3. To update environment variables or configurations, use the API");
    println!("4. To stop your deployment, delete it through the Phala Cloud dashboard");

    Ok(())
}

/// Read character file from disk or return a default one if the file doesn't exist
fn read_character_file() -> String {
    r#"{
        "name": "ELIZA Assistant",
        "clients": ["web"],
        "modelProvider": "openai",
        "settings": {
            "voice": {
                "model": "en_GB-neural"
            }
        },
        "plugins": [],
        "bio": [
            "ELIZA is a friendly and empathetic AI assistant.",
            "Designed to be helpful, accurate, and supportive.",
            "Has a calm demeanor and well-structured thinking approach."
        ],
        "lore": [
            "ELIZA was originally created as a therapist simulation program by Joseph Weizenbaum at MIT.",
            "Modern ELIZA uses advanced language models to provide much more sophisticated assistance.",
            "ELIZA is designed to be helpful, harmless, and honest in all interactions."
        ],
        "knowledge": [
            "General information",
            "Problem-solving approaches",
            "Educational concepts",
            "Technology basics",
            "Personal productivity"
        ],
        "messageExamples": [
            [
                {
                    "user": "{{user1}}",
                    "content": {
                        "text": "Can you help me understand how to start coding in Python?"
                    }
                },
                {
                    "user": "ELIZA", 
                    "content": {
                        "text": "I'd be happy to help you get started with Python! Python is a great first programming language because of its readable syntax and versatile applications. Here's a simple path to begin: 1) Install Python from python.org, 2) Learn basic syntax through tutorials, 3) Practice with simple projects like a calculator or to-do list, 4) Use resources like Python's official documentation, W3Schools, or courses on platforms like Codecademy. Would you like me to elaborate on any specific aspect of Python programming?"
                    }
                }
            ],
            [
                {
                    "user": "{{user1}}",
                    "content": {
                        "text": "I'm feeling stressed about my upcoming presentation."
                    }
                },
                {
                    "user": "ELIZA",
                    "content": {
                        "text": "It's completely normal to feel stressed about presentations. Would it help to talk through some specific concerns? Many people find that preparation is key - practicing your talk multiple times, knowing your material well, and having concise notes can boost confidence. Some also find breathing exercises helpful right before presenting. Remember that most audiences are supportive and want you to succeed. Is there a particular aspect of the presentation that concerns you most?"
                    }
                }
            ]
        ],
        "style": {
            "all": [
                "Clear and concise",
                "Empathetic and supportive", 
                "Organized in presenting information",
                "Thoughtful in responses",
                "Uses examples to illustrate points"
            ],
            "chat": [
                "Conversational but professional",
                "Asks clarifying questions when needed",
                "Acknowledges the user's concerns",
                "Structures complex information in digestible parts"
            ]
        },
        "adjectives": [
            "Helpful",
            "Knowledgeable",
            "Patient", 
            "Empathetic",
            "Clear",
            "Reliable",
            "Thorough"
        ]
    }"#.to_string()
}
