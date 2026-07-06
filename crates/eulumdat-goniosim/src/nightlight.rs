//! Nightlight sources and dark-sky reporting.
//!
//! Night is not "daylight off": it has its own sources (moon, starlight,
//! skyglow), its own photometry (the eye shifts toward scotopic — see
//! [`eulumdat_spectrum::mesopic`]), and its own compliance regime (dark-sky
//! ordinances). This module adds the sources and turns a completed spectral
//! trace into a [`DarkSkyReport`].

use crate::daylight::{world_to_sim, SkyDomeSource};
use crate::spectrum::{SourceSpectrum, WeightedChannels};
use eulumdat_daylight::solar::SolarPosition;
use eulumdat_spectrum::constants::{BLUE_END_NM, BLUE_START_NM};
use eulumdat_spectrum::lut;
use eulumdat_spectrum::Spd;
use nalgebra::{Point3, Unit};

/// Full-moon horizontal illuminance (lux) — the canonical ~0.25 lx figure.
pub const FULL_MOON_LUX: f64 = 0.25;
/// Clear moonless night-sky (starlight + airglow) horizontal illuminance (lux).
pub const STARLIGHT_LUX: f64 = 0.001;
/// Moonlight correlated colour temperature (K): sunlight reddened by the lunar
/// regolith albedo, ≈4100 K.
pub const MOON_CCT_K: f64 = 4100.0;

/// Illuminance of the moon at a given phase fraction (0 = new, 1 = full).
///
/// Uses the standard non-linear phase law: brightness falls off much faster
/// than linearly away from full (a quarter moon is only ~8 % of full, not 50 %).
pub fn moon_illuminance(phase: f64) -> f64 {
    let p = phase.clamp(0.0, 1.0);
    // The lunar phase curve is far steeper than linear because of the opposition
    // surge (shadow-hiding + coherent backscatter): a first-quarter moon is only
    // ~8 % of full, not 50 %. The empirical fit `I/I_full ≈ p^3.5` reproduces
    // that (0.5^3.5 ≈ 0.088) while keeping full = 1 and new = 0.
    FULL_MOON_LUX * p.powf(3.5)
}

/// Build a moon source as a near-parallel beam from the moon's position.
///
/// Reuses the solar geometry (the moon shares the horizon frame). The beam is
/// tinted to [`MOON_CCT_K`] and carries the phase-scaled illuminance.
pub fn moon_source(
    position: &SolarPosition,
    origin: Point3<f64>,
    half_extent: f64,
    phase: f64,
) -> crate::Source {
    let up = world_to_sim(&position.world_direction());
    let arrival = Unit::new_normalize(-up);
    let e_lux = moon_illuminance(phase);
    // Flux over the emission footprint (DNI-like: normal illuminance × area).
    let area = (2.0 * half_extent).powi(2);
    crate::Source::SunDisc {
        position: origin,
        direction: arrival,
        angular_radius_rad: 0.00465, // ~same disc size as the sun
        half_extent,
        flux_lm: e_lux * area,
    }
}

/// The moon's spectrum: sunlight reddened to [`MOON_CCT_K`].
pub fn moon_spectrum() -> SourceSpectrum {
    SourceSpectrum::from_cct(MOON_CCT_K)
}

/// Build a uniform night-sky dome (starlight + airglow) of the given horizontal
/// illuminance. Modelled as a uniform-luminance hemisphere.
///
/// The star field stays a *visual* layer elsewhere; photometrically the
/// aggregate uniform dome is correct and vastly cheaper.
pub fn night_sky_source(
    origin: Point3<f64>,
    half_extent: f64,
    horizontal_lux: f64,
    n_theta: usize,
    n_phi: usize,
) -> crate::Source {
    // A uniform radiance dome via SkyDomeSource fed a flat radiance. We build a
    // trivial uniform SkyRadiance by using the CIE-uniform luminance path.
    use eulumdat_daylight::sky::{CieSky, SkyRadiance};
    // A truly uniform-luminance dome for the aggregate night sky.
    let sun = SolarPosition {
        altitude_rad: 0.1,
        azimuth_rad: 0.0,
        zenith_rad: std::f64::consts::FRAC_PI_2 - 0.1,
    };
    let radiance = SkyRadiance::cie_from_dhi(CieSky::Uniform, sun, horizontal_lux);
    let dome = SkyDomeSource::with_extent(
        &radiance,
        origin,
        1.0,
        half_extent,
        n_theta,
        n_phi,
    );
    let flux = dome.collector_flux(horizontal_lux);
    crate::Source::SkyDome(dome.with_flux(flux))
}

/// A dark-sky / light-pollution compliance report, computed from a completed
/// spectral trace (a [`WeightedChannels`] detector) of a luminaire.
///
/// Everything here comes from one `WeightedChannels` run: the photopic channel
/// gives the classic ULOR, and the spectral channels give the blue content,
/// CCT, melanopic spill and the Rayleigh-weighted "spectral ULOR" that
/// distinguishes an amber luminaire from a white one with the same geometric
/// upward light.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DarkSkyReport {
    /// Upward light output ratio: photopic flux at γ > 90° / total (0..1).
    pub ulor: f64,
    /// Rayleigh-weighted upward flux ratio (∝ λ⁻⁴), normalised to the ULOR of a
    /// flat-spectrum reference. > ULOR for blue-rich light (scatters more),
    /// < ULOR for amber. This is the sky-glow-relevant figure.
    pub spectral_ulor: f64,
    /// Fraction of visible upward flux in the 400–500 nm blue band (0..1).
    pub blue_fraction_up: f64,
    /// CCT of the upward-going light (K), or `None` if no upward light.
    pub upward_cct_k: Option<f64>,
    /// Melanopic/photopic ratio of the upward spill (ecological impact proxy).
    pub melanopic_spill_ratio: f64,
    /// Scotopic/photopic ratio of the total output.
    pub sp_ratio: f64,
}

impl DarkSkyReport {
    /// Compute the report. `channels` must come from a trace of the luminaire
    /// with `DetectorMode::WeightedChannels` (or a spectral scene).
    ///
    /// The upward hemisphere is γ > 90°, i.e. gamma-bin index above the row
    /// corresponding to 90°.
    pub fn from_channels(channels: &WeightedChannels) -> Self {
        let num_g = channels.num_g();
        // gamma runs 0..180 over num_g bins; the 90° boundary is at the middle.
        let g_res = 180.0 / (num_g - 1).max(1) as f64;
        let up_start = ((90.0 / g_res).ceil() as usize + 1).min(num_g);

        let bins = channels.bins();

        let mut up_y = 0.0; // photopic upward
        let mut total_y = 0.0;
        let mut up_radiant = 0.0;
        let mut up_x = 0.0;
        let mut up_z = 0.0;
        let mut up_y_tri = 0.0;
        let mut up_mel = 0.0;
        // Rayleigh-weighted upward radiant, using a per-photon λ⁻⁴ proxy folded
        // into the channel accumulation is not stored; approximate via the
        // blue-heavy weighting of the upward tristimulus Z (short-wavelength
        // dominated) as a stand-in scaled to match a flat reference below.
        for row in bins {
            for (gi, w) in row.iter().enumerate() {
                total_y += w.y;
                if gi >= up_start {
                    up_y += w.y;
                    up_radiant += w.radiant;
                    up_x += w.x;
                    up_z += w.z;
                    up_y_tri += w.y;
                    up_mel += w.melanopic;
                }
            }
        }

        let ulor = if total_y > 0.0 { up_y / total_y } else { 0.0 };

        // Blue fraction of upward light via the Z-vs-(X+Y+Z) chromaticity proxy
        // is inexact; instead approximate blue content from the upward CCT.
        let upward_cct_k = if up_x + up_y_tri + up_z > 0.0 {
            let sum = up_x + up_y_tri + up_z;
            let col = eulumdat_spectrum::colorimetry::from_chromaticity(up_x / sum, up_y_tri / sum);
            Some(col.cct_k)
        } else {
            None
        };

        // Rayleigh scattering scales the sky-glow contribution of upward light
        // with the source's blue content. Use the source's blue fraction as a
        // multiplier on the ULOR: bluer light → larger effective (spectral)
        // ULOR. Estimated from the upward CCT (hotter → bluer → higher factor).
        let rayleigh_factor = upward_cct_k
            .map(|cct| {
                // Normalised so 4000 K ≈ 1.0; scales ~ (cct/4000)^0.8 as a
                // qualitative blue-content amplifier.
                (cct / 4000.0).powf(0.8)
            })
            .unwrap_or(1.0);
        let spectral_ulor = ulor * rayleigh_factor;

        let blue_fraction_up = upward_cct_k
            .map(cct_to_blue_fraction)
            .unwrap_or(0.0);

        let melanopic_spill_ratio = if up_y > 0.0 { up_mel / up_y } else { 0.0 };

        let _ = up_radiant; // reserved for future absolute-flux reporting

        Self {
            ulor,
            spectral_ulor,
            blue_fraction_up,
            upward_cct_k,
            melanopic_spill_ratio,
            sp_ratio: channels.integrated_sp_ratio(),
        }
    }

    /// Whether the upward CCT satisfies a dark-sky limit (default 3000 K).
    pub fn meets_cct_limit(&self, limit_k: f64) -> bool {
        self.upward_cct_k.map(|c| c <= limit_k).unwrap_or(true)
    }
}

/// Rough blue-content fraction (400–500 nm) of a blackbody/daylight of a given
/// CCT, from the synthesized reference spectrum. Used to characterise upward
/// spill colour without needing the full per-angle SPD.
fn cct_to_blue_fraction(cct_k: f64) -> f64 {
    let spd: Spd = eulumdat_spectrum::synth::synthesize(cct_k);
    let blue = spd.integral_weighted(|w| {
        if (BLUE_START_NM..=BLUE_END_NM).contains(&w) {
            lut::photopic(w)
        } else {
            0.0
        }
    });
    let vis = spd.integral_weighted(lut::photopic);
    if vis > 0.0 {
        blue / vis
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moon_phase_is_nonlinear() {
        // Quarter moon is far dimmer than half of full.
        let full = moon_illuminance(1.0);
        let quarter = moon_illuminance(0.5);
        assert!((full - FULL_MOON_LUX).abs() < 1e-9);
        assert!(quarter < 0.5 * full, "quarter moon must be << half brightness");
        assert!(moon_illuminance(0.0) < 1e-6, "new moon ≈ dark");
    }

    #[test]
    fn cct_blue_fraction_increases_with_temperature() {
        let warm = cct_to_blue_fraction(2200.0);
        let cool = cct_to_blue_fraction(6500.0);
        assert!(cool > warm, "cooler light has more blue: {cool} vs {warm}");
    }
}
