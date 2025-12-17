//! Error types for ATLA S001 / TM-33 / UNI 11733 parsing

use thiserror::Error;

/// Errors that can occur when parsing ATLA documents
#[derive(Error, Debug)]
pub enum AtlaError {
    #[error("XML parsing error: {0}")]
    XmlParse(String),

    #[error("JSON parsing error: {0}")]
    JsonParse(String),

    #[error("Missing required element: {0}")]
    MissingElement(String),

    #[error("Invalid value for {field}: {value}")]
    InvalidValue { field: String, value: String },

    #[error("Invalid intensity metric type: {0}")]
    InvalidIntensityMetric(String),

    #[error("Invalid photometry type: {0}")]
    InvalidPhotometryType(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Unsupported schema version: {0}")]
    UnsupportedVersion(String),
}

#[cfg(feature = "xml")]
impl From<quick_xml::Error> for AtlaError {
    fn from(e: quick_xml::Error) -> Self {
        AtlaError::XmlParse(e.to_string())
    }
}

#[cfg(feature = "xml")]
impl From<quick_xml::DeError> for AtlaError {
    fn from(e: quick_xml::DeError) -> Self {
        AtlaError::XmlParse(e.to_string())
    }
}

#[cfg(feature = "json")]
impl From<serde_json::Error> for AtlaError {
    fn from(e: serde_json::Error) -> Self {
        AtlaError::JsonParse(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AtlaError>;
