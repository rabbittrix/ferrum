//! Serializable infrastructure graph for Dashboard rendering.

use ferrum_state::ResourceInstance;
use serde::{Deserialize, Serialize};

pub const GRAPH_FILENAME: &str = "ferrum.graph.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InfrastructureGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    Pending,
    Creating,
    Active,
    Failed,
    Drifted,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub address: String,
    pub resource_type: String,
    pub provider: String,
    pub cloud_uid: String,
    pub has_sensitive: bool,
    pub status: NodeStatus,
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
}

impl InfrastructureGraph {
    pub fn from_resources(resources: &[ResourceInstance], deps: &[(String, String)]) -> Self {
        let edges: Vec<GraphEdge> = deps
            .iter()
            .map(|(from, to)| GraphEdge {
                from: from.clone(),
                to: to.clone(),
            })
            .collect();

        let mut node_ids: std::collections::HashSet<String> =
            resources.iter().map(|r| r.address.clone()).collect();
        for (from, to) in deps {
            node_ids.insert(from.clone());
            node_ids.insert(to.clone());
        }

        let order: Vec<_> = node_ids.into_iter().collect();
        let mut nodes: Vec<GraphNode> = resources
            .iter()
            .map(|r| GraphNode {
                id: r.address.clone(),
                address: r.address.clone(),
                resource_type: r.resource_type.clone(),
                provider: r.provider.clone(),
                cloud_uid: r.cloud_uid.clone(),
                has_sensitive: !r.secrets.is_empty() || attrs_have_sensitive(&r.attributes),
                status: map_status(&r.status),
                x: 0.0,
                y: 0.0,
            })
            .collect();

        for addr in &order {
            if !nodes.iter().any(|n| n.address == *addr) {
                nodes.push(GraphNode {
                    id: addr.clone(),
                    address: addr.clone(),
                    resource_type: "unknown".into(),
                    provider: "unknown".into(),
                    cloud_uid: String::new(),
                    has_sensitive: false,
                    status: NodeStatus::Pending,
                    x: 0.0,
                    y: 0.0,
                });
            }
        }

        layout_nodes(&mut nodes, &edges);
        Self { nodes, edges }
    }

    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)
    }

    pub fn load(path: &std::path::Path) -> std::io::Result<Self> {
        let raw = std::fs::read_to_string(path)?;
        serde_json::from_str(&raw).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

fn map_status(status: &ferrum_state::ResourceStatus) -> NodeStatus {
    use ferrum_state::ResourceStatus;
    match status {
        ResourceStatus::Pending => NodeStatus::Pending,
        ResourceStatus::Creating | ResourceStatus::Updating => NodeStatus::Creating,
        ResourceStatus::Active => NodeStatus::Active,
        ResourceStatus::Failed | ResourceStatus::Tainted => NodeStatus::Failed,
        ResourceStatus::Deleting => NodeStatus::Pending,
    }
}

fn attrs_have_sensitive(attrs: &serde_json::Value) -> bool {
    if attrs.get("ferrum_vault_protected").and_then(|v| v.as_bool()) == Some(true) {
        return true;
    }
    const KEYS: &[&str] = &["password", "secret", "token", "private_key", "access_key"];
    if let Some(obj) = attrs.as_object() {
        return obj.keys().any(|k| {
            let lower = k.to_lowercase();
            KEYS.iter().any(|s| lower.contains(s))
        });
    }
    false
}

fn layout_nodes(nodes: &mut [GraphNode], edges: &[GraphEdge]) {
    let mut depth: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for node in nodes.iter() {
        depth.insert(node.address.clone(), 0);
    }
    for _ in 0..nodes.len().max(1) {
        for edge in edges {
            let d = depth.get(&edge.from).copied().unwrap_or(0) + 1;
            let entry = depth.entry(edge.to.clone()).or_insert(0);
            if d > *entry {
                *entry = d;
            }
        }
    }

    let mut by_level: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
    for (i, node) in nodes.iter().enumerate() {
        let l = depth.get(&node.address).copied().unwrap_or(0);
        by_level.entry(l).or_default().push(i);
    }

    for (level, indices) in by_level {
        let count = indices.len();
        for (pos, idx) in indices.into_iter().enumerate() {
            nodes[idx].y = 80.0 + (level as f64) * 120.0;
            nodes[idx].x = if count == 1 {
                300.0
            } else {
                120.0 + (pos as f64) * 200.0
            };
        }
    }
}
