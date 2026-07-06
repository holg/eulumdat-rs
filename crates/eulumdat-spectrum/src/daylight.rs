//! CIE D-series daylight illuminant reconstruction.
//!
//! Any daylight phase (D50, D65, or the per-direction CCT of a sky patch) is
//! reconstructed from three eigenvector spectra: `S(λ) = S₀ + M₁·S₁ + M₂·S₂`,
//! with M₁, M₂ derived from the target chromaticity on the CIE daylight locus.
//! This lets the sky dome carry a colour that varies with altitude/azimuth
//! without storing a spectrum per direction — just a CCT field.

use crate::spd::Spd;

/// Reconstruct a CIE D-series daylight SPD at a correlated colour temperature.
///
/// Valid for roughly 4000–25000 K (the CIE daylight locus range); outside it
/// the chromaticity is clamped to the endpoints. Returned on the 10 nm
/// eigenvector grid (300–830 nm), which the tracer resamples as needed.
pub fn d_illuminant(cct_k: f64) -> Spd {
    let t = cct_k.clamp(4000.0, 25000.0);
    // CIE daylight locus chromaticity x_D(T).
    let xd = if t <= 7000.0 {
        0.244063 + 0.09911e3 / t + 2.9678e6 / (t * t) - 4.6070e9 / (t * t * t)
    } else {
        0.237040 + 0.24748e3 / t + 1.9018e6 / (t * t) - 2.0064e9 / (t * t * t)
    };
    let yd = -3.0 * xd * xd + 2.870 * xd - 0.275;

    // Eigenvector mixing coefficients.
    let denom = 0.0241 + 0.2562 * xd - 0.7341 * yd;
    let m1 = (-1.3515 - 1.7703 * xd + 5.9114 * yd) / denom;
    let m2 = (0.0300 - 31.4424 * xd + 30.0717 * yd) / denom;

    let values: Vec<f64> = (0..S0.len())
        .map(|i| S0[i] + m1 * S1[i] + m2 * S2[i])
        .map(|v| v.max(0.0))
        .collect();
    Spd::uniform(D_START_NM, D_STEP_NM, &values)
}

const D_START_NM: f64 = 300.0;
const D_STEP_NM: f64 = 10.0;

// CIE daylight eigenvectors S0, S1, S2, 300–830 nm @ 10 nm (54 entries).
const S0: [f64; 54] = [
    0.04, 6.0, 29.6, 55.3, 57.3, 61.8, 61.5, 68.8, 63.4, 65.8, 94.8, 104.8, 105.9, 96.8, 113.9,
    125.6, 125.5, 121.3, 121.3, 113.5, 113.1, 110.8, 106.5, 108.8, 105.3, 104.4, 100.0, 96.0, 95.1,
    89.1, 90.5, 90.3, 88.4, 84.0, 85.1, 81.9, 82.6, 84.9, 81.3, 71.9, 74.3, 76.4, 63.3, 71.7, 77.0,
    65.2, 47.7, 68.6, 65.0, 66.0, 61.0, 53.3, 58.9, 61.9,
];
const S1: [f64; 54] = [
    0.02, 4.5, 22.4, 42.0, 40.6, 41.6, 38.0, 42.4, 38.5, 35.0, 43.4, 46.3, 43.9, 37.1, 36.7, 35.9,
    32.6, 27.9, 24.3, 20.1, 16.2, 13.2, 8.6, 6.1, 4.2, 1.9, 0.0, -1.6, -3.5, -3.5, -5.8, -7.2, -8.6,
    -9.5, -10.9, -10.7, -12.0, -14.0, -13.6, -12.0, -13.3, -12.9, -10.6, -11.6, -12.2, -10.2, -7.8,
    -11.2, -10.4, -10.6, -9.7, -8.3, -9.3, -9.8,
];
const S2: [f64; 54] = [
    0.0, 2.0, 4.0, 8.5, 7.8, 6.7, 5.3, 6.1, 2.0, 1.2, -1.1, -0.5, -0.7, -1.2, -2.6, -2.9, -2.8,
    -2.6, -2.6, -1.8, -1.5, -1.3, -1.2, -1.0, -0.5, -0.3, 0.0, 0.2, 0.5, 2.1, 3.2, 4.1, 4.7, 5.1,
    6.7, 7.3, 8.6, 9.8, 10.2, 8.3, 9.6, 8.5, 7.0, 7.6, 8.0, 6.7, 5.2, 7.4, 6.8, 7.0, 6.4, 5.5, 6.1,
    6.5,
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colorimetry::analyze;
    use approx::assert_relative_eq;

    #[test]
    fn d65_recovers_6500k() {
        let d65 = d_illuminant(6500.0);
        let c = analyze(&d65);
        assert!(
            (c.cct_k - 6500.0).abs() < 60.0,
            "D65 should be ~6500 K, got {}",
            c.cct_k
        );
        // D-series sits essentially on the daylight locus, small Duv.
        assert!(c.duv.abs() < 0.005);
    }

    #[test]
    fn d50_recovers_5000k() {
        let d50 = d_illuminant(5000.0);
        let c = analyze(&d50);
        assert!(
            (c.cct_k - 5000.0).abs() < 60.0,
            "D50 should be ~5000 K, got {}",
            c.cct_k
        );
    }

    #[test]
    fn d65_chromaticity_is_canonical() {
        // Canonical D65 chromaticity: x=0.31272, y=0.32903.
        let d65 = d_illuminant(6504.0);
        let c = analyze(&d65);
        assert_relative_eq!(c.x, 0.31272, epsilon = 0.003);
        assert_relative_eq!(c.y, 0.32903, epsilon = 0.003);
    }
}
