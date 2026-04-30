//! Schema validation FFI types and functions (ATLA S001, TM-33-23, TM-32-24)

use crate::types::{to_core_eulumdat, Eulumdat};

/// A validation message from schema validation
#[derive(Debug, Clone, uniffi::Record)]
pub struct SchemaValidationMessage {
    pub code: String,
    pub message: String,
}

impl From<&eulumdat::atla::validate::ValidationMessage> for SchemaValidationMessage {
    fn from(m: &eulumdat::atla::validate::ValidationMessage) -> Self {
        Self {
            code: m.code.clone(),
            message: m.message.clone(),
        }
    }
}

/// Result of schema validation
#[derive(Debug, Clone, uniffi::Record)]
pub struct SchemaValidationResult {
    pub is_valid: bool,
    pub errors: Vec<SchemaValidationMessage>,
    pub warnings: Vec<SchemaValidationMessage>,
}

impl From<eulumdat::atla::validate::ValidationResult> for SchemaValidationResult {
    fn from(r: eulumdat::atla::validate::ValidationResult) -> Self {
        Self {
            is_valid: r.errors.is_empty(),
            errors: r.errors.iter().map(|e| e.into()).collect(),
            warnings: r.warnings.iter().map(|w| w.into()).collect(),
        }
    }
}

/// Validate against ATLA S001 schema
#[uniffi::export]
pub fn validate_schema_s001(ldt: &Eulumdat) -> SchemaValidationResult {
    let core_ldt = to_core_eulumdat(ldt);
    let doc = eulumdat::atla::LuminaireOpticalData::from_eulumdat(&core_ldt);
    let result = eulumdat::atla::validate::validate_with_schema(
        &doc,
        eulumdat::atla::validate::ValidationSchema::AtlaS001,
    );
    result.into()
}

/// Validate against TM-33-23 schema
#[uniffi::export]
pub fn validate_schema_tm33(ldt: &Eulumdat) -> SchemaValidationResult {
    let core_ldt = to_core_eulumdat(ldt);
    let doc = eulumdat::atla::LuminaireOpticalData::from_eulumdat(&core_ldt);
    let result = eulumdat::atla::validate::validate_with_schema(
        &doc,
        eulumdat::atla::validate::ValidationSchema::Tm3323,
    );
    result.into()
}

/// Validate against TM-32-24 schema
#[uniffi::export]
pub fn validate_schema_tm32(ldt: &Eulumdat) -> SchemaValidationResult {
    let core_ldt = to_core_eulumdat(ldt);
    let doc = eulumdat::atla::LuminaireOpticalData::from_eulumdat(&core_ldt);
    let result = eulumdat::atla::validate::validate_with_schema(
        &doc,
        eulumdat::atla::validate::ValidationSchema::Tm3224,
    );
    result.into()
}
