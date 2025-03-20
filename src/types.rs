use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Docker registry authentication configuration.
///
/// Used to access private Docker registries when deploying containers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerConfig {
    /// Docker registry username
    pub username: String,

    /// Docker registry password
    pub password: String,

    /// Optional custom registry URL
    pub registry: Option<String>,
}

/// Advanced features configuration for TEE deployments.
///
/// Controls security and visibility settings for deployed applications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedFeatures {
    /// Enable transparent proxy support
    pub tproxy: bool,

    /// Enable Key Management System integration
    pub kms: bool,

    /// Make system information publicly accessible
    pub public_sys_info: bool,

    /// Make application logs publicly accessible
    pub public_logs: bool,

    /// Docker registry authentication settings
    pub docker_config: DockerConfig,

    /// List this deployment in public directories
    pub listed: bool,
}

/// Docker Compose manifest configuration.
///
/// Defines the application structure using Docker Compose format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeManifest {
    /// Name of the application
    pub name: String,

    /// Enabled features for this deployment
    pub features: Vec<String>,

    /// Docker Compose file content
    pub docker_compose_file: String,
}

/// Virtual Machine configuration for a TEE deployment.
///
/// Defines the resources and settings for the VM that will run the containerized application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    /// Name of the deployment
    pub name: String,

    /// Docker Compose manifest configuration
    pub compose_manifest: ComposeManifest,

    /// Number of virtual CPU cores
    pub vcpu: u32,

    /// Memory allocation in MB
    pub memory: u32,

    /// Disk size allocation in GB
    pub disk_size: u32,

    /// ID of the TEEPod to deploy to
    pub teepod_id: u64,

    /// Container image to use
    pub image: String,

    /// Advanced features configuration
    pub advanced_features: AdvancedFeatures,
}

/// Encrypted environment variable entry.
///
/// Used for secure transmission of sensitive environment variables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedEnv {
    /// Environment variable name
    pub key: String,

    /// Encrypted environment variable value
    pub value: String,
}

/// Response from a deployment operation.
///
/// Contains information about the created deployment including its ID and status.
/// This struct uses custom deserialization to handle variations in API responses.
#[derive(Debug, Clone, Serialize)]
pub struct DeploymentResponse {
    /// Unique identifier for the deployment
    pub id: u64,

    /// Current status of the deployment (e.g., "pending", "running")
    pub status: String,

    /// Additional deployment details as key-value pairs
    pub details: Option<HashMap<String, serde_json::Value>>,
}

// Implement custom deserialization to handle different API response formats
impl<'de> Deserialize<'de> for DeploymentResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        // First, try to deserialize as a generic Value
        let value = serde_json::Value::deserialize(deserializer)?;

        // If it's not an object, return an error
        let obj = match value.as_object() {
            Some(obj) => obj,
            None => return Err(D::Error::custom("Expected object for DeploymentResponse")),
        };

        // Try to extract the ID field from various possible formats
        let id = if let Some(id_value) = obj.get("id") {
            if let Some(id_num) = id_value.as_u64() {
                id_num
            } else if let Some(id_str) = id_value.as_str() {
                id_str.parse::<u64>().unwrap_or(0)
            } else {
                0 // Default ID if can't parse
            }
        } else if let Some(id_value) = obj.get("uuid") {
            if let Some(id_num) = id_value.as_u64() {
                id_num
            } else if let Some(id_str) = id_value.as_str() {
                id_str.parse::<u64>().unwrap_or(0)
            } else {
                0 // Default ID if can't parse
            }
        } else if let Some(id_value) = obj.get("app_id") {
            if let Some(id_str) = id_value.as_str() {
                // Extract numeric part from "app_123" format
                if id_str.starts_with("app_") {
                    id_str[4..].parse::<u64>().unwrap_or(0)
                } else {
                    id_str.parse::<u64>().unwrap_or(0)
                }
            } else {
                0 // Default ID if can't parse
            }
        } else {
            // If no ID field is found, generate a random one
            use rand::Rng;
            rand::thread_rng().gen()
        };

        // Extract status field
        let status = obj
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("pending")
            .to_string();

        // Create details map with all fields from the response
        let mut details = HashMap::new();
        for (k, v) in obj {
            details.insert(k.clone(), v.clone());
        }

        Ok(DeploymentResponse {
            id,
            status,
            details: Some(details),
        })
    }
}

/// Response when retrieving a compose configuration.
///
/// Contains both the compose configuration and the public key needed for
/// encrypting environment variables for updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeResponse {
    /// The compose file configuration
    pub compose_file: serde_json::Value,

    /// Public key for encrypting environment variables
    pub env_pubkey: String,
}

/// Response from a pubkey request.
///
/// Contains the public key and other configuration details needed for deployment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PubkeyResponse {
    /// Public key for encrypting environment variables
    pub app_env_encrypt_pubkey: String,

    /// Generated application ID
    pub app_id: String,

    /// Salt used in app ID generation
    pub app_id_salt: String,

    /// Compose manifest configuration
    pub compose_manifest: ComposeManifest,

    /// Disk size in GB
    pub disk_size: u64,

    /// Encrypted environment variables
    pub encrypted_env: String,

    /// Container image to use
    pub image: String,

    /// Whether the deployment should be listed publicly
    pub listed: bool,

    /// Memory allocation in MB
    pub memory: u64,

    /// Name of the deployment
    pub name: String,

    /// Port mappings
    pub ports: Option<Vec<String>>,

    /// ID of the TEEPod to deploy to
    pub teepod_id: u64,

    /// User ID associated with deployment
    pub user_id: Option<String>,

    /// Number of virtual CPUs
    pub vcpu: u64,
}

/// Compose manifest configuration.
///
/// Contains Docker Compose and related deployment settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeManifestResponse {
    /// Optional bash script
    pub bash_script: Option<String>,

    /// Docker Compose file contents
    pub docker_compose_file: String,

    /// Docker registry configuration
    pub docker_config: DockerConfig,

    /// Enabled features for the deployment
    pub features: Vec<String>,

    /// Whether KMS is enabled
    pub kms_enabled: bool,

    /// Manifest version number
    pub manifest_version: u64,

    /// Name of the deployment
    pub name: String,

    /// Pre-launch script contents
    pub pre_launch_script: String,

    /// Whether logs should be public
    pub public_logs: bool,

    /// Whether system info should be public
    pub public_sysinfo: bool,

    /// Runner type (e.g. "docker-compose")
    pub runner: String,

    /// Salt for configuration
    pub salt: String,

    /// Whether transparent proxy is enabled
    pub tproxy_enabled: bool,

    /// Version of the manifest
    pub version: String,
}

/// Response from the TEEPod discovery API endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeePodDiscoveryResponse {
    /// Capacity limits for the TEEPod cluster
    pub capacity: TeePodCapacity,

    /// List of available TEEPod nodes
    pub nodes: Vec<TeePodNode>,

    /// Service tier (e.g. "pro")
    pub tier: String,
}

/// Capacity configuration for a TEEPod cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeePodCapacity {
    /// Maximum disk space in GB
    pub max_disk: u64,

    /// Maximum number of instances
    pub max_instances: u64,

    /// Maximum memory in MB
    pub max_memory: u64,

    /// Maximum number of virtual CPUs
    pub max_vcpu: u64,
}

/// Information about a TEEPod node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeePodNode {
    /// Available VM images
    pub images: Vec<TeePodImage>,

    /// Whether the node is publicly listed
    pub listed: bool,

    /// Node name
    pub name: String,

    /// Number of remaining CVM slots
    pub remaining_cvm_slots: u64,

    /// Remaining memory in MB
    pub remaining_memory: f64,

    /// Remaining virtual CPU capacity
    pub remaining_vcpu: f64,

    /// Resource availability score (0.0-1.0)
    pub resource_score: f64,

    /// Unique identifier for the TEEPod
    pub teepod_id: u64,
}

/// VM image configuration for a TEEPod.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeePodImage {
    /// BIOS file name
    pub bios: String,

    /// Kernel command line parameters
    pub cmdline: String,

    /// Image configuration description
    pub description: String,

    /// Hard disk image path (if any)
    pub hda: Option<String>,

    /// Initial ramdisk file name
    pub initrd: String,

    /// Whether this is a development image
    pub is_dev: bool,

    /// Kernel image file name
    pub kernel: String,

    /// Image name
    pub name: String,

    /// Root filesystem image name
    pub rootfs: String,

    /// Root filesystem hash
    pub rootfs_hash: String,

    /// Whether root filesystem is shared read-only
    pub shared_ro: bool,

    /// Image version numbers [major, minor, patch]
    pub version: Vec<u64>,
}

/// Response containing network information for a deployment.
///
/// Provides details about connectivity, IP addresses, and public URLs
/// for accessing the deployed application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfoResponse {
    /// Whether the deployment is online and responding
    pub is_online: bool,

    /// Whether the deployment is publicly accessible
    pub is_public: bool,

    /// Error message if there's a connectivity issue (null if no error)
    pub error: Option<String>,

    /// Internal IP address of the deployment
    pub internal_ip: String,

    /// Timestamp of the latest connectivity handshake
    pub latest_handshake: String,

    /// Public URLs for accessing the deployment
    pub public_urls: PublicUrls,
}

/// Public URLs for accessing a deployment.
///
/// Contains URLs for different parts of the deployment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicUrls {
    /// URL for the main application
    pub app: String,

    /// URL for the instance/container
    pub instance: String,
}

/// System information for a disk in a virtual machine.
///
/// Contains details about a disk's name, mount point, and size information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    /// Name of the disk (e.g., "sda1")
    pub name: String,

    /// Mount point of the disk (e.g., "/", "/home")
    pub mount_point: Option<String>,

    /// Total size of the disk in bytes
    pub total_size: u64,

    /// Free space available on the disk in bytes
    pub free_size: u64,
}

/// Detailed system information for a virtual machine.
///
/// Contains comprehensive details about the operating system, hardware,
/// and resource utilization of a deployed container VM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Operating system name (e.g., "Linux")
    pub os_name: String,

    /// Operating system version
    pub os_version: String,

    /// Linux kernel version
    pub kernel_version: String,

    /// CPU model name
    pub cpu_model: String,

    /// Number of CPU cores
    pub num_cpus: u32,

    /// Total physical memory in bytes
    pub total_memory: u64,

    /// Available memory in bytes
    pub available_memory: u64,

    /// Used memory in bytes
    pub used_memory: u64,

    /// Free memory in bytes
    pub free_memory: u64,

    /// Total swap space in bytes
    pub total_swap: u64,

    /// Used swap space in bytes
    pub used_swap: u64,

    /// Free swap space in bytes
    pub free_swap: u64,

    /// System uptime in seconds
    pub uptime: u64,

    /// 1-minute load average
    pub loadavg_one: f32,

    /// 5-minute load average
    pub loadavg_five: f32,

    /// 15-minute load average
    pub loadavg_fifteen: f32,

    /// Information about mounted disks
    pub disks: Vec<DiskInfo>,
}

/// Response containing system statistics for a container VM.
///
/// Provides details about the operational status and system resource usage
/// of a deployed application in the Phala TEE Cloud.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatsResponse {
    /// Whether the VM is online and responding
    pub is_online: bool,

    /// Whether the VM is publicly accessible
    pub is_public: bool,

    /// Error message if there's an issue (null if no error)
    pub error: Option<String>,

    /// Detailed system information
    pub sysinfo: SystemInfo,
}
