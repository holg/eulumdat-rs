//! Shared spectral building blocks for the colour-quality metrics
//! (TM-30 / CIE 224 in `tm30.rs`, CIE 13.3 CRI in `cri.rs`).
//!
//! Everything works on one grid: 380–780 nm at 5 nm (`cie_tables::N` values),
//! the grid of the CIE data tables and of the TM-30 colour evaluation samples.

use super::cie_tables::{self as t, N};
use crate::atla::types::SpectralDistribution;

/// A spectrum on the 380–780 nm / 5 nm grid.
pub(crate) type Grid = [f64; N];

/// Colour-matching functions of one standard observer.
pub(crate) struct Observer {
    pub x: &'static Grid,
    pub y: &'static Grid,
    pub z: &'static Grid,
}

/// CIE 1931 2° standard observer.
pub(crate) const OBSERVER_2: Observer = Observer {
    x: &t::CMF2_X,
    y: &t::CMF2_Y,
    z: &t::CMF2_Z,
};

/// CIE 1964 10° standard observer.
pub(crate) const OBSERVER_10: Observer = Observer {
    x: &t::CMF10_X,
    y: &t::CMF10_Y,
    z: &t::CMF10_Z,
};

/// Wavelength (nm) of grid index `i`.
pub(crate) fn wavelength(i: usize) -> f64 {
    t::START_NM + t::STEP_NM * i as f64
}

/// Resamples a measured spectrum onto the grid. Outside the measured range
/// the values are zero, as CIE 224:2017 recommends for spectra that do not
/// span 380–780 nm.
///
/// Coarser or equal spacing (≥ 5 nm): linear interpolation. Finer spacing
/// (e.g. 1 nm spectrometer data): the mean over each 5 nm band, so narrow
/// emission lines between two grid points are not lost or over-counted.
pub(crate) fn resample(spd: &SpectralDistribution) -> Grid {
    let mut out = [0.0; N];
    let n = spd.wavelengths.len().min(spd.values.len());
    if n == 0 {
        return out;
    }
    // Sort by wavelength (files are usually sorted; be tolerant).
    let mut pts: Vec<(f64, f64)> = (0..n)
        .map(|i| (spd.wavelengths[i], spd.values[i]))
        .filter(|(w, v)| w.is_finite() && v.is_finite())
        .collect();
    pts.sort_by(|a, b| a.0.total_cmp(&b.0));
    pts.dedup_by(|b, a| (b.0 - a.0).abs() < 1e-9);
    if pts.is_empty() {
        return out;
    }
    let mut steps: Vec<f64> = pts.windows(2).map(|w| w[1].0 - w[0].0).collect();
    steps.sort_by(f64::total_cmp);
    let median_step = steps.get(steps.len() / 2).copied().unwrap_or(t::STEP_NM);
    if median_step < 0.75 * t::STEP_NM {
        for (i, o) in out.iter_mut().enumerate() {
            let wl = wavelength(i);
            let half = 0.5 * t::STEP_NM;
            *o = integrate_linear(&pts, wl - half, wl + half) / t::STEP_NM;
        }
        return out;
    }
    for (i, o) in out.iter_mut().enumerate() {
        *o = interpolate_linear(&pts, wavelength(i));
    }
    out
}

/// Piecewise-linear value at `x`, zero outside the data range.
fn interpolate_linear(pts: &[(f64, f64)], x: f64) -> f64 {
    let (lo, hi) = (pts[0].0, pts[pts.len() - 1].0);
    if x < lo || x > hi {
        return 0.0;
    }
    let j = pts.partition_point(|p| p.0 <= x).saturating_sub(1);
    let (w0, v0) = pts[j];
    match pts.get(j + 1) {
        Some(&(w1, v1)) if w1 > w0 => v0 + (x - w0) / (w1 - w0) * (v1 - v0),
        _ => v0,
    }
}

/// Integral of the piecewise-linear data over [a, b] (zero outside it).
fn integrate_linear(pts: &[(f64, f64)], a: f64, b: f64) -> f64 {
    let mut sum = 0.0;
    for w in pts.windows(2) {
        let (x0, y0) = w[0];
        let (x1, y1) = w[1];
        let lo = x0.max(a);
        let hi = x1.min(b);
        if hi <= lo || x1 <= x0 {
            continue;
        }
        let f = |x: f64| y0 + (x - x0) / (x1 - x0) * (y1 - y0);
        sum += 0.5 * (f(lo) + f(hi)) * (hi - lo);
    }
    sum
}

/// Relative spectrum of a Planckian (black-body) radiator.
pub(crate) fn planck(cct: f64) -> Grid {
    const C2: f64 = 1.4388e-2; // m·K
    let mut out = [0.0; N];
    for (i, o) in out.iter_mut().enumerate() {
        let wl = wavelength(i) * 1e-9;
        *o = 1.0 / (wl.powi(5) * ((C2 / (wl * cct)).exp() - 1.0));
    }
    let max = out.iter().cloned().fold(0.0, f64::max);
    if max > 0.0 {
        for v in &mut out {
            *v /= max;
        }
    }
    out
}

/// CIE D-series daylight at a correlated colour temperature (CIE 015):
/// chromaticity on the daylight locus, then S0 + M1·S1 + M2·S2 with M1, M2
/// rounded to three decimals as CIE 015 prescribes.
pub(crate) fn daylight(cct: f64) -> Grid {
    let tt = cct.clamp(4000.0, 25000.0);
    let xd = if tt <= 7000.0 {
        -4.6070e9 / tt.powi(3) + 2.9678e6 / tt.powi(2) + 0.09911e3 / tt + 0.244063
    } else {
        -2.0064e9 / tt.powi(3) + 1.9018e6 / tt.powi(2) + 0.24748e3 / tt + 0.237040
    };
    let yd = -3.0 * xd * xd + 2.870 * xd - 0.275;
    let m = 0.0241 + 0.2562 * xd - 0.7341 * yd;
    let round3 = |v: f64| (v * 1000.0).round() / 1000.0;
    let m1 = round3((-1.3515 - 1.7703 * xd + 5.9114 * yd) / m);
    let m2 = round3((0.0300 - 31.4424 * xd + 30.0717 * yd) / m);
    let mut out = [0.0; N];
    for (i, o) in out.iter_mut().enumerate() {
        *o = t::D_S0[i] + m1 * t::D_S1[i] + m2 * t::D_S2[i];
    }
    out
}

/// Unnormalised tristimulus sums of a spectrum, optionally seen through a
/// reflectance (`refl`).
pub(crate) fn tristimulus(s: &Grid, refl: Option<&Grid>, obs: &Observer) -> [f64; 3] {
    let mut xyz = [0.0; 3];
    for i in 0..N {
        let p = s[i] * refl.map_or(1.0, |r| r[i]);
        xyz[0] += p * obs.x[i];
        xyz[1] += p * obs.y[i];
        xyz[2] += p * obs.z[i];
    }
    xyz
}

/// CIE 1960 UCS (u, v) of a tristimulus value.
pub(crate) fn uv1960(xyz: [f64; 3]) -> (f64, f64) {
    let d = xyz[0] + 15.0 * xyz[1] + 3.0 * xyz[2];
    if d.abs() < 1e-30 {
        return (0.0, 0.0);
    }
    (4.0 * xyz[0] / d, 6.0 * xyz[1] / d)
}

/// (u, v) of a Planckian radiator at `cct`, CIE 1931 2° observer.
fn planck_uv(cct: f64) -> (f64, f64) {
    uv1960(tristimulus(&planck(cct), None, &OBSERVER_2))
}

/// Correlated colour temperature and Duv of a CIE 1960 (u, v) point.
///
/// Exact by definition: the CCT is the temperature of the nearest point on
/// the Planckian locus (computed with the CIE 1931 2° observer), Duv the
/// signed distance to it, positive above the locus (towards green). The
/// search runs in reciprocal temperature from 1 000 K to 50 000 K.
pub fn cct_duv(u: f64, v: f64) -> (f64, f64) {
    let dist2 = |mired: f64| {
        let (pu, pv) = planck_uv(1.0e6 / mired);
        (u - pu).powi(2) + (v - pv).powi(2)
    };
    // Coarse scan, then golden-section refinement around the best sample.
    let (lo_m, hi_m) = (20.0, 1000.0);
    let steps = 98;
    let mut best = lo_m;
    let mut best_d = f64::MAX;
    for k in 0..=steps {
        let m = lo_m + (hi_m - lo_m) * k as f64 / steps as f64;
        let d = dist2(m);
        if d < best_d {
            best_d = d;
            best = m;
        }
    }
    let step = (hi_m - lo_m) / steps as f64;
    let (mut a, mut b) = ((best - step).max(lo_m), (best + step).min(hi_m));
    let g = 0.5 * (5f64.sqrt() - 1.0);
    let mut c = b - g * (b - a);
    let mut d = a + g * (b - a);
    let (mut fc, mut fd) = (dist2(c), dist2(d));
    for _ in 0..80 {
        if fc < fd {
            b = d;
            d = c;
            fd = fc;
            c = b - g * (b - a);
            fc = dist2(c);
        } else {
            a = c;
            c = d;
            fc = fd;
            d = a + g * (b - a);
            fd = dist2(d);
        }
        if (b - a).abs() < 1e-9 {
            break;
        }
    }
    let mired = 0.5 * (a + b);
    let cct = 1.0e6 / mired;
    let (pu, pv) = planck_uv(cct);
    // Normal of the locus pointing towards higher v ("above" = greenish).
    let (u1, v1) = planck_uv(1.0e6 / (mired + 0.01));
    let (tu, tv) = (u1 - pu, v1 - pv);
    let (mut nu, mut nv) = (-tv, tu);
    if nv < 0.0 {
        nu = -nu;
        nv = -nv;
    }
    let nlen = (nu * nu + nv * nv).sqrt().max(1e-30);
    let duv = ((u - pu) * nu + (v - pv) * nv) / nlen;
    (cct, duv)
}

/// CCT and Duv of a measured spectrum at its own resolution: chromaticity
/// from `colorimetry::analyze` (CIE 1931 2°, integrated on the spectrum's
/// grid), then the exact locus search of [`cct_duv`].
pub(crate) fn measured_cct_duv(spd: &SpectralDistribution) -> (f64, f64) {
    let c = super::colorimetry::analyze(spd);
    cct_duv(c.u_1960, c.v_1960)
}

/// CCT and Duv of a gridded spectrum (CIE 1931 2° observer).
#[cfg(test)]
pub(crate) fn spectrum_cct_duv(s: &Grid) -> (f64, f64) {
    let (u, v) = uv1960(tristimulus(s, None, &OBSERVER_2));
    cct_duv(u, v)
}

/// Whether a measured spectrum covers enough of the visible range for the
/// colour-rendering metrics (at least 400–700 nm with some signal).
pub(crate) fn covers_visible(spd: &SpectralDistribution) -> bool {
    let n = spd.wavelengths.len().min(spd.values.len());
    if n < 2 {
        return false;
    }
    let lo = spd.wavelengths[..n]
        .iter()
        .cloned()
        .fold(f64::MAX, f64::min);
    let hi = spd.wavelengths[..n]
        .iter()
        .cloned()
        .fold(f64::MIN, f64::max);
    lo <= 400.0 && hi >= 700.0 && spd.values[..n].iter().any(|v| *v > 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planckian_points_have_zero_duv_and_their_own_cct() {
        for cct in [1800.0, 2700.0, 4000.0, 6500.0, 10000.0] {
            let (u, v) = planck_uv(cct);
            let (c, d) = cct_duv(u, v);
            assert!((c - cct).abs() / cct < 1e-6, "{cct}: {c}");
            assert!(d.abs() < 1e-9, "{cct}: duv {d}");
        }
    }

    #[test]
    fn points_above_the_locus_are_positive() {
        let (u, v) = planck_uv(3000.0);
        let (_, d_up) = cct_duv(u, v + 0.004);
        let (_, d_down) = cct_duv(u, v - 0.004);
        assert!(d_up > 0.0035 && d_down < -0.0035, "{d_up} {d_down}");
    }

    #[test]
    fn d65_from_the_basis_functions() {
        // CIE D65: CCT ≈ 6504 K, slightly above the locus (Duv ≈ +0.0032).
        let (cct, duv) = spectrum_cct_duv(&daylight(6504.0));
        assert!((cct - 6504.0).abs() < 15.0, "{cct}");
        assert!((duv - 0.0032).abs() < 0.0004, "{duv}");
    }
}
