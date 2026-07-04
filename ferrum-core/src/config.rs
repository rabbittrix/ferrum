//! Load `.fe` configuration, validate, resolve dependencies, and convert to state resources.

use std::path::{Path, PathBuf};

use ferrum_parser::{parse_fe_dir, FeResource, FeValue};
use ferrum_resolver::{resolve_fe, ExecutionPlan};
use ferrum_state::{ResourceInstance, ResourceStatus};

use crate::error::{CoreError, Result};
use crate::graph_export::{InfrastructureGraph, GRAPH_FILENAME};
use crate::load_balancer::expand_load_balancers;
use crate::plan::deps_from_resources;

/// Parsed and validated Ferrum project configuration.
#[derive(Clone, Debug)]
pub struct LoadedProject {
    pub config_path: PathBuf,
    pub resources: Vec<ResourceInstance>,
    pub execution_plan: ExecutionPlan,
}

/// Resolve the directory containing `.fe` files (state parent, cwd, or repo root).
pub fn find_project_dir(state_path: &Path) -> PathBuf {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Some(parent) = state_path.parent() {
        if !parent.as_os_str().is_empty() {
            candidates.push(parent.to_path_buf());
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd);
    }
    if let Some(parent) = state_path.parent().and_then(|p| p.parent()) {
        candidates.push(parent.to_path_buf());
    }

    for dir in candidates {
        if dir_has_fe_files(&dir) {
            return dir;
        }
    }

    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn dir_has_fe_files(dir: &Path) -> bool {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries.filter_map(|e| e.ok()).any(|e| {
                e.path()
                    .extension()
                    .is_some_and(|ext| ext == "fe")
            })
        })
        .unwrap_or(false)
}

/// Discover `.fe` files in `dir`, parse, validate, and build execution graph.
pub fn load_project(dir: &Path) -> Result<LoadedProject> {
    let mut config = parse_fe_dir(dir).map_err(CoreError::Parse)?;
    expand_load_balancers(&mut config);
    let execution_plan = resolve_fe(&config).map_err(CoreError::Resolve)?;
    let resources = fe_resources_to_instances(&config.resources);
    Ok(LoadedProject {
        config_path: dir.to_path_buf(),
        resources,
        execution_plan,
    })
}

/// Load project configuration using state path to locate `.fe` files.
pub fn load_project_for_state(state_path: &Path) -> Result<LoadedProject> {
    load_project(&find_project_dir(state_path))
}

/// Convert parsed resources to state instances (desired configuration).
pub fn fe_resources_to_instances(resources: &[FeResource]) -> Vec<ResourceInstance> {
    resources.iter().map(fe_resource_to_instance).collect()
}

fn fe_resource_to_instance(r: &FeResource) -> ResourceInstance {
    let provider = infer_provider(&r.resource_type);
    let mut instance = ResourceInstance::new(
        r.address(),
        &r.resource_type,
        format!("pending:{}", r.address()),
        provider,
    );
    instance.status = ResourceStatus::Pending;
    instance.attributes = attributes_to_json(r);
    instance
}

fn infer_provider(resource_type: &str) -> String {
    resource_type
        .split('_')
        .next()
        .unwrap_or("unknown")
        .to_string()
}

fn attributes_to_json(r: &FeResource) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (k, v) in &r.attributes {
        if k == "depends_on" {
            continue;
        }
        map.insert(k.clone(), fe_value_to_json(v));
    }
    if !r.depends_on.is_empty() {
        map.insert("depends_on".into(), serde_json::json!(r.depends_on));
    }
    serde_json::Value::Object(map)
}

fn fe_value_to_json(v: &FeValue) -> serde_json::Value {
    match v {
        FeValue::String(s) => serde_json::json!(s),
        FeValue::Number(n) => serde_json::json!(n),
        FeValue::Bool(b) => serde_json::json!(b),
        FeValue::Ref(r) => match &r.attribute {
            Some(a) => serde_json::json!(format!("{}.{}.{}", r.resource_type, r.name, a)),
            None => serde_json::json!(format!("{}.{}", r.resource_type, r.name)),
        },
        FeValue::List(items) => {
            serde_json::json!(items.iter().map(fe_value_to_json).collect::<Vec<_>>())
        }
        FeValue::Object(obj) => {
            let m: serde_json::Map<String, serde_json::Value> = obj
                .iter()
                .map(|(k, v)| (k.clone(), fe_value_to_json(v)))
                .collect();
            serde_json::Value::Object(m)
        }
    }
}

/// Build dashboard graph from desired `.fe` configuration.
pub fn graph_from_desired(resources: &[ResourceInstance]) -> InfrastructureGraph {
    let deps = deps_from_resources(resources);
    InfrastructureGraph::from_resources(resources, &deps)
}

/// Write `ferrum.graph.json` beside the state file for GUI visualization.
pub fn save_plan_graph(state_path: &Path, resources: &[ResourceInstance]) -> Result<PathBuf> {
    let graph = graph_from_desired(resources);
    let graph_path = state_path
        .parent()
        .unwrap_or(Path::new("."))
        .join(GRAPH_FILENAME);
    graph.save(&graph_path)?;
    Ok(graph_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrum_parser::parse_fe_source;

    #[test]
    fn converts_fe_to_resource_instances() {
        let file = parse_fe_source(
            r#"
resource aws_vpc main { cidr_block = "10.0.0.0/16" }
resource aws_subnet public {
    vpc_id: aws_vpc.main.id,
    cidr_block: "10.0.1.0/24"
}
"#,
        )
        .unwrap();
        let instances = fe_resources_to_instances(&file.resources);
        assert_eq!(instances.len(), 2);
        assert_eq!(instances[1].attributes["vpc_id"], "aws_vpc.main.id");
    }
}
