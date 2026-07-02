use thiserror::Error;

#[derive(Debug, Error)]
pub enum ResolveError {
    #[error("dependency cycle: {0}")]
    Cycle(String),

    #[error("missing dependency: {0}")]
    Missing(String),
}

pub type Result<T> = std::result::Result<T, ResolveError>;
