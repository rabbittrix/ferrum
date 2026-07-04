use ferrum_state::{ResourceInstance, ResourceStatus, State};

use crate::cost::{estimate_plan_cost, CostEstimate};
use crate::error::Result;
use crate::graph::DependencyGraph;
use crate::graph_export::InfrastructureGraph;
use crate::uid::UidResolver;

/// Planned change to a resource.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChangeAction {
    Create,
    Update,
    Delete,
    Rename,
    NoOp,
}

#[derive(Clone, Debug)]
pub struct PlannedChange {
    pub address: String,
    pub resource_type: String,
    pub action: ChangeAction,
    pub reason: String,
}

#[derive(Clone, Debug)]
pub struct Plan {
    pub changes: Vec<PlannedChange>,
    pub execution_order: Vec<String>,
}

impl Plan {
    pub fn has_changes(&self) -> bool {
        self.changes.iter().any(|c| c.action != ChangeAction::NoOp)
    }

    pub fn summary(&self) -> PlanSummary {
        let mut summary = PlanSummary::default();
        for c in &self.changes {
            match c.action {
                ChangeAction::Create => summary.create += 1,
                ChangeAction::Update => summary.update += 1,
                ChangeAction::Delete => summary.delete += 1,
                ChangeAction::Rename => summary.rename += 1,
                ChangeAction::NoOp => summary.noop += 1,
            }
        }
        summary
    }
}

#[derive(Clone, Debug, Default)]
pub struct PlanSummary {
    pub create: usize,
    pub update: usize,
    pub delete: usize,
    pub rename: usize,
    pub noop: usize,
}

/// Compute a plan from desired configuration vs current state.
pub fn compute_plan(state: &mut State, desired: &[ResourceInstance]) -> Result<Plan> {
    let reconcile = UidResolver::reconcile(state, desired);

    let mut changes = Vec::new();

    for rename in &reconcile.renames {
        changes.push(PlannedChange {
            address: rename.new_address.clone(),
            resource_type: String::new(),
            action: ChangeAction::Rename,
            reason: format!(
                "UID {} matched — rename {} → {} without destroy/recreate",
                rename.cloud_uid, rename.old_address, rename.new_address
            ),
        });
    }

    for resource in desired {
        if reconcile.matched.contains(&resource.address) {
            if let Some(existing) = state.find_by_address(&resource.address) {
                if existing.attributes != resource.attributes {
                    changes.push(PlannedChange {
                        address: resource.address.clone(),
                        resource_type: resource.resource_type.clone(),
                        action: ChangeAction::Update,
                        reason: "attributes changed".into(),
                    });
                } else {
                    changes.push(PlannedChange {
                        address: resource.address.clone(),
                        resource_type: resource.resource_type.clone(),
                        action: ChangeAction::NoOp,
                        reason: "no changes".into(),
                    });
                }
            }
        } else if reconcile
            .unmatched
            .iter()
            .any(|u| u.address == resource.address)
        {
            changes.push(PlannedChange {
                address: resource.address.clone(),
                resource_type: resource.resource_type.clone(),
                action: ChangeAction::Create,
                reason: "resource not in state".into(),
            });
        }
    }

    // Resources in state but not in desired → delete
    for existing in state.resources().iter() {
        let still_desired = desired.iter().any(|d| {
            d.address == existing.address || d.cloud_uid == existing.cloud_uid
        });
        if !still_desired {
            changes.push(PlannedChange {
                address: existing.address.clone(),
                resource_type: existing.resource_type.clone(),
                action: ChangeAction::Delete,
                reason: "removed from configuration".into(),
            });
        }
    }

    let graph = DependencyGraph::from_resources(desired);
    let execution_order = graph.execution_order().unwrap_or_default();

    Ok(Plan {
        changes,
        execution_order,
    })
}

/// Build a destroy plan (delete all resources in reverse dependency order).
pub fn compute_destroy_plan(state: &State) -> Result<Plan> {
    let resources = state.resources().to_vec();
    let deps = deps_from_resources(&resources);
    let mut execution_order: Vec<String> = resources.iter().map(|r| r.address.clone()).collect();

    if !deps.is_empty() {
        let dg = DependencyGraph::from_resources(&resources);
        if let Ok(order) = dg.execution_order() {
            execution_order = order.into_iter().rev().collect();
        }
    }

    let changes: Vec<PlannedChange> = resources
        .iter()
        .map(|r| PlannedChange {
            address: r.address.clone(),
            resource_type: r.resource_type.clone(),
            action: ChangeAction::Delete,
            reason: "destroy requested".into(),
        })
        .collect();

    Ok(Plan {
        changes,
        execution_order,
    })
}

/// Apply a plan to state (in-memory; persist separately).
pub fn apply_plan(state: &mut State, plan: &Plan, desired: &[ResourceInstance]) -> Result<()> {
    for change in &plan.changes {
        match change.action {
            ChangeAction::Create => {
                if let Some(resource) = desired.iter().find(|r| r.address == change.address) {
                    let mut r = resource.clone();
                    r.status = ResourceStatus::Active;
                    state.resources_mut().push(r);
                }
            }
            ChangeAction::Update => {
                if let Some(resource) = desired.iter().find(|r| r.address == change.address) {
                    if let Some(existing) = state
                        .resources_mut()
                        .iter_mut()
                        .find(|r| r.address == change.address)
                    {
                        existing.attributes = resource.attributes.clone();
                        existing.updated_at = chrono::Utc::now();
                        existing.status = ResourceStatus::Active;
                    }
                }
            }
            ChangeAction::Delete => {
                state.resources_mut().retain(|r| r.address != change.address);
            }
            ChangeAction::Rename | ChangeAction::NoOp => {}
        }
    }
    Ok(())
}

pub fn format_plan_colored(plan: &Plan) -> String {
    const GREEN: &str = "\x1b[32m";
    const YELLOW: &str = "\x1b[33m";
    const RED: &str = "\x1b[31m";
    const CYAN: &str = "\x1b[36m";
    const DIM: &str = "\x1b[2m";
    const RESET: &str = "\x1b[0m";

    let summary = plan.summary();
    let mut out = String::new();
    out.push_str(&format!(
        "{CYAN}Plan:{RESET} {GREEN}{} to create{RESET}, {YELLOW}{} to update{RESET}, {RED}{} to delete{RESET}, {} to rename, {} unchanged\n\n",
        summary.create, summary.update, summary.delete, summary.rename, summary.noop
    ));

    for c in &plan.changes {
        if c.action == ChangeAction::NoOp {
            continue;
        }
        let (symbol, color) = match c.action {
            ChangeAction::Create => ("+", GREEN),
            ChangeAction::Update => ("~", YELLOW),
            ChangeAction::Delete => ("-", RED),
            ChangeAction::Rename => ("↔", CYAN),
            ChangeAction::NoOp => (" ", DIM),
        };
        out.push_str(&format!(
            "  {color}{symbol}{RESET} {CYAN}{}{RESET} ({DIM}{}{RESET}) — {}\n",
            c.address, c.resource_type, c.reason
        ));
    }

    if !plan.execution_order.is_empty() {
        out.push_str(&format!("\n{DIM}Execution order:{RESET}\n"));
        for (i, addr) in plan.execution_order.iter().enumerate() {
            out.push_str(&format!("  {}. {}\n", i + 1, addr));
        }
    }

    out
}

/// Format plan for CLI output.
pub fn format_plan(plan: &Plan) -> String {
    let summary = plan.summary();
    let mut out = String::new();
    out.push_str(&format!(
        "Plan: {} to create, {} to update, {} to delete, {} to rename, {} unchanged\n\n",
        summary.create, summary.update, summary.delete, summary.rename, summary.noop
    ));
    for c in &plan.changes {
        if c.action == ChangeAction::NoOp {
            continue;
        }
        let symbol = match c.action {
            ChangeAction::Create => "+",
            ChangeAction::Update => "~",
            ChangeAction::Delete => "-",
            ChangeAction::Rename => "↔",
            ChangeAction::NoOp => " ",
        };
        out.push_str(&format!(
            "  {} {} ({}) — {}\n",
            symbol, c.address, c.resource_type, c.reason
        ));
    }
    out
}

/// Cost estimate for billable plan changes (create / update / delete).
pub fn plan_cost_estimate(plan: &Plan) -> CostEstimate {
    let billable: Vec<(String, String)> = plan
        .changes
        .iter()
        .filter_map(|c| {
            let action = match c.action {
                ChangeAction::Create => "create",
                ChangeAction::Update => "update",
                ChangeAction::Delete => "delete",
                ChangeAction::Rename => "rename",
                ChangeAction::NoOp => return None,
            };
            Some((c.address.clone(), action.to_string()))
        })
        .collect();
    let refs: Vec<(&str, &str)> = billable
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    estimate_plan_cost(&refs)
}

/// Collect dependency edges from resource `depends_on` attributes.
pub fn deps_from_resources(resources: &[ResourceInstance]) -> Vec<(String, String)> {
    let mut deps = Vec::new();
    for r in resources {
        if let Some(arr) = r.attributes.get("depends_on").and_then(|v| v.as_array()) {
            for d in arr {
                if let Some(dep) = d.as_str() {
                    deps.push((dep.to_string(), r.address.clone()));
                }
            }
        }
    }
    deps
}

/// Rebuild dashboard graph from current encrypted state.
pub fn graph_from_state(state: &State) -> InfrastructureGraph {
    let deps = deps_from_resources(state.resources());
    InfrastructureGraph::from_resources(state.resources(), &deps)
}
