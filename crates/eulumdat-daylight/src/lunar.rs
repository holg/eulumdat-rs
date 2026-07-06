//! Lunar position and phase — the moon is the dominant nighttime light source,
//! and its brightness swings ~250× from new moon (starlight only) to full moon
//! (~0.25 lx), so a night simulation *needs* it.
//!
//! Uses a compact low-precision model (Meeus, *Astronomical Algorithms*, ch. 47
//! truncated series) for the moon's ecliptic longitude/latitude, then the same
//! equatorial → horizontal conversion the solar model uses. Accuracy is a few
//! arc-minutes in position and well under 1 % in illuminated fraction — far
//! better than any photometric night model needs.

use crate::solar::solar_position;
use std::f64::consts::PI;

const DEG: f64 = PI / 180.0;

/// The moon's position and illumination at an instant and place.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MoonPosition {
    /// Altitude above the horizon (radians). Negative ⇒ below the horizon.
    pub altitude_rad: f64,
    /// Azimuth from North, clockwise toward East (radians).
    pub azimuth_rad: f64,
    /// Illuminated fraction of the disc, 0 (new) … 1 (full).
    pub illuminated_fraction: f64,
    /// Phase angle Sun–Moon–Earth (radians): 0 = full, π = new.
    pub phase_angle_rad: f64,
    /// Whether the moon is waxing (true) or waning (false).
    pub waxing: bool,
}

impl MoonPosition {
    /// Altitude above the horizon in degrees.
    pub fn altitude_deg(&self) -> f64 {
        self.altitude_rad / DEG
    }
    /// Azimuth from North clockwise in degrees.
    pub fn azimuth_deg(&self) -> f64 {
        self.azimuth_rad / DEG
    }
    /// True when the moon is above the horizon (contributing light).
    pub fn is_up(&self) -> bool {
        self.altitude_rad > 0.0
    }
    /// A phase name for the current illuminated fraction + waxing state.
    pub fn phase_name(&self) -> MoonPhase {
        let f = self.illuminated_fraction;
        if f < 0.04 {
            MoonPhase::New
        } else if f > 0.96 {
            MoonPhase::Full
        } else if (f - 0.5).abs() < 0.06 {
            if self.waxing {
                MoonPhase::FirstQuarter
            } else {
                MoonPhase::LastQuarter
            }
        } else if f < 0.5 {
            if self.waxing {
                MoonPhase::WaxingCrescent
            } else {
                MoonPhase::WaningCrescent
            }
        } else if self.waxing {
            MoonPhase::WaxingGibbous
        } else {
            MoonPhase::WaningGibbous
        }
    }
}

/// The eight canonical moon phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MoonPhase {
    New,
    WaxingCrescent,
    FirstQuarter,
    WaxingGibbous,
    Full,
    WaningGibbous,
    LastQuarter,
    WaningCrescent,
}

impl MoonPhase {
    /// A representative emoji for the phase (Northern-hemisphere orientation).
    pub fn emoji(&self) -> &'static str {
        match self {
            MoonPhase::New => "🌑",
            MoonPhase::WaxingCrescent => "🌒",
            MoonPhase::FirstQuarter => "🌓",
            MoonPhase::WaxingGibbous => "🌔",
            MoonPhase::Full => "🌕",
            MoonPhase::WaningGibbous => "🌖",
            MoonPhase::LastQuarter => "🌗",
            MoonPhase::WaningCrescent => "🌘",
        }
    }
}

/// Julian Day from a UTC calendar instant (same formulation as the solar model).
fn julian_day(year: i32, month: u32, day: u32, utc_hours: f64) -> f64 {
    let (y, m) = if month <= 2 {
        (year - 1, month as i32 + 12)
    } else {
        (year, month as i32)
    };
    let a = (y as f64 / 100.0).floor();
    let b = 2.0 - a + (a / 4.0).floor();
    (365.25 * (y as f64 + 4716.0)).floor()
        + (30.6001 * (m as f64 + 1.0)).floor()
        + day as f64
        + b
        - 1524.5
        + utc_hours / 24.0
}

/// Compute the moon's position and phase for a UTC instant and observer.
pub fn moon_position(
    year: i32,
    month: u32,
    day: u32,
    utc_hours: f64,
    latitude_deg: f64,
    longitude_deg: f64,
) -> MoonPosition {
    let jd = julian_day(year, month, day, utc_hours);
    let t = (jd - 2_451_545.0) / 36525.0; // Julian centuries since J2000

    // --- Moon's fundamental arguments (Meeus ch. 47), degrees ---
    let lp = norm360(218.3164477 + 481267.88123421 * t); // mean longitude L'
    let d = norm360(297.8501921 + 445267.1114034 * t); // mean elongation
    let m = norm360(357.5291092 + 35999.0502909 * t); // sun mean anomaly
    let mp = norm360(134.9633964 + 477198.8675055 * t); // moon mean anomaly M'
    let f = norm360(93.272095 + 483202.0175233 * t); // argument of latitude

    let (dr, mr, mpr, fr) = (d * DEG, m * DEG, mp * DEG, f * DEG);

    // Truncated periodic terms for longitude (Σl, in 1e-6 degrees) — the
    // largest few dozen terms give arc-minute accuracy.
    let lon = lp
        + (6.288774 * mpr.sin()
            + 1.274027 * (2.0 * dr - mpr).sin()
            + 0.658314 * (2.0 * dr).sin()
            + 0.213618 * (2.0 * mpr).sin()
            - 0.185116 * mr.sin()
            - 0.114332 * (2.0 * fr).sin()
            + 0.058793 * (2.0 * dr - 2.0 * mpr).sin()
            + 0.057066 * (2.0 * dr - mr - mpr).sin()
            + 0.053322 * (2.0 * dr + mpr).sin()
            + 0.045758 * (2.0 * dr - mr).sin()
            - 0.040923 * (mr - mpr).sin()
            - 0.034720 * dr.sin()
            - 0.030383 * (mr + mpr).sin()
            + 0.015327 * (2.0 * dr - 2.0 * fr).sin()
            - 0.012528 * (mpr + 2.0 * fr).sin()
            + 0.010980 * (mpr - 2.0 * fr).sin());

    // Latitude (Σb).
    let lat = 5.128122 * fr.sin()
        + 0.280602 * (mpr + fr).sin()
        + 0.277693 * (mpr - fr).sin()
        + 0.173237 * (2.0 * dr - fr).sin()
        + 0.055413 * (2.0 * dr - mpr + fr).sin()
        + 0.046271 * (2.0 * dr - mpr - fr).sin()
        + 0.032573 * (2.0 * dr + fr).sin()
        + 0.017198 * (2.0 * mpr + fr).sin();

    let lambda = lon * DEG; // ecliptic longitude (rad)
    let beta = lat * DEG; // ecliptic latitude (rad)

    // Obliquity of the ecliptic.
    let eps = (23.439291 - 0.0130042 * t) * DEG;

    // Ecliptic → equatorial (RA, dec).
    let ra = (lambda.sin() * eps.cos() - beta.tan() * eps.sin()).atan2(lambda.cos());
    let dec = (beta.sin() * eps.cos() + beta.cos() * eps.sin() * lambda.sin()).asin();

    // Local horizontal (alt/az) via local sidereal time.
    let gmst = norm360(280.46061837 + 360.98564736629 * (jd - 2_451_545.0)) * DEG;
    let lst = gmst + longitude_deg * DEG;
    let ha = lst - ra; // hour angle
    let phi = latitude_deg * DEG;

    let alt = (phi.sin() * dec.sin() + phi.cos() * dec.cos() * ha.cos()).asin();
    let mut az = (-ha.sin()).atan2(dec.sin() * phi.cos() - dec.cos() * phi.sin() * ha.cos());
    // Convert azimuth to "from North, clockwise" and wrap to [0, 2π).
    // The formula above already gives North=0 CW→East for this sign convention.
    if az < 0.0 {
        az += 2.0 * PI;
    }

    // --- Phase: elongation of the moon from the sun, and illuminated fraction.
    // Use the sun's ecliptic longitude from the solar model's celestial geometry
    // by comparing directions; a compact approximation uses the mean elongation
    // D plus the equation-of-centre corrections, but the illuminated fraction is
    // well-approximated by (1 - cos(phase_angle))/2 with phase_angle ≈ π - D_true.
    // Compute the true elongation from apparent longitudes for good accuracy.
    let sun = solar_position(year, month, day, utc_hours, latitude_deg, longitude_deg);
    let _ = &sun; // used below only for consistency; elongation via longitudes
    // Sun apparent ecliptic longitude (low precision).
    let sun_lon =
        norm360(280.46646 + 36000.76983 * t) + 1.914602 * (mr).sin() + 0.019993 * (2.0 * mr).sin();
    let elongation = norm360(lon - sun_lon) * DEG; // moon − sun ecliptic longitude
    // Phase angle i ≈ 180° − elongation (as seen from Earth, small-parallax approx).
    let phase_angle = PI - elongation;
    let phase_angle = phase_angle.rem_euclid(2.0 * PI);
    // Illuminated fraction k = (1 + cos i) / 2.
    let illuminated = (1.0 + phase_angle.cos()) / 2.0;
    // Waxing while elongation grows from 0→180° (moon east of sun).
    let waxing = elongation < PI;

    MoonPosition {
        altitude_rad: alt,
        azimuth_rad: az,
        illuminated_fraction: illuminated.clamp(0.0, 1.0),
        phase_angle_rad: (PI - elongation).abs().min(PI),
        waxing,
    }
}

fn norm360(x: f64) -> f64 {
    x.rem_euclid(360.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A known full moon: 2026-01-03 ~10:03 UTC (illuminated fraction ≈ 1.0).
    #[test]
    fn known_full_moon_is_fully_lit() {
        let m = moon_position(2026, 1, 3, 10.0, 0.0, 0.0);
        assert!(
            m.illuminated_fraction > 0.97,
            "2026-01-03 should be near-full, got {:.3}",
            m.illuminated_fraction
        );
        assert_eq!(m.phase_name(), MoonPhase::Full);
    }

    /// A known new moon: 2026-01-18 ~19:52 UTC (illuminated fraction ≈ 0).
    #[test]
    fn known_new_moon_is_dark() {
        let m = moon_position(2026, 1, 18, 20.0, 0.0, 0.0);
        assert!(
            m.illuminated_fraction < 0.03,
            "2026-01-18 should be near-new, got {:.3}",
            m.illuminated_fraction
        );
        assert_eq!(m.phase_name(), MoonPhase::New);
    }

    /// The illuminated fraction cycles through a full synodic month (~29.5 d):
    /// from a full moon, ~14.75 days later it should be near new.
    #[test]
    fn phase_cycles_over_a_synodic_month() {
        let full = moon_position(2026, 1, 3, 10.0, 0.0, 0.0).illuminated_fraction;
        let half_cycle = moon_position(2026, 1, 18, 0.0, 0.0, 0.0).illuminated_fraction;
        assert!(full > 0.9 && half_cycle < 0.1, "full {full:.2} → new {half_cycle:.2}");
    }

    /// The moon's altitude is a real number in range, and over a day it both
    /// rises and sets at a mid-latitude (unless circumpolar).
    #[test]
    fn moon_rises_and_sets() {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for h in 0..24 {
            let m = moon_position(2026, 6, 10, h as f64, 45.0, 0.0);
            min = min.min(m.altitude_deg());
            max = max.max(m.altitude_deg());
            assert!((-90.0..=90.0).contains(&m.altitude_deg()));
        }
        assert!(max > 0.0, "moon should be up at some point");
        assert!(min < 0.0, "moon should be down at some point");
    }

    /// Waxing vs waning is consistent: a few days after new moon it is waxing.
    #[test]
    fn waxing_after_new_moon() {
        // ~4 days after the 2026-01-18 new moon → waxing crescent.
        let m = moon_position(2026, 1, 22, 20.0, 0.0, 0.0);
        assert!(m.waxing, "should be waxing after new moon");
        assert!(m.illuminated_fraction > 0.05 && m.illuminated_fraction < 0.5);
    }
}
