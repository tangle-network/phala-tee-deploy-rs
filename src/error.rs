use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),

    #[error("Invalid configuration: {0}")]
    Configuration(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("API error: {status_code} - {message}")]
    Api { status_code: u16, message: String },

    #[error("Missing required environment variable: {0}")]
    MissingEnvVar(String),

    #[error("Invalid key format: {0}")]
    InvalidKey(String),
}
