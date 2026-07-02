use ferrum_state::State;

use crate::error::Result;
use crate::graph::DependencyGraph;
use crate::plan::{apply_plan, compute_plan, Plan};
use crate::refresh::{refresh_resources, RefreshReport};
use crate::uid::UidResolver;

/// Central Ferrum engine coordinating plan/apply/refresh.
pub struct Engine {
    pub state: State,
}

impl Engine {
    pub fn new(state: State) -> Self {
        Self { state }
    }

    pub fn dependency_graph(&self) -> DependencyGraph {
        DependencyGraph::from_resources(self.state.resources())
    }

    pub fn plan(&mut self, desired: &[ferrum_state::ResourceInstance]) -> Result<Plan> {
        compute_plan(&mut self.state, desired)
    }

    pub fn apply(&mut self, plan: &Plan, desired: &[ferrum_state::ResourceInstance]) -> Result<()> {
        apply_plan(&mut self.state, plan, desired)?;
        self.state.save()?;
        Ok(())
    }

    pub async fn refresh(&mut self) -> Result<RefreshReport> {
        let report = refresh_resources(&mut self.state).await?;
        self.state.save()?;
        Ok(report)
    }

    pub fn reconcile(&mut self, desired: &[ferrum_state::ResourceInstance]) -> crate::uid::ReconcileResult {
        UidResolver::reconcile(&mut self.state, desired)
    }
}
