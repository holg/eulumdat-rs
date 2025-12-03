//! Photometric calculations for Eulumdat data.
//!
//! Implements standard lighting calculations including:
//! - Downward flux fraction
//! - Total luminous output
//! - Utilization factors (direct ratios)

use crate::eulumdat::{Eulumdat, Symmetry};
use std::f64::consts::PI;

/// Photometric calculations on Eulumdat data.
pub struct PhotometricCalculations;

impl PhotometricCalculations {
    /// Calculate the downward flux fraction up to a given arc angle.
    ///
    /// Integrates the luminous intensity distribution from 0° to the specified
    /// arc angle to determine the percentage of light directed downward.
    ///
    /// # Arguments
    /// * `ldt` - The Eulumdat data
    /// * `arc` - The maximum angle from vertical (0° = straight down, 90° = horizontal)
    ///
    /// # Returns
    /// The downward flux fraction as a percentage (0-100).
    pub fn downward_flux(ldt: &Eulumdat, arc: f64) -> f64 {
        let total_output = Self::total_output(ldt);
        if total_output <= 0.0 {
            return 0.0;
        }

        let downward = match ldt.symmetry {
            Symmetry::None => Self::downward_no_symmetry(ldt, arc),
            Symmetry::VerticalAxis => Self::downward_for_plane(ldt, 0, arc),
            Symmetry::PlaneC0C180 => Self::downward_c0_c180(ldt, arc),
            Symmetry::PlaneC90C270 => Self::downward_c90_c270(ldt, arc),
            Symmetry::BothPlanes => Self::downward_both_planes(ldt, arc),
        };

        100.0 * downward / total_output
    }

    /// Calculate downward flux for no symmetry case.
    fn downward_no_symmetry(ldt: &Eulumdat, arc: f64) -> f64 {
        let mc = ldt.actual_c_planes();
        if mc == 0 || ldt.c_angles.is_empty() {
            return 0.0;
        }

        let mut sum = 0.0;

        for i in 1..mc {
            let delta_c = ldt.c_angles[i] - ldt.c_angles[i - 1];
            sum += delta_c * Self::downward_for_plane(ldt, i - 1, arc);
        }

        // Handle wrap-around from last plane to first
        if mc > 1 {
            let delta_c = 360.0 - ldt.c_angles[mc - 1];
            sum += delta_c * Self::downward_for_plane(ldt, mc - 1, arc);
        }

        sum / 360.0
    }

    /// Calculate downward flux for C0-C180 symmetry.
    fn downward_c0_c180(ldt: &Eulumdat, arc: f64) -> f64 {
        let mc = ldt.actual_c_planes();
        if mc == 0 || ldt.c_angles.is_empty() {
            return 0.0;
        }

        let mut sum = 0.0;

        for i in 1..mc {
            let delta_c = ldt.c_angles[i] - ldt.c_angles[i - 1];
            sum += 2.0 * delta_c * Self::downward_for_plane(ldt, i - 1, arc);
        }

        // Handle to 180°
        if mc > 0 {
            let delta_c = 180.0 - ldt.c_angles[mc - 1];
            sum += 2.0 * delta_c * Self::downward_for_plane(ldt, mc - 1, arc);
        }

        sum / 360.0
    }

    /// Calculate downward flux for C90-C270 symmetry.
    fn downward_c90_c270(ldt: &Eulumdat, arc: f64) -> f64 {
        // Similar to C0-C180 but shifted
        Self::downward_c0_c180(ldt, arc)
    }

    /// Calculate downward flux for both planes symmetry.
    fn downward_both_planes(ldt: &Eulumdat, arc: f64) -> f64 {
        let mc = ldt.actual_c_planes();
        if mc == 0 || ldt.c_angles.is_empty() {
            return 0.0;
        }

        let mut sum = 0.0;

        for i in 1..mc {
            let delta_c = ldt.c_angles[i] - ldt.c_angles[i - 1];
            sum += 4.0 * delta_c * Self::downward_for_plane(ldt, i - 1, arc);
        }

        // Handle to 90°
        if mc > 0 {
            let delta_c = 90.0 - ldt.c_angles[mc - 1];
            sum += 4.0 * delta_c * Self::downward_for_plane(ldt, mc - 1, arc);
        }

        sum / 360.0
    }

    /// Calculate downward flux for a single C-plane up to arc angle.
    fn downward_for_plane(ldt: &Eulumdat, c_index: usize, arc: f64) -> f64 {
        if c_index >= ldt.intensities.len() || ldt.g_angles.is_empty() {
            return 0.0;
        }

        let intensities = &ldt.intensities[c_index];
        let mut sum = 0.0;

        for j in 1..ldt.g_angles.len() {
            let g_prev = ldt.g_angles[j - 1];
            let g_curr = ldt.g_angles[j];

            // Only integrate up to arc angle
            if g_prev >= arc {
                break;
            }

            let g_end = g_curr.min(arc);
            let delta_g = g_end - g_prev;

            if delta_g <= 0.0 {
                continue;
            }

            // Average intensity in this segment
            let i_prev = intensities.get(j - 1).copied().unwrap_or(0.0);
            let i_curr = intensities.get(j).copied().unwrap_or(0.0);
            let avg_intensity = (i_prev + i_curr) / 2.0;

            // Convert to radians for solid angle calculation
            let g_prev_rad = g_prev * PI / 180.0;
            let g_end_rad = g_end * PI / 180.0;

            // Solid angle element: sin(g) * dg
            let solid_angle = (g_prev_rad.cos() - g_end_rad.cos()).abs();

            sum += avg_intensity * solid_angle;
        }

        sum * 2.0 * PI
    }

    /// Calculate total luminous output.
    ///
    /// Integrates the luminous intensity over the entire sphere.
    pub fn total_output(ldt: &Eulumdat) -> f64 {
        // Use downward_flux with 180° to get full sphere
        let mc = ldt.actual_c_planes();
        if mc == 0 {
            return 0.0;
        }

        match ldt.symmetry {
            Symmetry::None => Self::downward_no_symmetry(ldt, 180.0),
            Symmetry::VerticalAxis => Self::downward_for_plane(ldt, 0, 180.0),
            Symmetry::PlaneC0C180 => Self::downward_c0_c180(ldt, 180.0),
            Symmetry::PlaneC90C270 => Self::downward_c90_c270(ldt, 180.0),
            Symmetry::BothPlanes => Self::downward_both_planes(ldt, 180.0),
        }
    }

    /// Calculate the luminous flux from the stored intensity distribution.
    ///
    /// This uses the conversion factor to convert from cd/klm to actual lumens.
    pub fn calculated_luminous_flux(ldt: &Eulumdat) -> f64 {
        Self::total_output(ldt) * ldt.conversion_factor
    }

    /// Calculate direct ratios (utilization factors) for standard room indices.
    ///
    /// Room indices k: 0.60, 0.80, 1.00, 1.25, 1.50, 2.00, 2.50, 3.00, 4.00, 5.00
    ///
    /// # Arguments
    /// * `ldt` - The Eulumdat data
    /// * `shr` - Spacing to Height Ratio (typically "1.00", "1.25", or "1.50")
    ///
    /// # Returns
    /// Array of 10 direct ratio values for the standard room indices.
    pub fn calculate_direct_ratios(ldt: &Eulumdat, shr: &str) -> [f64; 10] {
        // Coefficient lookup tables from standard
        let (e, f, g, h) = Self::get_shr_coefficients(shr);

        // Calculate flux values at critical angles
        let a = Self::downward_flux(ldt, 41.4);
        let b = Self::downward_flux(ldt, 60.0);
        let c = Self::downward_flux(ldt, 75.5);
        let d = Self::downward_flux(ldt, 90.0);

        let mut ratios = [0.0; 10];

        for i in 0..10 {
            let t = a * e[i] + b * f[i] + c * g[i] + d * h[i];
            ratios[i] = t / 100_000.0;
        }

        ratios
    }

    /// Get SHR coefficients for direct ratio calculation.
    fn get_shr_coefficients(shr: &str) -> ([f64; 10], [f64; 10], [f64; 10], [f64; 10]) {
        match shr {
            "1.00" => (
                [
                    943.0, 752.0, 636.0, 510.0, 429.0, 354.0, 286.0, 258.0, 236.0, 231.0,
                ],
                [
                    -317.0, -33.0, 121.0, 238.0, 275.0, 248.0, 190.0, 118.0, -6.0, -99.0,
                ],
                [
                    481.0, 372.0, 310.0, 282.0, 309.0, 363.0, 416.0, 463.0, 512.0, 518.0,
                ],
                [
                    -107.0, -91.0, -67.0, -30.0, -13.0, 35.0, 108.0, 161.0, 258.0, 350.0,
                ],
            ),
            "1.25" => (
                [
                    967.0, 808.0, 695.0, 565.0, 476.0, 386.0, 307.0, 273.0, 243.0, 234.0,
                ],
                [
                    -336.0, -82.0, 73.0, 200.0, 249.0, 243.0, 201.0, 137.0, 18.0, -73.0,
                ],
                [
                    451.0, 339.0, 280.0, 255.0, 278.0, 331.0, 384.0, 432.0, 485.0, 497.0,
                ],
                [
                    -82.0, -65.0, -48.0, -20.0, -3.0, 40.0, 108.0, 158.0, 254.0, 342.0,
                ],
            ),
            _ => (
                [
                    983.0, 851.0, 744.0, 614.0, 521.0, 418.0, 329.0, 289.0, 252.0, 239.0,
                ],
                [
                    -348.0, -122.0, 31.0, 163.0, 220.0, 231.0, 203.0, 149.0, 39.0, -48.0,
                ],
                [
                    430.0, 315.0, 256.0, 233.0, 253.0, 304.0, 356.0, 404.0, 460.0, 476.0,
                ],
                [
                    -65.0, -44.0, -31.0, -10.0, 6.0, 47.0, 112.0, 158.0, 249.0, 333.0,
                ],
            ),
        }
    }

    /// Calculate beam angle (angle where intensity drops to 50% of maximum).
    pub fn beam_angle(ldt: &Eulumdat) -> f64 {
        Self::angle_at_percentage(ldt, 0.5)
    }

    /// Calculate field angle (angle where intensity drops to 10% of maximum).
    pub fn field_angle(ldt: &Eulumdat) -> f64 {
        Self::angle_at_percentage(ldt, 0.1)
    }

    /// Find the angle at which intensity drops to a given percentage of maximum.
    fn angle_at_percentage(ldt: &Eulumdat, percentage: f64) -> f64 {
        if ldt.intensities.is_empty() || ldt.g_angles.is_empty() {
            return 0.0;
        }

        // Use first C-plane (or average for non-symmetric)
        let intensities = &ldt.intensities[0];
        let max_intensity = intensities.iter().copied().fold(0.0, f64::max);

        if max_intensity <= 0.0 {
            return 0.0;
        }

        let threshold = max_intensity * percentage;

        // Find where intensity drops below threshold
        for (i, &intensity) in intensities.iter().enumerate() {
            if intensity < threshold && i > 0 {
                // Interpolate between previous and current
                let prev_intensity = intensities[i - 1];
                let prev_angle = ldt.g_angles[i - 1];
                let curr_angle = ldt.g_angles[i];

                if prev_intensity > threshold {
                    let ratio = (prev_intensity - threshold) / (prev_intensity - intensity);
                    return prev_angle + ratio * (curr_angle - prev_angle);
                }
            }
        }

        // If never drops below threshold, return last angle
        *ldt.g_angles.last().unwrap_or(&0.0)
    }

    /// Calculate UGR (Unified Glare Rating) cross-section data.
    ///
    /// Returns intensity values at standard viewing angles for UGR calculation.
    pub fn ugr_crosssection(ldt: &Eulumdat) -> Vec<(f64, f64)> {
        // Standard UGR angles: 45°, 55°, 65°, 75°, 85°
        let ugr_angles = [45.0, 55.0, 65.0, 75.0, 85.0];

        ugr_angles
            .iter()
            .map(|&angle| {
                let intensity = crate::symmetry::SymmetryHandler::get_intensity_at(ldt, 0.0, angle);
                (angle, intensity)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eulumdat::LampSet;

    fn create_test_ldt() -> Eulumdat {
        let mut ldt = Eulumdat::new();
        ldt.symmetry = Symmetry::VerticalAxis;
        ldt.num_c_planes = 1;
        ldt.num_g_planes = 7;
        ldt.c_angles = vec![0.0];
        ldt.g_angles = vec![0.0, 15.0, 30.0, 45.0, 60.0, 75.0, 90.0];
        // Typical downlight distribution
        ldt.intensities = vec![vec![1000.0, 980.0, 900.0, 750.0, 500.0, 200.0, 50.0]];
        ldt.lamp_sets.push(LampSet {
            num_lamps: 1,
            lamp_type: "LED".to_string(),
            total_luminous_flux: 1000.0,
            color_appearance: "3000K".to_string(),
            color_rendering_group: "80".to_string(),
            wattage_with_ballast: 10.0,
        });
        ldt.conversion_factor = 1.0;
        ldt
    }

    #[test]
    fn test_total_output() {
        let ldt = create_test_ldt();
        let output = PhotometricCalculations::total_output(&ldt);
        assert!(output > 0.0, "Total output should be positive");
    }

    #[test]
    fn test_downward_flux() {
        let ldt = create_test_ldt();
        let flux_90 = PhotometricCalculations::downward_flux(&ldt, 90.0);
        let flux_180 = PhotometricCalculations::downward_flux(&ldt, 180.0);

        // Flux at 90° should be less than at 180° (full hemisphere)
        assert!(flux_90 <= flux_180 + 0.001);
        // Both should be between 0 and 100%
        assert!((0.0..=100.0).contains(&flux_90));
        assert!((0.0..=100.0).contains(&flux_180));
    }

    #[test]
    fn test_beam_angle() {
        let ldt = create_test_ldt();
        let beam = PhotometricCalculations::beam_angle(&ldt);
        // Beam angle should be positive and less than 90° for a downlight
        assert!(beam > 0.0 && beam <= 90.0, "Beam angle was {}", beam);
    }

    #[test]
    fn test_direct_ratios() {
        let ldt = create_test_ldt();
        let ratios = PhotometricCalculations::calculate_direct_ratios(&ldt, "1.00");

        // All ratios should be between 0 and 1
        for ratio in &ratios {
            assert!(*ratio >= 0.0 && *ratio <= 1.0);
        }

        // Ratios should generally increase with room index
        // (larger rooms capture more light)
        for i in 1..10 {
            // Allow small variance
            assert!(ratios[i] >= ratios[0] - 0.1);
        }
    }
}
