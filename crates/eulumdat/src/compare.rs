//! Photometric file comparison engine.
//!
//! Compares two photometric files side-by-side, computing deltas across
//! all key metrics and producing a similarity score.
//!
//! # Example
//!
//! ```rust,no_run
//! use eulumdat::{Eulumdat, PhotometricComparison};
//!
//! let a = Eulumdat::from_file("luminaire_a.ldt").unwrap();
//! let b = Eulumdat::from_file("luminaire_b.ldt").unwrap();
//!
//! let cmp = PhotometricComparison::from_eulumdat(&a, &b, "File A", "File B");
//! println!("{}", cmp.to_text());
//! println!("Similarity: {:.1}%", cmp.similarity_score * 100.0);
//! ```

use crate::bug_rating::BugDiagram;
use crate::calculations::PhotometricSummary;
use crate::eulumdat::Eulumdat;
use crate::units::UnitSystem;

/// Significance level of a metric delta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Significance {
    /// < 5% difference
    Negligible,
    /// 5–15% difference
    Minor,
    /// 15–30% difference
    Moderate,
    /// > 30% difference
    Major,
}

impl Significance {
    /// Classify a percentage delta into a significance level.
    pub fn from_delta_percent(delta_pct: f64) -> Self {
        let abs = delta_pct.abs();
        if abs < 5.0 {
            Self::Negligible
        } else if abs < 15.0 {
            Self::Minor
        } else if abs < 30.0 {
            Self::Moderate
        } else {
            Self::Major
        }
    }
}

impl std::fmt::Display for Significance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Negligible => write!(f, "negligible"),
            Self::Minor => write!(f, "minor"),
            Self::Moderate => write!(f, "moderate"),
            Self::Major => write!(f, "MAJOR"),
        }
    }
}

/// A single compared metric between two files.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ComparisonMetric {
    /// Human-readable metric name
    pub name: String,
    /// Short key (for programmatic use)
    pub key: String,
    /// Unit string (e.g. "lm", "%", "cd/klm")
    pub unit: String,
    /// Value from file A
    pub value_a: f64,
    /// Value from file B
    pub value_b: f64,
    /// Absolute delta (B - A)
    pub delta: f64,
    /// Percentage delta relative to A (0 if A is zero)
    pub delta_percent: f64,
    /// Significance classification
    pub significance: Significance,
    /// Weight for similarity score (higher = more important)
    weight: f64,
}

/// Result of comparing two photometric files.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PhotometricComparison {
    /// Label for file A
    pub label_a: String,
    /// Label for file B
    pub label_b: String,
    /// Summary of file A
    pub summary_a: PhotometricSummary,
    /// Summary of file B
    pub summary_b: PhotometricSummary,
    /// All compared metrics
    pub metrics: Vec<ComparisonMetric>,
    /// Overall similarity score (0.0 = completely different, 1.0 = identical)
    pub similarity_score: f64,
}

impl PhotometricComparison {
    /// Compare two Eulumdat files.
    pub fn from_eulumdat(a: &Eulumdat, b: &Eulumdat, label_a: &str, label_b: &str) -> Self {
        Self::from_eulumdat_with_units(a, b, label_a, label_b, UnitSystem::default())
    }

    /// Compare two Eulumdat files with unit system for dimension metrics.
    pub fn from_eulumdat_with_units(
        a: &Eulumdat,
        b: &Eulumdat,
        label_a: &str,
        label_b: &str,
        units: UnitSystem,
    ) -> Self {
        let summary_a = PhotometricSummary::from_eulumdat(a);
        let summary_b = PhotometricSummary::from_eulumdat(b);
        let mut metrics = build_metrics(&summary_a, &summary_b);

        // BUG ratings (need raw Eulumdat)
        let bug_a = BugDiagram::from_eulumdat(a);
        let bug_b = BugDiagram::from_eulumdat(b);
        metrics.push(metric(
            "BUG Backlight (B)",
            "bug_b",
            "",
            bug_a.rating.b as f64,
            bug_b.rating.b as f64,
            0.5,
        ));
        metrics.push(metric(
            "BUG Uplight (U)",
            "bug_u",
            "",
            bug_a.rating.u as f64,
            bug_b.rating.u as f64,
            0.5,
        ));
        metrics.push(metric(
            "BUG Glare (G)",
            "bug_g",
            "",
            bug_a.rating.g as f64,
            bug_b.rating.g as f64,
            0.5,
        ));

        // Physical dimensions (need raw Eulumdat)
        let dim_unit = units.dimension_label();
        metrics.push(metric(
            "Luminaire Length",
            "length",
            dim_unit,
            units.convert_mm(a.length),
            units.convert_mm(b.length),
            0.3,
        ));
        metrics.push(metric(
            "Luminaire Width",
            "width",
            dim_unit,
            units.convert_mm(a.width),
            units.convert_mm(b.width),
            0.3,
        ));
        metrics.push(metric(
            "Luminaire Height",
            "height",
            dim_unit,
            units.convert_mm(a.height),
            units.convert_mm(b.height),
            0.3,
        ));

        let similarity_score = compute_similarity(&metrics);
        Self {
            label_a: label_a.to_string(),
            label_b: label_b.to_string(),
            summary_a,
            summary_b,
            metrics,
            similarity_score,
        }
    }

    /// Compare two Eulumdat files with localized metric names.
    #[cfg(feature = "i18n")]
    pub fn from_eulumdat_with_locale(
        a: &Eulumdat,
        b: &Eulumdat,
        label_a: &str,
        label_b: &str,
        locale: &eulumdat_i18n::Locale,
    ) -> Self {
        Self::from_eulumdat_with_units_and_locale(
            a,
            b,
            label_a,
            label_b,
            UnitSystem::default(),
            locale,
        )
    }

    /// Compare two Eulumdat files with unit system and localized metric names.
    #[cfg(feature = "i18n")]
    pub fn from_eulumdat_with_units_and_locale(
        a: &Eulumdat,
        b: &Eulumdat,
        label_a: &str,
        label_b: &str,
        units: UnitSystem,
        locale: &eulumdat_i18n::Locale,
    ) -> Self {
        let summary_a = PhotometricSummary::from_eulumdat(a);
        let summary_b = PhotometricSummary::from_eulumdat(b);
        let mut metrics = build_metrics_with_locale(&summary_a, &summary_b, locale);

        // BUG ratings
        let bug_a = BugDiagram::from_eulumdat(a);
        let bug_b = BugDiagram::from_eulumdat(b);
        metrics.push(metric_localized(
            "BUG Backlight (B)",
            "bug_b",
            "",
            bug_a.rating.b as f64,
            bug_b.rating.b as f64,
            0.5,
            locale,
        ));
        metrics.push(metric_localized(
            "BUG Uplight (U)",
            "bug_u",
            "",
            bug_a.rating.u as f64,
            bug_b.rating.u as f64,
            0.5,
            locale,
        ));
        metrics.push(metric_localized(
            "BUG Glare (G)",
            "bug_g",
            "",
            bug_a.rating.g as f64,
            bug_b.rating.g as f64,
            0.5,
            locale,
        ));

        // Physical dimensions
        let dim_unit = units.dimension_label();
        metrics.push(metric_localized(
            "Luminaire Length",
            "length",
            dim_unit,
            units.convert_mm(a.length),
            units.convert_mm(b.length),
            0.3,
            locale,
        ));
        metrics.push(metric_localized(
            "Luminaire Width",
            "width",
            dim_unit,
            units.convert_mm(a.width),
            units.convert_mm(b.width),
            0.3,
            locale,
        ));
        metrics.push(metric_localized(
            "Luminaire Height",
            "height",
            dim_unit,
            units.convert_mm(a.height),
            units.convert_mm(b.height),
            0.3,
            locale,
        ));

        let similarity_score = compute_similarity(&metrics);
        Self {
            label_a: label_a.to_string(),
            label_b: label_b.to_string(),
            summary_a,
            summary_b,
            metrics,
            similarity_score,
        }
    }

    /// Compare two pre-computed summaries.
    pub fn from_summaries(
        summary_a: PhotometricSummary,
        summary_b: PhotometricSummary,
        label_a: &str,
        label_b: &str,
    ) -> Self {
        let metrics = build_metrics(&summary_a, &summary_b);
        let similarity_score = compute_similarity(&metrics);
        Self {
            label_a: label_a.to_string(),
            label_b: label_b.to_string(),
            summary_a,
            summary_b,
            metrics,
            similarity_score,
        }
    }

    /// Return only metrics at or above the given significance threshold.
    pub fn significant_metrics(&self, min: Significance) -> Vec<&ComparisonMetric> {
        self.metrics
            .iter()
            .filter(|m| m.significance >= min)
            .collect()
    }

    /// Format as a terminal-friendly text table.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "COMPARISON: {} vs {}\n",
            self.label_a, self.label_b
        ));
        out.push_str(&format!(
            "Similarity: {:.1}%\n",
            self.similarity_score * 100.0
        ));
        out.push_str(&"=".repeat(80));
        out.push('\n');
        out.push_str(&format!(
            "{:<28} {:>12} {:>12} {:>10} {:>8}  {}\n",
            "Metric", "A", "B", "Delta", "%", "Significance"
        ));
        out.push_str(&"-".repeat(80));
        out.push('\n');

        for m in &self.metrics {
            let unit = if m.unit.is_empty() {
                String::new()
            } else {
                format!(" {}", m.unit)
            };
            out.push_str(&format!(
                "{:<28} {:>10.1}{:<2} {:>10.1}{:<2} {:>+9.1} {:>+7.1}%  {}\n",
                m.name, m.value_a, unit, m.value_b, unit, m.delta, m.delta_percent, m.significance,
            ));
        }

        out.push_str(&"-".repeat(80));
        out.push('\n');
        out
    }

    /// Format as CSV with a header row.
    pub fn to_csv(&self) -> String {
        let mut out = String::new();
        out.push_str("metric,key,unit,value_a,value_b,delta,delta_percent,significance\n");
        for m in &self.metrics {
            out.push_str(&format!(
                "\"{}\",{},{},{:.4},{:.4},{:.4},{:.4},{}\n",
                m.name,
                m.key,
                m.unit,
                m.value_a,
                m.value_b,
                m.delta,
                m.delta_percent,
                m.significance,
            ));
        }
        out
    }
}

impl std::fmt::Display for PhotometricComparison {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_text())
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn metric(name: &str, key: &str, unit: &str, a: f64, b: f64, weight: f64) -> ComparisonMetric {
    let delta = b - a;
    let delta_percent = if a.abs() > 1e-9 {
        (delta / a) * 100.0
    } else if b.abs() > 1e-9 {
        100.0 // A is zero, B is not → 100% change
    } else {
        0.0 // both zero
    };
    ComparisonMetric {
        name: name.to_string(),
        key: key.to_string(),
        unit: unit.to_string(),
        value_a: a,
        value_b: b,
        delta,
        delta_percent,
        significance: Significance::from_delta_percent(delta_percent),
        weight,
    }
}

fn build_metrics(a: &PhotometricSummary, b: &PhotometricSummary) -> Vec<ComparisonMetric> {
    vec![
        // Flux & efficiency (8 metrics)
        metric(
            "Total Lamp Flux",
            "total_lamp_flux",
            "lm",
            a.total_lamp_flux,
            b.total_lamp_flux,
            2.0,
        ),
        metric(
            "Calculated Flux",
            "calculated_flux",
            "lm",
            a.calculated_flux,
            b.calculated_flux,
            2.0,
        ),
        metric("Light Output Ratio", "lor", "%", a.lor, b.lor, 1.5),
        metric("DLOR", "dlor", "%", a.dlor, b.dlor, 1.0),
        metric("ULOR", "ulor", "%", a.ulor, b.ulor, 1.0),
        metric(
            "Lamp Efficacy",
            "lamp_efficacy",
            "lm/W",
            a.lamp_efficacy,
            b.lamp_efficacy,
            2.0,
        ),
        metric(
            "Luminaire Efficacy",
            "luminaire_efficacy",
            "lm/W",
            a.luminaire_efficacy,
            b.luminaire_efficacy,
            2.0,
        ),
        metric(
            "Total Wattage",
            "total_wattage",
            "W",
            a.total_wattage,
            b.total_wattage,
            1.5,
        ),
        // IES beam characteristics (2 metrics)
        metric(
            "Beam Angle (IES)",
            "beam_angle",
            "deg",
            a.beam_angle,
            b.beam_angle,
            2.0,
        ),
        metric(
            "Field Angle (IES)",
            "field_angle",
            "deg",
            a.field_angle,
            b.field_angle,
            1.5,
        ),
        // CIE beam characteristics (2 metrics)
        metric(
            "Beam Angle (CIE)",
            "beam_angle_cie",
            "deg",
            a.beam_angle_cie,
            b.beam_angle_cie,
            1.5,
        ),
        metric(
            "Field Angle (CIE)",
            "field_angle_cie",
            "deg",
            a.field_angle_cie,
            b.field_angle_cie,
            1.0,
        ),
        // Upward beam characteristics (2 metrics)
        metric(
            "Upward Beam Angle",
            "upward_beam_angle",
            "deg",
            a.upward_beam_angle,
            b.upward_beam_angle,
            0.5,
        ),
        metric(
            "Upward Field Angle",
            "upward_field_angle",
            "deg",
            a.upward_field_angle,
            b.upward_field_angle,
            0.5,
        ),
        // Intensity statistics (3 metrics)
        metric(
            "Max Intensity",
            "max_intensity",
            "cd/klm",
            a.max_intensity,
            b.max_intensity,
            1.5,
        ),
        metric(
            "Min Intensity",
            "min_intensity",
            "cd/klm",
            a.min_intensity,
            b.min_intensity,
            0.5,
        ),
        metric(
            "Avg Intensity",
            "avg_intensity",
            "cd/klm",
            a.avg_intensity,
            b.avg_intensity,
            1.0,
        ),
        // Spacing criterion (2 metrics)
        metric(
            "Spacing C0",
            "spacing_c0",
            "",
            a.spacing_c0,
            b.spacing_c0,
            1.0,
        ),
        metric(
            "Spacing C90",
            "spacing_c90",
            "",
            a.spacing_c90,
            b.spacing_c90,
            1.0,
        ),
        // Downward zonal lumens (3 metrics)
        metric(
            "Zonal 0-30°",
            "zonal_0_30",
            "%",
            a.zonal_lumens.zone_0_30,
            b.zonal_lumens.zone_0_30,
            1.0,
        ),
        metric(
            "Zonal 30-60°",
            "zonal_30_60",
            "%",
            a.zonal_lumens.zone_30_60,
            b.zonal_lumens.zone_30_60,
            1.0,
        ),
        metric(
            "Zonal 60-90°",
            "zonal_60_90",
            "%",
            a.zonal_lumens.zone_60_90,
            b.zonal_lumens.zone_60_90,
            1.0,
        ),
        // Upper zonal lumens (3 metrics)
        metric(
            "Zonal 90-120°",
            "zonal_90_120",
            "%",
            a.zonal_lumens.zone_90_120,
            b.zonal_lumens.zone_90_120,
            0.5,
        ),
        metric(
            "Zonal 120-150°",
            "zonal_120_150",
            "%",
            a.zonal_lumens.zone_120_150,
            b.zonal_lumens.zone_120_150,
            0.5,
        ),
        metric(
            "Zonal 150-180°",
            "zonal_150_180",
            "%",
            a.zonal_lumens.zone_150_180,
            b.zonal_lumens.zone_150_180,
            0.5,
        ),
        // CIE flux codes (5 metrics)
        metric(
            "CIE N1",
            "cie_n1",
            "%",
            a.cie_flux_codes.n1,
            b.cie_flux_codes.n1,
            0.5,
        ),
        metric(
            "CIE N2",
            "cie_n2",
            "%",
            a.cie_flux_codes.n2,
            b.cie_flux_codes.n2,
            0.5,
        ),
        metric(
            "CIE N3",
            "cie_n3",
            "%",
            a.cie_flux_codes.n3,
            b.cie_flux_codes.n3,
            0.5,
        ),
        metric(
            "CIE N4",
            "cie_n4",
            "%",
            a.cie_flux_codes.n4,
            b.cie_flux_codes.n4,
            0.5,
        ),
        metric(
            "CIE N5",
            "cie_n5",
            "%",
            a.cie_flux_codes.n5,
            b.cie_flux_codes.n5,
            0.5,
        ),
    ]
}

#[cfg(feature = "i18n")]
fn metric_localized(
    fallback_name: &str,
    key: &str,
    unit: &str,
    a: f64,
    b: f64,
    weight: f64,
    locale: &eulumdat_i18n::Locale,
) -> ComparisonMetric {
    let name = locale
        .comparison_metric_name(key)
        .unwrap_or(fallback_name)
        .to_string();
    let delta = b - a;
    let delta_percent = if a.abs() > 1e-9 {
        (delta / a) * 100.0
    } else if b.abs() > 1e-9 {
        100.0
    } else {
        0.0
    };
    ComparisonMetric {
        name,
        key: key.to_string(),
        unit: unit.to_string(),
        value_a: a,
        value_b: b,
        delta,
        delta_percent,
        significance: Significance::from_delta_percent(delta_percent),
        weight,
    }
}

#[cfg(feature = "i18n")]
fn build_metrics_with_locale(
    a: &PhotometricSummary,
    b: &PhotometricSummary,
    locale: &eulumdat_i18n::Locale,
) -> Vec<ComparisonMetric> {
    vec![
        metric_localized(
            "Total Lamp Flux",
            "total_lamp_flux",
            "lm",
            a.total_lamp_flux,
            b.total_lamp_flux,
            2.0,
            locale,
        ),
        metric_localized(
            "Calculated Flux",
            "calculated_flux",
            "lm",
            a.calculated_flux,
            b.calculated_flux,
            2.0,
            locale,
        ),
        metric_localized("Light Output Ratio", "lor", "%", a.lor, b.lor, 1.5, locale),
        metric_localized("DLOR", "dlor", "%", a.dlor, b.dlor, 1.0, locale),
        metric_localized("ULOR", "ulor", "%", a.ulor, b.ulor, 1.0, locale),
        metric_localized(
            "Lamp Efficacy",
            "lamp_efficacy",
            "lm/W",
            a.lamp_efficacy,
            b.lamp_efficacy,
            2.0,
            locale,
        ),
        metric_localized(
            "Luminaire Efficacy",
            "luminaire_efficacy",
            "lm/W",
            a.luminaire_efficacy,
            b.luminaire_efficacy,
            2.0,
            locale,
        ),
        metric_localized(
            "Total Wattage",
            "total_wattage",
            "W",
            a.total_wattage,
            b.total_wattage,
            1.5,
            locale,
        ),
        metric_localized(
            "Beam Angle (IES)",
            "beam_angle",
            "deg",
            a.beam_angle,
            b.beam_angle,
            2.0,
            locale,
        ),
        metric_localized(
            "Field Angle (IES)",
            "field_angle",
            "deg",
            a.field_angle,
            b.field_angle,
            1.5,
            locale,
        ),
        metric_localized(
            "Beam Angle (CIE)",
            "beam_angle_cie",
            "deg",
            a.beam_angle_cie,
            b.beam_angle_cie,
            1.5,
            locale,
        ),
        metric_localized(
            "Field Angle (CIE)",
            "field_angle_cie",
            "deg",
            a.field_angle_cie,
            b.field_angle_cie,
            1.0,
            locale,
        ),
        metric_localized(
            "Upward Beam Angle",
            "upward_beam_angle",
            "deg",
            a.upward_beam_angle,
            b.upward_beam_angle,
            0.5,
            locale,
        ),
        metric_localized(
            "Upward Field Angle",
            "upward_field_angle",
            "deg",
            a.upward_field_angle,
            b.upward_field_angle,
            0.5,
            locale,
        ),
        metric_localized(
            "Max Intensity",
            "max_intensity",
            "cd/klm",
            a.max_intensity,
            b.max_intensity,
            1.5,
            locale,
        ),
        metric_localized(
            "Min Intensity",
            "min_intensity",
            "cd/klm",
            a.min_intensity,
            b.min_intensity,
            0.5,
            locale,
        ),
        metric_localized(
            "Avg Intensity",
            "avg_intensity",
            "cd/klm",
            a.avg_intensity,
            b.avg_intensity,
            1.0,
            locale,
        ),
        metric_localized(
            "Spacing C0",
            "spacing_c0",
            "",
            a.spacing_c0,
            b.spacing_c0,
            1.0,
            locale,
        ),
        metric_localized(
            "Spacing C90",
            "spacing_c90",
            "",
            a.spacing_c90,
            b.spacing_c90,
            1.0,
            locale,
        ),
        metric_localized(
            "Zonal 0-30°",
            "zonal_0_30",
            "%",
            a.zonal_lumens.zone_0_30,
            b.zonal_lumens.zone_0_30,
            1.0,
            locale,
        ),
        metric_localized(
            "Zonal 30-60°",
            "zonal_30_60",
            "%",
            a.zonal_lumens.zone_30_60,
            b.zonal_lumens.zone_30_60,
            1.0,
            locale,
        ),
        metric_localized(
            "Zonal 60-90°",
            "zonal_60_90",
            "%",
            a.zonal_lumens.zone_60_90,
            b.zonal_lumens.zone_60_90,
            1.0,
            locale,
        ),
        metric_localized(
            "Zonal 90-120°",
            "zonal_90_120",
            "%",
            a.zonal_lumens.zone_90_120,
            b.zonal_lumens.zone_90_120,
            0.5,
            locale,
        ),
        metric_localized(
            "Zonal 120-150°",
            "zonal_120_150",
            "%",
            a.zonal_lumens.zone_120_150,
            b.zonal_lumens.zone_120_150,
            0.5,
            locale,
        ),
        metric_localized(
            "Zonal 150-180°",
            "zonal_150_180",
            "%",
            a.zonal_lumens.zone_150_180,
            b.zonal_lumens.zone_150_180,
            0.5,
            locale,
        ),
        metric_localized(
            "CIE N1",
            "cie_n1",
            "%",
            a.cie_flux_codes.n1,
            b.cie_flux_codes.n1,
            0.5,
            locale,
        ),
        metric_localized(
            "CIE N2",
            "cie_n2",
            "%",
            a.cie_flux_codes.n2,
            b.cie_flux_codes.n2,
            0.5,
            locale,
        ),
        metric_localized(
            "CIE N3",
            "cie_n3",
            "%",
            a.cie_flux_codes.n3,
            b.cie_flux_codes.n3,
            0.5,
            locale,
        ),
        metric_localized(
            "CIE N4",
            "cie_n4",
            "%",
            a.cie_flux_codes.n4,
            b.cie_flux_codes.n4,
            0.5,
            locale,
        ),
        metric_localized(
            "CIE N5",
            "cie_n5",
            "%",
            a.cie_flux_codes.n5,
            b.cie_flux_codes.n5,
            0.5,
            locale,
        ),
    ]
}

fn compute_similarity(metrics: &[ComparisonMetric]) -> f64 {
    let total_weight: f64 = metrics.iter().map(|m| m.weight).sum();
    if total_weight == 0.0 {
        return 1.0;
    }
    let weighted_sum: f64 = metrics
        .iter()
        .map(|m| {
            let score = 1.0 - (m.delta_percent.abs() / 100.0).clamp(0.0, 1.0);
            score * m.weight
        })
        .sum();
    weighted_sum / total_weight
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_summary(flux: f64, lor: f64, beam: f64) -> PhotometricSummary {
        PhotometricSummary {
            total_lamp_flux: flux,
            calculated_flux: flux * lor / 100.0,
            lor,
            dlor: 70.0,
            ulor: 30.0,
            lamp_efficacy: 100.0,
            luminaire_efficacy: lor,
            total_wattage: flux / 100.0,
            beam_angle: beam,
            field_angle: beam * 2.0,
            max_intensity: 300.0,
            min_intensity: 10.0,
            avg_intensity: 150.0,
            spacing_c0: 1.2,
            spacing_c90: 1.3,
            ..Default::default()
        }
    }

    #[test]
    fn identical_files_have_score_one() {
        let s = make_summary(5000.0, 80.0, 60.0);
        let cmp = PhotometricComparison::from_summaries(s.clone(), s, "A", "B");
        assert!(
            (cmp.similarity_score - 1.0).abs() < 1e-6,
            "Expected ~1.0, got {}",
            cmp.similarity_score
        );
        for m in &cmp.metrics {
            assert!(m.delta.abs() < 1e-6, "Expected zero delta for {}", m.name);
            assert_eq!(m.significance, Significance::Negligible);
        }
    }

    #[test]
    fn different_files_have_lower_score() {
        let a = make_summary(5000.0, 80.0, 60.0);
        let b = make_summary(3000.0, 65.0, 90.0);
        let cmp = PhotometricComparison::from_summaries(a, b, "A", "B");
        assert!(cmp.similarity_score < 1.0);
        assert!(cmp.similarity_score > 0.0);
    }

    #[test]
    fn significance_thresholds() {
        assert_eq!(
            Significance::from_delta_percent(0.0),
            Significance::Negligible
        );
        assert_eq!(
            Significance::from_delta_percent(4.9),
            Significance::Negligible
        );
        assert_eq!(Significance::from_delta_percent(5.0), Significance::Minor);
        assert_eq!(Significance::from_delta_percent(14.9), Significance::Minor);
        assert_eq!(
            Significance::from_delta_percent(15.0),
            Significance::Moderate
        );
        assert_eq!(
            Significance::from_delta_percent(29.9),
            Significance::Moderate
        );
        assert_eq!(Significance::from_delta_percent(30.0), Significance::Major);
        assert_eq!(Significance::from_delta_percent(-50.0), Significance::Major);
    }

    #[test]
    fn text_output_contains_header() {
        let s = make_summary(5000.0, 80.0, 60.0);
        let cmp = PhotometricComparison::from_summaries(s.clone(), s, "Lamp A", "Lamp B");
        let text = cmp.to_text();
        assert!(text.contains("Lamp A"));
        assert!(text.contains("Lamp B"));
        assert!(text.contains("Similarity:"));
        assert!(text.contains("Total Lamp Flux"));
    }

    #[test]
    fn csv_output_has_header_row() {
        let s = make_summary(5000.0, 80.0, 60.0);
        let cmp = PhotometricComparison::from_summaries(s.clone(), s, "A", "B");
        let csv = cmp.to_csv();
        let lines: Vec<&str> = csv.lines().collect();
        assert!(lines[0].starts_with("metric,"));
        // 30 metrics (from build_metrics) + 1 header = 31 lines
        // Note: BUG + dimension metrics (6 more) only added via from_eulumdat()
        assert_eq!(lines.len(), 31);
    }

    #[test]
    fn significant_metrics_filter() {
        let a = make_summary(5000.0, 80.0, 60.0);
        let b = make_summary(3000.0, 65.0, 90.0);
        let cmp = PhotometricComparison::from_summaries(a, b, "A", "B");
        let major = cmp.significant_metrics(Significance::Major);
        assert!(!major.is_empty());
        for m in &major {
            assert_eq!(m.significance, Significance::Major);
        }
    }
}
