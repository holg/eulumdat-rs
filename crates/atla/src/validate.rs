//! XSD validation for ATLA XML documents
//!
//! Provides validation against the ATLA S001 / TM-33-18 / UNI 11733 and TM-33-23 XML schemas.
//! Uses `xmllint` when available for full XSD validation.
//!
//! # Schema Support
//!
//! This module supports two schema versions:
//! - **ATLA S001 / TM-33-18**: Original schema with `<LuminaireOpticalData>` root
//! - **TM-33-23 (IESTM33-22)**: Updated schema with `<IESTM33-22>` root
//!
//! The validator automatically detects the schema version and applies appropriate rules.

use crate::error::{AtlaError, Result};
use crate::types::*;
use std::path::Path;
use std::process::Command;

/// Embedded XSD schema for ATLA S001 / TM-33-18 / UNI 11733
pub const ATLA_XSD_SCHEMA: &str = include_str!("../../../docs/atla-s001.xsd");

/// Schema type for validation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ValidationSchema {
    /// Auto-detect schema from document
    #[default]
    Auto,
    /// Validate against ATLA S001 / TM-33-18 schema
    AtlaS001,
    /// Validate against TM-33-23 (IESTM33-22) schema
    Tm3323,
    /// Validate against TM-32-24 BIM requirements (includes TM-33-23)
    Tm3224,
}

/// Validation result containing errors and warnings
#[derive(Debug, Clone, Default)]
pub struct ValidationResult {
    /// Validation errors (schema violations)
    pub errors: Vec<ValidationMessage>,
    /// Validation warnings (non-critical issues)
    pub warnings: Vec<ValidationMessage>,
}

impl ValidationResult {
    /// Returns true if validation passed with no errors
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Returns true if there are any issues (errors or warnings)
    pub fn has_issues(&self) -> bool {
        !self.errors.is_empty() || !self.warnings.is_empty()
    }
}

/// A validation message with code and description
#[derive(Debug, Clone)]
pub struct ValidationMessage {
    /// Error/warning code
    pub code: String,
    /// Human-readable message
    pub message: String,
    /// Optional line number
    pub line: Option<usize>,
    /// Optional column number
    pub column: Option<usize>,
}

impl std::fmt::Display for ValidationMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let (Some(line), Some(col)) = (self.line, self.column) {
            write!(f, "[{}] {}:{}: {}", self.code, line, col, self.message)
        } else if let Some(line) = self.line {
            write!(f, "[{}] line {}: {}", self.code, line, self.message)
        } else {
            write!(f, "[{}] {}", self.code, self.message)
        }
    }
}

/// Validate an ATLA document structure (without XSD)
///
/// Performs basic structural validation:
/// - Required elements present
/// - Intensity data consistency
/// - Value ranges
///
/// This function auto-detects the schema version and applies appropriate rules.
/// For explicit schema validation, use [`validate_with_schema`].
pub fn validate(doc: &LuminaireOpticalData) -> ValidationResult {
    validate_with_schema(doc, ValidationSchema::Auto)
}

/// Validate an ATLA document against a specific schema version
///
/// # Arguments
/// * `doc` - The document to validate
/// * `schema` - The schema to validate against (Auto, AtlaS001, or Tm3323)
///
/// # Schema-specific rules
///
/// ## Common rules (both schemas)
/// - E001: Missing version
/// - E002: No emitters
/// - E003: Negative lumens
/// - E004: Negative watts
/// - E005: CRI out of range
/// - E006-E008: Intensity array issues
/// - W001: Negative dimensions
/// - W002: Zero quantity
/// - W003: CCT out of typical range
///
/// ## TM-33-23 additional rules
/// - TM33-E001: Header.Description required
/// - TM33-E002: Header.Laboratory required
/// - TM33-E003: Header.ReportNumber required
/// - TM33-E004: Header.ReportDate required
/// - TM33-E010: Emitter.Description required
/// - TM33-E011: Emitter.InputWatts required
/// - TM33-E020: CustomData.Name required
/// - TM33-E021: CustomData.UniqueIdentifier required
/// - TM33-W001: Multiplier should be positive
/// - TM33-W002: SymmetryType inconsistent with data
///
/// ## TM-32-24 BIM additional rules (includes all TM-33-23 rules)
/// - TM32-E001: Header.Manufacturer required for BIM
/// - TM32-E002: Header.CatalogNumber required for BIM
/// - TM32-W001: Missing recommended BIM parameters
/// - TM32-W002: CCT should be specified for BIM
/// - TM32-W003: CRI should be specified for BIM
pub fn validate_with_schema(
    doc: &LuminaireOpticalData,
    schema: ValidationSchema,
) -> ValidationResult {
    let mut result = ValidationResult::default();

    // Determine effective schema
    let effective_schema = match schema {
        ValidationSchema::Auto => doc.schema_version,
        ValidationSchema::AtlaS001 => SchemaVersion::AtlaS001,
        ValidationSchema::Tm3323 => SchemaVersion::Tm3323,
        ValidationSchema::Tm3224 => SchemaVersion::Tm3324,
    };

    // Common validation for all schemas
    validate_common(doc, &mut result);

    // Schema-specific validation
    match effective_schema {
        SchemaVersion::Tm3323 => {
            validate_tm33_23(doc, &mut result);
        }
        SchemaVersion::Tm3324 => {
            // TM-32-24 includes all TM-33-23 rules plus BIM-specific rules
            validate_tm33_23(doc, &mut result);
            validate_tm32_24(doc, &mut result);
        }
        SchemaVersion::AtlaS001 => {
            // ATLA S001 has fewer required fields - just common validation
        }
    }

    result
}

/// Common validation rules for all schema versions
fn validate_common(doc: &LuminaireOpticalData, result: &mut ValidationResult) {
    // Check version
    if doc.version.is_empty() {
        result.errors.push(ValidationMessage {
            code: "E001".to_string(),
            message: "Missing required 'version' attribute".to_string(),
            line: None,
            column: None,
        });
    }

    // Check for at least one emitter
    if doc.emitters.is_empty() {
        result.errors.push(ValidationMessage {
            code: "E002".to_string(),
            message: "At least one Emitter element is required".to_string(),
            line: None,
            column: None,
        });
    }

    // Validate each emitter
    for (i, emitter) in doc.emitters.iter().enumerate() {
        validate_emitter(emitter, i, result);
    }

    // Validate luminaire dimensions if present
    if let Some(ref luminaire) = doc.luminaire {
        if let Some(ref dims) = luminaire.dimensions {
            if dims.length < 0.0 || dims.width < 0.0 || dims.height < 0.0 {
                result.warnings.push(ValidationMessage {
                    code: "W001".to_string(),
                    message: "Luminaire dimensions should be non-negative".to_string(),
                    line: None,
                    column: None,
                });
            }
        }
    }
}

/// TM-33-23 specific validation rules
fn validate_tm33_23(doc: &LuminaireOpticalData, result: &mut ValidationResult) {
    // Header required fields
    if doc.header.description.is_none()
        || doc.header.description.as_ref().is_none_or(|s| s.is_empty())
    {
        result.errors.push(ValidationMessage {
            code: "TM33-E001".to_string(),
            message: "TM-33-23: Header.Description is required".to_string(),
            line: None,
            column: None,
        });
    }

    if doc.header.laboratory.is_none()
        || doc.header.laboratory.as_ref().is_none_or(|s| s.is_empty())
    {
        result.errors.push(ValidationMessage {
            code: "TM33-E002".to_string(),
            message: "TM-33-23: Header.Laboratory is required".to_string(),
            line: None,
            column: None,
        });
    }

    if doc.header.report_number.is_none()
        || doc
            .header
            .report_number
            .as_ref()
            .is_none_or(|s| s.is_empty())
    {
        result.errors.push(ValidationMessage {
            code: "TM33-E003".to_string(),
            message: "TM-33-23: Header.ReportNumber is required".to_string(),
            line: None,
            column: None,
        });
    }

    if doc.header.report_date.is_none()
        || doc.header.report_date.as_ref().is_none_or(|s| s.is_empty())
    {
        result.errors.push(ValidationMessage {
            code: "TM33-E004".to_string(),
            message: "TM-33-23: Header.ReportDate is required (format: YYYY-MM-DD)".to_string(),
            line: None,
            column: None,
        });
    } else if let Some(ref date) = doc.header.report_date {
        // Validate date format (YYYY-MM-DD)
        if !is_valid_date_format(date) {
            result.warnings.push(ValidationMessage {
                code: "TM33-W003".to_string(),
                message: format!(
                    "TM-33-23: ReportDate '{}' should be in YYYY-MM-DD format",
                    date
                ),
                line: None,
                column: None,
            });
        }
    }

    // Validate emitters for TM-33-23 requirements
    for (i, emitter) in doc.emitters.iter().enumerate() {
        validate_emitter_tm33_23(emitter, i, result);
    }

    // Validate CustomData items (TM-33-23 requires Name and UniqueIdentifier)
    for (i, item) in doc.custom_data_items.iter().enumerate() {
        validate_custom_data_item(item, i, result);
    }
}

/// TM-32-24 BIM-specific validation rules
///
/// TM-32-24 is the ANSI/IES standard for BIM (Building Information Modeling)
/// parameters for lighting equipment. It requires all TM-33-23 fields plus
/// additional BIM-specific requirements.
fn validate_tm32_24(doc: &LuminaireOpticalData, result: &mut ValidationResult) {
    use crate::bim::BimParameters;

    // TM-32-24 requires Manufacturer for BIM integration
    if doc.header.manufacturer.is_none()
        || doc
            .header
            .manufacturer
            .as_ref()
            .is_none_or(|s| s.is_empty())
    {
        result.errors.push(ValidationMessage {
            code: "TM32-E001".to_string(),
            message: "TM-32-24: Header.Manufacturer is required for BIM integration".to_string(),
            line: None,
            column: None,
        });
    }

    // TM-32-24 requires CatalogNumber for product identification
    if doc.header.catalog_number.is_none()
        || doc
            .header
            .catalog_number
            .as_ref()
            .is_none_or(|s| s.is_empty())
    {
        result.errors.push(ValidationMessage {
            code: "TM32-E002".to_string(),
            message: "TM-32-24: Header.CatalogNumber is required for BIM integration".to_string(),
            line: None,
            column: None,
        });
    }

    // Extract BIM parameters and check for recommended fields
    let bim = BimParameters::from_atla(doc);
    let populated = bim.populated_count();

    // TM-32-24 recommends having at least basic photometric BIM parameters
    if populated < 5 {
        result.warnings.push(ValidationMessage {
            code: "TM32-W001".to_string(),
            message: format!(
                "TM-32-24: Only {} BIM parameters populated. Consider adding more for complete BIM integration",
                populated
            ),
            line: None,
            column: None,
        });
    }

    // Check for recommended color parameters
    let has_cct = doc.emitters.iter().any(|e| e.cct.is_some());
    if !has_cct && bim.cct_kelvin.is_none() {
        result.warnings.push(ValidationMessage {
            code: "TM32-W002".to_string(),
            message: "TM-32-24: CCT (Color Temperature) should be specified for BIM integration"
                .to_string(),
            line: None,
            column: None,
        });
    }

    // Check for CRI
    let has_cri = doc
        .emitters
        .iter()
        .any(|e| e.color_rendering.as_ref().is_some_and(|cr| cr.ra.is_some()));
    if !has_cri && bim.cri.is_none() {
        result.warnings.push(ValidationMessage {
            code: "TM32-W003".to_string(),
            message:
                "TM-32-24: CRI (Color Rendering Index) should be specified for BIM integration"
                    .to_string(),
            line: None,
            column: None,
        });
    }

    // Check for wattage (critical for energy modeling)
    let has_watts = doc.emitters.iter().any(|e| e.input_watts.is_some());
    if !has_watts && bim.watts.is_none() {
        result.warnings.push(ValidationMessage {
            code: "TM32-W004".to_string(),
            message: "TM-32-24: Input wattage should be specified for energy modeling".to_string(),
            line: None,
            column: None,
        });
    }

    // Check for lumens (critical for lighting calculations)
    let has_lumens = doc.emitters.iter().any(|e| e.rated_lumens.is_some());
    if !has_lumens && bim.total_luminous_flux.is_none() {
        result.warnings.push(ValidationMessage {
            code: "TM32-W005".to_string(),
            message:
                "TM-32-24: Luminous flux (lumens) should be specified for lighting calculations"
                    .to_string(),
            line: None,
            column: None,
        });
    }

    // TM-32-24 requires physical dimensions for BIM integration
    match doc.luminaire.as_ref().and_then(|l| l.dimensions.as_ref()) {
        None => {
            result.errors.push(ValidationMessage {
                code: "TM32-E003".to_string(),
                message: "TM-32-24: Luminaire dimensions are required for BIM integration"
                    .to_string(),
                line: None,
                column: None,
            });
        }
        Some(dims) => {
            if dims.length == 0.0 || dims.width == 0.0 || dims.height == 0.0 {
                let mut zero_fields = Vec::new();
                if dims.length == 0.0 {
                    zero_fields.push("length");
                }
                if dims.width == 0.0 {
                    zero_fields.push("width");
                }
                if dims.height == 0.0 {
                    zero_fields.push("height");
                }
                result.errors.push(ValidationMessage {
                    code: "TM32-E004".to_string(),
                    message: format!(
                        "TM-32-24: Luminaire {} must be non-zero for BIM integration",
                        zero_fields.join(", ")
                    ),
                    line: None,
                    column: None,
                });
            }
        }
    }
}

/// TM-33-23 specific emitter validation
fn validate_emitter_tm33_23(emitter: &Emitter, index: usize, result: &mut ValidationResult) {
    let prefix = format!("Emitter[{}]", index);

    // Description is required in TM-33-23
    if emitter.description.is_none() || emitter.description.as_ref().is_none_or(|s| s.is_empty()) {
        result.errors.push(ValidationMessage {
            code: "TM33-E010".to_string(),
            message: format!("TM-33-23: {}.Description is required", prefix),
            line: None,
            column: None,
        });
    }

    // InputWatts (InputWattage) is required in TM-33-23
    if emitter.input_watts.is_none() {
        result.errors.push(ValidationMessage {
            code: "TM33-E011".to_string(),
            message: format!("TM-33-23: {}.InputWattage is required", prefix),
            line: None,
            column: None,
        });
    }

    // Validate intensity distribution TM-33-23 specific fields
    if let Some(ref dist) = emitter.intensity_distribution {
        validate_intensity_tm33_23(dist, &prefix, result);
    }

    // Validate angular spectral data if present
    if let Some(ref angular_spectral) = emitter.angular_spectral {
        validate_angular_spectral(angular_spectral, &prefix, result);
    }

    // Validate angular color data if present
    if let Some(ref angular_color) = emitter.angular_color {
        validate_angular_color(angular_color, &prefix, result);
    }
}

/// TM-33-23 intensity distribution validation
fn validate_intensity_tm33_23(
    dist: &IntensityDistribution,
    prefix: &str,
    result: &mut ValidationResult,
) {
    // Multiplier should be positive if specified
    if let Some(multiplier) = dist.multiplier {
        if multiplier <= 0.0 {
            result.warnings.push(ValidationMessage {
                code: "TM33-W001".to_string(),
                message: format!(
                    "{}: Multiplier should be positive (got {})",
                    prefix, multiplier
                ),
                line: None,
                column: None,
            });
        }
    }

    // Validate symmetry type consistency
    if let Some(ref symmetry) = dist.symmetry {
        validate_symmetry_consistency(symmetry, dist, prefix, result);
    }
}

/// Validate symmetry type is consistent with actual data
fn validate_symmetry_consistency(
    symmetry: &SymmetryType,
    dist: &IntensityDistribution,
    prefix: &str,
    result: &mut ValidationResult,
) {
    let h_count = dist.horizontal_angles.len();

    match symmetry {
        SymmetryType::Full => {
            // Full symmetry should have only 1 horizontal angle
            if h_count > 1 {
                result.warnings.push(ValidationMessage {
                    code: "TM33-W002".to_string(),
                    message: format!(
                        "{}: SymmetryType is Full but {} horizontal angles provided (expected 1)",
                        prefix, h_count
                    ),
                    line: None,
                    column: None,
                });
            }
        }
        SymmetryType::Bi0 | SymmetryType::Bi90 => {
            // Bilateral symmetry should have angles 0-180
            if let Some(&max_h) = dist
                .horizontal_angles
                .iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
            {
                if max_h > 180.0 {
                    result.warnings.push(ValidationMessage {
                        code: "TM33-W002".to_string(),
                        message: format!(
                            "{}: SymmetryType is {:?} but horizontal angles exceed 180° (max: {})",
                            prefix, symmetry, max_h
                        ),
                        line: None,
                        column: None,
                    });
                }
            }
        }
        SymmetryType::Quad => {
            // Quadrilateral symmetry should have angles 0-90
            if let Some(&max_h) = dist
                .horizontal_angles
                .iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
            {
                if max_h > 90.0 {
                    result.warnings.push(ValidationMessage {
                        code: "TM33-W002".to_string(),
                        message: format!(
                            "{}: SymmetryType is Quad but horizontal angles exceed 90° (max: {})",
                            prefix, max_h
                        ),
                        line: None,
                        column: None,
                    });
                }
            }
        }
        SymmetryType::None | SymmetryType::Arbitrary => {
            // No constraints
        }
    }
}

/// Validate angular spectral data
fn validate_angular_spectral(
    data: &AngularSpectralData,
    prefix: &str,
    result: &mut ValidationResult,
) {
    if data.data_points.is_empty() {
        result.warnings.push(ValidationMessage {
            code: "TM33-W004".to_string(),
            message: format!("{}: AngularSpectralData has no data points", prefix),
            line: None,
            column: None,
        });
    }

    // Check multiplier
    if let Some(multiplier) = data.multiplier {
        if multiplier <= 0.0 {
            result.warnings.push(ValidationMessage {
                code: "TM33-W005".to_string(),
                message: format!(
                    "{}: AngularSpectralData.Multiplier should be positive",
                    prefix
                ),
                line: None,
                column: None,
            });
        }
    }

    // Check wavelength ranges (typically 380-780nm for visible)
    for (i, point) in data.data_points.iter().enumerate() {
        if point.w < 100.0 || point.w > 2000.0 {
            result.warnings.push(ValidationMessage {
                code: "TM33-W006".to_string(),
                message: format!(
                    "{}: AngularSpectralData point {} has unusual wavelength {} nm",
                    prefix, i, point.w
                ),
                line: None,
                column: None,
            });
        }
    }
}

/// Validate angular color data
fn validate_angular_color(data: &AngularColorData, prefix: &str, result: &mut ValidationResult) {
    if data.data_points.is_empty() {
        result.warnings.push(ValidationMessage {
            code: "TM33-W007".to_string(),
            message: format!("{}: AngularColorData has no data points", prefix),
            line: None,
            column: None,
        });
    }

    // Check CIE x,y values are in valid range (0-1)
    for (i, point) in data.data_points.iter().enumerate() {
        if point.x < 0.0 || point.x > 1.0 {
            result.errors.push(ValidationMessage {
                code: "TM33-E030".to_string(),
                message: format!(
                    "{}: AngularColorData point {} has invalid CIE x={} (must be 0-1)",
                    prefix, i, point.x
                ),
                line: None,
                column: None,
            });
        }
        if point.y < 0.0 || point.y > 1.0 {
            result.errors.push(ValidationMessage {
                code: "TM33-E031".to_string(),
                message: format!(
                    "{}: AngularColorData point {} has invalid CIE y={} (must be 0-1)",
                    prefix, i, point.y
                ),
                line: None,
                column: None,
            });
        }
        // Check that x + y doesn't exceed practical limits
        if point.x + point.y > 1.0 {
            result.warnings.push(ValidationMessage {
                code: "TM33-W008".to_string(),
                message: format!(
                    "{}: AngularColorData point {} has x+y={} which exceeds typical chromaticity bounds",
                    prefix, i, point.x + point.y
                ),
                line: None,
                column: None,
            });
        }
    }
}

/// Validate CustomData item (TM-33-23)
fn validate_custom_data_item(item: &CustomDataItem, index: usize, result: &mut ValidationResult) {
    let prefix = format!("CustomData[{}]", index);

    if item.name.is_empty() {
        result.errors.push(ValidationMessage {
            code: "TM33-E020".to_string(),
            message: format!("TM-33-23: {}.Name is required", prefix),
            line: None,
            column: None,
        });
    }

    if item.unique_identifier.is_empty() {
        result.errors.push(ValidationMessage {
            code: "TM33-E021".to_string(),
            message: format!("TM-33-23: {}.UniqueIdentifier is required", prefix),
            line: None,
            column: None,
        });
    }
}

/// Check if a string is in valid date format (YYYY-MM-DD)
fn is_valid_date_format(date: &str) -> bool {
    if date.len() != 10 {
        return false;
    }
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return false;
    }
    // Year should be 4 digits, month and day 2 digits
    parts[0].len() == 4
        && parts[1].len() == 2
        && parts[2].len() == 2
        && parts[0].chars().all(|c| c.is_ascii_digit())
        && parts[1].chars().all(|c| c.is_ascii_digit())
        && parts[2].chars().all(|c| c.is_ascii_digit())
}

/// Validate an emitter element
fn validate_emitter(emitter: &Emitter, index: usize, result: &mut ValidationResult) {
    let prefix = format!("Emitter[{}]", index);

    // Check quantity
    if emitter.quantity == 0 {
        result.warnings.push(ValidationMessage {
            code: "W002".to_string(),
            message: format!("{}: Quantity is 0, should be at least 1", prefix),
            line: None,
            column: None,
        });
    }

    // Check lumens
    if let Some(lumens) = emitter.rated_lumens {
        if lumens < 0.0 {
            result.errors.push(ValidationMessage {
                code: "E003".to_string(),
                message: format!("{}: RatedLumens cannot be negative", prefix),
                line: None,
                column: None,
            });
        }
    }

    // Check watts
    if let Some(watts) = emitter.input_watts {
        if watts < 0.0 {
            result.errors.push(ValidationMessage {
                code: "E004".to_string(),
                message: format!("{}: InputWatts cannot be negative", prefix),
                line: None,
                column: None,
            });
        }
    }

    // Check CCT range
    if let Some(cct) = emitter.cct {
        if !(1000.0..=20000.0).contains(&cct) {
            result.warnings.push(ValidationMessage {
                code: "W003".to_string(),
                message: format!(
                    "{}: CCT {} is outside typical range (1000-20000K)",
                    prefix, cct
                ),
                line: None,
                column: None,
            });
        }
    }

    // Check CRI range
    if let Some(ref cr) = emitter.color_rendering {
        if let Some(ra) = cr.ra {
            if !(0.0..=100.0).contains(&ra) {
                result.errors.push(ValidationMessage {
                    code: "E005".to_string(),
                    message: format!("{}: Ra must be between 0 and 100", prefix),
                    line: None,
                    column: None,
                });
            }
        }
    }

    // Validate intensity distribution
    if let Some(ref dist) = emitter.intensity_distribution {
        validate_intensity_distribution(dist, &prefix, result);
    }
}

/// Validate intensity distribution
fn validate_intensity_distribution(
    dist: &IntensityDistribution,
    prefix: &str,
    result: &mut ValidationResult,
) {
    // Check angle arrays
    if dist.horizontal_angles.is_empty() {
        result.warnings.push(ValidationMessage {
            code: "W004".to_string(),
            message: format!("{}: IntensityDistribution has no horizontal angles", prefix),
            line: None,
            column: None,
        });
    }

    if dist.vertical_angles.is_empty() {
        result.warnings.push(ValidationMessage {
            code: "W005".to_string(),
            message: format!("{}: IntensityDistribution has no vertical angles", prefix),
            line: None,
            column: None,
        });
    }

    // Check intensity array dimensions
    let expected_h = dist.horizontal_angles.len();
    let expected_v = dist.vertical_angles.len();

    if dist.intensities.len() != expected_h {
        result.errors.push(ValidationMessage {
            code: "E006".to_string(),
            message: format!(
                "{}: Intensity array has {} horizontal planes, expected {}",
                prefix,
                dist.intensities.len(),
                expected_h
            ),
            line: None,
            column: None,
        });
    }

    for (i, plane) in dist.intensities.iter().enumerate() {
        if plane.len() != expected_v {
            result.errors.push(ValidationMessage {
                code: "E007".to_string(),
                message: format!(
                    "{}: Intensity plane {} has {} values, expected {}",
                    prefix,
                    i,
                    plane.len(),
                    expected_v
                ),
                line: None,
                column: None,
            });
        }
    }

    // Check for negative intensities
    for (i, plane) in dist.intensities.iter().enumerate() {
        for (j, &value) in plane.iter().enumerate() {
            if value < 0.0 {
                result.errors.push(ValidationMessage {
                    code: "E008".to_string(),
                    message: format!("{}: Negative intensity {} at [{},{}]", prefix, value, i, j),
                    line: None,
                    column: None,
                });
            }
        }
    }
}

/// Validate XML string against XSD schema using xmllint
///
/// Returns Ok(ValidationResult) if xmllint ran successfully (even if validation failed).
/// Returns Err if xmllint is not available or failed to run.
pub fn validate_xsd(xml: &str) -> Result<ValidationResult> {
    validate_xsd_with_schema(xml, ATLA_XSD_SCHEMA)
}

/// Validate XML string against a custom XSD schema using xmllint
pub fn validate_xsd_with_schema(xml: &str, xsd: &str) -> Result<ValidationResult> {
    // Create temporary files
    let temp_dir = std::env::temp_dir();
    let xml_path = temp_dir.join("atla_validate_temp.xml");
    let xsd_path = temp_dir.join("atla_validate_temp.xsd");

    std::fs::write(&xml_path, xml)?;
    std::fs::write(&xsd_path, xsd)?;

    let result = validate_xsd_files(&xml_path, &xsd_path);

    // Clean up temp files
    let _ = std::fs::remove_file(&xml_path);
    let _ = std::fs::remove_file(&xsd_path);

    result
}

/// Validate XML file against XSD schema file using xmllint
pub fn validate_xsd_files(xml_path: &Path, xsd_path: &Path) -> Result<ValidationResult> {
    let output = Command::new("xmllint")
        .args([
            "--noout",
            "--schema",
            xsd_path.to_str().unwrap_or(""),
            xml_path.to_str().unwrap_or(""),
        ])
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AtlaError::XmlParse(
                    "xmllint not found. Install libxml2 for XSD validation.".to_string(),
                )
            } else {
                AtlaError::Io(e)
            }
        })?;

    let mut result = ValidationResult::default();

    // Parse xmllint output
    let stderr = String::from_utf8_lossy(&output.stderr);
    for line in stderr.lines() {
        if line.contains(" validates") {
            // Success message
            continue;
        }
        if line.contains(" fails to validate") || line.contains("error") {
            // Parse error line format: "file.xml:line: element X: Schemas validity error"
            let msg = parse_xmllint_error(line);
            result.errors.push(msg);
        } else if line.contains("warning") {
            let msg = parse_xmllint_error(line);
            result.warnings.push(msg);
        }
    }

    Ok(result)
}

/// Parse an xmllint error message
fn parse_xmllint_error(line: &str) -> ValidationMessage {
    // Try to parse "file:line:col: message" format
    let parts: Vec<&str> = line.splitn(4, ':').collect();

    let (line_num, col_num, message) = if parts.len() >= 3 {
        let line_no = parts[1].trim().parse().ok();
        let col = parts.get(2).and_then(|s| s.trim().parse().ok());
        let msg = parts.get(3).map(|s| s.trim()).unwrap_or(line);
        (line_no, col, msg.to_string())
    } else {
        (None, None, line.to_string())
    };

    ValidationMessage {
        code: "XSD".to_string(),
        message,
        line: line_num,
        column: col_num,
    }
}

/// Check if xmllint is available for XSD validation
pub fn is_xmllint_available() -> bool {
    Command::new("xmllint")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get the embedded XSD schema path (writes to temp file if needed)
pub fn get_schema_path() -> Result<std::path::PathBuf> {
    let schema_dir = std::env::temp_dir().join("atla");
    std::fs::create_dir_all(&schema_dir)?;
    let schema_path = schema_dir.join("atla-s001.xsd");
    std::fs::write(&schema_path, ATLA_XSD_SCHEMA)?;
    Ok(schema_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===========================================
    // Common validation tests (ATLA S001)
    // ===========================================

    #[test]
    fn test_validate_empty_doc() {
        let doc = LuminaireOpticalData::new();
        let result = validate(&doc);
        assert!(!result.is_valid()); // Should fail - no emitters
    }

    #[test]
    fn test_validate_minimal_valid() {
        let mut doc = LuminaireOpticalData::new();
        doc.emitters.push(Emitter {
            quantity: 1,
            ..Default::default()
        });
        let result = validate(&doc);
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_negative_lumens() {
        let mut doc = LuminaireOpticalData::new();
        doc.emitters.push(Emitter {
            quantity: 1,
            rated_lumens: Some(-100.0),
            ..Default::default()
        });
        let result = validate(&doc);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "E003"));
    }

    #[test]
    fn test_validate_intensity_mismatch() {
        let mut doc = LuminaireOpticalData::new();
        doc.emitters.push(Emitter {
            quantity: 1,
            intensity_distribution: Some(IntensityDistribution {
                horizontal_angles: vec![0.0, 90.0],
                vertical_angles: vec![0.0, 45.0, 90.0],
                intensities: vec![vec![100.0, 80.0, 60.0]], // Only 1 plane, should be 2
                ..Default::default()
            }),
            ..Default::default()
        });
        let result = validate(&doc);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "E006"));
    }

    #[test]
    fn test_xmllint_available() {
        // Just check if the function runs without panic
        let _ = is_xmllint_available();
    }

    // ===========================================
    // TM-33-23 specific validation tests
    // ===========================================

    #[test]
    fn test_tm33_23_requires_header_description() {
        let mut doc = LuminaireOpticalData::new();
        doc.schema_version = SchemaVersion::Tm3323;
        doc.emitters.push(Emitter {
            quantity: 1,
            description: Some("Test Emitter".to_string()),
            input_watts: Some(50.0),
            ..Default::default()
        });
        doc.header.laboratory = Some("Test Lab".to_string());
        doc.header.report_number = Some("RPT-001".to_string());
        doc.header.report_date = Some("2024-01-15".to_string());
        // Missing: description

        let result = validate_with_schema(&doc, ValidationSchema::Tm3323);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "TM33-E001"));
    }

    #[test]
    fn test_tm33_23_requires_laboratory() {
        let mut doc = LuminaireOpticalData::new();
        doc.schema_version = SchemaVersion::Tm3323;
        doc.emitters.push(Emitter {
            quantity: 1,
            description: Some("Test Emitter".to_string()),
            input_watts: Some(50.0),
            ..Default::default()
        });
        doc.header.description = Some("Test Description".to_string());
        doc.header.report_number = Some("RPT-001".to_string());
        doc.header.report_date = Some("2024-01-15".to_string());
        // Missing: laboratory

        let result = validate_with_schema(&doc, ValidationSchema::Tm3323);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "TM33-E002"));
    }

    #[test]
    fn test_tm33_23_requires_report_number() {
        let mut doc = LuminaireOpticalData::new();
        doc.schema_version = SchemaVersion::Tm3323;
        doc.emitters.push(Emitter {
            quantity: 1,
            description: Some("Test Emitter".to_string()),
            input_watts: Some(50.0),
            ..Default::default()
        });
        doc.header.description = Some("Test Description".to_string());
        doc.header.laboratory = Some("Test Lab".to_string());
        doc.header.report_date = Some("2024-01-15".to_string());
        // Missing: report_number

        let result = validate_with_schema(&doc, ValidationSchema::Tm3323);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "TM33-E003"));
    }

    #[test]
    fn test_tm33_23_requires_report_date() {
        let mut doc = LuminaireOpticalData::new();
        doc.schema_version = SchemaVersion::Tm3323;
        doc.emitters.push(Emitter {
            quantity: 1,
            description: Some("Test Emitter".to_string()),
            input_watts: Some(50.0),
            ..Default::default()
        });
        doc.header.description = Some("Test Description".to_string());
        doc.header.laboratory = Some("Test Lab".to_string());
        doc.header.report_number = Some("RPT-001".to_string());
        // Missing: report_date

        let result = validate_with_schema(&doc, ValidationSchema::Tm3323);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "TM33-E004"));
    }

    #[test]
    fn test_tm33_23_requires_emitter_description() {
        let mut doc = create_tm33_23_valid_doc();
        doc.emitters[0].description = None; // Remove description

        let result = validate_with_schema(&doc, ValidationSchema::Tm3323);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "TM33-E010"));
    }

    #[test]
    fn test_tm33_23_requires_input_wattage() {
        let mut doc = create_tm33_23_valid_doc();
        doc.emitters[0].input_watts = None; // Remove input watts

        let result = validate_with_schema(&doc, ValidationSchema::Tm3323);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "TM33-E011"));
    }

    #[test]
    fn test_tm33_23_valid_document() {
        let doc = create_tm33_23_valid_doc();
        let result = validate_with_schema(&doc, ValidationSchema::Tm3323);
        assert!(result.is_valid(), "Errors: {:?}", result.errors);
    }

    #[test]
    fn test_tm33_23_custom_data_requires_name() {
        let mut doc = create_tm33_23_valid_doc();
        doc.custom_data_items.push(CustomDataItem {
            name: "".to_string(), // Empty name
            unique_identifier: "urn:example:custom".to_string(),
            raw_content: "<data>test</data>".to_string(),
        });

        let result = validate_with_schema(&doc, ValidationSchema::Tm3323);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "TM33-E020"));
    }

    #[test]
    fn test_tm33_23_custom_data_requires_unique_identifier() {
        let mut doc = create_tm33_23_valid_doc();
        doc.custom_data_items.push(CustomDataItem {
            name: "TestData".to_string(),
            unique_identifier: "".to_string(), // Empty identifier
            raw_content: "<data>test</data>".to_string(),
        });

        let result = validate_with_schema(&doc, ValidationSchema::Tm3323);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "TM33-E021"));
    }

    #[test]
    fn test_tm33_23_multiplier_warning() {
        let mut doc = create_tm33_23_valid_doc();
        doc.emitters[0].intensity_distribution = Some(IntensityDistribution {
            horizontal_angles: vec![0.0],
            vertical_angles: vec![0.0, 90.0, 180.0],
            intensities: vec![vec![100.0, 80.0, 60.0]],
            multiplier: Some(-1.0), // Invalid negative multiplier
            ..Default::default()
        });

        let result = validate_with_schema(&doc, ValidationSchema::Tm3323);
        assert!(result.warnings.iter().any(|w| w.code == "TM33-W001"));
    }

    #[test]
    fn test_tm33_23_symmetry_full_consistency() {
        let mut doc = create_tm33_23_valid_doc();
        doc.emitters[0].intensity_distribution = Some(IntensityDistribution {
            horizontal_angles: vec![0.0, 90.0], // Full symmetry should have 1 angle
            vertical_angles: vec![0.0, 90.0, 180.0],
            intensities: vec![vec![100.0, 80.0, 60.0], vec![100.0, 80.0, 60.0]],
            symmetry: Some(SymmetryType::Full),
            ..Default::default()
        });

        let result = validate_with_schema(&doc, ValidationSchema::Tm3323);
        assert!(result.warnings.iter().any(|w| w.code == "TM33-W002"));
    }

    #[test]
    fn test_tm33_23_angular_color_invalid_cie_x() {
        let mut doc = create_tm33_23_valid_doc();
        doc.emitters[0].angular_color = Some(AngularColorData {
            symmetry: None,
            multiplier: None,
            number_measured: 1,
            number_horz: 1,
            number_vert: 1,
            data_points: vec![AngularColorPoint {
                h: 0.0,
                v: 0.0,
                x: 1.5, // Invalid: > 1.0
                y: 0.3,
            }],
        });

        let result = validate_with_schema(&doc, ValidationSchema::Tm3323);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "TM33-E030"));
    }

    #[test]
    fn test_tm33_23_angular_color_invalid_cie_y() {
        let mut doc = create_tm33_23_valid_doc();
        doc.emitters[0].angular_color = Some(AngularColorData {
            symmetry: None,
            multiplier: None,
            number_measured: 1,
            number_horz: 1,
            number_vert: 1,
            data_points: vec![AngularColorPoint {
                h: 0.0,
                v: 0.0,
                x: 0.3,
                y: -0.1, // Invalid: < 0
            }],
        });

        let result = validate_with_schema(&doc, ValidationSchema::Tm3323);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "TM33-E031"));
    }

    #[test]
    fn test_tm33_23_date_format_warning() {
        let mut doc = create_tm33_23_valid_doc();
        doc.header.report_date = Some("15-01-2024".to_string()); // Wrong format

        let result = validate_with_schema(&doc, ValidationSchema::Tm3323);
        assert!(result.warnings.iter().any(|w| w.code == "TM33-W003"));
    }

    #[test]
    fn test_s001_not_enforcing_tm33_requirements() {
        // Create a doc that would fail TM-33-23 validation
        let mut doc = LuminaireOpticalData::new();
        doc.schema_version = SchemaVersion::AtlaS001;
        doc.emitters.push(Emitter {
            quantity: 1,
            // No description, no input_watts - these are optional in S001
            ..Default::default()
        });
        // No header fields - these are optional in S001

        let result = validate_with_schema(&doc, ValidationSchema::AtlaS001);
        // S001 should pass - these fields are optional
        assert!(result.is_valid());

        // Same doc should fail TM-33-23
        let result_tm33 = validate_with_schema(&doc, ValidationSchema::Tm3323);
        assert!(!result_tm33.is_valid());
    }

    #[test]
    fn test_auto_detect_uses_doc_schema_version() {
        // Create TM-33-23 doc with missing required fields
        let mut doc = LuminaireOpticalData::new();
        doc.schema_version = SchemaVersion::Tm3323;
        doc.emitters.push(Emitter {
            quantity: 1,
            ..Default::default()
        });

        // Auto should detect TM-33-23 and apply stricter rules
        let result = validate_with_schema(&doc, ValidationSchema::Auto);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code.starts_with("TM33-")));
    }

    #[test]
    fn test_date_format_validation() {
        // Valid formats
        assert!(is_valid_date_format("2024-01-15"));
        assert!(is_valid_date_format("2024-12-31"));

        // Invalid formats
        assert!(!is_valid_date_format("15-01-2024")); // DD-MM-YYYY
        assert!(!is_valid_date_format("2024/01/15")); // Wrong separator
        assert!(!is_valid_date_format("24-01-15")); // Short year
        assert!(!is_valid_date_format("2024-1-15")); // Single digit month
        assert!(!is_valid_date_format("Jan 15, 2024")); // Text format
    }

    // ===========================================
    // Helper functions
    // ===========================================

    /// Create a minimal valid TM-33-23 document for testing
    fn create_tm33_23_valid_doc() -> LuminaireOpticalData {
        let mut doc = LuminaireOpticalData::new();
        doc.schema_version = SchemaVersion::Tm3323;
        doc.header.description = Some("Test Luminaire".to_string());
        doc.header.laboratory = Some("Test Laboratory".to_string());
        doc.header.report_number = Some("RPT-2024-001".to_string());
        doc.header.report_date = Some("2024-01-15".to_string());
        doc.emitters.push(Emitter {
            quantity: 1,
            description: Some("LED Module".to_string()),
            input_watts: Some(50.0),
            ..Default::default()
        });
        doc
    }

    /// Create a minimal valid TM-32-24 document for testing
    fn create_tm32_24_valid_doc() -> LuminaireOpticalData {
        let mut doc = create_tm33_23_valid_doc();
        doc.schema_version = SchemaVersion::Tm3324;
        doc.header.manufacturer = Some("Test Manufacturer".to_string());
        doc.header.catalog_number = Some("CAT-001".to_string());
        // Add CCT and CRI for BIM compliance
        doc.emitters[0].cct = Some(4000.0);
        doc.emitters[0].color_rendering = Some(ColorRendering {
            ra: Some(90.0),
            r9: None,
            rf: None,
            rg: None,
        });
        doc.emitters[0].rated_lumens = Some(3000.0);
        // BIM requires physical dimensions
        doc.luminaire = Some(Luminaire {
            dimensions: Some(Dimensions {
                length: 600.0,
                width: 300.0,
                height: 80.0,
            }),
            ..Default::default()
        });
        doc
    }

    // ===========================================
    // TM-32-24 BIM validation tests
    // ===========================================

    #[test]
    fn test_tm32_24_requires_manufacturer() {
        let mut doc = create_tm32_24_valid_doc();
        doc.header.manufacturer = None;

        let result = validate_with_schema(&doc, ValidationSchema::Tm3224);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "TM32-E001"));
    }

    #[test]
    fn test_tm32_24_requires_catalog_number() {
        let mut doc = create_tm32_24_valid_doc();
        doc.header.catalog_number = None;

        let result = validate_with_schema(&doc, ValidationSchema::Tm3224);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "TM32-E002"));
    }

    #[test]
    fn test_tm32_24_valid_document() {
        let doc = create_tm32_24_valid_doc();
        let result = validate_with_schema(&doc, ValidationSchema::Tm3224);
        assert!(result.is_valid(), "Errors: {:?}", result.errors);
    }

    #[test]
    fn test_tm32_24_includes_tm33_23_rules() {
        // TM-32-24 should still enforce TM-33-23 rules
        let mut doc = LuminaireOpticalData::new();
        doc.schema_version = SchemaVersion::Tm3324;
        doc.header.manufacturer = Some("Test".to_string());
        doc.header.catalog_number = Some("CAT-001".to_string());
        // Missing TM-33-23 required fields: description, laboratory, report_number, report_date
        doc.emitters.push(Emitter {
            quantity: 1,
            ..Default::default()
        });

        let result = validate_with_schema(&doc, ValidationSchema::Tm3224);
        assert!(!result.is_valid());
        // Should have TM-33-23 errors
        assert!(result.errors.iter().any(|e| e.code.starts_with("TM33-")));
    }

    #[test]
    fn test_tm32_24_warns_missing_cct() {
        let mut doc = create_tm32_24_valid_doc();
        doc.emitters[0].cct = None;

        let result = validate_with_schema(&doc, ValidationSchema::Tm3224);
        // Should still be valid but with warning
        assert!(result.is_valid());
        assert!(result.warnings.iter().any(|w| w.code == "TM32-W002"));
    }

    #[test]
    fn test_tm32_24_warns_missing_cri() {
        let mut doc = create_tm32_24_valid_doc();
        doc.emitters[0].color_rendering = None;

        let result = validate_with_schema(&doc, ValidationSchema::Tm3224);
        // Should still be valid but with warning
        assert!(result.is_valid());
        assert!(result.warnings.iter().any(|w| w.code == "TM32-W003"));
    }

    #[test]
    fn test_tm32_24_warns_missing_wattage() {
        let mut doc = create_tm32_24_valid_doc();
        doc.emitters[0].input_watts = None;

        let result = validate_with_schema(&doc, ValidationSchema::Tm3224);
        // This should fail due to TM-33-23 requirement for input_watts
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "TM33-E011"));
    }

    #[test]
    fn test_tm32_24_warns_missing_lumens() {
        let mut doc = create_tm32_24_valid_doc();
        doc.emitters[0].rated_lumens = None;

        let result = validate_with_schema(&doc, ValidationSchema::Tm3224);
        // Should still be valid but with warning
        assert!(result.is_valid());
        assert!(result.warnings.iter().any(|w| w.code == "TM32-W005"));
    }

    #[test]
    fn test_tm32_24_auto_detect() {
        let doc = create_tm32_24_valid_doc();
        // Auto-detect should apply TM-32-24 rules
        let result = validate_with_schema(&doc, ValidationSchema::Auto);
        assert!(result.is_valid());
    }

    #[test]
    fn test_tm32_24_requires_dimensions() {
        let mut doc = create_tm32_24_valid_doc();
        doc.luminaire = None;

        let result = validate_with_schema(&doc, ValidationSchema::Tm3224);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "TM32-E003"));
    }

    #[test]
    fn test_tm32_24_rejects_zero_dimensions() {
        let mut doc = create_tm32_24_valid_doc();
        doc.luminaire
            .as_mut()
            .unwrap()
            .dimensions
            .as_mut()
            .unwrap()
            .height = 0.0;

        let result = validate_with_schema(&doc, ValidationSchema::Tm3224);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "TM32-E004"));
    }
}
