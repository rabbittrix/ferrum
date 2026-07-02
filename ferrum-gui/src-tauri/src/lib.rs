use ferrum_core::{format_plan, Engine};
use ferrum_state::State;
use serde::Serialize;

#[derive(Serialize)]
pub struct PlanResponse {
    pub summary: String,
    pub has_changes: bool,
}

#[derive(Serialize)]
pub struct StateInfo {
    pub resource_count: usize,
    pub serial: u64,
    pub lineage: String,
}

#[tauri::command]
fn ferrum_plan(state_path: String, passphrase: Option<String>) -> Result<PlanResponse, String> {
    let state = State::load(&state_path, passphrase.as_deref()).map_err(|e| e.to_string())?;
    let desired = state.resources().to_vec();
    let mut engine = Engine::new(state);
    let plan = engine.plan(&desired).map_err(|e| e.to_string())?;
    Ok(PlanResponse {
        summary: format_plan(&plan),
        has_changes: plan.has_changes(),
    })
}

#[tauri::command]
fn ferrum_state_info(state_path: String, passphrase: Option<String>) -> Result<StateInfo, String> {
    let state = State::load(&state_path, passphrase.as_deref()).map_err(|e| e.to_string())?;
    Ok(StateInfo {
        resource_count: state.resources().len(),
        serial: state.metadata.serial,
        lineage: state.metadata.lineage.clone(),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![ferrum_plan, ferrum_state_info])
        .run(tauri::generate_context!())
        .expect("error while running Ferrum GUI");
}
