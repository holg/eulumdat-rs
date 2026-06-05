//! Brute-force layout optimizer for a given luminaire + compliance target.
//!
//! Given a base [`StreetLayout`] (lane count, width, overhang, tilt,
//! sidewalk, pole offset), a luminaire, and a way to check compliance for
//! a candidate design, this module searches the 3-parameter grid
//! `{pole_spacing_m, mounting_height_m, arrangement}` for configurations
//! that both **pass compliance** and **optimize a user-chosen cost**.
//!
//! Design choices:
//!
//! - **Brute force** on purpose — the search space is small (~300–800
//!   candidates with default bounds), each candidate is a `compute()`
//!   call that already runs in milliseconds, and a grid search is far
//!   easier to reason about than a gradient method when the feasibility
//!   region is discrete (arrangement is categorical).
//! - **Compliance as a callback** (`fit: impl Fn(&DesignResult) -> bool`)
//!   so callers compose whichever standard(s) apply — we don't bind the
//!   optimizer to a single `LightingStandard` impl.
//! - **Cost as an enum** so the `Display` of results is stable across
//!   callers and the intermediate metrics are easy to show in the UI.

use super::layout::{Arrangement, StreetLayout};
use crate::standards::DesignResult;
use crate::Eulumdat;

/// What the optimizer is minimizing (or maximizing) over passing configs.
///
/// Each variant maps to a `value(candidate) → f64` where **lower is
/// better** — safety-margin is negated internally so the same ordering
/// rule applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OptimizerObjective {
    /// Fewest poles per km of roadway. Cheapest install.
    PoleCountPerKm,
    /// Lowest total luminaire luminous flux per km. Best energy / BUG.
    TotalFluxPerKm,
    /// Highest `min/avg` uniformity — biggest safety margin above minimum.
    SafetyMargin,
}

impl OptimizerObjective {
    /// Human-readable name — kept stable so UI & tests can compare.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PoleCountPerKm => "Pole count per km",
            Self::TotalFluxPerKm => "Total flux per km",
            Self::SafetyMargin => "Safety margin (min/avg)",
        }
    }
}

/// Bounds over which the optimizer sweeps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OptimizerBounds {
    /// Inclusive range of pole spacings to try (m).
    pub pole_spacing_m: (f64, f64),
    /// Step size for the pole-spacing sweep (m).
    pub pole_spacing_step_m: f64,
    /// Inclusive range of mounting heights to try (m).
    pub mounting_height_m: (f64, f64),
    /// Step size for the mounting-height sweep (m).
    pub mounting_height_step_m: f64,
    /// Arrangements to consider. Empty = sweep all three.
    pub arrangements: &'static [Arrangement],
    /// Maintenance / light-loss factor applied to the grid compute.
    pub maintenance_factor: f64,
}

impl Default for OptimizerBounds {
    fn default() -> Self {
        Self {
            pole_spacing_m: (15.0, 60.0),
            pole_spacing_step_m: 5.0,
            mounting_height_m: (4.0, 15.0),
            mounting_height_step_m: 1.0,
            arrangements: &[
                Arrangement::SingleSide,
                Arrangement::Opposite,
                Arrangement::Staggered,
            ],
            maintenance_factor: 0.8,
        }
    }
}

/// One point in the optimizer's output — a passing configuration with
/// both its design metrics and the cost that drove ranking.
#[derive(Debug, Clone, PartialEq)]
pub struct OptimizationCandidate {
    pub pole_spacing_m: f64,
    pub mounting_height_m: f64,
    pub arrangement: Arrangement,
    /// Roadway-only design metrics (avg/min/max lux, uniformity).
    pub design: DesignResult,
    /// Raw cost value (lower = better). See [`OptimizerObjective`].
    pub cost: f64,
    /// Poles per km, precomputed for display regardless of objective.
    pub poles_per_km: f64,
    /// Total luminaire luminous flux per km, precomputed for display.
    pub flux_per_km: f64,
}

/// Run the optimizer.
///
/// `base` supplies every geometry field **except** the three the
/// optimizer sweeps. Its `pole_spacing_m`, `mounting_height_m`, and
/// `arrangement` are ignored (the sweep overrides them).
///
/// `fit` should return `true` for a passing [`DesignResult`] — e.g. a
/// closure that runs a `LightingStandard::check_design` and inspects
/// `result.passed()`. The optimizer keeps only passing candidates.
///
/// Returns the top `max_results` by objective (lowest cost first). An
/// empty return means nothing in the search space passed `fit`.
///
/// For layout-trade-off scatter plots and other downstream analyses
/// that need the *full* passing set, see [`optimize_layout_all`].
pub fn optimize_layout(
    ldc: &Eulumdat,
    base: &StreetLayout,
    bounds: &OptimizerBounds,
    objective: OptimizerObjective,
    max_results: usize,
    fit: impl Fn(&DesignResult) -> bool,
) -> Vec<OptimizationCandidate> {
    let mut passing = optimize_layout_all(ldc, base, bounds, objective, fit);
    passing.sort_by(|a, b| {
        a.cost
            .partial_cmp(&b.cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    passing.truncate(max_results);
    passing
}

/// Like [`optimize_layout`] but returns **every passing candidate** in
/// the search space, unsorted and untruncated. Each candidate carries
/// its design metrics, poles/km and flux/km — everything the
/// [`layout_tradeoff_chart`](super::svg::layout_tradeoff_chart) renderer needs to plot a
/// power-vs-quality trade-off.
pub fn optimize_layout_all(
    ldc: &Eulumdat,
    base: &StreetLayout,
    bounds: &OptimizerBounds,
    objective: OptimizerObjective,
    fit: impl Fn(&DesignResult) -> bool,
) -> Vec<OptimizationCandidate> {
    let arrangements = if bounds.arrangements.is_empty() {
        &[
            Arrangement::SingleSide,
            Arrangement::Opposite,
            Arrangement::Staggered,
        ][..]
    } else {
        bounds.arrangements
    };

    // Per-lamp-set total luminous flux is constant across candidates — we
    // pull it once so the per-km cost is cheap.
    let luminaire_flux_lm: f64 = ldc.lamp_sets.iter().map(|ls| ls.total_luminous_flux).sum();

    let spacings = float_range(
        bounds.pole_spacing_m.0,
        bounds.pole_spacing_m.1,
        bounds.pole_spacing_step_m,
    );
    let heights = float_range(
        bounds.mounting_height_m.0,
        bounds.mounting_height_m.1,
        bounds.mounting_height_step_m,
    );

    let mut passing: Vec<OptimizationCandidate> = Vec::new();

    for &arrangement in arrangements {
        for &spacing in &spacings {
            for &height in &heights {
                let mut candidate = base.clone();
                candidate.pole_spacing_m = spacing;
                candidate.mounting_height_m = height;
                candidate.arrangement = arrangement;

                let area = candidate.compute(ldc, bounds.maintenance_factor);
                let design = candidate.design_result(&area);
                if !fit(&design) {
                    continue;
                }

                let poles_per_km = poles_per_km(spacing, arrangement);
                let flux_per_km = poles_per_km * luminaire_flux_lm;

                let cost = match objective {
                    OptimizerObjective::PoleCountPerKm => poles_per_km,
                    OptimizerObjective::TotalFluxPerKm => flux_per_km,
                    OptimizerObjective::SafetyMargin => -design.uniformity_overall,
                };

                passing.push(OptimizationCandidate {
                    pole_spacing_m: spacing,
                    mounting_height_m: height,
                    arrangement,
                    design,
                    cost,
                    poles_per_km,
                    flux_per_km,
                });
            }
        }
    }

    passing
}

/// Compute the **Pareto front** of a candidate set against the
/// power-vs-quality trade-off used by the layout trade-off chart.
///
/// "Pareto-optimal" here means: there is no other candidate that has
/// **fewer poles per km AND higher average illuminance**. Ties on either
/// axis don't disqualify a candidate.
///
/// Returns the indices into `candidates` that lie on the front, sorted
/// by ascending poles-per-km. Empty for empty input.
pub fn pareto_front_tradeoff(candidates: &[OptimizationCandidate]) -> Vec<usize> {
    let mut front: Vec<usize> = (0..candidates.len()).collect();
    front.retain(|&i| {
        let a = &candidates[i];
        !candidates.iter().enumerate().any(|(j, b)| {
            i != j
                // b strictly dominates a: fewer poles AND higher lux,
                // with at least one of the two being strict.
                && b.poles_per_km <= a.poles_per_km
                && b.design.avg_illuminance_lux >= a.design.avg_illuminance_lux
                && (b.poles_per_km < a.poles_per_km
                    || b.design.avg_illuminance_lux > a.design.avg_illuminance_lux)
        })
    });
    front.sort_by(|&i, &j| {
        candidates[i]
            .poles_per_km
            .partial_cmp(&candidates[j].poles_per_km)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    front
}

/// Inclusive float range: `[lo, lo+step, lo+2*step, ..., hi]`.
fn float_range(lo: f64, hi: f64, step: f64) -> Vec<f64> {
    if step <= 0.0 || lo >= hi {
        return vec![lo];
    }
    let mut v = Vec::new();
    let mut x = lo;
    // Tolerance keeps the inclusive upper bound even with fp drift.
    while x <= hi + step * 0.001 {
        v.push((x * 1e6).round() / 1e6);
        x += step;
    }
    v
}

/// How many poles the layout installs per kilometer of roadway.
/// Opposite arrangement has two poles per spacing cycle; single/staggered
/// have one.
fn poles_per_km(spacing_m: f64, arrangement: Arrangement) -> f64 {
    let per_cycle = match arrangement {
        Arrangement::SingleSide => 1.0,
        Arrangement::Staggered => 1.0,
        Arrangement::Opposite => 2.0,
    };
    1000.0 / spacing_m * per_cycle
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::standards::{
        rp8::{PedestrianConflict, RoadClass, Rp8Selection, Rp8Standard},
        LightingStandard,
    };

    fn road_ldc() -> Eulumdat {
        let p = "../eulumdat-wasm/templates/road_luminaire.ldt";
        let content = std::fs::read_to_string(p).expect("template must exist");
        Eulumdat::parse(&content).expect("template must parse")
    }

    fn base_layout() -> StreetLayout {
        StreetLayout {
            length_m: 60.0,
            lane_width_m: 3.5,
            num_lanes: 2,
            pole_spacing_m: 30.0,
            arrangement: Arrangement::Staggered,
            mounting_height_m: 10.0,
            overhang_m: 1.0,
            tilt_deg: 0.0,
            pole_offset_m: 0.5,
            sidewalk_width_m: 0.0,
        }
    }

    #[test]
    fn poles_per_km_math_matches_arrangement_families() {
        assert!((poles_per_km(25.0, Arrangement::SingleSide) - 40.0).abs() < 1e-6);
        assert!((poles_per_km(25.0, Arrangement::Opposite) - 80.0).abs() < 1e-6);
        assert!((poles_per_km(25.0, Arrangement::Staggered) - 40.0).abs() < 1e-6);
    }

    #[test]
    fn float_range_is_inclusive_and_deterministic() {
        assert_eq!(float_range(15.0, 25.0, 5.0), vec![15.0, 20.0, 25.0]);
        assert_eq!(float_range(4.0, 4.0, 1.0), vec![4.0]);
    }

    #[test]
    fn pole_count_objective_prefers_longer_spacing() {
        let ldc = road_ldc();
        let base = base_layout();

        // Low-bar compliance check so we always have passing candidates in
        // the search space: RP-8 Local/Low requires only ≥4 lux, avg/min ≤ 6.
        let sel = Rp8Selection {
            road_class: RoadClass::Local,
            pedestrian_conflict: PedestrianConflict::Low,
        };
        let standard = Rp8Standard;
        let fit = |design: &DesignResult| {
            standard
                .check_design(&sel, design)
                .map(|r| r.passed())
                .unwrap_or(false)
        };

        let bounds = OptimizerBounds {
            // Keep the sweep cheap for tests.
            pole_spacing_m: (20.0, 40.0),
            pole_spacing_step_m: 10.0,
            mounting_height_m: (8.0, 10.0),
            mounting_height_step_m: 2.0,
            ..OptimizerBounds::default()
        };

        let top = optimize_layout(
            &ldc,
            &base,
            &bounds,
            OptimizerObjective::PoleCountPerKm,
            3,
            fit,
        );
        // At least one passing candidate from the grid.
        assert!(!top.is_empty(), "expected ≥1 passing candidate");
        // Results sorted by pole count ascending → first result's spacing
        // is the largest among passing configs (fewer poles per km).
        for w in top.windows(2) {
            assert!(
                w[0].poles_per_km <= w[1].poles_per_km + 1e-9,
                "results not sorted by pole count: {:?}",
                top
            );
        }
    }

    #[test]
    fn no_passing_means_empty_result() {
        let ldc = road_ldc();
        let base = base_layout();
        // Require impossibly high avg: 1 000 000 lux → nothing passes.
        let fit = |design: &DesignResult| design.avg_illuminance_lux > 1_000_000.0;
        let out = optimize_layout(
            &ldc,
            &base,
            &OptimizerBounds {
                pole_spacing_m: (30.0, 30.0),
                pole_spacing_step_m: 5.0,
                mounting_height_m: (10.0, 10.0),
                mounting_height_step_m: 1.0,
                ..OptimizerBounds::default()
            },
            OptimizerObjective::PoleCountPerKm,
            3,
            fit,
        );
        assert!(out.is_empty());
    }

    #[test]
    fn safety_margin_objective_inverts_ordering() {
        let ldc = road_ldc();
        let base = base_layout();
        let sel = Rp8Selection {
            road_class: RoadClass::Local,
            pedestrian_conflict: PedestrianConflict::Low,
        };
        let standard = Rp8Standard;
        let fit = |design: &DesignResult| {
            standard
                .check_design(&sel, design)
                .map(|r| r.passed())
                .unwrap_or(false)
        };
        let bounds = OptimizerBounds {
            pole_spacing_m: (20.0, 40.0),
            pole_spacing_step_m: 10.0,
            mounting_height_m: (8.0, 10.0),
            mounting_height_step_m: 2.0,
            ..OptimizerBounds::default()
        };

        let by_safety = optimize_layout(
            &ldc,
            &base,
            &bounds,
            OptimizerObjective::SafetyMargin,
            3,
            fit,
        );

        // Safety-margin ranking: first result has the highest
        // uniformity_overall (min/avg).
        for w in by_safety.windows(2) {
            assert!(
                w[0].design.uniformity_overall >= w[1].design.uniformity_overall - 1e-9,
                "safety results not sorted by uniformity descending: {:?}",
                by_safety
            );
        }
    }

    /// Hand-constructed candidate set so we can assert pareto behavior
    /// without going through a full optimizer run.
    fn synthetic_candidate(poles_per_km: f64, avg_lux: f64) -> OptimizationCandidate {
        OptimizationCandidate {
            pole_spacing_m: 1000.0 / poles_per_km,
            mounting_height_m: 10.0,
            arrangement: Arrangement::SingleSide,
            design: DesignResult {
                avg_illuminance_lux: avg_lux,
                min_illuminance_lux: avg_lux * 0.4,
                max_illuminance_lux: avg_lux * 1.5,
                avg_luminance_cd_m2: None,
                uniformity_overall: 0.4,
                uniformity_longitudinal: None,
                threshold_increment_pct: None,
            },
            cost: poles_per_km,
            poles_per_km,
            flux_per_km: poles_per_km * 10000.0,
        }
    }

    #[test]
    fn pareto_front_drops_dominated_candidates() {
        // Three candidates:
        //   A: 40 poles/km, 25 lux  (cheap, dim)
        //   B: 50 poles/km, 30 lux  (more poles AND more lux)
        //   C: 50 poles/km, 20 lux  (DOMINATED by A: more poles, less lux)
        let cands = vec![
            synthetic_candidate(40.0, 25.0),
            synthetic_candidate(50.0, 30.0),
            synthetic_candidate(50.0, 20.0),
        ];
        let front = pareto_front_tradeoff(&cands);
        assert_eq!(front.len(), 2, "expected 2 non-dominated, got {front:?}");
        // Sorted by ascending poles/km → A first, B second. C absent.
        assert_eq!(front, vec![0, 1]);
    }

    #[test]
    fn pareto_front_keeps_strict_lex_dominators_on_ties() {
        // Tie-breaking: equal poles/km, different lux. The brighter
        // candidate dominates the dimmer one.
        let cands = vec![
            synthetic_candidate(50.0, 30.0),
            synthetic_candidate(50.0, 25.0),
        ];
        let front = pareto_front_tradeoff(&cands);
        assert_eq!(front, vec![0]);
    }

    #[test]
    fn pareto_front_empty_input_returns_empty() {
        let front = pareto_front_tradeoff(&[]);
        assert!(front.is_empty());
    }
}
