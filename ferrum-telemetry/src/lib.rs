//! Anonymous install telemetry — notifies author on first run (opt-out available).

use std::path::PathBuf;

const MARKER_FILE: &str = ".ferrum_telemetry_sent";
const WEBHOOK_URL: &str = "https://your-webhook.com/install";
const AUTHOR_EMAIL: &str = "rabbittrix@hotmail.com";

pub fn maybe_notify_install(version: &str) {
    if std::env::var("FERRUM_TELEMETRY_DISABLED").is_ok() {
        return;
    }

    let marker = marker_path();
    if marker.exists() {
        return;
    }

    let version = version.to_string();
    std::thread::spawn(move || {
        notify_install(&version);
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

fn notify_install(version: &str) {
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
            "arch": std::env::consts::ARCH,
        }))
        .send();
}
