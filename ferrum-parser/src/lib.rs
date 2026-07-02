//! Strongly-typed `.fe` language parser for Ferrum IaC.

mod ast;
mod error;
mod parse;
mod symbol;
mod types;

pub use ast::{FeFile, FeProvider, FeReference, FeResource, FeValue};
pub use error::{line_col, ParseError, Result};
pub use parse::{merge_fe_files, parse_fe, parse_fe_dir, parse_fe_source};
pub use symbol::{ResourceSymbol, SymbolTable};
pub use types::{fe_value_display, typecheck, typecheck_or_err, validate_file, TypeCheckReport};
