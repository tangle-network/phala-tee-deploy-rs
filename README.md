# Phala TEE Deploy - Rust Library

A robust Rust library for deploying and managing secure applications in Phala's Trusted Execution Environment (TEE) infrastructure.

## Features

- **Secure Deployment**: Deploy containerized applications to secure, isolated TEE environments
- **ELIZA Chatbot Support**: Specialized support for deploying ELIZA-based chatbot services
- **Docker Compose Integration**: Deploy complex multi-container applications using Docker Compose
- **Environment Variable Encryption**: Secure transmission of sensitive environment variables
- **Robust API Handling**: Resilient handling of API response variations and inconsistencies
- **Comprehensive Error Management**: Detailed error information with recovery mechanisms
- **Network Configuration**: Automatic network setup and public URL exposure
- **System Monitoring**: Access to detailed system statistics and performance metrics

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
phala-tee-deploy-rs = { git = "https://github.com/yourusername/phala-tee-deploy-rs.git" }
```

## Quick Start

```rust
use phala_tee_deploy_rs::{Error, TeeDeployerBuilder};

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Load environment variables from .env file if present
    dotenv::dotenv().ok();

    // Get API key from environment
    let api_key = std::env::var("PHALA_CLOUD_API_KEY")
        .expect("PHALA_CLOUD_API_KEY environment variable is required");

    // Create the TEE deployer
    let mut deployer = TeeDeployerBuilder::new()
        .with_api_key(api_key)
        .build()?;

    // Discover available TEEPods
    let teepods = deployer.discover_teepod().await?;
    println!("Found {} TEEPods available", teepods.nodes.len());

    // Deploy a simple service
    let result = deployer
        .provision_eliza(
            "my-eliza-assistant".to_string(),
            get_character_file(),
            vec!["OPENAI_API_KEY".to_string()],
            "phalanetwork/eliza:latest".to_string(),
        )
        .await?;

    println!("Deployment successful!");
    println!("Deployment ID: {}", result.id);
    println!("Status: {}", result.status);

    Ok(())
}

fn get_character_file() -> String {
    // Your character configuration JSON here
    r#"{"name": "ExampleAgent", "bio": ["Helpful AI assistant"]}"#.to_string()
}
```

## API Response Handling

This library employs a robust approach to handling API responses:

- **Custom Deserialization**: The `DeploymentResponse` struct uses custom deserialization to handle variations in API responses
- **Field Name Flexibility**: Handles multiple field names for the same data (e.g., `id`, `uuid`, or `app_id`)
- **Fallback Mechanisms**: When specific fields are missing, the library will attempt to find alternatives
- **Comprehensive Details**: All response fields are preserved in the `details` field for reference

## Examples

The library includes several examples to demonstrate its functionality:

- **Basic ELIZA Deployment**: Simple deployment of an ELIZA chatbot assistant
- **Robust ELIZA Deployment**: Enhanced version with error handling and fallback mechanisms
- **Docker Compose Deployment**: Deployment of multi-container applications
- **System Monitoring**: Fetching and displaying system statistics

See the [examples directory](./examples) for more information.

## Environment Variables

The library requires the following environment variables:

- `PHALA_CLOUD_API_KEY`: Your API key for accessing the Phala Cloud services

Optional environment variables that may be used by deployed applications:

- `OPENAI_API_KEY`: For ELIZA deployments that use OpenAI's services
- `ELIZA_PORT`: To specify a custom port for ELIZA services
- `ELIZA_LOG_LEVEL`: To control logging verbosity in ELIZA services

## Error Handling

The library provides comprehensive error handling through the `Error` enum, which includes variants for different error scenarios:

- `Error::Configuration`: Configuration-related errors
- `Error::Api`: API-related errors with status code and message
- `Error::HttpClient`: HTTP client errors
- `Error::Crypto`: Cryptography-related errors
- `Error::Serialization`: Serialization/deserialization errors

## Advanced Usage

### Custom Docker Compose Deployment

```rust
use phala_tee_deploy_rs::{Error, TeeDeployerBuilder};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let api_key = std::env::var("PHALA_CLOUD_API_KEY").expect("API key is required");
    let mut deployer = TeeDeployerBuilder::new().with_api_key(api_key).build()?;

    // Custom Docker Compose configuration
    let docker_compose = r#"
    version: '3'
    services:
      web:
        image: nginx:alpine
        ports:
          - "80:80"
        volumes:
          - ./html:/usr/share/nginx/html
      db:
        image: postgres:14
        environment:
          - POSTGRES_PASSWORD=password
          - POSTGRES_USER=user
          - POSTGRES_DB=mydb
        volumes:
          - db-data:/var/lib/postgresql/data
    volumes:
      db-data:
    "#;

    // Environment variables
    let mut env_vars = HashMap::new();
    env_vars.insert("POSTGRES_PASSWORD".to_string(), "secure_password".to_string());

    // Deploy the compose configuration
    let result = deployer
        .deploy_compose_from_string(
            docker_compose,
            "my-web-app",
            env_vars,
            Some(2),     // 2 CPUs
            Some(2048),  // 2GB RAM
            Some(20),    // 20GB disk
        )
        .await?;

    println!("Deployment successful with ID: {}", result.id);

    Ok(())
}
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
