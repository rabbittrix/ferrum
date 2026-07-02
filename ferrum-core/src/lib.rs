//! Ferrum IaC engine — dependency graph, concurrency, and resource lifecycle.

mod ai;
mod config;
mod cost;
mod drift;
mod engine;
mod error;
mod graph;
mod graph_export;
mod import;
mod lock;
mod plan;
mod provider;
mod refresh;
mod uid;
mod vault;

pub use ai::{diagnose_failure, AiDiagnosis};
pub use config::{
    fe_resources_to_instances, find_project_dir, graph_from_desired, load_project,
    load_project_for_state, save_plan_graph, LoadedProject,
};
pub use cost::{estimate_plan_cost, CostEstimate, CostLineItem};
pub use drift::{detect_drift, DriftEvent, DriftReport};
pub use engine::Engine;
pub use error::{CoreError, Result};
pub use graph::DependencyGraph;
pub use graph_export::{
    GraphEdge, GraphNode, InfrastructureGraph, NodeStatus, GRAPH_FILENAME,
};
pub use import::{import_tfstate, ImportReport};
pub use lock::{LockBackend, LockManager, StateLock};
pub use plan::{
    deps_from_resources, format_plan, format_plan_colored, graph_from_state, plan_cost_estimate,
    apply_plan, compute_plan, ChangeAction, Plan, PlannedChange, PlanSummary,
};
pub use provider::{apply_with_providers, load_schemas_for_types};
pub use refresh::refresh_resources;
pub use uid::UidResolver;
pub use vault::Vault;
