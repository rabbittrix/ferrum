use std::collections::HashMap;

use crate::ast::{FeFile, FeReference, FeResource, FeValue};

/// Symbol table for resource addresses and cross-reference validation.
#[derive(Clone, Debug, Default)]
pub struct SymbolTable {
    resources: HashMap<String, ResourceSymbol>,
}

#[derive(Clone, Debug)]
pub struct ResourceSymbol {
    pub address: String,
    pub resource_type: String,
    pub name: String,
    pub line: usize,
    pub column: usize,
}

impl SymbolTable {
    pub fn from_file(file: &FeFile) -> Self {
        let mut table = Self::default();
        for resource in &file.resources {
            table.register(resource);
        }
        table
    }

    pub fn register(&mut self, resource: &FeResource) {
        self.resources.insert(
            resource.address(),
            ResourceSymbol {
                address: resource.address(),
                resource_type: resource.resource_type.clone(),
                name: resource.name.clone(),
                line: resource.line,
                column: resource.column,
            },
        );
    }

    pub fn contains(&self, address: &str) -> bool {
        self.resources.contains_key(address)
    }

    pub fn get(&self, address: &str) -> Option<&ResourceSymbol> {
        self.resources.get(address)
    }

    pub fn addresses(&self) -> impl Iterator<Item = &String> {
        self.resources.keys()
    }

    /// Validate that a reference points to a declared resource.
    pub fn resolve_ref(&self, reference: &FeReference) -> Option<&ResourceSymbol> {
        self.get(&reference.address())
    }

    /// Walk attribute values and collect reference validation errors.
    pub fn validate_refs_in_value(
        &self,
        value: &FeValue,
        line: usize,
        column: usize,
        resource: &str,
        errors: &mut Vec<(usize, usize, String, String)>,
    ) {
        match value {
            FeValue::Ref(r) => {
                if !self.contains(&r.address()) {
                    errors.push((
                        line,
                        column,
                        resource.into(),
                        format!(
                            "reference '{}' is undefined — no resource with address '{}'",
                            format_ref(r),
                            r.address()
                        ),
                    ));
                }
            }
            FeValue::List(items) => {
                for item in items {
                    self.validate_refs_in_value(item, line, column, resource, errors);
                }
            }
            FeValue::Object(map) => {
                for item in map.values() {
                    self.validate_refs_in_value(item, line, column, resource, errors);
                }
            }
            _ => {}
        }
    }
}

fn format_ref(r: &FeReference) -> String {
    match &r.attribute {
        Some(a) => format!("{}.{}.{}", r.resource_type, r.name, a),
        None => r.address(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_fe_source;

    #[test]
    fn symbol_table_registers_resources() {
        let file = parse_fe_source(
            r#"resource aws_vpc main { cidr_block = "10.0.0.0/16" }"#,
        )
        .unwrap();
        let table = SymbolTable::from_file(&file);
        assert!(table.contains("aws_vpc.main"));
    }
}
