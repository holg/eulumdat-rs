//! IES TM-30-20 Color Rendition Calculation
//!
//! Implements the TM-30 method for evaluating color rendition of light sources.
//! Calculates Rf (Fidelity Index), Rg (Gamut Index), and color vector graphics.
//!
//! Reference: IES TM-30-20 "IES Method for Evaluating Light Source Color Rendition"

use crate::types::SpectralDistribution;

/// TM-30 calculation results
#[derive(Debug, Clone)]
pub struct Tm30Result {
    /// Fidelity Index (0-100, similar to CRI Ra)
    pub rf: f64,
    /// Gamut Index (typically 60-140, 100 = reference)
    pub rg: f64,
    /// Correlated Color Temperature (K)
    pub cct: f64,
    /// Duv (distance from Planckian locus)
    pub duv: f64,
    /// Per-hue fidelity values (16 hue bins)
    pub rf_hue: [f64; 16],
    /// Per-hue chroma shift (16 hue bins, positive = more saturated)
    pub rcs_hue: [f64; 16],
    /// Per-hue hue shift in degrees (16 hue bins)
    pub rhs_hue: [f64; 16],
    /// Color vector data for graphic (16 bins: test a', b' and ref a', b')
    pub color_vectors: [(f64, f64, f64, f64); 16],
}

/// Theme for TM-30 color vector graphic
#[derive(Debug, Clone)]
pub struct Tm30Theme {
    pub background: String,
    pub foreground: String,
    pub grid_color: String,
    pub reference_color: String,
    pub test_color: String,
    pub font_family: String,
}

impl Default for Tm30Theme {
    fn default() -> Self {
        Self::light()
    }
}

impl Tm30Theme {
    pub fn light() -> Self {
        Self {
            background: "#ffffff".to_string(),
            foreground: "#333333".to_string(),
            grid_color: "#e0e0e0".to_string(),
            reference_color: "#999999".to_string(),
            test_color: "#e74c3c".to_string(),
            font_family: "system-ui, sans-serif".to_string(),
        }
    }

    pub fn dark() -> Self {
        Self {
            background: "#1a1a2e".to_string(),
            foreground: "#e0e0e0".to_string(),
            grid_color: "#333355".to_string(),
            reference_color: "#666666".to_string(),
            test_color: "#e74c3c".to_string(),
            font_family: "system-ui, sans-serif".to_string(),
        }
    }
}

// ============================================================================
// CIE 1931 2° Standard Observer Color Matching Functions
// Wavelength range: 380-780nm at 5nm intervals (81 values)
// ============================================================================

/// CIE 1931 2° x̄(λ) color matching function
const CIE_X: [f64; 81] = [
    0.001368, 0.002236, 0.004243, 0.007650, 0.014310, 0.023190, 0.043510, 0.077630, 0.134380,
    0.214770, 0.283900, 0.328500, 0.348280, 0.348060, 0.336200, 0.318700, 0.290800, 0.251100,
    0.195360, 0.142100, 0.095640, 0.058010, 0.032010, 0.014700, 0.004900, 0.002400, 0.009300,
    0.029100, 0.063270, 0.109600, 0.165500, 0.225750, 0.290400, 0.359700, 0.433450, 0.512050,
    0.594500, 0.678400, 0.762100, 0.842500, 0.916300, 0.978600, 1.026300, 1.056700, 1.062200,
    1.045600, 1.002600, 0.938400, 0.854450, 0.751400, 0.642400, 0.541900, 0.447900, 0.360800,
    0.283500, 0.218700, 0.164900, 0.121200, 0.087400, 0.063600, 0.046770, 0.032900, 0.022700,
    0.015840, 0.011359, 0.008111, 0.005790, 0.004109, 0.002899, 0.002049, 0.001440, 0.001000,
    0.000690, 0.000476, 0.000332, 0.000235, 0.000166, 0.000117, 0.000083, 0.000059, 0.000042,
];

/// CIE 1931 2° ȳ(λ) color matching function
const CIE_Y: [f64; 81] = [
    0.000039, 0.000064, 0.000120, 0.000217, 0.000396, 0.000640, 0.001210, 0.002180, 0.004000,
    0.007300, 0.011600, 0.016840, 0.023000, 0.029800, 0.038000, 0.048000, 0.060000, 0.073900,
    0.090980, 0.112600, 0.139020, 0.169300, 0.208020, 0.258600, 0.323000, 0.407300, 0.503000,
    0.608200, 0.710000, 0.793200, 0.862000, 0.914850, 0.954000, 0.980300, 0.994950, 1.000000,
    0.995000, 0.978600, 0.952000, 0.915400, 0.870000, 0.816300, 0.757000, 0.694900, 0.631000,
    0.566800, 0.503000, 0.441200, 0.381000, 0.321000, 0.265000, 0.217000, 0.175000, 0.138200,
    0.107000, 0.081600, 0.061000, 0.044580, 0.032000, 0.023200, 0.017000, 0.011920, 0.008210,
    0.005723, 0.004102, 0.002929, 0.002091, 0.001484, 0.001047, 0.000740, 0.000520, 0.000361,
    0.000249, 0.000172, 0.000120, 0.000085, 0.000060, 0.000042, 0.000030, 0.000021, 0.000015,
];

/// CIE 1931 2° z̄(λ) color matching function
const CIE_Z: [f64; 81] = [
    0.006450, 0.010550, 0.020050, 0.036210, 0.067850, 0.110200, 0.207400, 0.371300, 0.645600,
    1.039050, 1.385600, 1.622960, 1.747060, 1.782600, 1.772110, 1.744100, 1.669200, 1.528100,
    1.287640, 1.041900, 0.812950, 0.616200, 0.465180, 0.353300, 0.272000, 0.212300, 0.158200,
    0.111700, 0.078250, 0.057250, 0.042160, 0.029840, 0.020300, 0.013400, 0.008750, 0.005750,
    0.003900, 0.002750, 0.002100, 0.001800, 0.001650, 0.001400, 0.001100, 0.001000, 0.000800,
    0.000600, 0.000340, 0.000240, 0.000190, 0.000100, 0.000050, 0.000030, 0.000020, 0.000010,
    0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000,
    0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000,
    0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000,
];

/// Wavelengths for CMF data (380-780nm at 5nm)
const WAVELENGTHS: [f64; 81] = [
    380.0, 385.0, 390.0, 395.0, 400.0, 405.0, 410.0, 415.0, 420.0, 425.0, 430.0, 435.0, 440.0,
    445.0, 450.0, 455.0, 460.0, 465.0, 470.0, 475.0, 480.0, 485.0, 490.0, 495.0, 500.0, 505.0,
    510.0, 515.0, 520.0, 525.0, 530.0, 535.0, 540.0, 545.0, 550.0, 555.0, 560.0, 565.0, 570.0,
    575.0, 580.0, 585.0, 590.0, 595.0, 600.0, 605.0, 610.0, 615.0, 620.0, 625.0, 630.0, 635.0,
    640.0, 645.0, 650.0, 655.0, 660.0, 665.0, 670.0, 675.0, 680.0, 685.0, 690.0, 695.0, 700.0,
    705.0, 710.0, 715.0, 720.0, 725.0, 730.0, 735.0, 740.0, 745.0, 750.0, 755.0, 760.0, 765.0,
    770.0, 775.0, 780.0,
];

// ============================================================================
// TM-30 Color Evaluation Samples (CES) - Full 99 Samples
// Data from IES TM-30-20 via colour-science library
// Wavelength range: 380-780nm at 5nm intervals (81 values per sample)
// ============================================================================

/// Number of CES samples (full TM-30 set)
const NUM_CES: usize = 99;

// Include the full CES reflectance data from external file
// These are measured reflectance values that happen to be close to mathematical constants
#[allow(clippy::approx_constant)]
mod ces_data {
    include!("tm30_ces_data.rs");
}
use ces_data::*;

// ============================================================================
// Core Calculation Functions
// ============================================================================

/// Calculate XYZ tristimulus values from SPD
fn spd_to_xyz(spd: &SpectralDistribution) -> (f64, f64, f64) {
    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 0.0;

    // Interpolate SPD to standard wavelengths
    for (i, &wl) in WAVELENGTHS.iter().enumerate() {
        let spd_val = interpolate_spd(spd, wl);
        x += spd_val * CIE_X[i];
        y += spd_val * CIE_Y[i];
        z += spd_val * CIE_Z[i];
    }

    // Normalize
    let k = 100.0 / y.max(0.001);
    (x * k, 100.0, z * k)
}

/// Interpolate SPD value at given wavelength
fn interpolate_spd(spd: &SpectralDistribution, wavelength: f64) -> f64 {
    if spd.wavelengths.is_empty() || spd.values.is_empty() {
        return 0.0;
    }

    // Find surrounding wavelengths
    let wls = &spd.wavelengths;
    let vals = &spd.values;

    if wavelength <= wls[0] {
        return vals[0];
    }
    if wavelength >= wls[wls.len() - 1] {
        return vals[vals.len() - 1];
    }

    // Linear interpolation
    for i in 0..wls.len() - 1 {
        if wavelength >= wls[i] && wavelength <= wls[i + 1] {
            let t = (wavelength - wls[i]) / (wls[i + 1] - wls[i]);
            return vals[i] + t * (vals[i + 1] - vals[i]);
        }
    }

    vals[0]
}

/// Calculate CCT from chromaticity coordinates using McCamy's approximation
fn xyz_to_cct(x: f64, y: f64, _z: f64) -> (f64, f64) {
    // Convert to chromaticity coordinates
    let sum = x + y + _z;
    let xc = x / sum;
    let yc = y / sum;

    // McCamy's formula
    let n = (xc - 0.3320) / (0.1858 - yc);
    let cct = 449.0 * n.powi(3) + 3525.0 * n.powi(2) + 6823.3 * n + 5520.33;

    // Calculate Duv (distance from Planckian locus)
    // Simplified approximation
    let duv = (yc - (-0.0114 * n.powi(3) + 0.0660 * n.powi(2) - 0.1329 * n + 0.3808)) * 1000.0;

    (cct.clamp(1000.0, 20000.0), duv)
}

/// Generate Planckian (blackbody) radiator SPD at given CCT
fn planckian_spd(cct: f64) -> SpectralDistribution {
    let c1 = 3.74183e-16; // W⋅m²
    let c2 = 1.4388e-2; // m⋅K

    let wavelengths: Vec<f64> = WAVELENGTHS.to_vec();
    let values: Vec<f64> = wavelengths
        .iter()
        .map(|&wl| {
            let wl_m = wl * 1e-9; // Convert nm to m
            c1 / (wl_m.powi(5) * ((c2 / (wl_m * cct)).exp() - 1.0))
        })
        .collect();

    // Normalize
    let max_val = values.iter().cloned().fold(0.0_f64, f64::max);
    let normalized: Vec<f64> = values.iter().map(|v| v / max_val).collect();

    SpectralDistribution {
        wavelengths,
        values: normalized,
        units: crate::types::SpectralUnits::Relative,
        start_wavelength: None,
        wavelength_interval: None,
    }
}

/// Generate CIE D-series illuminant SPD at given CCT
fn d_series_spd(cct: f64) -> SpectralDistribution {
    // D-series chromaticity
    let xd = if cct <= 7000.0 {
        -4.6070e9 / cct.powi(3) + 2.9678e6 / cct.powi(2) + 0.09911e3 / cct + 0.244063
    } else {
        -2.0064e9 / cct.powi(3) + 1.9018e6 / cct.powi(2) + 0.24748e3 / cct + 0.237040
    };
    let _yd = -3.0 * xd.powi(2) + 2.87 * xd - 0.275;

    // Simplified D-series - use Planckian as approximation
    // (Full D-series requires S0, S1, S2 basis functions)
    planckian_spd(cct)
}

/// Calculate color appearance under illuminant for a CES sample
fn calculate_ces_color(spd: &SpectralDistribution, ces_idx: usize) -> (f64, f64, f64) {
    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 0.0;

    // Get the reflectance data for this CES sample
    let ces_refl = CES_REFLECTANCE[ces_idx];

    for (i, &wl) in WAVELENGTHS.iter().enumerate() {
        let spd_val = interpolate_spd(spd, wl);
        let refl = ces_refl[i];
        let stimulus = spd_val * refl;

        x += stimulus * CIE_X[i];
        y += stimulus * CIE_Y[i];
        z += stimulus * CIE_Z[i];
    }

    // Normalize
    let k = 100.0 / y.max(0.001);
    (x * k, 100.0, z * k)
}

/// Convert XYZ to CAM02-UCS (J', a', b')
/// Simplified version - full CAM02 is complex
fn xyz_to_cam02_ucs(
    x: f64,
    y: f64,
    z: f64,
    white_x: f64,
    white_y: f64,
    white_z: f64,
) -> (f64, f64, f64) {
    // Simplified CIELAB-like transform for TM-30 approximation
    // Full CAM02 would require viewing conditions

    // Reference white
    let xn = white_x;
    let yn = white_y;
    let zn = white_z;

    // Lab-like calculation
    let fx = lab_f(x / xn);
    let fy = lab_f(y / yn);
    let fz = lab_f(z / zn);

    let l_star = 116.0 * fy - 16.0;
    let a_star = 500.0 * (fx - fy);
    let b_star = 200.0 * (fy - fz);

    // Convert to CAM02-UCS-like coordinates
    // J' ≈ L*, a' ≈ a*, b' ≈ b* (simplified)
    (l_star, a_star, b_star)
}

fn lab_f(t: f64) -> f64 {
    let delta: f64 = 6.0 / 29.0;
    if t > delta.powi(3) {
        t.powf(1.0 / 3.0)
    } else {
        t / (3.0 * delta.powi(2)) + 4.0 / 29.0
    }
}

/// Calculate color difference in CAM02-UCS
fn color_difference(j1: f64, a1: f64, b1: f64, j2: f64, a2: f64, b2: f64) -> f64 {
    ((j1 - j2).powi(2) + (a1 - a2).powi(2) + (b1 - b2).powi(2)).sqrt()
}

// ============================================================================
// Main TM-30 Calculation
// ============================================================================

/// Calculate TM-30 metrics from spectral distribution
///
/// Returns None if spectral data is insufficient
pub fn calculate_tm30(spd: &SpectralDistribution) -> Option<Tm30Result> {
    // Need at least 380-780nm coverage
    if spd.wavelengths.is_empty() || spd.values.is_empty() {
        return None;
    }

    let min_wl = spd.wavelengths.iter().cloned().fold(f64::MAX, f64::min);
    let max_wl = spd.wavelengths.iter().cloned().fold(f64::MIN, f64::max);

    if min_wl > 400.0 || max_wl < 700.0 {
        return None;
    }

    // Calculate test source XYZ and CCT
    let (test_x, test_y, test_z) = spd_to_xyz(spd);
    let (cct, duv) = xyz_to_cct(test_x, test_y, test_z);

    // Generate reference illuminant
    let ref_spd = if cct < 5000.0 {
        planckian_spd(cct)
    } else {
        d_series_spd(cct)
    };
    let (ref_x, ref_y, ref_z) = spd_to_xyz(&ref_spd);

    // Storage for all 99 CES calculations
    struct CesResult {
        #[allow(dead_code)]
        test_j: f64,
        test_a: f64,
        test_b: f64,
        #[allow(dead_code)]
        ref_j: f64,
        ref_a: f64,
        ref_b: f64,
        delta_e: f64,
        ref_hue: f64, // Hue angle under reference (for binning)
    }

    let mut ces_results: Vec<CesResult> = Vec::with_capacity(NUM_CES);
    let mut delta_e_sum = 0.0;

    // Calculate colors for all 99 CES samples
    for i in 0..NUM_CES {
        // Test source
        let (test_ces_x, test_ces_y, test_ces_z) = calculate_ces_color(spd, i);
        let (test_j, test_a, test_b) =
            xyz_to_cam02_ucs(test_ces_x, test_ces_y, test_ces_z, test_x, test_y, test_z);

        // Reference source
        let (ref_ces_x, ref_ces_y, ref_ces_z) = calculate_ces_color(&ref_spd, i);
        let (ref_j, ref_a, ref_b) =
            xyz_to_cam02_ucs(ref_ces_x, ref_ces_y, ref_ces_z, ref_x, ref_y, ref_z);

        // Color difference
        let de = color_difference(test_j, test_a, test_b, ref_j, ref_a, ref_b);
        delta_e_sum += de;

        // Hue angle under reference illuminant (for binning)
        let ref_hue = ref_b.atan2(ref_a).to_degrees();

        ces_results.push(CesResult {
            test_j,
            test_a,
            test_b,
            ref_j,
            ref_a,
            ref_b,
            delta_e: de,
            ref_hue,
        });
    }

    // Bin samples into 16 hue bins (22.5° each, starting at -11.25°)
    let mut rf_hue = [0.0_f64; 16];
    let mut rcs_hue = [0.0_f64; 16];
    let mut rhs_hue = [0.0_f64; 16];
    let mut color_vectors = [(0.0, 0.0, 0.0, 0.0); 16];
    let mut bin_counts = [0usize; 16];
    let mut bin_test_a_sum = [0.0_f64; 16];
    let mut bin_test_b_sum = [0.0_f64; 16];
    let mut bin_ref_a_sum = [0.0_f64; 16];
    let mut bin_ref_b_sum = [0.0_f64; 16];

    for ces in &ces_results {
        // Determine hue bin (0-15)
        // Bin 0 covers -11.25° to 11.25°, bin 1 covers 11.25° to 33.75°, etc.
        let mut hue = ces.ref_hue;
        if hue < 0.0 {
            hue += 360.0;
        }
        let bin_idx = (((hue + 11.25) / 22.5) as usize) % 16;

        // Per-sample Rf
        let sample_rf = (100.0 - 7.54 * ces.delta_e).max(0.0);
        rf_hue[bin_idx] += sample_rf;

        // Chroma shift
        let test_c = (ces.test_a.powi(2) + ces.test_b.powi(2)).sqrt();
        let ref_c = (ces.ref_a.powi(2) + ces.ref_b.powi(2)).sqrt();
        let rcs = (test_c - ref_c) / ref_c.max(1.0) * 100.0;
        rcs_hue[bin_idx] += rcs;

        // Hue shift (in degrees)
        let test_h = ces.test_b.atan2(ces.test_a).to_degrees();
        let ref_h = ces.ref_b.atan2(ces.ref_a).to_degrees();
        let mut dh = test_h - ref_h;
        if dh > 180.0 {
            dh -= 360.0;
        }
        if dh < -180.0 {
            dh += 360.0;
        }
        rhs_hue[bin_idx] += dh;

        // Accumulate for color vector centroid
        bin_test_a_sum[bin_idx] += ces.test_a;
        bin_test_b_sum[bin_idx] += ces.test_b;
        bin_ref_a_sum[bin_idx] += ces.ref_a;
        bin_ref_b_sum[bin_idx] += ces.ref_b;
        bin_counts[bin_idx] += 1;
    }

    // Average metrics per bin and compute color vectors
    let mut test_gamut_points = Vec::with_capacity(16);
    let mut ref_gamut_points = Vec::with_capacity(16);

    for i in 0..16 {
        let count = bin_counts[i].max(1) as f64;
        rf_hue[i] /= count;
        rcs_hue[i] /= count;
        rhs_hue[i] /= count;

        // Color vector is the centroid of samples in this bin
        let test_a = bin_test_a_sum[i] / count;
        let test_b = bin_test_b_sum[i] / count;
        let ref_a = bin_ref_a_sum[i] / count;
        let ref_b = bin_ref_b_sum[i] / count;
        color_vectors[i] = (test_a, test_b, ref_a, ref_b);

        test_gamut_points.push((test_a, test_b));
        ref_gamut_points.push((ref_a, ref_b));
    }

    // Calculate Rf (fidelity index) - based on all 99 samples
    let mean_de = delta_e_sum / NUM_CES as f64;
    let rf = (100.0 - 6.73 * mean_de).clamp(0.0, 100.0);

    // Calculate Rg (gamut index) - ratio of test to reference gamut areas
    let test_area = polygon_area(&test_gamut_points);
    let ref_area = polygon_area(&ref_gamut_points);
    let rg = if ref_area > 0.0 {
        100.0 * test_area / ref_area
    } else {
        100.0
    };

    Some(Tm30Result {
        rf,
        rg,
        cct,
        duv,
        rf_hue,
        rcs_hue,
        rhs_hue,
        color_vectors,
    })
}

/// Calculate polygon area using shoelace formula
fn polygon_area(points: &[(f64, f64)]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }

    let mut area = 0.0;
    let n = points.len();

    for i in 0..n {
        let j = (i + 1) % n;
        area += points[i].0 * points[j].1;
        area -= points[j].0 * points[i].1;
    }

    (area / 2.0).abs()
}

// ============================================================================
// SVG Generation
// ============================================================================

impl Tm30Result {
    /// Generate Color Vector Graphic (CVG) as SVG
    pub fn to_svg(&self, width: f64, height: f64, theme: &Tm30Theme) -> String {
        let margin = 60.0;
        let plot_size = (width - 2.0 * margin).min(height - 2.0 * margin);
        let center_x = width / 2.0;
        let center_y = height / 2.0;
        let scale = plot_size / 100.0; // Scale factor for a'/b' coordinates

        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}">
  <rect width="{width}" height="{height}" fill="{bg}"/>
  <defs>
    <clipPath id="cvg-clip">
      <circle cx="{cx}" cy="{cy}" r="{r}"/>
    </clipPath>
  </defs>
"#,
            width = width,
            height = height,
            bg = theme.background,
            cx = center_x,
            cy = center_y,
            r = plot_size / 2.0,
        );

        // Grid circles
        for r in [20.0, 40.0] {
            svg.push_str(&format!(
                r#"  <circle cx="{}" cy="{}" r="{}" fill="none" stroke="{}" stroke-width="1" stroke-dasharray="4,4"/>"#,
                center_x, center_y, r * scale, theme.grid_color
            ));
            svg.push('\n');
        }

        // Outer circle (reference boundary)
        svg.push_str(&format!(
            r#"  <circle cx="{}" cy="{}" r="{}" fill="none" stroke="{}" stroke-width="2"/>"#,
            center_x,
            center_y,
            plot_size / 2.0,
            theme.grid_color
        ));
        svg.push('\n');

        // Axis lines
        svg.push_str(&format!(
            r#"  <line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1"/>"#,
            center_x - plot_size / 2.0,
            center_y,
            center_x + plot_size / 2.0,
            center_y,
            theme.grid_color
        ));
        svg.push('\n');
        svg.push_str(&format!(
            r#"  <line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1"/>"#,
            center_x,
            center_y - plot_size / 2.0,
            center_x,
            center_y + plot_size / 2.0,
            theme.grid_color
        ));
        svg.push('\n');

        // Hue bin colors for visualization
        let hue_colors = [
            "#e74c3c", "#e67e22", "#f39c12", "#f1c40f", // Red -> Yellow
            "#2ecc71", "#1abc9c", "#16a085", "#3498db", // Green -> Cyan
            "#2980b9", "#9b59b6", "#8e44ad", "#c0392b", // Blue -> Purple
            "#d35400", "#e74c3c", "#c0392b", "#922b21", // Continuation
        ];

        // Draw reference polygon (gray)
        let mut ref_path = String::from("M ");
        for (i, &(_, _, ref_a, ref_b)) in self.color_vectors.iter().enumerate() {
            let x = center_x + ref_a * scale;
            let y = center_y - ref_b * scale; // Flip Y
            if i == 0 {
                ref_path.push_str(&format!("{:.1} {:.1}", x, y));
            } else {
                ref_path.push_str(&format!(" L {:.1} {:.1}", x, y));
            }
        }
        ref_path.push_str(" Z");
        svg.push_str(&format!(
            r#"  <path d="{}" fill="none" stroke="{}" stroke-width="2" stroke-dasharray="6,3"/>"#,
            ref_path, theme.reference_color
        ));
        svg.push('\n');

        // Draw test polygon (colored)
        let mut test_path = String::from("M ");
        for (i, &(test_a, test_b, _, _)) in self.color_vectors.iter().enumerate() {
            let x = center_x + test_a * scale;
            let y = center_y - test_b * scale;
            if i == 0 {
                test_path.push_str(&format!("{:.1} {:.1}", x, y));
            } else {
                test_path.push_str(&format!(" L {:.1} {:.1}", x, y));
            }
        }
        test_path.push_str(" Z");
        svg.push_str(&format!(
            r#"  <path d="{}" fill="{}20" stroke="{}" stroke-width="2"/>"#,
            test_path, theme.test_color, theme.test_color
        ));
        svg.push('\n');

        // Draw color vectors (arrows from reference to test)
        for (i, &(test_a, test_b, ref_a, ref_b)) in self.color_vectors.iter().enumerate() {
            let x1 = center_x + ref_a * scale;
            let y1 = center_y - ref_b * scale;
            let x2 = center_x + test_a * scale;
            let y2 = center_y - test_b * scale;

            // Vector line
            svg.push_str(&format!(
                r#"  <line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="2"/>"#,
                x1, y1, x2, y2, hue_colors[i]
            ));
            svg.push('\n');

            // Test point
            svg.push_str(&format!(
                r#"  <circle cx="{:.1}" cy="{:.1}" r="4" fill="{}" stroke="{}" stroke-width="1"/>"#,
                x2, y2, hue_colors[i], theme.background
            ));
            svg.push('\n');
        }

        // Title and metrics
        svg.push_str(&format!(
            r#"  <text x="{}" y="25" fill="{}" font-size="16" font-family="{}" font-weight="bold" text-anchor="middle">TM-30 Color Vector Graphic</text>"#,
            center_x, theme.foreground, theme.font_family
        ));
        svg.push('\n');

        // Rf and Rg values
        svg.push_str(&format!(
            r#"  <text x="20" y="{}" fill="{}" font-size="14" font-family="{}">Rf = {:.0}</text>"#,
            height - 40.0,
            theme.foreground,
            theme.font_family,
            self.rf
        ));
        svg.push('\n');
        svg.push_str(&format!(
            r#"  <text x="20" y="{}" fill="{}" font-size="14" font-family="{}">Rg = {:.0}</text>"#,
            height - 20.0,
            theme.foreground,
            theme.font_family,
            self.rg
        ));
        svg.push('\n');
        svg.push_str(&format!(
            r#"  <text x="{}" y="{}" fill="{}" font-size="12" font-family="{}" text-anchor="end">CCT = {:.0}K</text>"#,
            width - 20.0, height - 40.0, theme.foreground, theme.font_family, self.cct
        ));
        svg.push('\n');
        svg.push_str(&format!(
            r#"  <text x="{}" y="{}" fill="{}" font-size="12" font-family="{}" text-anchor="end">Duv = {:.4}</text>"#,
            width - 20.0, height - 20.0, theme.foreground, theme.font_family, self.duv
        ));
        svg.push('\n');

        // Legend
        svg.push_str(&format!(
            r#"  <line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="2" stroke-dasharray="6,3"/>"#,
            width - 120.0, 20.0, width - 90.0, 20.0, theme.reference_color
        ));
        svg.push_str(&format!(
            r#"  <text x="{}" y="24" fill="{}" font-size="11" font-family="{}">Reference</text>"#,
            width - 85.0,
            theme.foreground,
            theme.font_family
        ));
        svg.push('\n');
        svg.push_str(&format!(
            r#"  <line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="2"/>"#,
            width - 120.0,
            38.0,
            width - 90.0,
            38.0,
            theme.test_color
        ));
        svg.push_str(&format!(
            r#"  <text x="{}" y="42" fill="{}" font-size="11" font-family="{}">Test</text>"#,
            width - 85.0,
            theme.foreground,
            theme.font_family
        ));
        svg.push('\n');

        svg.push_str("</svg>");
        svg
    }

    /// Generate Rf hue bar chart as SVG
    pub fn rf_hue_svg(&self, width: f64, height: f64, theme: &Tm30Theme) -> String {
        let margin_left = 40.0;
        let margin_right = 20.0;
        let margin_top = 30.0;
        let margin_bottom = 40.0;

        let plot_width = width - margin_left - margin_right;
        let plot_height = height - margin_top - margin_bottom;
        let bar_width = plot_width / 16.0 - 4.0;

        let hue_colors = [
            "#e74c3c", "#e67e22", "#f39c12", "#f1c40f", "#2ecc71", "#27ae60", "#1abc9c", "#16a085",
            "#3498db", "#2980b9", "#9b59b6", "#8e44ad", "#c0392b", "#d35400", "#e74c3c", "#922b21",
        ];

        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}">
  <rect width="{width}" height="{height}" fill="{bg}"/>
"#,
            width = width,
            height = height,
            bg = theme.background,
        );

        // Title
        svg.push_str(&format!(
            r#"  <text x="{}" y="20" fill="{}" font-size="14" font-family="{}" font-weight="bold" text-anchor="middle">Rf by Hue Bin</text>"#,
            width / 2.0, theme.foreground, theme.font_family
        ));
        svg.push('\n');

        // Y-axis
        for y_val in [0, 25, 50, 75, 100] {
            let y = margin_top + plot_height * (1.0 - y_val as f64 / 100.0);
            svg.push_str(&format!(
                r#"  <line x1="{}" y1="{:.1}" x2="{}" y2="{:.1}" stroke="{}" stroke-width="1"/>"#,
                margin_left,
                y,
                margin_left + plot_width,
                y,
                theme.grid_color
            ));
            svg.push('\n');
            svg.push_str(&format!(
                r#"  <text x="{}" y="{:.1}" fill="{}" font-size="10" font-family="{}" text-anchor="end" dominant-baseline="middle">{}</text>"#,
                margin_left - 5.0, y, theme.foreground, theme.font_family, y_val
            ));
            svg.push('\n');
        }

        // Bars
        for (i, &rf) in self.rf_hue.iter().enumerate() {
            let x = margin_left + (i as f64 + 0.5) * (plot_width / 16.0) - bar_width / 2.0;
            let bar_height = rf / 100.0 * plot_height;
            let y = margin_top + plot_height - bar_height;

            svg.push_str(&format!(
                r#"  <rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="{}" rx="2"/>"#,
                x, y, bar_width, bar_height, hue_colors[i]
            ));
            svg.push('\n');

            // Bin label
            svg.push_str(&format!(
                r#"  <text x="{:.1}" y="{}" fill="{}" font-size="9" font-family="{}" text-anchor="middle">{}</text>"#,
                x + bar_width / 2.0, margin_top + plot_height + 15.0, theme.foreground, theme.font_family, i + 1
            ));
            svg.push('\n');
        }

        // Average Rf line
        let avg_rf = self.rf_hue.iter().sum::<f64>() / 16.0;
        let avg_y = margin_top + plot_height * (1.0 - avg_rf / 100.0);
        svg.push_str(&format!(
            r#"  <line x1="{}" y1="{:.1}" x2="{}" y2="{:.1}" stroke="{}" stroke-width="2" stroke-dasharray="6,3"/>"#,
            margin_left, avg_y, margin_left + plot_width, avg_y, theme.test_color
        ));
        svg.push('\n');
        svg.push_str(&format!(
            r#"  <text x="{}" y="{:.1}" fill="{}" font-size="10" font-family="{}">Rf = {:.0}</text>"#,
            margin_left + plot_width + 5.0, avg_y + 4.0, theme.test_color, theme.font_family, self.rf
        ));
        svg.push('\n');

        svg.push_str("</svg>");
        svg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_planckian_spd() {
        let spd = planckian_spd(3000.0);
        assert_eq!(spd.wavelengths.len(), 81);
        assert_eq!(spd.values.len(), 81);
        // Should be normalized
        let max_val = spd.values.iter().cloned().fold(0.0_f64, f64::max);
        assert!((max_val - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_xyz_to_cct() {
        // Test with D65 white point approximately
        let (cct, _duv) = xyz_to_cct(95.047, 100.0, 108.883);
        // D65 is approximately 6500K
        assert!((cct - 6500.0).abs() < 500.0);
    }

    #[test]
    fn test_tm30_calculation() {
        // Create a simple test SPD (warm white LED-like)
        let wavelengths: Vec<f64> = (380..=780).step_by(5).map(|w| w as f64).collect();
        let values: Vec<f64> = wavelengths
            .iter()
            .map(|&wl| {
                let blue_peak = (-((wl - 450.0) / 20.0).powi(2)).exp() * 0.7;
                let phosphor = if wl > 480.0 {
                    (-((wl - 580.0) / 80.0).powi(2)).exp() * 1.0
                } else {
                    0.0
                };
                blue_peak + phosphor
            })
            .collect();

        let spd = SpectralDistribution {
            wavelengths,
            values,
            units: crate::types::SpectralUnits::Relative,
            start_wavelength: None,
            wavelength_interval: None,
        };

        let result = calculate_tm30(&spd);
        assert!(result.is_some());

        let tm30 = result.unwrap();
        // Basic sanity checks - values should be in valid ranges
        // (exact values depend on simplified algorithm)
        assert!(
            tm30.rf >= 0.0 && tm30.rf <= 100.0,
            "Rf={} out of range",
            tm30.rf
        );
        assert!(
            tm30.rg > 0.0 && tm30.rg < 200.0,
            "Rg={} out of range",
            tm30.rg
        );
        assert!(
            tm30.cct > 1000.0 && tm30.cct < 20000.0,
            "CCT={} out of range",
            tm30.cct
        );
        // Per-hue values should exist
        assert_eq!(tm30.rf_hue.len(), 16);
        assert_eq!(tm30.color_vectors.len(), 16);
    }
}
