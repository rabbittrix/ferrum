//! SHA-256 checksum verification before launching provider binaries.

use sha2::{Digest, Sha256};
use std::path::Path;

use crate::error::{BridgeError, Result};

pub fn sha256_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(sha256_bytes(&bytes))
}

pub fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

pub fn verify_checksum(path: &Path, expected: &str) -> Result<()> {
    let actual = sha256_file(path)?;
    if actual.eq_ignore_ascii_case(expected.trim()) {
        Ok(())
    } else {
        Err(BridgeError::ChecksumMismatch {
            path: path.display().to_string(),
            expected: expected.to_string(),
            actual,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_hex_length() {
        assert_eq!(sha256_bytes(b"ferrum").len(), 64);
    }
}
