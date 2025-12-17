//! XSD validation for ATLA XML documents
//!
//! Provides validation against the ATLA S001 / TM-33 / UNI 11733 XML schema.
//! Uses `xmllint` when available for full XSD validation.

use crate::error::{AtlaError, Result};
use crate::types::*;
use std::path::Path;
use std::process::Command;

/// Embedded XSD schema for ATLA S001 / TM-33 / UNI 11733
pub const ATLA_XSD_SCHEMA: &str = include_str!("../../../docs/atla-s001.xsd");

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
pub fn validate(doc: &LuminaireOpticalData) -> ValidationResult {
    let mut result = ValidationResult::default();

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
        validate_emitter(emitter, i, &mut result);
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

    result
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
}
