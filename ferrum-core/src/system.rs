//! Cross-platform system checks shared by `ferrum doctor` and auto-configuration.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DoctorReport {
    pub checks: Vec<HealthCheck>,
    pub ferrum_version: String,
    pub os: String,
    pub arch: String,
}

impl DoctorReport {
    pub fn all_critical_pass(&self) -> bool {
        self.checks
            .iter()
            .filter(|c| c.name != "update_available")
            .all(|c| c.status != CheckStatus::Fail)
    }

    pub fn has_failures(&self) -> bool {
        self.checks.iter().any(|c| c.status == CheckStatus::Fail)
    }
}

pub fn run_doctor(version: &str) -> DoctorReport {
    let mut checks = Vec::new();
    checks.push(check_path());
    checks.extend(check_cloud_credentials());
    checks.push(check_docker_socket());
    checks.push(check_rancher());
    checks.push(check_ferrum_state());

    DoctorReport {
        checks,
        ferrum_version: version.to_string(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
    }
}

fn check_path() -> HealthCheck {
    let exe = if cfg!(windows) { "ferrum.exe" } else { "ferrum" };
    let path_var = std::env::var("PATH").unwrap_or_default();
    let sep = if cfg!(windows) { ';' } else { ':' };

    let found = path_var.split(sep).any(|dir| {
        let candidate = Path::new(dir).join(exe);
        candidate.is_file()
    });

    if found {
        HealthCheck {
            name: "path".into(),
            status: CheckStatus::Pass,
            message: format!("`{exe}` is on the system PATH"),
        }
    } else {
        HealthCheck {
            name: "path".into(),
            status: CheckStatus::Warn,
            message: format!(
                "`{exe}` not found on PATH — add the Ferrum install directory to PATH"
            ),
        }
    }
}

fn check_cloud_credentials() -> Vec<HealthCheck> {
    let mut checks = Vec::new();

    let aws_id = std::env::var("AWS_ACCESS_KEY_ID").ok();
    let aws_secret = std::env::var("AWS_SECRET_ACCESS_KEY").ok();
    checks.push(match (&aws_id, &aws_secret) {
        (Some(id), Some(_)) if !id.is_empty() => HealthCheck {
            name: "aws_credentials".into(),
            status: CheckStatus::Pass,
            message: "AWS credentials detected (AWS_ACCESS_KEY_ID)".into(),
        },
        _ => HealthCheck {
            name: "aws_credentials".into(),
            status: CheckStatus::Warn,
            message: "AWS credentials not set — export AWS_ACCESS_KEY_ID / AWS_SECRET_ACCESS_KEY".into(),
        },
    });

    let azure = std::env::var("ARM_CLIENT_ID")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("AZURE_CLIENT_ID").ok().filter(|s| !s.is_empty()));
    checks.push(match azure {
        Some(_) => HealthCheck {
            name: "azure_credentials".into(),
            status: CheckStatus::Pass,
            message: "Azure credentials detected".into(),
        },
        None => HealthCheck {
            name: "azure_credentials".into(),
            status: CheckStatus::Warn,
            message: "Azure credentials not set — use ARM_* or AZURE_* variables".into(),
        },
    });

    let gcp = std::env::var("GOOGLE_APPLICATION_CREDENTIALS")
        .ok()
        .filter(|s| !s.is_empty());
    checks.push(match gcp {
        Some(path) if Path::new(&path).exists() => HealthCheck {
            name: "gcp_credentials".into(),
            status: CheckStatus::Pass,
            message: format!("GCP service account file found at {path}"),
        },
        Some(path) => HealthCheck {
            name: "gcp_credentials".into(),
            status: CheckStatus::Warn,
            message: format!("GOOGLE_APPLICATION_CREDENTIALS set but file missing: {path}"),
        },
        None => HealthCheck {
            name: "gcp_credentials".into(),
            status: CheckStatus::Warn,
            message: "GCP credentials not set — export GOOGLE_APPLICATION_CREDENTIALS".into(),
        },
    });

    checks
}

pub fn docker_socket_path() -> Option<PathBuf> {
    if cfg!(windows) {
        PathBuf::from(r"\\.\pipe\docker_engine").exists().then(|| {
            PathBuf::from(r"\\.\pipe\docker_engine")
        })
    } else {
        let sock = PathBuf::from("/var/run/docker.sock");
        sock.exists().then_some(sock)
    }
}

pub fn detect_docker() -> bool {
    docker_socket_path().is_some()
        || Command::new(if cfg!(windows) { "docker.exe" } else { "docker" })
            .args(["info", "--format", "{{.ServerVersion}}"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
}

fn check_docker_socket() -> HealthCheck {
    if detect_docker() {
        let via = if docker_socket_path().is_some() {
            "socket/pipe accessible"
        } else {
            "docker CLI reachable"
        };
        HealthCheck {
            name: "docker".into(),
            status: CheckStatus::Pass,
            message: format!("Docker detected ({via})"),
        }
    } else {
        HealthCheck {
            name: "docker".into(),
            status: CheckStatus::Warn,
            message: "Docker not detected — install Docker Desktop or Rancher Desktop".into(),
        }
    }
}

pub fn detect_rancher_endpoint() -> Option<String> {
    std::env::var("RANCHER_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("CATTLE_SERVER").ok().filter(|s| !s.is_empty()))
}

fn check_rancher() -> HealthCheck {
    match detect_rancher_endpoint() {
        Some(url) => HealthCheck {
            name: "rancher".into(),
            status: CheckStatus::Pass,
            message: format!("Rancher endpoint configured: {url}"),
        },
        None => HealthCheck {
            name: "rancher".into(),
            status: CheckStatus::Warn,
            message: "Rancher not configured — set RANCHER_URL for K8s orchestration".into(),
        },
    }
}

fn check_ferrum_state() -> HealthCheck {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let state = cwd.join("ferrum.fstate");
    if state.exists() {
        HealthCheck {
            name: "project_state".into(),
            status: CheckStatus::Pass,
            message: format!("State file found: {}", state.display()),
        }
    } else {
        HealthCheck {
            name: "project_state".into(),
            status: CheckStatus::Warn,
            message: "No ferrum.fstate in current directory — run `ferrum init`".into(),
        }
    }
}

pub fn version_info(version: &str, build_date: &str) -> serde_json::Value {
    serde_json::json!({
        "version": version,
        "build_date": build_date,
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "author": "Roberto de Souza",
        "email": "rabbittrix@hotmail.com",
        "repository": "https://github.com/rabbittrix/ferrum",
    })
}
