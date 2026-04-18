//! IESNA roadway luminaire classification.
//!
//! Implements the IESNA (IES) classification system for roadway and area
//! lighting luminaires per ANSI/IES RP-8 and TM-15:
//!
//! - **Lateral distribution type** (I, II, III, IV, V): Based on the location
//!   of the half-maximum isocandela trace relative to the road axis.
//! - **Longitudinal classification** (Short, Medium, Long, Very Long): Based on
//!   the maximum candela angle in the C0-C180 plane.
//! - **Cutoff classification** (Full Cutoff, Cutoff, Semi-Cutoff, Non-Cutoff):
//!   Based on the intensity at high angles (80° and 90° from nadir).

use crate::Eulumdat;

// ============================================================================
// Lateral distribution type (Type I–V)
// ============================================================================

/// IESNA lateral light distribution type.
///
/// Classifies how light is spread laterally (perpendicular to the road axis).
/// Based on the width of the half-maximum isocandela trace on the C90-C270 plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LateralType {
    /// Type I: Narrow symmetric distribution, for walkways and narrow paths.
    /// Peak intensity near C0/C180, half-max width < 15° from road axis.
    TypeI,
    /// Type II: Slightly wider than Type I, for narrower roadways.
    /// Half-max width 15°–25° from road axis.
    TypeII,
    /// Type III: Asymmetric, throws light to one side.
    /// Half-max extends 25°–40° from road axis.
    TypeIII,
    /// Type IV: Semi-cutoff forward throw.
    /// Half-max extends 40°–55° from road axis, minimal backlight.
    TypeIV,
    /// Type V: Symmetric circular distribution (area lighting).
    /// Approximately equal intensity in all lateral directions.
    TypeV,
}

impl std::fmt::Display for LateralType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TypeI => write!(f, "Type I"),
            Self::TypeII => write!(f, "Type II"),
            Self::TypeIII => write!(f, "Type III"),
            Self::TypeIV => write!(f, "Type IV"),
            Self::TypeV => write!(f, "Type V"),
        }
    }
}

// ============================================================================
// Longitudinal classification (Short / Medium / Long)
// ============================================================================

/// IESNA longitudinal light distribution classification.
///
/// Based on the angle of maximum candela in the C0-C180 (along-road) plane,
/// relative to nadir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LongitudinalClass {
    /// Short: Maximum candela at gamma < 52° (1.0 MH throw).
    Short,
    /// Medium: Maximum candela at 52°–63° (1.0–2.25 MH throw).
    Medium,
    /// Long: Maximum candela at 63°–70° (2.25–2.75 MH throw).
    Long,
    /// Very Long: Maximum candela at gamma > 70° (>2.75 MH throw).
    VeryLong,
}

impl std::fmt::Display for LongitudinalClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Short => write!(f, "Short"),
            Self::Medium => write!(f, "Medium"),
            Self::Long => write!(f, "Long"),
            Self::VeryLong => write!(f, "Very Long"),
        }
    }
}

// ============================================================================
// Cutoff classification
// ============================================================================

/// IESNA cutoff classification.
///
/// Based on the intensity at high angles relative to the maximum intensity.
/// Determines how well the luminaire controls glare and uplight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CutoffClass {
    /// Full Cutoff: No light at or above 90°, ≤10% at 80°.
    FullCutoff,
    /// Cutoff: ≤2.5% at 90°, ≤25% at 80°.
    Cutoff,
    /// Semi-Cutoff: ≤5% at 90°, ≤50% at 80°.
    SemiCutoff,
    /// Non-Cutoff: Exceeds Semi-Cutoff limits.
    NonCutoff,
}

impl std::fmt::Display for CutoffClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FullCutoff => write!(f, "Full Cutoff"),
            Self::Cutoff => write!(f, "Cutoff"),
            Self::SemiCutoff => write!(f, "Semi-Cutoff"),
            Self::NonCutoff => write!(f, "Non-Cutoff"),
        }
    }
}

// ============================================================================
// Combined classification result
// ============================================================================

/// Whether the IESNA roadway classification is applicable to this luminaire.
///
/// The IES RP-8 classification system is designed for outdoor roadway and area
/// lighting. It is not meaningful for:
/// - Indoor luminaires (ceiling, recessed, pendant)
/// - Uplights (primary emission above horizontal)
/// - Decorative/accent fixtures
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Applicability {
    /// Classification is applicable — luminaire has significant downward outdoor
    /// distribution characteristics.
    Applicable,
    /// Luminaire is primarily an uplight (>50% flux above horizontal).
    /// Roadway classification is not meaningful.
    Uplight,
    /// Luminaire is primarily indoor (symmetric, short throw, no cutoff control).
    /// Classification values are computed but may not be meaningful.
    IndoorType,
}

impl std::fmt::Display for Applicability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Applicable => write!(f, "Applicable"),
            Self::Uplight => write!(f, "Not applicable (uplight)"),
            Self::IndoorType => write!(f, "Not applicable (indoor type)"),
        }
    }
}

/// Complete IESNA roadway luminaire classification.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IesnaClassification {
    /// Whether this classification is meaningful for this luminaire.
    pub applicability: Applicability,
    /// Lateral distribution type (I–V).
    pub lateral_type: LateralType,
    /// Longitudinal throw classification.
    pub longitudinal: LongitudinalClass,
    /// Cutoff classification.
    pub cutoff: CutoffClass,
    /// Maximum candela value (cd/klm).
    pub max_candela: f64,
    /// Gamma angle of maximum candela in C0-C180 plane (degrees).
    pub max_candela_gamma: f64,
    /// Intensity at 80° from nadir as percentage of max.
    pub intensity_at_80: f64,
    /// Intensity at 90° from nadir as percentage of max.
    pub intensity_at_90: f64,
    /// Designation string, e.g. "Type III Medium Cutoff".
    pub designation: String,
}

impl std::fmt::Display for IesnaClassification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.designation)
    }
}

// ============================================================================
// Classification logic
// ============================================================================

/// Classify a luminaire according to IESNA roadway standards.
///
/// Always computes the classification values, but also indicates whether
/// the classification is meaningful for the given luminaire type.
/// Indoor luminaires, uplights, and decorative fixtures will have
/// `applicability != Applicable`.
pub fn classify(ldt: &Eulumdat) -> IesnaClassification {
    let max_candela = ldt.max_intensity();

    // Find angle of peak intensity in C0-C180 plane
    let max_candela_gamma = find_peak_gamma(ldt);

    // Intensity ratios at critical angles
    let i_at_80 = sample_max_across_c_planes(ldt, 80.0);
    let i_at_90 = sample_max_across_c_planes(ldt, 90.0);

    let pct_at_80 = if max_candela > 0.0 {
        i_at_80 / max_candela * 100.0
    } else {
        0.0
    };
    let pct_at_90 = if max_candela > 0.0 {
        i_at_90 / max_candela * 100.0
    } else {
        0.0
    };

    let lateral = classify_lateral(ldt);
    let longitudinal = classify_longitudinal(max_candela_gamma);
    let cutoff = classify_cutoff(pct_at_80, pct_at_90);

    // Determine applicability
    let applicability = determine_applicability(ldt, &lateral, max_candela_gamma);

    let designation = if applicability == Applicability::Applicable {
        format!("{} {} {}", lateral, longitudinal, cutoff)
    } else {
        format!(
            "{} {} {} ({})",
            lateral, longitudinal, cutoff, applicability
        )
    };

    IesnaClassification {
        applicability,
        lateral_type: lateral,
        longitudinal,
        cutoff,
        max_candela,
        max_candela_gamma,
        intensity_at_80: pct_at_80,
        intensity_at_90: pct_at_90,
        designation,
    }
}

/// Determine whether the IESNA roadway classification is meaningful.
fn determine_applicability(ldt: &Eulumdat, lateral: &LateralType, max_gamma: f64) -> Applicability {
    // Check if primarily uplight: downward_flux_fraction < 50%
    if ldt.downward_flux_fraction < 50.0 {
        return Applicability::Uplight;
    }

    // Check if indoor type: symmetric (Type V) with short throw and
    // peak near nadir (typical of ceiling-mounted indoor fixtures)
    if *lateral == LateralType::TypeV && max_gamma < 30.0 {
        return Applicability::IndoorType;
    }

    // Rotationally symmetric sources with very narrow beam at nadir
    // are typically indoor downlights
    if ldt.symmetry == crate::Symmetry::VerticalAxis && max_gamma < 15.0 {
        return Applicability::IndoorType;
    }

    Applicability::Applicable
}

/// Find the gamma angle of peak intensity, searching C0 and C180 planes.
fn find_peak_gamma(ldt: &Eulumdat) -> f64 {
    let mut max_i = 0.0f64;
    let mut max_gamma = 0.0f64;

    // Search in 0.5° steps from 0° to 90°
    for gi in 0..=180 {
        let gamma = gi as f64 * 0.5;
        // Check C0 and C180 (along-road planes)
        let i_c0 = ldt.sample(0.0, gamma);
        let i_c180 = ldt.sample(180.0, gamma);
        let i = i_c0.max(i_c180);
        if i > max_i {
            max_i = i;
            max_gamma = gamma;
        }
    }
    max_gamma
}

/// Maximum intensity at a given gamma across all C-planes.
fn sample_max_across_c_planes(ldt: &Eulumdat, gamma: f64) -> f64 {
    let mut max_i = 0.0f64;
    // Sample every 5° in C
    for ci in 0..72 {
        let c = ci as f64 * 5.0;
        let i = ldt.sample(c, gamma);
        if i > max_i {
            max_i = i;
        }
    }
    max_i
}

/// Classify lateral distribution type (I–V).
///
/// Based on the angular width of the half-maximum isocandela in the
/// C90-C270 plane (perpendicular to road axis).
fn classify_lateral(ldt: &Eulumdat) -> LateralType {
    // Check if distribution is approximately symmetric (Type V)
    // by comparing C0, C90, C180, C270 at gamma=60°
    let i_c0 = ldt.sample(0.0, 60.0);
    let i_c90 = ldt.sample(90.0, 60.0);
    let i_c180 = ldt.sample(180.0, 60.0);
    let i_c270 = ldt.sample(270.0, 60.0);

    let avg = (i_c0 + i_c90 + i_c180 + i_c270) / 4.0;
    if avg > 0.0 {
        let max_dev = [i_c0, i_c90, i_c180, i_c270]
            .iter()
            .map(|&i| ((i - avg) / avg).abs())
            .fold(0.0f64, f64::max);

        // If intensity varies < 25% across all directions → Type V (circular)
        if max_dev < 0.25 {
            return LateralType::TypeV;
        }
    }

    // Find the half-maximum width in the C90-C270 plane
    // Peak is typically at or near C90 or C270
    let mut peak_c90 = 0.0f64;
    let mut peak_gamma = 0.0;
    for gi in 0..=18 {
        let gamma = gi as f64 * 5.0;
        let i = ldt.sample(90.0, gamma).max(ldt.sample(270.0, gamma));
        if i > peak_c90 {
            peak_c90 = i;
            peak_gamma = gamma;
        }
    }

    if peak_c90 <= 0.0 {
        return LateralType::TypeI;
    }

    // Find the maximum candela overall to compute half-max threshold
    let max_cd = ldt.max_intensity();
    let half_max = max_cd * 0.5;

    // Measure the lateral spread: at the peak gamma angle, sweep C from 0° to 180°
    // and find where intensity exceeds half-max
    let mut max_lateral_angle = 0.0f64;
    for ci in 0..=36 {
        let c = ci as f64 * 5.0;
        // Check both sides (C and 360-C)
        let i = ldt.sample(c, peak_gamma);
        if i >= half_max {
            // Lateral angle = min angle from road axis (C0/C180)
            let lat = c.min(180.0 - c).min((360.0 - c).abs());
            if lat > max_lateral_angle {
                max_lateral_angle = lat;
            }
        }
    }

    match max_lateral_angle {
        a if a < 15.0 => LateralType::TypeI,
        a if a < 25.0 => LateralType::TypeII,
        a if a < 40.0 => LateralType::TypeIII,
        _ => LateralType::TypeIV,
    }
}

/// Classify longitudinal throw based on the gamma angle of maximum candela.
fn classify_longitudinal(max_gamma: f64) -> LongitudinalClass {
    match max_gamma {
        g if g < 52.0 => LongitudinalClass::Short,
        g if g < 63.0 => LongitudinalClass::Medium,
        g if g < 70.0 => LongitudinalClass::Long,
        _ => LongitudinalClass::VeryLong,
    }
}

/// Classify cutoff based on intensity at 80° and 90° as percentage of max.
fn classify_cutoff(pct_at_80: f64, pct_at_90: f64) -> CutoffClass {
    if pct_at_90 <= 0.5 && pct_at_80 <= 10.0 {
        CutoffClass::FullCutoff
    } else if pct_at_90 <= 2.5 && pct_at_80 <= 25.0 {
        CutoffClass::Cutoff
    } else if pct_at_90 <= 5.0 && pct_at_80 <= 50.0 {
        CutoffClass::SemiCutoff
    } else {
        CutoffClass::NonCutoff
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_road_luminaire() {
        let content = include_str!("../../eulumdat-wasm/templates/road_luminaire.ldt");
        let ldt = Eulumdat::parse(content).unwrap();
        let cls = classify(&ldt);

        eprintln!("Road luminaire: {}", cls.designation);
        eprintln!("  Lateral: {}", cls.lateral_type);
        eprintln!(
            "  Longitudinal: {} (peak gamma={:.1}°)",
            cls.longitudinal, cls.max_candela_gamma
        );
        eprintln!(
            "  Cutoff: {} (80°={:.1}%, 90°={:.1}%)",
            cls.cutoff, cls.intensity_at_80, cls.intensity_at_90
        );
        eprintln!("  Max cd/klm: {:.1}", cls.max_candela);

        // Road luminaires are typically Type II-IV, Medium-Long throw
        assert!(
            matches!(
                cls.lateral_type,
                LateralType::TypeII | LateralType::TypeIII | LateralType::TypeIV
            ),
            "Road luminaire should be Type II-IV, got {}",
            cls.lateral_type
        );
    }

    #[test]
    fn classify_fluorescent() {
        let content = include_str!("../../eulumdat-wasm/templates/fluorescent_luminaire.ldt");
        let ldt = Eulumdat::parse(content).unwrap();
        let cls = classify(&ldt);

        eprintln!("Fluorescent: {}", cls.designation);
        eprintln!("  Lateral: {}", cls.lateral_type);
        eprintln!(
            "  Longitudinal: {} (peak gamma={:.1}°)",
            cls.longitudinal, cls.max_candela_gamma
        );
        eprintln!(
            "  Cutoff: {} (80°={:.1}%, 90°={:.1}%)",
            cls.cutoff, cls.intensity_at_80, cls.intensity_at_90
        );

        // Indoor fluorescent: typically Type V (symmetric), Short throw
        assert_eq!(
            cls.longitudinal,
            LongitudinalClass::Short,
            "Fluorescent should be Short throw, got {}",
            cls.longitudinal
        );
    }

    #[test]
    fn classify_projector() {
        let content = include_str!("../../eulumdat-wasm/templates/projector.ldt");
        let ldt = Eulumdat::parse(content).unwrap();
        let cls = classify(&ldt);

        eprintln!("Projector: {}", cls.designation);
        eprintln!("  Lateral: {}", cls.lateral_type);
        eprintln!(
            "  Longitudinal: {} (peak gamma={:.1}°)",
            cls.longitudinal, cls.max_candela_gamma
        );
        eprintln!(
            "  Cutoff: {} (80°={:.1}%, 90°={:.1}%)",
            cls.cutoff, cls.intensity_at_80, cls.intensity_at_90
        );
    }

    #[test]
    fn classify_uplight() {
        let content = include_str!("../../eulumdat-wasm/templates/floor_uplight.ldt");
        let ldt = Eulumdat::parse(content).unwrap();
        let cls = classify(&ldt);

        eprintln!("Uplight: {}", cls.designation);
        // Uplights emit upward, so gamma of max intensity > 90° typically
        // The classification is designed for roadway luminaires but should still work
    }

    #[test]
    fn cutoff_thresholds() {
        assert_eq!(classify_cutoff(0.0, 0.0), CutoffClass::FullCutoff);
        assert_eq!(classify_cutoff(10.0, 0.5), CutoffClass::FullCutoff);
        assert_eq!(classify_cutoff(10.1, 0.5), CutoffClass::Cutoff);
        assert_eq!(classify_cutoff(25.0, 2.5), CutoffClass::Cutoff);
        assert_eq!(classify_cutoff(25.1, 2.5), CutoffClass::SemiCutoff);
        assert_eq!(classify_cutoff(50.0, 5.0), CutoffClass::SemiCutoff);
        assert_eq!(classify_cutoff(50.1, 5.0), CutoffClass::NonCutoff);
        assert_eq!(classify_cutoff(60.0, 10.0), CutoffClass::NonCutoff);
    }

    #[test]
    fn longitudinal_thresholds() {
        assert_eq!(classify_longitudinal(0.0), LongitudinalClass::Short);
        assert_eq!(classify_longitudinal(51.9), LongitudinalClass::Short);
        assert_eq!(classify_longitudinal(52.0), LongitudinalClass::Medium);
        assert_eq!(classify_longitudinal(62.9), LongitudinalClass::Medium);
        assert_eq!(classify_longitudinal(63.0), LongitudinalClass::Long);
        assert_eq!(classify_longitudinal(69.9), LongitudinalClass::Long);
        assert_eq!(classify_longitudinal(70.0), LongitudinalClass::VeryLong);
    }

    #[test]
    fn display_formatting() {
        assert_eq!(format!("{}", LateralType::TypeIII), "Type III");
        assert_eq!(format!("{}", LongitudinalClass::Medium), "Medium");
        assert_eq!(format!("{}", CutoffClass::FullCutoff), "Full Cutoff");
    }
}
