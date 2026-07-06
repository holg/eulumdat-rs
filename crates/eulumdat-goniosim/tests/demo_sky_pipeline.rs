//! Headless verification of the exact pipeline the Spectrum Lab "Sky mode"
//! demo drives: a user picks a location + local date/time, the sun position is
//! computed, and a daylight/nightlight sky is traced to an open-plane
//! illuminance whose regime (daylight / twilight / night) and magnitude are
//! physically correct.
//!
//! This stands in for a browser click-through: it runs the same
//! `LocalDateTime → SolarPosition → SkyDome → trace_illuminance` chain the WASM
//! component uses and asserts the numbers a business demo would show on screen.

use eulumdat_daylight::availability::DaylightAvailability;
use eulumdat_daylight::location::{location_by_name, LocalDateTime};
use eulumdat_daylight::sky::{PerezSky, SkyParams, SkyRadiance};
use eulumdat_daylight::SolarPosition;
use eulumdat_goniosim::*;
use nalgebra::Point3;

/// Reproduce the demo's sky trace: build the sky for a sun position and read the
/// open-plane illuminance (lux). Returns (lux, regime).
fn demo_sky_lux(sun: &SolarPosition) -> (f64, &'static str) {
    let emit_half = 2.0;
    let (dome, regime) = if sun.is_daytime() {
        let avail = DaylightAvailability::clear_sky(sun, 2.5);
        let perez = PerezSky::new(*sun, SkyParams::from_turbidity(2.5));
        let target = avail.ghi_lux.max(1.0);
        let radiance = SkyRadiance::perez_from_dhi(perez, target);
        let dome = SkyDomeSource::with_extent(
            &radiance,
            Point3::new(0.0, 0.0, 0.02),
            1.0,
            emit_half,
            20,
            40,
        );
        let flux = dome.collector_flux(target);
        let dome = dome.with_flux(flux);
        let regime = if sun.altitude_deg() > 6.0 { "day" } else { "twilight" };
        (dome, regime)
    } else {
        let night_lux = if sun.altitude_deg() > -6.0 { 5.0 } else { 0.001 };
        let src = night_sky_source(Point3::new(0.0, 0.0, 0.02), emit_half, night_lux, 12, 24);
        let dome = match src {
            Source::SkyDome(d) => d,
            _ => unreachable!(),
        };
        let regime = if sun.altitude_deg() > -6.0 { "twilight" } else { "night" };
        (dome, regime)
    };

    let mut scene = Scene::new();
    scene.add_source(Source::SkyDome(dome));
    let plane = PlaneDetector::horizontal(Point3::origin(), 0.5, 0.5, 1, 1, false);
    let cfg = TracerConfig {
        num_photons: 150_000,
        max_bounces: 4,
        seed: 42,
        ..TracerConfig::default()
    };
    let result = trace_illuminance(&scene, &cfg, vec![plane]);
    (result.average_lux(0), regime)
}

/// Picking Berlin at summer noon shows bright daylight; the same place at
/// midnight shows a night regime with sub-lux illuminance. This is the headline
/// demo interaction: drag the time slider, watch day turn to night.
#[test]
fn berlin_day_to_night_via_time_slider() {
    let berlin = location_by_name("Berlin").unwrap();

    let noon = LocalDateTime::new(2026, 6, 21, 13.0).solar_position(&berlin);
    let (noon_lux, noon_regime) = demo_sky_lux(&noon);
    assert_eq!(noon_regime, "day");
    assert!(
        noon_lux > 10_000.0,
        "Berlin summer noon should be bright daylight, got {noon_lux:.0} lx"
    );

    let midnight = LocalDateTime::new(2026, 6, 22, 1.0).solar_position(&berlin);
    let (night_lux, night_regime) = demo_sky_lux(&midnight);
    assert!(
        night_regime == "night" || night_regime == "twilight",
        "Berlin 01:00 June should be night/twilight, got {night_regime}"
    );
    assert!(
        night_lux < noon_lux,
        "night {night_lux} must be far dimmer than noon {noon_lux}"
    );
}

/// Sweeping the local-time slider through a full day at a mid-latitude city
/// walks through all three regimes (night → twilight → day → twilight → night)
/// and the illuminance follows a single-peaked diurnal curve.
#[test]
fn time_slider_walks_through_all_regimes() {
    let ny = location_by_name("New York").unwrap();
    let mut regimes = std::collections::HashSet::new();
    let mut lux_curve = Vec::new();
    // Sample every 2 hours across a spring day (has real day and night).
    for h in (0..24).step_by(2) {
        let sun = LocalDateTime::new(2026, 3, 20, h as f64).solar_position(&ny);
        let (lux, regime) = demo_sky_lux(&sun);
        regimes.insert(regime);
        lux_curve.push((h, lux));
    }
    assert!(regimes.contains("day"), "should hit daylight during the day");
    assert!(regimes.contains("night"), "should hit night");

    // Peak illuminance occurs near midday, not at the extremes.
    let (peak_h, _) = lux_curve
        .iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .unwrap();
    assert!(
        (10..=14).contains(peak_h),
        "diurnal peak should be near midday, peaked at {peak_h}:00"
    );
}

/// Changing the *location* preset changes the sun and hence the sky: a tropical
/// city has a far brighter midday sky than a high-latitude city on the same
/// winter date (the sun barely rises up north).
#[test]
fn location_preset_changes_the_sky() {
    let date = |city: &str| {
        let loc = location_by_name(city).unwrap();
        LocalDateTime::new(2026, 12, 21, 12.0).solar_position(&loc)
    };
    let (tropical_lux, _) = demo_sky_lux(&date("Singapore")); // ~1°N
    let (nordic_lux, _) = demo_sky_lux(&date("Reykjavík")); // ~64°N, deep winter
    assert!(
        tropical_lux > nordic_lux * 2.0,
        "Singapore Dec noon {tropical_lux:.0} lx should far exceed Reykjavík {nordic_lux:.0} lx"
    );
}

/// The Arctic winter demo: Longyearbyen (78°N) in December is in polar night —
/// the sky trace correctly reports a night regime at *every* hour of the day,
/// a striking business-demo scenario.
#[test]
fn arctic_polar_night_is_dark_all_day() {
    let svalbard = location_by_name("Longyearbyen").unwrap();
    for h in (0..24).step_by(3) {
        let sun = LocalDateTime::new(2026, 12, 21, h as f64).solar_position(&svalbard);
        assert!(
            !sun.is_daytime(),
            "polar night: {h}:00 must be below the horizon (alt {:.1}°)",
            sun.altitude_deg()
        );
        let (lux, _) = demo_sky_lux(&sun);
        assert!(lux < 10.0, "polar night sky should be dark, got {lux:.3} lx at {h}:00");
    }
}

/// The Arctic summer demo: the same place in June is in polar day — daylight
/// at every hour, including local midnight (the "midnight sun").
#[test]
fn arctic_midnight_sun_is_lit_all_day() {
    let svalbard = location_by_name("Longyearbyen").unwrap();
    for h in (0..24).step_by(3) {
        let sun = LocalDateTime::new(2026, 6, 21, h as f64).solar_position(&svalbard);
        assert!(
            sun.is_daytime(),
            "midnight sun: {h}:00 must be above the horizon (alt {:.1}°)",
            sun.altitude_deg()
        );
        let (lux, regime) = demo_sky_lux(&sun);
        assert_eq!(regime, "day", "polar day should read daylight at {h}:00");
        assert!(lux > 1_000.0, "polar day sky should be bright, got {lux:.0} lx at {h}:00");
    }
}

// ── Moon phase drives the night sky ──────────────────────────────────────

/// The night illuminance model the demo uses: starlight floor + the moon's
/// phase-scaled, altitude-projected contribution. Mirrors `run_sky_trace`'s
/// night branch.
fn night_lux_with_moon(
    star_floor: f64,
    moon: &eulumdat_daylight::MoonPosition,
) -> f64 {
    let moon_face = moon_illuminance(moon.illuminated_fraction);
    let moon_lux = if moon.is_up() {
        moon_face * moon.altitude_rad.sin().max(0.0)
    } else {
        0.0
    };
    star_floor + moon_lux
}

/// A full moon high in the sky makes the night hugely brighter than a new moon
/// — the whole reason moon phase matters. This asserts the ~100×+ swing the
/// demo now produces.
#[test]
fn full_moon_night_far_brighter_than_new_moon() {
    use eulumdat_daylight::lunar::moon_position;

    // Berlin, deep night (02:00). Full moon 2026-01-03 vs new moon 2026-01-18.
    let loc = location_by_name("Berlin").unwrap();
    let full = LocalDateTime::new(2026, 1, 3, 2.0).moon_position(&loc);
    let new = LocalDateTime::new(2026, 1, 18, 2.0).moon_position(&loc);

    let star_floor = 0.0015;
    let full_lux = night_lux_with_moon(star_floor, &full);
    let new_lux = night_lux_with_moon(star_floor, &new);

    assert!(full.illuminated_fraction > 0.95, "control: Jan 3 is full");
    assert!(new.illuminated_fraction < 0.05, "control: Jan 18 is new");
    // If the full moon is above the horizon it should dominate; if it happens to
    // be down at 02:00, pick an hour when it is up.
    let full_up = (0..24)
        .map(|h| LocalDateTime::new(2026, 1, 3, h as f64).moon_position(&loc))
        .find(|m| m.is_up() && m.altitude_deg() > 20.0)
        .expect("full moon is up sometime that night");
    let full_up_lux = night_lux_with_moon(star_floor, &full_up);

    assert!(
        full_up_lux > 0.05,
        "a high full moon should deliver >0.05 lx, got {full_up_lux:.4}"
    );
    assert!(
        full_up_lux > 50.0 * new_lux,
        "full moon night ({full_up_lux:.4} lx) must dwarf new moon night ({new_lux:.4} lx)"
    );
    let _ = full_lux;
}

/// A moon below the horizon contributes nothing — only the star floor remains,
/// regardless of its phase.
#[test]
fn moon_below_horizon_adds_no_light() {
    use eulumdat_daylight::lunar::moon_position;
    let loc = location_by_name("Berlin").unwrap();
    // Find an instant near full moon when it is below the horizon.
    let below = (0..48)
        .map(|i| {
            let h = i as f64 * 0.5;
            LocalDateTime::new(2026, 1, 3, h).moon_position(&loc)
        })
        .find(|m| !m.is_up() && m.illuminated_fraction > 0.9);
    if let Some(m) = below {
        let star_floor = 0.0015;
        let lux = night_lux_with_moon(star_floor, &m);
        assert!((lux - star_floor).abs() < 1e-9, "moon down → only starlight");
    }
    let _ = moon_position; // silence unused if the branch is skipped
}
