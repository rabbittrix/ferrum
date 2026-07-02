use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::crypto::EncryptedBlob;

/// Lifecycle status of a managed resource.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceStatus {
    Pending,
    Creating,
    Active,
    Updating,
    Deleting,
    Tainted,
    Failed,
}

impl Default for ResourceStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// A secret value — always stored encrypted, never plain text on disk.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecretValue {
    pub encrypted: EncryptedBlob,
    #[serde(skip)]
    pub plaintext_cache: Option<String>,
}

impl SecretValue {
    pub fn from_encrypted(encrypted: EncryptedBlob) -> Self {
        Self {
            encrypted,
            plaintext_cache: None,
        }
    }
}

/// A single infrastructure resource tracked by cloud-native UID.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceInstance {
    /// Logical address in Ferrum code (e.g. `aws_instance.web`).
    pub address: String,
    /// Cloud provider resource type (e.g. `aws_instance`).
    pub resource_type: String,
    /// Cloud-native unique identifier — survives renames in code.
    pub cloud_uid: String,
    /// Provider name (aws, azure, gcp, …).
    pub provider: String,
    /// Non-sensitive attributes (encrypted at state-file level).
    pub attributes: serde_json::Value,
    /// Sensitive attributes — individually encrypted, never plain text.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secrets: Vec<(String, SecretValue)>,
    pub status: ResourceStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl ResourceInstance {
    pub fn new(
        address: impl Into<String>,
        resource_type: impl Into<String>,
        cloud_uid: impl Into<String>,
        provider: impl Into<String>,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            address: address.into(),
            resource_type: resource_type.into(),
            cloud_uid: cloud_uid.into(),
            provider: provider.into(),
            attributes: serde_json::json!({}),
            secrets: Vec::new(),
            status: ResourceStatus::Pending,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn id(&self) -> Uuid {
        // Deterministic UUID v5-style from cloud UID for graph edges
        Uuid::new_v5(
            &Uuid::NAMESPACE_OID,
            format!("{}:{}", self.provider, self.cloud_uid).as_bytes(),
        )
    }
}
