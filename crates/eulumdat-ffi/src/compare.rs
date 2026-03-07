//! Photometric comparison FFI types and functions

use crate::diagram::Language;
use crate::types::{to_core_eulumdat, Eulumdat};

/// Significance level of a comparison metric difference
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SignificanceLevel {
    Negligible,
    Minor,
    Moderate,
    Major,
}

impl From<eulumdat::compare::Significance> for SignificanceLevel {
    fn from(s: eulumdat::compare::Significance) -> Self {
        match s {
            eulumdat::compare::Significance::Negligible => SignificanceLevel::Negligible,
            eulumdat::compare::Significance::Minor => SignificanceLevel::Minor,
            eulumdat::compare::Significance::Moderate => SignificanceLevel::Moderate,
            eulumdat::compare::Significance::Major => SignificanceLevel::Major,
        }
    }
}

/// A single comparison metric between two luminaires
#[derive(Debug, Clone, uniffi::Record)]
pub struct ComparisonMetricFfi {
    pub name: String,
    pub key: String,
    pub unit: String,
    pub value_a: f64,
    pub value_b: f64,
    pub delta: f64,
    pub delta_percent: f64,
    pub significance: SignificanceLevel,
}

impl From<&eulumdat::compare::ComparisonMetric> for ComparisonMetricFfi {
    fn from(m: &eulumdat::compare::ComparisonMetric) -> Self {
        Self {
            name: m.name.clone(),
            key: m.key.clone(),
            unit: m.unit.clone(),
            value_a: m.value_a,
            value_b: m.value_b,
            delta: m.delta,
            delta_percent: m.delta_percent,
            significance: m.significance.into(),
        }
    }
}

/// Result of comparing two luminaires photometrically
#[derive(Debug, Clone, uniffi::Record)]
pub struct PhotometricComparisonResult {
    pub label_a: String,
    pub label_b: String,
    pub metrics: Vec<ComparisonMetricFfi>,
    pub similarity_score: f64,
    pub csv: String,
    pub text_report: String,
}

/// Compare two luminaires and return photometric metrics
#[uniffi::export]
pub fn compare_photometric(
    ldt_a: &Eulumdat,
    ldt_b: &Eulumdat,
    label_a: String,
    label_b: String,
) -> PhotometricComparisonResult {
    let core_a = to_core_eulumdat(ldt_a);
    let core_b = to_core_eulumdat(ldt_b);
    let comparison =
        eulumdat::PhotometricComparison::from_eulumdat(&core_a, &core_b, &label_a, &label_b);
    PhotometricComparisonResult {
        label_a: comparison.label_a.clone(),
        label_b: comparison.label_b.clone(),
        metrics: comparison.metrics.iter().map(|m| m.into()).collect(),
        similarity_score: comparison.similarity_score,
        csv: comparison.to_csv(),
        text_report: comparison.to_text(),
    }
}

/// Compare two luminaires with localized metric names
#[uniffi::export]
pub fn compare_photometric_localized(
    ldt_a: &Eulumdat,
    ldt_b: &Eulumdat,
    label_a: String,
    label_b: String,
    language: Language,
) -> PhotometricComparisonResult {
    let core_a = to_core_eulumdat(ldt_a);
    let core_b = to_core_eulumdat(ldt_b);
    let locale = language.to_locale();
    let comparison = eulumdat::PhotometricComparison::from_eulumdat_with_locale(
        &core_a, &core_b, &label_a, &label_b, &locale,
    );
    PhotometricComparisonResult {
        label_a: comparison.label_a.clone(),
        label_b: comparison.label_b.clone(),
        metrics: comparison.metrics.iter().map(|m| m.into()).collect(),
        similarity_score: comparison.similarity_score,
        csv: comparison.to_csv(),
        text_report: comparison.to_text(),
    }
}
