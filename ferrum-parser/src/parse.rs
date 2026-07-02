use std::collections::HashMap;
use std::fs;
use std::path::Path;

use pest::error::LineColLocation;
use pest::Parser;
use pest_derive::Parser;

use crate::ast::{FeFile, FeProvider, FeReference, FeResource, FeValue};
use crate::error::{line_col, ParseError, Result};
use crate::symbol::SymbolTable;
use crate::types::validate_file;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct FeGrammar;

/// Parse a `.fe` source file into a strongly-typed AST.
pub fn parse_fe(path: &Path) -> Result<FeFile> {
    let source = fs::read_to_string(path)?;
    parse_fe_source(&source)
}

/// Parse and validate `.fe` source (syntax + symbol table + schema).
pub fn parse_fe_source(source: &str) -> Result<FeFile> {
    let file = parse_syntax(source)?;
    let symbols = SymbolTable::from_file(&file);
    validate_file(&file, &symbols)?;
    Ok(file)
}

fn parse_syntax(source: &str) -> Result<FeFile> {
    let pairs = FeGrammar::parse(Rule::file, source).map_err(|e| {
        let (line, column) = match e.line_col {
            LineColLocation::Pos((l, c)) => (l, c),
            LineColLocation::Span((l, c), _) => (l, c),
        };
        ParseError::Syntax {
            line,
            column,
            message: e.to_string(),
        }
    })?;

    let mut providers = Vec::new();
    let mut resources = Vec::new();

    for pair in pairs {
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::block => {
                    let block = inner.into_inner().next().unwrap();
                    match block.as_rule() {
                        Rule::provider_block => {
                            providers.push(parse_provider(block, source)?);
                        }
                        Rule::resource_block => {
                            resources.push(parse_resource(block, source)?);
                        }
                        _ => {}
                    }
                }
                Rule::provider_block => {
                    providers.push(parse_provider(inner, source)?);
                }
                Rule::resource_block => {
                    resources.push(parse_resource(inner, source)?);
                }
                Rule::EOI | Rule::COMMENT => {}
                _ => {}
            }
        }
    }

    Ok(FeFile {
        providers,
        resources,
    })
}

fn span_pos(source: &str, span: pest::Span<'_>) -> (usize, usize) {
    line_col(source, span.start())
}

fn parse_provider(pair: pest::iterators::Pair<'_, Rule>, source: &str) -> Result<FeProvider> {
    let span = pair.as_span();
    let (line, column) = span_pos(source, span);
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let config = parse_attr_list(inner, source)?;
    Ok(FeProvider {
        name,
        config,
        line,
        column,
    })
}

fn parse_resource(pair: pest::iterators::Pair<'_, Rule>, source: &str) -> Result<FeResource> {
    let span = pair.as_span();
    let (line, column) = span_pos(source, span);
    let mut inner = pair.into_inner();
    let resource_type = parse_resource_id(inner.next().unwrap());
    let name = parse_resource_id(inner.next().unwrap());
    let attrs = parse_attr_list(inner, source)?;

    let mut depends_on = extract_depends_on(&attrs);
    let implicit = implicit_dependencies(&attrs);
    for dep in implicit {
        if !depends_on.contains(&dep) {
            depends_on.push(dep);
        }
    }

    Ok(FeResource {
        resource_type,
        name,
        attributes: attrs,
        depends_on,
        line,
        column,
    })
}

fn parse_resource_id(pair: pest::iterators::Pair<'_, Rule>) -> String {
    match pair.as_rule() {
        Rule::quoted => unquote(pair.as_str()),
        Rule::ident => pair.as_str().to_string(),
        Rule::resource_id => {
            let inner = pair.into_inner().next().unwrap();
            parse_resource_id(inner)
        }
        _ => pair.as_str().to_string(),
    }
}

fn parse_attr_list(
    pairs: pest::iterators::Pairs<'_, Rule>,
    source: &str,
) -> Result<HashMap<String, FeValue>> {
    let mut map = HashMap::new();
    for attr in pairs {
        if attr.as_rule() == Rule::attr {
            let (key, value) = parse_attr_pair(attr, source)?;
            map.insert(key, value);
        }
    }
    Ok(map)
}

fn parse_attr_pair(
    attr: pest::iterators::Pair<'_, Rule>,
    source: &str,
) -> Result<(String, FeValue)> {
    let mut parts = attr.into_inner();
    let key = parts.next().unwrap().as_str().to_string();
    parts.next(); // assign
    let value = parse_value(parts.next().unwrap(), source)?;
    Ok((key, value))
}

fn parse_value(pair: pest::iterators::Pair<'_, Rule>, source: &str) -> Result<FeValue> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::quoted => Ok(FeValue::String(unquote(inner.as_str()))),
        Rule::number => {
            let n: f64 = inner.as_str().parse().map_err(|_| {
                let (line, column) = span_pos(source, inner.as_span());
                ParseError::Syntax {
                    line,
                    column,
                    message: format!("invalid number '{}'", inner.as_str()),
                }
            })?;
            Ok(FeValue::Number(n))
        }
        Rule::bool_lit => Ok(FeValue::Bool(inner.as_str() == "true")),
        Rule::reference => Ok(FeValue::Ref(parse_reference(inner.as_str()))),
        Rule::list => {
            let items = inner
                .into_inner()
                .filter(|p| p.as_rule() == Rule::value)
                .map(|p| parse_value(p, source))
                .collect::<Result<Vec<_>>>()?;
            Ok(FeValue::List(items))
        }
        Rule::object => {
            let obj = parse_attr_list(inner.into_inner(), source)?;
            Ok(FeValue::Object(obj))
        }
        _ => {
            let (line, column) = span_pos(source, inner.as_span());
            Err(ParseError::Syntax {
                line,
                column,
                message: "expected value".into(),
            })
        }
    }
}

fn parse_reference(raw: &str) -> FeReference {
    FeReference::parse(raw).unwrap_or_else(|| FeReference {
        resource_type: raw.into(),
        name: String::new(),
        attribute: None,
    })
}

fn unquote(s: &str) -> String {
    s.trim_matches('"').to_string()
}

fn extract_depends_on(attrs: &HashMap<String, FeValue>) -> Vec<String> {
    match attrs.get("depends_on") {
        Some(FeValue::List(items)) => items
            .iter()
            .filter_map(dep_from_value)
            .collect(),
        Some(v) => dep_from_value(v).into_iter().collect(),
        None => Vec::new(),
    }
}

fn dep_from_value(v: &FeValue) -> Option<String> {
    match v {
        FeValue::String(s) => Some(normalize_dep(s)),
        FeValue::Ref(r) => Some(r.address()),
        _ => None,
    }
}

fn normalize_dep(s: &str) -> String {
    if let Some(r) = FeReference::parse(s) {
        r.address()
    } else {
        s.to_string()
    }
}

/// Collect resource addresses referenced in attribute values.
pub fn implicit_dependencies(attrs: &HashMap<String, FeValue>) -> Vec<String> {
    let mut deps = Vec::new();
    for (key, value) in attrs {
        if key == "depends_on" {
            continue;
        }
        collect_ref_deps(value, &mut deps);
    }
    deps
}

fn collect_ref_deps(value: &FeValue, deps: &mut Vec<String>) {
    match value {
        FeValue::Ref(r) => {
            let addr = r.address();
            if !deps.contains(&addr) {
                deps.push(addr);
            }
        }
        FeValue::List(items) => items.iter().for_each(|v| collect_ref_deps(v, deps)),
        FeValue::Object(map) => map.values().for_each(|v| collect_ref_deps(v, deps)),
        _ => {}
    }
}

/// Merge multiple parsed `.fe` files into one project file.
pub fn merge_fe_files(mut files: Vec<FeFile>) -> Result<FeFile> {
    let mut merged = FeFile {
        providers: Vec::new(),
        resources: Vec::new(),
    };
    for file in files.drain(..) {
        merged.providers.extend(file.providers);
        for resource in file.resources {
            if merged
                .resources
                .iter()
                .any(|r| r.address() == resource.address())
            {
                return Err(ParseError::TypeError {
                    line: resource.line,
                    column: resource.column,
                    resource: resource.address(),
                    message: format!("duplicate resource address '{}'", resource.address()),
                });
            }
            merged.resources.push(resource);
        }
    }
    let symbols = SymbolTable::from_file(&merged);
    validate_file(&merged, &symbols)?;
    Ok(merged)
}

/// Discover and parse all `.fe` files in a directory (non-recursive).
pub fn parse_fe_dir(dir: &Path) -> Result<FeFile> {
    let mut paths: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "fe"))
        .collect();
    paths.sort();

    if paths.is_empty() {
        return Err(ParseError::Syntax {
            line: 1,
            column: 1,
            message: format!(
                "no .fe configuration files found in {}",
                dir.display()
            ),
        });
    }

    let files: Result<Vec<_>> = paths.iter().map(|p| parse_fe(p)).collect();
    merge_fe_files(files?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::typecheck;

    const EXAMPLE: &str = r#"
resource "aws_vpc" "main" {
    cidr_block: "10.0.0.0/16",
    enable_dns_support: true,
    tags: {
        name: "Production-VPC"
    }
}

resource "aws_subnet" "public" {
    vpc_id: aws_vpc.main.id,
    cidr_block: "10.0.1.0/24"
}
"#;

    #[test]
    fn parse_colon_syntax_and_refs() {
        let file = parse_fe_source(EXAMPLE).unwrap();
        assert_eq!(file.resources.len(), 2);
        assert_eq!(file.resources[0].address(), "aws_vpc.main");
        assert!(matches!(
            file.resources[1].attributes.get("vpc_id"),
            Some(FeValue::Ref(r)) if r.address() == "aws_vpc.main"
        ));
        assert!(file.resources[1].depends_on.contains(&"aws_vpc.main".to_string()));
        let report = typecheck(&file);
        assert!(report.errors.is_empty());
    }

    #[test]
    fn parse_legacy_equals_syntax() {
        let src = r#"
            provider aws { region = "us-east-1" }
            resource aws_vpc main { cidr_block = "10.0.0.0/16" }
        "#;
        let file = parse_fe_source(src).unwrap();
        assert_eq!(file.resources.len(), 1);
        assert_eq!(file.resources[0].address(), "aws_vpc.main");
    }

    #[test]
    fn type_error_missing_cidr_has_line_column() {
        let src = r#"resource aws_vpc main { region = "x" }"#;
        let err = parse_fe_source(src).unwrap_err();
        match err {
            ParseError::TypeError { line, column, .. } => {
                assert_eq!(line, 1);
                assert!(column >= 1);
            }
            other => panic!("expected TypeError, got {other:?}"),
        }
    }

    #[test]
    fn unknown_ref_fails_validation() {
        let src = r#"
resource aws_subnet public {
    vpc_id: aws_vpc.missing.id,
    cidr_block: "10.0.1.0/24"
}
"#;
        let err = parse_fe_source(src).unwrap_err();
        assert!(err.to_string().contains("aws_vpc.missing"));
    }
}
