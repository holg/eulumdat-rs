//! Convert detector output to Eulumdat struct for .ldt/.ies export.

use crate::detector::Detector;
use eulumdat::{Eulumdat, LampSet, Symmetry};

/// Configuration for Eulumdat export.
#[derive(Debug, Clone)]
pub struct ExportConfig {
    /// C-plane interval in degrees (default 15.0).
    pub c_step_deg: f64,
    /// Gamma angle interval in degrees (default 5.0).
    pub g_step_deg: f64,
    /// Force a symmetry type, or `None` for auto-detect.
    pub symmetry: Option<Symmetry>,
    /// Luminaire name.
    pub luminaire_name: String,
    /// Manufacturer / identification.
    pub manufacturer: String,
    /// Luminaire dimensions in mm: (length, width, height).
    pub luminaire_dimensions_mm: (f64, f64, f64),
    /// Luminous area in mm: (length, width).
    pub luminous_area_mm: (f64, f64),
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            c_step_deg: 15.0,
            g_step_deg: 5.0,
            symmetry: None,
            luminaire_name: "Simulated Luminaire".to_string(),
            manufacturer: "eulumdat-goniosim".to_string(),
            luminaire_dimensions_mm: (100.0, 100.0, 50.0),
            luminous_area_mm: (80.0, 80.0),
        }
    }
}

/// Build an Eulumdat struct from detector data.
///
/// `source_flux_lm` is the flux that was actually emitted into the scene
/// (luminaire output flux = lamp flux * LOR). This is used for candela
/// normalization in the detector.
///
/// `lamp_flux_lm` is the total lamp flux (before LOR). This is used for
/// the cd/klm conversion, since EULUMDAT intensities are per 1000 lm of
/// lamp flux, not luminaire output flux.
///
/// If `lamp_flux_lm` is None, `source_flux_lm` is used for both.
pub fn detector_to_eulumdat(
    detector: &Detector,
    source_flux_lm: f64,
    config: &ExportConfig,
) -> Eulumdat {
    detector_to_eulumdat_with_lamp_flux(detector, source_flux_lm, source_flux_lm, config)
}

/// Build an Eulumdat struct from detector data, with explicit lamp flux.
pub fn detector_to_eulumdat_with_lamp_flux(
    detector: &Detector,
    source_flux_lm: f64,
    lamp_flux_lm: f64,
    config: &ExportConfig,
) -> Eulumdat {
    detector_to_eulumdat_at_angles(
        detector,
        source_flux_lm,
        lamp_flux_lm,
        None, // use uniform grid from config
        None,
        config,
    )
}

/// Build an Eulumdat struct from detector data, sampling at explicit C/G angles.
///
/// If `c_angles` or `g_angles` is None, uses uniform grid from config.
/// This enables exact reproduction of non-uniform C-plane spacing from the source LDT.
pub fn detector_to_eulumdat_at_angles(
    detector: &Detector,
    source_flux_lm: f64,
    lamp_flux_lm: f64,
    c_angles_opt: Option<&[f64]>,
    g_angles_opt: Option<&[f64]>,
    config: &ExportConfig,
) -> Eulumdat {
    // Determine the angle grid to use
    let c_angles: Vec<f64> = match c_angles_opt {
        Some(angles) => angles.to_vec(),
        None => {
            let num_c = (360.0 / config.c_step_deg).round() as usize;
            (0..num_c).map(|i| i as f64 * config.c_step_deg).collect()
        }
    };

    let g_angles: Vec<f64> = match g_angles_opt {
        Some(angles) => angles.to_vec(),
        None => {
            let num_g = (180.0 / config.g_step_deg).round() as usize + 1;
            (0..num_g).map(|i| i as f64 * config.g_step_deg).collect()
        }
    };

    let num_c = c_angles.len();
    let num_g = g_angles.len();

    // Build intensity grid.
    // If explicit angles were provided, use the fine-resolution detector's candela grid
    // and extract values at the closest bin (nearest-neighbor), which avoids the
    // solid-angle interpolation bias of candela_at().
    let scale = 1000.0 / lamp_flux_lm.max(1.0);

    let intensities: Vec<Vec<f64>> = if c_angles_opt.is_some() || g_angles_opt.is_some() {
        // Get full candela grid at detector resolution
        let full_cd = detector.to_candela(source_flux_lm);
        let det_c_res = detector.c_resolution_deg();
        let det_g_res = detector.g_resolution_deg();
        let det_num_c = detector.num_c();
        let det_num_g = detector.num_g();

        c_angles
            .iter()
            .map(|&c| {
                // Find nearest C-bin
                let c_norm = c.rem_euclid(360.0);
                let ci = ((c_norm / det_c_res).round() as usize).min(det_num_c - 1);
                g_angles
                    .iter()
                    .map(|&g| {
                        let gi = ((g / det_g_res).round() as usize).min(det_num_g - 1);
                        full_cd[ci][gi] * scale
                    })
                    .collect()
            })
            .collect()
    } else {
        // Uniform grid — use resample for best accuracy
        let resampled = detector.resample(config.c_step_deg, config.g_step_deg);
        let candela = resampled.to_candela(source_flux_lm);
        candela
            .iter()
            .map(|c_plane| c_plane.iter().map(|cd| cd * scale).collect())
            .collect()
    };

    // Compute downward flux fraction from detector data
    let downward_energy: f64 = {
        let det_bins = detector.bins();
        let mut down = 0.0;
        let mut total = 0.0;
        for ci in 0..detector.num_c() {
            for gi in 0..detector.num_g() {
                let g_deg = gi as f64 * detector.g_resolution_deg();
                let e = det_bins[ci][gi];
                total += e;
                if g_deg <= 90.0 {
                    down += e;
                }
            }
        }
        if total > 0.0 { 100.0 * down / total } else { 50.0 }
    };

    // Handle symmetry: reduce C-planes for symmetric sources so that
    // downstream flux integration produces correct results.
    let symmetry = config.symmetry.unwrap_or(Symmetry::None);

    let (c_angles, intensities) = match symmetry {
        Symmetry::VerticalAxis => {
            // Rotationally symmetric: average ALL C-planes into one.
            // The integration for VerticalAxis multiplies by 2*pi.
            let mut avg = vec![0.0; num_g];
            for gi in 0..num_g {
                let sum: f64 = intensities.iter().map(|cp| cp[gi]).sum();
                avg[gi] = sum / num_c as f64;
            }
            (vec![0.0], vec![avg])
        }
        _ => (c_angles, intensities),
    };
    let num_c = c_angles.len();

    let mut ldt = Eulumdat::new();
    ldt.identification = config.manufacturer.clone();
    ldt.luminaire_name = config.luminaire_name.clone();
    ldt.luminaire_number = String::new();
    ldt.file_name = String::new();
    ldt.date_user = String::new();
    ldt.measurement_report_number = "GonioSim".to_string();

    ldt.symmetry = symmetry;
    ldt.num_c_planes = num_c;
    // If explicit angles were provided, compute spacing from them (0 = non-uniform)
    ldt.c_plane_distance = if c_angles_opt.is_some() && num_c > 1 {
        let d = c_angles[1] - c_angles[0];
        if c_angles.windows(2).all(|w| (w[1] - w[0] - d).abs() < 0.01) { d } else { 0.0 }
    } else {
        config.c_step_deg
    };
    ldt.num_g_planes = num_g;
    ldt.g_plane_distance = if g_angles_opt.is_some() && num_g > 1 {
        let d = g_angles[1] - g_angles[0];
        if g_angles.windows(2).all(|w| (w[1] - w[0] - d).abs() < 0.01) { d } else { 0.0 }
    } else {
        config.g_step_deg
    };

    ldt.length = config.luminaire_dimensions_mm.0;
    ldt.width = config.luminaire_dimensions_mm.1;
    ldt.height = config.luminaire_dimensions_mm.2;
    ldt.luminous_area_length = config.luminous_area_mm.0;
    ldt.luminous_area_width = config.luminous_area_mm.1;

    ldt.downward_flux_fraction = downward_energy;
    ldt.light_output_ratio = 100.0; // simulated = 100% (losses are in the simulation)
    ldt.conversion_factor = 1.0;
    ldt.tilt_angle = 0.0;

    ldt.lamp_sets = vec![LampSet {
        num_lamps: 1,
        lamp_type: "LED".to_string(),
        total_luminous_flux: source_flux_lm,
        color_appearance: "4000K".to_string(),
        color_rendering_group: "1A".to_string(),
        wattage_with_ballast: source_flux_lm / 150.0, // assume ~150 lm/W
        ..LampSet::default()
    }];

    ldt.direct_ratios = [0.0; 10];
    ldt.c_angles = c_angles;
    ldt.g_angles = g_angles;
    ldt.intensities = intensities;

    ldt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_produces_valid_ldt() {
        let mut detector = Detector::new(15.0, 5.0);
        // Simulate some isotropic data
        for ci in 0..detector.num_c() {
            for gi in 0..detector.num_g() {
                let dir = cg_to_direction(
                    ci as f64 * 15.0,
                    gi as f64 * 5.0,
                );
                detector.record(&dir, 1.0);
            }
        }

        let config = ExportConfig::default();
        let ldt = detector_to_eulumdat(&detector, 1000.0, &config);

        assert_eq!(ldt.luminaire_name, "Simulated Luminaire");
        assert!(!ldt.intensities.is_empty());
        assert!(!ldt.c_angles.is_empty());
        assert!(!ldt.g_angles.is_empty());

        // Should produce valid LDT string
        let ldt_string = ldt.to_ldt();
        assert!(!ldt_string.is_empty());
    }

    /// Helper: convert C/gamma angles back to a direction vector.
    fn cg_to_direction(c_deg: f64, g_deg: f64) -> nalgebra::Vector3<f64> {
        let g_rad = g_deg.to_radians();
        let c_rad = c_deg.to_radians();
        nalgebra::Vector3::new(
            g_rad.sin() * c_rad.cos(),
            g_rad.sin() * c_rad.sin(),
            -g_rad.cos(),
        )
    }
}
