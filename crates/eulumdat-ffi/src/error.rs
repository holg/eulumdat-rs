//! Error types for FFI

/// Error type for FFI
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum EulumdatError {
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("IO error: {0}")]
    IoError(String),
}
