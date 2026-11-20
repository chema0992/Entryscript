use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("Lexer error at {line}:{column}: {message}")]
    Lexer {
        line: usize,
        column: usize,
        message: String,
    },

    #[error("Parser error at {line}:{column}: {message}")]
    Parser {
        line: usize,
        column: usize,
        message: String,
    },

    #[error("Semantic error: {0}")]
    Semantic(String),

    #[error("Entry project error: {0}")]
    Project(String),

    #[error("Invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
}

impl CompileError {
    pub fn lexer(line: usize, column: usize, message: impl Into<String>) -> Self {
        Self::Lexer {
            line,
            column,
            message: message.into(),
        }
    }

    pub fn parser(line: usize, column: usize, message: impl Into<String>) -> Self {
        Self::Parser {
            line,
            column,
            message: message.into(),
        }
    }
}
