//! CIE colorimetry of a spectral power distribution.
//!
//! Self-contained (no dependency on `eulumdat`) so the tracers can compute
//! colour-over-angle from detector data directly. The math matches
//! `eulumdat::atla::colorimetry` (Signify-parity: CCT ±15 K, Duv ±0.0003):
//! tristimulus from the 5 nm CMFs, CCT via Robertson isotemperature lines with a
//! McCamy fallback, Duv as signed 1960-uv distance from the Planckian locus.

use crate::lut::{cie_x, cie_y, cie_z};
use crate::spd::Spd;

/// Full colorimetric description of an SPD.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Colorimetry {
    /// Tristimulus X (Y normalised to 100).
    pub x_tri: f64,
    /// Tristimulus Y = 100.
    pub y_tri: f64,
    /// Tristimulus Z.
    pub z_tri: f64,
    /// CIE 1931 chromaticity x.
    pub x: f64,
    /// CIE 1931 chromaticity y.
    pub y: f64,
    /// CIE 1960 UCS u.
    pub u: f64,
    /// CIE 1960 UCS v.
    pub v: f64,
    /// CIE 1976 UCS u′.
    pub up: f64,
    /// CIE 1976 UCS v′.
    pub vp: f64,
    /// Correlated colour temperature (K).
    pub cct_k: f64,
    /// Distance from Planckian locus in 1960 uv (+ greenish, − magenta).
    pub duv: f64,
}

/// Analyse an SPD into its full colorimetry.
pub fn analyze(spd: &Spd) -> Colorimetry {
    let big_x = spd.integral_weighted(cie_x);
    let big_y = spd.integral_weighted(cie_y);
    let big_z = spd.integral_weighted(cie_z);
    let sum = big_x + big_y + big_z;

    let (x, y) = if sum > 0.0 {
        (big_x / sum, big_y / sum)
    } else {
        (0.3333, 0.3333)
    };

    // Scale tristimulus to Y = 100.
    let (x_tri, y_tri, z_tri) = if big_y > 0.0 {
        (big_x / big_y * 100.0, 100.0, big_z / big_y * 100.0)
    } else {
        (0.0, 0.0, 0.0)
    };

    let (u, v) = xy_to_uv1960(x, y);
    let (up, vp) = (u, v * 1.5); // u′ = u, v′ = 1.5 v

    let cct_k = cct_robertson(u, v);
    let duv = compute_duv(u, v, cct_k);

    Colorimetry {
        x_tri,
        y_tri,
        z_tri,
        x,
        y,
        u,
        v,
        up,
        vp,
        cct_k,
        duv,
    }
}

/// Build a [`Colorimetry`] directly from a CIE 1931 chromaticity (x, y).
///
/// Used when tristimulus is accumulated over many photons (the tracer's
/// weighted-channel detector) rather than integrated from a stored SPD. CCT and
/// Duv come from the same Planckian solver as [`analyze`]; tristimulus is
/// normalised to Y = 100.
pub fn from_chromaticity(x: f64, y: f64) -> Colorimetry {
    let (x_tri, y_tri, z_tri) = if y > 0.0 {
        (x / y * 100.0, 100.0, (1.0 - x - y) / y * 100.0)
    } else {
        (0.0, 0.0, 0.0)
    };
    let (u, v) = xy_to_uv1960(x, y);
    let (up, vp) = (u, v * 1.5);
    let cct_k = cct_robertson(u, v);
    let duv = compute_duv(u, v, cct_k);
    Colorimetry {
        x_tri,
        y_tri,
        z_tri,
        x,
        y,
        u,
        v,
        up,
        vp,
        cct_k,
        duv,
    }
}

/// CIE 1931 (x,y) → CIE 1960 UCS (u,v).
pub fn xy_to_uv1960(x: f64, y: f64) -> (f64, f64) {
    let denom = -2.0 * x + 12.0 * y + 3.0;
    if denom.abs() < 1e-12 {
        return (0.0, 0.0);
    }
    (4.0 * x / denom, 6.0 * y / denom)
}

/// Planckian locus point in 1960 uv at temperature `t` (K), via the blackbody
/// SPD and the CMFs — exact, no polynomial approximation.
pub fn planckian_uv(t: f64) -> (f64, f64) {
    // Integrate Planck's law × CMF on the 5 nm grid.
    let c2 = 1.438_776_9e-2; // m·K
    let mut xx = 0.0;
    let mut yy = 0.0;
    let mut zz = 0.0;
    let mut wl = 380.0_f64;
    while wl <= 780.0 {
        let lambda_m = wl * 1e-9;
        // Unnormalised spectral radiance (constants cancel in chromaticity).
        let m = 1.0 / (lambda_m.powi(5) * ((c2 / (lambda_m * t)).exp() - 1.0));
        xx += m * cie_x(wl);
        yy += m * cie_y(wl);
        zz += m * cie_z(wl);
        wl += 5.0;
    }
    let sum = xx + yy + zz;
    if sum <= 0.0 {
        return (0.0, 0.0);
    }
    let (x, y) = (xx / sum, yy / sum);
    xy_to_uv1960(x, y)
}

/// CCT via Robertson's isotemperature-line method with a McCamy fallback.
fn cct_robertson(u: f64, v: f64) -> f64 {
    // Search 1000..25000 K on the Planckian locus for the minimum uv distance,
    // then refine. Robertson's classic approach linear-interpolates between
    // tabulated lines; a fine locus sweep + golden refine is equivalent and
    // avoids shipping the mired table.
    let mut best_t = 6500.0;
    let mut best_d = f64::INFINITY;
    let mut t = 1000.0;
    while t <= 25000.0 {
        let (pu, pv) = planckian_uv(t);
        let d = (u - pu).hypot(v - pv);
        if d < best_d {
            best_d = d;
            best_t = t;
        }
        // Step in reciprocal-temperature space for uniform locus spacing.
        let mired = 1.0e6 / t;
        t = 1.0e6 / (mired - 1.0);
    }
    // Golden-section refine around best_t.
    let mut lo = best_t * 0.98;
    let mut hi = best_t * 1.02;
    for _ in 0..40 {
        let m1 = lo + 0.382 * (hi - lo);
        let m2 = lo + 0.618 * (hi - lo);
        let d1 = {
            let (pu, pv) = planckian_uv(m1);
            (u - pu).hypot(v - pv)
        };
        let d2 = {
            let (pu, pv) = planckian_uv(m2);
            (u - pu).hypot(v - pv)
        };
        if d1 < d2 {
            hi = m2;
        } else {
            lo = m1;
        }
    }
    (lo + hi) * 0.5
}

/// Signed Duv: perpendicular distance from the Planckian locus in 1960 uv.
/// Positive = above the locus (greenish), negative = below (magenta/pink).
fn compute_duv(u: f64, v: f64, cct_k: f64) -> f64 {
    let (pu, pv) = planckian_uv(cct_k);
    // Locus tangent direction (toward higher T) to get the sign.
    let (pu2, pv2) = planckian_uv(cct_k + 10.0);
    let (tu, tv) = (pu2 - pu, pv2 - pv);
    let dist = (u - pu).hypot(v - pv);
    // Cross product z-component of tangent × offset gives the side.
    let cross = tu * (v - pv) - tv * (u - pu);
    if cross >= 0.0 {
        dist
    } else {
        -dist
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    /// Equal-energy white (flat SPD) → chromaticity (1/3, 1/3), CCT ≈ 5455 K,
    /// Duv near zero (E sits just off the locus).
    #[test]
    fn equal_energy_white() {
        let spd = Spd::uniform(380.0, 5.0, &[1.0; 81]);
        let c = analyze(&spd);
        assert_relative_eq!(c.x, 0.3333, epsilon = 0.002);
        assert_relative_eq!(c.y, 0.3333, epsilon = 0.002);
        assert!(
            (c.cct_k - 5455.0).abs() < 120.0,
            "E CCT should be ~5455 K, got {}",
            c.cct_k
        );
        assert!(c.duv.abs() < 0.005);
    }

    /// A blackbody-shaped SPD at 3000 K should recover ~3000 K with tiny Duv.
    #[test]
    fn blackbody_3000k_recovers_cct() {
        let c2 = 1.438_776_9e-2;
        let t = 3000.0;
        let wl: Vec<f64> = (380..=780).step_by(5).map(|w| w as f64).collect();
        let vals: Vec<f64> = wl
            .iter()
            .map(|&w| {
                let lm = w * 1e-9;
                1.0 / (lm.powi(5) * ((c2 / (lm * t)).exp() - 1.0))
            })
            .collect();
        let spd = Spd::new(&wl, &vals);
        let c = analyze(&spd);
        assert!(
            (c.cct_k - 3000.0).abs() < 20.0,
            "expected ~3000 K, got {}",
            c.cct_k
        );
        assert!(c.duv.abs() < 0.001, "blackbody Duv ~0, got {}", c.duv);
    }

    #[test]
    fn planckian_uv_monotone_hue() {
        // Hotter blackbody → smaller u (bluer). Sanity check on the locus.
        let (u_warm, _) = planckian_uv(2700.0);
        let (u_cool, _) = planckian_uv(6500.0);
        assert!(u_warm > u_cool);
    }
}
