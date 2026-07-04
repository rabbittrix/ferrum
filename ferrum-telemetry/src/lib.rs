//! Anonymous install telemetry — notifies author on first run (opt-out available).

use std::path::PathBuf;

const MARKER_FILE: &str = ".ferrum_telemetry_sent";
const WEBHOOK_URL: &str = "https://your-webhook.com/install";
const AUTHOR_EMAIL: &str = "rabbittrix@hotmail.com";

pub fn maybe_notify_install(version: &str) {
    maybe_notify_first_run(version, &[], None);
}

pub fn maybe_notify_install_with_providers(version: &str, providers: &[String]) {
    maybe_notify_first_run(version, providers, None);
}

/// First-run notification (doctor, init, or successful smoke test).
pub fn maybe_notify_first_run(version: &str, providers: &[String], smoke_test: Option<bool>) {
    if std::env::var("FERRUM_TELEMETRY_DISABLED").is_ok() {
        return;
    }

    let marker = marker_path();
    if marker.exists() {
        return;
    }

    let version = version.to_string();
    let providers = providers.to_vec();
    std::thread::spawn(move || {
        notify_install(&version, &providers, smoke_test);
        if let Some(parent) = marker.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&marker, chrono::Utc::now().to_rfc3339());
    });
}

fn marker_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ferrum")
        .join(MARKER_FILE)
}

fn os_family_label() -> &'static str {
    match std::env::consts::OS {
        "windows" => "Windows",
        "linux" => "Linux",
        "macos" => "macOS",
        other => other,
    }
}

fn notify_install(version: &str, providers: &[String], smoke_test: Option<bool>) {
    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return,
    };

    let _ = client
        .post(WEBHOOK_URL)
        .json(&serde_json::json!({
            "author_email": AUTHOR_EMAIL,
            "event": "new_install",
            "ferrum_version": version,
            "os": std::env::consts::OS,
            "os_family": os_family_label(),
            "arch": std::env::consts::ARCH,
            "providers_initialized": providers,
            "smoke_test_success": smoke_test,
        }))
        .send();
}

pub fn provider_display_names(names: &[String]) -> Vec<String> {
    names.to_vec()
}
