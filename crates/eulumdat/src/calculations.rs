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

    /// Calculate beam angle (full angle where intensity drops to 50% of maximum).
    ///
    /// Uses the IES definition: angle between directions where intensity is 50%
    /// of the **maximum** intensity (FWHM - Full Width at Half Maximum).
    ///
    /// **Important**: Per CIE S 017:2020 (17-27-077), beam angle is defined as a
    /// **full angle**, not a half angle. This function returns the full angle
    /// (2× the half-angle from nadir).
    ///
    /// Reference: <https://cie.co.at/eilvterm/17-27-077>
    pub fn beam_angle(ldt: &Eulumdat) -> f64 {
        // Return full angle (2× half angle) per CIE definition
        Self::angle_at_percentage(ldt, 0.5) * 2.0
    }

    /// Calculate field angle (full angle where intensity drops to 10% of maximum).
    ///
    /// Uses the IES definition: angle between directions where intensity is 10%
    /// of the **maximum** intensity.
    ///
    /// **Important**: Per CIE S 017:2020, field angle is defined as a **full angle**,
    /// not a half angle. This function returns the full angle (2× the half-angle from nadir).
    pub fn field_angle(ldt: &Eulumdat) -> f64 {
        // Return full angle (2× half angle) per CIE definition
        Self::angle_at_percentage(ldt, 0.1) * 2.0
    }

    /// Calculate beam angle using CIE definition (center-beam intensity).
    ///
    /// Uses the CIE/NEMA definition: angle between directions where intensity
    /// is 50% of the **center-beam** intensity (intensity at 0° gamma).
    ///
    /// **Important**: Per CIE S 017:2020 (17-27-077), beam angle is defined as a
    /// **full angle**, not a half angle. This function returns the full angle.
    ///
    /// This can differ significantly from the IES (max-based) definition for luminaires
    /// with "batwing" distributions where center-beam intensity is less than
    /// maximum intensity.
    pub fn beam_angle_cie(ldt: &Eulumdat) -> f64 {
        // Return full angle (2× half angle) per CIE definition
        Self::angle_at_percentage_of_center(ldt, 0.5) * 2.0
    }

    /// Calculate field angle using CIE definition (center-beam intensity).
    ///
    /// Uses the CIE/NEMA definition: angle between directions where intensity
    /// is 10% of the **center-beam** intensity.
    ///
    /// **Important**: Per CIE S 017:2020, field angle is defined as a **full angle**,
    /// not a half angle. This function returns the full angle.
    pub fn field_angle_cie(ldt: &Eulumdat) -> f64 {
        // Return full angle (2× half angle) per CIE definition
        Self::angle_at_percentage_of_center(ldt, 0.1) * 2.0
    }

    /// Calculate half beam angle (angle from nadir to 50% intensity).
    ///
    /// This returns the **half angle** from nadir (0°) to where intensity drops
    /// to 50% of maximum. For the full beam angle per CIE definition, use `beam_angle()`.
    ///
    /// This is useful for cone diagrams and coverage calculations where the
    /// half-angle is needed.
    pub fn half_beam_angle(ldt: &Eulumdat) -> f64 {
        Self::angle_at_percentage(ldt, 0.5)
    }

    /// Calculate half field angle (angle from nadir to 10% intensity).
    ///
    /// This returns the **half angle** from nadir (0°) to where intensity drops
    /// to 10% of maximum. For the full field angle per CIE definition, use `field_angle()`.
    pub fn half_field_angle(ldt: &Eulumdat) -> f64 {
        Self::angle_at_percentage(ldt, 0.1)
    }

    /// Get detailed beam/field angle analysis comparing IES and CIE definitions.
    ///
    /// Returns a `BeamFieldAnalysis` struct containing:
    /// - Beam and field angles using both IES (max) and CIE (center-beam) definitions
    /// - Maximum intensity and center-beam intensity values
    /// - Whether the distribution has a "batwing" pattern (center < max)
    ///
    /// This is useful for understanding luminaires like the examples in the
    /// Wikipedia "Beam angle" article where the two definitions give different results.
    pub fn beam_field_analysis(ldt: &Eulumdat) -> BeamFieldAnalysis {
        if ldt.intensities.is_empty() || ldt.g_angles.is_empty() {
            return BeamFieldAnalysis::default();
        }

        let intensities = &ldt.intensities[0];
        let max_intensity = intensities.iter().copied().fold(0.0, f64::max);
        let center_intensity = intensities.first().copied().unwrap_or(0.0);

        // Find the gamma angle of maximum intensity
        let max_gamma = ldt
            .g_angles
            .iter()
            .zip(intensities.iter())
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(g, _)| *g)
            .unwrap_or(0.0);

        let is_batwing = center_intensity < max_intensity * 0.95;

        BeamFieldAnalysis {
            // IES definition (based on maximum intensity) - full angles per CIE S 017:2020
            beam_angle_ies: Self::angle_at_percentage(ldt, 0.5) * 2.0,
            field_angle_ies: Self::angle_at_percentage(ldt, 0.1) * 2.0,
            // CIE definition (based on center-beam intensity) - full angles per CIE S 017:2020
            beam_angle_cie: Self::angle_at_percentage_of_center(ldt, 0.5) * 2.0,
            field_angle_cie: Self::angle_at_percentage_of_center(ldt, 0.1) * 2.0,
            // Reference intensities
            max_intensity,
            center_intensity,
            max_intensity_gamma: max_gamma,
            // Distribution type
            is_batwing,
            // Threshold values for diagram overlays
            beam_threshold_ies: max_intensity * 0.5,
            beam_threshold_cie: center_intensity * 0.5,
            field_threshold_ies: max_intensity * 0.1,
            field_threshold_cie: center_intensity * 0.1,
        }
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

    /// Find the angle at which intensity drops to a given percentage of center-beam intensity.
    ///
    /// This implements the CIE/NEMA definition which uses center-beam (nadir) intensity
    /// as the reference rather than maximum intensity.
    fn angle_at_percentage_of_center(ldt: &Eulumdat, percentage: f64) -> f64 {
        if ldt.intensities.is_empty() || ldt.g_angles.is_empty() {
            return 0.0;
        }

        let intensities = &ldt.intensities[0];
        let center_intensity = intensities.first().copied().unwrap_or(0.0);

        if center_intensity <= 0.0 {
            // If center intensity is zero, fall back to max-based calculation
            return Self::angle_at_percentage(ldt, percentage);
        }

        let threshold = center_intensity * percentage;

        // Find where intensity drops below threshold
        for (i, &intensity) in intensities.iter().enumerate() {
            if intensity < threshold && i > 0 {
                let prev_intensity = intensities[i - 1];
                let prev_angle = ldt.g_angles[i - 1];
                let curr_angle = ldt.g_angles[i];

                if prev_intensity > threshold {
                    let ratio = (prev_intensity - threshold) / (prev_intensity - intensity);
                    return prev_angle + ratio * (curr_angle - prev_angle);
                }
            }
        }

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

    // ========================================================================
    // CIE Flux Codes
    // ========================================================================

    /// Calculate CIE Flux Codes.
    ///
    /// Returns a tuple of 5 values (N1, N2, N3, N4, N5) representing the
    /// percentage of lamp flux in different angular zones:
    /// - N1: % in lower hemisphere (0-90°)
    /// - N2: % in 0-60° zone
    /// - N3: % in 0-40° zone
    /// - N4: % in upper hemisphere (90-180°)
    /// - N5: % in 90-120° zone (near-horizontal uplight)
    ///
    /// The flux code is typically written as: N1 N2 N3 N4 N5
    /// Example: "92 68 42 8 3" means 92% downward, 68% within 60°, etc.
    pub fn cie_flux_codes(ldt: &Eulumdat) -> CieFluxCodes {
        let total = Self::total_output(ldt);
        if total <= 0.0 {
            return CieFluxCodes::default();
        }

        // Calculate flux in each zone
        let flux_40 = Self::downward_flux(ldt, 40.0);
        let flux_60 = Self::downward_flux(ldt, 60.0);
        let flux_90 = Self::downward_flux(ldt, 90.0);
        let flux_120 = Self::downward_flux(ldt, 120.0);
        let flux_180 = Self::downward_flux(ldt, 180.0);

        CieFluxCodes {
            n1: flux_90,            // 0-90° (DLOR)
            n2: flux_60,            // 0-60°
            n3: flux_40,            // 0-40°
            n4: flux_180 - flux_90, // 90-180° (ULOR)
            n5: flux_120 - flux_90, // 90-120° (near-horizontal uplight)
        }
    }

    // ========================================================================
    // Luminaire Efficacy
    // ========================================================================

    /// Calculate luminaire efficacy in lm/W.
    ///
    /// This differs from lamp efficacy by accounting for the Light Output Ratio (LOR).
    /// Luminaire efficacy = (lamp flux × LOR) / system watts
    ///
    /// # Returns
    /// Luminaire efficacy in lumens per watt (lm/W)
    pub fn luminaire_efficacy(ldt: &Eulumdat) -> f64 {
        let total_watts = ldt.total_wattage();
        if total_watts <= 0.0 {
            return 0.0;
        }

        let lamp_flux = ldt.total_luminous_flux();
        let lor = ldt.light_output_ratio / 100.0;

        (lamp_flux * lor) / total_watts
    }

    /// Calculate luminaire efficiency (same as LOR but calculated from intensities).
    ///
    /// Compares calculated luminous flux to rated lamp flux.
    ///
    /// # Returns
    /// Efficiency as a percentage (0-100)
    pub fn luminaire_efficiency(ldt: &Eulumdat) -> f64 {
        let lamp_flux = ldt.total_luminous_flux();
        if lamp_flux <= 0.0 {
            return 0.0;
        }

        let calculated_flux = Self::calculated_luminous_flux(ldt);
        (calculated_flux / lamp_flux) * 100.0
    }

    // ========================================================================
    // Spacing Criterion (S/H Ratio)
    // ========================================================================

    /// Calculate the spacing criterion (S/H ratio) for uniform illumination.
    ///
    /// The spacing criterion indicates the maximum ratio of luminaire spacing
    /// to mounting height that will provide reasonably uniform illumination.
    ///
    /// # Arguments
    /// * `ldt` - The Eulumdat data
    /// * `c_plane` - The C-plane to analyze (typically 0 or 90)
    ///
    /// # Returns
    /// The spacing to height ratio (typically 1.0 to 2.0)
    pub fn spacing_criterion(ldt: &Eulumdat, c_plane: f64) -> f64 {
        if ldt.intensities.is_empty() || ldt.g_angles.is_empty() {
            return 1.0;
        }

        // Find intensity at nadir (0°)
        let i_nadir = crate::symmetry::SymmetryHandler::get_intensity_at(ldt, c_plane, 0.0);
        if i_nadir <= 0.0 {
            return 1.0;
        }

        // Find angle where intensity drops to 50% of nadir
        let threshold = i_nadir * 0.5;
        let mut half_angle = 0.0;

        for g in 0..90 {
            let intensity =
                crate::symmetry::SymmetryHandler::get_intensity_at(ldt, c_plane, g as f64);
            if intensity < threshold {
                // Interpolate
                let prev_intensity = crate::symmetry::SymmetryHandler::get_intensity_at(
                    ldt,
                    c_plane,
                    (g - 1) as f64,
                );
                if prev_intensity > threshold {
                    let ratio = (prev_intensity - threshold) / (prev_intensity - intensity);
                    half_angle = (g - 1) as f64 + ratio;
                }
                break;
            }
            half_angle = g as f64;
        }

        // S/H = 2 * tan(half_angle)
        // Typical values: narrow beam = 0.8-1.0, wide beam = 1.5-2.0
        let s_h = 2.0 * (half_angle * PI / 180.0).tan();

        // Clamp to reasonable range
        s_h.clamp(0.5, 3.0)
    }

    /// Calculate spacing criteria for both principal planes.
    ///
    /// # Returns
    /// (S/H parallel, S/H perpendicular) - spacing ratios for C0 and C90 planes
    pub fn spacing_criteria(ldt: &Eulumdat) -> (f64, f64) {
        let s_h_parallel = Self::spacing_criterion(ldt, 0.0);
        let s_h_perpendicular = Self::spacing_criterion(ldt, 90.0);
        (s_h_parallel, s_h_perpendicular)
    }

    // ========================================================================
    // Standard Zonal Lumens
    // ========================================================================

    /// Calculate luminous flux in standard angular zones.
    ///
    /// Returns flux percentages in 10° zones from 0° to 180°.
    ///
    /// # Returns
    /// Array of 18 values representing % flux in each 10° zone
    pub fn zonal_lumens_10deg(ldt: &Eulumdat) -> [f64; 18] {
        let mut zones = [0.0; 18];
        let total = Self::total_output(ldt);

        if total <= 0.0 {
            return zones;
        }

        let mut prev_flux = 0.0;
        for (i, zone) in zones.iter_mut().enumerate() {
            let angle = ((i + 1) * 10) as f64;
            let cumulative = Self::downward_flux(ldt, angle);
            *zone = cumulative - prev_flux;
            prev_flux = cumulative;
        }

        zones
    }

    /// Calculate luminous flux in standard 30° zones.
    ///
    /// # Returns
    /// ZonalLumens30 struct with flux in each 30° zone
    pub fn zonal_lumens_30deg(ldt: &Eulumdat) -> ZonalLumens30 {
        let total = Self::total_output(ldt);

        if total <= 0.0 {
            return ZonalLumens30::default();
        }

        let f30 = Self::downward_flux(ldt, 30.0);
        let f60 = Self::downward_flux(ldt, 60.0);
        let f90 = Self::downward_flux(ldt, 90.0);
        let f120 = Self::downward_flux(ldt, 120.0);
        let f150 = Self::downward_flux(ldt, 150.0);
        let f180 = Self::downward_flux(ldt, 180.0);

        ZonalLumens30 {
            zone_0_30: f30,
            zone_30_60: f60 - f30,
            zone_60_90: f90 - f60,
            zone_90_120: f120 - f90,
            zone_120_150: f150 - f120,
            zone_150_180: f180 - f150,
        }
    }

    // ========================================================================
    // K-Factor (Utilance)
    // ========================================================================

    /// Calculate the K-factor (utilance) for a room.
    ///
    /// K = (E_avg × A) / Φ_lamp
    ///
    /// Where:
    /// - E_avg = average illuminance on work plane
    /// - A = room area
    /// - Φ_lamp = total lamp flux
    ///
    /// # Arguments
    /// * `ldt` - The Eulumdat data
    /// * `room_index` - Room index k = (L×W) / (H_m × (L+W))
    /// * `reflectances` - (ceiling, wall, floor) reflectances as decimals
    ///
    /// # Returns
    /// K-factor (utilance) as a decimal (0-1)
    pub fn k_factor(ldt: &Eulumdat, room_index: f64, reflectances: (f64, f64, f64)) -> f64 {
        // Use direct ratio as base
        let room_index_idx = Self::room_index_to_idx(room_index);
        let direct_ratios = Self::calculate_direct_ratios(ldt, "1.25");
        let direct = direct_ratios[room_index_idx];

        // Apply reflection factors (simplified model)
        let (rho_c, rho_w, rho_f) = reflectances;

        // Indirect component depends on room reflectances
        let avg_reflectance = (rho_c + rho_w + rho_f) / 3.0;
        let indirect_factor = avg_reflectance / (1.0 - avg_reflectance);

        // Simplified: K ≈ direct × (1 + indirect_factor × upward_fraction)
        let upward_fraction = 1.0 - (ldt.downward_flux_fraction / 100.0);

        direct * (1.0 + indirect_factor * upward_fraction * 0.5)
    }

    /// Convert room index to array index for direct ratio lookup.
    fn room_index_to_idx(room_index: f64) -> usize {
        // Room indices: 0.60, 0.80, 1.00, 1.25, 1.50, 2.00, 2.50, 3.00, 4.00, 5.00
        let indices = [0.60, 0.80, 1.00, 1.25, 1.50, 2.00, 2.50, 3.00, 4.00, 5.00];

        for (i, &k) in indices.iter().enumerate() {
            if room_index <= k {
                return i;
            }
        }
        9 // Return last index if room_index > 5.0
    }

    // ========================================================================
    // Full UGR Calculation
    // ========================================================================

    /// Calculate UGR (Unified Glare Rating) for a specific room configuration.
    ///
    /// UGR = 8 × log₁₀((0.25/Lb) × Σ(L²×ω/p²))
    ///
    /// Where:
    /// - Lb = background luminance
    /// - L = luminaire luminance in direction of observer
    /// - ω = solid angle of luminaire
    /// - p = position index
    ///
    /// # Arguments
    /// * `ldt` - The Eulumdat data
    /// * `params` - UGR calculation parameters
    ///
    /// # Returns
    /// UGR value (typically 10-30, lower is better)
    pub fn ugr(ldt: &Eulumdat, params: &UgrParams) -> f64 {
        let luminaire_area = (ldt.length / 1000.0) * (ldt.width / 1000.0);
        if luminaire_area <= 0.0 {
            return 0.0;
        }

        // Background luminance (simplified: based on room reflectance and illuminance)
        let lb = params.background_luminance();

        let mut sum = 0.0;

        // Calculate for each luminaire position
        for pos in &params.luminaire_positions {
            // Viewing angle from observer to luminaire
            let dx = pos.0 - params.observer_x;
            let dy = pos.1 - params.observer_y;
            let dz = params.mounting_height - params.eye_height;

            let horizontal_dist = (dx * dx + dy * dy).sqrt();
            let viewing_angle = (horizontal_dist / dz).atan() * 180.0 / PI;

            // Get luminance in viewing direction
            let c_angle = dy.atan2(dx) * 180.0 / PI;
            let c_angle = if c_angle < 0.0 {
                c_angle + 360.0
            } else {
                c_angle
            };

            let intensity =
                crate::symmetry::SymmetryHandler::get_intensity_at(ldt, c_angle, viewing_angle);

            // Luminance = I / A (cd/m²)
            let luminance = intensity * 1000.0 / luminaire_area; // Convert from cd/klm

            // Solid angle
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
            let omega = luminaire_area / (dist * dist);

            // Position index (Guth position index, simplified)
            let p = Self::guth_position_index(viewing_angle, horizontal_dist, dz);

            if p > 0.0 {
                sum += (luminance * luminance * omega) / (p * p);
            }
        }

        if sum <= 0.0 || lb <= 0.0 {
            return 0.0;
        }

        8.0 * (0.25 * sum / lb).log10()
    }

    /// Calculate Guth position index.
    fn guth_position_index(gamma: f64, h: f64, v: f64) -> f64 {
        // Simplified Guth position index
        // Based on viewing angle and geometry
        let t = if v > 0.0 { h / v } else { 1.0 };

        // Simplified approximation: increases with viewing angle
        let p = 1.0 + (gamma / 90.0).powf(2.0) * t;
        p.max(1.0)
    }
}

// ============================================================================
// PhotometricSummary - Complete calculated metrics
// ============================================================================

/// Complete photometric summary with all calculated values.
///
/// This struct provides a comprehensive overview of luminaire performance
/// that can be used for reports, GLDF export, or display.
#[derive(Debug, Clone, Default)]
pub struct PhotometricSummary {
    // Flux and efficiency
    /// Total lamp flux (lm)
    pub total_lamp_flux: f64,
    /// Calculated flux from intensity integration (lm)
    pub calculated_flux: f64,
    /// Light Output Ratio (%)
    pub lor: f64,
    /// Downward Light Output Ratio (%)
    pub dlor: f64,
    /// Upward Light Output Ratio (%)
    pub ulor: f64,

    // Efficacy
    /// Lamp efficacy (lm/W)
    pub lamp_efficacy: f64,
    /// Luminaire efficacy (lm/W)
    pub luminaire_efficacy: f64,
    /// Total system wattage (W)
    pub total_wattage: f64,

    // CIE Flux Codes
    /// CIE flux codes (N1-N5)
    pub cie_flux_codes: CieFluxCodes,

    // Beam characteristics (IES definition - based on maximum intensity)
    /// Beam angle - 50% of max intensity (degrees) - IES definition
    pub beam_angle: f64,
    /// Field angle - 10% of max intensity (degrees) - IES definition
    pub field_angle: f64,

    // Beam characteristics (CIE definition - based on center-beam intensity)
    /// Beam angle - 50% of center intensity (degrees) - CIE definition
    pub beam_angle_cie: f64,
    /// Field angle - 10% of center intensity (degrees) - CIE definition
    pub field_angle_cie: f64,
    /// True if distribution is batwing (center < max, IES ≠ CIE)
    pub is_batwing: bool,

    // Intensity statistics
    /// Maximum intensity (cd/klm)
    pub max_intensity: f64,
    /// Minimum intensity (cd/klm)
    pub min_intensity: f64,
    /// Average intensity (cd/klm)
    pub avg_intensity: f64,

    // Spacing criterion
    /// S/H ratio for C0 plane
    pub spacing_c0: f64,
    /// S/H ratio for C90 plane
    pub spacing_c90: f64,

    // Zonal lumens
    /// Zonal lumens in 30° zones
    pub zonal_lumens: ZonalLumens30,
}

impl PhotometricSummary {
    /// Calculate complete photometric summary from Eulumdat data.
    pub fn from_eulumdat(ldt: &Eulumdat) -> Self {
        let cie_codes = PhotometricCalculations::cie_flux_codes(ldt);
        let (s_c0, s_c90) = PhotometricCalculations::spacing_criteria(ldt);

        Self {
            // Flux
            total_lamp_flux: ldt.total_luminous_flux(),
            calculated_flux: PhotometricCalculations::calculated_luminous_flux(ldt),
            lor: ldt.light_output_ratio,
            dlor: ldt.downward_flux_fraction,
            ulor: 100.0 - ldt.downward_flux_fraction,

            // Efficacy
            lamp_efficacy: ldt.luminous_efficacy(),
            luminaire_efficacy: PhotometricCalculations::luminaire_efficacy(ldt),
            total_wattage: ldt.total_wattage(),

            // CIE
            cie_flux_codes: cie_codes,

            // Beam (IES definition)
            beam_angle: PhotometricCalculations::beam_angle(ldt),
            field_angle: PhotometricCalculations::field_angle(ldt),

            // Beam (CIE definition)
            beam_angle_cie: PhotometricCalculations::beam_angle_cie(ldt),
            field_angle_cie: PhotometricCalculations::field_angle_cie(ldt),
            is_batwing: {
                let analysis = PhotometricCalculations::beam_field_analysis(ldt);
                analysis.is_batwing
            },

            // Intensity
            max_intensity: ldt.max_intensity(),
            min_intensity: ldt.min_intensity(),
            avg_intensity: ldt.avg_intensity(),

            // Spacing
            spacing_c0: s_c0,
            spacing_c90: s_c90,

            // Zonal
            zonal_lumens: PhotometricCalculations::zonal_lumens_30deg(ldt),
        }
    }

    /// Format as multi-line text report.
    pub fn to_text(&self) -> String {
        format!(
            r#"PHOTOMETRIC SUMMARY
==================

Luminous Flux
  Total Lamp Flux:     {:.0} lm
  Calculated Flux:     {:.0} lm
  LOR:                 {:.1}%
  DLOR / ULOR:         {:.1}% / {:.1}%

Efficacy
  Lamp Efficacy:       {:.1} lm/W
  Luminaire Efficacy:  {:.1} lm/W
  Total Wattage:       {:.1} W

CIE Flux Code:         {}

Beam Characteristics
  Beam Angle (50%):    {:.1}°
  Field Angle (10%):   {:.1}°

Intensity (cd/klm)
  Maximum:             {:.1}
  Minimum:             {:.1}
  Average:             {:.1}

Spacing Criterion (S/H)
  C0 Plane:            {:.2}
  C90 Plane:           {:.2}

Zonal Lumens (%)
  0-30°:               {:.1}%
  30-60°:              {:.1}%
  60-90°:              {:.1}%
  90-120°:             {:.1}%
  120-150°:            {:.1}%
  150-180°:            {:.1}%
"#,
            self.total_lamp_flux,
            self.calculated_flux,
            self.lor,
            self.dlor,
            self.ulor,
            self.lamp_efficacy,
            self.luminaire_efficacy,
            self.total_wattage,
            self.cie_flux_codes,
            self.beam_angle,
            self.field_angle,
            self.max_intensity,
            self.min_intensity,
            self.avg_intensity,
            self.spacing_c0,
            self.spacing_c90,
            self.zonal_lumens.zone_0_30,
            self.zonal_lumens.zone_30_60,
            self.zonal_lumens.zone_60_90,
            self.zonal_lumens.zone_90_120,
            self.zonal_lumens.zone_120_150,
            self.zonal_lumens.zone_150_180,
        )
    }

    /// Format as single-line compact summary.
    pub fn to_compact(&self) -> String {
        format!(
            "CIE:{} Beam:{:.0}° Field:{:.0}° Eff:{:.0}lm/W S/H:{:.1}×{:.1}",
            self.cie_flux_codes,
            self.beam_angle,
            self.field_angle,
            self.luminaire_efficacy,
            self.spacing_c0,
            self.spacing_c90,
        )
    }

    /// Format as key-value pairs for machine parsing.
    pub fn to_key_value(&self) -> Vec<(&'static str, String)> {
        vec![
            ("total_lamp_flux_lm", format!("{:.1}", self.total_lamp_flux)),
            ("calculated_flux_lm", format!("{:.1}", self.calculated_flux)),
            ("lor_percent", format!("{:.1}", self.lor)),
            ("dlor_percent", format!("{:.1}", self.dlor)),
            ("ulor_percent", format!("{:.1}", self.ulor)),
            ("lamp_efficacy_lm_w", format!("{:.1}", self.lamp_efficacy)),
            (
                "luminaire_efficacy_lm_w",
                format!("{:.1}", self.luminaire_efficacy),
            ),
            ("total_wattage_w", format!("{:.1}", self.total_wattage)),
            ("cie_flux_code", self.cie_flux_codes.to_string()),
            ("cie_n1", format!("{:.1}", self.cie_flux_codes.n1)),
            ("cie_n2", format!("{:.1}", self.cie_flux_codes.n2)),
            ("cie_n3", format!("{:.1}", self.cie_flux_codes.n3)),
            ("cie_n4", format!("{:.1}", self.cie_flux_codes.n4)),
            ("cie_n5", format!("{:.1}", self.cie_flux_codes.n5)),
            ("beam_angle_deg", format!("{:.1}", self.beam_angle)),
            ("field_angle_deg", format!("{:.1}", self.field_angle)),
            ("max_intensity_cd_klm", format!("{:.1}", self.max_intensity)),
            ("min_intensity_cd_klm", format!("{:.1}", self.min_intensity)),
            ("avg_intensity_cd_klm", format!("{:.1}", self.avg_intensity)),
            ("spacing_c0", format!("{:.2}", self.spacing_c0)),
            ("spacing_c90", format!("{:.2}", self.spacing_c90)),
            (
                "zonal_0_30_percent",
                format!("{:.1}", self.zonal_lumens.zone_0_30),
            ),
            (
                "zonal_30_60_percent",
                format!("{:.1}", self.zonal_lumens.zone_30_60),
            ),
            (
                "zonal_60_90_percent",
                format!("{:.1}", self.zonal_lumens.zone_60_90),
            ),
            (
                "zonal_90_120_percent",
                format!("{:.1}", self.zonal_lumens.zone_90_120),
            ),
            (
                "zonal_120_150_percent",
                format!("{:.1}", self.zonal_lumens.zone_120_150),
            ),
            (
                "zonal_150_180_percent",
                format!("{:.1}", self.zonal_lumens.zone_150_180),
            ),
        ]
    }
}

impl std::fmt::Display for PhotometricSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_text())
    }
}

// ============================================================================
// GLDF-Compatible Photometric Data
// ============================================================================

/// GLDF-compatible photometric data export.
///
/// Contains all properties required by the GLDF (Global Lighting Data Format)
/// specification for photometric data exchange.
#[derive(Debug, Clone, Default)]
pub struct GldfPhotometricData {
    /// CIE Flux Code (e.g., "45 72 95 100 100")
    pub cie_flux_code: String,
    /// Light Output Ratio - total efficiency (%)
    pub light_output_ratio: f64,
    /// Luminous efficacy (lm/W)
    pub luminous_efficacy: f64,
    /// Downward Flux Fraction (%)
    pub downward_flux_fraction: f64,
    /// Downward Light Output Ratio (%)
    pub downward_light_output_ratio: f64,
    /// Upward Light Output Ratio (%)
    pub upward_light_output_ratio: f64,
    /// Luminaire luminance (cd/m²) - average luminance at 65° viewing angle
    pub luminaire_luminance: f64,
    /// Cut-off angle - angle where intensity drops below 2.5% (degrees)
    pub cut_off_angle: f64,
    /// UGR table values for standard room (4H/8H, 0.70/0.50/0.20)
    pub ugr_4h_8h_705020: Option<UgrTableValues>,
    /// Photometric classification code
    pub photometric_code: String,
    /// Tenth peak (field) divergence angles (C0-C180, C90-C270) in degrees
    pub tenth_peak_divergence: (f64, f64),
    /// Half peak (beam) divergence angles (C0-C180, C90-C270) in degrees
    pub half_peak_divergence: (f64, f64),
    /// BUG rating (Backlight, Uplight, Glare)
    pub light_distribution_bug_rating: String,
}

/// UGR table values for GLDF export
#[derive(Debug, Clone, Default)]
pub struct UgrTableValues {
    /// UGR crosswise (C90) looking direction
    pub crosswise: f64,
    /// UGR endwise (C0) looking direction
    pub endwise: f64,
}

/// IES LM-63-19 specific metadata for GLDF integration.
///
/// Contains information from IES files that doesn't map directly to GLDF
/// `DescriptivePhotometry` but is valuable for data provenance, validation,
/// and geometry definition.
#[derive(Debug, Clone, Default)]
pub struct IesMetadata {
    /// IES file format version
    pub version: String,
    /// Test report number (`[TEST]` keyword)
    pub test_report: String,
    /// Photometric testing laboratory (`[TESTLAB]`)
    pub test_lab: String,
    /// Date manufacturer issued the file (`[ISSUEDATE]`)
    pub issue_date: String,
    /// Manufacturer of luminaire (`[MANUFAC]`)
    pub manufacturer: String,
    /// Luminaire catalog number (`[LUMCAT]`)
    pub luminaire_catalog: String,
    /// Lamp catalog number (`[LAMPCAT]`)
    pub lamp_catalog: String,
    /// Ballast description (`[BALLAST]`)
    pub ballast: String,

    /// File generation type (LM-63-19 Section 5.13)
    pub file_generation_type: String,
    /// Whether data is from an accredited test lab
    pub is_accredited: bool,
    /// Whether luminous flux values were scaled
    pub is_scaled: bool,
    /// Whether angle values were interpolated
    pub is_interpolated: bool,
    /// Whether data is from computer simulation
    pub is_simulation: bool,

    /// Luminous opening shape (LM-63-19 Table 1)
    pub luminous_shape: String,
    /// Luminous opening width in meters (absolute value, negative in IES = curved)
    pub luminous_width: f64,
    /// Luminous opening length in meters
    pub luminous_length: f64,
    /// Luminous opening height in meters
    pub luminous_height: f64,
    /// Whether the shape is rectangular (positive dims) or curved (negative dims)
    pub is_rectangular: bool,
    /// Whether the shape is circular (|width| == |length|, both negative)
    pub is_circular: bool,

    /// Photometric type (A/B/C)
    pub photometric_type: String,
    /// Unit type (Feet/Meters)
    pub unit_type: String,

    /// TILT information present
    pub has_tilt_data: bool,
    /// Lamp geometry (1-3) if TILT data present
    pub tilt_lamp_geometry: i32,
    /// Number of TILT angle/factor pairs
    pub tilt_angle_count: usize,

    /// IES maintenance category (1-6)
    pub maintenance_category: Option<i32>,
    /// Ballast factor
    pub ballast_factor: f64,
    /// Input watts
    pub input_watts: f64,
    /// Number of lamps
    pub num_lamps: i32,
    /// Lumens per lamp (-1 = absolute photometry)
    pub lumens_per_lamp: f64,
    /// Is this absolute photometry (lumens = -1)?
    pub is_absolute_photometry: bool,
}

impl IesMetadata {
    /// Create IesMetadata from parsed IesData.
    pub fn from_ies_data(ies: &crate::ies::IesData) -> Self {
        use crate::ies::{FileGenerationType, LuminousShape, PhotometricType, UnitType};

        let shape = &ies.luminous_shape;
        let gen_type = &ies.file_generation_type;

        Self {
            version: ies.version.header().to_string(),
            test_report: ies.test.clone(),
            test_lab: ies.test_lab.clone(),
            issue_date: ies.issue_date.clone(),
            manufacturer: ies.manufacturer.clone(),
            luminaire_catalog: ies.luminaire_catalog.clone(),
            lamp_catalog: ies.lamp_catalog.clone(),
            ballast: ies.ballast.clone(),

            file_generation_type: gen_type.title().to_string(),
            is_accredited: gen_type.is_accredited(),
            is_scaled: gen_type.is_scaled(),
            is_interpolated: gen_type.is_interpolated(),
            is_simulation: matches!(gen_type, FileGenerationType::ComputerSimulation),

            luminous_shape: shape.description().to_string(),
            luminous_width: ies.width.abs(),
            luminous_length: ies.length.abs(),
            luminous_height: ies.height.abs(),
            is_rectangular: matches!(
                shape,
                LuminousShape::Rectangular | LuminousShape::RectangularWithSides
            ),
            is_circular: matches!(
                shape,
                LuminousShape::Circular | LuminousShape::Sphere | LuminousShape::VerticalCylinder
            ),

            photometric_type: match ies.photometric_type {
                PhotometricType::TypeC => "C".to_string(),
                PhotometricType::TypeB => "B".to_string(),
                PhotometricType::TypeA => "A".to_string(),
            },
            unit_type: match ies.unit_type {
                UnitType::Feet => "Feet".to_string(),
                UnitType::Meters => "Meters".to_string(),
            },

            has_tilt_data: ies.tilt_data.is_some(),
            tilt_lamp_geometry: ies.tilt_data.as_ref().map_or(0, |t| t.lamp_geometry),
            tilt_angle_count: ies.tilt_data.as_ref().map_or(0, |t| t.angles.len()),

            maintenance_category: ies.maintenance_category,
            ballast_factor: ies.ballast_factor,
            input_watts: ies.input_watts,
            num_lamps: ies.num_lamps,
            lumens_per_lamp: ies.lumens_per_lamp,
            is_absolute_photometry: ies.lumens_per_lamp < 0.0,
        }
    }

    /// Export as key-value pairs for GLDF integration.
    pub fn to_gldf_properties(&self) -> Vec<(&'static str, String)> {
        let mut props = vec![];

        if !self.version.is_empty() {
            props.push(("ies_version", self.version.clone()));
        }
        if !self.test_report.is_empty() {
            props.push(("test_report", self.test_report.clone()));
        }
        if !self.test_lab.is_empty() {
            props.push(("test_lab", self.test_lab.clone()));
        }
        if !self.issue_date.is_empty() {
            props.push(("issue_date", self.issue_date.clone()));
        }
        if !self.manufacturer.is_empty() {
            props.push(("manufacturer", self.manufacturer.clone()));
        }
        if !self.luminaire_catalog.is_empty() {
            props.push(("luminaire_catalog", self.luminaire_catalog.clone()));
        }

        props.push(("file_generation_type", self.file_generation_type.clone()));
        props.push(("is_accredited", self.is_accredited.to_string()));
        props.push(("is_scaled", self.is_scaled.to_string()));
        props.push(("is_interpolated", self.is_interpolated.to_string()));
        props.push(("is_simulation", self.is_simulation.to_string()));

        props.push(("luminous_shape", self.luminous_shape.clone()));
        if self.luminous_width > 0.0 {
            props.push(("luminous_width_m", format!("{:.4}", self.luminous_width)));
        }
        if self.luminous_length > 0.0 {
            props.push(("luminous_length_m", format!("{:.4}", self.luminous_length)));
        }
        if self.luminous_height > 0.0 {
            props.push(("luminous_height_m", format!("{:.4}", self.luminous_height)));
        }

        props.push(("photometric_type", self.photometric_type.clone()));

        if self.has_tilt_data {
            props.push(("has_tilt_data", "true".to_string()));
            props.push(("tilt_lamp_geometry", self.tilt_lamp_geometry.to_string()));
            props.push(("tilt_angle_count", self.tilt_angle_count.to_string()));
        }

        if let Some(cat) = self.maintenance_category {
            props.push(("maintenance_category", cat.to_string()));
        }

        if self.ballast_factor != 1.0 {
            props.push(("ballast_factor", format!("{:.3}", self.ballast_factor)));
        }

        props.push(("input_watts", format!("{:.1}", self.input_watts)));
        props.push(("num_lamps", self.num_lamps.to_string()));

        if self.is_absolute_photometry {
            props.push(("absolute_photometry", "true".to_string()));
        } else {
            props.push(("lumens_per_lamp", format!("{:.1}", self.lumens_per_lamp)));
        }

        props
    }

    /// Get GLDF-compatible emitter geometry info.
    ///
    /// Returns (shape_type, width_mm, length_mm, diameter_mm) for GLDF SimpleGeometry.
    pub fn to_gldf_emitter_geometry(&self) -> (&'static str, i32, i32, i32) {
        let width_mm = (self.luminous_width * 1000.0).round() as i32;
        let length_mm = (self.luminous_length * 1000.0).round() as i32;
        let diameter_mm = width_mm.max(length_mm);

        if self.is_circular {
            ("circular", 0, 0, diameter_mm)
        } else if self.is_rectangular {
            ("rectangular", width_mm, length_mm, 0)
        } else {
            ("point", 0, 0, 0)
        }
    }
}

impl GldfPhotometricData {
    /// Calculate GLDF-compatible photometric data from Eulumdat.
    pub fn from_eulumdat(ldt: &Eulumdat) -> Self {
        let cie_codes = PhotometricCalculations::cie_flux_codes(ldt);
        let bug = crate::bug_rating::BugRating::from_eulumdat(ldt);

        // Calculate beam/field angles for both planes
        let beam_c0 = PhotometricCalculations::beam_angle_for_plane(ldt, 0.0);
        let beam_c90 = PhotometricCalculations::beam_angle_for_plane(ldt, 90.0);
        let field_c0 = PhotometricCalculations::field_angle_for_plane(ldt, 0.0);
        let field_c90 = PhotometricCalculations::field_angle_for_plane(ldt, 90.0);

        // Calculate luminaire luminance at 65° (standard viewing angle)
        let luminance = PhotometricCalculations::luminaire_luminance(ldt, 65.0);

        // Calculate cut-off angle (where intensity < 2.5% of max)
        let cut_off = PhotometricCalculations::cut_off_angle(ldt);

        // Calculate UGR for standard room configuration
        let ugr_values = PhotometricCalculations::ugr_table_values(ldt);

        // Generate photometric classification code
        let photo_code = PhotometricCalculations::photometric_code(ldt);

        Self {
            cie_flux_code: cie_codes.to_string(),
            light_output_ratio: ldt.light_output_ratio,
            luminous_efficacy: PhotometricCalculations::luminaire_efficacy(ldt),
            downward_flux_fraction: ldt.downward_flux_fraction,
            downward_light_output_ratio: cie_codes.n1 * ldt.light_output_ratio / 100.0,
            upward_light_output_ratio: cie_codes.n4 * ldt.light_output_ratio / 100.0,
            luminaire_luminance: luminance,
            cut_off_angle: cut_off,
            ugr_4h_8h_705020: ugr_values,
            photometric_code: photo_code,
            tenth_peak_divergence: (field_c0, field_c90),
            half_peak_divergence: (beam_c0, beam_c90),
            light_distribution_bug_rating: format!("{}", bug),
        }
    }

    /// Export as key-value pairs for GLDF XML generation.
    pub fn to_gldf_properties(&self) -> Vec<(&'static str, String)> {
        let mut props = vec![
            ("cie_flux_code", self.cie_flux_code.clone()),
            (
                "light_output_ratio",
                format!("{:.1}", self.light_output_ratio),
            ),
            (
                "luminous_efficacy",
                format!("{:.1}", self.luminous_efficacy),
            ),
            (
                "downward_flux_fraction",
                format!("{:.1}", self.downward_flux_fraction),
            ),
            (
                "downward_light_output_ratio",
                format!("{:.1}", self.downward_light_output_ratio),
            ),
            (
                "upward_light_output_ratio",
                format!("{:.1}", self.upward_light_output_ratio),
            ),
            (
                "luminaire_luminance",
                format!("{:.0}", self.luminaire_luminance),
            ),
            ("cut_off_angle", format!("{:.1}", self.cut_off_angle)),
            ("photometric_code", self.photometric_code.clone()),
            (
                "tenth_peak_divergence",
                format!(
                    "{:.1} / {:.1}",
                    self.tenth_peak_divergence.0, self.tenth_peak_divergence.1
                ),
            ),
            (
                "half_peak_divergence",
                format!(
                    "{:.1} / {:.1}",
                    self.half_peak_divergence.0, self.half_peak_divergence.1
                ),
            ),
            (
                "light_distribution_bug_rating",
                self.light_distribution_bug_rating.clone(),
            ),
        ];

        if let Some(ref ugr) = self.ugr_4h_8h_705020 {
            props.push((
                "ugr_4h_8h_705020_crosswise",
                format!("{:.1}", ugr.crosswise),
            ));
            props.push(("ugr_4h_8h_705020_endwise", format!("{:.1}", ugr.endwise)));
        }

        props
    }

    /// Export as formatted text report.
    pub fn to_text(&self) -> String {
        let mut s = String::from("GLDF PHOTOMETRIC DATA\n");
        s.push_str("=====================\n\n");

        s.push_str(&format!(
            "CIE Flux Code:           {}\n",
            self.cie_flux_code
        ));
        s.push_str(&format!(
            "Light Output Ratio:      {:.1}%\n",
            self.light_output_ratio
        ));
        s.push_str(&format!(
            "Luminous Efficacy:       {:.1} lm/W\n",
            self.luminous_efficacy
        ));
        s.push_str(&format!(
            "Downward Flux Fraction:  {:.1}%\n",
            self.downward_flux_fraction
        ));
        s.push_str(&format!(
            "DLOR:                    {:.1}%\n",
            self.downward_light_output_ratio
        ));
        s.push_str(&format!(
            "ULOR:                    {:.1}%\n",
            self.upward_light_output_ratio
        ));
        s.push_str(&format!(
            "Luminaire Luminance:     {:.0} cd/m²\n",
            self.luminaire_luminance
        ));
        s.push_str(&format!(
            "Cut-off Angle:           {:.1}°\n",
            self.cut_off_angle
        ));

        if let Some(ref ugr) = self.ugr_4h_8h_705020 {
            s.push_str(&format!(
                "UGR (4H×8H, 70/50/20):   C: {:.1} / E: {:.1}\n",
                ugr.crosswise, ugr.endwise
            ));
        }

        s.push_str(&format!(
            "Photometric Code:        {}\n",
            self.photometric_code
        ));
        s.push_str(&format!(
            "Half Peak Divergence:    {:.1}° / {:.1}° (C0/C90)\n",
            self.half_peak_divergence.0, self.half_peak_divergence.1
        ));
        s.push_str(&format!(
            "Tenth Peak Divergence:   {:.1}° / {:.1}° (C0/C90)\n",
            self.tenth_peak_divergence.0, self.tenth_peak_divergence.1
        ));
        s.push_str(&format!(
            "BUG Rating:              {}\n",
            self.light_distribution_bug_rating
        ));

        s
    }
}

impl std::fmt::Display for GldfPhotometricData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_text())
    }
}

// ============================================================================
// Additional GLDF-related Calculations
// ============================================================================

impl PhotometricCalculations {
    /// Calculate beam angle (50% intensity) for a specific C-plane.
    ///
    /// Returns the **full angle** per CIE S 017:2020 definition.
    pub fn beam_angle_for_plane(ldt: &Eulumdat, c_plane: f64) -> f64 {
        // Return full angle (2× half angle) per CIE definition
        Self::angle_at_percentage_for_plane(ldt, c_plane, 0.5) * 2.0
    }

    /// Calculate field angle (10% intensity) for a specific C-plane.
    ///
    /// Returns the **full angle** per CIE S 017:2020 definition.
    pub fn field_angle_for_plane(ldt: &Eulumdat, c_plane: f64) -> f64 {
        // Return full angle (2× half angle) per CIE definition
        Self::angle_at_percentage_for_plane(ldt, c_plane, 0.1) * 2.0
    }

    /// Calculate half beam angle for a specific C-plane.
    ///
    /// Returns the half angle from nadir to 50% intensity point.
    pub fn half_beam_angle_for_plane(ldt: &Eulumdat, c_plane: f64) -> f64 {
        Self::angle_at_percentage_for_plane(ldt, c_plane, 0.5)
    }

    /// Calculate half field angle for a specific C-plane.
    ///
    /// Returns the half angle from nadir to 10% intensity point.
    pub fn half_field_angle_for_plane(ldt: &Eulumdat, c_plane: f64) -> f64 {
        Self::angle_at_percentage_for_plane(ldt, c_plane, 0.1)
    }

    /// Find the angle at which intensity drops to a percentage for a specific plane.
    fn angle_at_percentage_for_plane(ldt: &Eulumdat, c_plane: f64, percentage: f64) -> f64 {
        if ldt.g_angles.is_empty() {
            return 0.0;
        }

        let i_nadir = crate::symmetry::SymmetryHandler::get_intensity_at(ldt, c_plane, 0.0);
        if i_nadir <= 0.0 {
            return 0.0;
        }

        let threshold = i_nadir * percentage;

        for g in 1..90 {
            let intensity =
                crate::symmetry::SymmetryHandler::get_intensity_at(ldt, c_plane, g as f64);
            if intensity < threshold {
                // Interpolate
                let prev = crate::symmetry::SymmetryHandler::get_intensity_at(
                    ldt,
                    c_plane,
                    (g - 1) as f64,
                );
                if prev > threshold && prev > intensity {
                    let ratio = (prev - threshold) / (prev - intensity);
                    return (g - 1) as f64 + ratio;
                }
                return g as f64;
            }
        }

        90.0
    }

    /// Calculate luminaire luminance at a given viewing angle (cd/m²).
    ///
    /// L = I / A_projected
    /// Where A_projected is the luminous area projected in viewing direction.
    pub fn luminaire_luminance(ldt: &Eulumdat, viewing_angle: f64) -> f64 {
        // Get luminous area in m²
        let la_length = ldt.luminous_area_length / 1000.0;
        let la_width = ldt.luminous_area_width / 1000.0;

        if la_length <= 0.0 || la_width <= 0.0 {
            return 0.0;
        }

        let area = la_length * la_width;

        // Projected area at viewing angle
        let angle_rad = viewing_angle.to_radians();
        let projected_area = area * angle_rad.cos();

        if projected_area <= 0.001 {
            return 0.0;
        }

        // Average intensity at this angle across planes
        let i_c0 = crate::symmetry::SymmetryHandler::get_intensity_at(ldt, 0.0, viewing_angle);
        let i_c90 = crate::symmetry::SymmetryHandler::get_intensity_at(ldt, 90.0, viewing_angle);
        let avg_intensity = (i_c0 + i_c90) / 2.0;

        // Convert from cd/klm to actual cd using total flux
        let total_flux = ldt.total_luminous_flux();
        let actual_intensity = avg_intensity * total_flux / 1000.0;

        // Luminance = I / A
        actual_intensity / projected_area
    }

    /// Calculate cut-off angle (where intensity drops below 2.5% of maximum).
    pub fn cut_off_angle(ldt: &Eulumdat) -> f64 {
        let max_intensity = ldt.max_intensity();
        if max_intensity <= 0.0 {
            return 90.0;
        }

        let threshold = max_intensity * 0.025;

        // Search from 90° downward to find where intensity first exceeds threshold
        for g in (0..=90).rev() {
            let i_c0 = crate::symmetry::SymmetryHandler::get_intensity_at(ldt, 0.0, g as f64);
            let i_c90 = crate::symmetry::SymmetryHandler::get_intensity_at(ldt, 90.0, g as f64);

            if i_c0 > threshold || i_c90 > threshold {
                return g as f64;
            }
        }

        0.0
    }

    /// Calculate UGR table values for standard room configuration.
    ///
    /// Standard configuration: 4H×8H room, reflectances 0.70/0.50/0.20
    pub fn ugr_table_values(ldt: &Eulumdat) -> Option<UgrTableValues> {
        let luminaire_area = (ldt.length / 1000.0) * (ldt.width / 1000.0);
        if luminaire_area <= 0.0 {
            return None;
        }

        // Standard room: 4H wide × 8H long, where H is mounting height
        // Typical mounting height for calculation: 2.5m
        let h = 2.5;
        let room_width = 4.0 * h; // 10m
        let room_length = 8.0 * h; // 20m

        // Standard reflectances
        let rho_c = 0.70;
        let rho_w = 0.50;
        let rho_f = 0.20;

        // Calculate for crosswise (looking along C90 direction)
        let params_cross = UgrParams {
            room_length,
            room_width,
            mounting_height: 2.8,
            eye_height: 1.2,
            observer_x: room_length / 2.0,
            observer_y: 1.5, // Near wall, looking across
            luminaire_positions: vec![
                (room_length / 4.0, room_width / 2.0),
                (room_length / 2.0, room_width / 2.0),
                (3.0 * room_length / 4.0, room_width / 2.0),
            ],
            rho_ceiling: rho_c,
            rho_wall: rho_w,
            rho_floor: rho_f,
            illuminance: 500.0,
        };

        // Calculate for endwise (looking along C0 direction)
        let params_end = UgrParams {
            room_length,
            room_width,
            mounting_height: 2.8,
            eye_height: 1.2,
            observer_x: 1.5, // Near wall, looking along
            observer_y: room_width / 2.0,
            luminaire_positions: vec![
                (room_length / 4.0, room_width / 2.0),
                (room_length / 2.0, room_width / 2.0),
                (3.0 * room_length / 4.0, room_width / 2.0),
            ],
            rho_ceiling: rho_c,
            rho_wall: rho_w,
            rho_floor: rho_f,
            illuminance: 500.0,
        };

        let ugr_cross = Self::ugr(ldt, &params_cross);
        let ugr_end = Self::ugr(ldt, &params_end);

        // Only return if we got valid values
        if ugr_cross > 0.0 || ugr_end > 0.0 {
            Some(UgrTableValues {
                crosswise: ugr_cross.max(0.0),
                endwise: ugr_end.max(0.0),
            })
        } else {
            None
        }
    }

    /// Generate photometric classification code.
    ///
    /// Format: D/I where:
    /// - D = Distribution type (1=direct, 2=semi-direct, 3=general diffuse, 4=semi-indirect, 5=indirect)
    /// - I = Intensity class based on max intensity
    pub fn photometric_code(ldt: &Eulumdat) -> String {
        let dlor = ldt.downward_flux_fraction;

        // Distribution type based on downward flux fraction
        let dist_type = if dlor >= 90.0 {
            "D" // Direct
        } else if dlor >= 60.0 {
            "SD" // Semi-direct
        } else if dlor >= 40.0 {
            "GD" // General diffuse
        } else if dlor >= 10.0 {
            "SI" // Semi-indirect
        } else {
            "I" // Indirect
        };

        // Beam classification (using full angle per CIE S 017:2020)
        let beam_angle = Self::beam_angle(ldt);
        let beam_class = if beam_angle < 40.0 {
            "VN" // Very narrow (< 20° half angle)
        } else if beam_angle < 60.0 {
            "N" // Narrow (20-30° half angle)
        } else if beam_angle < 90.0 {
            "M" // Medium (30-45° half angle)
        } else if beam_angle < 120.0 {
            "W" // Wide (45-60° half angle)
        } else {
            "VW" // Very wide (> 60° half angle)
        };

        format!("{}-{}", dist_type, beam_class)
    }
}

// ============================================================================
// Supporting Types
// ============================================================================

/// Comprehensive beam and field angle analysis.
///
/// Compares IES (maximum intensity based) and CIE (center-beam intensity based)
/// definitions of beam angle and field angle. This is important for luminaires
/// with non-standard distributions like "batwing" patterns.
///
/// See Wikipedia "Beam angle" article for the distinction between these definitions.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BeamFieldAnalysis {
    // IES definition (based on maximum intensity)
    /// Beam angle using IES definition (50% of max intensity) in degrees
    pub beam_angle_ies: f64,
    /// Field angle using IES definition (10% of max intensity) in degrees
    pub field_angle_ies: f64,

    // CIE/NEMA definition (based on center-beam intensity)
    /// Beam angle using CIE definition (50% of center-beam intensity) in degrees
    pub beam_angle_cie: f64,
    /// Field angle using CIE definition (10% of center-beam intensity) in degrees
    pub field_angle_cie: f64,

    // Reference intensity values
    /// Maximum intensity anywhere in the distribution (cd/klm)
    pub max_intensity: f64,
    /// Center-beam intensity at nadir/0° gamma (cd/klm)
    pub center_intensity: f64,
    /// Gamma angle at which maximum intensity occurs (degrees)
    pub max_intensity_gamma: f64,

    // Distribution type
    /// True if this is a "batwing" distribution (center < max)
    pub is_batwing: bool,

    // Threshold values for diagram overlays
    /// 50% of max intensity (IES beam threshold)
    pub beam_threshold_ies: f64,
    /// 50% of center intensity (CIE beam threshold)
    pub beam_threshold_cie: f64,
    /// 10% of max intensity (IES field threshold)
    pub field_threshold_ies: f64,
    /// 10% of center intensity (CIE field threshold)
    pub field_threshold_cie: f64,
}

impl BeamFieldAnalysis {
    /// Returns the difference between CIE and IES beam angles.
    ///
    /// A large positive value indicates a batwing distribution where
    /// the CIE definition gives a wider beam angle.
    pub fn beam_angle_difference(&self) -> f64 {
        self.beam_angle_cie - self.beam_angle_ies
    }

    /// Returns the difference between CIE and IES field angles.
    pub fn field_angle_difference(&self) -> f64 {
        self.field_angle_cie - self.field_angle_ies
    }

    /// Returns the ratio of center intensity to max intensity.
    ///
    /// A value less than 1.0 indicates a batwing or off-axis peak distribution.
    pub fn center_to_max_ratio(&self) -> f64 {
        if self.max_intensity > 0.0 {
            self.center_intensity / self.max_intensity
        } else {
            0.0
        }
    }

    /// Get descriptive classification of the distribution type.
    pub fn distribution_type(&self) -> &'static str {
        let ratio = self.center_to_max_ratio();
        if ratio >= 0.95 {
            "Standard (center-peak)"
        } else if ratio >= 0.7 {
            "Mild batwing"
        } else if ratio >= 0.4 {
            "Moderate batwing"
        } else {
            "Strong batwing"
        }
    }

    /// Format for display with both IES and CIE values.
    pub fn to_string_detailed(&self) -> String {
        format!(
            "Beam: IES {:.1}° / CIE {:.1}° (Δ{:+.1}°)\n\
             Field: IES {:.1}° / CIE {:.1}° (Δ{:+.1}°)\n\
             Center/Max: {:.1}% ({})",
            self.beam_angle_ies,
            self.beam_angle_cie,
            self.beam_angle_difference(),
            self.field_angle_ies,
            self.field_angle_cie,
            self.field_angle_difference(),
            self.center_to_max_ratio() * 100.0,
            self.distribution_type()
        )
    }
}

impl std::fmt::Display for BeamFieldAnalysis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Beam: {:.1}° (IES) / {:.1}° (CIE), Field: {:.1}° (IES) / {:.1}° (CIE)",
            self.beam_angle_ies, self.beam_angle_cie, self.field_angle_ies, self.field_angle_cie
        )
    }
}

/// CIE Flux Code values
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CieFluxCodes {
    /// N1: % flux in lower hemisphere (0-90°) - equivalent to DLOR
    pub n1: f64,
    /// N2: % flux in 0-60° zone
    pub n2: f64,
    /// N3: % flux in 0-40° zone
    pub n3: f64,
    /// N4: % flux in upper hemisphere (90-180°) - equivalent to ULOR
    pub n4: f64,
    /// N5: % flux in 90-120° zone (near-horizontal uplight)
    pub n5: f64,
}

impl std::fmt::Display for CieFluxCodes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:.0} {:.0} {:.0} {:.0} {:.0}",
            self.n1.round(),
            self.n2.round(),
            self.n3.round(),
            self.n4.round(),
            self.n5.round()
        )
    }
}

/// Zonal lumens in 30° zones
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ZonalLumens30 {
    /// 0-30° zone (nadir to 30°)
    pub zone_0_30: f64,
    /// 30-60° zone
    pub zone_30_60: f64,
    /// 60-90° zone (approaching horizontal)
    pub zone_60_90: f64,
    /// 90-120° zone (above horizontal)
    pub zone_90_120: f64,
    /// 120-150° zone
    pub zone_120_150: f64,
    /// 150-180° zone (zenith region)
    pub zone_150_180: f64,
}

impl ZonalLumens30 {
    /// Get total downward flux (0-90°)
    pub fn downward_total(&self) -> f64 {
        self.zone_0_30 + self.zone_30_60 + self.zone_60_90
    }

    /// Get total upward flux (90-180°)
    pub fn upward_total(&self) -> f64 {
        self.zone_90_120 + self.zone_120_150 + self.zone_150_180
    }
}

/// Parameters for UGR calculation
#[derive(Debug, Clone)]
pub struct UgrParams {
    /// Room length (m)
    pub room_length: f64,
    /// Room width (m)
    pub room_width: f64,
    /// Mounting height above floor (m)
    pub mounting_height: f64,
    /// Observer eye height (m), typically 1.2m seated, 1.7m standing
    pub eye_height: f64,
    /// Observer X position (m)
    pub observer_x: f64,
    /// Observer Y position (m)
    pub observer_y: f64,
    /// Luminaire positions as (x, y) tuples
    pub luminaire_positions: Vec<(f64, f64)>,
    /// Ceiling reflectance (0-1)
    pub rho_ceiling: f64,
    /// Wall reflectance (0-1)
    pub rho_wall: f64,
    /// Floor reflectance (0-1)
    pub rho_floor: f64,
    /// Target illuminance (lux)
    pub illuminance: f64,
}

impl Default for UgrParams {
    fn default() -> Self {
        Self {
            room_length: 8.0,
            room_width: 4.0,
            mounting_height: 2.8,
            eye_height: 1.2,
            observer_x: 4.0,
            observer_y: 2.0,
            luminaire_positions: vec![(2.0, 2.0), (6.0, 2.0)],
            rho_ceiling: 0.7,
            rho_wall: 0.5,
            rho_floor: 0.2,
            illuminance: 500.0,
        }
    }
}

impl UgrParams {
    /// Calculate background luminance from room parameters
    pub fn background_luminance(&self) -> f64 {
        // Lb = E × ρ_avg / π
        let avg_rho = (self.rho_ceiling + self.rho_wall + self.rho_floor) / 3.0;
        self.illuminance * avg_rho / PI
    }

    /// Create params for a standard office room
    pub fn standard_office() -> Self {
        Self {
            room_length: 6.0,
            room_width: 4.0,
            mounting_height: 2.8,
            eye_height: 1.2,
            observer_x: 3.0,
            observer_y: 2.0,
            luminaire_positions: vec![(2.0, 2.0), (4.0, 2.0)],
            rho_ceiling: 0.7,
            rho_wall: 0.5,
            rho_floor: 0.2,
            illuminance: 500.0,
        }
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
        // Beam angle is full angle per CIE S 017:2020, should be positive and <= 180°
        assert!(beam > 0.0 && beam <= 180.0, "Beam angle was {}", beam);

        // Half beam angle should be half of full angle
        let half_beam = PhotometricCalculations::half_beam_angle(&ldt);
        assert!(
            (beam - half_beam * 2.0).abs() < 0.01,
            "Half beam should be half of full beam"
        );
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

    #[test]
    fn test_cie_flux_codes() {
        let ldt = create_test_ldt();
        let codes = PhotometricCalculations::cie_flux_codes(&ldt);

        // For a downlight, most flux should be in lower hemisphere
        assert!(
            codes.n1 > 50.0,
            "N1 (DLOR) should be > 50% for downlight, got {}",
            codes.n1
        );
        assert!(
            codes.n4 < 50.0,
            "N4 (ULOR) should be < 50% for downlight, got {}",
            codes.n4
        );

        // N3 < N2 < N1 (flux accumulates with angle)
        assert!(codes.n3 <= codes.n2, "N3 should be <= N2");
        assert!(codes.n2 <= codes.n1, "N2 should be <= N1");

        // Test display format
        let display = format!("{}", codes);
        assert!(!display.is_empty());
    }

    #[test]
    fn test_luminaire_efficacy() {
        let mut ldt = create_test_ldt();
        ldt.light_output_ratio = 80.0; // 80% LOR

        let lamp_efficacy = ldt.luminous_efficacy();
        let luminaire_efficacy = PhotometricCalculations::luminaire_efficacy(&ldt);

        // Luminaire efficacy should be less than lamp efficacy due to LOR
        assert!(luminaire_efficacy > 0.0);
        assert!(luminaire_efficacy <= lamp_efficacy);
        assert!((luminaire_efficacy - lamp_efficacy * 0.8).abs() < 0.01);
    }

    #[test]
    fn test_spacing_criterion() {
        let ldt = create_test_ldt();
        let s_h = PhotometricCalculations::spacing_criterion(&ldt, 0.0);

        // S/H should be in reasonable range
        assert!((0.5..=3.0).contains(&s_h), "S/H was {}", s_h);

        // Test both planes
        let (s_h_par, s_h_perp) = PhotometricCalculations::spacing_criteria(&ldt);
        assert!(s_h_par > 0.0);
        assert!(s_h_perp > 0.0);
    }

    #[test]
    fn test_zonal_lumens() {
        let ldt = create_test_ldt();

        // Test 10° zones
        let zones_10 = PhotometricCalculations::zonal_lumens_10deg(&ldt);
        let total_10: f64 = zones_10.iter().sum();
        assert!(
            (total_10 - 100.0).abs() < 1.0,
            "Total should be ~100%, got {}",
            total_10
        );

        // Test 30° zones
        let zones_30 = PhotometricCalculations::zonal_lumens_30deg(&ldt);
        let total_30 = zones_30.downward_total() + zones_30.upward_total();
        assert!(
            (total_30 - 100.0).abs() < 1.0,
            "Total should be ~100%, got {}",
            total_30
        );

        // For a downlight, most flux should be downward
        assert!(zones_30.downward_total() > zones_30.upward_total());
    }

    #[test]
    fn test_k_factor() {
        let mut ldt = create_test_ldt();
        ldt.downward_flux_fraction = 90.0;

        let k = PhotometricCalculations::k_factor(&ldt, 1.0, (0.7, 0.5, 0.2));

        // K-factor should be between 0 and 1.5
        assert!((0.0..=1.5).contains(&k), "K-factor was {}", k);
    }

    #[test]
    fn test_ugr_calculation() {
        let mut ldt = create_test_ldt();
        ldt.length = 600.0; // 600mm
        ldt.width = 600.0; // 600mm
                           // Lower intensity for more realistic UGR
        ldt.intensities = vec![vec![200.0, 196.0, 180.0, 150.0, 100.0, 40.0, 10.0]];

        let params = UgrParams::standard_office();
        let ugr = PhotometricCalculations::ugr(&ldt, &params);

        // UGR should be positive (calculation works)
        assert!(ugr >= 0.0, "UGR should be >= 0, got {}", ugr);
        // UGR is calculated - specific value depends on geometry
        // Real-world values are typically 10-30 for office luminaires
    }

    #[test]
    fn test_ugr_params() {
        let params = UgrParams::default();
        let lb = params.background_luminance();
        assert!(lb > 0.0, "Background luminance should be positive");

        let office = UgrParams::standard_office();
        assert_eq!(office.illuminance, 500.0);
    }

    #[test]
    fn test_gldf_photometric_data() {
        let mut ldt = create_test_ldt();
        ldt.light_output_ratio = 85.0;
        ldt.downward_flux_fraction = 95.0;
        ldt.luminous_area_length = 600.0;
        ldt.luminous_area_width = 600.0;
        ldt.length = 620.0;
        ldt.width = 620.0;

        let gldf = GldfPhotometricData::from_eulumdat(&ldt);

        // Check basic values
        assert_eq!(gldf.light_output_ratio, 85.0);
        assert_eq!(gldf.downward_flux_fraction, 95.0);

        // Check calculated values
        assert!(gldf.luminous_efficacy > 0.0);
        assert!(gldf.downward_light_output_ratio > 0.0);
        assert!(gldf.cut_off_angle > 0.0);

        // Check photometric code
        assert!(!gldf.photometric_code.is_empty());
        assert!(gldf.photometric_code.contains('-'));

        // Check text output
        let text = gldf.to_text();
        assert!(text.contains("GLDF PHOTOMETRIC DATA"));
        assert!(text.contains("CIE Flux Code"));
        assert!(text.contains("BUG Rating"));

        // Check key-value export
        let props = gldf.to_gldf_properties();
        assert!(props.len() >= 12);
        assert!(props.iter().any(|(k, _)| *k == "cie_flux_code"));
        assert!(props.iter().any(|(k, _)| *k == "half_peak_divergence"));
    }

    #[test]
    fn test_photometric_summary() {
        let mut ldt = create_test_ldt();
        ldt.light_output_ratio = 85.0;
        ldt.downward_flux_fraction = 90.0;

        let summary = PhotometricSummary::from_eulumdat(&ldt);

        // Check basic values
        assert_eq!(summary.total_lamp_flux, 1000.0);
        assert_eq!(summary.lor, 85.0);
        assert_eq!(summary.dlor, 90.0);
        assert_eq!(summary.ulor, 10.0);

        // Check efficacy
        assert!(summary.lamp_efficacy > 0.0);
        assert!(summary.luminaire_efficacy > 0.0);
        assert!(summary.luminaire_efficacy <= summary.lamp_efficacy);

        // Check beam angles (both should be positive)
        assert!(summary.beam_angle > 0.0);
        assert!(summary.field_angle > 0.0);

        // Check text output
        let text = summary.to_text();
        assert!(text.contains("PHOTOMETRIC SUMMARY"));
        assert!(text.contains("CIE Flux Code"));

        // Check compact output
        let compact = summary.to_compact();
        assert!(compact.contains("CIE:"));
        assert!(compact.contains("Beam:"));

        // Check key-value output
        let kv = summary.to_key_value();
        assert!(!kv.is_empty());
        assert!(kv.iter().any(|(k, _)| *k == "beam_angle_deg"));
    }
}
