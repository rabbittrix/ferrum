//! Map Terraform provider schemas (official v5/v6) to Ferrum `.fe` validation rules.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::{BridgeError, Result};
use crate::tfplugin::TfPluginClient;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ResourceSchema {
    pub resource_type: String,
    pub required_attributes: Vec<String>,
    pub optional_attributes: Vec<String>,
    pub computed_attributes: Vec<String>,
    pub sensitive_attributes: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProviderSchemaRegistry {
    pub provider_name: String,
    pub protocol_version: u32,
    pub resources: HashMap<String, ResourceSchema>,
}

impl ProviderSchemaRegistry {
    pub fn required_for(&self, resource_type: &str) -> Option<&[String]> {
        self.resources
            .get(resource_type)
            .map(|s| s.required_attributes.as_slice())
    }

    pub fn merge_into_parser_registry(&self) -> HashMap<String, Vec<String>> {
        self.resources
            .iter()
            .map(|(k, v)| (k.clone(), v.required_attributes.clone()))
            .collect()
    }
}

pub async fn fetch_provider_schemas(
    client: &mut TfPluginClient,
    provider_name: &str,
) -> Result<ProviderSchemaRegistry> {
    match client {
        TfPluginClient::V5 { .. } => {
            let response = client.get_schema_v5().await?;
            check_diagnostics_v5(&response.diagnostics)?;
            let mut resources = HashMap::new();
            for (type_name, schema) in response.resource_schemas {
                resources.insert(type_name.clone(), map_schema_v5(&type_name, &schema));
            }
            Ok(ProviderSchemaRegistry {
                provider_name: provider_name.to_string(),
                protocol_version: 5,
                resources,
            })
        }
        TfPluginClient::V6 { .. } => {
            let response = client.get_schema_v6().await?;
            check_diagnostics_v6(&response.diagnostics)?;
            let mut resources = HashMap::new();
            for (type_name, schema) in response.resource_schemas {
                resources.insert(type_name.clone(), map_schema_v6(&type_name, &schema));
            }
            Ok(ProviderSchemaRegistry {
                provider_name: provider_name.to_string(),
                protocol_version: 6,
                resources,
            })
        }
    }
}

fn map_schema_v5(resource_type: &str, schema: &crate::tfplugin5::Schema) -> ResourceSchema {
    let mut required = Vec::new();
    let mut optional = Vec::new();
    let mut computed = Vec::new();
    let mut sensitive = Vec::new();

    if let Some(block) = &schema.block {
        for attr in &block.attributes {
            classify_attr(attr, &mut required, &mut optional, &mut computed, &mut sensitive);
        }
        for nested in &block.block_types {
            if let Some(inner) = &nested.block {
                for attr in &inner.attributes {
                    classify_attr(attr, &mut required, &mut optional, &mut computed, &mut sensitive);
                }
            }
        }
    }

    ResourceSchema {
        resource_type: resource_type.to_string(),
        required_attributes: required,
        optional_attributes: optional,
        computed_attributes: computed,
        sensitive_attributes: sensitive,
    }
}

fn map_schema_v6(resource_type: &str, schema: &crate::tfplugin6::Schema) -> ResourceSchema {
    let mut required = Vec::new();
    let mut optional = Vec::new();
    let mut computed = Vec::new();
    let mut sensitive = Vec::new();

    if let Some(block) = &schema.block {
        for attr in &block.attributes {
            classify_attr_v6(attr, &mut required, &mut optional, &mut computed, &mut sensitive);
        }
        for nested in &block.block_types {
            if let Some(inner) = &nested.block {
                for attr in &inner.attributes {
                    classify_attr_v6(attr, &mut required, &mut optional, &mut computed, &mut sensitive);
                }
            }
        }
    }

    ResourceSchema {
        resource_type: resource_type.to_string(),
        required_attributes: required,
        optional_attributes: optional,
        computed_attributes: computed,
        sensitive_attributes: sensitive,
    }
}

fn classify_attr(
    attr: &crate::tfplugin5::schema::Attribute,
    required: &mut Vec<String>,
    optional: &mut Vec<String>,
    computed: &mut Vec<String>,
    sensitive: &mut Vec<String>,
) {
    if attr.required {
        required.push(attr.name.clone());
    } else if attr.optional {
        optional.push(attr.name.clone());
    }
    if attr.computed {
        computed.push(attr.name.clone());
    }
    if attr.sensitive {
        sensitive.push(attr.name.clone());
    }
}

fn classify_attr_v6(
    attr: &crate::tfplugin6::schema::Attribute,
    required: &mut Vec<String>,
    optional: &mut Vec<String>,
    computed: &mut Vec<String>,
    sensitive: &mut Vec<String>,
) {
    if attr.required {
        required.push(attr.name.clone());
    } else if attr.optional {
        optional.push(attr.name.clone());
    }
    if attr.computed {
        computed.push(attr.name.clone());
    }
    if attr.sensitive {
        sensitive.push(attr.name.clone());
    }
}

fn check_diagnostics_v5(diags: &[crate::tfplugin5::Diagnostic]) -> Result<()> {
    for diag in diags {
        if diag.severity == 1 {
            return Err(BridgeError::Schema(format!(
                "{}: {}",
                diag.summary, diag.detail
            )));
        }
    }
    Ok(())
}

fn check_diagnostics_v6(diags: &[crate::tfplugin6::Diagnostic]) -> Result<()> {
    for diag in diags {
        if diag.severity == 1 {
            return Err(BridgeError::Schema(format!(
                "{}: {}",
                diag.summary, diag.detail
            )));
        }
    }
    Ok(())
}

pub fn provider_for_resource_type(resource_type: &str) -> &str {
    resource_type.split('_').next().unwrap_or("unknown")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tfplugin5::schema::{Attribute, Block};
    use crate::tfplugin5::Schema;

    #[test]
    fn maps_v5_required_attributes() {
        let schema = Schema {
            version: 0,
            block: Some(Block {
                version: 0,
                attributes: vec![Attribute {
                    name: "cidr_block".into(),
                    r#type: vec![],
                    description: String::new(),
                    required: true,
                    optional: false,
                    computed: false,
                    sensitive: false,
                    description_kind: 0,
                    deprecated: false,
                    write_only: false,
                    deprecation_message: String::new(),
                }],
                block_types: vec![],
                description: String::new(),
                description_kind: 0,
                deprecated: false,
                deprecation_message: String::new(),
            }),
        };
        let mapped = map_schema_v5("aws_vpc", &schema);
        assert_eq!(mapped.required_attributes, vec!["cidr_block"]);
    }
}
