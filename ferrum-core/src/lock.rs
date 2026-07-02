//! Distributed state locking for team workflows (S3/DynamoDB or native server).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateLock {
    pub lock_id: String,
    pub holder: String,
    pub acquired_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub enum LockBackend {
    /// In-process lock for single-user dev.
    Memory,
    /// S3 + DynamoDB (Terraform-compatible pattern).
    S3Dynamo { bucket: String, table: String, region: String },
    /// Ferrum native lock server (Rust gRPC).
    Native { endpoint: String },
}

pub struct LockManager {
    backend: LockBackend,
}

impl LockManager {
    pub fn new(backend: LockBackend) -> Self {
        Self { backend }
    }

    pub fn acquire(&self, holder: &str) -> Result<StateLock, LockError> {
        match &self.backend {
            LockBackend::Memory => Ok(StateLock {
                lock_id: Uuid::new_v4().to_string(),
                holder: holder.to_string(),
                acquired_at: Utc::now(),
                expires_at: Utc::now() + chrono::Duration::minutes(30),
            }),
            LockBackend::S3Dynamo { .. } | LockBackend::Native { .. } => {
                Err(LockError::NotConfigured(
                    "distributed lock backend not yet connected — configure ferrum.toml [lock]".into(),
                ))
            }
        }
    }

    pub fn release(&self, _lock: &StateLock) -> Result<(), LockError> {
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LockError {
    #[error("lock held by another user")]
    Held,

    #[error("lock backend: {0}")]
    NotConfigured(String),
}
