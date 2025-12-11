//! Symmetry handling for Eulumdat data.
//!
//! This module provides utilities for working with symmetric luminous intensity distributions.
//! Symmetry can significantly reduce the amount of data needed to represent a luminaire.

use crate::eulumdat::{Eulumdat, Symmetry};

/// Handler for symmetry-based operations on photometric data.
pub struct SymmetryHandler;

impl SymmetryHandler {
    /// Expand symmetric data to full 360° distribution.
    ///
    /// Takes the stored (reduced) intensity data and expands it based on the symmetry type.
    /// Returns a full intensity matrix with all C-planes from 0° to 360°.
    pub fn expand_to_full(eulumdat: &Eulumdat) -> Vec<Vec<f64>> {
        match eulumdat.symmetry {
            Symmetry::None => eulumdat.intensities.clone(),
            Symmetry::VerticalAxis => Self::expand_vertical_axis(eulumdat),
            Symmetry::PlaneC0C180 => Self::expand_c0_c180(eulumdat),
            Symmetry::PlaneC90C270 => Self::expand_c90_c270(eulumdat),
            Symmetry::BothPlanes => Self::expand_both_planes(eulumdat),
        }
    }

    /// Expand vertically symmetric data (single C-plane to all planes).
    fn expand_vertical_axis(eulumdat: &Eulumdat) -> Vec<Vec<f64>> {
        if eulumdat.intensities.is_empty() {
            return Vec::new();
        }

        // For vertical axis symmetry, all C-planes have the same intensity distribution
        let single_plane = &eulumdat.intensities[0];
        let num_planes = eulumdat.num_c_planes.max(1);

        (0..num_planes).map(|_| single_plane.clone()).collect()
    }

    /// Expand C0-C180 symmetric data (mirror across C0-C180 plane).
    fn expand_c0_c180(eulumdat: &Eulumdat) -> Vec<Vec<f64>> {
        let mc = eulumdat.actual_c_planes();
        if mc == 0 || eulumdat.intensities.is_empty() {
            return Vec::new();
        }

        let mut result = eulumdat.intensities.clone();

        // Mirror the data: C-planes from 180° to 360° mirror 180° to 0°
        for i in 1..mc {
            if mc - i < eulumdat.intensities.len() {
                result.push(eulumdat.intensities[mc - i].clone());
            }
        }

        result
    }

    /// Expand C90-C270 symmetric data (mirror across C90-C270 plane).
    fn expand_c90_c270(eulumdat: &Eulumdat) -> Vec<Vec<f64>> {
        let mc = eulumdat.actual_c_planes();
        if mc == 0 || eulumdat.intensities.is_empty() {
            return Vec::new();
        }

        let mut result = eulumdat.intensities.clone();

        // Mirror the data: C-planes from 270° to 360° and 0° to 90° mirror 90° to 270°
        for i in 1..mc {
            if mc - i < eulumdat.intensities.len() {
                result.push(eulumdat.intensities[mc - i].clone());
            }
        }

        result
    }

    /// Expand data symmetric in both planes (quarter to full).
    fn expand_both_planes(eulumdat: &Eulumdat) -> Vec<Vec<f64>> {
        let mc = eulumdat.actual_c_planes();
        if mc == 0 || eulumdat.intensities.is_empty() {
            return Vec::new();
        }

        let mut result = Vec::new();

        // First quadrant (0° to 90°)
        for i in 0..mc {
            if i < eulumdat.intensities.len() {
                result.push(eulumdat.intensities[i].clone());
            }
        }

        // Second quadrant (90° to 180°) - mirror of first
        for i in (1..mc - 1).rev() {
            if i < eulumdat.intensities.len() {
                result.push(eulumdat.intensities[i].clone());
            }
        }

        // Third quadrant (180° to 270°) - same as first
        for i in 0..mc {
            if i < eulumdat.intensities.len() {
                result.push(eulumdat.intensities[i].clone());
            }
        }

        // Fourth quadrant (270° to 360°) - mirror of first
        for i in (1..mc - 1).rev() {
            if i < eulumdat.intensities.len() {
                result.push(eulumdat.intensities[i].clone());
            }
        }

        result
    }

    /// Get the C-plane angles for the full 360° distribution.
    pub fn expand_c_angles(eulumdat: &Eulumdat) -> Vec<f64> {
        match eulumdat.symmetry {
            Symmetry::None => eulumdat.c_angles.clone(),
            Symmetry::VerticalAxis => {
                // Generate angles based on Nc and Dc
                (0..eulumdat.num_c_planes)
                    .map(|i| i as f64 * eulumdat.c_plane_distance)
                    .collect()
            }
            Symmetry::PlaneC0C180 => {
                // For C0-C180 symmetry, all C-angles are already stored in the file
                // No expansion needed, just return them
                eulumdat.c_angles.clone()
            }
            Symmetry::PlaneC90C270 => {
                // For C90-C270 symmetry, all C-angles are already stored in the file
                // No expansion needed, just return them
                eulumdat.c_angles.clone()
            }
            Symmetry::BothPlanes => {
                let mut angles = Vec::new();
                // First quadrant
                for &angle in &eulumdat.c_angles {
                    angles.push(angle);
                }
                // Second quadrant (mirror of first)
                for &angle in eulumdat.c_angles.iter().rev().skip(1) {
                    angles.push(180.0 - angle);
                }
                // Third quadrant
                for &angle in eulumdat.c_angles.iter().skip(1) {
                    angles.push(180.0 + angle);
                }
                // Fourth quadrant (mirror)
                for &angle in eulumdat.c_angles.iter().rev().skip(1) {
                    angles.push(360.0 - angle);
                }
                angles
            }
        }
    }

    /// Get intensity at any C and G angle by interpolation.
    ///
    /// This handles symmetry automatically, interpolating between stored data points.
    pub fn get_intensity_at(eulumdat: &Eulumdat, c_angle: f64, g_angle: f64) -> f64 {
        // Normalize C angle to 0-360 range
        let c_normalized = c_angle.rem_euclid(360.0);

        // Clamp G angle to 0-180 range
        let g_clamped = g_angle.clamp(0.0, 180.0);

        // Find the effective C index based on symmetry
        let effective_c = match eulumdat.symmetry {
            Symmetry::None => c_normalized,
            Symmetry::VerticalAxis => 0.0, // All C-planes are the same
            Symmetry::PlaneC0C180 => {
                if c_normalized <= 180.0 {
                    c_normalized
                } else {
                    360.0 - c_normalized
                }
            }
            Symmetry::PlaneC90C270 => {
                let shifted = (c_normalized + 90.0).rem_euclid(360.0);
                if shifted <= 180.0 {
                    shifted - 90.0
                } else {
                    270.0 - shifted
                }
            }
            Symmetry::BothPlanes => {
                let in_first_half = c_normalized <= 180.0;
                let c_in_half = if in_first_half {
                    c_normalized
                } else {
                    360.0 - c_normalized
                };
                if c_in_half <= 90.0 {
                    c_in_half
                } else {
                    180.0 - c_in_half
                }
            }
        };

        // Find surrounding C indices
        let c_idx = Self::find_interpolation_indices(&eulumdat.c_angles, effective_c);
        let g_idx = Self::find_interpolation_indices(&eulumdat.g_angles, g_clamped);

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
}
