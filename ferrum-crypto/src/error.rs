use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("encryption error: {0}")]
    Encryption(String),

    #[error("decryption error: {0}")]
    Decryption(String),
}

pub type Result<T> = std::result::Result<T, CryptoError>;
