//! Synthetic spectra: blackbody / Planckian and a CCT(+CRI) LED model.
//!
//! Used to seed sources when only a colour temperature is known (the common
//! case for an LDT that carries `color_appearance = "3000K"` but no SPD), and
//! as ground truth for the spectral round-trip test.

use crate::daylight::d_illuminant;
use crate::spd::Spd;

/// Planckian (blackbody) SPD at temperature `t` (K), 380–780 nm @ 5 nm,
/// normalised to peak 1.0. Relative spectral shape only.
pub fn planck_spd(t: f64) -> Spd {
    let c2 = 1.438_776_9e-2; // m·K
    let wl: Vec<f64> = (380..=780).step_by(5).map(|w| w as f64).collect();
    let mut vals: Vec<f64> = wl
        .iter()
        .map(|&w| {
            let lm = w * 1e-9;
            1.0 / (lm.powi(5) * ((c2 / (lm * t)).exp() - 1.0))
        })
        .collect();
    let peak = vals.iter().cloned().fold(0.0_f64, f64::max);
    if peak > 0.0 {
        for v in &mut vals {
            *v /= peak;
        }
    }
    Spd::new(&wl, &vals)
}

/// Synthesize an illuminant SPD from a target CCT.
///
/// Below 5000 K uses a Planckian blackbody (incandescent/warm-white behaviour);
/// at or above 5000 K uses the CIE D-series daylight reconstruction. This
/// mirrors how the CIE selects reference illuminants for CRI/TM-30.
pub fn synthesize(cct_k: f64) -> Spd {
    if cct_k < 5000.0 {
        planck_spd(cct_k)
    } else {
        d_illuminant(cct_k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colorimetry::analyze;

    #[test]
    fn synthesize_3000k_round_trips() {
        let spd = synthesize(3000.0);
        let c = analyze(&spd);
        assert!(
            (c.cct_k - 3000.0).abs() < 15.0,
            "synthesized 3000 K should analyze back to ~3000 K, got {}",
            c.cct_k
        );
    }

    #[test]
    fn synthesize_6500k_round_trips() {
        let spd = synthesize(6500.0);
        let c = analyze(&spd);
        assert!(
            (c.cct_k - 6500.0).abs() < 40.0,
            "synthesized 6500 K should analyze back to ~6500 K, got {}",
            c.cct_k
        );
    }

    #[test]
    fn planck_peak_shifts_with_temperature() {
        // Wien's law: hotter → peak at shorter wavelength. Compare blue/red.
        let warm = planck_spd(2700.0);
        let cool = planck_spd(9000.0);
        let blue_over_red = |s: &Spd| s.value_at(450.0) / s.value_at(650.0);
        assert!(blue_over_red(&cool) > blue_over_red(&warm));
    }
}
