//! Spectral weighting functions on a 5 nm grid, 380–780 nm.
//!
//! Every function is a *response per unit spectral power at wavelength λ*. To
//! get a photometric/actinic quantity you integrate `SPD(λ)·f(λ) dλ` and apply
//! the relevant efficacy constant (see [`crate::constants`]).
//!
//! Tables:
//! - CIE 1931 2° colour-matching functions x̄, ȳ, z̄ (ȳ = V, the photopic
//!   luminous efficiency function).
//! - CIE 1951 scotopic luminous efficiency V′(λ).
//! - CIE S 026:2018 melanopic action spectrum s_mel(λ) (melanopsin).
//! - PAR quantum weight (flat 1 over 400–700 nm, 0 elsewhere) used with the
//!   photon-energy factor to convert radiant power to µmol photons.

#[allow(unused_imports)]
use crate::prelude::*;

/// First wavelength of every LUT (nm).
pub const LUT_START_NM: f64 = 380.0;
/// Grid step of every LUT (nm).
pub const LUT_STEP_NM: f64 = 5.0;
/// Number of samples in every LUT.
pub const LUT_LEN: usize = 81;

/// Linear interpolation into a 5 nm / 81-entry LUT. Zero outside 380–780 nm.
fn lerp_lut(table: &[f64; LUT_LEN], wl_nm: f64) -> f64 {
    let last = LUT_START_NM + (LUT_LEN - 1) as f64 * LUT_STEP_NM;
    if wl_nm <= LUT_START_NM {
        return if wl_nm < LUT_START_NM { 0.0 } else { table[0] };
    }
    if wl_nm >= last {
        return if wl_nm > last {
            0.0
        } else {
            table[LUT_LEN - 1]
        };
    }
    let pos = (wl_nm - LUT_START_NM) / LUT_STEP_NM;
    let i0 = pos.floor() as usize;
    let i1 = (i0 + 1).min(LUT_LEN - 1);
    let t = pos - i0 as f64;
    table[i0] * (1.0 - t) + table[i1] * t
}

/// CIE 1931 x̄(λ).
pub fn cie_x(wl_nm: f64) -> f64 {
    lerp_lut(&CMF_X, wl_nm)
}
/// CIE 1931 ȳ(λ) = photopic V(λ).
pub fn cie_y(wl_nm: f64) -> f64 {
    lerp_lut(&CMF_Y, wl_nm)
}
/// CIE 1931 z̄(λ).
pub fn cie_z(wl_nm: f64) -> f64 {
    lerp_lut(&CMF_Z, wl_nm)
}
/// Photopic luminous efficiency V(λ) (peak 1.0 at 555 nm).
pub fn photopic(wl_nm: f64) -> f64 {
    lerp_lut(&CMF_Y, wl_nm)
}
/// Scotopic luminous efficiency V′(λ) (peak 1.0 at 507 nm).
pub fn scotopic(wl_nm: f64) -> f64 {
    lerp_lut(&V_SCOTOPIC, wl_nm)
}
/// CIE S 026 melanopic action spectrum s_mel(λ) (peak 1.0 near 490 nm).
pub fn melanopic(wl_nm: f64) -> f64 {
    lerp_lut(&S_MEL, wl_nm)
}
/// PAR (photosynthetically active radiation) quantum weight: 1 over
/// 400–700 nm, 0 elsewhere. Multiply by SPD and the per-wavelength photon
/// factor to obtain PPFD.
pub fn par_weight(wl_nm: f64) -> f64 {
    if (400.0..=700.0).contains(&wl_nm) {
        1.0
    } else {
        0.0
    }
}

/// CIE 1931 2° CMF x̄, 380–780 nm @ 5 nm. Mirrors the table already used by
/// `eulumdat::atla::colorimetry`, kept here so this crate is dependency-free.
pub const CMF_X: [f64; LUT_LEN] = [
    0.001368, 0.002236, 0.004243, 0.007650, 0.014310, 0.023190, 0.043510, 0.077630, 0.134380,
    0.214770, 0.283900, 0.328500, 0.348280, 0.348060, 0.336200, 0.318700, 0.290800, 0.251100,
    0.195360, 0.142100, 0.095640, 0.057950, 0.032010, 0.014700, 0.004900, 0.002400, 0.009300,
    0.029100, 0.063270, 0.109600, 0.165500, 0.225750, 0.290400, 0.359700, 0.433450, 0.512050,
    0.594500, 0.678400, 0.762100, 0.842500, 0.916300, 0.978600, 1.026300, 1.056700, 1.062200,
    1.045600, 1.002600, 0.938400, 0.854450, 0.751400, 0.642400, 0.541900, 0.447900, 0.360800,
    0.283500, 0.218700, 0.164900, 0.121200, 0.087400, 0.063600, 0.046770, 0.032900, 0.022700,
    0.015840, 0.011359, 0.008111, 0.005790, 0.004109, 0.002899, 0.002049, 0.001440, 0.001000,
    0.000690, 0.000476, 0.000332, 0.000235, 0.000166, 0.000117, 0.000083, 0.000059, 0.000042,
];

/// CIE 1931 2° CMF ȳ = V(λ).
pub const CMF_Y: [f64; LUT_LEN] = [
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

/// CIE 1931 2° CMF z̄.
pub const CMF_Z: [f64; LUT_LEN] = [
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

/// CIE 1951 scotopic luminous efficiency V′(λ), 380–780 nm @ 5 nm, peak 1.0 at
/// 507 nm. CIE S 026:2018 defines the rhodopic action spectrum as V′(λ); the
/// values are every fifth one of the rhodopic column of the official 1 nm data
/// table "CIE alpha-opic action spectra" (DOI 10.25039/CIE.DS.vqqhzp5a, CC
/// BY-SA 4.0, International Commission on Illumination), copied unchanged and
/// pinned by `tests::scotopic_matches_cie`. (Until 2026-11-21 this table was an
/// approximation that peaked at 500 nm and read 0.33 instead of 0.48 at 550 nm.)
pub const V_SCOTOPIC: [f64; LUT_LEN] = [
    5.890000e-04,
    0.001110,
    0.002210,
    0.004530,
    0.009290,
    0.018500,
    0.034800,
    0.060400,
    0.096600,
    0.144000,
    0.200000,
    0.263000,
    0.328000,
    0.393000,
    0.455000,
    0.513000,
    0.567000,
    0.620000,
    0.676000,
    0.734000,
    0.793000,
    0.851000,
    0.904000,
    0.949000,
    0.982000,
    0.998000,
    0.997000,
    0.975000,
    0.935000,
    0.880000,
    0.811000,
    0.733000,
    0.650000,
    0.564000,
    0.481000,
    0.402000,
    0.329000,
    0.264000,
    0.208000,
    0.160000,
    0.121000,
    0.089900,
    0.065500,
    0.046900,
    0.033200,
    0.023100,
    0.015900,
    0.010900,
    0.007370,
    0.004970,
    0.003340,
    0.002240,
    0.001500,
    0.001010,
    6.770000e-04,
    4.590000e-04,
    3.130000e-04,
    2.150000e-04,
    1.480000e-04,
    1.030000e-04,
    7.150000e-05,
    5.010000e-05,
    3.530000e-05,
    2.500000e-05,
    1.780000e-05,
    1.270000e-05,
    9.140000e-06,
    6.600000e-06,
    4.780000e-06,
    3.480000e-06,
    2.550000e-06,
    1.870000e-06,
    1.380000e-06,
    1.020000e-06,
    7.600000e-07,
    5.670000e-07,
    4.250000e-07,
    3.200000e-07,
    2.410000e-07,
    1.830000e-07,
    1.390000e-07,
];

/// CIE S 026:2018 melanopic action spectrum s_mel(λ), 380–780 nm @ 5 nm, peak
/// 1.0 at 490 nm: every fifth value of the official 1 nm data table "CIE
/// alpha-opic action spectra" (CIE S 026:2018 Table 2, DOI
/// 10.25039/CIE.DS.vqqhzp5a, CC BY-SA 4.0, International Commission on
/// Illumination). Values copied unchanged; `tests::melanopic_matches_cie_s026`
/// pins them. (Until 2026-11-21 this table was an approximation that peaked at
/// 480 nm and read 0.80 instead of 0.55 at 450 nm: melanopic values of white
/// LEDs came out too high.)
pub const S_MEL: [f64; LUT_LEN] = [
    9.181650e-04,
    0.001667,
    0.003094,
    0.005880,
    0.011428,
    0.022811,
    0.046155,
    0.079477,
    0.137237,
    0.187096,
    0.253865,
    0.320679,
    0.401587,
    0.474002,
    0.553715,
    0.629654,
    0.708049,
    0.785216,
    0.860291,
    0.917734,
    0.965605,
    0.990621,
    1.000000,
    0.992022,
    0.965952,
    0.922299,
    0.862888,
    0.785233,
    0.699628,
    0.609422,
    0.519309,
    0.432533,
    0.351707,
    0.279135,
    0.215722,
    0.162056,
    0.118526,
    0.084346,
    0.058701,
    0.040009,
    0.026875,
    0.017862,
    0.011790,
    0.007734,
    0.005067,
    0.003318,
    0.002177,
    0.001433,
    9.473130e-04,
    6.276480e-04,
    4.179550e-04,
    2.798010e-04,
    1.883410e-04,
    1.273370e-04,
    8.657510e-05,
    5.919140e-05,
    4.069450e-05,
    2.813200e-05,
    1.955350e-05,
    1.364800e-05,
    9.576370e-06,
    6.754250e-06,
    4.788040e-06,
    3.408410e-06,
    2.438190e-06,
    1.752520e-06,
    1.265600e-06,
    9.180780e-07,
    6.689910e-07,
    4.895310e-07,
    3.597660e-07,
    2.654930e-07,
    1.967400e-07,
    1.463700e-07,
    1.093320e-07,
    8.195870e-08,
    6.167490e-08,
    4.659160e-08,
    3.532720e-08,
    2.688030e-08,
    2.052580e-08,
];

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn photopic_peaks_at_555() {
        assert_relative_eq!(photopic(555.0), 1.0, epsilon = 1e-9);
        assert!(photopic(555.0) >= photopic(500.0));
        assert!(photopic(555.0) >= photopic(600.0));
    }

    #[test]
    fn scotopic_peaks_in_blue_green() {
        // V′ peaks around 500–507 nm (well below photopic's 555 nm) and is
        // near its maximum across that whole band.
        assert!(scotopic(505.0) > 0.99); // V′ peaks at 507 nm (0.982 at 500 nm)
        assert!(scotopic(507.0) > 0.97);
        assert!(scotopic(500.0) > scotopic(555.0));
        assert!(scotopic(500.0) > scotopic(450.0));
    }

    /// Spot values of the official CIE S 026 table (1 nm data, DOI
    /// 10.25039/CIE.DS.vqqhzp5a) at wavelengths on our 5 nm grid.
    #[test]
    fn melanopic_matches_cie_s026() {
        for (wl, want) in [
            (380.0, 9.18165e-4),
            (450.0, 0.553715),
            (480.0, 0.965605),
            (490.0, 1.0),
            (500.0, 0.965952),
            (550.0, 0.215722),
            (600.0, 0.00506686),
        ] {
            assert!(
                (melanopic(wl) - want).abs() < 5e-7,
                "{wl} nm: {} vs {want}",
                melanopic(wl)
            );
        }
        // peak 1.0 at 490 nm, and the area under the curve (CIE gives 87.675 for the 1 nm table)
        let peak = S_MEL.iter().cloned().fold(0.0, f64::max);
        assert_eq!(peak, melanopic(490.0));
        let area: f64 = S_MEL.iter().sum::<f64>() * LUT_STEP_NM;
        assert!((area - 87.675).abs() < 0.05, "area {area}");
    }

    /// V′(λ) = the rhodopic column of the same CIE table.
    #[test]
    fn scotopic_matches_cie() {
        for (wl, want) in [
            (450.0, 0.455),
            (505.0, 0.998),
            (510.0, 0.997),
            (550.0, 0.481),
            (600.0, 0.0332),
        ] {
            assert!(
                (scotopic(wl) - want).abs() < 5e-7,
                "{wl} nm: {} vs {want}",
                scotopic(wl)
            );
        }
        let area: f64 = V_SCOTOPIC.iter().sum::<f64>() * LUT_STEP_NM;
        assert!((area - 97.073).abs() < 0.05, "area {area}");
    }

    #[test]
    fn melanopic_is_blue_shifted_vs_photopic() {
        // Melanopic peaks well below photopic (490 vs 555 nm).
        assert!(melanopic(490.0) > melanopic(555.0));
        assert!(photopic(555.0) > photopic(490.0));
    }

    #[test]
    fn par_is_a_box() {
        assert_eq!(par_weight(399.0), 0.0);
        assert_eq!(par_weight(400.0), 1.0);
        assert_eq!(par_weight(550.0), 1.0);
        assert_eq!(par_weight(700.0), 1.0);
        assert_eq!(par_weight(701.0), 0.0);
    }

    #[test]
    fn out_of_band_is_zero() {
        assert_eq!(cie_x(300.0), 0.0);
        assert_eq!(cie_y(900.0), 0.0);
        assert_eq!(scotopic(1000.0), 0.0);
    }
}
