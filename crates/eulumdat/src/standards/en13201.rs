//! DIN EN 13201 — European roadway lighting compliance (skeleton).
//!
//! EN 13201-2 defines three class families:
//! - **M-classes** (M1–M6): motorized traffic, luminance-method targets
//! - **C-classes** (C0–C5): conflict areas, illuminance targets (fallback
//!   when luminance can't be computed)
//! - **P-classes** (P1–P6): pedestrian and low-speed areas, illuminance
//!
//! This module implements the **C- and P-class illuminance checks** only.
//! M-classes require the luminance method (R-tables, veiling luminance, TI)
//! and are deferred to a later phase — same policy as [`super::rp8`].
//!
//! The goal of this skeleton is to prove the [`LightingStandard`] trait
//! works cross-region, not to ship production EN 13201 compliance. Before
//! using this in anger, verify values against the current edition of
//! EN 13201-2 and add the remaining classes.

use super::{ComplianceItem, ComplianceResult, DesignResult, LightingStandard, Region};

/// EN 13201 lighting class. Each class maps to one criteria row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum En13201Class {
    /// C-class conflict areas (illuminance method, EN 13201-2 Table 3).
    C0,
    C1,
    C2,
    C3,
    C4,
    C5,
    /// P-class pedestrian/low-speed areas (illuminance method, Table 4).
    P1,
    P2,
    P3,
    P4,
    P5,
    P6,
}

impl std::fmt::Display for En13201Class {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

/// Criteria looked up for a given [`En13201Class`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct En13201Criteria {
    /// Average maintained horizontal illuminance, lux.
    pub avg_illuminance_lux: f64,
    /// Minimum maintained horizontal illuminance, lux.
    pub min_illuminance_lux: f64,
}

impl En13201Class {
    /// Look up criteria for this class.
    ///
    /// Values from EN 13201-2:2015 Tables 3 and 4. Confirm against the
    /// current edition before using in certified designs.
    pub fn criteria(self) -> En13201Criteria {
        // (avg lux, min lux) per class.
        let (avg, min) = match self {
            // C-classes (conflict areas)
            Self::C0 => (50.0, 30.0),
            Self::C1 => (30.0, 18.0),
            Self::C2 => (20.0, 12.0),
            Self::C3 => (15.0, 9.0),
            Self::C4 => (10.0, 6.0),
            Self::C5 => (7.5, 4.5),
            // P-classes (pedestrian / low-speed)
            Self::P1 => (15.0, 3.0),
            Self::P2 => (10.0, 2.0),
            Self::P3 => (7.5, 1.5),
            Self::P4 => (5.0, 1.0),
            Self::P5 => (3.0, 0.6),
            Self::P6 => (2.0, 0.4),
        };
        En13201Criteria {
            avg_illuminance_lux: avg,
            min_illuminance_lux: min,
        }
    }
}

/// DIN EN 13201 — illuminance method (C/P classes).
#[derive(Debug, Clone, Copy, Default)]
pub struct En13201Standard;

impl LightingStandard for En13201Standard {
    type Selection = En13201Class;

    fn name(&self) -> &'static str {
        "DIN EN 13201 (illuminance)"
    }

    fn region(&self) -> Region {
        Region::Eu
    }

    fn check_design(
        &self,
        class: &Self::Selection,
        design: &DesignResult,
    ) -> Option<ComplianceResult> {
        let crit = class.criteria();

        let items = vec![
            ComplianceItem {
                parameter: "Average Illuminance (Ē)".into(),
                required: format!("≥ {:.1} lux", crit.avg_illuminance_lux),
                achieved: format!("{:.1} lux", design.avg_illuminance_lux),
                passed: design.avg_illuminance_lux >= crit.avg_illuminance_lux,
            },
            ComplianceItem {
                parameter: "Minimum Illuminance (Emin)".into(),
                required: format!("≥ {:.2} lux", crit.min_illuminance_lux),
                achieved: format!("{:.2} lux", design.min_illuminance_lux),
                passed: design.min_illuminance_lux >= crit.min_illuminance_lux,
            },
        ];

        Some(ComplianceResult {
            standard: format!("DIN EN 13201 ({class})").into(),
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
    fn c3_passes_with_sufficient_illuminance() {
        // C3 wants avg ≥ 15 lux, min ≥ 9 lux.
        let result = En13201Standard
            .check_design(&En13201Class::C3, &design(18.0, 10.0))
            .unwrap();
        assert!(result.passed());
        assert_eq!(result.region, Region::Eu);
    }

    #[test]
    fn p3_fails_on_minimum() {
        // P3 wants avg ≥ 7.5 lux, min ≥ 1.5 lux.
        let d = design(10.0, 0.5); // min too low
        let result = En13201Standard.check_design(&En13201Class::P3, &d).unwrap();
        assert!(!result.passed());
        assert_eq!(result.failure_count(), 1);
        assert!(result.items[0].passed);
        assert!(!result.items[1].passed);
    }

    #[test]
    fn every_class_has_distinct_criteria() {
        use En13201Class::*;
        let all = [C0, C1, C2, C3, C4, C5, P1, P2, P3, P4, P5, P6];
        // Classes get strictly less strict as the number goes up
        // (within C-family and within P-family).
        let c_avgs: Vec<f64> = [C0, C1, C2, C3, C4, C5]
            .iter()
            .map(|c| c.criteria().avg_illuminance_lux)
            .collect();
        for w in c_avgs.windows(2) {
            assert!(
                w[0] >= w[1],
                "C-class avg illuminance should be monotonically non-increasing"
            );
        }
        let p_avgs: Vec<f64> = [P1, P2, P3, P4, P5, P6]
            .iter()
            .map(|p| p.criteria().avg_illuminance_lux)
            .collect();
        for w in p_avgs.windows(2) {
            assert!(
                w[0] >= w[1],
                "P-class avg illuminance should be monotonically non-increasing"
            );
        }
        let _ = all;
    }
}
