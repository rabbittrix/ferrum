//! `ferrum test-drive` — Docker smoke test for first-run verification.

use std::path::Path;

use anyhow::Result;
use ferrum_core::{cleanup_smoke_test, run_smoke_test, SmokeTestResult};
use ferrum_telemetry;

pub async fn test_drive(cleanup: bool, base: &Path) -> Result<SmokeTestResult> {
    if cleanup {
        cleanup_smoke_test(base).await?;
        return Ok(SmokeTestResult {
            success: true,
            message: "Smoke test environment cleaned up.".into(),
            project_dir: ferrum_core::smoke_test_dir(base),
            graph_path: std::path::PathBuf::new(),
            docker_available: ferrum_core::detect_docker(),
        });
    }

    let result = run_smoke_test(base).await?;
    if result.success {
        ferrum_telemetry::maybe_notify_first_run(
            env!("CARGO_PKG_VERSION"),
            &[],
            Some(true),
        );
    }
    Ok(result)
}

pub fn print_smoke_result(result: &SmokeTestResult) {
    if result.success {
        println!("✓ {}", result.message);
        if !result.graph_path.as_os_str().is_empty() {
            println!("  Graph: {}", result.graph_path.display());
        }
        println!("  Project: {}", result.project_dir.display());
        println!("  Cleanup: ferrum test-drive --cleanup");
    } else {
        eprintln!("✗ {}", result.message);
        if !result.docker_available {
            eprintln!("  Install Docker to run a test — see MANUAL.md#docker-local");
        }
    }
}
