//! Distributed and local state locking for concurrent apply protection.

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateLock {
    pub lock_id: String,
    pub holder: String,
    pub acquired_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub pid: u32,
}

#[derive(Clone, Debug)]
pub enum LockBackend {
    /// In-process lock for single-user dev.
    Memory,
    /// Exclusive lock file beside state (`ferrum.fstate.lock`).
    File { state_path: PathBuf },
    /// Remote HTTP lock server.
    Remote { endpoint: String },
    /// S3 + DynamoDB (Terraform-compatible pattern).
    S3Dynamo {
        bucket: String,
        table: String,
        region: String,
    },
}

pub struct LockManager {
    backend: LockBackend,
}

impl LockManager {
    pub fn for_state(state_path: &Path) -> Self {
        Self {
            backend: LockBackend::File {
                state_path: state_path.to_path_buf(),
            },
        }
    }

    pub fn new(backend: LockBackend) -> Self {
        Self { backend }
    }

    pub fn acquire(&self, holder: &str) -> Result<StateLock, LockError> {
        match &self.backend {
            LockBackend::Memory => Ok(StateLock {
                lock_id: Uuid::new_v4().to_string(),
                holder: holder.to_string(),
                acquired_at: Utc::now(),
                expires_at: Utc::now() + Duration::minutes(30),
                pid: std::process::id(),
            }),
            LockBackend::File { state_path } => acquire_file_lock(state_path, holder),
            LockBackend::Remote { endpoint } => acquire_remote_lock(endpoint, holder),
            LockBackend::S3Dynamo { .. } => Err(LockError::NotConfigured(
                "S3/DynamoDB lock backend not yet connected — use file or remote".into(),
            )),
        }
    }

    pub fn release(&self, lock: &StateLock) -> Result<(), LockError> {
        match &self.backend {
            LockBackend::Memory => Ok(()),
            LockBackend::File { state_path } => release_file_lock(state_path, lock),
            LockBackend::Remote { endpoint } => release_remote_lock(endpoint, lock),
            LockBackend::S3Dynamo { .. } => Ok(()),
        }
    }
}

fn lock_path(state_path: &Path) -> PathBuf {
    let mut p = state_path.to_path_buf();
    let name = format!(
        "{}.lock",
        p.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("ferrum.fstate")
    );
    p.set_file_name(name);
    p
}

fn acquire_file_lock(state_path: &Path, holder: &str) -> Result<StateLock, LockError> {
    let path = lock_path(state_path);

    if path.exists() {
        if let Ok(existing) = read_lock_file(&path) {
            if existing.expires_at > Utc::now() && process_alive(existing.pid) {
                return Err(LockError::Held {
                    holder: existing.holder,
                    since: existing.acquired_at,
                });
            }
        }
        let _ = std::fs::remove_file(&path);
    }

    let lock = StateLock {
        lock_id: Uuid::new_v4().to_string(),
        holder: holder.to_string(),
        acquired_at: Utc::now(),
        expires_at: Utc::now() + Duration::minutes(30),
        pid: std::process::id(),
    };

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                LockError::Held {
                    holder: "another process".into(),
                    since: Utc::now(),
                }
            } else {
                LockError::Io(e.to_string())
            }
        })?;

    let json = serde_json::to_string_pretty(&lock).map_err(|e| LockError::Io(e.to_string()))?;
    file.write_all(json.as_bytes())
        .map_err(|e| LockError::Io(e.to_string()))?;
    Ok(lock)
}

fn release_file_lock(state_path: &Path, lock: &StateLock) -> Result<(), LockError> {
    let path = lock_path(state_path);
    if path.exists() {
        if let Ok(existing) = read_lock_file(&path) {
            if existing.lock_id == lock.lock_id {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
    Ok(())
}

fn read_lock_file(path: &Path) -> Result<StateLock, LockError> {
    let mut raw = String::new();
    File::open(path)
        .and_then(|mut f| f.read_to_string(&mut raw))
        .map_err(|e| LockError::Io(e.to_string()))?;
    serde_json::from_str(&raw).map_err(|e| LockError::Io(e.to_string()))
}

fn process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(windows)]
    {
        use std::process::Command;
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}")])
            .output()
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .contains(&pid.to_string())
            })
            .unwrap_or(false)
    }
}

fn acquire_remote_lock(endpoint: &str, holder: &str) -> Result<StateLock, LockError> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| LockError::Io(e.to_string()))?;

    let lock = StateLock {
        lock_id: Uuid::new_v4().to_string(),
        holder: holder.to_string(),
        acquired_at: Utc::now(),
        expires_at: Utc::now() + Duration::minutes(30),
        pid: std::process::id(),
    };

    let resp = client
        .post(format!("{endpoint}/lock/acquire"))
        .json(&lock)
        .send()
        .map_err(|e| LockError::Io(e.to_string()))?;

    if resp.status().is_success() {
        Ok(lock)
    } else if resp.status().as_u16() == 409 {
        Err(LockError::Held {
            holder: "remote holder".into(),
            since: Utc::now(),
        })
    } else {
        Err(LockError::NotConfigured(format!(
            "remote lock failed: {}",
            resp.status()
        )))
    }
}

fn release_remote_lock(endpoint: &str, lock: &StateLock) -> Result<(), LockError> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| LockError::Io(e.to_string()))?;

    let _ = client
        .post(format!("{endpoint}/lock/release"))
        .json(lock)
        .send()
        .map_err(|e| LockError::Io(e.to_string()))?;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum LockError {
    #[error("lock held by {holder} since {since}")]
    Held {
        holder: String,
        since: DateTime<Utc>,
    },

    #[error("lock backend: {0}")]
    NotConfigured(String),

    #[error("lock I/O: {0}")]
    Io(String),
}
