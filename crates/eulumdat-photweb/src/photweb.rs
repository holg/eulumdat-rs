//! PhotometricWeb - Core representation of a luminous intensity distribution

use eulumdat::{Eulumdat, Symmetry};

/// A photometric web representing the full 3D luminous intensity distribution.
///
/// This structure provides efficient sampling of intensity values at any
/// C-plane and gamma angle, handling symmetry automatically.
#[derive(Debug, Clone)]
pub struct PhotometricWeb {
    /// C-plane angles in degrees (0-360)
    c_angles: Vec<f64>,
    /// Gamma angles in degrees (0-180)
    g_angles: Vec<f64>,
    /// Intensity values in cd/klm, indexed as [c_index][g_index]
    intensities: Vec<Vec<f64>>,
    /// Symmetry type
    symmetry: Symmetry,
    /// Maximum intensity value (cached)
    max_intensity: f64,
    /// Minimum intensity value (cached)
    min_intensity: f64,
}

impl PhotometricWeb {
    /// Create a new PhotometricWeb from raw data.
    pub fn new(
        c_angles: Vec<f64>,
        g_angles: Vec<f64>,
        intensities: Vec<Vec<f64>>,
        symmetry: Symmetry,
    ) -> Self {
        let max_intensity = intensities
            .iter()
            .flat_map(|row| row.iter())
            .copied()
            .fold(0.0, f64::max);
        let min_intensity = intensities
            .iter()
            .flat_map(|row| row.iter())
            .copied()
            .fold(f64::MAX, f64::min);

        Self {
            c_angles,
            g_angles,
            intensities,
            symmetry,
            max_intensity,
            min_intensity,
        }
    }

    /// Sample intensity at any C and G angle using bilinear interpolation.
    ///
    /// Handles symmetry automatically - you can query any angle in the full
    /// 0-360° C range and 0-180° G range regardless of stored symmetry.
    ///
    /// # Arguments
    /// * `c_angle` - C-plane angle in degrees (will be normalized to 0-360)
    /// * `g_angle` - Gamma angle in degrees (will be clamped to 0-180)
    ///
    /// # Returns
    /// Intensity in cd/klm
    pub fn sample(&self, c_angle: f64, g_angle: f64) -> f64 {
        // Normalize C angle to 0-360 range
        let c_normalized = c_angle.rem_euclid(360.0);
        // Clamp G angle to 0-180 range
        let g_clamped = g_angle.clamp(0.0, 180.0);

        // Find the effective C based on symmetry
        let effective_c = self.apply_symmetry(c_normalized);

        // Find interpolation indices
        let (ci, cf) = self.find_interpolation_index(&self.c_angles, effective_c);
        let (gi, gf) = self.find_interpolation_index(&self.g_angles, g_clamped);

        // Bilinear interpolation
        self.bilinear_interpolate(ci, cf, gi, gf)
    }

    /// Sample normalized intensity (0.0 to 1.0) at any C and G angle.
    ///
    /// This is useful for mesh generation where you want distances scaled
    /// relative to the maximum intensity.
    pub fn sample_normalized(&self, c_angle: f64, g_angle: f64) -> f64 {
        if self.max_intensity <= 0.0 {
            return 0.0;
        }
        self.sample(c_angle, g_angle) / self.max_intensity
    }

    /// Get the maximum intensity value.
    pub fn max_intensity(&self) -> f64 {
        self.max_intensity
    }

    /// Get the minimum intensity value.
    pub fn min_intensity(&self) -> f64 {
        self.min_intensity
    }

    /// Get the symmetry type.
    pub fn symmetry(&self) -> Symmetry {
        self.symmetry
    }

    /// Get the stored C-plane angles.
    pub fn c_angles(&self) -> &[f64] {
        &self.c_angles
    }

    /// Get the stored gamma angles.
    pub fn g_angles(&self) -> &[f64] {
        &self.g_angles
    }

    /// Apply symmetry to map any C angle to the stored range.
    fn apply_symmetry(&self, c_normalized: f64) -> f64 {
        match self.symmetry {
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
                    (shifted - 90.0).abs()
                } else {
                    (270.0 - shifted).abs()
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
        }
    }

    /// Find interpolation index and fraction for a target angle.
    fn find_interpolation_index(&self, angles: &[f64], target: f64) -> (usize, f64) {
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

    /// Perform bilinear interpolation.
    fn bilinear_interpolate(&self, ci: usize, cf: f64, gi: usize, gf: f64) -> f64 {
        let get = |c: usize, g: usize| -> f64 {
            self.intensities
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
}

impl From<&Eulumdat> for PhotometricWeb {
    fn from(ldt: &Eulumdat) -> Self {
        Self::new(
            ldt.c_angles.clone(),
            ldt.g_angles.clone(),
            ldt.intensities.clone(),
            ldt.symmetry,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_web() -> PhotometricWeb {
        PhotometricWeb::new(
            vec![0.0, 90.0, 180.0, 270.0],
            vec![0.0, 45.0, 90.0, 135.0, 180.0],
            vec![
                vec![100.0, 80.0, 50.0, 30.0, 10.0], // C0
                vec![90.0, 70.0, 40.0, 25.0, 8.0],   // C90
                vec![80.0, 60.0, 30.0, 20.0, 5.0],   // C180
                vec![85.0, 65.0, 35.0, 22.0, 6.0],   // C270
            ],
            Symmetry::None,
        )
    }

    #[test]
    fn test_sample_exact_angles() {
        let web = create_test_web();

        // Test exact stored angles
        let i = web.sample(0.0, 0.0);
        assert!((i - 100.0).abs() < 0.001);

        let i = web.sample(90.0, 45.0);
        assert!((i - 70.0).abs() < 0.001);

        let i = web.sample(180.0, 90.0);
        assert!((i - 30.0).abs() < 0.001);
    }

    #[test]
    fn test_sample_interpolated() {
        let web = PhotometricWeb::new(
            vec![0.0, 90.0],
            vec![0.0, 90.0],
            vec![
                vec![100.0, 0.0], // C0
                vec![100.0, 0.0], // C90
            ],
            Symmetry::None,
        );

        // Midpoint should be 50
        let i = web.sample(0.0, 45.0);
        assert!((i - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_sample_normalized() {
        let web = create_test_web();

        // Max intensity is 100, so normalized at (0, 0) should be 1.0
        let n = web.sample_normalized(0.0, 0.0);
        assert!((n - 1.0).abs() < 0.001);

        // Half intensity should be 0.5
        let n = web.sample_normalized(0.0, 90.0);
        assert!((n - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_symmetry_both_planes() {
        let web = PhotometricWeb::new(
            vec![0.0, 45.0, 90.0],
            vec![0.0, 45.0, 90.0],
            vec![
                vec![100.0, 80.0, 50.0], // C0
                vec![90.0, 70.0, 40.0],  // C45
                vec![80.0, 60.0, 30.0],  // C90
            ],
            Symmetry::BothPlanes,
        );

        // C180 should mirror C0
        let i_c0 = web.sample(0.0, 45.0);
        let i_c180 = web.sample(180.0, 45.0);
        assert!((i_c0 - i_c180).abs() < 0.001);

        // C270 should mirror C90
        let i_c90 = web.sample(90.0, 45.0);
        let i_c270 = web.sample(270.0, 45.0);
        assert!((i_c90 - i_c270).abs() < 0.001);
    }

    #[test]
    fn test_from_eulumdat() {
        let mut ldt = Eulumdat::default();
        ldt.c_angles = vec![0.0, 90.0];
        ldt.g_angles = vec![0.0, 90.0];
        ldt.intensities = vec![vec![100.0, 50.0], vec![80.0, 40.0]];
        ldt.symmetry = Symmetry::None;

        let web = PhotometricWeb::from(&ldt);
        assert_eq!(web.max_intensity(), 100.0);
        assert_eq!(web.c_angles().len(), 2);
    }
}
