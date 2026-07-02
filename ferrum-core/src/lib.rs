//! Ferrum IaC engine — dependency graph, concurrency, and resource lifecycle.

mod engine;
mod error;
mod graph;
mod import;
mod plan;
mod refresh;
mod uid;

pub use engine::Engine;
pub use error::{CoreError, Result};
pub use graph::DependencyGraph;
pub use import::import_tfstate;
pub use plan::{format_plan, ChangeAction, Plan, PlannedChange, PlanSummary};
pub use refresh::refresh_resources;
pub use uid::UidResolver;
