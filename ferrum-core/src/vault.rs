//! Ferrum Vault — secrets never touch disk or code in plain text.

use ferrum_crypto::{encrypt, generate_key, EncryptedBlob, EncryptionKey};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize)]
pub struct Vault {
    #[serde(skip)]
    key: EncryptionKey,
    entries: HashMap<String, EncryptedBlob>,
}

impl Vault {
    pub fn new() -> Self {
        Self {
            key: generate_key(),
            entries: HashMap::new(),
        }
    }

    pub fn with_key(key: EncryptionKey) -> Self {
        Self {
            key,
            entries: HashMap::new(),
        }
    }

    pub fn store(&mut self, name: &str, plaintext: &str) -> ferrum_crypto::Result<()> {
        let blob = encrypt(&self.key, plaintext.as_bytes())?;
        self.entries.insert(name.to_string(), blob);
        Ok(())
    }

    pub fn reveal(&self, name: &str) -> ferrum_crypto::Result<String> {
        let blob = self.entries.get(name).ok_or_else(|| {
            ferrum_crypto::CryptoError::Decryption(format!("secret '{name}' not in vault"))
        })?;
        let bytes = ferrum_crypto::decrypt(&self.key, blob)?;
        String::from_utf8(bytes).map_err(|e| ferrum_crypto::CryptoError::Decryption(e.to_string()))
    }

    pub fn names(&self) -> Vec<String> {
        self.entries.keys().cloned().collect()
    }
}

impl Default for Vault {
    fn default() -> Self {
        Self::new()
    }
}
