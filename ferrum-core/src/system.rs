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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fix_hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_path: Option<String>,
}

impl HealthCheck {
    fn new(name: impl Into<String>, status: CheckStatus, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status,
            message: message.into(),
            fix_hint: None,
            help_url: None,
            config_path: None,
        }
    }

    fn with_hints(mut self, fix: impl Into<String>, help: impl Into<String>) -> Self {
        self.fix_hint = Some(fix.into());
        self.help_url = Some(help.into());
        self
    }

    fn with_config(mut self, path: impl Into<String>) -> Self {
        self.config_path = Some(path.into());
        self
    }
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
    checks.push(check_project_files());
    #[cfg(target_os = "linux")]
    checks.push(check_linux_gui_deps());

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
        HealthCheck::new("path", CheckStatus::Pass, format!("`{exe}` is on the system PATH"))
    } else {
        HealthCheck::new(
            "path",
            CheckStatus::Warn,
            format!("`{exe}` not found on PATH — add the Ferrum install directory to PATH"),
        )
        .with_hints(
            if cfg!(windows) {
                "Run: cargo install --path ferrum-cli --force  (or add target\\release to PATH)"
            } else {
                "Run: ./scripts/install-linux.sh  or  cargo install --path ferrum-cli --force"
            },
            "https://github.com/rabbittrix/ferrum/blob/main/MANUAL.md#installation",
        )
    }
}

fn check_cloud_credentials() -> Vec<HealthCheck> {
    let mut checks = Vec::new();

    let aws_id = std::env::var("AWS_ACCESS_KEY_ID").ok();
    let aws_secret = std::env::var("AWS_SECRET_ACCESS_KEY").ok();
    checks.push(match (&aws_id, &aws_secret) {
        (Some(id), Some(_)) if !id.is_empty() => {
            HealthCheck::new("aws_credentials", CheckStatus::Pass, "AWS credentials detected (AWS_ACCESS_KEY_ID)")
        }
        _ => HealthCheck::new(
            "aws_credentials",
            CheckStatus::Warn,
            "AWS credentials not set — export AWS_ACCESS_KEY_ID / AWS_SECRET_ACCESS_KEY",
        )
        .with_hints(
            "Set AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY in your shell profile",
            "https://github.com/rabbittrix/ferrum/blob/main/MANUAL.md#aws",
        ),
    });

    let azure = std::env::var("ARM_CLIENT_ID")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("AZURE_CLIENT_ID").ok().filter(|s| !s.is_empty()));
    checks.push(match azure {
        Some(_) => HealthCheck::new("azure_credentials", CheckStatus::Pass, "Azure credentials detected"),
        None => HealthCheck::new(
            "azure_credentials",
            CheckStatus::Warn,
            "Azure credentials not set — use ARM_* or AZURE_* variables",
        ),
    });

    let gcp = std::env::var("GOOGLE_APPLICATION_CREDENTIALS")
        .ok()
        .filter(|s| !s.is_empty());
    checks.push(match gcp {
        Some(path) if Path::new(&path).exists() => {
            HealthCheck::new("gcp_credentials", CheckStatus::Pass, format!("GCP service account file found at {path}"))
        }
        Some(path) => HealthCheck::new(
            "gcp_credentials",
            CheckStatus::Warn,
            format!("GOOGLE_APPLICATION_CREDENTIALS set but file missing: {path}"),
        ),
        None => HealthCheck::new(
            "gcp_credentials",
            CheckStatus::Warn,
            "GCP credentials not set — export GOOGLE_APPLICATION_CREDENTIALS",
        ),
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
        HealthCheck::new("docker", CheckStatus::Pass, format!("Docker detected ({via})"))
    } else {
        HealthCheck::new(
            "docker",
            CheckStatus::Warn,
            "Docker not detected — install Docker Desktop or Rancher Desktop to run smoke tests",
        )
        .with_hints(
            "Install Docker Desktop (Windows) or Docker Engine (Linux), then restart Ferrum",
            "https://github.com/rabbittrix/ferrum/blob/main/MANUAL.md#docker-local",
        )
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
        Some(url) => HealthCheck::new("rancher", CheckStatus::Pass, format!("Rancher endpoint configured: {url}")),
        None => HealthCheck::new(
            "rancher",
            CheckStatus::Warn,
            "Rancher not configured — set RANCHER_URL for K8s orchestration",
        ),
    }
}

fn count_fe_files(dir: &Path) -> usize {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .is_some_and(|ext| ext == "fe")
                })
                .count()
        })
        .unwrap_or(0)
}

fn check_project_files() -> HealthCheck {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let count = count_fe_files(&cwd);
    if count > 0 {
        HealthCheck::new(
            "project_files",
            CheckStatus::Pass,
            format!("Found {count} .fe configuration file(s) in {}", cwd.display()),
        )
    } else {
        HealthCheck::new(
            "project_files",
            CheckStatus::Warn,
            "No .fe configuration files in the current directory — Ferrum needs at least one to plan or apply",
        )
        .with_hints(
            "Run: ferrum init  (or ferrum init --template docker-local)",
            "MANUAL.md#getting-started",
        )
        .with_config("main.fe")
    }
}

fn check_ferrum_state() -> HealthCheck {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let state = cwd.join("ferrum.fstate");
    if state.exists() {
        HealthCheck::new("project_state", CheckStatus::Pass, format!("State file found: {}", state.display()))
    } else {
        HealthCheck::new(
            "project_state",
            CheckStatus::Warn,
            "No ferrum.fstate in current directory — run `ferrum init`",
        )
        .with_hints("Run: ferrum init --template docker-local", "MANUAL.md#getting-started")
        .with_config("ferrum.json")
    }
}

#[cfg(target_os = "linux")]
fn check_linux_gui_deps() -> HealthCheck {
    let webkit = Command::new("pkg-config")
        .args(["--exists", "webkit2gtk-4.1"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
        || Command::new("pkg-config")
            .args(["--exists", "webkit2gtk-4.0"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

    if webkit {
        HealthCheck::new("linux_gui_deps", CheckStatus::Pass, "webkit2gtk detected (Tauri GUI supported)")
    } else {
        HealthCheck::new(
            "linux_gui_deps",
            CheckStatus::Warn,
            "webkit2gtk not found — required to build/run Ferrum GUI on Linux",
        )
        .with_hints(
            "Ubuntu/Debian: sudo apt install libwebkit2gtk-4.1-dev build-essential",
            "https://v2.tauri.app/start/prerequisites/#linux",
        )
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
