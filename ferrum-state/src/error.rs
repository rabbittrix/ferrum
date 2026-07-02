use thiserror::Error;

#[derive(Debug, Error)]
pub enum StateError {
    #[error("state file not found: {0}")]
    NotFound(String),

    #[error("encryption error: {0}")]
    Encryption(String),

    #[error("decryption error: {0}")]
    Decryption(String),

    #[error("invalid state format: {0}")]
    InvalidFormat(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, StateError>;
