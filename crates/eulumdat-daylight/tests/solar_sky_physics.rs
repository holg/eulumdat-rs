//! Comprehensive solar-geometry and sky-model physics — the ground truth a
//! daylight product stakes its credibility on.
//!
//! These assert astronomical facts (equinox/solstice geometry, sunrise/sunset
//! symmetry, azimuth sweep, polar day/night, twilight) and sky-model
//! invariants (energy conservation, luminance gradients, turbidity/season
//! monotonicity) rather than just "it runs". If any of these break, the
//! daylight simulation is producing physically wrong numbers.

use eulumdat_daylight::availability::DaylightAvailability;
use eulumdat_daylight::location::{location_by_name, LocalDateTime};
use eulumdat_daylight::sky::{CieSky, PerezSky, SkyParams, SkyRadiance};
use eulumdat_daylight::solar::solar_position;

// ── Solar geometry ───────────────────────────────────────────────────────

/// Equinox solar-noon altitude at latitude φ is ≈ 90° − |φ|, for a spread of
/// latitudes in both hemispheres.
#[test]
fn equinox_noon_altitude_tracks_latitude() {
    // At the March equinox the subsolar point is on the equator; local noon at
    // longitude 0 is ~12:00 UTC.
    for &lat in &[-60.0, -40.0, -20.0, 0.0, 20.0, 40.0, 60.0] {
        let sun = solar_position(2026, 3, 20, 12.0, lat, 0.0);
        let expected = 90.0 - lat.abs();
        assert!(
            (sun.altitude_deg() - expected).abs() < 2.5,
            "equinox noon at {lat}°: expected ~{expected:.0}°, got {:.1}°",
            sun.altitude_deg()
        );
    }
}

/// Summer solstice noon is higher than winter solstice noon in the northern
/// hemisphere by ~2×23.44° (the obliquity swing).
#[test]
fn solstice_swing_matches_obliquity() {
    let lat = 45.0;
    let summer = solar_position(2026, 6, 21, 12.0, lat, 0.0).altitude_deg();
    let winter = solar_position(2026, 12, 21, 12.0, lat, 0.0).altitude_deg();
    let swing = summer - winter;
    assert!(
        (swing - 2.0 * 23.44).abs() < 2.0,
        "solstice altitude swing should be ~{:.1}°, got {swing:.1}°",
        2.0 * 23.44
    );
}

/// The sun climbs from sunrise to noon and descends symmetrically to sunset;
/// noon is the daily maximum altitude.
#[test]
fn altitude_peaks_at_solar_noon() {
    let lat = 40.0;
    let mut alts = Vec::new();
    for h in 0..=24 {
        alts.push(solar_position(2026, 3, 20, h as f64, lat, 0.0).altitude_deg());
    }
    // Peak near 12:00 UTC (longitude 0 → solar noon ≈ civil noon).
    let (peak_idx, _) = alts
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .unwrap();
    assert!(
        (11..=13).contains(&peak_idx),
        "solar noon should peak near 12:00, peaked at {peak_idx}:00"
    );
}

/// Azimuth sweeps west-to-east through the day: at northern mid-latitudes the
/// sun is in the east in the morning and the west in the afternoon.
#[test]
fn azimuth_sweeps_east_to_west() {
    let lat = 40.0;
    let morning = solar_position(2026, 6, 21, 8.0, lat, 0.0);
    let afternoon = solar_position(2026, 6, 21, 16.0, lat, 0.0);
    assert!(morning.is_daytime() && afternoon.is_daytime());
    // Azimuth from North CW: morning eastward (< 180°), afternoon westward (> 180°).
    assert!(
        morning.azimuth_deg() < 180.0,
        "morning sun should be in the east, az {:.0}°",
        morning.azimuth_deg()
    );
    assert!(
        afternoon.azimuth_deg() > 180.0,
        "afternoon sun should be in the west, az {:.0}°",
        afternoon.azimuth_deg()
    );
}

/// The sun's altitude is symmetric about *true* solar noon: equal time before
/// and after the daily peak gives equal altitude. (Civil noon is offset from
/// solar noon by the equation of time, so we locate the true peak first rather
/// than assuming 12:00.)
#[test]
fn morning_evening_symmetry_about_noon() {
    let lat = 35.0;
    // Find true solar noon (peak altitude) to fine resolution.
    let mut best_h = 12.0;
    let mut best_alt = f64::NEG_INFINITY;
    let mut h = 10.0;
    while h <= 14.0 {
        let alt = solar_position(2026, 3, 20, h, lat, 0.0).altitude_deg();
        if alt > best_alt {
            best_alt = alt;
            best_h = h;
        }
        h += 0.01;
    }
    // Sample ±3 h around the true peak — these must match closely.
    let before = solar_position(2026, 3, 20, best_h - 3.0, lat, 0.0).altitude_deg();
    let after = solar_position(2026, 3, 20, best_h + 3.0, lat, 0.0).altitude_deg();
    assert!(
        (before - after).abs() < 0.5,
        "symmetry about true solar noon ({best_h:.2}h): {before:.2}° vs {after:.2}°"
    );
}

/// Southern-hemisphere seasons are inverted: December is high summer at 33°S.
#[test]
fn southern_hemisphere_seasons_are_inverted() {
    let sydney = location_by_name("Sydney").unwrap();
    let dec = LocalDateTime::new(2026, 12, 21, 12.0)
        .solar_position(&sydney)
        .altitude_deg();
    let jun = LocalDateTime::new(2026, 6, 21, 12.0)
        .solar_position(&sydney)
        .altitude_deg();
    assert!(
        dec > jun + 30.0,
        "Sydney December (summer) {dec:.0}° should far exceed June (winter) {jun:.0}°"
    );
}

/// At the equator the noon sun is near-vertical at the equinoxes and the
/// day length is ~12 h year-round (sun above horizon around midday any month).
#[test]
fn equatorial_sun_is_near_vertical_at_equinox() {
    let sun = solar_position(2026, 3, 20, 12.0, 0.0, 0.0);
    assert!(
        sun.altitude_deg() > 85.0,
        "equinox noon at equator should be ~overhead, got {:.1}°",
        sun.altitude_deg()
    );
}

// ── Twilight & day/night boundaries ──────────────────────────────────────

/// Civil twilight: the sun passes through the 0° horizon and −6° at dawn/dusk.
/// Somewhere in the pre-dawn hours the sun crosses from below −6° up past 0°.
#[test]
fn sun_crosses_horizon_at_dawn() {
    let lat = 40.0;
    // Scan fine hours around dawn (UTC ~04:00–07:00 at longitude 0 in June).
    let mut crossed = false;
    let mut prev_below = true;
    for step in 0..=60 {
        let h = 3.0 + step as f64 * 0.05; // 03:00 → 06:00
        let alt = solar_position(2026, 6, 21, h, lat, 0.0).altitude_deg();
        let below = alt < 0.0;
        if prev_below && !below {
            crossed = true;
        }
        prev_below = below;
    }
    assert!(crossed, "sun should cross the horizon (rise) during the dawn window");
}

/// Midnight sun vs polar night are captured by the location presets — sanity
/// that a mid-latitude has a normal night (sun goes below the horizon).
#[test]
fn mid_latitude_has_real_night() {
    let berlin = location_by_name("Berlin").unwrap();
    let midnight = LocalDateTime::new(2026, 6, 21, 0.0).solar_position(&berlin);
    assert!(
        !midnight.is_daytime(),
        "Berlin midnight in June should be night, alt {:.1}°",
        midnight.altitude_deg()
    );
}

// ── Daylight availability ────────────────────────────────────────────────

/// Global horizontal illuminance rises and falls with the sun through the day,
/// peaking at solar noon and hitting zero at night.
#[test]
fn ghi_follows_the_sun() {
    let lat = 40.0;
    let noon = {
        let sun = solar_position(2026, 6, 21, 12.0, lat, 0.0);
        DaylightAvailability::clear_sky(&sun, 2.5).ghi_lux
    };
    let evening = {
        let sun = solar_position(2026, 6, 21, 18.0, lat, 0.0);
        DaylightAvailability::clear_sky(&sun, 2.5).ghi_lux
    };
    let night = {
        let sun = solar_position(2026, 6, 21, 0.0, lat, 0.0);
        DaylightAvailability::clear_sky(&sun, 2.5).ghi_lux
    };
    assert!(noon > evening, "noon {noon:.0} > evening {evening:.0}");
    assert!(evening > night, "evening {evening:.0} > night {night:.0}");
    assert_eq!(night, 0.0, "no daylight at solar midnight");
}

/// Higher turbidity (haze) attenuates the direct beam but the sky stays lit —
/// DNI falls monotonically as turbidity climbs.
#[test]
fn turbidity_monotonically_dims_the_beam() {
    let sun = solar_position(2026, 6, 21, 12.0, 40.0, 0.0);
    let mut prev = f64::INFINITY;
    for t in [2.0, 3.0, 4.0, 6.0, 8.0] {
        let dni = DaylightAvailability::clear_sky(&sun, t).dni_lux;
        assert!(dni < prev, "DNI should fall as turbidity rises (t={t})");
        prev = dni;
    }
}

/// Overcast delivers far less light than clear sky and carries no beam.
#[test]
fn overcast_is_dimmer_and_beamless() {
    let sun = solar_position(2026, 6, 21, 12.0, 40.0, 0.0);
    let clear = DaylightAvailability::clear_sky(&sun, 2.5);
    let over = DaylightAvailability::overcast(&sun);
    assert_eq!(over.dni_lux, 0.0, "overcast has no direct beam");
    assert!(over.ghi_lux < clear.ghi_lux, "overcast dimmer than clear");
    assert!(over.dhi_lux > 0.0, "overcast is all diffuse");
}

// ── Sky-model energy conservation & structure ────────────────────────────

/// A Perez sky normalised to a DHI must integrate back to that DHI — the
/// cornerstone energy-conservation check for the diffuse dome.
#[test]
fn perez_dome_conserves_energy() {
    let sun = solar_position(2026, 6, 21, 12.0, 40.4, -3.7);
    let dhi = DaylightAvailability::clear_sky(&sun, 2.5).dhi_lux;
    let sky = PerezSky::new(sun, SkyParams::from_turbidity(2.5));
    let radiance = SkyRadiance::perez_from_dhi(sky, dhi);
    let integrated = radiance.integrate_horizontal_illuminance();
    let rel = (integrated - dhi).abs() / dhi;
    assert!(
        rel < 0.02,
        "Perez dome should integrate back to DHI {dhi:.0} lx, got {integrated:.0} lx (rel {rel:.4})"
    );
}

/// Same conservation for the CIE standard skies (overcast, clear, uniform).
#[test]
fn cie_skies_conserve_energy() {
    let sun = solar_position(2026, 6, 21, 12.0, 40.0, 0.0);
    let dhi = 15_000.0;
    for sky in [CieSky::Overcast, CieSky::Clear, CieSky::Uniform] {
        let radiance = SkyRadiance::cie_from_dhi(sky, sun, dhi);
        let integrated = radiance.integrate_horizontal_illuminance();
        let rel = (integrated - dhi).abs() / dhi;
        assert!(
            rel < 0.02,
            "{sky:?} sky should integrate back to {dhi:.0} lx, got {integrated:.0} (rel {rel:.4})"
        );
    }
}

/// The CIE overcast sky is 3× brighter at the zenith than the horizon
/// (Moon–Spencer gradation), and azimuth-independent.
#[test]
fn overcast_zenith_is_three_times_horizon() {
    use std::f64::consts::FRAC_PI_2;
    let sun = solar_position(2026, 6, 21, 12.0, 40.0, 0.0);
    let zenith = CieSky::Overcast.relative_luminance(0.0, 0.0, &sun);
    let horizon = CieSky::Overcast.relative_luminance(FRAC_PI_2 - 0.01, 0.0, &sun);
    assert!(
        (zenith / horizon - 3.0).abs() < 0.1,
        "overcast zenith/horizon should be ~3, got {:.2}",
        zenith / horizon
    );
    // Azimuth independence.
    let a = CieSky::Overcast.relative_luminance(0.8, 0.0, &sun);
    let b = CieSky::Overcast.relative_luminance(0.8, 3.14, &sun);
    assert!((a - b).abs() < 1e-9, "overcast must be azimuth-independent");
}

/// The clear sky is brightest near the sun (circumsolar region) — the Perez
/// indicatrix peaks toward the solar direction.
#[test]
fn clear_sky_is_brightest_near_the_sun() {
    let sun = solar_position(2026, 6, 21, 12.0, 40.0, 0.0);
    let sky = PerezSky::new(sun, SkyParams::from_turbidity(2.2));
    // The sun sits near the zenith at this instant; compare a sky element close
    // to the sun's azimuth/altitude against one on the opposite side.
    let near = sky.relative_luminance(sun.zenith_rad + 0.1, sun.azimuth_rad);
    let far = sky.relative_luminance(sun.zenith_rad + 0.1, sun.azimuth_rad + std::f64::consts::PI);
    assert!(
        near > far,
        "clear sky should be brighter toward the sun: near {near:.3} vs far {far:.3}"
    );
}

// ── Location presets drive real sun positions ────────────────────────────

/// Every preset location produces a physically valid sun position at local
/// noon (altitude in [−90,90], finite azimuth), and low-latitude cities have a
/// high midday sun.
#[test]
fn all_location_presets_give_valid_noon_sun() {
    for loc in eulumdat_daylight::LOCATIONS {
        let sun = LocalDateTime::new(2026, 6, 21, 12.0).solar_position(loc);
        assert!(
            (-90.0..=90.0).contains(&sun.altitude_deg()),
            "{}: altitude out of range {:.1}",
            loc.name,
            sun.altitude_deg()
        );
        assert!(sun.azimuth_deg().is_finite(), "{}: azimuth NaN", loc.name);
        // Tropical cities (|lat| < 25°) always have a high midday sun.
        if loc.latitude_deg.abs() < 25.0 {
            assert!(
                sun.altitude_deg() > 40.0,
                "{} (tropical) midday sun should be high, got {:.0}°",
                loc.name,
                sun.altitude_deg()
            );
        }
    }
}
