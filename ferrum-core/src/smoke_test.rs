//! Docker smoke test — Hello Ferrum deployment for first-run verification.

use std::path::{Path, PathBuf};

use ferrum_state::STATE_FILENAME;

use crate::error::{CoreError, Result};
use crate::plan::{compute_destroy_plan, format_plan_colored};
use crate::provider::apply_with_providers;
use crate::system::detect_docker;
use crate::{load_project, save_plan_graph, Engine};
use ferrum_provider_bridge::ProviderPool;

pub const SMOKE_DIR_NAME: &str = ".ferrum-smoke-test";

const SMOKE_FE: &str = r#"// Ferrum smoke test — Hello Ferrum (nginx)
resource docker_container hello {
  name  = "ferrum-hello"
  image = "nginx:alpine"
  ports = ["8088:80"]
}
"#;

const SMOKE_JSON: &str = r#"{
  "project": { "name": "ferrum-smoke-test", "template": "smoke-test" },
  "state": { "file": "ferrum.fstate", "encrypted": true },
  "orchestration": { "docker": true },
  "telemetry": { "disabled": false }
}
"#;

#[derive(Clone, Debug, serde::Serialize)]
pub struct SmokeTestResult {
    pub success: bool,
    pub message: String,
    pub project_dir: PathBuf,
    pub graph_path: PathBuf,
    pub docker_available: bool,
}

pub fn smoke_test_dir(base: &Path) -> PathBuf {
    base.join(SMOKE_DIR_NAME)
}

pub async fn run_smoke_test(base: &Path) -> Result<SmokeTestResult> {
    let docker_ok = detect_docker();
    if !docker_ok {
        return Ok(SmokeTestResult {
            success: false,
            message: "Install Docker to run a test. Docker Desktop (Windows) or Docker Engine (Linux) is required.".into(),
            project_dir: smoke_test_dir(base),
            graph_path: PathBuf::new(),
            docker_available: false,
        });
    }

    let dir = smoke_test_dir(base);
    std::fs::create_dir_all(&dir).map_err(|e| CoreError::Io(e))?;
    std::fs::write(dir.join("smoke.fe"), SMOKE_FE).map_err(|e| CoreError::Io(e))?;
    std::fs::write(dir.join("ferrum.json"), SMOKE_JSON).map_err(|e| CoreError::Io(e))?;

    let state_path = dir.join(STATE_FILENAME);
    if !state_path.exists() {
        let mut state = ferrum_state::State::new(&state_path);
        state.save().map_err(CoreError::State)?;
        state.save_key_file().map_err(CoreError::State)?;
    }

    let project = load_project(&dir)?;
    let state = ferrum_state::State::load(&state_path, None).map_err(CoreError::State)?;
    let mut engine = Engine::new(state);
    let plan = engine.plan(&project.resources)?;

    if plan.has_changes() {
        println!("{}", format_plan_colored(&plan));
        let pool = ProviderPool::default();
        apply_with_providers(&mut engine.state, &plan, &project.resources, &pool).await?;
    }

    let graph_path = save_plan_graph(&state_path, &project.resources)?;

    Ok(SmokeTestResult {
        success: true,
        message: format!(
            "Smoke test applied. Open the graph to see docker_container.hello turn green. HTTP: http://localhost:8088"
        ),
        project_dir: dir,
        graph_path,
        docker_available: true,
    })
}

pub async fn cleanup_smoke_test(base: &Path) -> Result<()> {
    let dir = smoke_test_dir(base);
    if !dir.exists() {
        return Ok(());
    }

    let state_path = dir.join(STATE_FILENAME);
    if state_path.exists() {
        let state = ferrum_state::State::load(&state_path, None).map_err(CoreError::State)?;
        let plan = compute_destroy_plan(&state)?;
        let mut engine = Engine::new(state);
        let empty: Vec<ferrum_state::ResourceInstance> = vec![];
        engine.apply(&plan, &empty)?;
    }

    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}
