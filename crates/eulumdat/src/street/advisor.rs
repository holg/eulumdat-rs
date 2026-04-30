//! Advisor recommendations for failed compliance criteria.
//!
//! After a compliance check produces a [`ComplianceResult`] with one or
//! more failing items, the advisor turns each failure into a single-line
//! recommendation aimed at a lighting designer. The advice is heuristic —
//! it picks the *most likely* lever to pull based on the failure category
//! (glare, uniformity, average illuminance, backlight, uplight) and gives
//! a concrete next step.
//!
//! The strings are intentionally English and stable so we can hash/test
//! them. UI layers may translate them via i18n if needed; for now they
//! are short enough to read in any pane.
//!
//! # Categories
//!
//! - **Glare (MLO G)**: peak intensity at γ > 80° too high → suggest
//!   full-cutoff optics or a tighter beam.
//! - **Backlight (MLO B)**: too much spill behind the luminaire →
//!   reduce backlight via shielding or different optic.
//! - **Uplight (MLO U)**: any LZ that's not LZ4 expects U=0 →
//!   eliminate uplight (use a flat or downward-only luminaire).
//! - **Average illuminance**: design too dim → tighter spacing, taller
//!   pole isn't the right answer (lowers density of light), so the
//!   recommendation favors closer spacing or higher-flux luminaires.
//! - **Uniformity (U₀ or avg/min)**: hot spots vs. dark patches →
//!   closer spacing or a wider beam (lower mounting height tightens
//!   uniformity but darkens between poles, so we lead with spacing).
//! - **Threshold increment / TI** (when present): too much glare for
//!   drivers → reduce intensity at γ > 70° and/or raise mounting height.

use crate::standards::{ComplianceItem, ComplianceResult};

/// One recommendation line tied to a specific failed criterion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvisorTip {
    /// The original failing parameter string from the standard, e.g.
    /// `"Glare (G)"` or `"Uniformity (avg/min)"`.
    pub parameter: String,
    /// One-line recommendation. ≤120 characters; plain text.
    pub message: String,
}

/// Generate advisor tips for all failing items across the compliance
/// results. Returns an empty vec when everything passes.
pub fn advise(results: &[ComplianceResult]) -> Vec<AdvisorTip> {
    let mut out = Vec::new();
    for r in results {
        for item in r.items.iter().filter(|i| !i.passed) {
            out.push(AdvisorTip {
                parameter: item.parameter.clone(),
                message: advice_for(item),
            });
        }
    }
    out
}

/// Pick a recommendation message for a single failed item. The match
/// is case-insensitive substring on the parameter name; this keeps the
/// advisor working when standards are extended (a new "Glare TI" item
/// still routes to the glare branch).
fn advice_for(item: &ComplianceItem) -> String {
    let p = item.parameter.to_lowercase();

    if p.contains("glare") {
        return "Glare exceeds limit. Reduce peak intensity at γ > 80° — use full-cutoff optics or a tighter beam.".to_string();
    }
    if p.contains("backlight") {
        return "Backlight (B) exceeds limit. Add house-side shielding or pick a luminaire with lower BUG B-rating.".to_string();
    }
    if p.contains("uplight") {
        return "Uplight (U) above limit for this lighting zone. Eliminate up-emission — use a flat or fully-shielded optic.".to_string();
    }
    if p.contains("uniformity") || p.contains("u₀") || p.contains("u0") {
        return "Uniformity insufficient. Reduce pole spacing or pick a wider-beam optic; raising mounting height usually hurts.".to_string();
    }
    if p.contains("threshold increment") || p.contains("ti") {
        return "Threshold increment too high. Reduce intensity at γ > 70° and/or raise mounting height.".to_string();
    }
    if p.contains("minimum illuminance") || p.contains("emin") {
        return "Minimum illuminance too low. Reduce pole spacing — the dark band between poles is dragging the minimum down.".to_string();
    }
    if p.contains("average illuminance") || p.contains("avg") || p.contains("ē") {
        return "Average illuminance below target. Reduce pole spacing or step up to a higher-flux luminaire.".to_string();
    }
    if p.contains("luminance") {
        return "Average luminance below target. Increase R-table reflectance, reduce spacing, or pick a luminaire with more downward flux.".to_string();
    }
    "Criterion failed — review the spec and consider a higher-grade luminaire or tighter geometry."
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::standards::{ComplianceItem, ComplianceResult, Region};
    use std::borrow::Cow;

    fn fail(parameter: &str) -> ComplianceItem {
        ComplianceItem {
            parameter: parameter.into(),
            required: "X".into(),
            achieved: "Y".into(),
            passed: false,
        }
    }

    fn result_with(items: Vec<ComplianceItem>) -> ComplianceResult {
        ComplianceResult {
            standard: Cow::Borrowed("test"),
            region: Region::International,
            items,
        }
    }

    #[test]
    fn glare_failure_routes_to_glare_advice() {
        let r = result_with(vec![fail("Glare (G)")]);
        let tips = advise(&[r]);
        assert_eq!(tips.len(), 1);
        assert!(tips[0].message.contains("γ > 80°"), "{}", tips[0].message);
    }

    #[test]
    fn uniformity_routes_to_spacing_advice() {
        let r = result_with(vec![fail("Uniformity (avg/min)")]);
        let tips = advise(&[r]);
        assert!(tips[0].message.to_lowercase().contains("spacing"));
    }

    #[test]
    fn passing_items_produce_no_advice() {
        let pass_item = ComplianceItem {
            parameter: "Glare (G)".into(),
            required: "B0".into(),
            achieved: "B0".into(),
            passed: true,
        };
        let r = result_with(vec![pass_item]);
        let tips = advise(&[r]);
        assert!(tips.is_empty());
    }

    #[test]
    fn unknown_parameter_falls_back_to_generic_message() {
        let r = result_with(vec![fail("Mystery Metric Z")]);
        let tips = advise(&[r]);
        assert_eq!(tips.len(), 1);
        assert!(tips[0].message.contains("Criterion failed"));
    }
}
