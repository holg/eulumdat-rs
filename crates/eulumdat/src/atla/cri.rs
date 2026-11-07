//! CIE 13.3 colour rendering index: Ra, the special indices R1–R15 and
//! Re (mean of R1–R15).
//!
//! Method (CIE 13.3-1995):
//! - spectrum on 380–780 nm at 5 nm, CIE 1931 2° observer;
//! - reference: Planckian radiator below 5000 K, CIE daylight from 5000 K,
//!   at the test source's CCT;
//! - von Kries adaptation in CIE 1960 uv (the c, d form of CIE 13.3);
//! - CIE 1964 U*V*W*, Ri = 100 − 4.6·ΔE, Ra = mean of R1–R8.
//!
//! R9–R14 are the saturated and natural colours of CIE 13.3; R15 is the
//! Japanese skin sample (CIE data set TCS15). The CRI is only defined for
//! sources near the Planckian locus: CIE 13.3 asks for |Duv| < 5.4·10⁻³,
//! `within_duv_limit` reports it.
//!
//! The implementation is our own; its results are compared with an
//! independent implementation (colour-science) in the tests.

use super::cie_tables::TCS;
use super::colour_common::{
    covers_visible, daylight, measured_cct_duv, planck, resample, tristimulus, uv1960, Grid,
    OBSERVER_2,
};
use crate::atla::types::SpectralDistribution;

/// CIE 13.3 |Duv| limit for a meaningful CRI.
pub const CRI_DUV_LIMIT: f64 = 5.4e-3;

/// CRI results.
#[derive(Debug, Clone)]
pub struct CriResult {
    /// General colour rendering index Ra (mean of R1–R8).
    pub ra: f64,
    /// Special colour rendering indices R1–R15 (`ri[8]` is R9).
    pub ri: [f64; 15],
    /// Mean of R1–R15 ("Re").
    pub re: f64,
    /// CCT used for the reference (K), CIE 1931 2° observer.
    pub cct: f64,
    /// Duv of the test source.
    pub duv: f64,
    /// Reference is CIE daylight (CCT ≥ 5000 K), else Planckian.
    pub daylight_reference: bool,
    /// |Duv| below the CIE 13.3 limit of 5.4·10⁻³.
    pub within_duv_limit: bool,
}

impl CriResult {
    /// R9 (strong red).
    pub fn r9(&self) -> f64 {
        self.ri[8]
    }
}

fn c_d(u: f64, v: f64) -> (f64, f64) {
    (
        (4.0 - u - 10.0 * v) / v,
        (1.708 * v + 0.404 - 1.481 * u) / v,
    )
}

/// U*V*W* of the 15 samples under an illuminant, relative to the white
/// (u_w, v_w); with `adapt = Some((c_t, d_t, c_r, d_r))` the samples are
/// first adapted from the test white to the reference white.
fn uvw(s: &Grid, white: (f64, f64), adapt: Option<(f64, f64, f64, f64)>) -> [(f64, f64, f64); 15] {
    let y_w = tristimulus(s, None, &OBSERVER_2)[1];
    let k = 100.0 / y_w;
    let mut out = [(0.0, 0.0, 0.0); 15];
    for (i, o) in out.iter_mut().enumerate() {
        let xyz = tristimulus(s, Some(&TCS[i]), &OBSERVER_2).map(|v| v * k);
        let (mut u, mut v) = uv1960(xyz);
        if let Some((c_t, d_t, c_r, d_r)) = adapt {
            let (c_i, d_i) = c_d(u, v);
            let cc = c_r / c_t * c_i;
            let dd = d_r / d_t * d_i;
            let den = 16.518 + 1.481 * cc - dd;
            u = (10.872 + 0.404 * cc - 4.0 * dd) / den;
            v = 5.520 / den;
        }
        let w = 25.0 * xyz[1].max(0.0).powf(1.0 / 3.0) - 17.0;
        *o = (13.0 * w * (u - white.0), 13.0 * w * (v - white.1), w);
    }
    out
}

/// Calculate the CIE 13.3 colour rendering indices of a spectrum.
///
/// Returns `None` when the spectrum does not cover at least 400–700 nm.
pub fn calculate_cri(spd: &SpectralDistribution) -> Option<CriResult> {
    if !covers_visible(spd) {
        return None;
    }
    let test = resample(spd);
    let (cct, duv) = measured_cct_duv(spd);
    if !cct.is_finite() {
        return None;
    }
    let daylight_reference = cct >= 5000.0;
    let reference = if daylight_reference {
        daylight(cct)
    } else {
        planck(cct)
    };

    let (u_t, v_t) = uv1960(tristimulus(&test, None, &OBSERVER_2));
    let (u_r, v_r) = uv1960(tristimulus(&reference, None, &OBSERVER_2));
    let (c_t, d_t) = c_d(u_t, v_t);
    let (c_r, d_r) = c_d(u_r, v_r);

    let under_test = uvw(&test, (u_r, v_r), Some((c_t, d_t, c_r, d_r)));
    let under_ref = uvw(&reference, (u_r, v_r), None);

    let mut ri = [0.0; 15];
    for i in 0..15 {
        let (a, b) = (under_test[i], under_ref[i]);
        let de = ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2) + (a.2 - b.2).powi(2)).sqrt();
        ri[i] = 100.0 - 4.6 * de;
    }
    let ra = ri[..8].iter().sum::<f64>() / 8.0;
    let re = ri.iter().sum::<f64>() / 15.0;
    Some(CriResult {
        ra,
        ri,
        re,
        cct,
        duv,
        daylight_reference,
        within_duv_limit: duv.abs() < CRI_DUV_LIMIT,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atla::cie_tables::N;
    use crate::atla::colour_common::wavelength;

    fn spd_from_grid(g: &Grid) -> SpectralDistribution {
        SpectralDistribution {
            wavelengths: (0..N).map(wavelength).collect(),
            values: g.to_vec(),
            units: crate::atla::types::SpectralUnits::Relative,
            start_wavelength: None,
            wavelength_interval: None,
        }
    }

    /// Against the lab spectrometer values (Ra, R1–R15, CCT, Duv) in the
    /// Signify files of docs/SPDs/Signify (1 nm spectra, food lighting).
    #[test]
    fn matches_signify_lab_values() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/SPDs/Signify");
        if !std::path::Path::new(root).exists() {
            return;
        }
        let (mut worst_ra, mut worst_ri, mut worst_cct, mut worst_duv) = (0f64, 0f64, 0f64, 0f64);
        let mut checked = 0;
        for entry in std::fs::read_dir(root).unwrap().flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("csv") {
                continue;
            }
            let loaded = crate::atla::spd_loader::load(&path).expect("load Signify file");
            let lab = loaded.reference.expect("lab values");
            let c = calculate_cri(&loaded.spd).expect("CRI");
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            if let Some(ra) = lab.ra {
                println!("{name}: Ra ours {:.1} lab {ra:.1}", c.ra);
                worst_ra = worst_ra.max((c.ra - ra).abs());
            }
            for (i, r) in lab.r_special.iter().enumerate() {
                if let Some(r) = r {
                    worst_ri = worst_ri.max((c.ri[i] - r).abs());
                    if (c.ri[i] - r).abs() > 1.0 {
                        println!("{name}: R{} ours {:.1} lab {r:.1}", i + 1, c.ri[i]);
                    }
                }
            }
            if let Some(t) = lab.cct_k {
                worst_cct = worst_cct.max((c.cct - t).abs());
            }
            if let Some(d) = lab.duv {
                worst_duv = worst_duv.max((c.duv - d).abs());
            }
            checked += 1;
        }
        println!(
            "worst: Ra {worst_ra:.2}, Ri {worst_ri:.2}, CCT {worst_cct:.1} K, Duv {worst_duv:.5}"
        );
        assert!(checked >= 6, "checked {checked}");
        // Measured (2026-11-05): Ra 0.22, Ri 0.87, CCT 13.6 K, Duv 0.0003.
        assert!(worst_ra < 0.4, "Ra {worst_ra}");
        assert!(worst_ri < 1.5, "Ri {worst_ri}");
        assert!(worst_cct < 25.0, "CCT {worst_cct}");
        assert!(worst_duv < 6e-4, "Duv {worst_duv}");
    }

    #[test]
    fn reference_sources_score_100() {
        for cct in [2856.0, 4000.0] {
            let c = calculate_cri(&spd_from_grid(&planck(cct))).unwrap();
            assert!((c.ra - 100.0).abs() < 0.05, "{cct}: Ra {}", c.ra);
            assert!(c.ri.iter().all(|r| (r - 100.0).abs() < 0.1), "{:?}", c.ri);
            assert!(c.within_duv_limit);
        }
        let d = calculate_cri(&spd_from_grid(&daylight(6500.0))).unwrap();
        assert!(d.daylight_reference);
        assert!((d.ra - 100.0).abs() < 0.2, "D65-ish Ra {}", d.ra);
    }
}
