//! CJJ 45-2015 — China urban road lighting compliance (skeleton).
//!
//! CJJ 45 is the Chinese industry standard "City Road Lighting Design
//! Specification". It defines four motorized road classes (I–IV) plus
//! separate pedestrian/residential categories. Targets are given in both
//! the luminance method (average luminance, uniformity, TI) and the
//! illuminance method (average + min illuminance).
//!
//! This module implements the **illuminance-method checks** for the four
//! motorized classes — same deferral policy as [`super::rp8`] and
//! [`super::en13201`].
//!
//! The table values here are reference approximations drawn from public
//! summaries of CJJ 45-2015; verify against the official Chinese text
//! before using in certified designs.

use super::{ComplianceItem, ComplianceResult, DesignResult, LightingStandard, Region};

/// CJJ 45 motorized road classes.
///
/// The Chinese spec orders these by importance/volume: ClassI is highest
/// (expressways and main arterials), ClassIV is residential/branch roads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum Cjj45Class {
    /// 快速路 / 主干路 — expressway / main arterial.
    ClassI,
    /// 次干路 — secondary arterial.
    ClassII,
    /// 支路 — branch road.
    ClassIII,
    /// 居住区道路 — residential road.
    ClassIV,
}

impl std::fmt::Display for Cjj45Class {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClassI => write!(f, "Class I (Expressway)"),
            Self::ClassII => write!(f, "Class II (Secondary)"),
            Self::ClassIII => write!(f, "Class III (Branch)"),
            Self::ClassIV => write!(f, "Class IV (Residential)"),
        }
    }
}

/// Illuminance-method criteria for a [`Cjj45Class`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cjj45Criteria {
    /// Required maintained average horizontal illuminance, lux.
    pub avg_illuminance_lux: f64,
    /// Required overall uniformity (min / avg). Higher = better.
    pub min_uniformity_min_avg: f64,
}

impl Cjj45Class {
    /// Look up the illuminance-method criteria.
    ///
    /// Reference values from public summaries of CJJ 45-2015 Table 3.3.2.
    /// Confirm against the official specification before certification use.
    pub fn criteria(self) -> Cjj45Criteria {
        let (avg, u0) = match self {
            Self::ClassI => (20.0, 0.4),
            Self::ClassII => (15.0, 0.4),
            Self::ClassIII => (10.0, 0.35),
            Self::ClassIV => (8.0, 0.3),
        };
        Cjj45Criteria {
            avg_illuminance_lux: avg,
            min_uniformity_min_avg: u0,
        }
    }
}

/// China CJJ 45-2015 — illuminance method.
#[derive(Debug, Clone, Copy, Default)]
pub struct Cjj45Standard;

impl LightingStandard for Cjj45Standard {
    type Selection = Cjj45Class;

    fn name(&self) -> &'static str {
        "CJJ 45-2015 (illuminance)"
    }

    fn region(&self) -> Region {
        Region::Cn
    }

    fn check_design(
        &self,
        class: &Self::Selection,
        design: &DesignResult,
    ) -> Option<ComplianceResult> {
        let crit = class.criteria();

        // U₀ = min / avg — higher is more uniform.
        let achieved_u0 = if design.avg_illuminance_lux > 0.0 {
            design.min_illuminance_lux / design.avg_illuminance_lux
        } else {
            0.0
        };

        let items = vec![
            ComplianceItem {
                parameter: "Average Illuminance (Ē)".into(),
                required: format!("≥ {:.1} lux", crit.avg_illuminance_lux),
                achieved: format!("{:.1} lux", design.avg_illuminance_lux),
                passed: design.avg_illuminance_lux >= crit.avg_illuminance_lux,
            },
            ComplianceItem {
                parameter: "Uniformity U₀ (min/avg)".into(),
                required: format!("≥ {:.2}", crit.min_uniformity_min_avg),
                achieved: format!("{:.2}", achieved_u0),
                passed: achieved_u0 >= crit.min_uniformity_min_avg,
            },
        ];

        Some(ComplianceResult {
            standard: format!("CJJ 45-2015 ({class})").into(),
            region: self.region(),
            items,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn design(avg: f64, min: f64) -> DesignResult {
        DesignResult {
            avg_illuminance_lux: avg,
            min_illuminance_lux: min,
            max_illuminance_lux: avg * 1.5,
            avg_luminance_cd_m2: None,
            uniformity_overall: if avg > 0.0 { min / avg } else { 0.0 },
            uniformity_longitudinal: None,
            threshold_increment_pct: None,
        }
    }

    #[test]
    fn class_i_passes_with_high_quality_design() {
        // ClassI wants ≥20 lux, U₀ ≥ 0.4.
        let d = design(25.0, 12.0); // U₀ = 0.48
        let result = Cjj45Standard.check_design(&Cjj45Class::ClassI, &d).unwrap();
        assert!(result.passed());
        assert_eq!(result.region, Region::Cn);
    }

    #[test]
    fn class_iv_fails_on_uniformity_only() {
        // ClassIV wants ≥8 lux (easy), U₀ ≥ 0.3.
        let d = design(10.0, 2.0); // U₀ = 0.2 (fails)
        let result = Cjj45Standard
            .check_design(&Cjj45Class::ClassIV, &d)
            .unwrap();
        assert!(!result.passed());
        assert_eq!(result.failure_count(), 1);
        assert!(result.items[0].passed, "avg illuminance passes");
        assert!(!result.items[1].passed, "uniformity fails");
    }

    #[test]
    fn class_criteria_monotonic_by_importance() {
        // Higher class (more traffic) → stricter illuminance target.
        use Cjj45Class::*;
        let avgs: Vec<f64> = [ClassI, ClassII, ClassIII, ClassIV]
            .iter()
            .map(|c| c.criteria().avg_illuminance_lux)
            .collect();
        for w in avgs.windows(2) {
            assert!(
                w[0] >= w[1],
                "higher-tier class should have higher illuminance target"
            );
        }
    }
}
