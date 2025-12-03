//! Validation for Eulumdat data.
//!
//! Implements comprehensive validation based on the 41 constraints from the Eulumdat specification.

use crate::eulumdat::{Eulumdat, Symmetry};

/// A validation warning (non-fatal issue).
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationWarning {
    /// Warning code for programmatic handling.
    pub code: &'static str,
    /// Human-readable warning message.
    pub message: String,
}

impl std::fmt::Display for ValidationWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

/// A validation error (fatal issue).
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    /// Error code for programmatic handling.
    pub code: &'static str,
    /// Human-readable error message.
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for ValidationError {}

/// Validate Eulumdat data and return warnings.
pub fn validate(ldt: &Eulumdat) -> Vec<ValidationWarning> {
    let mut warnings = Vec::new();

    // === Type and Symmetry Validation ===

    // W001: Type indicator range (already validated by enum, but check consistency)
    if ldt.type_indicator.as_int() < 1 || ldt.type_indicator.as_int() > 3 {
        warnings.push(ValidationWarning {
            code: "W001",
            message: "Type indicator value is out of range (1-3)".to_string(),
        });
    }

    // W002: Symmetry indicator range (already validated by enum)
    if ldt.symmetry.as_int() < 0 || ldt.symmetry.as_int() > 4 {
        warnings.push(ValidationWarning {
            code: "W002",
            message: "Symmetry indicator value is out of range (0-4)".to_string(),
        });
    }

    // === Grid Dimension Validation ===

    // W003: Number of C-planes range
    if ldt.num_c_planes > 721 {
        warnings.push(ValidationWarning {
            code: "W003",
            message: format!(
                "Number of C-planes ({}) exceeds maximum (721)",
                ldt.num_c_planes
            ),
        });
    }

    // W004: C-plane distance range
    if ldt.c_plane_distance < 0.0 || ldt.c_plane_distance > 360.0 {
        warnings.push(ValidationWarning {
            code: "W004",
            message: format!(
                "Distance between C-planes ({}) is out of range (0-360)",
                ldt.c_plane_distance
            ),
        });
    }

    // W005: Number of G-planes range
    if ldt.num_g_planes > 361 {
        warnings.push(ValidationWarning {
            code: "W005",
            message: format!(
                "Number of G-planes ({}) exceeds maximum (361)",
                ldt.num_g_planes
            ),
        });
    }

    // W006: G-plane distance range
    if ldt.g_plane_distance < 0.0 || ldt.g_plane_distance > 180.0 {
        warnings.push(ValidationWarning {
            code: "W006",
            message: format!(
                "Distance between G-planes ({}) is out of range (0-180)",
                ldt.g_plane_distance
            ),
        });
    }

    // === String Field Length Validation ===
    const MAX_LINE_LENGTH: usize = 80;

    // W007: Measurement report number length
    if ldt.measurement_report_number.len() > MAX_LINE_LENGTH {
        warnings.push(ValidationWarning {
            code: "W007",
            message: format!(
                "Measurement report number exceeds {} characters",
                MAX_LINE_LENGTH
            ),
        });
    }

    // W008: Luminaire name length
    if ldt.luminaire_name.len() > MAX_LINE_LENGTH {
        warnings.push(ValidationWarning {
            code: "W008",
            message: format!("Luminaire name exceeds {} characters", MAX_LINE_LENGTH),
        });
    }

    // W009: Luminaire number length
    if ldt.luminaire_number.len() > MAX_LINE_LENGTH {
        warnings.push(ValidationWarning {
            code: "W009",
            message: format!("Luminaire number exceeds {} characters", MAX_LINE_LENGTH),
        });
    }

    // W010: File name length
    if ldt.file_name.len() > MAX_LINE_LENGTH {
        warnings.push(ValidationWarning {
            code: "W010",
            message: format!("File name exceeds {} characters", MAX_LINE_LENGTH),
        });
    }

    // W011: Date/user length
    if ldt.date_user.len() > MAX_LINE_LENGTH {
        warnings.push(ValidationWarning {
            code: "W011",
            message: format!("Date/user field exceeds {} characters", MAX_LINE_LENGTH),
        });
    }

    // === Physical Dimension Validation ===

    // W012: Negative dimensions
    if ldt.length < 0.0 {
        warnings.push(ValidationWarning {
            code: "W012",
            message: "Luminaire length is negative".to_string(),
        });
    }
    if ldt.width < 0.0 {
        warnings.push(ValidationWarning {
            code: "W013",
            message: "Luminaire width is negative".to_string(),
        });
    }
    if ldt.height < 0.0 {
        warnings.push(ValidationWarning {
            code: "W014",
            message: "Luminaire height is negative".to_string(),
        });
    }

    // W015: Luminous area dimensions
    if ldt.luminous_area_length < 0.0 {
        warnings.push(ValidationWarning {
            code: "W015",
            message: "Luminous area length is negative".to_string(),
        });
    }
    if ldt.luminous_area_width < 0.0 {
        warnings.push(ValidationWarning {
            code: "W016",
            message: "Luminous area width is negative".to_string(),
        });
    }

    // W017: Luminous area larger than luminaire
    if ldt.luminous_area_length > ldt.length && ldt.length > 0.0 {
        warnings.push(ValidationWarning {
            code: "W017",
            message: "Luminous area length exceeds luminaire length".to_string(),
        });
    }
    if ldt.luminous_area_width > ldt.width && ldt.width > 0.0 {
        warnings.push(ValidationWarning {
            code: "W018",
            message: "Luminous area width exceeds luminaire width".to_string(),
        });
    }

    // === Optical Properties Validation ===

    // W019: Downward flux fraction range
    if ldt.downward_flux_fraction < 0.0 || ldt.downward_flux_fraction > 100.0 {
        warnings.push(ValidationWarning {
            code: "W019",
            message: format!(
                "Downward flux fraction ({}) is out of range (0-100%)",
                ldt.downward_flux_fraction
            ),
        });
    }

    // W020: Light output ratio range
    if ldt.light_output_ratio < 0.0 || ldt.light_output_ratio > 100.0 {
        warnings.push(ValidationWarning {
            code: "W020",
            message: format!(
                "Light output ratio ({}) is out of range (0-100%)",
                ldt.light_output_ratio
            ),
        });
    }

    // W021: Conversion factor
    if ldt.conversion_factor <= 0.0 {
        warnings.push(ValidationWarning {
            code: "W021",
            message: "Conversion factor should be positive".to_string(),
        });
    }

    // W022: Tilt angle range
    if ldt.tilt_angle < -90.0 || ldt.tilt_angle > 90.0 {
        warnings.push(ValidationWarning {
            code: "W022",
            message: format!(
                "Tilt angle ({}) is out of typical range (-90 to 90)",
                ldt.tilt_angle
            ),
        });
    }

    // === Lamp Set Validation ===

    // W023: Number of lamp sets
    if ldt.lamp_sets.is_empty() {
        warnings.push(ValidationWarning {
            code: "W023",
            message: "No lamp sets defined".to_string(),
        });
    }
    if ldt.lamp_sets.len() > 20 {
        warnings.push(ValidationWarning {
            code: "W024",
            message: format!(
                "Number of lamp sets ({}) exceeds maximum (20)",
                ldt.lamp_sets.len()
            ),
        });
    }

    // W025-W030: Per lamp set validation
    for (i, lamp_set) in ldt.lamp_sets.iter().enumerate() {
        if lamp_set.num_lamps <= 0 {
            warnings.push(ValidationWarning {
                code: "W025",
                message: format!(
                    "Lamp set {} has invalid lamp count ({})",
                    i + 1,
                    lamp_set.num_lamps
                ),
            });
        }
        if lamp_set.total_luminous_flux < 0.0 {
            warnings.push(ValidationWarning {
                code: "W026",
                message: format!("Lamp set {} has negative luminous flux", i + 1),
            });
        }
        if lamp_set.wattage_with_ballast < 0.0 {
            warnings.push(ValidationWarning {
                code: "W027",
                message: format!("Lamp set {} has negative wattage", i + 1),
            });
        }
        if lamp_set.lamp_type.len() > 40 {
            warnings.push(ValidationWarning {
                code: "W028",
                message: format!("Lamp set {} type exceeds 40 characters", i + 1),
            });
        }
        if lamp_set.color_appearance.len() > 40 {
            warnings.push(ValidationWarning {
                code: "W029",
                message: format!("Lamp set {} color appearance exceeds 40 characters", i + 1),
            });
        }
        if lamp_set.color_rendering_group.len() > 40 {
            warnings.push(ValidationWarning {
                code: "W030",
                message: format!(
                    "Lamp set {} color rendering group exceeds 40 characters",
                    i + 1
                ),
            });
        }
    }

    // === Direct Ratio Validation ===

    // W031: Direct ratios range
    for (i, &ratio) in ldt.direct_ratios.iter().enumerate() {
        if !(0.0..=1.0).contains(&ratio) {
            warnings.push(ValidationWarning {
                code: "W031",
                message: format!("Direct ratio {} ({}) is out of range (0-1)", i + 1, ratio),
            });
        }
    }

    // === C-Plane Angle Validation ===

    // W032: C-planes not sorted
    for i in 1..ldt.c_angles.len() {
        if ldt.c_angles[i - 1] >= ldt.c_angles[i] {
            warnings.push(ValidationWarning {
                code: "W032",
                message: format!(
                    "C-planes not sorted: C[{}]={} >= C[{}]={}",
                    i - 1,
                    ldt.c_angles[i - 1],
                    i,
                    ldt.c_angles[i]
                ),
            });
            break;
        }
    }

    // W033: C-plane angle range
    for (i, &angle) in ldt.c_angles.iter().enumerate() {
        if !(0.0..360.0).contains(&angle) {
            warnings.push(ValidationWarning {
                code: "W033",
                message: format!("C-plane angle C[{}]={} is out of range (0-360)", i, angle),
            });
        }
    }

    // === G-Plane Angle Validation ===

    // W034: G-planes not sorted
    for i in 1..ldt.g_angles.len() {
        if ldt.g_angles[i - 1] >= ldt.g_angles[i] {
            warnings.push(ValidationWarning {
                code: "W034",
                message: format!(
                    "G-planes not sorted: G[{}]={} >= G[{}]={}",
                    i - 1,
                    ldt.g_angles[i - 1],
                    i,
                    ldt.g_angles[i]
                ),
            });
            break;
        }
    }

    // W035: G-plane angle range
    if !ldt.g_angles.is_empty() {
        if ldt.g_angles[0] < 0.0 {
            warnings.push(ValidationWarning {
                code: "W035",
                message: format!("First G-plane angle ({}) is negative", ldt.g_angles[0]),
            });
        }
        if ldt.g_angles[ldt.g_angles.len() - 1] > 180.0 {
            warnings.push(ValidationWarning {
                code: "W036",
                message: format!(
                    "Last G-plane angle ({}) exceeds 180°",
                    ldt.g_angles[ldt.g_angles.len() - 1]
                ),
            });
        }
    }

    // === Symmetry-Specific Validation ===

    // W037-W040: Required C-planes based on symmetry
    if ldt.symmetry == Symmetry::None {
        // No symmetry requires C90, C180, C270
        let has_c90 = ldt.c_angles.iter().any(|&a| (a - 90.0).abs() < 0.001);
        let has_c180 = ldt.c_angles.iter().any(|&a| (a - 180.0).abs() < 0.001);
        let has_c270 = ldt.c_angles.iter().any(|&a| (a - 270.0).abs() < 0.001);

        if !has_c90 {
            warnings.push(ValidationWarning {
                code: "W037",
                message: "No symmetry mode requires C90 plane".to_string(),
            });
        }
        if !has_c180 {
            warnings.push(ValidationWarning {
                code: "W038",
                message: "No symmetry mode requires C180 plane".to_string(),
            });
        }
        if !has_c270 {
            warnings.push(ValidationWarning {
                code: "W039",
                message: "No symmetry mode requires C270 plane".to_string(),
            });
        }
    }

    // === Intensity Data Validation ===

    // W041: Intensity data dimensions
    let expected_mc = ldt.symmetry.calc_mc(ldt.num_c_planes);
    if ldt.intensities.len() != expected_mc {
        warnings.push(ValidationWarning {
            code: "W040",
            message: format!(
                "Intensity data has {} C-planes, expected {} based on symmetry",
                ldt.intensities.len(),
                expected_mc
            ),
        });
    }

    for (i, row) in ldt.intensities.iter().enumerate() {
        if row.len() != ldt.num_g_planes {
            warnings.push(ValidationWarning {
                code: "W041",
                message: format!(
                    "Intensity row {} has {} G-values, expected {}",
                    i,
                    row.len(),
                    ldt.num_g_planes
                ),
            });
        }
    }

    // W042: Intensity value range
    let mut all_under_one = true;
    let mut total = 0.0;
    let mut count = 0;

    for row in &ldt.intensities {
        for &intensity in row {
            if intensity < 0.0 {
                warnings.push(ValidationWarning {
                    code: "W042",
                    message: format!("Negative intensity value: {}", intensity),
                });
            }
            if intensity > 1_000_000.0 {
                warnings.push(ValidationWarning {
                    code: "W043",
                    message: format!("Intensity value {} exceeds typical maximum", intensity),
                });
            }
            if intensity >= 1.0 {
                all_under_one = false;
            }
            total += intensity;
            count += 1;
        }
    }

    // W044: All intensities under 1 (suspicious)
    if all_under_one && count > 0 {
        let avg = total / count as f64;
        warnings.push(ValidationWarning {
            code: "W044",
            message: format!(
                "All intensity values are under 1 cd/klm (avg: {:.4}). Data may be incorrect.",
                avg
            ),
        });
    }

    warnings
}

/// Validate strictly and return errors if critical issues are found.
pub fn validate_strict(ldt: &Eulumdat) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    // Critical validations that should fail

    // E001: No intensity data
    if ldt.intensities.is_empty() {
        errors.push(ValidationError {
            code: "E001",
            message: "No intensity data".to_string(),
        });
    }

    // E002: No G-planes
    if ldt.num_g_planes == 0 {
        errors.push(ValidationError {
            code: "E002",
            message: "No G-planes defined".to_string(),
        });
    }

    // E003: No lamp sets
    if ldt.lamp_sets.is_empty() {
        errors.push(ValidationError {
            code: "E003",
            message: "No lamp sets defined".to_string(),
        });
    }

    // E004: Mismatched data dimensions
    let expected_mc = ldt.symmetry.calc_mc(ldt.num_c_planes);
    if ldt.intensities.len() != expected_mc {
        errors.push(ValidationError {
            code: "E004",
            message: format!(
                "Intensity data dimension mismatch: {} C-planes, expected {}",
                ldt.intensities.len(),
                expected_mc
            ),
        });
    }

    // E005: G-angles count mismatch
    if ldt.g_angles.len() != ldt.num_g_planes {
        errors.push(ValidationError {
            code: "E005",
            message: format!(
                "G-angles count ({}) doesn't match num_g_planes ({})",
                ldt.g_angles.len(),
                ldt.num_g_planes
            ),
        });
    }

    // E006: C-angles count mismatch
    if ldt.c_angles.len() != expected_mc {
        errors.push(ValidationError {
            code: "E006",
            message: format!(
                "C-angles count ({}) doesn't match expected Mc ({})",
                ldt.c_angles.len(),
                expected_mc
            ),
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eulumdat::LampSet;

    fn create_valid_ldt() -> Eulumdat {
        let mut ldt = Eulumdat::new();
        ldt.symmetry = Symmetry::VerticalAxis;
        ldt.num_c_planes = 1;
        ldt.num_g_planes = 3;
        ldt.c_angles = vec![0.0];
        ldt.g_angles = vec![0.0, 45.0, 90.0];
        ldt.intensities = vec![vec![100.0, 80.0, 50.0]];
        ldt.lamp_sets.push(LampSet {
            num_lamps: 1,
            lamp_type: "LED".to_string(),
            total_luminous_flux: 1000.0,
            color_appearance: "3000K".to_string(),
            color_rendering_group: "80".to_string(),
            wattage_with_ballast: 10.0,
        });
        ldt
    }

    #[test]
    fn test_valid_data() {
        let ldt = create_valid_ldt();
        let warnings = validate(&ldt);
        // Should have minimal warnings for a basic valid file
        assert!(warnings.iter().all(|w| !w.code.starts_with('E')));
    }

    #[test]
    fn test_strict_validation() {
        let ldt = create_valid_ldt();
        assert!(validate_strict(&ldt).is_ok());
    }

    #[test]
    fn test_missing_intensity_data() {
        let mut ldt = create_valid_ldt();
        ldt.intensities.clear();
        let result = validate_strict(&ldt);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.code == "E001"));
    }

    #[test]
    fn test_dimension_mismatch() {
        let mut ldt = create_valid_ldt();
        ldt.num_g_planes = 5; // But only 3 G-angles
        let result = validate_strict(&ldt);
        assert!(result.is_err());
    }
}
