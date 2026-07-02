use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::crypto::{decrypt, encrypt, derive_key_from_passphrase, generate_key, EncryptionKey, EncryptedBlob};
use crate::error::{Result, StateError};
use crate::resource::{ResourceInstance, SecretValue};

pub const STATE_FILENAME: &str = "ferrum.fstate";
pub const KEY_FILENAME: &str = ".ferrum_key";

/// Metadata stored alongside encrypted payload (unencrypted header).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateMetadata {
    pub version: u32,
    pub serial: u64,
    pub lineage: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Default for StateMetadata {
    fn default() -> Self {
        let now = chrono::Utc::now();
        Self {
            version: 1,
            serial: 0,
            lineage: uuid::Uuid::new_v4().to_string(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// On-disk envelope for ferrum.fstate.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct StateEnvelope {
    metadata: StateMetadata,
    /// Salt for passphrase-derived keys (base64).
    key_salt_b64: String,
    /// The entire state body, AES-256-GCM encrypted.
    encrypted_body: EncryptedBlob,
}

/// In-memory decrypted state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateBody {
    pub resources: Vec<ResourceInstance>,
    pub outputs: HashMap<String, serde_json::Value>,
}

impl Default for StateBody {
    fn default() -> Self {
        Self {
            resources: Vec::new(),
            outputs: HashMap::new(),
        }
    }
}

/// Full Ferrum state with encryption key held in memory only.
pub struct State {
    pub metadata: StateMetadata,
    pub body: StateBody,
    key: EncryptionKey,
    key_salt: Vec<u8>,
    path: PathBuf,
}

impl State {
    /// Create a fresh empty state.
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            metadata: StateMetadata::default(),
            body: StateBody::default(),
            key: generate_key(),
            key_salt: rand::random::<[u8; 16]>().to_vec(),
            path: path.as_ref().to_path_buf(),
        }
    }

    /// Create state with passphrase-derived encryption key.
    pub fn new_with_passphrase(path: impl AsRef<Path>, passphrase: &str) -> Self {
        let salt = rand::random::<[u8; 16]>();
        let key = derive_key_from_passphrase(passphrase, &salt);
        Self {
            metadata: StateMetadata::default(),
            body: StateBody::default(),
            key,
            key_salt: salt.to_vec(),
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn resources(&self) -> &[ResourceInstance] {
        &self.body.resources
    }

    pub fn resources_mut(&mut self) -> &mut Vec<ResourceInstance> {
        &mut self.body.resources
    }

    /// Find resource by logical address.
    pub fn find_by_address(&self, address: &str) -> Option<&ResourceInstance> {
        self.body.resources.iter().find(|r| r.address == address)
    }

    /// Find resource by cloud-native UID (smart refactoring).
    pub fn find_by_cloud_uid(&self, cloud_uid: &str) -> Option<&ResourceInstance> {
        self.body.resources.iter().find(|r| r.cloud_uid == cloud_uid)
    }

    /// Resolve a rename: if cloud_uid matches but address differs, update address.
    pub fn reconcile_rename(&mut self, address: &str, cloud_uid: &str) -> bool {
        if let Some(idx) = self
            .body
            .resources
            .iter()
            .position(|r| r.cloud_uid == cloud_uid && r.address != address)
        {
            self.body.resources[idx].address = address.to_string();
            self.body.resources[idx].updated_at = chrono::Utc::now();
            true
        } else {
            false
        }
    }

    /// Encrypt and persist state to disk. Secrets never touch disk in plain text.
    pub fn save(&mut self) -> Result<()> {
        self.metadata.updated_at = chrono::Utc::now();
        self.metadata.serial += 1;

        let body_json = serde_json::to_vec(&self.body)?;
        let encrypted_body = encrypt(&self.key, &body_json)?;

        let envelope = StateEnvelope {
            metadata: self.metadata.clone(),
            key_salt_b64: base64::engine::general_purpose::STANDARD.encode(&self.key_salt),
            encrypted_body,
        };

        let envelope_json = serde_json::to_vec_pretty(&envelope)?;
        fs::write(&self.path, envelope_json)?;
        Ok(())
    }

    /// Load and decrypt state from disk.
    pub fn load(path: impl AsRef<Path>, passphrase: Option<&str>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if !path.exists() {
            return Err(StateError::NotFound(path.display().to_string()));
        }

        let raw = fs::read_to_string(&path)?;
        let envelope: StateEnvelope =
            serde_json::from_str(&raw).map_err(|e| StateError::InvalidFormat(e.to_string()))?;

        let key_salt = base64::engine::general_purpose::STANDARD
            .decode(&envelope.key_salt_b64)
            .map_err(|e| StateError::InvalidFormat(e.to_string()))?;

        let key = if let Some(pass) = passphrase {
            derive_key_from_passphrase(pass, &key_salt)
        } else {
            resolve_auto_key(&path)?
        };

        let body_bytes = decrypt(&key, &envelope.encrypted_body)?;
        let body: StateBody =
            serde_json::from_slice(&body_bytes).map_err(|e| StateError::InvalidFormat(e.to_string()))?;

        Ok(Self {
            metadata: envelope.metadata,
            body,
            key,
            key_salt,
            path,
        })
    }

    /// Export encryption key as hex (for CI/CD key management).
    pub fn export_key_hex(&self) -> String {
        hex::encode(self.key.as_bytes())
    }

    /// Persist auto-generated key alongside state (when no passphrase is used).
    pub fn save_key_file(&self) -> Result<()> {
        let key_path = self.path.parent().unwrap_or(Path::new(".")).join(KEY_FILENAME);
        fs::write(&key_path, self.export_key_hex())?;
        Ok(())
    }

    /// Encrypt a secret string for storage inside a resource.
    pub fn encrypt_secret(&self, plaintext: &str) -> Result<SecretValue> {
        let blob = encrypt(&self.key, plaintext.as_bytes())?;
        Ok(SecretValue::from_encrypted(blob))
    }

    /// Decrypt a secret value (in memory only).
    pub fn decrypt_secret(&self, secret: &SecretValue) -> Result<String> {
        let bytes = decrypt(&self.key, &secret.encrypted)?;
        String::from_utf8(bytes).map_err(|e| StateError::Decryption(e.to_string()))
    }
}

fn resolve_auto_key(state_path: &Path) -> Result<EncryptionKey> {
    if let Ok(key_hex) = std::env::var("FERRUM_STATE_KEY") {
        return parse_key_hex(&key_hex);
    }
    let key_path = state_path.parent().unwrap_or(Path::new(".")).join(KEY_FILENAME);
    if key_path.exists() {
        let key_hex = fs::read_to_string(&key_path)?.trim().to_string();
        return parse_key_hex(&key_hex);
    }
    Err(StateError::Decryption(
        "passphrase, .ferrum_key, or FERRUM_STATE_KEY required to decrypt state".into(),
    ))
}

fn parse_key_hex(key_hex: &str) -> Result<EncryptionKey> {
    let bytes = hex::decode(key_hex.trim()).map_err(|e| StateError::Decryption(e.to_string()))?;
    if bytes.len() != 32 {
        return Err(StateError::Decryption(
            "encryption key must be 32 bytes (64 hex chars)".into(),
        ));
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Ok(EncryptionKey::from_bytes(key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::ResourceInstance;
    use tempfile::tempdir;

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(STATE_FILENAME);
        let passphrase = "ferrum-test-passphrase";

        let mut state = State::new_with_passphrase(&path, passphrase);
        state.body.resources.push(ResourceInstance::new(
            "aws_instance.web",
            "aws_instance",
            "i-0abc123def456",
            "aws",
        ));
        state.save().unwrap();

        let loaded = State::load(&path, Some(passphrase)).unwrap();
        assert_eq!(loaded.body.resources.len(), 1);
        assert_eq!(loaded.body.resources[0].cloud_uid, "i-0abc123def456");
    }

    #[test]
    fn reconcile_rename_by_uid() {
        let dir = tempdir().unwrap();
        let mut state = State::new(dir.path().join(STATE_FILENAME));
        state.body.resources.push(ResourceInstance::new(
            "aws_instance.old_name",
            "aws_instance",
            "i-0abc123",
            "aws",
        ));

        let renamed = state.reconcile_rename("aws_instance.new_name", "i-0abc123");
        assert!(renamed);
        assert_eq!(state.body.resources[0].address, "aws_instance.new_name");
    }
}
