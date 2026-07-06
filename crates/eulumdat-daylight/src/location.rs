//! Named locations and local ⇄ UTC time handling for daylight calculations.
//!
//! [`solar_position`](crate::solar::solar_position) takes UTC decimal hours, but
//! a product exposes a *place* and a *local wall-clock time*. This module bridges
//! the two: a [`NamedLocation`] carries latitude, longitude and a standard UTC
//! offset, and [`LocalDateTime`] converts a local calendar instant to the UTC
//! hours the solar algorithm needs — rolling the date across midnight when the
//! offset pushes UTC into the previous or next day.
//!
//! The timezone offsets are **standard** (non-DST) fixed offsets. Daylight
//! saving is deliberately *not* modelled: DST rules are political, change over
//! time, and are irrelevant to sun geometry (the sun does not observe DST). A
//! caller wanting "civil summer time" simply passes a local time one hour later.

use crate::solar::{solar_position, SolarPosition};

/// Which daylight-saving-time regime a place follows. Used only to line up the
/// *displayed civil clock* with solar time — the sun itself ignores DST.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DstRule {
    /// No daylight saving (equatorial/tropical places, Iceland, most of Asia).
    None,
    /// EU rule: +1 h from the last Sunday of March to the last Sunday of October.
    Eu,
    /// US rule: +1 h from the second Sunday of March to the first Sunday of November.
    Us,
    /// Southern-hemisphere (inverted): +1 h ~October–March.
    SouthernEu,
}

/// A named place with the geographic and timezone data needed to turn a local
/// wall-clock time into a sun position.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NamedLocation {
    /// Display name, e.g. "Berlin".
    pub name: &'static str,
    /// Latitude, degrees, +North.
    pub latitude_deg: f64,
    /// Longitude, degrees, +East.
    pub longitude_deg: f64,
    /// Standard (non-DST) UTC offset in hours, e.g. +1.0 for CET, -8.0 for PST.
    pub utc_offset_hours: f64,
    /// Daylight-saving regime this place observes.
    pub dst: DstRule,
}

impl NamedLocation {
    /// Build a location with an explicit DST rule.
    pub const fn with_dst(
        name: &'static str,
        latitude_deg: f64,
        longitude_deg: f64,
        utc_offset_hours: f64,
        dst: DstRule,
    ) -> Self {
        Self {
            name,
            latitude_deg,
            longitude_deg,
            utc_offset_hours,
            dst,
        }
    }

    /// Build a location that does not observe DST.
    pub const fn new(
        name: &'static str,
        latitude_deg: f64,
        longitude_deg: f64,
        utc_offset_hours: f64,
    ) -> Self {
        Self::with_dst(name, latitude_deg, longitude_deg, utc_offset_hours, DstRule::None)
    }

    /// The effective UTC offset on a given date, including any DST shift.
    ///
    /// In summer the civil clock is +1 h ahead of standard time, so solar noon
    /// lands near 13:00 on the wall clock — which is what a user in Berlin
    /// expects. The `+1` is added to the offset so a given local wall-clock time
    /// maps to an *earlier* UTC instant (and thus the sun peaks an hour later on
    /// the local-time axis).
    pub fn effective_offset(&self, month: u32, day: u32) -> f64 {
        if dst_active(self.dst, month, day) {
            self.utc_offset_hours + 1.0
        } else {
            self.utc_offset_hours
        }
    }
}

/// Whether DST is active on a (month, day) for a rule. Uses month-boundary
/// approximations (exact to within the changeover week) — good enough to place
/// the solar peak on the civil clock; we intentionally avoid a full calendar.
fn dst_active(rule: DstRule, month: u32, day: u32) -> bool {
    match rule {
        DstRule::None => false,
        // EU: last Sun of March → last Sun of October. Approx: Apr–Oct fully in,
        // late March in, late October transitions out (use day<25 for Oct).
        DstRule::Eu => {
            (4..=9).contains(&month)
                || (month == 3 && day >= 25)
                || (month == 10 && day < 25)
        }
        // US: 2nd Sun of March → 1st Sun of November. Approx: Apr–Oct in,
        // mid-March in, early November transitions out.
        DstRule::Us => {
            (4..=10).contains(&month)
                || (month == 3 && day >= 8)
                || (month == 11 && day < 7)
        }
        // Southern hemisphere (e.g. parts of AU/SA): Oct–March.
        DstRule::SouthernEu => {
            month >= 10 || month <= 3 && !(month == 3 && day >= 25) && !(month == 4)
        }
    }
}

/// A curated table of major world locations, spanning both hemispheres, the
/// equator, and high latitudes (for polar-day/night behaviour). Ordered
/// roughly west-to-east so a UI dropdown reads sensibly.
pub const LOCATIONS: &[NamedLocation] = &[
    NamedLocation::new("Honolulu", 21.31, -157.86, -10.0), // Hawaii: no DST
    NamedLocation::with_dst("Anchorage", 61.22, -149.90, -9.0, DstRule::Us),
    NamedLocation::with_dst("Los Angeles", 34.05, -118.24, -8.0, DstRule::Us),
    NamedLocation::new("Mexico City", 19.43, -99.13, -6.0), // MX ended DST 2022
    NamedLocation::with_dst("Chicago", 41.88, -87.63, -6.0, DstRule::Us),
    NamedLocation::with_dst("New York", 40.71, -74.01, -5.0, DstRule::Us),
    NamedLocation::with_dst("São Paulo", -23.55, -46.63, -3.0, DstRule::None), // ended 2019
    NamedLocation::new("Reykjavík", 64.15, -21.94, 0.0), // Iceland: no DST
    NamedLocation::with_dst("London", 51.51, -0.13, 0.0, DstRule::Eu),
    NamedLocation::with_dst("Madrid", 40.42, -3.70, 1.0, DstRule::Eu),
    NamedLocation::with_dst("Lüdinghausen", 51.77, 7.44, 1.0, DstRule::Eu),
    NamedLocation::with_dst("Berlin", 52.52, 13.40, 1.0, DstRule::Eu),
    NamedLocation::new("Cairo", 30.04, 31.24, 2.0),
    NamedLocation::new("Nairobi", -1.29, 36.82, 3.0),
    NamedLocation::new("Dubai", 25.20, 55.27, 4.0),
    NamedLocation::new("Mumbai", 19.08, 72.88, 5.5),
    NamedLocation::new("Singapore", 1.35, 103.82, 8.0),
    NamedLocation::new("Beijing", 39.90, 116.41, 8.0),
    NamedLocation::new("Wuhan", 30.59, 114.30, 8.0), // China: single tz, no DST
    NamedLocation::new("Tokyo", 35.68, 139.65, 9.0),
    NamedLocation::with_dst("Sydney", -33.87, 151.21, 10.0, DstRule::SouthernEu),
    NamedLocation::with_dst("Longyearbyen", 78.22, 15.65, 1.0, DstRule::Eu), // Arctic
    NamedLocation::new("Equator (0°,0°)", 0.0, 0.0, 0.0),
];

/// Look up a preset location by name (case-sensitive exact match).
pub fn location_by_name(name: &str) -> Option<NamedLocation> {
    LOCATIONS.iter().copied().find(|l| l.name == name)
}

/// A local calendar date and wall-clock time at a [`NamedLocation`].
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LocalDateTime {
    /// Calendar year (e.g. 2026).
    pub year: i32,
    /// Month, 1–12.
    pub month: u32,
    /// Day of month, 1–31.
    pub day: u32,
    /// Local time in decimal hours (e.g. 13.5 = 13:30 local).
    pub local_hours: f64,
}

impl LocalDateTime {
    /// A local instant.
    pub fn new(year: i32, month: u32, day: u32, local_hours: f64) -> Self {
        Self {
            year,
            month,
            day,
            local_hours,
        }
    }

    /// Convert this local instant at `loc` to a `(year, month, day, utc_hours)`
    /// tuple, rolling the calendar date across midnight when the timezone
    /// offset moves UTC into an adjacent day.
    ///
    /// Uses the **DST-adjusted** offset for the date, so a summer wall-clock
    /// time maps to the correct UTC instant (and the solar peak appears near
    /// 13:00 on the local axis in EU/US summer, matching the civil clock).
    pub fn to_utc(&self, loc: &NamedLocation) -> (i32, u32, u32, f64) {
        let utc = self.local_hours - loc.effective_offset(self.month, self.day);
        if utc < 0.0 {
            let (y, m, d) = prev_day(self.year, self.month, self.day);
            (y, m, d, utc + 24.0)
        } else if utc >= 24.0 {
            let (y, m, d) = next_day(self.year, self.month, self.day);
            (y, m, d, utc - 24.0)
        } else {
            (self.year, self.month, self.day, utc)
        }
    }

    /// Sun position for this local instant at `loc`.
    pub fn solar_position(&self, loc: &NamedLocation) -> SolarPosition {
        let (y, m, d, utc) = self.to_utc(loc);
        solar_position(y, m, d, utc, loc.latitude_deg, loc.longitude_deg)
    }

    /// Moon position + phase for this local instant at `loc`.
    pub fn moon_position(&self, loc: &NamedLocation) -> crate::lunar::MoonPosition {
        let (y, m, d, utc) = self.to_utc(loc);
        crate::lunar::moon_position(y, m, d, utc, loc.latitude_deg, loc.longitude_deg)
    }
}

/// Number of days in a Gregorian month.
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
            if leap {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

/// The calendar day before `(year, month, day)`.
fn prev_day(year: i32, month: u32, day: u32) -> (i32, u32, u32) {
    if day > 1 {
        (year, month, day - 1)
    } else if month > 1 {
        (year, month - 1, days_in_month(year, month - 1))
    } else {
        (year - 1, 12, 31)
    }
}

/// The calendar day after `(year, month, day)`.
fn next_day(year: i32, month: u32, day: u32) -> (i32, u32, u32) {
    if day < days_in_month(year, month) {
        (year, month, day + 1)
    } else if month < 12 {
        (year, month + 1, 1)
    } else {
        (year + 1, 1, 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_offset_no_rollover() {
        // Berlin standard time (winter, +1): 13:00 local → 12:00 UTC, same day.
        let dt = LocalDateTime::new(2026, 12, 21, 13.0);
        let berlin = location_by_name("Berlin").unwrap();
        let (y, m, d, utc) = dt.to_utc(&berlin);
        assert_eq!((y, m, d), (2026, 12, 21));
        assert!((utc - 12.0).abs() < 1e-9);
    }

    #[test]
    fn berlin_summer_uses_dst_offset() {
        // Berlin summer (DST, +2): 13:00 local → 11:00 UTC.
        let dt = LocalDateTime::new(2026, 6, 21, 13.0);
        let berlin = location_by_name("Berlin").unwrap();
        let (_, _, _, utc) = dt.to_utc(&berlin);
        assert!((utc - 11.0).abs() < 1e-9, "summer Berlin 13:00 → 11:00 UTC, got {utc}");
        assert_eq!(berlin.effective_offset(6, 21), 2.0);
        assert_eq!(berlin.effective_offset(12, 21), 1.0);
    }

    #[test]
    fn utc_rollover_backwards() {
        // Tokyo (+9): 06:00 local → 21:00 UTC previous day.
        let dt = LocalDateTime::new(2026, 3, 1, 6.0);
        let tokyo = location_by_name("Tokyo").unwrap();
        let (y, m, d, utc) = dt.to_utc(&tokyo);
        assert_eq!((y, m, d), (2026, 2, 28));
        assert!((utc - 21.0).abs() < 1e-9);
    }

    #[test]
    fn utc_rollover_forwards() {
        // Los Angeles (−8): 20:00 local → 04:00 UTC next day.
        let dt = LocalDateTime::new(2026, 12, 31, 20.0);
        let la = location_by_name("Los Angeles").unwrap();
        let (y, m, d, utc) = dt.to_utc(&la);
        assert_eq!((y, m, d), (2027, 1, 1));
        assert!((utc - 4.0).abs() < 1e-9);
    }

    #[test]
    fn local_solar_noon_sun_is_high_in_summer() {
        // Local ~13:00 in Madrid at summer solstice → high sun (CET is ~15 min
        // ahead of local solar time at Madrid's longitude, so 13:00 civil is
        // near solar noon).
        let madrid = location_by_name("Madrid").unwrap();
        let dt = LocalDateTime::new(2026, 6, 21, 14.0); // CET summer clock-ish
        let sun = dt.solar_position(&madrid);
        assert!(
            sun.altitude_deg() > 65.0,
            "Madrid summer local noon should be high, got {}",
            sun.altitude_deg()
        );
    }

    #[test]
    fn arctic_polar_day_sun_never_sets() {
        // Longyearbyen (78°N) at summer solstice: the sun stays above the
        // horizon all 24 hours (polar day).
        let svalbard = location_by_name("Longyearbyen").unwrap();
        let mut min_alt = f64::INFINITY;
        for h in 0..24 {
            let dt = LocalDateTime::new(2026, 6, 21, h as f64);
            let sun = dt.solar_position(&svalbard);
            min_alt = min_alt.min(sun.altitude_deg());
        }
        assert!(
            min_alt > 0.0,
            "polar day: sun should never set at 78°N in June, min alt {min_alt}"
        );
    }

    #[test]
    fn arctic_polar_night_sun_never_rises() {
        // Longyearbyen at winter solstice: sun stays below the horizon all day.
        let svalbard = location_by_name("Longyearbyen").unwrap();
        let mut max_alt = f64::NEG_INFINITY;
        for h in 0..24 {
            let dt = LocalDateTime::new(2026, 12, 21, h as f64);
            let sun = dt.solar_position(&svalbard);
            max_alt = max_alt.max(sun.altitude_deg());
        }
        assert!(
            max_alt < 0.0,
            "polar night: sun should never rise at 78°N in December, max alt {max_alt}"
        );
    }

    #[test]
    fn all_presets_have_valid_coordinates() {
        for loc in LOCATIONS {
            assert!(
                (-90.0..=90.0).contains(&loc.latitude_deg),
                "{}: bad latitude",
                loc.name
            );
            assert!(
                (-180.0..=180.0).contains(&loc.longitude_deg),
                "{}: bad longitude",
                loc.name
            );
            assert!(
                (-12.0..=14.0).contains(&loc.utc_offset_hours),
                "{}: bad tz offset",
                loc.name
            );
        }
    }

    #[test]
    fn ludinghausen_and_wuhan_are_present() {
        let lud = location_by_name("Lüdinghausen").expect("Lüdinghausen present");
        assert!((lud.latitude_deg - 51.77).abs() < 0.01);
        assert!((lud.longitude_deg - 7.44).abs() < 0.01);
        assert_eq!(lud.dst, DstRule::Eu);

        let wuhan = location_by_name("Wuhan").expect("Wuhan present");
        assert!((wuhan.latitude_deg - 30.59).abs() < 0.01);
        assert!((wuhan.longitude_deg - 114.30).abs() < 0.01);
        assert_eq!(wuhan.dst, DstRule::None); // China: no DST
    }

    /// The headline DST behaviour: in EU summer the solar peak lands near 13:00
    /// on the civil clock (because clocks are +1 h), but near 12:00 (well, near
    /// solar noon for the longitude) in winter with standard time.
    #[test]
    fn summer_solar_peak_shifts_to_civil_13h() {
        let lud = location_by_name("Lüdinghausen").unwrap();

        // Find the civil hour of peak altitude in summer (DST active) vs winter.
        let peak_hour = |month: u32, day: u32| {
            let mut best_h = 12.0;
            let mut best_alt = f64::NEG_INFINITY;
            let mut h = 10.0;
            while h <= 15.0 {
                let alt = LocalDateTime::new(2026, month, day, h)
                    .solar_position(&lud)
                    .altitude_deg();
                if alt > best_alt {
                    best_alt = alt;
                    best_h = h;
                }
                h += 0.02;
            }
            best_h
        };

        let summer = peak_hour(6, 21); // DST active → peak ~13:2x civil
        let winter = peak_hour(12, 21); // standard time → peak ~12:2x civil

        assert!(
            (12.9..=13.7).contains(&summer),
            "EU summer solar peak should be near 13:00 civil, got {summer:.2}"
        );
        assert!(
            (11.9..=12.7).contains(&winter),
            "EU winter solar peak should be near 12:00 civil, got {winter:.2}"
        );
        // The DST shift is ~1 hour.
        assert!(
            (summer - winter - 1.0).abs() < 0.3,
            "DST should shift the peak by ~1 h: summer {summer:.2} − winter {winter:.2}"
        );
    }

    #[test]
    fn china_has_no_dst_shift() {
        // Wuhan (no DST): the peak hour should be the same season-to-season
        // (only the equation-of-time wiggle differs, well under the DST hour).
        let wuhan = location_by_name("Wuhan").unwrap();
        let peak = |month: u32| {
            let mut best_h = 12.0;
            let mut best_alt = f64::NEG_INFINITY;
            let mut h = 11.0;
            while h <= 14.0 {
                let alt = LocalDateTime::new(2026, month, 21, h)
                    .solar_position(&wuhan)
                    .altitude_deg();
                if alt > best_alt {
                    best_alt = alt;
                    best_h = h;
                }
                h += 0.02;
            }
            best_h
        };
        let (s, w) = (peak(6), peak(12));
        assert!((s - w).abs() < 0.5, "no-DST place: peak stable, {s:.2} vs {w:.2}");
    }
}
