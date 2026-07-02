use thiserror::Error;

#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("connection failed: {0}")]
    Connection(String),

    #[error("provider error: {0}")]
    Provider(String),

    #[error("plugin error: {0}")]
    Plugin(String),

    #[error("checksum verification failed for {path}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        path: String,
        expected: String,
        actual: String,
    },

    #[error("handshake failed: {0}")]
    Handshake(String),

    #[error("schema error: {0}")]
    Schema(String),

    #[error("download failed: {0}")]
    Download(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("tonic error: {0}")]
    Tonic(#[from] tonic::transport::Error),

    #[error("rpc error: {0}")]
    Rpc(#[from] tonic::Status),
}

pub type Result<T> = std::result::Result<T, BridgeError>;
