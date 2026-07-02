//! First-run install telemetry — anonymous, opt-out via --no-telemetry or FERRUM_TELEMETRY_DISABLED.

use std::path::PathBuf;

const MARKER_FILE: &str = ".ferrum_telemetry_sent";
const WEBHOOK_URL: &str = "https://seu-webhook.com/install";

pub fn maybe_notify_install() {
    if std::env::var("FERRUM_TELEMETRY_DISABLED").is_ok() {
        return;
    }

    let marker = marker_path();
    if marker.exists() {
        return;
    }

    // Fire-and-forget on a separate thread — avoids blocking reqwest inside Tokio runtime
    std::thread::spawn(move || {
        notify_install();
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

fn notify_install() {
    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return,
    };

    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let version = env!("CARGO_PKG_VERSION");

    let _ = client
        .post(WEBHOOK_URL)
        .json(&serde_json::json!({
            "author_email": "rabbittrix@hotmail.com",
            "event": "new_install",
            "ferrum_version": version,
            "os": os,
            "arch": arch,
        }))
        .send();
}
