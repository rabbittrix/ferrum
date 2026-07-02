use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeFile {
    pub providers: Vec<FeProvider>,
    pub resources: Vec<FeResource>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeProvider {
    pub name: String,
    pub config: HashMap<String, FeValue>,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeResource {
    pub resource_type: String,
    pub name: String,
    pub attributes: HashMap<String, FeValue>,
    pub depends_on: Vec<String>,
    pub line: usize,
    pub column: usize,
}

/// Cross-resource reference, e.g. `aws_vpc.main.id` or `aws_vpc.main`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FeReference {
    pub resource_type: String,
    pub name: String,
    pub attribute: Option<String>,
}

impl FeReference {
    pub fn address(&self) -> String {
        format!("{}.{}", self.resource_type, self.name)
    }

    pub fn parse(raw: &str) -> Option<Self> {
        let parts: Vec<&str> = raw.split('.').collect();
        match parts.as_slice() {
            [t, n] => Some(Self {
                resource_type: (*t).into(),
                name: (*n).into(),
                attribute: None,
            }),
            [t, n, a] => Some(Self {
                resource_type: (*t).into(),
                name: (*n).into(),
                attribute: Some((*a).into()),
            }),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum FeValue {
    String(String),
    Number(f64),
    Bool(bool),
    List(Vec<FeValue>),
    Object(HashMap<String, FeValue>),
    Ref(FeReference),
}

impl FeResource {
    pub fn address(&self) -> String {
        format!("{}.{}", self.resource_type, self.name)
    }
}
