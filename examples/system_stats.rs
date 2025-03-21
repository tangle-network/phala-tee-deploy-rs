use phala_tee_deploy_rs::{Error, Result, SystemStatsResponse, TeeDeployer};
use serde_json::json;
use std::{collections::HashMap, env, time::Duration};

/// This example demonstrates how to deploy an application and retrieve its system statistics
/// in the Phala TEE Cloud.
#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file if present
    dotenv::dotenv().ok();

    // Get required API key from environment
    let api_key = env::var("PHALA_CLOUD_API_KEY")
        .expect("PHALA_CLOUD_API_KEY environment variable is required");

    // Optional custom API endpoint
    let api_url = env::var("PHALA_CLOUD_API_ENDPOINT")
        .unwrap_or_else(|_| "https://cloud-api.phala.network/api/v1".to_string());

    println!("=== Phala TEE System Stats Example ===\n");
    println!("Using API endpoint: {}", api_url);

    // Initialize the deployer
    let mut deployer = TeeDeployer::new(api_key, Some(api_url))?;

    // Step 1: Discover available TEEPods
    println!("\n🔍 Discovering available TEEPods...");
    let teepods = deployer.discover_teepod().await?;
    println!("✅ Selected TEEPod with ID: {}", teepods.nodes[0].teepod_id);
    let teepod_id = teepods.nodes[0].teepod_id;
    let image = teepods.nodes[0].images[0].name.clone();

    // Step 2: Create JSON VM configuration for the Phala Cloud NextJS starter
    println!("\n🚀 Preparing and deploying a test application...");

    // Define Docker Compose content
    let docker_compose = r#"
services:
  app:
    image: leechael/phala-cloud-nextjs-starter:latest
    ports:
      - "3000:3000"
    volumes:
      - /var/run/tappd.sock:/var/run/tappd.sock
"#;

    // Create VM configuration using JSON directly
    let vm_config = json!({
        "name": "system-stats-test-app",
        "compose_manifest": {
            "docker_compose_file": docker_compose,
            "name": "system-stats-test-app",
            "features": ["kms", "tproxy-net"]
        },
        "vcpu": 1,
        "memory": 1024,
        "disk_size": 10,
        "teepod_id": teepod_id,
        "image": image
    });

    // Get the client from deployer for direct API access
    let client = deployer.get_client();

    // Step 3: Get encryption key
    println!("🔑 Getting encryption public key...");
    let pubkey_response = client.get_pubkey_for_config(&vm_config).await?;

    // Extract app_id from pubkey response
    let app_id = pubkey_response.app_id.clone();
    println!("✅ Received public key for encryption");
    println!("   App ID: {}", app_id);

    // Construct the full application identifier with the required "app_" prefix
    let full_app_id = format!("app_{}", app_id);

    // Step 4: Deploy application
    println!("\n🚀 Deploying test application...");
    let env_vars = vec![]; // Empty environment variables
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
    println!("   App ID: {}", app_id);
    println!("   Full Application Identifier: {}", full_app_id);
    println!("   Status: {}", deployment.status);

    // Step 5: Wait for system to initialize (may take some time)
    println!("\n⏳ Waiting for system to initialize...");
    println!("   (This may take up to 60 seconds)");

    // Retry mechanism for system stats with timeout
    let mut attempts = 0;
    let max_attempts = 12; // Try for up to 60 seconds (12 attempts * 5 seconds)
    let mut system_stats = None;

    while attempts < max_attempts {
        attempts += 1;
        tokio::time::sleep(Duration::from_secs(5)).await;

        match deployer.get_system_stats(&full_app_id).await {
            Ok(stats) if stats.is_online => {
                system_stats = Some(stats);
                println!("✅ System is online and ready!");
                break;
            }
            Ok(_) => {
                println!(
                    "   System not yet online, waiting... (attempt {}/{})",
                    attempts, max_attempts
                );
            }
            Err(e) => {
                println!(
                    "   Error checking system status (attempt {}/{}): {}",
                    attempts, max_attempts, e
                );
                if attempts == max_attempts {
                    return Err(e);
                }
            }
        }
    }

    // Step 6: Display system statistics
    println!(
        "\n📊 Retrieving system statistics for application ID: {}",
        full_app_id
    );

    match system_stats {
        Some(stats) => {
            display_system_stats(&stats);
        }
        None => {
            // Make one final attempt
            println!("   Making one final attempt to get system stats...");
            let stats = deployer.get_system_stats(&full_app_id).await?;
            display_system_stats(&stats);
        }
    }

    println!("\n✨ Example complete!");
    println!("   You can manually check the system stats again using:");
    println!("   cargo run --example system_stats {}", full_app_id);

    Ok(())
}

/// Display system statistics in a readable format
fn display_system_stats(stats: &SystemStatsResponse) {
    // Status information
    println!("\n=== System Status ===");
    println!(
        "Online: {}",
        if stats.is_online {
            "🟢 Yes"
        } else {
            "🔴 No"
        }
    );
    println!(
        "Publicly accessible: {}",
        if stats.is_public { "Yes" } else { "No" }
    );

    if let Some(error) = &stats.error {
        println!("⚠️ Error: {}", error);
    }

    // Operating system information
    println!("\n=== OS Information ===");
    println!("OS: {} {}", stats.sysinfo.os_name, stats.sysinfo.os_version);
    println!("Kernel version: {}", stats.sysinfo.kernel_version);

    // CPU information
    println!("\n=== CPU Information ===");
    println!("CPU model: {}", stats.sysinfo.cpu_model);
    println!("Number of CPUs: {}", stats.sysinfo.num_cpus);

    // Format memory values in a human-readable way (MB or GB)
    println!("\n=== Memory Usage ===");
    println!(
        "Total memory: {} MB",
        stats.sysinfo.total_memory / 1024 / 1024
    );
    println!(
        "Used memory: {} MB ({:.1}%)",
        stats.sysinfo.used_memory / 1024 / 1024,
        (stats.sysinfo.used_memory as f64 / stats.sysinfo.total_memory as f64) * 100.0
    );
    println!(
        "Free memory: {} MB",
        stats.sysinfo.free_memory / 1024 / 1024
    );

    // Swap information
    if stats.sysinfo.total_swap > 0 {
        println!("\n=== Swap Usage ===");
        println!("Total swap: {} MB", stats.sysinfo.total_swap / 1024 / 1024);
        println!(
            "Used swap: {} MB ({:.1}%)",
            stats.sysinfo.used_swap / 1024 / 1024,
            (stats.sysinfo.used_swap as f64 / stats.sysinfo.total_swap as f64) * 100.0
        );
        println!("Free swap: {} MB", stats.sysinfo.free_swap / 1024 / 1024);
    }

    // Load averages
    println!("\n=== System Load ===");
    println!(
        "Load averages: {:.2} (1m), {:.2} (5m), {:.2} (15m)",
        stats.sysinfo.loadavg_one, stats.sysinfo.loadavg_five, stats.sysinfo.loadavg_fifteen
    );

    // Uptime
    let uptime_days = stats.sysinfo.uptime / (60 * 60 * 24);
    let uptime_hours = (stats.sysinfo.uptime / (60 * 60)) % 24;
    let uptime_minutes = (stats.sysinfo.uptime / 60) % 60;
    let uptime_seconds = stats.sysinfo.uptime % 60;

    println!("\n=== Uptime ===");
    println!(
        "System uptime: {}d {}h {}m {}s",
        uptime_days, uptime_hours, uptime_minutes, uptime_seconds
    );

    // Disk information
    println!("\n=== Disk Information ===");
    if stats.sysinfo.disks.is_empty() {
        println!("No disk information available");
    } else {
        for (i, disk) in stats.sysinfo.disks.iter().enumerate() {
            println!("Disk {}: {}", i + 1, disk.name);

            if let Some(mount) = &disk.mount_point {
                println!("  Mount point: {}", mount);
            }

            let total_gb = disk.total_size as f64 / 1024.0 / 1024.0 / 1024.0;
            let free_gb = disk.free_size as f64 / 1024.0 / 1024.0 / 1024.0;
            let used_gb = total_gb - free_gb;
            let used_percent = (used_gb / total_gb) * 100.0;

            println!("  Total size: {:.2} GB", total_gb);
            println!("  Used: {:.2} GB ({:.1}%)", used_gb, used_percent);
            println!("  Free: {:.2} GB", free_gb);
        }
    }

    println!("\n=== End of System Stats ===");
}
