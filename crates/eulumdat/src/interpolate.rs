//! LED operating point interpolation.
//!
//! Interpolate photometric data between measurements at different operating
//! points (e.g., LED driver currents 350 mA vs 700 mA, or different CCTs).
//!
//! Given two or more files measured on the same luminaire at known operating
//! values, this module produces new `Eulumdat` data at arbitrary intermediate
//! (or extrapolated) points using piecewise linear interpolation.
//!
//! ## What gets interpolated
//!
//! - Intensity values (every cell in the C×G grid)
//! - Luminous flux, wattage, light output ratio
//! - Downward flux fraction, direct ratios
//!
//! ## What stays constant
//!
//! - Angle grids (c_angles, g_angles), symmetry, dimensions
//! - Lamp type, color appearance, color rendering group
//!
//! ## Example
//!
//! ```no_run
//! use eulumdat::{Eulumdat, interpolate};
//!
//! let a = Eulumdat::from_file("fixture_350mA.ies").unwrap();
//! let b = Eulumdat::from_file("fixture_700mA.ies").unwrap();
//!
//! // Interpolate at 50% between the two
//! let mid = interpolate::interpolate_eulumdat(&a, &b, 0.5).unwrap();
//!
//! // Generate a series at specific operating points
//! let inputs = vec![(a, 350.0), (b, 700.0)];
//! let targets = vec![400.0, 500.0, 600.0];
//! let series = interpolate::generate_series(&inputs, &targets).unwrap();
//! ```

use crate::Eulumdat;
use std::fmt;

/// Errors specific to interpolation.
#[derive(Debug)]
pub enum InterpolateError {
    /// C-plane angle grids differ between inputs.
    MismatchedCAngles { a: usize, b: usize },
    /// G-plane angle grids differ between inputs.
    MismatchedGAngles { a: usize, b: usize },
    /// LampSet counts differ.
    MismatchedLampSets { a: usize, b: usize },
    /// Need at least 2 input files.
    InsufficientInputs,
    /// Target value is outside the input range (extrapolation).
    Extrapolation { target: f64, min: f64, max: f64 },
}

impl fmt::Display for InterpolateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MismatchedCAngles { a, b } => {
                write!(f, "C-angle grids differ: {a} vs {b} planes")
            }
            Self::MismatchedGAngles { a, b } => {
                write!(f, "G-angle grids differ: {a} vs {b} angles")
            }
            Self::MismatchedLampSets { a, b } => {
                write!(f, "LampSet count differs: {a} vs {b}")
            }
            Self::InsufficientInputs => {
                write!(f, "need at least 2 input files for interpolation")
            }
            Self::Extrapolation { target, min, max } => {
                write!(
                    f,
                    "target {target} is outside input range [{min}..{max}] (extrapolating)"
                )
            }
        }
    }
}

impl std::error::Error for InterpolateError {}

/// Linearly interpolate between two `Eulumdat` datasets.
///
/// `t` = 0.0 returns a copy of `a`, `t` = 1.0 returns a copy of `b`.
/// Values outside [0, 1] extrapolate (intensities are clamped to ≥ 0).
///
/// The angle grids must match (same number and values of C/G angles).
/// Metadata (name, geometry, lamp type, color) is taken from `a`.
pub fn interpolate_eulumdat(
    a: &Eulumdat,
    b: &Eulumdat,
    t: f64,
) -> Result<Eulumdat, InterpolateError> {
    // Validate compatibility
    if a.c_angles.len() != b.c_angles.len() || a.intensities.len() != b.intensities.len() {
        return Err(InterpolateError::MismatchedCAngles {
            a: a.c_angles.len(),
            b: b.c_angles.len(),
        });
    }
    if a.g_angles.len() != b.g_angles.len() {
        return Err(InterpolateError::MismatchedGAngles {
            a: a.g_angles.len(),
            b: b.g_angles.len(),
        });
    }
    if a.lamp_sets.len() != b.lamp_sets.len() {
        return Err(InterpolateError::MismatchedLampSets {
            a: a.lamp_sets.len(),
            b: b.lamp_sets.len(),
        });
    }

    let inv_t = 1.0 - t;
    let mut result = a.clone();

    // Interpolate intensities (the core photometric data)
    for (c, plane_a) in a.intensities.iter().enumerate() {
        if c >= b.intensities.len() {
            break;
        }
        let plane_b = &b.intensities[c];
        let plane_out = &mut result.intensities[c];
        for (g, &val_a) in plane_a.iter().enumerate() {
            if g < plane_b.len() {
                let val = val_a * inv_t + plane_b[g] * t;
                plane_out[g] = val.max(0.0); // clamp negative from extrapolation
            }
        }
    }

    // Interpolate lamp set values
    for (i, ls) in result.lamp_sets.iter_mut().enumerate() {
        let ls_b = &b.lamp_sets[i];
        ls.total_luminous_flux = lerp(
            a.lamp_sets[i].total_luminous_flux,
            ls_b.total_luminous_flux,
            t,
        );
        ls.wattage_with_ballast = lerp(
            a.lamp_sets[i].wattage_with_ballast,
            ls_b.wattage_with_ballast,
            t,
        );
    }

    // Interpolate optical properties
    result.light_output_ratio = lerp(a.light_output_ratio, b.light_output_ratio, t);
    result.downward_flux_fraction = lerp(a.downward_flux_fraction, b.downward_flux_fraction, t);
    result.conversion_factor = lerp(a.conversion_factor, b.conversion_factor, t);

    // Interpolate direct ratios
    for i in 0..10 {
        result.direct_ratios[i] = lerp(a.direct_ratios[i], b.direct_ratios[i], t);
    }

    Ok(result)
}

/// Generate a series of interpolated files at specific target operating values.
///
/// `inputs` must contain at least 2 entries, sorted or unsorted — they will be
/// sorted internally by operating value.  Each entry is `(Eulumdat, value)`.
///
/// For targets within the input range, piecewise linear interpolation is used.
/// Targets outside the range are extrapolated from the nearest two endpoints.
///
/// Returns `(target_value, interpolated_eulumdat)` for each target.
pub fn generate_series(
    inputs: &[(Eulumdat, f64)],
    targets: &[f64],
) -> Result<Vec<(f64, Eulumdat)>, InterpolateError> {
    if inputs.len() < 2 {
        return Err(InterpolateError::InsufficientInputs);
    }

    // Sort by operating value
    let mut sorted: Vec<(usize, f64)> = inputs
        .iter()
        .enumerate()
        .map(|(i, (_, v))| (i, *v))
        .collect();
    sorted.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    let mut results = Vec::with_capacity(targets.len());

    for &target in targets {
        // Find bracketing pair
        let (lower_idx, upper_idx, t) = find_bracket(&sorted, target);

        let a = &inputs[sorted[lower_idx].0].0;
        let b = &inputs[sorted[upper_idx].0].0;

        let interpolated = interpolate_eulumdat(a, b, t)?;
        results.push((target, interpolated));
    }

    Ok(results)
}

/// Generate `count` evenly spaced values from `start` to `end` (inclusive).
pub fn linspace(start: f64, end: f64, count: usize) -> Vec<f64> {
    if count <= 1 {
        return vec![start];
    }
    let step = (end - start) / (count - 1) as f64;
    (0..count).map(|i| start + i as f64 * step).collect()
}

/// Format an operating-point value for filenames: integers stay as-is,
/// floats get one decimal place.
pub fn format_value(v: f64) -> String {
    if (v - v.round()).abs() < 1e-6 {
        format!("{}", v as i64)
    } else {
        format!("{:.1}", v)
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a * (1.0 - t) + b * t
}

/// Find the two bracketing indices in `sorted` for `target` and the t parameter.
fn find_bracket(sorted: &[(usize, f64)], target: f64) -> (usize, usize, f64) {
    let n = sorted.len();

    // Below range → extrapolate from first two
    if target <= sorted[0].1 {
        let span = sorted[1].1 - sorted[0].1;
        let t = if span.abs() > 1e-12 {
            (target - sorted[0].1) / span
        } else {
            0.0
        };
        return (0, 1, t);
    }

    // Above range → extrapolate from last two
    if target >= sorted[n - 1].1 {
        let span = sorted[n - 1].1 - sorted[n - 2].1;
        let t = if span.abs() > 1e-12 {
            (target - sorted[n - 2].1) / span
        } else {
            1.0
        };
        return (n - 2, n - 1, t);
    }

    // Find bracket
    for i in 0..n - 1 {
        if target >= sorted[i].1 && target <= sorted[i + 1].1 {
            let span = sorted[i + 1].1 - sorted[i].1;
            let t = if span.abs() > 1e-12 {
                (target - sorted[i].1) / span
            } else {
                0.0
            };
            return (i, i + 1, t);
        }
    }

    // Fallback (shouldn't reach)
    (n - 2, n - 1, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LampSet;

    fn make_test_ldt(intensity: f64, flux: f64, wattage: f64) -> Eulumdat {
        Eulumdat {
            c_angles: vec![0.0, 90.0, 180.0, 270.0],
            g_angles: vec![0.0, 30.0, 60.0, 90.0],
            intensities: vec![
                vec![intensity, intensity * 0.8, intensity * 0.4, 0.0],
                vec![intensity, intensity * 0.7, intensity * 0.3, 0.0],
                vec![intensity, intensity * 0.8, intensity * 0.4, 0.0],
                vec![intensity, intensity * 0.7, intensity * 0.3, 0.0],
            ],
            lamp_sets: vec![LampSet {
                num_lamps: 1,
                total_luminous_flux: flux,
                wattage_with_ballast: wattage,
                ..Default::default()
            }],
            light_output_ratio: 85.0,
            downward_flux_fraction: 70.0,
            ..Default::default()
        }
    }

    #[test]
    fn t0_returns_a() {
        let a = make_test_ldt(300.0, 5000.0, 50.0);
        let b = make_test_ldt(600.0, 10000.0, 100.0);
        let result = interpolate_eulumdat(&a, &b, 0.0).unwrap();

        assert!((result.intensities[0][0] - 300.0).abs() < 1e-6);
        assert!((result.lamp_sets[0].total_luminous_flux - 5000.0).abs() < 1e-6);
        assert!((result.lamp_sets[0].wattage_with_ballast - 50.0).abs() < 1e-6);
    }

    #[test]
    fn t1_returns_b() {
        let a = make_test_ldt(300.0, 5000.0, 50.0);
        let b = make_test_ldt(600.0, 10000.0, 100.0);
        let result = interpolate_eulumdat(&a, &b, 1.0).unwrap();

        assert!((result.intensities[0][0] - 600.0).abs() < 1e-6);
        assert!((result.lamp_sets[0].total_luminous_flux - 10000.0).abs() < 1e-6);
        assert!((result.lamp_sets[0].wattage_with_ballast - 100.0).abs() < 1e-6);
    }

    #[test]
    fn t05_averages() {
        let a = make_test_ldt(300.0, 5000.0, 50.0);
        let b = make_test_ldt(600.0, 10000.0, 100.0);
        let result = interpolate_eulumdat(&a, &b, 0.5).unwrap();

        assert!((result.intensities[0][0] - 450.0).abs() < 1e-6);
        assert!((result.lamp_sets[0].total_luminous_flux - 7500.0).abs() < 1e-6);
        assert!((result.lamp_sets[0].wattage_with_ballast - 75.0).abs() < 1e-6);
        assert!((result.light_output_ratio - 85.0).abs() < 1e-6);
    }

    #[test]
    fn mismatched_grids_error() {
        let a = make_test_ldt(300.0, 5000.0, 50.0);
        let mut b = make_test_ldt(600.0, 10000.0, 100.0);
        b.g_angles.push(120.0); // add extra angle
        for plane in &mut b.intensities {
            plane.push(0.0);
        }

        assert!(interpolate_eulumdat(&a, &b, 0.5).is_err());
    }

    #[test]
    fn series_generation() {
        let a = make_test_ldt(300.0, 5000.0, 50.0);
        let b = make_test_ldt(600.0, 10000.0, 100.0);

        let inputs = vec![(a, 350.0), (b, 700.0)];
        let targets = vec![350.0, 525.0, 700.0];
        let series = generate_series(&inputs, &targets).unwrap();

        assert_eq!(series.len(), 3);
        // At 350 → should be file A
        assert!((series[0].1.intensities[0][0] - 300.0).abs() < 1e-6);
        // At 525 → midpoint
        assert!((series[1].1.intensities[0][0] - 450.0).abs() < 1e-6);
        // At 700 → should be file B
        assert!((series[2].1.intensities[0][0] - 600.0).abs() < 1e-6);
    }

    #[test]
    fn three_point_interpolation() {
        let a = make_test_ldt(200.0, 3000.0, 30.0);
        let b = make_test_ldt(400.0, 6000.0, 60.0);
        let c = make_test_ldt(500.0, 8000.0, 90.0);

        let inputs = vec![(a, 200.0), (b, 400.0), (c, 600.0)];
        let targets = vec![300.0, 500.0];
        let series = generate_series(&inputs, &targets).unwrap();

        // At 300 → midpoint of A-B segment
        assert!((series[0].1.intensities[0][0] - 300.0).abs() < 1e-6);
        // At 500 → midpoint of B-C segment
        assert!((series[1].1.intensities[0][0] - 450.0).abs() < 1e-6);
    }

    #[test]
    fn linspace_generates_correct_values() {
        let vals = linspace(350.0, 700.0, 8);
        assert_eq!(vals.len(), 8);
        assert!((vals[0] - 350.0).abs() < 1e-6);
        assert!((vals[7] - 700.0).abs() < 1e-6);
        assert!((vals[1] - 400.0).abs() < 1e-6); // 350 + 50
    }

    #[test]
    fn format_value_integers() {
        assert_eq!(format_value(350.0), "350");
        assert_eq!(format_value(700.0), "700");
    }

    #[test]
    fn format_value_decimals() {
        assert_eq!(format_value(350.5), "350.5");
    }

    #[test]
    fn extrapolation_clamps_negative_intensities() {
        let a = make_test_ldt(300.0, 5000.0, 50.0);
        let b = make_test_ldt(100.0, 2000.0, 20.0);
        // t = 2.0 → extrapolate beyond b → could go negative
        let result = interpolate_eulumdat(&a, &b, 2.0).unwrap();
        // All intensities should be >= 0
        for plane in &result.intensities {
            for &val in plane {
                assert!(val >= 0.0, "intensity should be clamped to >= 0");
            }
        }
    }
}
