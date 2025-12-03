//! Error types for the eulumdat crate.

use thiserror::Error;

/// Result type alias for eulumdat operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur when working with Eulumdat files.
#[derive(Error, Debug)]
pub enum Error {
    /// I/O error when reading or writing files.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Parse error at a specific line.
    #[error("Parse error at line {line}: {message}")]
    Parse { line: usize, message: String },

    /// Invalid value for a field.
    #[error("Invalid value for {field}: {message}")]
    InvalidValue { field: String, message: String },

    /// Missing required data.
    #[error("Missing required data: {0}")]
    MissingData(String),

    /// Invalid symmetry configuration.
    #[error("Invalid symmetry configuration: {0}")]
    InvalidSymmetry(String),

    /// Data validation failed.
    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    /// Invalid file format.
    #[error("Invalid file format: {0}")]
    InvalidFormat(String),
}

impl Error {
    /// Create a parse error at a specific line.
    pub fn parse(line: usize, message: impl Into<String>) -> Self {
        Self::Parse {
            line,
            message: message.into(),
        }
    }

    /// Create an invalid value error.
    pub fn invalid_value(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InvalidValue {
            field: field.into(),
            message: message.into(),
        }
    }
}
