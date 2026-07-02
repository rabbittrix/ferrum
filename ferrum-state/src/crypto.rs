//! Re-exports ferrum-crypto with state-specific error mapping.

pub use ferrum_crypto::{
    decrypt, derive_key_from_passphrase, encrypt, generate_key, parse_key_hex,
    EncryptedBlob, EncryptionKey,
};

use crate::error::{Result, StateError};

pub fn encrypt_state(key: &EncryptionKey, plaintext: &[u8]) -> Result<EncryptedBlob> {
    encrypt(key, plaintext).map_err(|e| StateError::Encryption(e.to_string()))
}

pub fn decrypt_state(key: &EncryptionKey, blob: &EncryptedBlob) -> Result<Vec<u8>> {
    decrypt(key, blob).map_err(|e| StateError::Decryption(e.to_string()))
}
