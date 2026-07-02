use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;
use std::collections::HashMap;

use ferrum_state::ResourceInstance;

use crate::error::{CoreError, Result};

/// Directed acyclic graph of resource dependencies.
pub struct DependencyGraph {
    graph: DiGraph<ResourceNode, ()>,
    address_index: HashMap<String, NodeIndex>,
}

#[derive(Clone, Debug)]
pub struct ResourceNode {
    pub address: String,
    pub resource_type: String,
    pub cloud_uid: String,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            address_index: HashMap::new(),
        }
    }

    pub fn from_resources(resources: &[ResourceInstance]) -> Self {
        let mut g = Self::new();
        for r in resources {
            g.add_resource(r);
        }
        g
    }

    pub fn add_resource(&mut self, resource: &ResourceInstance) {
        if self.address_index.contains_key(&resource.address) {
            return;
        }
        let idx = self.graph.add_node(ResourceNode {
            address: resource.address.clone(),
            resource_type: resource.resource_type.clone(),
            cloud_uid: resource.cloud_uid.clone(),
        });
        self.address_index.insert(resource.address.clone(), idx);

        // Infer dependencies from attribute references (simplified)
        if let Some(deps) = resource.attributes.get("depends_on").and_then(|v| v.as_array()) {
            for dep in deps {
                if let Some(dep_addr) = dep.as_str() {
                    if let Some(&dep_idx) = self.address_index.get(dep_addr) {
                        self.graph.add_edge(dep_idx, idx, ());
                    }
                }
            }
        }
    }

    pub fn execution_order(&self) -> Result<Vec<String>> {
        match petgraph::algo::toposort(&self.graph, None) {
            Ok(sorted) => Ok(sorted
                .into_iter()
                .map(|idx| self.graph[idx].address.clone())
                .collect()),
            Err(_) => Err(CoreError::Graph(
                "dependency cycle detected — cannot determine execution order".into(),
            )),
        }
    }

    pub fn dependents(&self, address: &str) -> Vec<String> {
        let Some(&idx) = self.address_index.get(address) else {
            return Vec::new();
        };
        self.graph
            .neighbors_directed(idx, Direction::Incoming)
            .map(|i| self.graph[i].address.clone())
            .collect()
    }

    pub fn get_node(&self, address: &str) -> Option<&ResourceNode> {
        self.address_index
            .get(address)
            .map(|&idx| &self.graph[idx])
    }

    /// Resolve a graph node by cloud-native UID (smart refactoring).
    pub fn find_by_cloud_uid(&self, cloud_uid: &str) -> Option<&ResourceNode> {
        self.graph
            .node_weights()
            .find(|n| n.cloud_uid == cloud_uid)
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrum_state::ResourceInstance;

    #[test]
    fn topological_order() {
        let mut vpc = ResourceInstance::new("aws_vpc.main", "aws_vpc", "vpc-001", "aws");
        vpc.attributes = serde_json::json!({});

        let mut subnet = ResourceInstance::new("aws_subnet.public", "aws_subnet", "subnet-001", "aws");
        subnet.attributes = serde_json::json!({ "depends_on": ["aws_vpc.main"] });

        let mut instance = ResourceInstance::new("aws_instance.web", "aws_instance", "i-001", "aws");
        instance.attributes = serde_json::json!({ "depends_on": ["aws_subnet.public"] });

        let graph = DependencyGraph::from_resources(&[vpc, subnet, instance]);
        let order = graph.execution_order().unwrap();
        assert_eq!(order[0], "aws_vpc.main");
        assert_eq!(order[2], "aws_instance.web");

        let vpc_node = graph.get_node("aws_vpc.main").unwrap();
        assert_eq!(vpc_node.resource_type, "aws_vpc");
        assert_eq!(graph.find_by_cloud_uid("i-001").unwrap().address, "aws_instance.web");
    }
}
