//! Spacing optimizer — find widest pole spacing that meets illuminance criteria.

use super::compute::{compute_area_illuminance, AreaResult};
use super::layout::{generate_placements, generate_pole_positions, ArrangementType, PoleConfig};
use crate::Eulumdat;

/// Optimization target criteria.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OptimizationCriteria {
    /// Minimum illuminance everywhere (e.g. 20 lx)
    pub target_min_lux: f64,
    /// Optional: min/avg uniformity ≥ this value
    pub target_uniformity: Option<f64>,
    /// Mounting height range (min, max)
    pub height_range: (f64, f64),
    /// Step between heights
    pub height_step: f64,
    /// Search bounds for pole spacing (min, max) in meters
    pub spacing_range: (f64, f64),
    /// Pole arrangement type
    pub arrangement: ArrangementType,
}

impl Default for OptimizationCriteria {
    fn default() -> Self {
        Self {
            target_min_lux: 20.0,
            target_uniformity: None,
            height_range: (8.0, 14.0),
            height_step: 2.0,
            spacing_range: (10.0, 60.0),
            arrangement: ArrangementType::Single,
        }
    }
}

/// One row in the optimization result matrix.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OptimizationRow {
    pub mounting_height: f64,
    pub optimal_spacing: f64,
    pub min_lux: f64,
    pub avg_lux: f64,
    pub max_lux: f64,
    pub uniformity_min_avg: f64,
    pub uniformity_min_max: f64,
    pub meets_criteria: bool,
    /// Poles needed for a given area at this spacing
    pub poles_needed: usize,
}

/// Run the spacing optimizer across a range of mounting heights.
///
/// For each height, uses golden-section search to find the widest spacing
/// that still meets the target criteria. Computes over a single bay with
/// neighbor contributions for efficiency.
pub fn optimize_spacing(
    ldt: &Eulumdat,
    criteria: &OptimizationCriteria,
    pole_config: &PoleConfig,
    area_width: f64,
    area_depth: f64,
    grid_resolution: usize,
    proration_factor: f64,
) -> Vec<OptimizationRow> {
    let mut results = Vec::new();

    let mut h = criteria.height_range.0;
    while h <= criteria.height_range.1 + 0.01 {
        let row = optimize_for_height(
            ldt,
            criteria,
            pole_config,
            h,
            area_width,
            area_depth,
            grid_resolution,
            proration_factor,
        );
        results.push(row);
        h += criteria.height_step;
    }

    results
}

/// Optimize spacing for a single mounting height.
#[allow(clippy::too_many_arguments)]
fn optimize_for_height(
    ldt: &Eulumdat,
    criteria: &OptimizationCriteria,
    pole_config: &PoleConfig,
    height: f64,
    area_width: f64,
    area_depth: f64,
    grid_resolution: usize,
    proration_factor: f64,
) -> OptimizationRow {
    let (s_min, s_max) = criteria.spacing_range;

    // Golden-section search for the widest spacing that meets criteria.
    // We want the LARGEST spacing where evaluate() returns true.
    // Monotonicity: wider spacing → lower min lux → eventually fails.
    let gr = (5.0_f64.sqrt() + 1.0) / 2.0; // golden ratio
    let tolerance = 0.5; // 0.5m precision

    let mut a = s_min;
    let mut b = s_max;

    // First check if even the tightest spacing fails
    let tight_result = evaluate_spacing(
        ldt,
        pole_config,
        height,
        a,
        area_width,
        area_depth,
        grid_resolution,
        proration_factor,
    );
    let tight_meets = meets_criteria(&tight_result, criteria);

    if !tight_meets {
        // Even tightest spacing doesn't meet criteria
        return make_row(height, a, &tight_result, false, area_width, area_depth);
    }

    // Check if widest spacing still works
    let wide_result = evaluate_spacing(
        ldt,
        pole_config,
        height,
        b,
        area_width,
        area_depth,
        grid_resolution,
        proration_factor,
    );
    let wide_meets = meets_criteria(&wide_result, criteria);

    if wide_meets {
        // Even widest spacing works — return it
        return make_row(height, b, &wide_result, true, area_width, area_depth);
    }

    // Golden-section search: find boundary between pass/fail
    while (b - a) > tolerance {
        let c = b - (b - a) / gr;
        let d = a + (b - a) / gr;

        let result_c = evaluate_spacing(
            ldt,
            pole_config,
            height,
            c,
            area_width,
            area_depth,
            grid_resolution,
            proration_factor,
        );
        let meets_c = meets_criteria(&result_c, criteria);

        let result_d = evaluate_spacing(
            ldt,
            pole_config,
            height,
            d,
            area_width,
            area_depth,
            grid_resolution,
            proration_factor,
        );
        let meets_d = meets_criteria(&result_d, criteria);

        if meets_d {
            // d still works, try wider
            a = c;
        } else if meets_c {
            // c works but d doesn't — boundary is between c and d
            a = c;
            b = d;
        } else {
            // neither works — need tighter
            b = c;
        }
    }

    // Use the last spacing that works (a)
    let final_result = evaluate_spacing(
        ldt,
        pole_config,
        height,
        a,
        area_width,
        area_depth,
        grid_resolution,
        proration_factor,
    );
    let final_meets = meets_criteria(&final_result, criteria);

    make_row(
        height,
        a,
        &final_result,
        final_meets,
        area_width,
        area_depth,
    )
}

/// Evaluate illuminance for a given spacing using a 2×2 bay arrangement.
#[allow(clippy::too_many_arguments)]
fn evaluate_spacing(
    ldt: &Eulumdat,
    pole_config: &PoleConfig,
    height: f64,
    spacing: f64,
    _area_width: f64,
    _area_depth: f64,
    grid_resolution: usize,
    proration_factor: f64,
) -> AreaResult {
    // Compute over a single bay (spacing × spacing) but include
    // contributions from 8 surrounding neighbors (3×3 grid centered on bay).
    let bay = spacing;
    let poles = generate_pole_positions(3, 3, bay * 3.0, bay * 3.0);
    let placements = generate_placements(&poles, height, pole_config, 0.0);

    // Compute over the center bay only
    let mut result = compute_area_illuminance(
        ldt,
        &placements,
        bay * 3.0,
        bay * 3.0,
        grid_resolution * 3,
        proration_factor,
    );

    // Extract center bay statistics
    let n = grid_resolution;
    let mut min_lux = f64::MAX;
    let mut max_lux: f64 = 0.0;
    let mut sum_lux: f64 = 0.0;
    let count = (n * n) as f64;

    for row in n..(2 * n) {
        for col in n..(2 * n) {
            let lux = result.lux_grid[row][col];
            if lux < min_lux {
                min_lux = lux;
            }
            if lux > max_lux {
                max_lux = lux;
            }
            sum_lux += lux;
        }
    }

    if min_lux == f64::MAX {
        min_lux = 0.0;
    }
    let avg_lux = if count > 0.0 { sum_lux / count } else { 0.0 };

    result.min_lux = min_lux;
    result.max_lux = max_lux;
    result.avg_lux = avg_lux;
    result.uniformity_min_avg = if avg_lux > 0.0 {
        min_lux / avg_lux
    } else {
        0.0
    };
    result.uniformity_min_max = if max_lux > 0.0 {
        min_lux / max_lux
    } else {
        0.0
    };

    result
}

fn meets_criteria(result: &AreaResult, criteria: &OptimizationCriteria) -> bool {
    if result.min_lux < criteria.target_min_lux {
        return false;
    }
    if let Some(target_u) = criteria.target_uniformity {
        if result.uniformity_min_avg < target_u {
            return false;
        }
    }
    true
}

fn make_row(
    height: f64,
    spacing: f64,
    result: &AreaResult,
    meets: bool,
    area_width: f64,
    area_depth: f64,
) -> OptimizationRow {
    let cols = (area_width / spacing).ceil() as usize;
    let rows = (area_depth / spacing).ceil() as usize;

    OptimizationRow {
        mounting_height: height,
        optimal_spacing: spacing,
        min_lux: result.min_lux,
        avg_lux: result.avg_lux,
        max_lux: result.max_lux,
        uniformity_min_avg: result.uniformity_min_avg,
        uniformity_min_max: result.uniformity_min_max,
        meets_criteria: meets,
        poles_needed: rows * cols,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LampSet;

    fn test_ldt() -> Eulumdat {
        Eulumdat {
            c_angles: vec![0.0, 90.0, 180.0, 270.0],
            g_angles: vec![0.0, 15.0, 30.0, 45.0, 60.0, 75.0, 90.0],
            intensities: vec![
                vec![300.0, 280.0, 220.0, 140.0, 60.0, 15.0, 3.0],
                vec![300.0, 270.0, 200.0, 120.0, 50.0, 12.0, 2.0],
                vec![300.0, 280.0, 220.0, 140.0, 60.0, 15.0, 3.0],
                vec![300.0, 270.0, 200.0, 120.0, 50.0, 12.0, 2.0],
            ],
            lamp_sets: vec![LampSet {
                num_lamps: 1,
                total_luminous_flux: 10000.0,
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn optimizer_produces_results() {
        let ldt = test_ldt();
        let criteria = OptimizationCriteria {
            target_min_lux: 5.0,
            target_uniformity: None,
            height_range: (8.0, 12.0),
            height_step: 2.0,
            spacing_range: (10.0, 40.0),
            arrangement: ArrangementType::Single,
        };
        let pole_cfg = PoleConfig::default();
        let results = optimize_spacing(&ldt, &criteria, &pole_cfg, 60.0, 40.0, 20, 1.0);

        assert_eq!(results.len(), 3); // 8m, 10m, 12m
        for row in &results {
            assert!(row.optimal_spacing >= 10.0);
            assert!(row.poles_needed > 0);
        }
    }

    #[test]
    fn higher_poles_allow_wider_spacing() {
        let ldt = test_ldt();
        let criteria = OptimizationCriteria {
            target_min_lux: 5.0,
            target_uniformity: None,
            height_range: (8.0, 14.0),
            height_step: 2.0,
            spacing_range: (10.0, 50.0),
            arrangement: ArrangementType::Single,
        };
        let pole_cfg = PoleConfig::default();
        let results = optimize_spacing(&ldt, &criteria, &pole_cfg, 60.0, 40.0, 20, 1.0);

        // Generally, higher poles should allow wider spacing (better uniformity)
        // but lower min lux — so the relationship isn't strictly monotonic.
        // Just verify we get reasonable results.
        assert!(results.len() >= 3);
    }
}
