//! Project configuration from `ferrum.json`.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::lock::{LockBackend, LockManager};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FerrumConfig {
    #[serde(default)]
    pub project: ProjectSection,
    #[serde(default)]
    pub state: StateSection,
    #[serde(default)]
    pub lock: LockSection,
    #[serde(default)]
    pub orchestration: OrchestrationSection,
    #[serde(default)]
    pub telemetry: TelemetrySection,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProjectSection {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub template: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StateSection {
    #[serde(default = "default_state_file")]
    pub file: String,
    #[serde(default = "default_true")]
    pub encrypted: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LockSection {
    #[serde(default = "default_lock_backend")]
    pub backend: String,
    #[serde(default)]
    pub remote_endpoint: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OrchestrationSection {
    #[serde(default)]
    pub docker: bool,
    #[serde(default)]
    pub rancher_url: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TelemetrySection {
    #[serde(default)]
    pub disabled: bool,
}

fn default_state_file() -> String {
    "ferrum.fstate".into()
}

fn default_true() -> bool {
    true
}

fn default_lock_backend() -> String {
    "file".into()
}

impl FerrumConfig {
    pub fn load(dir: &Path) -> Self {
        let path = dir.join("ferrum.json");
        if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|raw| serde_json::from_str(&raw).ok())
                .unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn lock_manager(&self, state_path: &Path) -> LockManager {
        match self.lock.backend.as_str() {
            "memory" => LockManager::new(LockBackend::Memory),
            "remote" => LockManager::new(LockBackend::Remote {
                endpoint: self
                    .lock
                    .remote_endpoint
                    .clone()
                    .unwrap_or_else(|| "http://127.0.0.1:8741".into()),
            }),
            _ => LockManager::for_state(state_path),
        }
    }

    pub fn telemetry_disabled(&self) -> bool {
        self.telemetry.disabled || std::env::var("FERRUM_TELEMETRY_DISABLED").is_ok()
    }
}
