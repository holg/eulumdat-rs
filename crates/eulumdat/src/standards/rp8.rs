//! ANSI/IES RP-8 — US roadway lighting compliance (illuminance method).
//!
//! The RP-8 selection is a pair: `RoadClass × PedestrianConflict`. Together
//! they look up an `Rp8Criteria` row (avg illuminance + uniformity ratio)
//! that the computed design must meet.
//!
//! Luminance method (cd/m², U₀/Uₗ, TI) is deferred to a later phase — it
//! requires R-table integration and veiling-luminance math.

use super::{ComplianceItem, ComplianceResult, DesignResult, LightingStandard, Region};

/// RP-8 functional road classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum RoadClass {
    /// High-speed, high-volume arterials (freeways, expressways).
    Major,
    /// Moderate-traffic distributors.
    Collector,
    /// Low-speed residential / neighborhood streets.
    Local,
}

impl std::fmt::Display for RoadClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Major => write!(f, "Major"),
            Self::Collector => write!(f, "Collector"),
            Self::Local => write!(f, "Local"),
        }
    }
}

/// Pedestrian conflict area classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum PedestrianConflict {
    /// > 100 pedestrians/hr — urban commercial, dense mixed-use.
    High,
    /// 11–100 pedestrians/hr — urban mixed, suburban commercial.
    Medium,
    /// < 11 pedestrians/hr — suburban residential, rural.
    Low,
}

impl std::fmt::Display for PedestrianConflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::High => write!(f, "High"),
            Self::Medium => write!(f, "Medium"),
            Self::Low => write!(f, "Low"),
        }
    }
}

/// User selection for RP-8 compliance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct Rp8Selection {
    pub road_class: RoadClass,
    pub pedestrian_conflict: PedestrianConflict,
}

/// Threshold values looked up from the RP-8 illuminance-method table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rp8Criteria {
    /// Required average maintained illuminance, lux.
    pub avg_illuminance_lux: f64,
    /// Maximum allowed `avg / min` uniformity ratio (lower is better).
    pub max_uniformity_avg_min: f64,
}

impl Rp8Selection {
    /// Look up the illuminance-method criteria from the RP-8 table.
    ///
    /// Values are from RP-8-00; RP-8-25 preserves the structure with
    /// some adjusted values (verify against the latest edition before
    /// using in a real project).
    pub fn criteria(&self) -> Rp8Criteria {
        use PedestrianConflict::*;
        use RoadClass::*;
        let (avg, unif) = match (self.road_class, self.pedestrian_conflict) {
            (Major, High) => (17.0, 3.0),
            (Major, Medium) => (13.0, 3.0),
            (Major, Low) => (9.0, 3.0),
            (Collector, High) => (12.0, 4.0),
            (Collector, Medium) => (9.0, 4.0),
            (Collector, Low) => (6.0, 4.0),
            (Local, High) => (9.0, 6.0),
            (Local, Medium) => (7.0, 6.0),
            (Local, Low) => (4.0, 6.0),
        };
        Rp8Criteria {
            avg_illuminance_lux: avg,
            max_uniformity_avg_min: unif,
        }
    }
}

/// US ANSI/IES RP-8 — illuminance method.
#[derive(Debug, Clone, Copy, Default)]
pub struct Rp8Standard;

impl LightingStandard for Rp8Standard {
    type Selection = Rp8Selection;

    fn name(&self) -> &'static str {
        "ANSI/IES RP-8 (illuminance)"
    }

    fn region(&self) -> Region {
        Region::Us
    }

    fn check_design(
        &self,
        selection: &Self::Selection,
        design: &DesignResult,
    ) -> Option<ComplianceResult> {
        let crit = selection.criteria();

        let achieved_unif = if design.min_illuminance_lux > 0.0 {
            design.avg_illuminance_lux / design.min_illuminance_lux
        } else {
            f64::INFINITY
        };

        let items = vec![
            ComplianceItem {
                parameter: "Average Illuminance".into(),
                required: format!("≥ {:.1} lux", crit.avg_illuminance_lux),
                achieved: format!("{:.1} lux", design.avg_illuminance_lux),
                passed: design.avg_illuminance_lux >= crit.avg_illuminance_lux,
            },
            ComplianceItem {
                parameter: "Uniformity (avg/min)".into(),
                required: format!("≤ {:.1}", crit.max_uniformity_avg_min),
                achieved: if achieved_unif.is_finite() {
                    format!("{:.2}", achieved_unif)
                } else {
                    "∞".into()
                },
                passed: achieved_unif.is_finite()
                    && achieved_unif <= crit.max_uniformity_avg_min,
            },
        ];

        Some(ComplianceResult {
            standard: format!(
                "ANSI/IES RP-8 ({}/{})",
                selection.road_class, selection.pedestrian_conflict
            )
            .into(),
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
    fn passing_design_on_major_medium() {
        // Major/Medium wants ≥13 lux, avg/min ≤ 3.
        let sel = Rp8Selection {
            road_class: RoadClass::Major,
            pedestrian_conflict: PedestrianConflict::Medium,
        };
        let d = design(15.0, 7.0); // avg/min = 2.14
        let result = Rp8Standard.check_design(&sel, &d).unwrap();
        assert!(result.passed(), "{result:?}");
        assert_eq!(result.items.len(), 2);
    }

    #[test]
    fn fails_illuminance_on_major_high() {
        // Major/High wants ≥17 lux.
        let sel = Rp8Selection {
            road_class: RoadClass::Major,
            pedestrian_conflict: PedestrianConflict::High,
        };
        let d = design(12.0, 5.0);
        let result = Rp8Standard.check_design(&sel, &d).unwrap();
        assert!(!result.passed());
        assert_eq!(result.failure_count(), 1, "only illuminance fails");
        assert!(!result.items[0].passed);
        assert!(result.items[1].passed);
    }

    #[test]
    fn fails_uniformity_on_local_low() {
        // Local/Low wants ≥4 lux, avg/min ≤ 6.
        let sel = Rp8Selection {
            road_class: RoadClass::Local,
            pedestrian_conflict: PedestrianConflict::Low,
        };
        // avg=5 (passes), min=0.5 → avg/min = 10 (fails uniformity)
        let d = design(5.0, 0.5);
        let result = Rp8Standard.check_design(&sel, &d).unwrap();
        assert!(!result.passed());
        assert_eq!(result.failure_count(), 1);
        assert!(result.items[0].passed);
        assert!(!result.items[1].passed);
    }

    #[test]
    fn zero_min_illuminance_reports_infinity_and_fails() {
        let sel = Rp8Selection {
            road_class: RoadClass::Collector,
            pedestrian_conflict: PedestrianConflict::Low,
        };
        let d = design(7.0, 0.0);
        let result = Rp8Standard.check_design(&sel, &d).unwrap();
        assert_eq!(result.items[1].achieved, "∞");
        assert!(!result.items[1].passed);
    }

    #[test]
    fn check_file_returns_none() {
        // RP-8 is design-level only.
        let sel = Rp8Selection {
            road_class: RoadClass::Local,
            pedestrian_conflict: PedestrianConflict::Low,
        };
        let ldt = crate::Eulumdat::default();
        assert!(Rp8Standard.check_file(&sel, &ldt).is_none());
    }

    #[test]
    fn end_to_end_with_real_layout() {
        use crate::street::{Arrangement, StreetLayout};
        let ldt_content =
            std::fs::read_to_string("../eulumdat-wasm/templates/road_luminaire.ldt").unwrap();
        let ldt = crate::Eulumdat::parse(&ldt_content).unwrap();

        let layout = StreetLayout {
            arrangement: Arrangement::Staggered,
            pole_spacing_m: 30.0,
            ..Default::default()
        };
        let area = layout.compute(&ldt, 0.8);
        let design = layout.design_result(&area);

        let sel = Rp8Selection {
            road_class: RoadClass::Local,
            pedestrian_conflict: PedestrianConflict::Low,
        };
        let result = Rp8Standard.check_design(&sel, &design).unwrap();
        // We don't assert pass/fail (template luminaire may or may not meet
        // the table) — just that the pipeline produces a well-formed result.
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.region, Region::Us);
    }
}

