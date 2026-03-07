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

// ---------------------------------------------------------------------------
// English fallback message builder
// ---------------------------------------------------------------------------

fn english_message(code: &str, args: &[&dyn std::fmt::Display]) -> String {
    match code {
        "W001" => "Type indicator value is out of range (1-3)".to_string(),
        "W002" => "Symmetry indicator value is out of range (0-4)".to_string(),
        "W003" => format!("Number of C-planes ({}) exceeds maximum (721)", args[0]),
        "W004" => format!(
            "Distance between C-planes ({}) is out of range (0-360)",
            args[0]
        ),
        "W005" => format!("Number of G-planes ({}) exceeds maximum (361)", args[0]),
        "W006" => format!(
            "Distance between G-planes ({}) is out of range (0-180)",
            args[0]
        ),
        "W007" => format!("Measurement report number exceeds {} characters", args[0]),
        "W008" => format!("Luminaire name exceeds {} characters", args[0]),
        "W009" => format!("Luminaire number exceeds {} characters", args[0]),
        "W010" => format!("File name exceeds {} characters", args[0]),
        "W011" => format!("Date/user field exceeds {} characters", args[0]),
        "W012" => "Luminaire length is negative".to_string(),
        "W013" => "Luminaire width is negative".to_string(),
        "W014" => "Luminaire height is negative".to_string(),
        "W015" => "Luminous area length is negative".to_string(),
        "W016" => "Luminous area width is negative".to_string(),
        "W017" => "Luminous area length exceeds luminaire length".to_string(),
        "W018" => "Luminous area width exceeds luminaire width".to_string(),
        "W019" => format!(
            "Downward flux fraction ({}) is out of range (0-100%)",
            args[0]
        ),
        "W020" => format!("Light output ratio ({}) is out of range (0-100%)", args[0]),
        "W021" => "Conversion factor should be positive".to_string(),
        "W022" => format!(
            "Tilt angle ({}) is out of typical range (-90 to 90)",
            args[0]
        ),
        "W023" => "No lamp sets defined".to_string(),
        "W024" => format!("Number of lamp sets ({}) exceeds maximum (20)", args[0]),
        "W025" => format!("Lamp set {} has invalid lamp count ({})", args[0], args[1]),
        "W026" => format!("Lamp set {} has negative luminous flux", args[0]),
        "W027" => format!("Lamp set {} has negative wattage", args[0]),
        "W028" => format!("Lamp set {} type exceeds 40 characters", args[0]),
        "W029" => format!(
            "Lamp set {} color appearance exceeds 40 characters",
            args[0]
        ),
        "W030" => format!(
            "Lamp set {} color rendering group exceeds 40 characters",
            args[0]
        ),
        "W031" => format!(
            "Direct ratio {} ({}) is out of range (0-1)",
            args[0], args[1]
        ),
        "W032" => format!(
            "C-planes not sorted: C[{}]={} >= C[{}]={}",
            args[0], args[1], args[2], args[3]
        ),
        "W033" => format!(
            "C-plane angle C[{}]={} is out of range (0-360)",
            args[0], args[1]
        ),
        "W034" => format!(
            "G-planes not sorted: G[{}]={} >= G[{}]={}",
            args[0], args[1], args[2], args[3]
        ),
        "W035" => format!("First G-plane angle ({}) is negative", args[0]),
        "W036" => format!("Last G-plane angle ({}) exceeds 180°", args[0]),
        "W037" => "No symmetry mode requires C90 plane".to_string(),
        "W038" => "No symmetry mode requires C180 plane".to_string(),
        "W039" => "No symmetry mode requires C270 plane".to_string(),
        "W040" => format!(
            "Intensity data has {} C-planes, expected {} based on symmetry",
            args[0], args[1]
        ),
        "W041" => format!(
            "Intensity row {} has {} G-values, expected {}",
            args[0], args[1], args[2]
        ),
        "W042" => format!("Negative intensity value: {}", args[0]),
        "W043" => format!("Intensity value {} exceeds typical maximum", args[0]),
        "W044" => format!(
            "All intensity values are under 1 cd/klm (avg: {}). Data may be incorrect.",
            args[0]
        ),
        "W045" => "Luminaire length is zero — no real luminaire has zero length".to_string(),
        "W046" => "Luminaire height is zero — no real luminaire has zero height".to_string(),
        "W047" => format!(
            "Declared symmetry Isym={} may not match actual intensity data (RMS difference: {}% between {} planes)",
            args[0], args[1], args[2]
        ),
        "E001" => "No intensity data".to_string(),
        "E002" => "No G-planes defined".to_string(),
        "E003" => "No lamp sets defined".to_string(),
        "E004" => format!(
            "Intensity data dimension mismatch: {} C-planes, expected {}",
            args[0], args[1]
        ),
        "E005" => format!(
            "G-angles count ({}) doesn't match num_g_planes ({})",
            args[0], args[1]
        ),
        "E006" => format!(
            "C-angles count ({}) doesn't match expected Mc ({})",
            args[0], args[1]
        ),
        _ => format!("[{}]", code),
    }
}

// ---------------------------------------------------------------------------
// Symmetry correctness helpers
// ---------------------------------------------------------------------------

/// Compute RMS percentage difference between two gamma-angle intensity vectors.
fn rms_percent_difference(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len().min(b.len());
    if n == 0 {
        return 0.0;
    }
    let mut sum_sq = 0.0;
    for i in 0..n {
        let max_val = a[i].abs().max(b[i].abs());
        if max_val > 0.0 {
            let diff_pct = (a[i] - b[i]).abs() / max_val * 100.0;
            sum_sq += diff_pct * diff_pct;
        }
    }
    (sum_sq / n as f64).sqrt()
}

/// Check whether declared symmetry matches the actual intensity data.
/// Returns `Some((rms_pct, plane_description))` when the data shows significant
/// asymmetry that contradicts the declared symmetry type.
fn check_symmetry_correctness(ldt: &Eulumdat) -> Option<(f64, String)> {
    match ldt.symmetry {
        Symmetry::BothPlanes => {
            // Isym=4: C0 and C90 should be identical (both-planes symmetry)
            // intensities[0] = C0, intensities[last] = C90
            if ldt.intensities.len() < 2 {
                return None;
            }
            let c0 = &ldt.intensities[0];
            let c90 = &ldt.intensities[ldt.intensities.len() - 1];
            let rms = rms_percent_difference(c0, c90);
            if rms > 15.0 {
                Some((rms, "C0 and C90".to_string()))
            } else {
                None
            }
        }
        Symmetry::PlaneC0C180 => {
            // Isym=2: mirror across C0-C180 plane
            // intensities[i] should equal intensities[n-1-i]
            let n = ldt.intensities.len();
            if n < 2 {
                return None;
            }
            let mut max_rms = 0.0f64;
            for i in 0..n / 2 {
                let rms = rms_percent_difference(&ldt.intensities[i], &ldt.intensities[n - 1 - i]);
                max_rms = max_rms.max(rms);
            }
            if max_rms > 15.0 {
                Some((max_rms, "C0-C180 mirror".to_string()))
            } else {
                None
            }
        }
        Symmetry::PlaneC90C270 => {
            // Isym=3: mirror across C90-C270 plane
            let n = ldt.intensities.len();
            if n < 2 {
                return None;
            }
            let mut max_rms = 0.0f64;
            for i in 0..n / 2 {
                let rms = rms_percent_difference(&ldt.intensities[i], &ldt.intensities[n - 1 - i]);
                max_rms = max_rms.max(rms);
            }
            if max_rms > 15.0 {
                Some((max_rms, "C90-C270 mirror".to_string()))
            } else {
                None
            }
        }
        // Isym=0 (no symmetry) or Isym=1 (vertical axis, single plane) — no check needed
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Internal validation logic (shared by English and i18n variants)
// ---------------------------------------------------------------------------

fn validate_inner(
    ldt: &Eulumdat,
    msg: &dyn Fn(&str, &[&dyn std::fmt::Display]) -> String,
) -> Vec<ValidationWarning> {
    let mut warnings = Vec::new();

    // === Type and Symmetry Validation ===

    if ldt.type_indicator.as_int() < 1 || ldt.type_indicator.as_int() > 3 {
        warnings.push(ValidationWarning {
            code: "W001",
            message: msg("W001", &[]),
        });
    }

    if ldt.symmetry.as_int() < 0 || ldt.symmetry.as_int() > 4 {
        warnings.push(ValidationWarning {
            code: "W002",
            message: msg("W002", &[]),
        });
    }

    // === Grid Dimension Validation ===

    if ldt.num_c_planes > 721 {
        warnings.push(ValidationWarning {
            code: "W003",
            message: msg("W003", &[&ldt.num_c_planes]),
        });
    }

    if ldt.c_plane_distance < 0.0 || ldt.c_plane_distance > 360.0 {
        warnings.push(ValidationWarning {
            code: "W004",
            message: msg("W004", &[&ldt.c_plane_distance]),
        });
    }

    if ldt.num_g_planes > 361 {
        warnings.push(ValidationWarning {
            code: "W005",
            message: msg("W005", &[&ldt.num_g_planes]),
        });
    }

    if ldt.g_plane_distance < 0.0 || ldt.g_plane_distance > 180.0 {
        warnings.push(ValidationWarning {
            code: "W006",
            message: msg("W006", &[&ldt.g_plane_distance]),
        });
    }

    // === String Field Length Validation ===
    const MAX_LINE_LENGTH: usize = 80;

    if ldt.measurement_report_number.len() > MAX_LINE_LENGTH {
        warnings.push(ValidationWarning {
            code: "W007",
            message: msg("W007", &[&MAX_LINE_LENGTH]),
        });
    }

    if ldt.luminaire_name.len() > MAX_LINE_LENGTH {
        warnings.push(ValidationWarning {
            code: "W008",
            message: msg("W008", &[&MAX_LINE_LENGTH]),
        });
    }

    if ldt.luminaire_number.len() > MAX_LINE_LENGTH {
        warnings.push(ValidationWarning {
            code: "W009",
            message: msg("W009", &[&MAX_LINE_LENGTH]),
        });
    }

    if ldt.file_name.len() > MAX_LINE_LENGTH {
        warnings.push(ValidationWarning {
            code: "W010",
            message: msg("W010", &[&MAX_LINE_LENGTH]),
        });
    }

    if ldt.date_user.len() > MAX_LINE_LENGTH {
        warnings.push(ValidationWarning {
            code: "W011",
            message: msg("W011", &[&MAX_LINE_LENGTH]),
        });
    }

    // === Physical Dimension Validation ===

    if ldt.length < 0.0 {
        warnings.push(ValidationWarning {
            code: "W012",
            message: msg("W012", &[]),
        });
    }
    if ldt.width < 0.0 {
        warnings.push(ValidationWarning {
            code: "W013",
            message: msg("W013", &[]),
        });
    }
    if ldt.height < 0.0 {
        warnings.push(ValidationWarning {
            code: "W014",
            message: msg("W014", &[]),
        });
    }

    if ldt.length == 0.0 {
        warnings.push(ValidationWarning {
            code: "W045",
            message: msg("W045", &[]),
        });
    }
    if ldt.height == 0.0 {
        warnings.push(ValidationWarning {
            code: "W046",
            message: msg("W046", &[]),
        });
    }

    if ldt.luminous_area_length < 0.0 {
        warnings.push(ValidationWarning {
            code: "W015",
            message: msg("W015", &[]),
        });
    }
    if ldt.luminous_area_width < 0.0 {
        warnings.push(ValidationWarning {
            code: "W016",
            message: msg("W016", &[]),
        });
    }

    if ldt.luminous_area_length > ldt.length && ldt.length > 0.0 {
        warnings.push(ValidationWarning {
            code: "W017",
            message: msg("W017", &[]),
        });
    }
    if ldt.luminous_area_width > ldt.width && ldt.width > 0.0 {
        warnings.push(ValidationWarning {
            code: "W018",
            message: msg("W018", &[]),
        });
    }

    // === Optical Properties Validation ===

    if ldt.downward_flux_fraction < 0.0 || ldt.downward_flux_fraction > 100.0 {
        warnings.push(ValidationWarning {
            code: "W019",
            message: msg("W019", &[&ldt.downward_flux_fraction]),
        });
    }

    if ldt.light_output_ratio < 0.0 || ldt.light_output_ratio > 100.0 {
        warnings.push(ValidationWarning {
            code: "W020",
            message: msg("W020", &[&ldt.light_output_ratio]),
        });
    }

    if ldt.conversion_factor <= 0.0 {
        warnings.push(ValidationWarning {
            code: "W021",
            message: msg("W021", &[]),
        });
    }

    if ldt.tilt_angle < -90.0 || ldt.tilt_angle > 90.0 {
        warnings.push(ValidationWarning {
            code: "W022",
            message: msg("W022", &[&ldt.tilt_angle]),
        });
    }

    // === Lamp Set Validation ===

    if ldt.lamp_sets.is_empty() {
        warnings.push(ValidationWarning {
            code: "W023",
            message: msg("W023", &[]),
        });
    }
    if ldt.lamp_sets.len() > 20 {
        warnings.push(ValidationWarning {
            code: "W024",
            message: msg("W024", &[&ldt.lamp_sets.len()]),
        });
    }

    for (i, lamp_set) in ldt.lamp_sets.iter().enumerate() {
        let idx = i + 1;
        if lamp_set.num_lamps <= 0 {
            warnings.push(ValidationWarning {
                code: "W025",
                message: msg("W025", &[&idx, &lamp_set.num_lamps]),
            });
        }
        if lamp_set.total_luminous_flux < 0.0 {
            warnings.push(ValidationWarning {
                code: "W026",
                message: msg("W026", &[&idx]),
            });
        }
        if lamp_set.wattage_with_ballast < 0.0 {
            warnings.push(ValidationWarning {
                code: "W027",
                message: msg("W027", &[&idx]),
            });
        }
        if lamp_set.lamp_type.len() > 40 {
            warnings.push(ValidationWarning {
                code: "W028",
                message: msg("W028", &[&idx]),
            });
        }
        if lamp_set.color_appearance.len() > 40 {
            warnings.push(ValidationWarning {
                code: "W029",
                message: msg("W029", &[&idx]),
            });
        }
        if lamp_set.color_rendering_group.len() > 40 {
            warnings.push(ValidationWarning {
                code: "W030",
                message: msg("W030", &[&idx]),
            });
        }
    }

    // === Direct Ratio Validation ===

    for (i, &ratio) in ldt.direct_ratios.iter().enumerate() {
        if !(0.0..=1.0).contains(&ratio) {
            warnings.push(ValidationWarning {
                code: "W031",
                message: msg("W031", &[&(i + 1), &ratio]),
            });
        }
    }

    // === C-Plane Angle Validation ===

    for i in 1..ldt.c_angles.len() {
        if ldt.c_angles[i - 1] >= ldt.c_angles[i] {
            warnings.push(ValidationWarning {
                code: "W032",
                message: msg(
                    "W032",
                    &[&(i - 1), &ldt.c_angles[i - 1], &i, &ldt.c_angles[i]],
                ),
            });
            break;
        }
    }

    for (i, &angle) in ldt.c_angles.iter().enumerate() {
        if !(0.0..=360.0).contains(&angle) {
            warnings.push(ValidationWarning {
                code: "W033",
                message: msg("W033", &[&i, &angle]),
            });
        }
    }

    // === G-Plane Angle Validation ===

    for i in 1..ldt.g_angles.len() {
        if ldt.g_angles[i - 1] >= ldt.g_angles[i] {
            warnings.push(ValidationWarning {
                code: "W034",
                message: msg(
                    "W034",
                    &[&(i - 1), &ldt.g_angles[i - 1], &i, &ldt.g_angles[i]],
                ),
            });
            break;
        }
    }

    if !ldt.g_angles.is_empty() {
        if ldt.g_angles[0] < 0.0 {
            warnings.push(ValidationWarning {
                code: "W035",
                message: msg("W035", &[&ldt.g_angles[0]]),
            });
        }
        if ldt.g_angles[ldt.g_angles.len() - 1] > 180.0 {
            warnings.push(ValidationWarning {
                code: "W036",
                message: msg("W036", &[&ldt.g_angles[ldt.g_angles.len() - 1]]),
            });
        }
    }

    // === Symmetry-Specific Validation ===

    if ldt.symmetry == Symmetry::None {
        let has_c90 = ldt.c_angles.iter().any(|&a| (a - 90.0).abs() < 0.001);
        let has_c180 = ldt.c_angles.iter().any(|&a| (a - 180.0).abs() < 0.001);
        let has_c270 = ldt.c_angles.iter().any(|&a| (a - 270.0).abs() < 0.001);

        if !has_c90 {
            warnings.push(ValidationWarning {
                code: "W037",
                message: msg("W037", &[]),
            });
        }
        if !has_c180 {
            warnings.push(ValidationWarning {
                code: "W038",
                message: msg("W038", &[]),
            });
        }
        if !has_c270 {
            warnings.push(ValidationWarning {
                code: "W039",
                message: msg("W039", &[]),
            });
        }
    }

    // === Intensity Data Validation ===

    let expected_mc = ldt.symmetry.calc_mc(ldt.num_c_planes);
    if ldt.intensities.len() != expected_mc {
        warnings.push(ValidationWarning {
            code: "W040",
            message: msg("W040", &[&ldt.intensities.len(), &expected_mc]),
        });
    }

    for (i, row) in ldt.intensities.iter().enumerate() {
        if row.len() != ldt.num_g_planes {
            warnings.push(ValidationWarning {
                code: "W041",
                message: msg("W041", &[&i, &row.len(), &ldt.num_g_planes]),
            });
        }
    }

    let mut all_under_one = true;
    let mut total = 0.0;
    let mut count = 0;

    for row in &ldt.intensities {
        for &intensity in row {
            if intensity < 0.0 {
                warnings.push(ValidationWarning {
                    code: "W042",
                    message: msg("W042", &[&intensity]),
                });
            }
            if intensity > 1_000_000.0 {
                warnings.push(ValidationWarning {
                    code: "W043",
                    message: msg("W043", &[&intensity]),
                });
            }
            if intensity >= 1.0 {
                all_under_one = false;
            }
            total += intensity;
            count += 1;
        }
    }

    if all_under_one && count > 0 {
        let avg = total / count as f64;
        let avg_str = format!("{:.4}", avg);
        warnings.push(ValidationWarning {
            code: "W044",
            message: msg("W044", &[&avg_str]),
        });
    }

    // === Symmetry Correctness Validation ===

    if let Some((rms, planes)) = check_symmetry_correctness(ldt) {
        let isym = ldt.symmetry.as_int();
        let rms_str = format!("{:.1}", rms);
        warnings.push(ValidationWarning {
            code: "W047",
            message: msg("W047", &[&isym, &rms_str, &planes]),
        });
    }

    warnings
}

fn validate_strict_inner(
    ldt: &Eulumdat,
    msg: &dyn Fn(&str, &[&dyn std::fmt::Display]) -> String,
) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    if ldt.intensities.is_empty() {
        errors.push(ValidationError {
            code: "E001",
            message: msg("E001", &[]),
        });
    }

    if ldt.num_g_planes == 0 {
        errors.push(ValidationError {
            code: "E002",
            message: msg("E002", &[]),
        });
    }

    if ldt.lamp_sets.is_empty() {
        errors.push(ValidationError {
            code: "E003",
            message: msg("E003", &[]),
        });
    }

    let expected_mc = ldt.symmetry.calc_mc(ldt.num_c_planes);
    if ldt.intensities.len() != expected_mc {
        errors.push(ValidationError {
            code: "E004",
            message: msg("E004", &[&ldt.intensities.len(), &expected_mc]),
        });
    }

    if ldt.g_angles.len() != ldt.num_g_planes {
        errors.push(ValidationError {
            code: "E005",
            message: msg("E005", &[&ldt.g_angles.len(), &ldt.num_g_planes]),
        });
    }

    if ldt.c_angles.len() != expected_mc {
        errors.push(ValidationError {
            code: "E006",
            message: msg("E006", &[&ldt.c_angles.len(), &expected_mc]),
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Validate Eulumdat data and return warnings (English messages).
pub fn validate(ldt: &Eulumdat) -> Vec<ValidationWarning> {
    validate_inner(ldt, &english_message)
}

/// Validate strictly and return errors if critical issues are found (English messages).
pub fn validate_strict(ldt: &Eulumdat) -> Result<(), Vec<ValidationError>> {
    validate_strict_inner(ldt, &english_message)
}

/// Validate with localized messages.
#[cfg(feature = "i18n")]
pub fn validate_with_locale(
    ldt: &Eulumdat,
    locale: &eulumdat_i18n::Locale,
) -> Vec<ValidationWarning> {
    validate_inner(ldt, &|code, args| {
        locale
            .validation_message(code)
            .map(|tpl| eulumdat_i18n::format_template(tpl, args))
            .unwrap_or_else(|| english_message(code, args))
    })
}

/// Validate strictly with localized messages.
#[cfg(feature = "i18n")]
pub fn validate_strict_with_locale(
    ldt: &Eulumdat,
    locale: &eulumdat_i18n::Locale,
) -> Result<(), Vec<ValidationError>> {
    validate_strict_inner(ldt, &|code, args| {
        locale
            .validation_message(code)
            .map(|tpl| eulumdat_i18n::format_template(tpl, args))
            .unwrap_or_else(|| english_message(code, args))
    })
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
        ldt.length = 600.0;
        ldt.width = 300.0;
        ldt.height = 80.0;
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

    #[test]
    fn test_zero_dimensions() {
        let mut ldt = create_valid_ldt();
        ldt.length = 0.0;
        ldt.height = 0.0;
        let warnings = validate(&ldt);
        assert!(warnings.iter().any(|w| w.code == "W045"));
        assert!(warnings.iter().any(|w| w.code == "W046"));
    }

    #[test]
    fn test_zero_width_is_valid_circular() {
        let mut ldt = create_valid_ldt();
        ldt.width = 0.0; // 0 = circular cross-section, valid in LDT
        let warnings = validate(&ldt);
        // Should NOT produce a zero-width warning
        assert!(!warnings
            .iter()
            .any(|w| w.code == "W045" || w.code == "W046"));
    }

    #[test]
    fn test_symmetry_correctness_isym4_matching() {
        // Isym=4 with C0 == C90 — no W047
        let mut ldt = Eulumdat::new();
        ldt.symmetry = Symmetry::BothPlanes;
        ldt.num_c_planes = 36;
        ldt.num_g_planes = 3;
        ldt.c_angles = vec![0.0, 45.0, 90.0]; // Mc = 36/4+1 but we use 3 for test
        ldt.g_angles = vec![0.0, 45.0, 90.0];
        // C0 and C90 are identical
        ldt.intensities = vec![
            vec![100.0, 80.0, 50.0], // C0
            vec![95.0, 75.0, 45.0],  // C45
            vec![100.0, 80.0, 50.0], // C90 (same as C0)
        ];
        ldt.length = 200.0;
        ldt.height = 50.0;
        ldt.lamp_sets.push(LampSet {
            num_lamps: 1,
            lamp_type: "LED".to_string(),
            total_luminous_flux: 1000.0,
            color_appearance: "3000K".to_string(),
            color_rendering_group: "80".to_string(),
            wattage_with_ballast: 10.0,
        });
        let warnings = validate(&ldt);
        assert!(
            !warnings.iter().any(|w| w.code == "W047"),
            "Isym=4 with matching C0/C90 should not produce W047"
        );
    }

    #[test]
    fn test_symmetry_correctness_isym4_mismatched() {
        // Isym=4 with C0 ≠ C90 (2x difference) — should produce W047
        let mut ldt = Eulumdat::new();
        ldt.symmetry = Symmetry::BothPlanes;
        ldt.num_c_planes = 36;
        ldt.num_g_planes = 3;
        ldt.c_angles = vec![0.0, 45.0, 90.0];
        ldt.g_angles = vec![0.0, 45.0, 90.0];
        // C0 and C90 differ by 2x — clearly asymmetric
        ldt.intensities = vec![
            vec![100.0, 80.0, 50.0],   // C0
            vec![95.0, 75.0, 45.0],    // C45
            vec![200.0, 160.0, 100.0], // C90 (2x of C0!)
        ];
        ldt.length = 200.0;
        ldt.height = 50.0;
        ldt.lamp_sets.push(LampSet {
            num_lamps: 1,
            lamp_type: "LED".to_string(),
            total_luminous_flux: 1000.0,
            color_appearance: "3000K".to_string(),
            color_rendering_group: "80".to_string(),
            wattage_with_ballast: 10.0,
        });
        let warnings = validate(&ldt);
        assert!(
            warnings.iter().any(|w| w.code == "W047"),
            "Isym=4 with mismatched C0/C90 should produce W047"
        );
    }

    #[test]
    fn test_symmetry_correctness_isym0_no_check() {
        // Isym=0 (no symmetry) — no W047 regardless of data
        let mut ldt = Eulumdat::new();
        ldt.symmetry = Symmetry::None;
        ldt.num_c_planes = 4;
        ldt.num_g_planes = 3;
        ldt.c_angles = vec![0.0, 90.0, 180.0, 270.0];
        ldt.g_angles = vec![0.0, 45.0, 90.0];
        ldt.intensities = vec![
            vec![100.0, 80.0, 50.0],
            vec![200.0, 160.0, 100.0],
            vec![50.0, 40.0, 25.0],
            vec![300.0, 240.0, 150.0],
        ];
        ldt.length = 200.0;
        ldt.height = 50.0;
        ldt.lamp_sets.push(LampSet {
            num_lamps: 1,
            lamp_type: "LED".to_string(),
            total_luminous_flux: 1000.0,
            color_appearance: "3000K".to_string(),
            color_rendering_group: "80".to_string(),
            wattage_with_ballast: 10.0,
        });
        let warnings = validate(&ldt);
        assert!(
            !warnings.iter().any(|w| w.code == "W047"),
            "Isym=0 should never produce W047"
        );
    }
}
