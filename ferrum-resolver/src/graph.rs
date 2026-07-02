use std::collections::HashMap;

use ferrum_parser::FeFile;
use ferrum_state::ResourceInstance;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::toposort;

use crate::error::{ResolveError, Result};

#[derive(Clone, Debug, serde::Serialize)]
pub struct ExecutionPlan {
    pub order: Vec<String>,
    pub levels: Vec<Vec<String>>,
}

pub struct ResolvedGraph {
    graph: DiGraph<String, ()>,
    index: HashMap<String, NodeIndex>,
}

impl ResolvedGraph {
    pub fn from_fe(file: &FeFile) -> Self {
        let mut g = Self::empty();
        for r in &file.resources {
            g.add_node(&r.address());
            for dep in &r.depends_on {
                g.add_edge(dep, &r.address());
            }
        }
        g
    }

    pub fn from_state(resources: &[ResourceInstance]) -> Self {
        let mut g = Self::empty();
        for r in resources {
            g.add_node(&r.address);
            if let Some(deps) = r.attributes.get("depends_on").and_then(|v| v.as_array()) {
                for dep in deps {
                    if let Some(addr) = dep.as_str() {
                        g.add_edge(addr, &r.address);
                    }
                }
            }
        }
        g
    }

    fn empty() -> Self {
        Self {
            graph: DiGraph::new(),
            index: HashMap::new(),
        }
    }

    fn add_node(&mut self, address: &str) {
        if !self.index.contains_key(address) {
            let idx = self.graph.add_node(address.to_string());
            self.index.insert(address.to_string(), idx);
        }
    }

    fn add_edge(&mut self, from: &str, to: &str) {
        self.add_node(from);
        self.add_node(to);
        let from_idx = self.index[from];
        let to_idx = self.index[to];
        self.graph.add_edge(from_idx, to_idx, ());
    }

    pub fn execution_plan(&self) -> Result<ExecutionPlan> {
        let sorted = toposort(&self.graph, None).map_err(|_| {
            ResolveError::Cycle("dependency cycle detected in infrastructure graph".into())
        })?;

        let order: Vec<String> = sorted.iter().map(|&i| self.graph[i].clone()).collect();
        let levels = compute_levels(&self.graph, &sorted);

        Ok(ExecutionPlan { order, levels })
    }
}

pub struct Resolver {
    inner: ResolvedGraph,
}

impl Resolver {
    pub fn from_fe(file: &FeFile) -> Self {
        Self {
            inner: ResolvedGraph::from_fe(file),
        }
    }

    pub fn from_state(resources: &[ResourceInstance]) -> Self {
        Self {
            inner: ResolvedGraph::from_state(resources),
        }
    }

    pub fn execution_plan(&self) -> Result<ExecutionPlan> {
        self.inner.execution_plan()
    }
}

fn compute_levels(graph: &DiGraph<String, ()>, sorted: &[NodeIndex]) -> Vec<Vec<String>> {
    let mut level_map: HashMap<NodeIndex, usize> = HashMap::new();
    for &idx in sorted {
        let max_parent = graph
            .neighbors_directed(idx, petgraph::Direction::Incoming)
            .map(|p| level_map.get(&p).copied().unwrap_or(0))
            .max()
            .unwrap_or(0);
        let level = if graph
            .neighbors_directed(idx, petgraph::Direction::Incoming)
            .next()
            .is_some()
        {
            max_parent + 1
        } else {
            0
        };
        level_map.insert(idx, level);
    }

    let max_level = level_map.values().copied().max().unwrap_or(0);
    let mut levels = vec![Vec::new(); max_level + 1];
    for &idx in sorted {
        let l = level_map[&idx];
        levels[l].push(graph[idx].clone());
    }
    levels
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ResolveError;
    use ferrum_parser::parse_fe_source;

    #[test]
    fn detects_dependency_cycle() {
        let file = parse_fe_source(
            r#"
resource aws_vpc a { cidr_block = "10.0.0.0/16" depends_on = [aws_subnet.b] }
resource aws_subnet b { vpc_id: aws_vpc.a.id, cidr_block = "10.0.1.0/24" depends_on = [aws_vpc.a] }
"#,
        )
        .unwrap();
        let graph = ResolvedGraph::from_fe(&file);
        let err = graph.execution_plan().unwrap_err();
        assert!(matches!(err, ResolveError::Cycle(_)));
    }

    #[test]
    fn valid_dag_orders_vpc_before_subnet() {
        let file = parse_fe_source(
            r#"
resource aws_vpc main { cidr_block = "10.0.0.0/16" }
resource aws_subnet public { vpc_id: aws_vpc.main.id, cidr_block = "10.0.1.0/24" }
"#,
        )
        .unwrap();
        let plan = ResolvedGraph::from_fe(&file).execution_plan().unwrap();
        let vpc_pos = plan.order.iter().position(|a| a == "aws_vpc.main").unwrap();
        let subnet_pos = plan
            .order
            .iter()
            .position(|a| a == "aws_subnet.public")
            .unwrap();
        assert!(vpc_pos < subnet_pos);
    }
}
