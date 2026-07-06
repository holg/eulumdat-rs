//! Solar position via the PSA algorithm (Blanco-Muriel et al., 2001).
//!
//! Computes the sun's topocentric zenith and azimuth from UTC date/time and the
//! observer's latitude/longitude. Accuracy ≈ ±0.01° over 1999–2015 and a few
//! hundredths of a degree for years near that range — far better than any
//! daylight model needs. Pure arithmetic, no ephemeris tables.
//!
//! Reference: M. Blanco-Muriel, D. C. Alarcón-Padilla, T. López-Moratalla,
//! M. Lara-Coira, "Computing the solar vector", *Solar Energy* 70(5), 2001.

use crate::coords::{altaz_to_world, WorldDir};

/// Sun position in the local horizontal (topocentric) frame.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SolarPosition {
    /// Elevation above the horizon, radians. Negative ⇒ sun is below the
    /// horizon (night / twilight).
    pub altitude_rad: f64,
    /// Azimuth from North, clockwise toward East, radians (N=0, E=π/2).
    pub azimuth_rad: f64,
    /// Zenith angle (= π/2 − altitude), radians.
    pub zenith_rad: f64,
}

impl SolarPosition {
    /// Altitude above the horizon, degrees (negative below horizon).
    pub fn altitude_deg(&self) -> f64 {
        self.altitude_rad.to_degrees()
    }
    /// Azimuth from North clockwise, degrees.
    pub fn azimuth_deg(&self) -> f64 {
        self.azimuth_rad.to_degrees()
    }
    /// Zenith angle, degrees.
    pub fn zenith_deg(&self) -> f64 {
        self.zenith_rad.to_degrees()
    }
    /// True when the sun is above the horizon.
    pub fn is_daytime(&self) -> bool {
        self.altitude_rad > 0.0
    }
    /// Unit vector toward the sun in the ENU world frame (+X East, +Y up,
    /// +Z South). Up-going; negate for the photon arrival direction.
    pub fn world_direction(&self) -> WorldDir {
        altaz_to_world(self.altitude_deg(), self.azimuth_deg())
    }
}

/// Compute the sun position for a UTC instant and an observer location.
///
/// * `year`, `month`, `day` — calendar date (UTC).
/// * `utc_hours` — decimal hours UTC (e.g. 13.5 = 13:30 UTC).
/// * `latitude_deg` — observer latitude, +North.
/// * `longitude_deg` — observer longitude, +East.
pub fn solar_position(
    year: i32,
    month: u32,
    day: u32,
    utc_hours: f64,
    latitude_deg: f64,
    longitude_deg: f64,
) -> SolarPosition {
    use std::f64::consts::PI;
    let twopi = 2.0 * PI;
    let rad = PI / 180.0;
    let earth_mean_radius = 6371.01_f64; // km
    let astronomical_unit = 149_597_890.0_f64; // km

    // --- Julian Day from the Gregorian calendar (PSA formulation) ---
    let y = year as f64;
    let m = month as f64;
    let d = day as f64;
    let decimal_hours = utc_hours;
    // integer day-count terms
    let aux1 = ((m as i64 - 14) as f64 / 12.0).floor();
    let aux2 = (1461.0 * (y + 4800.0 + aux1) / 4.0).floor()
        + (367.0 * (m - 2.0 - 12.0 * aux1) / 12.0).floor()
        - (3.0 * ((y + 4900.0 + aux1) / 100.0).floor() / 4.0).floor()
        + d
        - 32075.0;
    let julian_date = aux2 - 0.5 + decimal_hours / 24.0;
    // Elapsed Julian days since J2000 (JD 2451545.0).
    let elapsed_julian_days = julian_date - 2_451_545.0;

    // --- Ecliptic coordinates ---
    let omega = 2.1429 - 0.0010394594 * elapsed_julian_days;
    let mean_longitude = 4.8950630 + 0.017202791698 * elapsed_julian_days; // rad
    let mean_anomaly = 6.2400600 + 0.0172019699 * elapsed_julian_days;
    let ecliptic_longitude = mean_longitude
        + 0.03341607 * mean_anomaly.sin()
        + 0.00034894 * (2.0 * mean_anomaly).sin()
        - 0.0001134
        - 0.0000203 * omega.sin();
    let ecliptic_obliquity = 0.4090928 - 6.2140e-9 * elapsed_julian_days + 0.0000396 * omega.cos();

    // --- Celestial (right ascension / declination) ---
    let sin_ecliptic_longitude = ecliptic_longitude.sin();
    let dy = ecliptic_obliquity.cos() * sin_ecliptic_longitude;
    let dx = ecliptic_longitude.cos();
    let mut right_ascension = dy.atan2(dx);
    if right_ascension < 0.0 {
        right_ascension += twopi;
    }
    let declination = (ecliptic_obliquity.sin() * sin_ecliptic_longitude).asin();

    // --- Local coordinates (azimuth / zenith) ---
    let greenwich_mean_sidereal_time =
        6.6974243242 + 0.0657098283 * elapsed_julian_days + decimal_hours;
    let local_mean_sidereal_time =
        (greenwich_mean_sidereal_time * 15.0 + longitude_deg) * rad;
    let hour_angle = local_mean_sidereal_time - right_ascension;
    let latitude = latitude_deg * rad;
    let cos_lat = latitude.cos();
    let sin_lat = latitude.sin();
    let cos_ha = hour_angle.cos();

    let mut zenith_angle =
        (cos_lat * cos_ha * declination.cos() + declination.sin() * sin_lat).acos();
    let ay = -hour_angle.sin();
    let ax = declination.tan() * cos_lat - sin_lat * cos_ha;
    let mut azimuth = ay.atan2(ax);
    if azimuth < 0.0 {
        azimuth += twopi;
    }
    // Parallax correction (Earth radius vs Sun distance).
    let parallax = (earth_mean_radius / astronomical_unit) * zenith_angle.sin();
    zenith_angle += parallax;

    let altitude = std::f64::consts::FRAC_PI_2 - zenith_angle;
    SolarPosition {
        altitude_rad: altitude,
        azimuth_rad: azimuth,
        zenith_rad: zenith_angle,
    }
}

/// Relative optical air mass (Kasten & Young, 1989). Valid for the sun above
/// the horizon; returns a large value as the sun approaches/sets.
pub fn air_mass(zenith_rad: f64) -> f64 {
    let z_deg = zenith_rad.to_degrees().min(90.0);
    1.0 / (zenith_rad.cos() + 0.50572 * (96.07995 - z_deg).powf(-1.6364))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// At the equinox, solar noon at the equator puts the sun ~overhead.
    #[test]
    fn equinox_noon_equator_high() {
        // 2026-03-20, ~12:00 UTC at lon 0, lat 0.
        let p = solar_position(2026, 3, 20, 12.0, 0.0, 0.0);
        assert!(
            p.altitude_deg() > 80.0,
            "equinox noon at equator should be near-overhead, got {}",
            p.altitude_deg()
        );
    }

    /// Solar-noon altitude at latitude φ on the equinox ≈ 90° − φ.
    #[test]
    fn equinox_noon_altitude_matches_latitude() {
        // lon 0 so 12:00 UTC ≈ local solar noon; lat 40°N.
        let p = solar_position(2026, 3, 20, 12.0, 40.0, 0.0);
        let expected = 90.0 - 40.0; // ≈ 50°
        assert!(
            (p.altitude_deg() - expected).abs() < 2.0,
            "equinox noon altitude at 40°N: expected ~{expected}, got {}",
            p.altitude_deg()
        );
    }

    /// Summer-solstice noon at 40°N ≈ 90 − 40 + 23.44.
    #[test]
    fn summer_solstice_noon_altitude() {
        let p = solar_position(2026, 6, 21, 12.0, 40.0, 0.0);
        let expected = 90.0 - 40.0 + 23.44; // ≈ 73.4°
        assert!(
            (p.altitude_deg() - expected).abs() < 2.0,
            "solstice noon at 40°N: expected ~{expected}, got {}",
            p.altitude_deg()
        );
    }

    /// Local solar noon ⇒ sun roughly due South in the northern hemisphere.
    #[test]
    fn noon_azimuth_is_south() {
        let p = solar_position(2026, 3, 20, 12.0, 40.0, 0.0);
        // Azimuth from North clockwise; due South = 180°.
        assert!(
            (p.azimuth_deg() - 180.0).abs() < 8.0,
            "noon azimuth should be ~South (180°), got {}",
            p.azimuth_deg()
        );
    }

    /// Midnight at the equator ⇒ sun below the horizon.
    #[test]
    fn midnight_below_horizon() {
        let p = solar_position(2026, 3, 20, 0.0, 0.0, 0.0);
        assert!(
            !p.is_daytime(),
            "midnight at lon0/lat0 should be night, alt {}",
            p.altitude_deg()
        );
    }
}
