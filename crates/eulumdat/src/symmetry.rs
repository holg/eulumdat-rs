//! Symmetry handling for Eulumdat data.
//!
//! This module provides utilities for working with symmetric luminous intensity distributions.
//! Symmetry can significantly reduce the amount of data needed to represent a luminaire.

use crate::eulumdat::{Eulumdat, Symmetry};

/// Handler for symmetry-based operations on photometric data.
pub struct SymmetryHandler;

impl SymmetryHandler {
    /// Fold any C angle into the domain of the stored planes for `symmetry`
    /// (0–180 for C0–C180, 90–270 for C90–C270, 0–90 for both planes).
    pub fn fold_c(symmetry: Symmetry, c_angle: f64) -> f64 {
        let c = c_angle.rem_euclid(360.0);
        match symmetry {
            Symmetry::None => c,
            Symmetry::VerticalAxis => 0.0,
            Symmetry::PlaneC0C180 => {
                if c <= 180.0 {
                    c
                } else {
                    360.0 - c
                }
            }
            Symmetry::PlaneC90C270 => {
                if (90.0..=270.0).contains(&c) {
                    c
                } else {
                    (180.0 - c).rem_euclid(360.0)
                }
            }
            Symmetry::BothPlanes => {
                let half = if c <= 180.0 { c } else { 360.0 - c };
                if half <= 90.0 {
                    half
                } else {
                    180.0 - half
                }
            }
        }
    }

    /// Angles of the stored intensity planes, one per row of `intensities`.
    ///
    /// EULUMDAT files list all `Nc` C angles but store only the `Mc`
    /// planes of the symmetric part; this picks those angles out of the
    /// list (0–180, 90–270 or 0–90), falling back to the first
    /// `intensities.len()` angles when the list is already reduced.
    pub fn stored_c_angles(eulumdat: &Eulumdat) -> Vec<f64> {
        let n = eulumdat.intensities.len();
        let in_domain = |a: &f64| match eulumdat.symmetry {
            Symmetry::None => true,
            Symmetry::VerticalAxis => false,
            Symmetry::PlaneC0C180 => *a <= 180.0 + 1e-9,
            Symmetry::PlaneC90C270 => (90.0 - 1e-9..=270.0 + 1e-9).contains(a),
            Symmetry::BothPlanes => *a <= 90.0 + 1e-9,
        };
        if eulumdat.symmetry == Symmetry::VerticalAxis {
            return vec![eulumdat.c_angles.first().copied().unwrap_or(0.0)];
        }
        let filtered: Vec<f64> = eulumdat
            .c_angles
            .iter()
            .copied()
            .filter(in_domain)
            .collect();
        if n == 0 || filtered.len() == n {
            filtered
        } else if eulumdat.c_angles.len() >= n {
            eulumdat.c_angles[..n].to_vec()
        } else {
            eulumdat.c_angles.clone()
        }
    }

    /// Get the C-plane angles for the full 360° distribution.
    ///
    /// When the file lists every plane (the normal case: `Nc` angles even
    /// for symmetric files) that list is returned as is. A reduced list
    /// (only the stored planes) is mirrored without duplicates.
    pub fn expand_c_angles(eulumdat: &Eulumdat) -> Vec<f64> {
        let ca = &eulumdat.c_angles;
        let complete = |list: &[f64]| {
            list.len() >= eulumdat.num_c_planes.max(2) || list.last().is_some_and(|l| *l > 270.0)
        };
        match eulumdat.symmetry {
            Symmetry::None => ca.clone(),
            Symmetry::VerticalAxis => {
                if complete(ca) {
                    ca.clone()
                } else {
                    (0..eulumdat.num_c_planes)
                        .map(|i| i as f64 * eulumdat.c_plane_distance)
                        .collect()
                }
            }
            sym => {
                if complete(ca) {
                    return ca.clone();
                }
                let stored = Self::stored_c_angles(eulumdat);
                let mut all: Vec<f64> = Vec::new();
                for &q in &stored {
                    let images: &[f64] = match sym {
                        Symmetry::PlaneC0C180 => &[q, 360.0 - q],
                        Symmetry::PlaneC90C270 => &[q, 180.0 - q],
                        _ => &[q, 180.0 - q, 180.0 + q, 360.0 - q],
                    };
                    for &a in images {
                        let a = a.rem_euclid(360.0);
                        if !all.iter().any(|x| (x - a).abs() < 1e-6) {
                            all.push(a);
                        }
                    }
                }
                all.sort_by(|x, y| x.partial_cmp(y).unwrap());
                all
            }
        }
    }

    /// Expand symmetric data to the full 360° distribution.
    ///
    /// Returns one intensity row per angle of [`Self::expand_c_angles`], so
    /// the two always line up. Each full angle is folded into the stored
    /// domain and takes the nearest stored plane.
    pub fn expand_to_full(eulumdat: &Eulumdat) -> Vec<Vec<f64>> {
        if eulumdat.symmetry == Symmetry::None || eulumdat.intensities.is_empty() {
            return eulumdat.intensities.clone();
        }
        let stored = Self::stored_c_angles(eulumdat);
        if stored.is_empty() {
            return Vec::new();
        }
        Self::expand_c_angles(eulumdat)
            .iter()
            .map(|&c| {
                let eff = Self::fold_c(eulumdat.symmetry, c);
                let (idx, _) = stored
                    .iter()
                    .enumerate()
                    .map(|(i, a)| (i, (a - eff).abs()))
                    .min_by(|x, y| x.1.partial_cmp(&y.1).unwrap())
                    .unwrap();
                eulumdat
                    .intensities
                    .get(idx.min(eulumdat.intensities.len() - 1))
                    .cloned()
                    .unwrap_or_default()
            })
            .collect()
    }

    /// Get intensity at any C and G angle by interpolation.
    ///
    /// This handles symmetry automatically, interpolating between stored data points.
    pub fn get_intensity_at(eulumdat: &Eulumdat, c_angle: f64, g_angle: f64) -> f64 {
        // Normalize C angle to 0-360 range
        let c_normalized = c_angle.rem_euclid(360.0);

        // Clamp G angle to 0-180 range
        let g_clamped = g_angle.clamp(0.0, 180.0);

        // Fold into the stored domain of this symmetry.
        let effective_c = Self::fold_c(eulumdat.symmetry, c_normalized);

        let g_idx = Self::find_interpolation_indices(&eulumdat.g_angles, g_clamped);

        // For Symmetry::None, C-planes are cyclic (360° wraps to 0°).
        // We need special handling when effective_c falls between the last
        // C-angle and 360° (which equals the first C-angle).
        if eulumdat.symmetry == Symmetry::None && !eulumdat.c_angles.is_empty() {
            let last_c = *eulumdat.c_angles.last().unwrap();
            let first_c = eulumdat.c_angles[0];

            // Check if the angle is beyond the last stored C-plane
            if effective_c > last_c && eulumdat.c_angles.len() > 1 {
                // Wrap: interpolate between last C-plane and first C-plane
                let span = (360.0 - last_c) + first_c; // gap across the wrap
                if span > 0.0 {
                    let fraction = (effective_c - last_c) / span;
                    let last_idx = eulumdat.c_angles.len() - 1;
                    return Self::bilinear_interpolate_wrap(
                        eulumdat, last_idx, 0, fraction, g_idx, g_clamped,
                    );
                }
            }
        }

        // Find surrounding C indices (non-wrapping) among the *stored* planes,
        // whose angles are a subset of the file's C list for symmetric files.
        let c_idx = if eulumdat.symmetry == Symmetry::None {
            Self::find_interpolation_indices(&eulumdat.c_angles, effective_c)
        } else {
            Self::find_interpolation_indices(&Self::stored_c_angles(eulumdat), effective_c)
        };

        // Bilinear interpolation
        Self::bilinear_interpolate(eulumdat, c_idx, g_idx, effective_c, g_clamped)
    }

    /// Find indices for interpolation (lower index and fraction).
    fn find_interpolation_indices(angles: &[f64], target: f64) -> (usize, f64) {
        if angles.is_empty() {
            return (0, 0.0);
        }

        if target <= angles[0] {
            return (0, 0.0);
        }

        if target >= angles[angles.len() - 1] {
            return (angles.len() - 1, 0.0);
        }

        for i in 0..angles.len() - 1 {
            if target >= angles[i] && target <= angles[i + 1] {
                let fraction = (target - angles[i]) / (angles[i + 1] - angles[i]);
                return (i, fraction);
            }
        }

        (angles.len() - 1, 0.0)
    }

    /// Perform bilinear interpolation on intensity data.
    fn bilinear_interpolate(
        eulumdat: &Eulumdat,
        c_idx: (usize, f64),
        g_idx: (usize, f64),
        _c_angle: f64,
        _g_angle: f64,
    ) -> f64 {
        let (ci, cf) = c_idx;
        let (gi, gf) = g_idx;

        // Get the four surrounding intensity values
        let get = |c: usize, g: usize| -> f64 {
            eulumdat
                .intensities
                .get(c)
                .and_then(|row| row.get(g))
                .copied()
                .unwrap_or(0.0)
        };

        let i00 = get(ci, gi);
        let i01 = get(ci, gi + 1);
        let i10 = get(ci + 1, gi);
        let i11 = get(ci + 1, gi + 1);

        // Bilinear interpolation
        let i0 = i00 * (1.0 - gf) + i01 * gf;
        let i1 = i10 * (1.0 - gf) + i11 * gf;

        i0 * (1.0 - cf) + i1 * cf
    }

    /// Bilinear interpolation wrapping between two explicit C-plane indices.
    /// Used when the C angle wraps from the last stored plane back to the first (360°→0°).
    fn bilinear_interpolate_wrap(
        eulumdat: &Eulumdat,
        ci_lo: usize,
        ci_hi: usize,
        cf: f64,
        g_idx: (usize, f64),
        _g_angle: f64,
    ) -> f64 {
        let (gi, gf) = g_idx;

        let get = |c: usize, g: usize| -> f64 {
            eulumdat
                .intensities
                .get(c)
                .and_then(|row| row.get(g))
                .copied()
                .unwrap_or(0.0)
        };

        let i00 = get(ci_lo, gi);
        let i01 = get(ci_lo, gi + 1);
        let i10 = get(ci_hi, gi);
        let i11 = get(ci_hi, gi + 1);

        let i0 = i00 * (1.0 - gf) + i01 * gf;
        let i1 = i10 * (1.0 - gf) + i11 * gf;

        i0 * (1.0 - cf) + i1 * cf
    }

    /// Convert polar coordinates (C, G, intensity) to Cartesian for visualization.
    ///
    /// Returns (x, y) coordinates where:
    /// - x axis points right (C=90°, G=90°)
    /// - y axis points up (C=0°, G=90°)
    /// - The returned coordinates are scaled by intensity.
    pub fn polar_to_cartesian(c_angle: f64, g_angle: f64, intensity: f64) -> (f64, f64) {
        // Convert to radians
        let c_rad = c_angle.to_radians();
        let g_rad = g_angle.to_radians();

        // For 2D polar diagram (viewing down the luminaire axis):
        // G angle is the radial distance from center
        // C angle is the rotation around the center
        let r = intensity * g_rad.sin();
        let x = r * c_rad.sin();
        let y = r * c_rad.cos();

        (x, y)
    }

    /// Generate points for a polar diagram of a C-plane.
    ///
    /// Returns a vector of (x, y) points for rendering.
    pub fn generate_polar_points(eulumdat: &Eulumdat, c_index: usize) -> Vec<(f64, f64)> {
        if c_index >= eulumdat.intensities.len() {
            return Vec::new();
        }

        let intensities = &eulumdat.intensities[c_index];
        let max_intensity = eulumdat.max_intensity();

        if max_intensity <= 0.0 {
            return Vec::new();
        }

        eulumdat
            .g_angles
            .iter()
            .zip(intensities.iter())
            .map(|(&g_angle, &intensity)| {
                // Normalize intensity and convert to Cartesian
                let normalized = intensity / max_intensity;
                let g_rad = g_angle.to_radians();
                // Standard polar: angle from vertical, distance = normalized intensity
                let x = normalized * (-(g_rad) + std::f64::consts::FRAC_PI_2).cos();
                let y = normalized * (-(g_rad) + std::f64::consts::FRAC_PI_2).sin();
                (x, y)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_mc() {
        assert_eq!(Symmetry::None.calc_mc(36), 36);
        assert_eq!(Symmetry::VerticalAxis.calc_mc(36), 1);
        assert_eq!(Symmetry::PlaneC0C180.calc_mc(36), 19);
        assert_eq!(Symmetry::PlaneC90C270.calc_mc(36), 19);
        assert_eq!(Symmetry::BothPlanes.calc_mc(36), 10);
    }

    #[test]
    fn test_polar_to_cartesian() {
        let (x, y) = SymmetryHandler::polar_to_cartesian(0.0, 90.0, 1.0);
        assert!((x - 0.0).abs() < 0.001);
        assert!((y - 1.0).abs() < 0.001);

        let (x, y) = SymmetryHandler::polar_to_cartesian(90.0, 90.0, 1.0);
        assert!((x - 1.0).abs() < 0.001);
        assert!((y - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_get_intensity_at_exact_angles() {
        let ldt = Eulumdat {
            symmetry: Symmetry::None,
            c_angles: vec![0.0, 90.0, 180.0, 270.0],
            g_angles: vec![0.0, 45.0, 90.0],
            intensities: vec![
                vec![100.0, 80.0, 50.0], // C0
                vec![90.0, 70.0, 40.0],  // C90
                vec![80.0, 60.0, 30.0],  // C180
                vec![70.0, 50.0, 20.0],  // C270
            ],
            ..Default::default()
        };

        // Test exact angles
        let i = SymmetryHandler::get_intensity_at(&ldt, 0.0, 0.0);
        assert!((i - 100.0).abs() < 0.001);

        let i = SymmetryHandler::get_intensity_at(&ldt, 90.0, 45.0);
        assert!((i - 70.0).abs() < 0.001);
    }

    #[test]
    fn test_get_intensity_at_interpolated() {
        let ldt = Eulumdat {
            symmetry: Symmetry::None,
            c_angles: vec![0.0, 90.0],
            g_angles: vec![0.0, 90.0],
            intensities: vec![
                vec![100.0, 0.0], // C0: 100 at nadir, 0 at horizontal
                vec![100.0, 0.0], // C90: same
            ],
            ..Default::default()
        };

        // Interpolate at G=45 should give ~50 (midpoint)
        let i = SymmetryHandler::get_intensity_at(&ldt, 0.0, 45.0);
        assert!((i - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_sample_method() {
        let ldt = Eulumdat {
            symmetry: Symmetry::BothPlanes,
            c_angles: vec![0.0, 45.0, 90.0],
            g_angles: vec![0.0, 30.0, 60.0, 90.0],
            intensities: vec![
                vec![100.0, 90.0, 70.0, 40.0], // C0
                vec![95.0, 85.0, 65.0, 35.0],  // C45
                vec![90.0, 80.0, 60.0, 30.0],  // C90
            ],
            ..Default::default()
        };

        // Test the sample() convenience method
        let i = ldt.sample(0.0, 0.0);
        assert!((i - 100.0).abs() < 0.001);

        // Test symmetry - C180 should mirror C0
        let i_c0 = ldt.sample(0.0, 30.0);
        let i_c180 = ldt.sample(180.0, 30.0);
        assert!((i_c0 - i_c180).abs() < 0.001);

        // Test symmetry - C270 should mirror C90
        let i_c90 = ldt.sample(90.0, 60.0);
        let i_c270 = ldt.sample(270.0, 60.0);
        assert!((i_c90 - i_c270).abs() < 0.001);
    }

    #[test]
    fn test_c_angle_wraparound_symmetry_none() {
        // With Symmetry::None, C-angles between the last stored plane and 360°
        // must interpolate correctly by wrapping to C0.
        let ldt = Eulumdat {
            symmetry: Symmetry::None,
            c_angles: vec![0.0, 90.0, 180.0, 270.0],
            g_angles: vec![0.0, 45.0, 90.0],
            intensities: vec![
                vec![100.0, 80.0, 50.0], // C0
                vec![100.0, 80.0, 50.0], // C90  (same as C0)
                vec![100.0, 80.0, 50.0], // C180 (same)
                vec![100.0, 80.0, 50.0], // C270 (same)
            ],
            ..Default::default()
        };

        // With all planes identical, any C angle should give the same result
        let i_0 = SymmetryHandler::get_intensity_at(&ldt, 0.0, 45.0);
        let i_315 = SymmetryHandler::get_intensity_at(&ldt, 315.0, 45.0);
        let i_350 = SymmetryHandler::get_intensity_at(&ldt, 350.0, 45.0);
        assert!(
            (i_0 - i_315).abs() < 0.001,
            "C315 should equal C0 when all planes identical"
        );
        assert!(
            (i_0 - i_350).abs() < 0.001,
            "C350 should equal C0 when all planes identical"
        );

        // Now test with different C0 and C270 to verify proper interpolation
        let ldt2 = Eulumdat {
            symmetry: Symmetry::None,
            c_angles: vec![0.0, 90.0, 180.0, 270.0],
            g_angles: vec![0.0, 90.0],
            intensities: vec![
                vec![100.0, 50.0],  // C0
                vec![100.0, 50.0],  // C90
                vec![100.0, 50.0],  // C180
                vec![200.0, 100.0], // C270
            ],
            ..Default::default()
        };

        // C315 = midpoint between C270 and C0(=C360)
        let i_c315 = SymmetryHandler::get_intensity_at(&ldt2, 315.0, 0.0);
        // Should be average of C270(200) and C0(100) = 150
        assert!(
            (i_c315 - 150.0).abs() < 0.01,
            "C315 at gamma=0 should be 150, got {i_c315}"
        );

        // C-angle symmetry: sample(45°) should mirror sample(315°)
        // when C0=C90 and C270 wraps to C0
        let i_c45 = SymmetryHandler::get_intensity_at(&ldt2, 45.0, 0.0);
        // C45 = midpoint between C0(100) and C90(100) = 100
        assert!(
            (i_c45 - 100.0).abs() < 0.01,
            "C45 at gamma=0 should be 100, got {i_c45}"
        );
    }
}

#[cfg(test)]
mod expansion_tests {
    use super::*;
    use crate::Eulumdat;

    const ISYM2: &str = include_str!("../../eulumdat-wasm-templates/templates/0-2-0.ldt");
    const ISYM3: &str = include_str!("../../eulumdat-wasm-templates/templates/0-3-0.ldt");
    const ISYM4: &str =
        include_str!("../../eulumdat-wasm-templates/templates/fluorescent_luminaire.ldt");

    fn row_at(ldt: &Eulumdat, full: &[Vec<f64>], angles: &[f64], c: f64) -> Vec<f64> {
        let i = angles
            .iter()
            .position(|a| (a - c).abs() < 1e-6)
            .unwrap_or_else(|| panic!("no angle {c} in {angles:?}"));
        let _ = ldt;
        full[i].clone()
    }

    fn check_file(text: &str, sym: Symmetry, pairs: &[(f64, f64)]) {
        let ldt = Eulumdat::parse(text).unwrap();
        assert_eq!(ldt.symmetry, sym);
        let angles = SymmetryHandler::expand_c_angles(&ldt);
        let full = SymmetryHandler::expand_to_full(&ldt);
        assert_eq!(angles.len(), ldt.num_c_planes, "angles vs Nc");
        assert_eq!(full.len(), angles.len(), "planes vs angles");
        assert_eq!(
            ldt.intensities.len(),
            ldt.actual_c_planes(),
            "stored planes vs Mc"
        );
        let stored = SymmetryHandler::stored_c_angles(&ldt);
        assert_eq!(stored.len(), ldt.intensities.len());
        // Mirrored planes equal their stored counterparts.
        for &(full_c, stored_c) in pairs {
            let si = stored
                .iter()
                .position(|a| (a - stored_c).abs() < 1e-6)
                .unwrap();
            assert_eq!(
                row_at(&ldt, &full, &angles, full_c),
                ldt.intensities[si],
                "C{full_c} should equal stored C{stored_c}"
            );
        }
        // Sampling at every full angle reproduces the expanded row.
        for (i, &c) in angles.iter().enumerate() {
            for (gi, &g) in ldt.g_angles.iter().enumerate() {
                let s = SymmetryHandler::get_intensity_at(&ldt, c, g);
                assert!(
                    (s - full[i][gi]).abs() < 1e-6,
                    "sample({c},{g}) = {s} vs {}",
                    full[i][gi]
                );
            }
        }
    }

    #[test]
    fn quadrant_symmetry_planes_match_angles() {
        check_file(
            ISYM4,
            Symmetry::BothPlanes,
            &[
                (105.0, 75.0),
                (180.0, 0.0),
                (195.0, 15.0),
                (270.0, 90.0),
                (345.0, 15.0),
            ],
        );
    }

    #[test]
    fn c0_c180_symmetry_planes_match_angles() {
        check_file(
            ISYM2,
            Symmetry::PlaneC0C180,
            &[(195.0, 165.0), (270.0, 90.0), (345.0, 15.0)],
        );
    }

    #[test]
    fn c90_c270_symmetry_planes_match_angles() {
        check_file(
            ISYM3,
            Symmetry::PlaneC90C270,
            &[(0.0, 180.0), (45.0, 135.0), (90.0, 90.0), (315.0, 225.0)],
        );
    }

    #[test]
    fn reduced_angle_list_is_mirrored_without_duplicates() {
        let mut ldt = Eulumdat::parse(ISYM4).unwrap();
        ldt.c_angles.truncate(ldt.actual_c_planes()); // only the quadrant, as a builder might do
        let angles = SymmetryHandler::expand_c_angles(&ldt);
        assert_eq!(angles.len(), 24, "{angles:?}");
        assert_eq!(angles[0], 0.0);
        assert_eq!(angles[6], 90.0);
        assert_eq!(angles[23], 345.0);
        assert_eq!(SymmetryHandler::expand_to_full(&ldt).len(), 24);
    }

    #[test]
    fn rotation_of_quadrant_symmetric_file_keeps_plane_count() {
        let mut ldt = Eulumdat::parse(ISYM4).unwrap();
        let before = SymmetryHandler::get_intensity_at(&ldt, 30.0, 45.0);
        ldt.rotate_c_planes(90.0);
        assert_eq!(ldt.symmetry, Symmetry::None);
        assert_eq!(ldt.intensities.len(), ldt.c_angles.len());
        let after = SymmetryHandler::get_intensity_at(&ldt, 120.0, 45.0);
        assert!((before - after).abs() < 1e-6, "{before} vs {after}");
    }
}
