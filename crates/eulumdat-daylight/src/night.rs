//! Natural light at night: the moon, twilight and the starlit sky.
//!
//! A full moon high in a clear sky puts about 0.2–0.3 lx on the ground:
//! as much as the minimum some emergency-lighting codes ask for on escape
//! routes (0.2 lx), and enough to find one's way. So night planning needs it
//! as a number, not only as a picture.
//!
//! - **Moon outside the atmosphere** ([`moon_extraterrestrial_lux`]): the
//!   lunar phase law of Krisciunas & Schaefer (1991, PASP 103, 1033), `I* =
//!   10^(−0.4 (3.84 + 0.026 |α| + 4·10⁻⁹ α⁴))` foot-candles at the mean
//!   distance (α = phase angle in degrees; 0.31 lx at α = 0), times (mean
//!   distance / distance)², with their opposition surge: × (1.35 − 0.05 |α|)
//!   within 7° of full (shadow hiding and coherent backscatter on the
//!   regolith make a full moon up to 35 % brighter than the smooth law).
//! - **Eclipses** ([`eclipse_factor`]): in the Earth's shadow the moon
//!   dims: from the penumbra's edge to half at the umbra's, to 10⁻⁴ in
//!   totality (umbra and penumbra radii after Meeus ch. 54, from the moon's
//!   parallax and the sun's semidiameter).
//! - **Through the atmosphere** ([`Moonlight::of`]): the same clear-sky model
//!   as the daylight ([`DaylightAvailability::clear_sky`]: beam attenuated
//!   over the air mass, diffuse sky light as a share of it), scaled from the
//!   sun's extraterrestrial illuminance to the moon's. The moon's light is
//!   reflected sunlight, so the atmosphere treats it alike.
//! - **Twilight** ([`twilight_lux`]): log-linear between commonly quoted clear-sky
//!   values: 400 lx at sunrise / sunset, 3.4 lx at the end of civil twilight
//!   (sun 6° below the horizon), the moonless night floor of starlight and
//!   airglow, 0.002 lx ([`STARLIGHT_LUX`]), from 18° below.
//!
//! All values are horizontal illuminance on open ground under a clear sky
//! unless named otherwise; clouds, buildings and trees are not modelled.

use crate::availability::DaylightAvailability;
use crate::constants::EXTRATERRESTRIAL_ILLUMINANCE_LUX;
use crate::location::{LocalDateTime, NamedLocation};
use crate::lunar::{MoonPosition, MEAN_DISTANCE_KM};
use crate::solar::SolarPosition;

/// Clear moonless night sky (starlight + airglow), horizontal (lx).
pub const STARLIGHT_LUX: f64 = 0.002;

/// Clear-sky horizontal illuminance at sunrise / sunset (lx).
pub const SUNSET_LUX: f64 = 400.0;

/// Clear-sky horizontal illuminance at the end of civil twilight (lx).
pub const CIVIL_TWILIGHT_END_LUX: f64 = 3.4;

const FOOT_CANDLE_LUX: f64 = 10.763_910_4;

/// The moon's illuminance at normal incidence outside the atmosphere (lx),
/// for a phase angle (Sun–Moon–Earth, degrees: 0 = full, 180 = new) and a
/// distance (km): phase law, opposition surge, distance, eclipse.
pub fn moon_extraterrestrial_lux(phase_angle_deg: f64, distance_km: f64) -> f64 {
    let a = phase_angle_deg.abs().min(180.0);
    let fc = 10f64.powf(-0.4 * (3.84 + 0.026 * a + 4e-9 * a.powi(4)));
    let surge = if a < 7.0 { 1.35 - 0.05 * a } else { 1.0 };
    let distance = if distance_km > 0.0 {
        distance_km
    } else {
        MEAN_DISTANCE_KM
    };
    let d = MEAN_DISTANCE_KM / distance;
    fc * FOOT_CANDLE_LUX * surge * d * d * eclipse_factor(a, distance)
}

/// How much of the moon's light is left in the Earth's shadow (1 = no
/// eclipse) for a phase angle (degrees; about the distance of the moon's
/// centre from the shadow's axis) and the moon's distance (km).
pub fn eclipse_factor(phase_angle_deg: f64, distance_km: f64) -> f64 {
    const EARTH_RADIUS_KM: f64 = 6378.14;
    const SUN_SEMIDIAMETER_DEG: f64 = 0.2666;
    const SUN_PARALLAX_DEG: f64 = 0.0024;
    let parallax = (EARTH_RADIUS_KM / distance_km.max(EARTH_RADIUS_KM * 2.0))
        .asin()
        .to_degrees();
    // shadow radii at the moon's distance (Meeus ch. 54, 2 % for the
    // atmosphere) and the moon's own semidiameter
    let umbra = 1.02 * (0.99834 * parallax - SUN_SEMIDIAMETER_DEG + SUN_PARALLAX_DEG);
    let penumbra = 1.02 * (0.99834 * parallax + SUN_SEMIDIAMETER_DEG + SUN_PARALLAX_DEG);
    let moon = 0.2725 * parallax;
    let g = phase_angle_deg.abs();
    if g >= penumbra + moon {
        1.0
    } else if g >= umbra + moon {
        // penumbral: down to half at the umbra's edge
        1.0 - 0.5 * (penumbra + moon - g) / (penumbra - umbra)
    } else if g > umbra - moon {
        // partial: the share of the disc outside the umbra, half lit
        let outside = (g - (umbra - moon)) / (2.0 * moon);
        (0.5 * outside).max(1e-4)
    } else {
        // totality: the red, dim eclipsed moon
        1e-4
    }
}

/// Moonlight on the ground under a clear sky (lx).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Moonlight {
    /// Outside the atmosphere, normal to the moon.
    pub extraterrestrial_lux: f64,
    /// On a surface facing the moon (beam only).
    pub normal_lux: f64,
    /// The beam on a horizontal surface.
    pub direct_horizontal_lux: f64,
    /// The moonlit sky on a horizontal surface.
    pub diffuse_horizontal_lux: f64,
    /// Both on a horizontal surface.
    pub global_horizontal_lux: f64,
}

impl Moonlight {
    /// Moonlight for the moon at `moon` and an atmosphere of `turbidity`
    /// (Linke, as for the daylight; 2.5 = clear); zero below the horizon.
    pub fn of(moon: &MoonPosition, turbidity: f64) -> Self {
        let outside =
            moon_extraterrestrial_lux(moon.phase_angle_rad.to_degrees(), moon.distance_km);
        let as_sun = SolarPosition {
            altitude_rad: moon.altitude_rad,
            azimuth_rad: moon.azimuth_rad,
            zenith_rad: std::f64::consts::FRAC_PI_2 - moon.altitude_rad,
        };
        let a = DaylightAvailability::clear_sky(&as_sun, turbidity);
        let k = outside / EXTRATERRESTRIAL_ILLUMINANCE_LUX;
        let direct = (a.ghi_lux - a.dhi_lux) * k;
        Moonlight {
            extraterrestrial_lux: outside,
            normal_lux: a.dni_lux * k,
            direct_horizontal_lux: direct,
            diffuse_horizontal_lux: a.dhi_lux * k,
            global_horizontal_lux: a.ghi_lux * k,
        }
    }
}

/// Clear-sky horizontal illuminance from the sun below (or just at) the
/// horizon, without the moon (lx): twilight down to the starlight floor.
pub fn twilight_lux(sun_altitude_deg: f64) -> f64 {
    let a = sun_altitude_deg;
    let lerp_log = |a0: f64, e0: f64, a1: f64, e1: f64| {
        let t = ((a - a0) / (a1 - a0)).clamp(0.0, 1.0);
        10f64.powf(e0.log10() + t * (e1.log10() - e0.log10()))
    };
    if a >= 0.0 {
        SUNSET_LUX
    } else if a >= -6.0 {
        lerp_log(0.0, SUNSET_LUX, -6.0, CIVIL_TWILIGHT_END_LUX)
    } else {
        lerp_log(-6.0, CIVIL_TWILIGHT_END_LUX, -18.0, STARLIGHT_LUX)
    }
}

/// Which source gives most of the natural light.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NaturalSource {
    Daylight,
    Twilight,
    Moon,
    Stars,
}

/// Natural horizontal illuminance outdoors under a clear sky (lx).
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NaturalLight {
    /// The sun and its sky (sun above the horizon).
    pub daylight_lux: f64,
    /// Twilight (sun up to 18° below the horizon; above the starlight floor).
    pub twilight_lux: f64,
    pub moon: Moonlight,
    pub starlight_lux: f64,
}

impl NaturalLight {
    /// The light of the sun at `sun` and the moon at `moon`.
    pub fn at(sun: &SolarPosition, moon: &MoonPosition, turbidity: f64) -> Self {
        let alt = sun.altitude_deg();
        // the clear-sky model has little sky light with a low sun: near the
        // horizon the twilight value is the better one
        let day = if sun.is_daytime() {
            DaylightAvailability::clear_sky(sun, turbidity).ghi_lux
        } else {
            0.0
        };
        let twilight = (twilight_lux(alt) - STARLIGHT_LUX).max(0.0);
        let (daylight_lux, twilight_lux) = if alt > 0.0 {
            (day.max(SUNSET_LUX), 0.0)
        } else {
            (0.0, twilight)
        };
        NaturalLight {
            daylight_lux,
            twilight_lux,
            moon: Moonlight::of(moon, turbidity),
            starlight_lux: STARLIGHT_LUX,
        }
    }

    /// Everything on a horizontal surface.
    pub fn total_lux(&self) -> f64 {
        self.daylight_lux + self.twilight_lux + self.moon.global_horizontal_lux + self.starlight_lux
    }

    /// The largest part.
    pub fn dominant(&self) -> NaturalSource {
        [
            (NaturalSource::Daylight, self.daylight_lux),
            (NaturalSource::Twilight, self.twilight_lux),
            (NaturalSource::Moon, self.moon.global_horizontal_lux),
            (NaturalSource::Stars, self.starlight_lux),
        ]
        .into_iter()
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map_or(NaturalSource::Stars, |(s, _)| s)
    }
}

/// One instant of a [`NightSummary`].
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NightSample {
    /// Hours after noon of the evening's date (0 = 12:00, 12 = midnight).
    pub hours: f64,
    pub light: NaturalLight,
    pub moon_altitude_deg: f64,
    pub sun_altitude_deg: f64,
}

/// The night from noon of a date to noon of the next day.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NightSummary {
    pub samples: Vec<NightSample>,
    /// Sunset and the next sunrise (hours after noon; `None` = polar day /
    /// night).
    pub sunset: Option<f64>,
    pub sunrise: Option<f64>,
    /// Moon rising and setting within the 24 h (hours after noon).
    pub moonrise: Option<f64>,
    pub moonset: Option<f64>,
    /// The moon at midnight.
    pub moon_at_midnight: MoonPosition,
    /// The most moonlight while the sun is down, and when.
    pub moon_max_lux: f64,
    pub moon_max_at: Option<f64>,
}

impl NightSummary {
    /// The night after `date` (its wall-clock time is ignored) at `loc`,
    /// sampled every `step_min` minutes.
    pub fn of(date: LocalDateTime, loc: &NamedLocation, turbidity: f64, step_min: f64) -> Self {
        let noon = LocalDateTime::new(date.year, date.month, date.day, 12.0);
        let step = (step_min / 60.0).clamp(1.0 / 60.0, 2.0);
        let n = (24.0 / step).round() as usize;
        let samples: Vec<NightSample> = (0..=n)
            .map(|i| {
                let h = i as f64 * step;
                let t = noon.plus_hours(h);
                let sun = t.solar_position(loc);
                let moon = t.moon_position(loc);
                NightSample {
                    hours: h,
                    light: NaturalLight::at(&sun, &moon, turbidity),
                    moon_altitude_deg: moon.altitude_deg(),
                    sun_altitude_deg: sun.altitude_deg(),
                }
            })
            .collect();
        // the first crossing of the horizon in each direction (linear
        // between samples)
        let crossing = |alt: &dyn Fn(&NightSample) -> f64, rising: bool| {
            samples.windows(2).find_map(|w| {
                let (a, b) = (alt(&w[0]), alt(&w[1]));
                let hit = if rising {
                    a <= 0.0 && b > 0.0
                } else {
                    a > 0.0 && b <= 0.0
                };
                hit.then(|| w[0].hours + (w[1].hours - w[0].hours) * a / (a - b))
            })
        };
        let sunset = crossing(&|s| s.sun_altitude_deg, false);
        let sunrise = crossing(&|s| s.sun_altitude_deg, true);
        let moonrise = crossing(&|s| s.moon_altitude_deg, true);
        let moonset = crossing(&|s| s.moon_altitude_deg, false);
        let (moon_max_lux, moon_max_at) = samples
            .iter()
            .filter(|s| s.sun_altitude_deg < 0.0)
            .map(|s| (s.light.moon.global_horizontal_lux, s.hours))
            .fold(
                (0.0, None),
                |acc, (e, h)| {
                    if e > acc.0 {
                        (e, Some(h))
                    } else {
                        acc
                    }
                },
            );
        NightSummary {
            moon_at_midnight: noon.plus_hours(12.0).moon_position(loc),
            samples,
            sunset,
            sunrise,
            moonrise,
            moonset,
            moon_max_lux,
            moon_max_at,
        }
    }

    /// Hours while the sun is down in which the moon alone gives at least
    /// `lux`.
    pub fn moon_hours_above(&self, lux: f64) -> f64 {
        self.dark_hours(|s| s.light.moon.global_horizontal_lux >= lux)
    }

    /// Hours while the sun is down in which all natural light together
    /// gives at least `lux`.
    pub fn natural_hours_above(&self, lux: f64) -> f64 {
        self.dark_hours(|s| s.light.total_lux() >= lux)
    }

    fn dark_hours(&self, f: impl Fn(&NightSample) -> bool) -> f64 {
        let step = self
            .samples
            .get(1)
            .map_or(0.0, |s| s.hours - self.samples[0].hours);
        self.samples
            .iter()
            .filter(|s| s.sun_altitude_deg < 0.0 && f(s))
            .count() as f64
            * step
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lunar::moon_position;

    #[test]
    fn full_moon_outside_the_atmosphere_with_its_opposition_surge() {
        // a typical full moon is a few degrees off exact opposition (else
        // it would be eclipsed): 3° → the smooth law × 1.2
        let e = moon_extraterrestrial_lux(3.0, MEAN_DISTANCE_KM);
        let smooth = 10f64.powf(-0.4 * (3.84 + 0.026 * 3.0 + 4e-9 * 81.0)) * FOOT_CANDLE_LUX;
        assert!((e / smooth - 1.2).abs() < 1e-9, "{}", e / smooth);
        assert!((0.34..0.37).contains(&e), "{e}");
        // the surge fades out by 7°
        let s7 = moon_extraterrestrial_lux(7.0, MEAN_DISTANCE_KM);
        let s7_smooth = 10f64.powf(-0.4 * (3.84 + 0.026 * 7.0 + 4e-9 * 2401.0)) * FOOT_CANDLE_LUX;
        assert!((s7 / s7_smooth - 1.0).abs() < 1e-9);
        // the steep phase law: a quarter moon is about a tenth of full
        let q = moon_extraterrestrial_lux(90.0, MEAN_DISTANCE_KM);
        assert!((0.07..0.11).contains(&(q / e)), "{}", q / e);
        // a close full moon (perigee) is brighter
        let near = moon_extraterrestrial_lux(3.0, 357_000.0);
        assert!((near / e - (384_400.0f64 / 357_000.0).powi(2)).abs() < 1e-12);
        // next to nothing near new moon (the law is fitted up to ~150°;
        // beyond, the moon is a thin crescent next to the sun)
        assert!(moon_extraterrestrial_lux(180.0, MEAN_DISTANCE_KM) < 1e-4);
    }

    #[test]
    fn eclipses_dim_the_moon() {
        let d = MEAN_DISTANCE_KM;
        assert_eq!(eclipse_factor(3.0, d), 1.0);
        assert_eq!(eclipse_factor(0.1, d), 1e-4, "totality");
        let partial = eclipse_factor(0.8, d);
        assert!(partial > 1e-4 && partial < 0.5, "{partial}");
        let penumbral = eclipse_factor(1.3, d);
        assert!((0.5..1.0).contains(&penumbral), "{penumbral}");
        // the total lunar eclipse of 2026-03-03 (greatest ~11:34 UTC)
        let m = moon_position(2026, 3, 3, 11.57, 0.0, 0.0);
        assert!(
            m.phase_angle_rad.to_degrees() < 0.45,
            "{}",
            m.phase_angle_rad.to_degrees()
        );
        let e = moon_extraterrestrial_lux(m.phase_angle_rad.to_degrees(), m.distance_km);
        assert!(e < 1e-3, "{e}");
        // the full moon of 2026-01-03 is no eclipse: its latitude keeps it
        // a few degrees off the shadow
        let jan = moon_position(2026, 1, 3, 10.05, 0.0, 0.0);
        let a = jan.phase_angle_rad.to_degrees();
        assert!((1.0..7.0).contains(&a), "{a}");
        assert_eq!(eclipse_factor(a, jan.distance_km), 1.0);
    }

    #[test]
    fn a_high_full_moon_lights_the_ground_to_0_25_to_0_35_lx() {
        let mut m = moon_position(2026, 1, 3, 10.0, 0.0, 0.0);
        m.altitude_rad = 70f64.to_radians();
        m.phase_angle_rad = 3f64.to_radians();
        m.distance_km = MEAN_DISTANCE_KM;
        let l = Moonlight::of(&m, 2.5);
        assert!((0.25..0.35).contains(&l.global_horizontal_lux), "{l:?}");
        assert!(l.normal_lux < l.extraterrestrial_lux);
        assert!(
            (l.direct_horizontal_lux + l.diffuse_horizontal_lux - l.global_horizontal_lux).abs()
                < 1e-12
        );
        // lower in the sky: longer path, flatter beam
        m.altitude_rad = 20f64.to_radians();
        let low = Moonlight::of(&m, 2.5);
        assert!(
            low.global_horizontal_lux < 0.4 * l.global_horizontal_lux,
            "{low:?}"
        );
        // below the horizon: nothing
        m.altitude_rad = -0.1;
        assert_eq!(Moonlight::of(&m, 2.5).global_horizontal_lux, 0.0);
    }

    #[test]
    fn twilight_falls_from_sunset_to_starlight() {
        assert_eq!(twilight_lux(0.0), SUNSET_LUX);
        assert!((twilight_lux(-6.0) - CIVIL_TWILIGHT_END_LUX).abs() < 1e-9);
        assert!((twilight_lux(-18.0) - STARLIGHT_LUX).abs() < 1e-12);
        assert!((twilight_lux(-40.0) - STARLIGHT_LUX).abs() < 1e-12);
        let mut last = f64::INFINITY;
        for a in 0..=18 {
            let e = twilight_lux(-(a as f64));
            assert!(e <= last);
            last = e;
        }
    }

    #[test]
    fn a_full_moon_night_in_berlin() {
        // 2026-01-03: full moon; Berlin in winter, the moon high at midnight
        let berlin = crate::location::location_by_name("Berlin").unwrap();
        let night = NightSummary::of(LocalDateTime::new(2026, 1, 2, 0.0), &berlin, 2.5, 10.0);
        assert!(night.moon_at_midnight.illuminated_fraction > 0.95);
        // the moon rises around sunset and sets around sunrise
        let (ss, sr) = (night.sunset.unwrap(), night.sunrise.unwrap());
        assert!(
            (3.5..5.0).contains(&ss) && (19.5..21.0).contains(&sr),
            "{ss} {sr}"
        );
        assert!(night.moonrise.is_some() && night.moonset.is_some());
        // up to ~0.3 lx from a close winter full moon high in the sky
        assert!(
            (0.2..0.38).contains(&night.moon_max_lux),
            "{}",
            night.moon_max_lux
        );
        // hours of moonlight above 0.1 lx: most of the long winter night
        let h = night.moon_hours_above(0.1);
        assert!(h > 8.0, "{h}");
        // at midnight the moon outshines the starlit sky a hundredfold
        let mid = night
            .samples
            .iter()
            .find(|s| (s.hours - 12.0).abs() < 1e-9)
            .unwrap();
        assert_eq!(mid.light.dominant(), NaturalSource::Moon);
        assert!(mid.light.moon.global_horizontal_lux > 50.0 * STARLIGHT_LUX);
    }

    #[test]
    fn a_new_moon_night_is_starlit() {
        let berlin = crate::location::location_by_name("Berlin").unwrap();
        let night = NightSummary::of(LocalDateTime::new(2026, 1, 18, 0.0), &berlin, 2.5, 10.0);
        assert!(night.moon_max_lux < 0.001, "{}", night.moon_max_lux);
        let mid = night
            .samples
            .iter()
            .find(|s| (s.hours - 12.0).abs() < 1e-9)
            .unwrap();
        assert_eq!(mid.light.dominant(), NaturalSource::Stars);
        assert!((mid.light.total_lux() - STARLIGHT_LUX).abs() < 1e-4);
        assert_eq!(night.moon_hours_above(0.05), 0.0);
    }
}
