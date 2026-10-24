//! Ultraviolet action spectra: weighting functions that turn a UV spectral
//! power distribution into an *effective* irradiance.
//!
//! The built-in table is the actinic UV hazard function S(λ) of
//! ICNIRP (2004) / ACGIH, 200–400 nm, normalised to 1 at 270 nm. It is the
//! basis of the UV exposure limit of 30 J/m² (3 mJ/cm²) effective per 8 h
//! working day, and is therefore the right weighting for a safety check of a
//! UV-C source. It is *not* a germicidal (disinfection) action spectrum;
//! those differ per organism and are supplied by the caller through
//! [`ActionSpectrum::from_table`] so a project can drop in its own data.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::spd::Spd;

/// A tabulated relative action spectrum, linearly interpolated, zero
/// outside the table.
#[derive(Clone, Debug, PartialEq)]
pub struct ActionSpectrum {
    pub name: &'static str,
    /// `(wavelength nm, relative effectiveness)`, ascending.
    pub table: Vec<(f64, f64)>,
}

/// ICNIRP/ACGIH actinic UV hazard S(λ), 1.0 at 270 nm.
pub const ACTINIC_UV_HAZARD: &[(f64, f64)] = &[
    (200.0, 0.030),
    (205.0, 0.051),
    (210.0, 0.075),
    (215.0, 0.095),
    (220.0, 0.120),
    (225.0, 0.150),
    (230.0, 0.190),
    (235.0, 0.240),
    (240.0, 0.300),
    (245.0, 0.360),
    (250.0, 0.430),
    (254.0, 0.500),
    (255.0, 0.520),
    (260.0, 0.650),
    (265.0, 0.810),
    (270.0, 1.000),
    (275.0, 0.960),
    (280.0, 0.880),
    (285.0, 0.770),
    (290.0, 0.640),
    (295.0, 0.540),
    (297.0, 0.460),
    (300.0, 0.300),
    (303.0, 0.120),
    (305.0, 0.060),
    (308.0, 0.026),
    (310.0, 0.015),
    (313.0, 0.006),
    (315.0, 0.003),
    (316.0, 0.0024),
    (317.0, 0.0020),
    (318.0, 0.0016),
    (319.0, 0.0012),
    (320.0, 0.0010),
    (325.0, 0.00050),
    (330.0, 0.00041),
    (335.0, 0.00034),
    (340.0, 0.00028),
    (345.0, 0.00024),
    (350.0, 0.00020),
    (355.0, 0.00016),
    (360.0, 0.00013),
    (365.0, 0.00011),
    (370.0, 0.000093),
    (375.0, 0.000077),
    (380.0, 0.000064),
    (385.0, 0.000053),
    (390.0, 0.000044),
    (395.0, 0.000036),
    (400.0, 0.000030),
];

/// ICNIRP effective exposure limit for the actinic hazard, J/m² per 8 h.
pub const ACTINIC_LIMIT_J_PER_M2: f64 = 30.0;

impl ActionSpectrum {
    pub fn actinic_uv_hazard() -> Self {
        Self {
            name: "ICNIRP actinic UV hazard S(λ)",
            table: ACTINIC_UV_HAZARD.to_vec(),
        }
    }

    /// Any user-supplied table (e.g. a germicidal curve for a specific
    /// organism). Rows are sorted by wavelength.
    pub fn from_table(name: &'static str, mut table: Vec<(f64, f64)>) -> Self {
        table.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        Self { name, table }
    }

    /// Relative effectiveness at `wl_nm` (0 outside the table).
    pub fn weight(&self, wl_nm: f64) -> f64 {
        let t = &self.table;
        if t.is_empty() || wl_nm < t[0].0 || wl_nm > t[t.len() - 1].0 {
            return 0.0;
        }
        let i = t.partition_point(|p| p.0 < wl_nm);
        if i == 0 {
            return t[0].1;
        }
        let (w0, v0) = t[i - 1];
        let (w1, v1) = t[i];
        if w1 <= w0 {
            return v1;
        }
        v0 + (v1 - v0) * (wl_nm - w0) / (w1 - w0)
    }

    /// Effective fraction of a source spectrum: ∫S(λ)·P(λ)dλ / ∫P(λ)dλ.
    /// Multiply a radiometric irradiance by this to get the effective
    /// irradiance for this action spectrum.
    pub fn effective_fraction(&self, spd: &Spd) -> f64 {
        let total = spd.integral();
        if total <= 0.0 {
            return 0.0;
        }
        spd.integral_weighted(|w| self.weight(w)) / total
    }
}

/// Seconds until `limit_j_per_m2` is reached at a constant effective
/// irradiance (W/m²); `None` when the irradiance is zero.
pub fn seconds_to_limit(effective_w_per_m2: f64, limit_j_per_m2: f64) -> Option<f64> {
    (effective_w_per_m2 > 0.0).then(|| limit_j_per_m2 / effective_w_per_m2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_is_sorted_and_peaks_at_270() {
        let s = ActionSpectrum::actinic_uv_hazard();
        assert!(s.table.windows(2).all(|w| w[0].0 < w[1].0));
        assert_eq!(s.weight(270.0), 1.0);
        assert!((s.weight(254.0) - 0.5).abs() < 1e-9);
        assert!((s.weight(267.5) - 0.905).abs() < 1e-9);
        assert_eq!(s.weight(199.0), 0.0);
        assert_eq!(s.weight(401.0), 0.0);
    }

    #[test]
    fn effective_fraction_of_narrow_lines() {
        let s = ActionSpectrum::actinic_uv_hazard();
        let line270 = Spd::new(&[269.0, 270.0, 271.0], &[0.0, 1.0, 0.0]);
        assert!((s.effective_fraction(&line270) - 1.0).abs() < 0.05);
        let line254 = Spd::new(&[253.0, 254.0, 255.0], &[0.0, 1.0, 0.0]);
        assert!((s.effective_fraction(&line254) - 0.5).abs() < 0.03);
        let visible = Spd::new(&[500.0, 550.0, 600.0], &[1.0, 1.0, 1.0]);
        assert_eq!(s.effective_fraction(&visible), 0.0);
        assert_eq!(seconds_to_limit(0.0, 30.0), None);
        assert_eq!(seconds_to_limit(0.5, 30.0), Some(60.0));
    }
}
