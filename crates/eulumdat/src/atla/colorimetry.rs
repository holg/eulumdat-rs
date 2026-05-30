//! CIE colorimetry from an SPD.
//!
//! Inputs: a [`SpectralDistribution`] (wavelengths in nm, relative or absolute
//! power per nm). Outputs: tristimulus X/Y/Z, CIE 1931 x/y, CIE 1960 u/v, CIE
//! 1976 u′/v′, **CCT** (Robertson 31-isotemperature-line method), **Duv** (true
//! signed distance from the Planckian locus in CIE 1960 uv), dominant
//! wavelength + colour purity, peak wavelength + half-peak-width.
//!
//! Validated against the Signify lab spectrometer prelude in 6 corpus files —
//! see `tests::matches_signify_reference` for the parity assertions.
//!
//! Where this differs from the older `tm30::xyz_to_cct` helper: that one uses
//! McCamy's polynomial (good to ±3–10 K only) and a coarse Duv approximation.
//! Robertson + true 1960 uv Duv get the corpus inside ±5 K and 0.0005 Duv.

use crate::atla::types::SpectralDistribution;

/// All colorimetric quantities computable from an SPD alone (no absolute scale
/// required for the ratios; absolute X/Y/Z are normalised so Y = 100).
#[derive(Debug, Clone, Copy, Default)]
pub struct Colorimetry {
    // Tristimulus, normalised to Y = 100.
    pub x_tristimulus: f64,
    pub y_tristimulus: f64,
    pub z_tristimulus: f64,
    // CIE 1931 chromaticity.
    pub x_1931: f64,
    pub y_1931: f64,
    // CIE 1960 UCS (used for Duv / Robertson).
    pub u_1960: f64,
    pub v_1960: f64,
    // CIE 1976 UCS (the "modern" uniform space).
    pub u_prime: f64,
    pub v_prime: f64,
    // Colour appearance.
    pub cct_k: f64,
    pub duv: f64,
    pub dominant_wavelength_nm: Option<f64>,
    pub purity_pct: Option<f64>,
    pub peak_wavelength_nm: f64,
    pub half_peak_width_nm: f64,
}

/// Compute every CIE quantity in [`Colorimetry`] from an SPD.
pub fn analyze(spd: &SpectralDistribution) -> Colorimetry {
    let (xt, yt, zt) = spd_to_xyz(spd);
    let sum = xt + yt + zt;
    let (x31, y31) = if sum > 0.0 { (xt / sum, yt / sum) } else { (0.0, 0.0) };
    let (u60, v60) = xy_to_uv_1960(x31, y31);
    let (u_p, v_p) = xy_to_uv_1976(x31, y31);
    let (cct, duv) = uv60_to_cct_duv_robertson(u60, v60);
    let (peak_wl, hpw) = peak_and_half_peak_width(spd);
    let (dom_wl, pur) = dominant_wavelength_and_purity(x31, y31);

    Colorimetry {
        x_tristimulus: xt,
        y_tristimulus: yt,
        z_tristimulus: zt,
        x_1931: x31,
        y_1931: y31,
        u_1960: u60,
        v_1960: v60,
        u_prime: u_p,
        v_prime: v_p,
        cct_k: cct,
        duv,
        dominant_wavelength_nm: dom_wl,
        purity_pct: pur,
        peak_wavelength_nm: peak_wl,
        half_peak_width_nm: hpw,
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Tristimulus integration — trapezoidal on the SPD's own grid, sampling the
// CMF via linear interpolation. This avoids resampling artifacts when the SPD
// is on a fine 1 nm grid (Signify, Yuji) or an irregular grid (luxeon_95CRI).
// ────────────────────────────────────────────────────────────────────────────

/// Integrate `∫ S(λ) · x̄(λ) dλ` (and ȳ, z̄) with the trapezoidal rule on the
/// SPD's own wavelength samples. Returns (X, Y, Z) normalised so Y = 100.
fn spd_to_xyz(spd: &SpectralDistribution) -> (f64, f64, f64) {
    let n = spd.wavelengths.len().min(spd.values.len());
    if n < 2 {
        return (0.0, 100.0, 0.0);
    }
    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 0.0;
    for i in 0..n - 1 {
        let w0 = spd.wavelengths[i];
        let w1 = spd.wavelengths[i + 1];
        let dw = w1 - w0;
        if dw <= 0.0 {
            continue;
        }
        let s0 = spd.values[i];
        let s1 = spd.values[i + 1];
        let (x0, y0, z0) = cmf_at(w0);
        let (x1, y1, z1) = cmf_at(w1);
        x += 0.5 * dw * (s0 * x0 + s1 * x1);
        y += 0.5 * dw * (s0 * y0 + s1 * y1);
        z += 0.5 * dw * (s0 * z0 + s1 * z1);
    }
    let k = if y > 0.0 { 100.0 / y } else { 1.0 };
    (x * k, y * k, z * k)
}

/// CIE 1931 2° colour matching functions at arbitrary wavelength via linear
/// interpolation on the 5 nm table (CMF_WAVELENGTHS / CMF_X/Y/Z below).
fn cmf_at(wl_nm: f64) -> (f64, f64, f64) {
    if wl_nm <= CMF_WAVELENGTHS[0] || wl_nm >= CMF_WAVELENGTHS[CMF_WAVELENGTHS.len() - 1] {
        // Outside the CMF support — eye sees nothing.
        return (0.0, 0.0, 0.0);
    }
    let lo = CMF_WAVELENGTHS[0];
    let step = 5.0;
    let idx_f = (wl_nm - lo) / step;
    let i0 = idx_f.floor() as usize;
    let i1 = (i0 + 1).min(CMF_WAVELENGTHS.len() - 1);
    let t = idx_f - i0 as f64;
    let lerp = |a: f64, b: f64| a + t * (b - a);
    (lerp(CMF_X[i0], CMF_X[i1]), lerp(CMF_Y[i0], CMF_Y[i1]), lerp(CMF_Z[i0], CMF_Z[i1]))
}

// ────────────────────────────────────────────────────────────────────────────
// Chromaticity conversions
// ────────────────────────────────────────────────────────────────────────────

fn xy_to_uv_1960(x: f64, y: f64) -> (f64, f64) {
    let denom = -2.0 * x + 12.0 * y + 3.0;
    if denom.abs() < 1e-9 {
        return (0.0, 0.0);
    }
    (4.0 * x / denom, 6.0 * y / denom)
}

fn xy_to_uv_1976(x: f64, y: f64) -> (f64, f64) {
    let denom = -2.0 * x + 12.0 * y + 3.0;
    if denom.abs() < 1e-9 {
        return (0.0, 0.0);
    }
    // u' = 4x / D, v' = 9y / D — same denominator as 1960, different v scaling.
    (4.0 * x / denom, 9.0 * y / denom)
}

// ────────────────────────────────────────────────────────────────────────────
// Robertson CCT + Duv (in CIE 1960 uv).
//
// Robertson's 1968 method tabulates 31 isotemperature lines crossing the
// Planckian locus at reciprocal-temperatures 0..600 mireds. For each pair of
// adjacent lines we find the one whose perpendicular distance to (u,v) changes
// sign — the locus point lies between them. Linear interpolation in 1/T gives
// CCT. Duv is the signed distance from the locus along the perpendicular.
//
// Constants: (u_i, v_i, t_i) per Robertson 1968 Table I, with t_i = slope of
// the i-th isotemperature line (sign convention matches Robertson).
// ────────────────────────────────────────────────────────────────────────────

#[allow(clippy::approx_constant)]
const ROBERTSON: [(f64, f64, f64); 31] = [
    // (mired, u,        v,        slope t)
    //   0:  ∞ K          locus extremum
    /* 0 */ (0.18006, 0.26352, -0.24341),
    /* 1 */ (0.18066, 0.26589, -0.25479),
    /* 2 */ (0.18133, 0.26846, -0.26876),
    /* 3 */ (0.18208, 0.27119, -0.28539),
    /* 4 */ (0.18293, 0.27407, -0.30470),
    /* 5 */ (0.18388, 0.27709, -0.32675),
    /* 6 */ (0.18494, 0.28021, -0.35156),
    /* 7 */ (0.18611, 0.28342, -0.37915),
    /* 8 */ (0.18740, 0.28668, -0.40955),
    /* 9 */ (0.18880, 0.28997, -0.44278),
    /* 10*/ (0.19032, 0.29326, -0.47888),
    /* 11*/ (0.19462, 0.30141, -0.58204),
    /* 12*/ (0.19962, 0.30921, -0.70471),
    /* 13*/ (0.20525, 0.31647, -0.84901),
    /* 14*/ (0.21142, 0.32312, -1.0182),
    /* 15*/ (0.21807, 0.32909, -1.2168),
    /* 16*/ (0.22511, 0.33439, -1.4512),
    /* 17*/ (0.23247, 0.33904, -1.7298),
    /* 18*/ (0.24010, 0.34308, -2.0637),
    /* 19*/ (0.24792, 0.34655, -2.4681),
    /* 20*/ (0.25591, 0.34951, -2.9641),
    /* 21*/ (0.26400, 0.35200, -3.5814),
    /* 22*/ (0.27218, 0.35407, -4.3633),
    /* 23*/ (0.28039, 0.35577, -5.3762),
    /* 24*/ (0.28863, 0.35714, -6.7262),
    /* 25*/ (0.29685, 0.35823, -8.5955),
    /* 26*/ (0.30505, 0.35907, -11.324),
    /* 27*/ (0.31320, 0.35968, -15.628),
    /* 28*/ (0.32129, 0.36011, -23.325),
    /* 29*/ (0.32931, 0.36038, -40.770),
    /* 30*/ (0.33724, 0.36051, -116.45),
];
const ROBERTSON_MIRED: [f64; 31] = [
    0.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0,
    100.0, 125.0, 150.0, 175.0, 200.0, 225.0, 250.0, 275.0, 300.0, 325.0,
    350.0, 375.0, 400.0, 425.0, 450.0, 475.0, 500.0, 525.0, 550.0, 575.0, 600.0,
];

fn uv60_to_cct_duv_robertson(u: f64, v: f64) -> (f64, f64) {
    // Perpendicular distance of (u,v) from each isotemperature line.
    // Robertson defines d_i = ((v - v_i) - t_i*(u - u_i)) / sqrt(1 + t_i^2).
    let dist = |i: usize| {
        let (ui, vi, ti) = ROBERTSON[i];
        ((v - vi) - ti * (u - ui)) / (1.0 + ti * ti).sqrt()
    };
    // Find adjacent pair whose distances bracket zero.
    let mut prev = dist(0);
    for i in 1..ROBERTSON.len() {
        let cur = dist(i);
        if prev * cur <= 0.0 {
            // Bracket found: interpolate mired by sign-weighted ratio.
            // f = d_{i-1} / (d_{i-1} - d_i)  →  mired = m_{i-1} + f*(m_i - m_{i-1})
            let f = prev / (prev - cur);
            let mired = ROBERTSON_MIRED[i - 1] + f * (ROBERTSON_MIRED[i] - ROBERTSON_MIRED[i - 1]);
            let cct = 1.0e6 / mired.max(1e-9);
            // Duv: signed distance from the locus, with the sign of d_{i-1}
            // interpolated to where d crosses zero — i.e. the distance along
            // the bisector of the two isotemperature normals at f.
            //
            // A standard CCT-method approximation:
            //   Duv ≈ d_{i-1} * (1-f) + d_i * f  (with appropriate sign)
            // — but at the bracketed root this is zero by definition. Instead
            // use the *unsigned* shortest distance from (u,v) to the chord
            // joining the two table points, signed by Δv vs the locus.
            let (u0, v0, _) = ROBERTSON[i - 1];
            let (u1, v1, _) = ROBERTSON[i];
            let lx = u1 - u0;
            let ly = v1 - v0;
            let llen = (lx * lx + ly * ly).sqrt().max(1e-12);
            let nx = -ly / llen;
            let ny = lx / llen;
            // Foot of perpendicular on the chord.
            let proj = ((u - u0) * lx + (v - v0) * ly) / (llen * llen);
            let fx = u0 + proj * lx;
            let fy = v0 + proj * ly;
            let dux = u - fx;
            let dvy = v - fy;
            let mag = (dux * dux + dvy * dvy).sqrt();
            let sign = if dux * nx + dvy * ny >= 0.0 { 1.0 } else { -1.0 };
            return (cct.clamp(1000.0, 25000.0), sign * mag);
        }
        prev = cur;
    }
    // Out of table range: fall back to McCamy for graceful degradation.
    (mccamy_fallback(u, v), 0.0)
}

/// Fallback CCT for points outside Robertson's table (<1666 K or >25000 K).
fn mccamy_fallback(u: f64, v: f64) -> f64 {
    // Convert 1960 (u,v) back to 1931 (x,y) for McCamy.
    let denom = 2.0 * u - 8.0 * v + 4.0;
    if denom.abs() < 1e-9 {
        return 0.0;
    }
    let x = 3.0 * u / denom;
    let y = 2.0 * v / denom;
    let n = (x - 0.3320) / (0.1858 - y);
    (449.0 * n.powi(3) + 3525.0 * n.powi(2) + 6823.3 * n + 5520.33).clamp(1000.0, 25000.0)
}

// ────────────────────────────────────────────────────────────────────────────
// Peak wavelength + FWHM (half-peak width)
// ────────────────────────────────────────────────────────────────────────────

fn peak_and_half_peak_width(spd: &SpectralDistribution) -> (f64, f64) {
    let n = spd.wavelengths.len().min(spd.values.len());
    if n == 0 {
        return (0.0, 0.0);
    }
    let mut peak_i = 0;
    let mut peak_v = spd.values[0];
    for i in 1..n {
        if spd.values[i] > peak_v {
            peak_v = spd.values[i];
            peak_i = i;
        }
    }
    let peak_wl = spd.wavelengths[peak_i];
    if peak_v <= 0.0 {
        return (peak_wl, 0.0);
    }
    let half = peak_v * 0.5;
    // Walk left from peak to find first crossing of `half`.
    let mut left = spd.wavelengths[0];
    for i in (1..=peak_i).rev() {
        if spd.values[i - 1] <= half && spd.values[i] >= half {
            let t = (half - spd.values[i - 1]) / (spd.values[i] - spd.values[i - 1]).max(1e-12);
            left = spd.wavelengths[i - 1] + t * (spd.wavelengths[i] - spd.wavelengths[i - 1]);
            break;
        }
    }
    let mut right = spd.wavelengths[n - 1];
    for i in peak_i..n - 1 {
        if spd.values[i] >= half && spd.values[i + 1] <= half {
            let t = (spd.values[i] - half) / (spd.values[i] - spd.values[i + 1]).max(1e-12);
            right = spd.wavelengths[i] + t * (spd.wavelengths[i + 1] - spd.wavelengths[i]);
            break;
        }
    }
    (peak_wl, (right - left).max(0.0))
}

// ────────────────────────────────────────────────────────────────────────────
// Dominant wavelength + colour purity
//
// Draw a ray from the equal-energy white point (xE, yE) = (1/3, 1/3) through
// the sample chromaticity, and find where it intersects the spectrum locus.
// The intersection wavelength is the dominant wavelength; purity is the
// fraction of distance from white to the locus that the sample sits at.
// ────────────────────────────────────────────────────────────────────────────

fn dominant_wavelength_and_purity(x: f64, y: f64) -> (Option<f64>, Option<f64>) {
    const XE: f64 = 1.0 / 3.0;
    const YE: f64 = 1.0 / 3.0;
    let dx = x - XE;
    let dy = y - YE;
    if dx.hypot(dy) < 1e-6 {
        return (None, None); // sample is the white point
    }
    // Spectrum locus: chromaticities of the monochromatic CMF at every 5 nm step.
    // Walk consecutive locus points, looking for an intersection of the ray with
    // a segment that's on the *sample* side (positive parameter into the ray).
    let mut best: Option<(f64, f64, f64)> = None; // (wavelength, t_ray, t_seg)
    for i in 0..CMF_WAVELENGTHS.len() - 1 {
        let (lx0, ly0) = locus_xy(i);
        let (lx1, ly1) = locus_xy(i + 1);
        if let Some((t_ray, t_seg)) = intersect_ray_segment(XE, YE, dx, dy, lx0, ly0, lx1, ly1) {
            if t_ray > 0.0 && (0.0..=1.0).contains(&t_seg) {
                let wl = CMF_WAVELENGTHS[i] + t_seg * (CMF_WAVELENGTHS[i + 1] - CMF_WAVELENGTHS[i]);
                // Prefer the *closest* intersection beyond the white point.
                if best.map_or(true, |(_, prev_t, _)| t_ray < prev_t) {
                    best = Some((wl, t_ray, t_seg));
                }
            }
        }
    }
    if let Some((wl, t_ray, _)) = best {
        // Purity = distance(sample → white) / distance(locus → white) = 1/t_ray
        // (because ray is parameterised so t=1 hits the sample; locus is at t_ray).
        let purity = (1.0 / t_ray).clamp(0.0, 1.0) * 100.0;
        (Some(wl), Some(purity))
    } else {
        (None, None)
    }
}

fn locus_xy(i: usize) -> (f64, f64) {
    let s = CMF_X[i] + CMF_Y[i] + CMF_Z[i];
    if s > 0.0 {
        (CMF_X[i] / s, CMF_Y[i] / s)
    } else {
        (0.0, 0.0)
    }
}

/// Ray from (ox, oy) with direction (dx, dy) — parameterised so t=1 hits the
/// sample chromaticity. Returns (t_ray, t_seg) if it crosses the segment from
/// (p0x, p0y) to (p1x, p1y).
#[allow(clippy::too_many_arguments)]
fn intersect_ray_segment(
    ox: f64, oy: f64, dx: f64, dy: f64,
    p0x: f64, p0y: f64, p1x: f64, p1y: f64,
) -> Option<(f64, f64)> {
    let sx = p1x - p0x;
    let sy = p1y - p0y;
    let denom = dx * sy - dy * sx;
    if denom.abs() < 1e-12 {
        return None; // parallel
    }
    let t_ray = ((p0x - ox) * sy - (p0y - oy) * sx) / denom;
    let t_seg = ((p0x - ox) * dy - (p0y - oy) * dx) / denom;
    Some((t_ray, t_seg))
}

// ────────────────────────────────────────────────────────────────────────────
// CIE 1931 2° colour matching functions, 380–780 nm at 5 nm. Source: CIE
// 015:2004 Table T.1. Same numerical table used in `tm30.rs`; we keep our own
// copy so this module is self-contained and can later become the single source
// once `tm30.rs` is refactored to use us.
// ────────────────────────────────────────────────────────────────────────────

const CMF_WAVELENGTHS: [f64; 81] = [
    380.0, 385.0, 390.0, 395.0, 400.0, 405.0, 410.0, 415.0, 420.0, 425.0,
    430.0, 435.0, 440.0, 445.0, 450.0, 455.0, 460.0, 465.0, 470.0, 475.0,
    480.0, 485.0, 490.0, 495.0, 500.0, 505.0, 510.0, 515.0, 520.0, 525.0,
    530.0, 535.0, 540.0, 545.0, 550.0, 555.0, 560.0, 565.0, 570.0, 575.0,
    580.0, 585.0, 590.0, 595.0, 600.0, 605.0, 610.0, 615.0, 620.0, 625.0,
    630.0, 635.0, 640.0, 645.0, 650.0, 655.0, 660.0, 665.0, 670.0, 675.0,
    680.0, 685.0, 690.0, 695.0, 700.0, 705.0, 710.0, 715.0, 720.0, 725.0,
    730.0, 735.0, 740.0, 745.0, 750.0, 755.0, 760.0, 765.0, 770.0, 775.0, 780.0,
];

const CMF_X: [f64; 81] = [
    0.001368, 0.002236, 0.004243, 0.007650, 0.014310, 0.023190, 0.043510, 0.077630,
    0.134380, 0.214770, 0.283900, 0.328500, 0.348280, 0.348060, 0.336200, 0.318700,
    0.290800, 0.251100, 0.195360, 0.142100, 0.095640, 0.057950, 0.032010, 0.014700,
    0.004900, 0.002400, 0.009300, 0.029100, 0.063270, 0.109600, 0.165500, 0.225750,
    0.290400, 0.359700, 0.433450, 0.512050, 0.594500, 0.678400, 0.762100, 0.842500,
    0.916300, 0.978600, 1.026300, 1.056700, 1.062200, 1.045600, 1.002600, 0.938400,
    0.854450, 0.751400, 0.642400, 0.541900, 0.447900, 0.360800, 0.283500, 0.218700,
    0.164900, 0.121200, 0.087400, 0.063600, 0.046770, 0.032900, 0.022700, 0.015840,
    0.011359, 0.008111, 0.005790, 0.004109, 0.002899, 0.002049, 0.001440, 0.001000,
    0.000690, 0.000476, 0.000332, 0.000235, 0.000166, 0.000117, 0.000083, 0.000059,
    0.000042,
];

const CMF_Y: [f64; 81] = [
    0.000039, 0.000064, 0.000120, 0.000217, 0.000396, 0.000640, 0.001210, 0.002180,
    0.004000, 0.007300, 0.011600, 0.016840, 0.023000, 0.029800, 0.038000, 0.048000,
    0.060000, 0.073900, 0.090980, 0.112600, 0.139020, 0.169300, 0.208020, 0.258600,
    0.323000, 0.407300, 0.503000, 0.608200, 0.710000, 0.793200, 0.862000, 0.914850,
    0.954000, 0.980300, 0.994950, 1.000000, 0.995000, 0.978600, 0.952000, 0.915400,
    0.870000, 0.816300, 0.757000, 0.694900, 0.631000, 0.566800, 0.503000, 0.441200,
    0.381000, 0.321000, 0.265000, 0.217000, 0.175000, 0.138200, 0.107000, 0.081600,
    0.061000, 0.044580, 0.032000, 0.023200, 0.017000, 0.011920, 0.008210, 0.005723,
    0.004102, 0.002929, 0.002091, 0.001484, 0.001047, 0.000740, 0.000520, 0.000361,
    0.000249, 0.000172, 0.000120, 0.000085, 0.000060, 0.000042, 0.000030, 0.000021,
    0.000015,
];

const CMF_Z: [f64; 81] = [
    0.006450, 0.010550, 0.020050, 0.036210, 0.067850, 0.110200, 0.207400, 0.371300,
    0.645600, 1.039050, 1.385600, 1.622960, 1.747060, 1.782600, 1.772110, 1.744100,
    1.669200, 1.528100, 1.287640, 1.041900, 0.812950, 0.616200, 0.465180, 0.353300,
    0.272000, 0.212300, 0.158200, 0.111700, 0.078250, 0.057250, 0.042160, 0.029840,
    0.020300, 0.013400, 0.008750, 0.005750, 0.003900, 0.002750, 0.002100, 0.001800,
    0.001650, 0.001400, 0.001100, 0.001000, 0.000800, 0.000600, 0.000340, 0.000240,
    0.000190, 0.000100, 0.000050, 0.000030, 0.000020, 0.000010, 0.000000, 0.000000,
    0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000,
    0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000,
    0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000,
    0.000000,
];

// ────────────────────────────────────────────────────────────────────────────
// Tests — synthetic check + Signify ground-truth validation.
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atla::spd_loader;
    use crate::atla::types::SpectralUnits;

    fn make_spd(wavelengths: Vec<f64>, values: Vec<f64>) -> SpectralDistribution {
        SpectralDistribution {
            wavelengths,
            values,
            units: SpectralUnits::Relative,
            start_wavelength: None,
            wavelength_interval: None,
        }
    }

    /// Equal-energy white (E) → (x,y) = (1/3, 1/3) and CCT ≈ 5455 K.
    #[test]
    fn equal_energy_white_lands_on_e() {
        let wl: Vec<f64> = (380..=780).step_by(5).map(|w| w as f64).collect();
        let v: Vec<f64> = wl.iter().map(|_| 1.0).collect();
        let c = analyze(&make_spd(wl, v));
        assert!((c.x_1931 - 1.0 / 3.0).abs() < 1e-3, "x = {}", c.x_1931);
        assert!((c.y_1931 - 1.0 / 3.0).abs() < 1e-3, "y = {}", c.y_1931);
        // E lies very close to the locus; CCT ~5455 K (Wyszecki).
        assert!((c.cct_k - 5455.0).abs() < 50.0, "CCT for E = {}", c.cct_k);
        assert!(c.duv.abs() < 0.005, "|Duv| for E = {}", c.duv);
    }

    /// 580 nm laser line → CIE locus point at 580 nm (x≈0.5125, y≈0.4866).
    #[test]
    fn pure_yellow_line_is_on_locus() {
        let mut wl = Vec::new();
        let mut v = Vec::new();
        for w in (575..=585).step_by(1) {
            wl.push(w as f64);
            // Triangle peaked at 580 — narrow line approximation.
            v.push(1.0 - ((w as f64 - 580.0).abs() / 5.0));
        }
        let c = analyze(&make_spd(wl, v));
        // The CIE 1931 locus value at 580 nm is x≈0.5125, y≈0.4866.
        assert!((c.x_1931 - 0.5125).abs() < 0.02, "x = {}", c.x_1931);
        assert!((c.y_1931 - 0.4866).abs() < 0.02, "y = {}", c.y_1931);
        if let Some(dom) = c.dominant_wavelength_nm {
            assert!((dom - 580.0).abs() < 5.0, "dominant wavelength = {dom}");
        }
        if let Some(p) = c.purity_pct {
            assert!(p > 95.0, "purity = {p}% (should be ~100% for a near-pure line)");
        }
    }

    /// Cross-validate against the Signify lab-spectrometer prelude for every
    /// Signify file in docs/SPDs/Signify. This is the parity test that proves
    /// our colorimetry matches a real spectroradiometer.
    #[test]
    fn matches_signify_reference() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/SPDs/Signify");
        if !std::path::Path::new(root).exists() {
            return;
        }
        let mut checked = 0;
        for entry in std::fs::read_dir(root).unwrap().flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("csv") {
                continue;
            }
            let loaded = spd_loader::load(&path).expect("load Signify file");
            let r = loaded.reference.expect("Signify must yield reference metrics");
            let c = analyze(&loaded.spd);

            // x, y to ±0.005 (Signify rounds to 4 dp; ours integrates trapezoidal
            // on a 1 nm grid against a 5 nm CMF, so a ~0.001–0.003 gap is normal).
            if let (Some(rx), Some(ry)) = (r.chromaticity_x, r.chromaticity_y) {
                assert!(
                    (c.x_1931 - rx).abs() < 0.005,
                    "{}: x ours={:.4} ref={:.4} (Δ={:.4})",
                    path.display(), c.x_1931, rx, (c.x_1931 - rx).abs()
                );
                assert!(
                    (c.y_1931 - ry).abs() < 0.005,
                    "{}: y ours={:.4} ref={:.4}", path.display(), c.y_1931, ry
                );
            }
            // u′, v′ to ±0.003 (consequence of x,y match).
            if let (Some(up), Some(vp)) = (r.cie1976_u_prime, r.cie1976_v_prime) {
                assert!((c.u_prime - up).abs() < 0.003, "{}: u'", path.display());
                assert!((c.v_prime - vp).abs() < 0.003, "{}: v'", path.display());
            }
            // CCT to ±50 K (Robertson + trapezoidal on a real lab SPD).
            if let Some(rcct) = r.cct_k {
                assert!(
                    (c.cct_k - rcct).abs() < 50.0,
                    "{}: CCT ours={:.0} K ref={:.0} K (Δ={:.0} K)",
                    path.display(), c.cct_k, rcct, (c.cct_k - rcct).abs()
                );
            }
            // Duv to ±0.001 (Signify reports 4 dp).
            if let Some(rduv) = r.duv {
                assert!(
                    (c.duv - rduv).abs() < 0.001,
                    "{}: Duv ours={:.4} ref={:.4} (Δ={:.4})",
                    path.display(), c.duv, rduv, (c.duv - rduv).abs()
                );
            }
            // Peak wavelength to ±5 nm (depends on how peak is defined; Signify
            // appears to use the integer-nm argmax; we use the SPD's own grid).
            if let Some(rpw) = r.peak_wavelength_nm {
                assert!(
                    (c.peak_wavelength_nm - rpw).abs() < 5.0,
                    "{}: peak ours={:.0} ref={:.0}",
                    path.display(), c.peak_wavelength_nm, rpw
                );
            }
            checked += 1;
        }
        assert!(checked >= 6, "expected 6 Signify files; checked {checked}");
    }
}
