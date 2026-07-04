//! `ferrum doctor` — system health checks.

use anyhow::Result;
use ferrum_core::{CheckStatus, DoctorReport, HealthCheck, run_doctor};

pub fn doctor(version: &str, json: bool) -> Result<()> {
    let mut report = run_doctor(version);
    report.checks.push(check_update(version));

    if json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap_or_default());
    } else {
        print_report(&report);
    }

    Ok(())
}

fn check_update(version: &str) -> HealthCheck {
    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(4))
        .user_agent(format!("ferrum/{version}"))
        .build()
    {
        Ok(c) => c,
        Err(_) => {
            return HealthCheck {
                name: "update_available".into(),
                status: CheckStatus::Warn,
                message: "Could not check for updates (offline)".into(),
                fix_hint: None,
                help_url: None,
                config_path: None,
            };
        }
    };

    let resp = client
        .get("https://api.github.com/repos/rabbittrix/ferrum/releases/latest")
        .header("Accept", "application/vnd.github+json")
        .send();

    match resp {
        Ok(r) if r.status().is_success() => {
            let body: serde_json::Value = r.json().unwrap_or_default();
            let latest = body
                .get("tag_name")
                .and_then(|v| v.as_str())
                .unwrap_or(version)
                .trim_start_matches('v');
            if latest != version {
                HealthCheck {
                    name: "update_available".into(),
                    status: CheckStatus::Warn,
                    message: format!("Update available: v{latest} (you have v{version})"),
                    fix_hint: None,
                    help_url: Some("https://github.com/rabbittrix/ferrum/releases/latest".into()),
                    config_path: None,
                }
            } else {
                HealthCheck {
                    name: "update_available".into(),
                    status: CheckStatus::Pass,
                    message: format!("Ferrum v{version} is up to date"),
                    fix_hint: None,
                    help_url: None,
                    config_path: None,
                }
            }
        }
        _ => HealthCheck {
            name: "update_available".into(),
            status: CheckStatus::Warn,
            message: "Could not reach GitHub for update check".into(),
            fix_hint: None,
            help_url: None,
            config_path: None,
        },
    }
}

fn print_report(report: &DoctorReport) {
    println!("Ferrum Doctor v{}", report.ferrum_version);
    println!("Platform: {} / {}", report.os, report.arch);
    println!();

    for check in &report.checks {
        let icon = match check.status {
            CheckStatus::Pass => "✓",
            CheckStatus::Warn => "⚠",
            CheckStatus::Fail => "✗",
        };
        println!("  {icon} {} — {}", check.name, check.message);
    }

    println!();
    if report.has_failures() {
        println!("Some checks failed. Fix issues above before running plan/apply.");
    } else {
        println!("System ready for Ferrum operations.");
    }
}
