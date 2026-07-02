use thiserror::Error;

#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("connection failed: {0}")]
    Connection(String),

    #[error("provider error: {0}")]
    Provider(String),

    #[error("tonic error: {0}")]
    Tonic(#[from] tonic::transport::Error),

    #[error("rpc error: {0}")]
    Rpc(#[from] tonic::Status),
}

pub type Result<T> = std::result::Result<T, BridgeError>;
