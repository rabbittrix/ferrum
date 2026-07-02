use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("parse error: {0}")]
    Parse(#[from] ferrum_parser::ParseError),

    #[error("resolve error: {0}")]
    Resolve(#[from] ferrum_resolver::ResolveError),

    #[error("state error: {0}")]
    State(#[from] ferrum_state::StateError),

    #[error("graph error: {0}")]
    Graph(String),

    #[error("import error: {0}")]
    Import(String),

    #[error("provider error: {0}")]
    Provider(String),

    #[error("plan error: {0}")]
    Plan(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, CoreError>;
