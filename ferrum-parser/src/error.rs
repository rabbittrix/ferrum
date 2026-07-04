use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("syntax error at line {line}, column {column}: {message}")]
    Syntax {
        line: usize,
        column: usize,
        message: String,
    },

    #[error("type error at line {line}, column {column} in {resource}: {message}")]
    TypeError {
        line: usize,
        column: usize,
        resource: String,
        message: String,
    },

    #[error(
        "No Ferrum configuration files (.fe) found in this directory.\n  Directory: {dir}\n  Try running `ferrum init` to create a project or `ferrum test-drive` to see a demo."
    )]
    NoConfigFiles { dir: String },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, ParseError>;

pub fn line_col(source: &str, pos: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (i, c) in source.char_indices() {
        if i >= pos {
            break;
        }
        if c == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}
