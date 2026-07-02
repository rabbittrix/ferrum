use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::error::{Result, StateError};

const NONCE_LEN: usize = 12;

/// 256-bit AES key material.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EncryptionKey([u8; 32]);

impl EncryptionKey {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Encrypted payload stored on disk.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EncryptedBlob {
    pub nonce_b64: String,
    pub ciphertext_b64: String,
}

/// Generate a cryptographically secure 256-bit key.
pub fn generate_key() -> EncryptionKey {
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    EncryptionKey(key)
}

/// Derive a 256-bit key from a user passphrase (PBKDF-style single round SHA-256).
pub fn derive_key_from_passphrase(passphrase: &str, salt: &[u8]) -> EncryptionKey {
    let mut hasher = Sha256::new();
    hasher.update(passphrase.as_bytes());
    hasher.update(salt);
    let digest = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&digest);
    EncryptionKey(key)
}

/// Encrypt plaintext with AES-256-GCM.
pub fn encrypt(key: &EncryptionKey, plaintext: &[u8]) -> Result<EncryptedBlob> {
    let cipher = Aes256Gcm::new_from_slice(key.as_bytes())
        .map_err(|e| StateError::Encryption(e.to_string()))?;

    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| StateError::Encryption(e.to_string()))?;

    Ok(EncryptedBlob {
        nonce_b64: B64.encode(nonce_bytes),
        ciphertext_b64: B64.encode(ciphertext),
    })
}

/// Decrypt an AES-256-GCM blob.
pub fn decrypt(key: &EncryptionKey, blob: &EncryptedBlob) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key.as_bytes())
        .map_err(|e| StateError::Decryption(e.to_string()))?;

    let nonce_bytes = B64
        .decode(&blob.nonce_b64)
        .map_err(|e| StateError::Decryption(e.to_string()))?;
    let ciphertext = B64
        .decode(&blob.ciphertext_b64)
        .map_err(|e| StateError::Decryption(e.to_string()))?;

    if nonce_bytes.len() != NONCE_LEN {
        return Err(StateError::Decryption("invalid nonce length".into()));
    }

    let nonce = Nonce::from_slice(&nonce_bytes);
    cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| StateError::Decryption(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_encrypt_decrypt() {
        let key = generate_key();
        let plaintext = b"ferrum secret state payload";
        let blob = encrypt(&key, plaintext).unwrap();
        let recovered = decrypt(&key, &blob).unwrap();
        assert_eq!(recovered, plaintext);
    }
}
