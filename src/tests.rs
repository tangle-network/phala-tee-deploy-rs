use super::*;
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// Helper function to create a test configuration
fn create_test_config(api_url: String) -> DeploymentConfig {
    let mut env_vars = HashMap::new();
    env_vars.insert("TEST_KEY".to_string(), "test_value".to_string());
    env_vars.insert("ANOTHER_KEY".to_string(), "another_value".to_string());

    DeploymentConfig::new(
        "test_api_key".to_string(),
        "version: '3'".to_string(),
        env_vars,
        1,
        "test-image:latest".to_string(),
    )
    .with_api_url(api_url)
}

#[tokio::test]
async fn test_successful_deployment_flow() {
    let mock_server = MockServer::start().await;

    // Mock the pubkey endpoint with validation
    Mock::given(method("POST"))
        .and(path("/cvms/pubkey/from_cvm_configuration"))
        .and(header("Content-Type", "application/json"))
        .and(header("x-api-key", "test_api_key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "app_env_encrypt_pubkey": format!("0x{}", hex::encode([1u8; 32])),
            "app_id_salt": "test_salt"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    // Mock the deployment endpoint with validation
    Mock::given(method("POST"))
        .and(path("/cvms/from_cvm_configuration"))
        .and(header("Content-Type", "application/json"))
        .and(header("x-api-key", "test_api_key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": 123,
            "status": "pending",
            "details": {
                "deployment_time": "2024-03-14T12:00:00Z"
            }
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = create_test_config(mock_server.uri());
    let client = TeeClient::new(config).unwrap();
    let result = client.deploy().await.unwrap();

    assert_eq!(result.id, 123);
    assert_eq!(result.status, "pending");
}

#[tokio::test]
async fn test_api_error_handling() {
    let mock_server = MockServer::start().await;

    // Mock API error response
    Mock::given(method("POST"))
        .and(path("/cvms/pubkey/from_cvm_configuration"))
        .respond_with(ResponseTemplate::new(422).set_body_json(json!({
            "error": "Invalid configuration"
        })))
        .mount(&mock_server)
        .await;

    let config = create_test_config(mock_server.uri());
    let client = TeeClient::new(config).unwrap();
    let result = client.deploy().await;

    assert!(matches!(
        result,
        Err(Error::Api {
            status_code: 422,
            ..
        })
    ));
}

#[tokio::test]
async fn test_timeout_handling() {
    let mock_server = MockServer::start().await;

    // Mock delayed response beyond timeout
    Mock::given(method("POST"))
        .and(path("/cvms/pubkey/from_cvm_configuration"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(6)))
        .mount(&mock_server)
        .await;

    let config = create_test_config(mock_server.uri());
    let client = TeeClient::new(config).unwrap();
    let result = client.deploy().await;

    assert!(matches!(result, Err(Error::HttpClient(_))));
}

#[tokio::test]
async fn test_get_available_teepods() {
    let mock_server = MockServer::start().await;

    // Mock the teepods available endpoint
    Mock::given(method("GET"))
        .and(path("/teepods/available"))
        .and(header("Content-Type", "application/json"))
        .and(header("x-api-key", "test_api_key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "nodes": [
                {
                    "teepod_id": 123,
                    "images": [
                        {
                            "name": "test-image:latest",
                            "tag": "latest"
                        }
                    ],
                    "status": "ready"
                }
            ]
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = create_test_config(mock_server.uri());
    let client = TeeClient::new(config).unwrap();
    let result = client.get_available_teepods().await.unwrap();

    assert_eq!(result["nodes"][0]["teepod_id"], 123);
    assert_eq!(result["nodes"][0]["images"][0]["name"], "test-image:latest");
}

#[tokio::test]
async fn test_get_available_teepods_error() {
    let mock_server = MockServer::start().await;

    // Mock error response
    Mock::given(method("GET"))
        .and(path("/teepods/available"))
        .respond_with(ResponseTemplate::new(403).set_body_json(json!({
            "error": "Unauthorized access"
        })))
        .mount(&mock_server)
        .await;

    let config = create_test_config(mock_server.uri());
    let client = TeeClient::new(config).unwrap();
    let result = client.get_available_teepods().await;

    assert!(matches!(
        result,
        Err(Error::Api {
            status_code: 403,
            ..
        })
    ));
}

#[tokio::test]
async fn test_get_pubkey_for_config() {
    let mock_server = MockServer::start().await;

    // Mock the pubkey endpoint
    Mock::given(method("POST"))
        .and(path("/cvms/pubkey/from_cvm_configuration"))
        .and(header("Content-Type", "application/json"))
        .and(header("x-api-key", "test_api_key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "app_env_encrypt_pubkey": format!("0x{}", hex::encode([1u8; 32])),
            "app_id_salt": "test_salt"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = create_test_config(mock_server.uri());
    let client = TeeClient::new(config).unwrap();

    let vm_config = json!({
        "name": "test-vm",
        "compose_manifest": {
            "docker_compose_file": "version: '3'",
            "name": "test"
        },
        "teepod_id": 123,
        "image": "test-image:latest"
    });

    let result = client.get_pubkey_for_config(&vm_config).await.unwrap();

    assert_eq!(result["app_id_salt"], "test_salt");
    assert!(result["app_env_encrypt_pubkey"]
        .as_str()
        .unwrap()
        .starts_with("0x"));
}

#[tokio::test]
async fn test_deploy_with_config() {
    let mock_server = MockServer::start().await;

    // Mock the deployment endpoint
    Mock::given(method("POST"))
        .and(path("/cvms/from_cvm_configuration"))
        .and(header("Content-Type", "application/json"))
        .and(header("x-api-key", "test_api_key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": 123,
            "status": "creating",
            "details": {
                "creation_time": "2024-03-14T12:00:00Z"
            }
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = create_test_config(mock_server.uri());
    let client = TeeClient::new(config).unwrap();

    let vm_config = json!({
        "name": "test-vm",
        "compose_manifest": {
            "docker_compose_file": "version: '3'",
            "name": "test"
        },
        "teepod_id": 123,
        "image": "test-image:latest"
    });

    let env_vars = vec![
        ("TEST_KEY".to_string(), "test_value".to_string()),
        ("DEBUG".to_string(), "true".to_string()),
    ];

    // Public key that would normally come from the API
    let pubkey = format!("0x{}", hex::encode([1u8; 32]));

    let result = client
        .deploy_with_config_do_encrypt(vm_config, &env_vars, &pubkey, "test_salt")
        .await
        .unwrap();

    assert_eq!(result.id, 123);
    assert_eq!(result.status, "creating");
    assert!(result.details.is_some());
}

#[tokio::test]
async fn test_deploy_with_config_error() {
    let mock_server = MockServer::start().await;

    // Mock deployment error
    Mock::given(method("POST"))
        .and(path("/cvms/from_cvm_configuration"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": "Invalid configuration"
        })))
        .mount(&mock_server)
        .await;

    let config = create_test_config(mock_server.uri());
    let client = TeeClient::new(config).unwrap();

    let vm_config = json!({
        "name": "test-vm",
        "compose_manifest": {
            "docker_compose_file": "version: '3'",
            "name": "test"
        },
        "teepod_id": 123,
        "image": "test-image:latest"
    });

    let env_vars = vec![("TEST_KEY".to_string(), "test_value".to_string())];

    // Public key that would normally come from the API
    let pubkey = format!("0x{}", hex::encode([1u8; 32]));

    let result = client
        .deploy_with_config_do_encrypt(vm_config, &env_vars, &pubkey, "test_salt")
        .await;

    assert!(matches!(
        result,
        Err(Error::Api {
            status_code: 400,
            ..
        })
    ));
}

#[tokio::test]
async fn test_get_compose() {
    let mock_server = MockServer::start().await;

    // Mock the compose endpoint
    Mock::given(method("GET"))
        .and(path("/cvms/test-app-123/compose"))
        .and(header("Content-Type", "application/json"))
        .and(header("x-api-key", "test_api_key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "compose_file": {
                "name": "test-app",
                "docker_compose_file": "version: '3'",
                "pre_launch_script": "#!/bin/bash\necho 'Hello'"
            },
            "env_pubkey": format!("0x{}", hex::encode([1u8; 32]))
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = create_test_config(mock_server.uri());
    let client = TeeClient::new(config).unwrap();
    let result = client.get_compose("test-app-123").await.unwrap();

    assert_eq!(result.compose_file["name"], "test-app");
    assert!(result.env_pubkey.starts_with("0x"));
}

#[tokio::test]
async fn test_update_compose() {
    let mock_server = MockServer::start().await;

    // Mock the update endpoint
    Mock::given(method("PUT"))
        .and(path("/cvms/test-app-123/compose"))
        .and(header("Content-Type", "application/json"))
        .and(header("x-api-key", "test_api_key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "message": "Compose configuration updated"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = create_test_config(mock_server.uri());
    let client = TeeClient::new(config).unwrap();

    let compose_file = json!({
        "name": "updated-app",
        "docker_compose_file": "version: '3'",
        "pre_launch_script": "#!/bin/bash\necho 'Hello Updated'"
    });

    let env_vars = HashMap::from([("NEW_VAR".to_string(), "new_value".to_string())]);

    // Public key that would normally come from the API
    let pubkey = format!("0x{}", hex::encode([1u8; 32]));

    let result = client
        .update_compose("test-app-123", compose_file, Some(env_vars), pubkey)
        .await
        .unwrap();

    assert_eq!(result["status"], "success");
}

#[tokio::test]
async fn test_update_compose_without_env_vars() {
    let mock_server = MockServer::start().await;

    // Mock the update endpoint
    Mock::given(method("PUT"))
        .and(path("/cvms/test-app-123/compose"))
        .and(header("Content-Type", "application/json"))
        .and(header("x-api-key", "test_api_key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "message": "Compose configuration updated"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = create_test_config(mock_server.uri());
    let client = TeeClient::new(config).unwrap();

    let compose_file = json!({
        "name": "updated-app",
        "docker_compose_file": "version: '3'",
        "pre_launch_script": "#!/bin/bash\necho 'Hello Updated'"
    });

    // Public key that would normally come from the API
    let pubkey = format!("0x{}", hex::encode([1u8; 32]));

    // Test without env vars
    let result = client
        .update_compose("test-app-123", compose_file, None, pubkey)
        .await
        .unwrap();

    assert_eq!(result["status"], "success");
}
