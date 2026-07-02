//! Dynamic schema registry — merges built-in and provider-fetched schemas.

use std::collections::HashMap;

/// Built-in fallback required attributes (used when provider is offline).
const BUILTIN_REQUIRED: &[(&str, &[&str])] = &[
    ("aws_vpc", &["cidr_block"]),
    ("aws_subnet", &["vpc_id", "cidr_block"]),
    ("aws_instance", &["ami", "instance_type"]),
    ("aws_security_group", &["name", "vpc_id"]),
    ("azurerm_resource_group", &["name", "location"]),
    ("google_compute_network", &["name"]),
];

#[derive(Clone, Debug, Default)]
pub struct SchemaRegistry {
    required: HashMap<String, Vec<String>>,
}

impl SchemaRegistry {
    pub fn with_builtins() -> Self {
        let mut required = HashMap::new();
        for (rtype, attrs) in BUILTIN_REQUIRED {
            required.insert((*rtype).to_string(), attrs.iter().map(|s| (*s).to_string()).collect());
        }
        Self { required }
    }

    pub fn merge_provider(&mut self, provider_resources: HashMap<String, Vec<String>>) {
        for (rtype, attrs) in provider_resources {
            self.required.insert(rtype, attrs);
        }
    }

    pub fn required_for(&self, resource_type: &str) -> Option<&[String]> {
        self.required.get(resource_type).map(|v| v.as_slice())
    }
}
