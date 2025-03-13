use thiserror::Error;

/// Error types for the Phala TEE deployment library.
///
/// This module defines a comprehensive error type hierarchy for handling
/// various error conditions that may occur during TEE deployments.
#[derive(Error, Debug)]
pub enum Error {
    /// Errors from the underlying HTTP client library.
    ///
    /// This variant wraps errors from the reqwest library, which include
    /// network connectivity issues, timeouts, and TLS/SSL errors.
    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),

    /// Configuration-related errors.
    ///
    /// These errors occur when the provided configuration is invalid,
    /// such as malformed Docker Compose files or invalid VM settings.
    #[error("Invalid configuration: {0}")]
    Configuration(String),

    /// Encryption-related errors.
    ///
    /// These errors occur during the encryption or decryption of
    /// sensitive data, such as environment variables.
    #[error("Encryption error: {0}")]
    Encryption(String),

    /// API response errors from the Phala Cloud API.
    ///
    /// These errors include status codes and error messages directly
    /// from the API, such as authentication failures or resource limitations.
    #[error("API error: {status_code} - {message}")]
    Api { status_code: u16, message: String },

    /// Missing environment variable errors.
    ///
    /// These errors occur when a required environment variable is not set
    /// in the current environment.
    #[error("Missing required environment variable: {0}")]
    MissingEnvVar(String),

    /// Invalid key format errors.
    ///
    /// These errors occur when a cryptographic key is malformed or
    /// in an unexpected format.
    #[error("Invalid key format: {0}")]
    InvalidKey(String),

    /// Serialization errors.
    ///
    /// These errors occur when serializing or deserializing data.
    #[error("Serialization error: {0}")]
    Serialization(String),
}
