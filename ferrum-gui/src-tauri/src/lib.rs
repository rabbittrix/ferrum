mod vault;

use ferrum_core::{
    apply_with_providers, format_plan, load_project_for_state, plan_cost_estimate, save_plan_graph,
    ChangeAction, CostEstimate, Engine, InfrastructureGraph, NodeStatus, Plan, GRAPH_FILENAME,
};
use ferrum_state::State;
use serde::Serialize;
use std::path::PathBuf;
use tauri::Emitter;

use vault::{
    resolve_state_path, vault_add_impl, vault_delete_impl, vault_list_impl, vault_reveal_impl,
    vault_set_impl, VaultListResponse, VaultRevealResponse,
};

#[derive(Serialize)]
pub struct PlanChangeItem {
    pub address: String,
    pub resource_type: String,
    pub action: String,
    pub reason: String,
    pub symbol: String,
}

#[derive(Serialize)]
pub struct PlanWithCostResponse {
    pub summary_text: String,
    pub has_changes: bool,
    pub create: usize,
    pub update: usize,
    pub delete: usize,
    pub rename: usize,
    pub noop: usize,
    pub cost: CostEstimate,
    pub changes: Vec<PlanChangeItem>,
}

#[derive(Serialize)]
pub struct ApplyResponse {
    pub applied: bool,
    pub message: String,
    pub graph_path: String,
}

#[derive(Serialize)]
pub struct StateInfo {
    pub resource_count: usize,
    pub serial: u64,
    pub lineage: String,
}

#[derive(Serialize)]
pub struct ResolveStateResponse {
    pub state_path: String,
    pub exists: bool,
}

fn graph_path_for(state_path: &PathBuf) -> PathBuf {
    state_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join(GRAPH_FILENAME)
}

fn load_or_err(path: &PathBuf, passphrase: Option<&str>) -> Result<State, String> {
    if !path.exists() {
        return Err(format!(
            "state file not found: {}. Run `ferrum init` in your project root.",
            path.display()
        ));
    }
    State::load(path, passphrase).map_err(|e| e.to_string())
}

fn plan_to_response(plan: &Plan) -> PlanWithCostResponse {
    let counts = plan.summary();
    let cost = plan_cost_estimate(plan);
    let changes = plan
        .changes
        .iter()
        .filter(|c| c.action != ChangeAction::NoOp)
        .map(|c| PlanChangeItem {
            address: c.address.clone(),
            resource_type: c.resource_type.clone(),
            action: format!("{:?}", c.action).to_lowercase(),
            reason: c.reason.clone(),
            symbol: match c.action {
                ChangeAction::Create => "+".into(),
                ChangeAction::Update => "~".into(),
                ChangeAction::Delete => "-".into(),
                ChangeAction::Rename => "↔".into(),
                ChangeAction::NoOp => " ".into(),
            },
        })
        .collect();

    PlanWithCostResponse {
        summary_text: format_plan(plan),
        has_changes: plan.has_changes(),
        create: counts.create,
        update: counts.update,
        delete: counts.delete,
        rename: counts.rename,
        noop: counts.noop,
        cost,
        changes,
    }
}

#[tauri::command]
fn ferrum_resolve_state(state_path: Option<String>) -> ResolveStateResponse {
    let path = resolve_state_path(state_path);
    ResolveStateResponse {
        exists: path.exists(),
        state_path: path.to_string_lossy().into(),
    }
}

#[tauri::command]
fn ferrum_plan_with_cost(
    state_path: Option<String>,
    passphrase: Option<String>,
) -> Result<PlanWithCostResponse, String> {
    let path = resolve_state_path(state_path);
    let project = load_project_for_state(&path).map_err(|e| e.to_string())?;

    let state = load_or_err(&path, passphrase.as_deref())?;
    let mut engine = Engine::new(state);
    let plan = engine
        .plan(&project.resources)
        .map_err(|e| e.to_string())?;

    let _ = save_plan_graph(&path, &project.resources);

    Ok(plan_to_response(&plan))
}

#[derive(Serialize, Clone)]
pub struct ApplyProgressEvent {
    pub address: String,
    pub status: String,
}

#[tauri::command]
async fn ferrum_apply(
    app: tauri::AppHandle,
    state_path: Option<String>,
    passphrase: Option<String>,
) -> Result<ApplyResponse, String> {
    let path = resolve_state_path(state_path);
    let project = load_project_for_state(&path).map_err(|e| e.to_string())?;

    let state = load_or_err(&path, passphrase.as_deref())?;
    let mut engine = Engine::new(state);
    let plan = engine
        .plan(&project.resources)
        .map_err(|e| e.to_string())?;

    if !plan.has_changes() {
        return Ok(ApplyResponse {
            applied: false,
            message: "No changes. Infrastructure is up-to-date.".into(),
            graph_path: graph_path_for(&path).to_string_lossy().into(),
        });
    }

    let graph_path = graph_path_for(&path);
    let pool = ferrum_provider_bridge::ProviderPool::default();

    for address in &plan.execution_order {
        let _ = app.emit(
            "apply-progress",
            ApplyProgressEvent {
                address: address.clone(),
                status: "creating".into(),
            },
        );
        if let Ok(mut graph) = InfrastructureGraph::load(&graph_path) {
            graph.set_node_status(address, NodeStatus::Creating);
            let _ = graph.save(&graph_path);
        }
    }

    if let Err(e) = apply_with_providers(&mut engine.state, &plan, &project.resources, &pool).await {
        for address in &plan.execution_order {
            let _ = app.emit(
                "apply-progress",
                ApplyProgressEvent {
                    address: address.clone(),
                    status: "failed".into(),
                },
            );
            if let Ok(mut graph) = InfrastructureGraph::load(&graph_path) {
                graph.set_node_status(address, NodeStatus::Failed);
                let _ = graph.save(&graph_path);
            }
        }
        return Err(e.to_string());
    }

    for address in &plan.execution_order {
        let _ = app.emit(
            "apply-progress",
            ApplyProgressEvent {
                address: address.clone(),
                status: "active".into(),
            },
        );
        if let Ok(mut graph) = InfrastructureGraph::load(&graph_path) {
            graph.set_node_status(address, NodeStatus::Active);
            let _ = graph.save(&graph_path);
        }
    }

    let graph_path = save_plan_graph(&path, &project.resources).map_err(|e| e.to_string())?;

    Ok(ApplyResponse {
        applied: true,
        message: format!("Applied {} change(s).", plan.changes.len()),
        graph_path: graph_path.to_string_lossy().into(),
    })
}

#[tauri::command]
fn ferrum_state_info(
    state_path: Option<String>,
    passphrase: Option<String>,
) -> Result<StateInfo, String> {
    let path = resolve_state_path(state_path);
    let state = load_or_err(&path, passphrase.as_deref())?;
    Ok(StateInfo {
        resource_count: state.resources().len(),
        serial: state.metadata.serial,
        lineage: state.metadata.lineage.clone(),
    })
}

#[tauri::command]
fn ferrum_load_graph(graph_path: String) -> Result<InfrastructureGraph, String> {
    InfrastructureGraph::load(&PathBuf::from(&graph_path)).map_err(|e| e.to_string())
}

#[tauri::command]
fn ferrum_default_graph_path(state_path: Option<String>) -> Result<String, String> {
    Ok(graph_path_for(&resolve_state_path(state_path))
        .to_string_lossy()
        .into())
}

#[tauri::command]
fn ferrum_vault_list(
    state_path: Option<String>,
    passphrase: Option<String>,
) -> Result<VaultListResponse, String> {
    vault_list_impl(state_path, passphrase)
}

#[tauri::command]
fn ferrum_vault_reveal(
    name: String,
    state_path: Option<String>,
    passphrase: Option<String>,
) -> Result<VaultRevealResponse, String> {
    vault_reveal_impl(name, state_path, passphrase)
}

#[tauri::command]
fn ferrum_vault_set(
    name: String,
    value: String,
    state_path: Option<String>,
    passphrase: Option<String>,
) -> Result<(), String> {
    vault_set_impl(name, value, state_path, passphrase)
}

#[tauri::command]
fn ferrum_vault_add(
    name: String,
    state_path: Option<String>,
    passphrase: Option<String>,
) -> Result<(), String> {
    vault_add_impl(name, state_path, passphrase)
}

#[tauri::command]
fn ferrum_vault_delete(
    name: String,
    state_path: Option<String>,
    passphrase: Option<String>,
) -> Result<(), String> {
    vault_delete_impl(name, state_path, passphrase)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            ferrum_resolve_state,
            ferrum_plan_with_cost,
            ferrum_apply,
            ferrum_state_info,
            ferrum_load_graph,
            ferrum_default_graph_path,
            ferrum_vault_list,
            ferrum_vault_reveal,
            ferrum_vault_set,
            ferrum_vault_add,
            ferrum_vault_delete,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Ferrum GUI");
}
