//! Photometric / actinic quantities derived from an SPD and the [`crate::lut`]
//! weighting functions.
//!
//! All integrals are per unit radiant power in the SPD's own units; ratios
//! (S/P, melanopic DER) are unit-free, and PPFD-per-watt is absolute.

use crate::constants::{
    joules_per_micromole, KM_PHOTOPIC, KM_SCOTOPIC, MEL_PER_LUM_D65, BLUE_END_NM, BLUE_START_NM,
};
use crate::lut::{melanopic, par_weight, photopic, scotopic};
use crate::spd::Spd;

/// A bundle of spectrally-weighted quantities for one SPD.
///
/// Photopic/scotopic/melanopic are *radiant-power-weighted integrals*
/// (∝ lumens etc. once multiplied by efficacy); the useful outputs are the
/// unit-free ratios and per-watt values.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpectralMetrics {
    /// ∫SPD·V dλ (photopic).
    pub photopic_integral: f64,
    /// ∫SPD·V′ dλ (scotopic).
    pub scotopic_integral: f64,
    /// ∫SPD·s_mel dλ (melanopic).
    pub melanopic_integral: f64,
    /// Luminous efficacy of radiation, lm/W (683·∫SPD·V / ∫SPD).
    pub ler_lm_per_w: f64,
    /// Scotopic/photopic ratio (CIE-defined, using Km/Km′).
    pub sp_ratio: f64,
    /// Melanopic daylight (D65-referenced) efficacy ratio; D65 = 1.0.
    pub melanopic_der: f64,
    /// PPFD per radiant watt of the SPD (µmol·s⁻¹·W⁻¹), for horticulture.
    pub ppfd_per_watt: f64,
    /// Fraction of visible radiant power in the 400–500 nm blue band.
    pub blue_fraction: f64,
}

impl SpectralMetrics {
    /// Compute all metrics from an SPD.
    pub fn from_spd(spd: &Spd) -> Self {
        let radiant = spd.integral();
        let photopic_integral = spd.integral_weighted(photopic);
        let scotopic_integral = spd.integral_weighted(scotopic);
        let melanopic_integral = spd.integral_weighted(melanopic);

        let ler_lm_per_w = if radiant > 0.0 {
            KM_PHOTOPIC * photopic_integral / radiant
        } else {
            0.0
        };

        let sp_ratio = sp_ratio_from_integrals(photopic_integral, scotopic_integral);
        let melanopic_der = melanopic_der(spd);

        let ppfd_per_watt = if radiant > 0.0 {
            ppfd_integral(spd) / radiant
        } else {
            0.0
        };

        let blue = spd.integral_weighted(|w| {
            if (BLUE_START_NM..=BLUE_END_NM).contains(&w) {
                1.0
            } else {
                0.0
            }
        });
        let visible = spd.integral();
        let blue_fraction = if visible > 0.0 { blue / visible } else { 0.0 };

        Self {
            photopic_integral,
            scotopic_integral,
            melanopic_integral,
            ler_lm_per_w,
            sp_ratio,
            melanopic_der,
            ppfd_per_watt,
            blue_fraction,
        }
    }
}

/// Scotopic/photopic (S/P) ratio directly from an SPD.
///
/// `S/P = (Km′·∫SPD·V′) / (Km·∫SPD·V)` with Km=683, Km′=1700.
pub fn sp_ratio(spd: &Spd) -> f64 {
    let p = spd.integral_weighted(photopic);
    let s = spd.integral_weighted(scotopic);
    sp_ratio_from_integrals(p, s)
}

fn sp_ratio_from_integrals(photopic: f64, scotopic: f64) -> f64 {
    if photopic <= 0.0 {
        return 0.0;
    }
    (KM_SCOTOPIC * scotopic) / (KM_PHOTOPIC * photopic)
}

/// Melanopic daylight efficacy ratio (DER), normalised so D65 = 1.0.
///
/// DER = (melanopic/photopic of the source) / (melanopic/photopic of D65).
/// Multiply photopic lux by DER to get melanopic EDI lux (CIE S 026).
pub fn melanopic_der(spd: &Spd) -> f64 {
    let p = spd.integral_weighted(photopic);
    if p <= 0.0 {
        return 0.0;
    }
    let m = spd.integral_weighted(melanopic);
    (m / p) / MEL_PER_LUM_D65
}

/// PPFD integral: µmol·s⁻¹ per unit radiant power of the SPD, over 400–700 nm.
///
/// Each spectral watt at λ carries `1/joules_per_micromole(λ)` µmol·s⁻¹.
pub fn ppfd_integral(spd: &Spd) -> f64 {
    spd.integral_weighted(|w| {
        if par_weight(w) > 0.0 {
            1.0 / joules_per_micromole(w)
        } else {
            0.0
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::planck_spd;
    use approx::assert_relative_eq;

    #[test]
    fn warm_led_sp_below_cool_led() {
        // Warm blackbody (2700 K) has less blue → lower S/P than cool (6500 K).
        let warm = planck_spd(2700.0);
        let cool = planck_spd(6500.0);
        assert!(sp_ratio(&warm) < sp_ratio(&cool));
        // Typical envelope: warm ~1.2–1.5, cool ~2.0–2.5.
        assert!(sp_ratio(&warm) > 1.0 && sp_ratio(&warm) < 1.7);
        assert!(sp_ratio(&cool) > 1.8 && sp_ratio(&cool) < 2.7);
    }

    #[test]
    fn d65_melanopic_der_is_about_one() {
        // D65 ≈ 6500 K daylight; DER is defined so D65 ≈ 1.0.
        let d65 = crate::daylight::d_illuminant(6500.0);
        let der = melanopic_der(&d65);
        assert_relative_eq!(der, 1.0, epsilon = 0.06);
    }

    #[test]
    fn ler_of_555_monochromatic_is_683() {
        // A narrow spike at 555 nm has LER ≈ 683 lm/W.
        let spd = Spd::new(&[550.0, 555.0, 560.0], &[0.0, 1.0, 0.0]);
        let m = SpectralMetrics::from_spd(&spd);
        assert!(
            (m.ler_lm_per_w - 683.0).abs() < 15.0,
            "555 nm LER should be ~683, got {}",
            m.ler_lm_per_w
        );
    }

    #[test]
    fn ppfd_positive_for_daylight() {
        let d = crate::daylight::d_illuminant(5000.0);
        assert!(ppfd_integral(&d) > 0.0);
    }
}
