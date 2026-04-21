//! IES/IDA Model Lighting Ordinance — file-level dark-sky compliance.
//!
//! Checks whether a luminaire's BUG rating fits a chosen [`LightingZone`].
//! This is purely a function of the photometric file: no geometry, no layout.
//!
//! The heavy lifting (zone lumen integration, BUG thresholds, per-zone max
//! BUG) already lives in [`crate::bug_rating`]. This module wraps that into
//! the shared [`LightingStandard`] interface so the UI can dispatch against
//! it the same way it does RP-8 or EN 13201.

use super::{ComplianceItem, ComplianceResult, LightingStandard, Region};
use crate::bug_rating::{BugRating, LightingZone, ZoneLumens};

/// IES/IDA Model Lighting Ordinance — file-level BUG-to-zone compliance.
#[derive(Debug, Clone, Copy, Default)]
pub struct MloStandard;

impl LightingStandard for MloStandard {
    type Selection = LightingZone;

    fn name(&self) -> &'static str {
        "IES/IDA MLO"
    }

    fn region(&self) -> Region {
        // MLO originated in the US but is recognized internationally as the
        // dark-sky reference. Classify as US here; international wrappers
        // can re-export it under their own Region if needed.
        Region::Us
    }

    fn check_file(
        &self,
        zone: &Self::Selection,
        ldt: &crate::Eulumdat,
    ) -> Option<ComplianceResult> {
        let zones = ZoneLumens::from_eulumdat(ldt);
        let bug = BugRating::from_zone_lumens(&zones);
        let max = zone.max_bug();

        let items = vec![
            ComplianceItem {
                parameter: "Backlight (B)".into(),
                required: format!("≤ B{}", max.b),
                achieved: format!("B{}", bug.b),
                passed: bug.b <= max.b,
            },
            ComplianceItem {
                parameter: "Uplight (U)".into(),
                required: format!("≤ U{}", max.u),
                achieved: format!("U{}", bug.u),
                passed: bug.u <= max.u,
            },
            ComplianceItem {
                parameter: "Glare (G)".into(),
                required: format!("≤ G{}", max.g),
                achieved: format!("G{}", bug.g),
                passed: bug.g <= max.g,
            },
        ];

        Some(ComplianceResult {
            standard: format!("IES/IDA MLO ({zone})").into(),
            region: self.region(),
            items,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Eulumdat;

    fn load_road() -> Eulumdat {
        let p = "../eulumdat-wasm/templates/road_luminaire.ldt";
        let content = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("read {p}: {e}"));
        Eulumdat::parse(&content).unwrap()
    }

    #[test]
    fn mlo_returns_three_items_per_check() {
        let ldt = load_road();
        let result = MloStandard.check_file(&LightingZone::LZ3, &ldt).unwrap();
        assert_eq!(result.items.len(), 3, "B, U, G must each be checked");
        assert_eq!(result.region, Region::Us);
        assert!(result.standard.contains("LZ3"));
    }

    #[test]
    fn mlo_zones_are_monotonic_in_strictness() {
        // LZ0 < LZ1 < LZ2 < LZ3 < LZ4 in permissiveness. For any luminaire,
        // if it passes a stricter zone it must pass all more-permissive ones.
        let ldt = load_road();
        let results: Vec<_> = LightingZone::all()
            .iter()
            .map(|z| MloStandard.check_file(z, &ldt).unwrap().passed())
            .collect();

        let mut seen_pass = false;
        for passed in &results {
            if *passed {
                seen_pass = true;
            } else if seen_pass {
                panic!(
                    "zones not monotonic in strictness: results = {results:?} \
                     (a stricter zone passed but a more-permissive one failed)"
                );
            }
        }
    }

    #[test]
    fn mlo_agrees_with_direct_bug_check() {
        // MloStandard::check_file must agree with BugRating::compliant_with.
        let ldt = load_road();
        let bug = BugRating::from_zone_lumens(&ZoneLumens::from_eulumdat(&ldt));
        for zone in LightingZone::all() {
            let result = MloStandard.check_file(zone, &ldt).unwrap();
            assert_eq!(
                result.passed(),
                bug.compliant_with(*zone),
                "disagreement at {zone}"
            );
        }
    }
}
