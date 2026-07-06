//! Daylight availability — direct/diffuse/global illuminance (lux) at a place
//! and time, for clear and overcast conditions.
//!
//! Lux-native: a simple clear-sky illuminance model keyed off solar altitude
//! and turbidity, so callers never have to touch W/m² or luminous efficacy.
//! (If you start from irradiance, convert with [`crate::constants::efficacy`].)

use crate::constants::EXTRATERRESTRIAL_ILLUMINANCE_LUX;
use crate::solar::{air_mass, SolarPosition};

/// Direct, diffuse and global horizontal daylight illuminance (lux).
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DaylightAvailability {
    /// Direct normal illuminance — the beam on a surface ⟂ to the sun, lux.
    pub dni_lux: f64,
    /// Diffuse horizontal illuminance — the sky dome on a horizontal plane, lux.
    pub dhi_lux: f64,
    /// Global horizontal illuminance = dni·sin(altitude) + dhi, lux.
    pub ghi_lux: f64,
}

impl DaylightAvailability {
    /// Clear-sky daylight from sun position and atmospheric turbidity.
    ///
    /// Beam: extraterrestrial illuminance attenuated by a Linke-turbidity
    /// Beer–Lambert term over the relative air mass. Diffuse: a small fraction
    /// of the horizontal beam that grows with turbidity (hazier ⇒ more diffuse).
    /// Below the horizon ⇒ all zero.
    pub fn clear_sky(sun: &SolarPosition, turbidity: f64) -> Self {
        if !sun.is_daytime() {
            return Self::dark();
        }
        let t = turbidity.max(1.0);
        let m = air_mass(sun.zenith_rad).max(1.0);

        // Broadband Beer–Lambert beam transmittance. The exponent grows with
        // air mass and turbidity; coefficient tuned so clear noon DNI lands
        // near ~100 klx and high-turbidity skies attenuate strongly.
        let optical_depth = 0.08 + 0.035 * t;
        let dni_lux = EXTRATERRESTRIAL_ILLUMINANCE_LUX * (-optical_depth * m).exp();

        let sin_alt = sun.altitude_rad.sin().max(0.0);
        let beam_horizontal = dni_lux * sin_alt;

        // Diffuse fraction of the horizontal beam — larger for hazier skies.
        let diffuse_fraction = (0.10 + 0.04 * (t - 1.0)).clamp(0.10, 0.6);
        let dhi_lux = beam_horizontal * diffuse_fraction;

        Self {
            dni_lux,
            dhi_lux,
            ghi_lux: beam_horizontal + dhi_lux,
        }
    }

    /// CIE-overcast daylight: no beam (DNI ≈ 0), all light diffuse. The global
    /// horizontal illuminance follows the standard overcast altitude relation
    /// `GHI ≈ G·(1 + 2·sin α)/3` scaled so a high overcast sun ≈ 20 klx.
    pub fn overcast(sun: &SolarPosition) -> Self {
        if !sun.is_daytime() {
            return Self::dark();
        }
        let sin_alt = sun.altitude_rad.sin().max(0.0);
        // Zenith overcast luminance ~ scales with sun altitude; pick a constant
        // so overhead overcast ≈ 20 klx (mid of the 10–25 klx literature band).
        let ghi = 20_000.0 * (1.0 + 2.0 * sin_alt) / 3.0;
        Self {
            dni_lux: 0.0,
            dhi_lux: ghi,
            ghi_lux: ghi,
        }
    }

    /// All-zero (night / sun below horizon).
    pub fn dark() -> Self {
        Self {
            dni_lux: 0.0,
            dhi_lux: 0.0,
            ghi_lux: 0.0,
        }
    }
}

/// Correlated colour temperature (K) of clear-sky daylight as a function of sun
/// altitude and atmospheric turbidity — a physically-grounded appearance model
/// on the CIE daylight locus.
///
/// The observed daylight CCT follows a well-known pattern:
/// - **Near the horizon** (rising/setting sun): warm, ~4000–5500 K — the beam
///   travels a long air-mass path, scattering out blue (the "golden hour").
/// - **Overcast / mid sun**: ~6500 K, close to D65 (the standard reference).
/// - **High clear sun**: cooler, ~6500–8000 K global; the *clear zenith patch*
///   alone can reach 10 000–25 000 K, but the mixed global daylight a surface
///   receives sits lower.
/// - **Higher turbidity** (haze) warms the light (more forward scatter, less
///   blue reaching the ground).
///
/// This returns the CCT of the **global** daylight (beam + sky mix) a horizontal
/// surface sees, clamped to the sensible 4000–12000 K daylight range. It is an
/// appearance model for visualisation and reporting, not a spectroradiometric
/// measurement.
pub fn daylight_cct(altitude_deg: f64, turbidity: f64) -> f64 {
    if altitude_deg <= 0.0 {
        // Sun at/below horizon: deep twilight afterglow is warm.
        return 4500.0;
    }
    let alt = altitude_deg.clamp(0.0, 90.0);
    let t = turbidity.clamp(1.0, 10.0);

    // Base rises with altitude: horizon ~5000 K → high sun ~6800 K.
    // A saturating curve (1 − e^{−alt/k}) captures the fast warm→neutral swing
    // low in the sky and the gentle approach to a clear-sky plateau up high.
    let base = 5000.0 + 2200.0 * (1.0 - (-alt / 22.0).exp());

    // A clear (low-turbidity) high sky adds blue; hazy skies stay warm.
    // Blue boost scales with altitude and with clarity (low turbidity).
    let clarity = ((5.0 - t) / 4.0).clamp(0.0, 1.0); // 1 at T=1 (clear) → 0 at T≥5
    let blue_boost = 1800.0 * clarity * (alt / 90.0);

    (base + blue_boost).clamp(4000.0, 12000.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solar::solar_position;

    /// Clear noon summer sun → bright global horizontal (~80–120 klx band).
    #[test]
    fn clear_noon_is_bright() {
        let sun = solar_position(2026, 6, 21, 12.0, 40.0, 0.0);
        let a = DaylightAvailability::clear_sky(&sun, 2.5);
        assert!(
            (60_000.0..=130_000.0).contains(&a.ghi_lux),
            "clear noon GHI {} lx out of band",
            a.ghi_lux
        );
        // DNI on the order of ~100 klx.
        assert!(
            (60_000.0..=133_800.0).contains(&a.dni_lux),
            "clear noon DNI {} lx out of band",
            a.dni_lux
        );
    }

    /// Overcast is far dimmer than clear, in the ~10–25 klx band at high sun.
    #[test]
    fn overcast_band() {
        let sun = solar_position(2026, 6, 21, 12.0, 40.0, 0.0);
        let a = DaylightAvailability::overcast(&sun);
        assert_eq!(a.dni_lux, 0.0);
        assert!(
            (8_000.0..=25_000.0).contains(&a.ghi_lux),
            "overcast GHI {} lx out of band",
            a.ghi_lux
        );
    }

    #[test]
    fn night_is_dark() {
        let sun = solar_position(2026, 6, 21, 0.0, 40.0, 0.0);
        let a = DaylightAvailability::clear_sky(&sun, 2.5);
        assert_eq!(a.ghi_lux, 0.0);
    }

    /// Higher turbidity reduces the direct beam.
    #[test]
    fn turbidity_attenuates_beam() {
        let sun = solar_position(2026, 6, 21, 12.0, 40.0, 0.0);
        let clear = DaylightAvailability::clear_sky(&sun, 2.0);
        let hazy = DaylightAvailability::clear_sky(&sun, 6.0);
        assert!(hazy.dni_lux < clear.dni_lux, "haze should cut the beam");
    }

    /// Daylight CCT is warm near the horizon, neutral-to-cool higher up, and
    /// stays inside the physical daylight band (no more 10 000 K nonsense at
    /// modest altitudes).
    #[test]
    fn daylight_cct_follows_the_locus() {
        let horizon = daylight_cct(2.0, 2.5); // sun just up → warm
        let mid = daylight_cct(30.0, 2.5);
        let high = daylight_cct(60.0, 2.5);

        assert!(
            (4000.0..=5800.0).contains(&horizon),
            "low sun should be warm (~4000–5800 K), got {horizon:.0}"
        );
        assert!(mid > horizon, "CCT rises off the horizon: {mid:.0} > {horizon:.0}");
        assert!(high >= mid, "CCT keeps rising with altitude");
        assert!(
            (6000.0..=9000.0).contains(&high),
            "clear high sun should be ~D65–8500 K, not 10000+, got {high:.0}"
        );
        // Everything stays in the daylight band.
        for alt in [1.0, 15.0, 45.0, 89.0] {
            let c = daylight_cct(alt, 2.5);
            assert!((4000.0..=12000.0).contains(&c), "alt {alt}: CCT {c:.0} out of band");
        }
    }

    /// Haze (high turbidity) warms the daylight — clear skies are bluer.
    #[test]
    fn haze_warms_the_daylight() {
        let clear = daylight_cct(50.0, 2.0);
        let hazy = daylight_cct(50.0, 8.0);
        assert!(
            clear > hazy,
            "clear high sky {clear:.0} K should be cooler/bluer than hazy {hazy:.0} K"
        );
    }

    /// Below the horizon returns a warm twilight value, never NaN or 0.
    #[test]
    fn twilight_cct_is_warm_and_finite() {
        let c = daylight_cct(-3.0, 2.5);
        assert!(c.is_finite() && (4000.0..=5000.0).contains(&c));
    }
}
