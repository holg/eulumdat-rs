//! Validation types and functions for FFI

use crate::error::EulumdatError;
use crate::types::{to_core_eulumdat, Eulumdat};

/// A validation warning (non-fatal issue)
#[derive(Debug, Clone, uniffi::Record)]
pub struct ValidationWarning {
    /// Warning code for programmatic handling
    pub code: String,
    /// Human-readable warning message
    pub message: String,
}

impl From<&eulumdat::ValidationWarning> for ValidationWarning {
    fn from(w: &eulumdat::ValidationWarning) -> Self {
        Self {
            code: w.code.to_string(),
            message: w.message.clone(),
        }
    }
}

/// A validation error (fatal issue)
#[derive(Debug, Clone, uniffi::Record)]
pub struct ValidationError {
    /// Error code for programmatic handling
    pub code: String,
    /// Human-readable error message
    pub message: String,
}

impl From<&eulumdat::ValidationError> for ValidationError {
    fn from(e: &eulumdat::ValidationError) -> Self {
        Self {
            code: e.code.to_string(),
            message: e.message.clone(),
        }
    }
}

/// Validate Eulumdat data and return warnings
#[uniffi::export]
pub fn validate_ldt(ldt: &Eulumdat) -> Vec<ValidationWarning> {
    let core_ldt = to_core_eulumdat(ldt);
    eulumdat::validate(&core_ldt)
        .iter()
        .map(|w: &eulumdat::ValidationWarning| w.into())
        .collect()
}

/// Validate Eulumdat data strictly, returning errors if invalid
#[uniffi::export]
pub fn validate_ldt_strict(ldt: &Eulumdat) -> Result<(), EulumdatError> {
    let core_ldt = to_core_eulumdat(ldt);
    eulumdat::validate_strict(&core_ldt).map_err(|errors: Vec<eulumdat::ValidationError>| {
        let messages: Vec<String> = errors
            .iter()
            .map(|e| format!("[{}] {}", e.code, e.message))
            .collect();
        EulumdatError::ValidationError(messages.join("; "))
    })
}

/// Get detailed validation errors (for UI display)
#[uniffi::export]
pub fn get_validation_errors(ldt: &Eulumdat) -> Vec<ValidationError> {
    let core_ldt = to_core_eulumdat(ldt);
    match eulumdat::validate_strict(&core_ldt) {
        Ok(()) => vec![],
        Err(errors) => errors
            .iter()
            .map(|e: &eulumdat::ValidationError| e.into())
            .collect(),
    }
}
