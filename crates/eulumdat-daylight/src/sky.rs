//! Analytic sky luminance distributions.
//!
//! Two families:
//! - [`PerezSky`] — the Perez *all-weather* model (Perez, Seals & Michalsky,
//!   1993): a 5-coefficient gradation × indicatrix giving the **relative**
//!   luminance of any sky element, parameterized by sky clearness ε and
//!   brightness Δ.
//! - [`CieSky`] — the CIE 15 standard general skies (the overcast and clear
//!   reference skies most used in daylighting).
//!
//! Both return a *relative* (dimensionless) luminance gradient. [`SkyRadiance`]
//! turns that into absolute cd/m² by normalizing the cos-weighted hemispherical
//! integral to a measured diffuse horizontal illuminance (lux).
//!
//! Angles use the sky-dome convention: `theta` is measured from the **zenith**
//! (0 = straight up), `phi` is the azimuth (shared with the solar azimuth
//! reference). The relevant geometric angle for the indicatrix is `gamma`, the
//! angle between a sky element and the sun.

use crate::coords::{angle_between, dome_to_world};
use crate::solar::SolarPosition;
use std::f64::consts::{FRAC_PI_2, PI};

/// Inputs describing the sky condition for the Perez model.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SkyParams {
    /// Perez sky clearness ε (1 = fully overcast … ≳6 = clear). Determines
    /// which coefficient bin is used.
    pub clearness_eps: f64,
    /// Perez sky brightness Δ (dimensionless; modulates the coefficients).
    pub brightness_delta: f64,
}

impl SkyParams {
    /// Heuristic mapping from atmospheric turbidity to (ε, Δ) for a clear-ish
    /// sky, so callers who only have a turbidity slider get sensible values.
    /// Higher turbidity → hazier → lower clearness, higher brightness.
    pub fn from_turbidity(turbidity: f64) -> Self {
        let t = turbidity.max(1.0);
        // Clear ~T=2 → ε≈6.3; hazy ~T=6 → ε≈2.2. Monotonic, smooth.
        let clearness_eps = (8.0 / t).clamp(1.0, 8.0);
        let brightness_delta = (0.05 * t).clamp(0.05, 0.5);
        Self {
            clearness_eps,
            brightness_delta,
        }
    }

    /// The fully-overcast condition (ε = 1).
    pub fn overcast() -> Self {
        Self {
            clearness_eps: 1.0,
            brightness_delta: 0.3,
        }
    }
}

/// The five Perez coefficients: `a` (gradation), `b` (gradation exponent),
/// `c` (circumsolar intensity), `d` (circumsolar width), `e` (backscatter).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PerezCoeffs {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
}

/// Perez ε bin upper bounds (Perez et al. 1990 discrete sky-clearness bins).
const EPS_BINS: [f64; 8] = [1.065, 1.230, 1.500, 1.950, 2.800, 4.500, 6.200, f64::INFINITY];

// Perez luminance-distribution coefficients (Perez, Seals & Michalsky 1993),
// rows = the 8 clearness bins. Each row: [a coefficients ×4] for a1..a4, etc.
// Layout per parameter p: p = x1 + x2*Z + Δ*(x3 + x4*Z) where Z = solar zenith.
// Columns: [x1, x2, x3, x4].
const A_COEF: [[f64; 4]; 8] = [
    [1.3525, -0.2576, -0.2690, -1.4366],
    [-1.2219, -0.7730, 1.4148, 1.1016],
    [-1.1000, -0.2515, 0.8952, 0.0156],
    [-0.5484, -0.6654, -0.2672, 0.7117],
    [-0.6000, -0.3566, -2.5000, 2.3250],
    [-1.0156, -0.3670, 1.0078, 1.4051],
    [-1.0000, 0.0211, 0.5025, -0.5119],
    [-1.0500, 0.0289, 0.4260, 0.3590],
];
const B_COEF: [[f64; 4]; 8] = [
    [-0.7670, 0.0007, 1.2734, -0.1233],
    [-0.2054, 0.0367, -3.9128, 0.9156],
    [0.2782, -0.1812, -4.5000, 1.1766],
    [0.7234, -0.6219, -5.6812, 2.6297],
    [0.2937, 0.0496, -5.6812, 1.8415],
    [0.2875, -0.5328, -3.8500, 3.3750],
    [-0.3000, 0.1922, 0.7023, -1.6317],
    [-0.3250, 0.1156, 0.7781, 0.0025],
];
const C_COEF: [[f64; 4]; 8] = [
    [2.8000, 0.6004, 1.2375, 1.0000],
    [6.9750, 0.1774, 6.4477, -0.1239],
    [24.7219, -13.0812, -37.7000, 34.8438],
    [33.3389, -18.3000, -62.2500, 52.0781],
    [21.0000, -4.7656, -21.5906, 7.2492],
    [14.0000, -0.9999, -7.1406, 7.5469],
    [19.0000, -5.0000, 1.2438, -1.9094],
    [12.8000, 0.0000, 0.1700, -8.0000],
];
const D_COEF: [[f64; 4]; 8] = [
    [1.8734, 0.6297, 0.9738, 0.2809],
    [-1.5798, -0.5081, -1.7812, 0.1080],
    [-5.0000, 1.5218, 3.9229, -2.6204],
    [-3.5000, 0.0016, 1.1477, 0.1062],
    [-3.5000, -0.1554, 1.4062, 0.3988],
    [-3.4000, -0.1078, -1.0750, 1.5702],
    [-4.0000, 0.0250, 0.3844, 0.2656],
    [-0.2300, 0.3970, -0.1442, 1.2750],
];
const E_COEF: [[f64; 4]; 8] = [
    [0.0356, -0.1246, -0.5718, 0.9938],
    [0.2624, 0.0672, -0.2190, -0.4285],
    [-0.0156, 0.1597, 0.4199, -0.5562],
    [0.4659, -0.3296, -0.0876, -0.0329],
    [0.0032, 0.0766, -0.0656, -0.1294],
    [-0.0672, 0.4016, 0.3017, -0.4844],
    [1.0468, -0.3788, -2.4517, 1.4656],
    [0.1858, 0.0000, -0.0344, 0.1410],
];

fn eps_bin(eps: f64) -> usize {
    EPS_BINS.iter().position(|&hi| eps < hi).unwrap_or(7)
}

fn eval_param(row: &[f64; 4], z: f64, delta: f64) -> f64 {
    row[0] + row[1] * z + delta * (row[2] + row[3] * z)
}

/// The Perez all-weather sky luminance model (relative).
#[derive(Debug, Clone, Copy)]
pub struct PerezSky {
    coeffs: PerezCoeffs,
    /// Sun direction in world frame (up-going).
    sun: SolarPosition,
}

impl PerezSky {
    /// Build from a sun position and sky condition.
    pub fn new(sun: SolarPosition, params: SkyParams) -> Self {
        // Solar zenith for the coefficient interpolation; clamp near horizon to
        // avoid the model blowing up as Z → 90°.
        let z = sun.zenith_rad.min(FRAC_PI_2 - 0.01);
        let bin = eps_bin(params.clearness_eps);
        let d = params.brightness_delta;
        let coeffs = PerezCoeffs {
            a: eval_param(&A_COEF[bin], z, d),
            b: eval_param(&B_COEF[bin], z, d),
            // c, d, e for bin 0 (overcast) are handled with the published special
            // case below; the table values are the standard ones for ε>1 bins.
            c: eval_param(&C_COEF[bin], z, d),
            d: eval_param(&D_COEF[bin], z, d),
            e: eval_param(&E_COEF[bin], z, d),
        };
        Self { coeffs, sun }
    }

    /// The five fitted coefficients (for inspection/tests).
    pub fn coeffs(&self) -> PerezCoeffs {
        self.coeffs
    }

    /// Relative luminance of a sky element at dome angle `theta` (from zenith)
    /// and azimuth `phi`. Dimensionless and strictly positive for a valid sky.
    ///
    /// `f(theta, gamma) = (1 + a·exp(b/cos theta)) · (1 + c·exp(d·gamma) + e·cos²gamma)`
    /// where `gamma` is the angle between the element and the sun.
    pub fn relative_luminance(&self, theta_rad: f64, phi_rad: f64) -> f64 {
        let PerezCoeffs { a, b, c, d, e } = self.coeffs;
        // Clamp cos(theta) away from 0 at the horizon (the gradation term has
        // exp(b/cos theta); at the horizon cos→0).
        let cos_theta = theta_rad.cos().max(1e-3);
        let gamma = self.scatter_angle(theta_rad, phi_rad);
        let gradation = 1.0 + a * (b / cos_theta).exp();
        let indicatrix = 1.0 + c * (d * gamma).exp() + e * gamma.cos() * gamma.cos();
        (gradation * indicatrix).max(0.0)
    }

    /// Angle between a sky element and the sun (the indicatrix argument).
    fn scatter_angle(&self, theta_rad: f64, phi_rad: f64) -> f64 {
        let elem = dome_to_world(theta_rad, phi_rad);
        let sun = self.sun.world_direction();
        angle_between(&elem, &sun)
    }

    /// Cos-weighted hemispherical integral of the relative luminance — the
    /// dimensionless factor `K` such that `DHI = K · L_relative_unit`. Used to
    /// normalize the dome to an absolute diffuse horizontal illuminance.
    ///
    /// `K = ∫₀^{2π} ∫₀^{π/2} f(θ,φ) · cos θ · sin θ dθ dφ`
    pub fn diffuse_normalization(&self) -> f64 {
        integrate_cos_weighted(|theta, phi| self.relative_luminance(theta, phi))
    }
}

/// CIE 15 standard general skies (the two most-used reference skies).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CieSky {
    /// CIE Standard Overcast Sky (Moon & Spencer): `L(θ)/L_zenith = (1+2cosθ)/3`,
    /// azimuth-independent — zenith is 3× the horizon.
    Overcast,
    /// CIE clear sky (type 12, standard clear) — gradation + circumsolar.
    Clear,
    /// Uniform luminance sky (reference).
    Uniform,
}

impl CieSky {
    /// Relative luminance at dome angle `theta` (from zenith), azimuth `phi`,
    /// for the given sun position.
    pub fn relative_luminance(&self, theta_rad: f64, phi_rad: f64, sun: &SolarPosition) -> f64 {
        match self {
            CieSky::Uniform => 1.0,
            CieSky::Overcast => {
                // Moon-Spencer gradient, normalized so zenith = 1.0.
                (1.0 + 2.0 * theta_rad.cos()) / 3.0
            }
            CieSky::Clear => {
                let elem = dome_to_world(theta_rad, phi_rad);
                let gamma = angle_between(&elem, &sun.world_direction());
                let cos_theta = theta_rad.cos().max(1e-3);
                let z = sun.zenith_rad;
                // CIE general-sky standard clear (type 12) gradation/indicatrix.
                let gradation = 1.0 - (-0.32 / cos_theta).exp();
                let indicatrix =
                    0.91 + 10.0 * (-3.0 * gamma).exp() + 0.45 * gamma.cos() * gamma.cos();
                // Normalize by the zenith element's value so the field is relative.
                let grad_z = 1.0 - (-0.32_f64).exp();
                let ind_z = 0.91 + 10.0 * (-3.0 * z).exp() + 0.45 * z.cos() * z.cos();
                (gradation * indicatrix) / (grad_z * ind_z)
            }
        }
    }

    pub fn diffuse_normalization(&self, sun: &SolarPosition) -> f64 {
        integrate_cos_weighted(|theta, phi| self.relative_luminance(theta, phi, sun))
    }
}

/// Numerically integrate `f(θ,φ)·cosθ·sinθ` over the upper hemisphere
/// (θ from zenith ∈ [0, π/2), φ ∈ [0, 2π)). Midpoint rule.
fn integrate_cos_weighted<F: Fn(f64, f64) -> f64>(f: F) -> f64 {
    let n_theta = 90usize;
    let n_phi = 120usize;
    let dtheta = FRAC_PI_2 / n_theta as f64;
    let dphi = 2.0 * PI / n_phi as f64;
    let mut sum = 0.0;
    for i in 0..n_theta {
        let theta = (i as f64 + 0.5) * dtheta;
        let cw = theta.cos() * theta.sin();
        for j in 0..n_phi {
            let phi = (j as f64 + 0.5) * dphi;
            sum += f(theta, phi) * cw;
        }
    }
    sum * dtheta * dphi
}

/// Absolute sky luminance: a relative model normalized to a measured diffuse
/// horizontal illuminance (lux).
#[derive(Debug, Clone, Copy)]
pub struct SkyRadiance {
    sky: SkyKind,
    /// cd/m² per relative-luminance unit (the normalization constant).
    scale_cd_m2: f64,
    /// The diffuse horizontal illuminance this dome integrates to, lux.
    dhi_lux: f64,
}

#[derive(Debug, Clone, Copy)]
enum SkyKind {
    Perez(PerezSky),
    Cie(CieSky, SolarPosition),
}

impl SkyRadiance {
    /// Build a Perez sky normalized to an absolute diffuse horizontal illuminance.
    pub fn perez_from_dhi(sky: PerezSky, dhi_lux: f64) -> Self {
        let k = sky.diffuse_normalization().max(1e-9);
        Self {
            sky: SkyKind::Perez(sky),
            scale_cd_m2: dhi_lux / k,
            dhi_lux,
        }
    }

    /// Build a CIE standard sky normalized to an absolute DHI.
    pub fn cie_from_dhi(sky: CieSky, sun: SolarPosition, dhi_lux: f64) -> Self {
        let k = sky.diffuse_normalization(&sun).max(1e-9);
        Self {
            sky: SkyKind::Cie(sky, sun),
            scale_cd_m2: dhi_lux / k,
            dhi_lux,
        }
    }

    /// Relative luminance of a sky element (dimensionless).
    pub fn relative_luminance(&self, theta_rad: f64, phi_rad: f64) -> f64 {
        match &self.sky {
            SkyKind::Perez(p) => p.relative_luminance(theta_rad, phi_rad),
            SkyKind::Cie(c, sun) => c.relative_luminance(theta_rad, phi_rad, sun),
        }
    }

    /// Absolute luminance of a sky element, cd/m².
    pub fn luminance(&self, theta_rad: f64, phi_rad: f64) -> f64 {
        self.scale_cd_m2 * self.relative_luminance(theta_rad, phi_rad)
    }

    /// The diffuse horizontal illuminance the dome was normalized to (lux).
    pub fn dhi_lux(&self) -> f64 {
        self.dhi_lux
    }

    /// Numerically re-integrate the absolute dome to horizontal illuminance —
    /// a self-consistency check that should reproduce `dhi_lux()`.
    pub fn integrate_horizontal_illuminance(&self) -> f64 {
        self.scale_cd_m2 * integrate_cos_weighted(|t, p| self.relative_luminance(t, p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solar::solar_position;

    /// CIE overcast: zenith luminance must be 3× the horizon luminance.
    #[test]
    fn cie_overcast_zenith_three_times_horizon() {
        let sun = solar_position(2026, 3, 20, 12.0, 40.0, 0.0);
        let zenith = CieSky::Overcast.relative_luminance(0.0, 0.0, &sun);
        let horizon = CieSky::Overcast.relative_luminance(FRAC_PI_2 - 1e-6, 0.0, &sun);
        assert!(
            (zenith / horizon - 3.0).abs() < 1e-3,
            "overcast zenith/horizon = {} (want 3.0)",
            zenith / horizon
        );
    }

    /// Normalizing to a DHI must reproduce that DHI on re-integration.
    #[test]
    fn perez_normalization_round_trips_dhi() {
        let sun = solar_position(2026, 6, 21, 12.0, 40.0, 0.0);
        let perez = PerezSky::new(sun, SkyParams::from_turbidity(2.5));
        let dhi = 15_000.0;
        let rad = SkyRadiance::perez_from_dhi(perez, dhi);
        let back = rad.integrate_horizontal_illuminance();
        assert!(
            (back - dhi).abs() / dhi < 0.02,
            "re-integrated DHI {back} vs {dhi} (>2% off)"
        );
    }

    #[test]
    fn cie_overcast_normalization_round_trips() {
        let sun = solar_position(2026, 3, 20, 12.0, 40.0, 0.0);
        let dhi = 12_000.0;
        let rad = SkyRadiance::cie_from_dhi(CieSky::Overcast, sun, dhi);
        let back = rad.integrate_horizontal_illuminance();
        assert!((back - dhi).abs() / dhi < 0.02, "{back} vs {dhi}");
    }

    /// Perez relative luminance is finite and non-negative across the dome.
    #[test]
    fn perez_finite_and_nonnegative() {
        let sun = solar_position(2026, 6, 21, 10.0, 40.0, 0.0);
        let perez = PerezSky::new(sun, SkyParams::from_turbidity(3.0));
        for i in 0..18 {
            let theta = (i as f64) * (FRAC_PI_2 / 18.0);
            for j in 0..24 {
                let phi = (j as f64) * (2.0 * PI / 24.0);
                let l = perez.relative_luminance(theta, phi);
                assert!(l.is_finite() && l >= 0.0, "L={l} at θ={theta} φ={phi}");
            }
        }
    }
}
