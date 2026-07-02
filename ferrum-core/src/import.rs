use std::fs;
use std::path::{Path, PathBuf};

use ferrum_state::{ResourceInstance, State};

use crate::error::{CoreError, Result};
use crate::graph_export::{InfrastructureGraph, GRAPH_FILENAME};

/// Terraform tfstate JSON schema (subset).
#[derive(Debug, serde::Deserialize)]
struct TfState {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    serial: u64,
    #[serde(default)]
    lineage: Option<String>,
    #[serde(default)]
    resources: Vec<TfResource>,
}

#[derive(Debug, serde::Deserialize)]
struct TfResource {
    #[serde(default)]
    mode: String,
    #[serde(rename = "type")]
    resource_type: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    provider: String,
    #[serde(default)]
    instances: Vec<TfInstance>,
}

#[derive(Debug, serde::Deserialize)]
struct TfInstance {
    #[serde(default)]
    attributes: serde_json::Value,
    #[serde(default, rename = "dependencies")]
    dependencies: Vec<String>,
}

/// Parse Terraform tfstate and convert to Ferrum encrypted state + infrastructure graph.
pub fn import_tfstate(tfstate_path: &Path, output_state: &mut State) -> Result<ImportReport> {
    let raw = fs::read_to_string(tfstate_path).map_err(|e| {
        CoreError::Import(format!("cannot read {}: {}", tfstate_path.display(), e))
    })?;

    let tf: TfState = if tfstate_path.extension().is_some_and(|e| e == "json")
        || raw.trim_start().starts_with('{')
    {
        serde_json::from_str(&raw)?
    } else {
        serde_json::from_str(&raw).map_err(|_| {
            CoreError::Import(
                "unsupported tfstate format — expected JSON (terraform state v4+)".into(),
            )
        })?
    };

    let mut imported = 0;
    let mut skipped = 0;
    let mut deps: Vec<(String, String)> = Vec::new();

    for resource in tf.resources {
        if resource.mode != "managed" && !resource.mode.is_empty() {
            skipped += 1;
            continue;
        }

        for instance in resource.instances {
            let cloud_uid = extract_cloud_uid(&resource.resource_type, &instance.attributes);
            if cloud_uid.is_empty() {
                skipped += 1;
                continue;
            }

            let address = format!("{}.{}", resource.resource_type, resource.name);
            let provider = normalize_provider(&resource.provider);
            let had_sensitive = attrs_have_sensitive(&instance.attributes);

            let mut ferrum_resource = ResourceInstance::new(
                address.clone(),
                resource.resource_type.clone(),
                cloud_uid,
                provider,
            );
            ferrum_resource.attributes = sanitize_attributes(instance.attributes);

            if !instance.dependencies.is_empty() {
                ferrum_resource.attributes["depends_on"] =
                    serde_json::Value::Array(
                        instance
                            .dependencies
                            .iter()
                            .map(|d| serde_json::Value::String(d.clone()))
                            .collect(),
                    );
            }

            for dep in &instance.dependencies {
                deps.push((dep.clone(), address.clone()));
            }

            if had_sensitive {
                ferrum_resource.attributes["ferrum_vault_protected"] =
                    serde_json::Value::Bool(true);
            }

            if output_state.find_by_cloud_uid(&ferrum_resource.cloud_uid).is_none() {
                output_state.resources_mut().push(ferrum_resource);
                imported += 1;
            } else {
                skipped += 1;
            }
        }
    }

    if let Some(lineage) = tf.lineage {
        output_state.metadata.lineage = lineage;
    }
    output_state.metadata.serial = tf.serial;

    let graph = InfrastructureGraph::from_resources(output_state.resources(), &deps);
    let graph_path = graph_output_path(output_state.path());
    graph
        .save(&graph_path)
        .map_err(|e| CoreError::Import(format!("cannot write graph: {e}")))?;

    Ok(ImportReport {
        imported,
        skipped,
        tf_version: tf.version,
        graph_path,
        edge_count: deps.len(),
    })
}

fn graph_output_path(state_path: &Path) -> PathBuf {
    state_path
        .parent()
        .unwrap_or(Path::new("."))
        .join(GRAPH_FILENAME)
}

fn extract_cloud_uid(resource_type: &str, attrs: &serde_json::Value) -> String {
    const UID_FIELDS: &[&str] = &[
        "id", "arn", "resource_id", "self_link", "azure_id", "identity",
    ];

    for field in UID_FIELDS {
        if let Some(val) = attrs.get(field) {
            if let Some(s) = val.as_str() {
                if !s.is_empty() {
                    return s.to_string();
                }
            }
        }
    }

    if let Some(name) = attrs.get("name").and_then(|v| v.as_str()) {
        return format!("{}:{}", resource_type, name);
    }

    String::new()
}

fn normalize_provider(provider: &str) -> String {
    provider
        .trim_start_matches("provider[")
        .trim_end_matches(']')
        .trim_matches('"')
        .split('.')
        .last()
        .unwrap_or("unknown")
        .to_string()
}

fn attrs_have_sensitive(attrs: &serde_json::Value) -> bool {
    if let Some(obj) = attrs.as_object() {
        const SENSITIVE: &[&str] = &[
            "password", "secret", "token", "access_key", "secret_key", "private_key",
        ];
        return obj.keys().any(|k| {
            let lower = k.to_lowercase();
            SENSITIVE.iter().any(|s| lower.contains(s))
        });
    }
    false
}

fn sanitize_attributes(mut attrs: serde_json::Value) -> serde_json::Value {
    if let Some(obj) = attrs.as_object_mut() {
        const SENSITIVE: &[&str] = &[
            "password", "secret", "token", "access_key", "secret_key", "private_key",
        ];
        obj.retain(|k, _| {
            let lower = k.to_lowercase();
            !SENSITIVE.iter().any(|s| lower.contains(s))
        });
    }
    attrs
}

#[derive(Debug)]
pub struct ImportReport {
    pub imported: usize,
    pub skipped: usize,
    pub tf_version: u32,
    pub graph_path: PathBuf,
    pub edge_count: usize,
}
