//! Pluggable lighting-compliance standards.
//!
//! Every standard (US RP-8, EU EN 13201, China CJJ 45, IES MLO, etc.) implements
//! the [`LightingStandard`] trait. That lets the WASM street designer, CLI, and
//! library consumers dispatch against a region/standard without hard-coding
//! which tables to use.
//!
//! Two classes of standards:
//!
//! - **File-level** (e.g. [`mlo`]): input is a photometric file alone; no
//!   geometry required. Result says whether the luminaire fits a given zone.
//! - **Design-level** (e.g. future `rp8`, `en13201`, `cjj45`): input is a
//!   [`DesignResult`] computed over a street/room layout; result checks
//!   whether the illuminance/luminance grid meets the criteria.
//!
//! Both funnel into the same [`ComplianceResult`] so downstream UI code can
//! render a pass/fail panel regardless of region.

pub mod mlo;

use std::borrow::Cow;

/// Geographic region or standards body a compliance standard belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Region {
    /// North America: ANSI/IES RP-8, TM-15, MLO, RP-20.
    Us,
    /// Europe: DIN EN 13201, DIN 67523.
    Eu,
    /// China: GB/CJJ 45 urban road lighting.
    Cn,
    /// International framework standards (CIE 115, etc.).
    International,
    /// Municipality-specific or user-defined override table.
    Custom,
}

impl std::fmt::Display for Region {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Us => write!(f, "US"),
            Self::Eu => write!(f, "EU"),
            Self::Cn => write!(f, "CN"),
            Self::International => write!(f, "International"),
            Self::Custom => write!(f, "Custom"),
        }
    }
}

/// A single pass/fail row in a compliance result.
#[derive(Debug, Clone, PartialEq)]
pub struct ComplianceItem {
    /// Human-readable parameter name, e.g. `"Average Illuminance"`.
    pub parameter: String,
    /// Required threshold as text (unit-agnostic so we can show `"B ≤ 3"`
    /// next to `"≥ 13.0 lux"` in the same table).
    pub required: String,
    /// Achieved / measured value as text.
    pub achieved: String,
    /// True if `achieved` satisfies `required`.
    pub passed: bool,
}

/// Outcome of a compliance check. One row per criterion; overall pass is the
/// logical AND over all rows.
#[derive(Debug, Clone, PartialEq)]
pub struct ComplianceResult {
    /// Standard name, e.g. `"IES/IDA MLO"` or `"ANSI/IES RP-8 (Major/Medium)"`.
    pub standard: Cow<'static, str>,
    /// Region the standard belongs to.
    pub region: Region,
    /// Per-criterion breakdown.
    pub items: Vec<ComplianceItem>,
}

impl ComplianceResult {
    /// Overall pass = every item passed. An empty result passes vacuously;
    /// callers that require at least one criterion should check `items` length.
    pub fn passed(&self) -> bool {
        self.items.iter().all(|i| i.passed)
    }

    /// Count of failed items.
    pub fn failure_count(&self) -> usize {
        self.items.iter().filter(|i| !i.passed).count()
    }
}

/// Output of a design-level calculation (e.g. an illuminance grid produced by
/// a street designer). Design-level standards consume this to check compliance.
///
/// Placeholder — filled in by the `street` module when the layout-compute
/// pipeline lands. File-level standards (like MLO) ignore this entirely.
#[derive(Debug, Clone, Default)]
pub struct DesignResult {
    /// Average illuminance across the evaluation surface, in lux.
    pub avg_illuminance_lux: f64,
    /// Minimum illuminance across the evaluation surface, in lux.
    pub min_illuminance_lux: f64,
    /// Maximum illuminance across the evaluation surface, in lux.
    pub max_illuminance_lux: f64,
    /// Average luminance (cd/m²). `None` if the design didn't compute luminance
    /// (e.g. illuminance-only mode, or no R-table selected).
    pub avg_luminance_cd_m2: Option<f64>,
    /// Overall uniformity (min/avg for illuminance, or the standard's equivalent).
    pub uniformity_overall: f64,
    /// Longitudinal uniformity (min/avg along the line of sight). `None` if N/A.
    pub uniformity_longitudinal: Option<f64>,
    /// Threshold increment (veiling luminance as % of target). `None` if N/A.
    pub threshold_increment_pct: Option<f64>,
}

/// Unified interface for pluggable compliance standards.
///
/// Implementors define:
/// - `Selection` — a user-facing choice (road class, lighting zone, pedestrian
///   level, etc.) that picks which criteria table to use.
/// - `check_file` or `check_design` (or both) — the actual pass/fail logic.
///
/// A standard can refuse to check by returning a [`ComplianceResult`] with
/// an item whose `passed = false` and an explanatory message — never panic.
pub trait LightingStandard {
    /// User-facing selector (e.g. `Rp8Selection { road_class, ped_conflict }`).
    type Selection;

    /// Short display name shown in UI, e.g. `"IES MLO"`.
    fn name(&self) -> &'static str;

    /// Region this standard belongs to.
    fn region(&self) -> Region;

    /// Check a single photometric file against this standard. File-level
    /// standards (MLO, luminaire classifications) return a real result;
    /// design-level standards should return `None` to indicate "need a
    /// `DesignResult` — call `check_design`".
    fn check_file(
        &self,
        _selection: &Self::Selection,
        _ldt: &crate::Eulumdat,
    ) -> Option<ComplianceResult> {
        None
    }

    /// Check a computed design result against this standard. Design-level
    /// standards implement this; file-level standards return `None`.
    fn check_design(
        &self,
        _selection: &Self::Selection,
        _design: &DesignResult,
    ) -> Option<ComplianceResult> {
        None
    }
}
