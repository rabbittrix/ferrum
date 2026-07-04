//! Ferrum IaC engine — dependency graph, concurrency, and resource lifecycle.

mod ai;
mod cost;
mod drift;
mod engine;
mod error;
mod graph;
mod graph_export;
mod import;
mod lock;
mod orchestration;
mod plan;
mod project_config;
mod provider;
mod refresh;
mod smoke_test;
mod system;
mod uid;
mod vault;
mod config;

pub use ai::{diagnose_failure, AiDiagnosis};
pub use config::{
    fe_resources_to_instances, find_project_dir, graph_from_desired, has_fe_files, load_project,
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
pub use config::load_balancer::expand_load_balancers;
pub use lock::{LockBackend, LockError, LockManager, StateLock};
pub use orchestration::{
    deploy_pod_and_service, specs_from_resource, OrchestrationResult, PodSpec, ServiceSpec,
};
pub use project_config::FerrumConfig;
pub use smoke_test::{cleanup_smoke_test, run_smoke_test, smoke_test_dir, SmokeTestResult, SMOKE_DIR_NAME};
pub use system::{detect_docker, detect_rancher_endpoint, run_doctor, version_info, CheckStatus, DoctorReport, HealthCheck};
pub use plan::{
    deps_from_resources, format_plan, format_plan_colored, graph_from_state, plan_cost_estimate,
    apply_plan, compute_destroy_plan, compute_plan, ChangeAction, Plan, PlannedChange, PlanSummary,
};
pub use provider::{apply_with_providers, load_schemas_for_types};
pub use refresh::refresh_resources;
pub use uid::UidResolver;
pub use vault::Vault;
