use crate::ast::{FeFile, FeResource, FeValue};
use crate::error::{ParseError, Result};
use crate::symbol::SymbolTable;

/// Known resource schemas — validated before `ferrum plan`.
const REQUIRED_ATTRS: &[(&str, &[&str])] = &[
    ("aws_vpc", &["cidr_block"]),
    ("aws_subnet", &["vpc_id", "cidr_block"]),
    ("aws_instance", &["ami", "instance_type"]),
    ("aws_security_group", &["name", "vpc_id"]),
    ("azurerm_resource_group", &["name", "location"]),
    ("google_compute_network", &["name"]),
];

#[derive(Debug, Default)]
pub struct TypeCheckReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn typecheck(file: &FeFile) -> TypeCheckReport {
    let symbols = SymbolTable::from_file(file);
    let mut report = TypeCheckReport::default();

    match validate_file(file, &symbols) {
        Ok(()) => {}
        Err(ParseError::TypeError { message, .. }) => report.errors.push(message),
        Err(e) => report.errors.push(e.to_string()),
    }

    for resource in &file.resources {
        if REQUIRED_ATTRS
            .iter()
            .all(|(t, _)| *t != resource.resource_type)
            && resource.resource_type.contains('_')
        {
            report.warnings.push(format!(
                "line {}, column {}: unknown resource type '{}' — provider plugin may be required",
                resource.line, resource.column, resource.resource_type
            ));
        }
    }

    report
}

pub fn typecheck_or_err(file: &FeFile) -> Result<()> {
    let symbols = SymbolTable::from_file(file);
    validate_file(file, &symbols)
}

pub fn validate_file(file: &FeFile, symbols: &SymbolTable) -> Result<()> {
    let mut errors: Vec<(usize, usize, String, String)> = Vec::new();

    for resource in &file.resources {
        validate_required_attrs(resource, &mut errors);
        validate_depends_on(resource, symbols, &mut errors);

        for value in resource.attributes.values() {
            symbols.validate_refs_in_value(
                value,
                resource.line,
                resource.column,
                &resource.address(),
                &mut errors,
            );
        }
    }

    if let Some((line, column, resource, message)) = errors.into_iter().next() {
        return Err(ParseError::TypeError {
            line,
            column,
            resource,
            message,
        });
    }

    Ok(())
}

fn validate_required_attrs(
    resource: &FeResource,
    errors: &mut Vec<(usize, usize, String, String)>,
) {
    let Some((_, required)) = REQUIRED_ATTRS
        .iter()
        .find(|(t, _)| *t == resource.resource_type)
    else {
        return;
    };

    for attr in *required {
        if !resource.attributes.contains_key(*attr) {
            errors.push((
                resource.line,
                resource.column,
                resource.address(),
                format!(
                    "missing required attribute '{}' for resource type '{}'",
                    attr, resource.resource_type
                ),
            ));
        }
    }
}

fn validate_depends_on(
    resource: &FeResource,
    symbols: &SymbolTable,
    errors: &mut Vec<(usize, usize, String, String)>,
) {
    for dep in &resource.depends_on {
        if !symbols.contains(dep) {
            errors.push((
                resource.line,
                resource.column,
                resource.address(),
                format!("depends_on references undefined resource '{dep}'"),
            ));
        }
    }
}

/// Convert a Ferrum value to a display string for diff output.
pub fn fe_value_display(value: &FeValue) -> String {
    match value {
        FeValue::String(s) => format!("\"{s}\""),
        FeValue::Number(n) => n.to_string(),
        FeValue::Bool(b) => b.to_string(),
        FeValue::Ref(r) => match &r.attribute {
            Some(a) => format!("{}.{}.{}", r.resource_type, r.name, a),
            None => format!("{}.{}", r.resource_type, r.name),
        },
        FeValue::List(items) => {
            let inner: Vec<_> = items.iter().map(fe_value_display).collect();
            format!("[{}]", inner.join(", "))
        }
        FeValue::Object(map) => {
            let inner: Vec<_> = map
                .iter()
                .map(|(k, v)| format!("{k}: {}", fe_value_display(v)))
                .collect();
            format!("{{ {} }}", inner.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::parse_fe_source;

    #[test]
    fn rejects_missing_required_field() {
        let err = parse_fe_source(r#"resource aws_vpc main { region = "x" }"#).unwrap_err();
        assert!(err.to_string().contains("cidr_block"));
    }
}
