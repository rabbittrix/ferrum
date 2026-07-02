//! Dependency resolution — decides creation order (VPC → Subnet → Instance).

mod error;
mod graph;

pub use error::{ResolveError, Result};
pub use graph::{ExecutionPlan, ResolvedGraph, Resolver};

use ferrum_parser::FeFile;
use ferrum_state::ResourceInstance;

/// Build execution order from parsed `.fe` files.
pub fn resolve_fe(file: &FeFile) -> Result<ExecutionPlan> {
    Resolver::from_fe(file).execution_plan()
}

/// Build execution order from state resources (post-import).
pub fn resolve_state(resources: &[ResourceInstance]) -> Result<ExecutionPlan> {
    Resolver::from_state(resources).execution_plan()
}
