//! Error types for report generation.

use thiserror::Error;

/// Result type for report generation operations.
pub type Result<T> = std::result::Result<T, ReportError>;

/// Errors that can occur during report generation.
#[derive(Error, Debug)]
pub enum ReportError {
    /// Error parsing photometric data
    #[error("Failed to parse photometric data: {0}")]
    ParseError(String),

    /// Error generating Typst source
    #[error("Failed to generate Typst source: {0}")]
    TemplateError(String),

    /// Error compiling Typst to PDF
    #[error("Failed to compile PDF: {0}")]
    CompileError(String),

    /// Error during Typst compilation (feature = "compile")
    #[error("Typst compilation failed: {0}")]
    Compile(String),

    /// Error generating SVG diagram
    #[error("Failed to generate diagram: {0}")]
    DiagramError(String),

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}
