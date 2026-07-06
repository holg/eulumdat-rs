//! D1/D2 daylight end-to-end.
//!
//! 1. Open-field sky dome: an unobstructed horizontal plane under a sky dome
//!    integrates to the diffuse horizontal illuminance (DHI) the dome was
//!    normalised to — the core sanity check for the SkyDome source + plane
//!    detector.
//! 2. Sun beam: a horizontal plane under the sun disc receives DNI·sin(altitude).
//! 3. Window daylight factor: a room lit only through a roof opening has a
//!    plausible daylight factor (interior << exterior, and > 0).

use eulumdat_daylight::availability::DaylightAvailability;
use eulumdat_daylight::sky::{PerezSky, SkyParams, SkyRadiance};
use eulumdat_daylight::solar::solar_position;
use eulumdat_goniosim::*;
use nalgebra::{Point3, Unit, Vector3};

fn cfg(n: u64, seed: u64) -> TracerConfig {
    TracerConfig {
        num_photons: n,
        max_bounces: 30,
        seed,
        ..TracerConfig::default()
    }
}

#[test]
fn open_field_plane_integrates_to_dhi() {
    // Clear-ish sky at Madrid summer noon; normalise the dome to its DHI.
    let sun = solar_position(2026, 6, 21, 12.0, 40.4, -3.7);
    let avail = DaylightAvailability::clear_sky(&sun, 2.5);
    let perez = PerezSky::new(sun, SkyParams::from_turbidity(2.5));
    let radiance = SkyRadiance::perez_from_dhi(perez, avail.dhi_lux);

    // A dome delivering `dhi_lux`. Emit over a square wider than the collector
    // so the small central sensor sits in a uniformly-lit field (drifted edge
    // photons are compensated). Emit from just above the z=0 plane.
    let collector_half = 0.5; // 1 m × 1 m sensor
    let emit_half = 2.0; // 4 m × 4 m emission footprint

    let dome = SkyDomeSource::with_extent(
        &radiance,
        Point3::new(0.0, 0.0, 0.02),
        1.0, // placeholder flux, set below
        emit_half,
        24,
        48,
    );
    let flux = dome.collector_flux(avail.dhi_lux);
    let dome = dome.with_flux(flux);

    let mut scene = Scene::new();
    scene.add_source(Source::SkyDome(dome));

    // Horizontal sensor at z=0 sized to the collector.
    let plane = PlaneDetector::horizontal(
        Point3::origin(),
        collector_half,
        collector_half,
        1,
        1,
        false,
    );

    let result = trace_illuminance(&scene, &cfg(400_000, 1), vec![plane]);
    let e = result.average_lux(0);

    // The plane should read back the DHI the dome was normalised to. This
    // closes the loop: SkyRadiance → dome CDF → photons → plane → lux.
    let rel = (e - avail.dhi_lux).abs() / avail.dhi_lux;
    assert!(
        rel < 0.06,
        "open-field plane {:.0} lx should match DHI {:.0} lx (rel {:.3})",
        e,
        avail.dhi_lux,
        rel
    );
}

#[test]
fn sun_beam_gives_dni_times_sin_altitude() {
    let sun = solar_position(2026, 6, 21, 12.0, 40.4, -3.7);
    let avail = DaylightAvailability::clear_sky(&sun, 2.5);

    // Sun beam over a 4 m² emission square, sensor 1 m² in the interior, emitted
    // from just above the plane so the parallel beam lands on the footprint.
    // Illuminance on a horizontal plane = DNI·sin(alt); size the emitted flux to
    // DNI·sin(alt)·A_emit so the sensor reads DNI·sin(alt).
    let sensor_half = 0.5;
    let emit_half = 2.0;
    let emit_area = (2.0 * emit_half) * (2.0 * emit_half);
    let sin_alt = sun.altitude_rad.sin();
    let expected_lux = avail.dni_lux * sin_alt;
    // DNI is the normal illuminance; total flux the beam carries across the
    // horizontal emission footprint is DNI·A_emit. The plane's cosθ (= sinα)
    // then yields DNI·sinα on the sensor.
    let flux_lm = avail.dni_lux * emit_area;

    let mut scene = Scene::new();
    scene.add_source(sun_source(
        &sun,
        Point3::new(0.0, 0.0, 0.05),
        emit_half,
        flux_lm,
    ));

    let plane = PlaneDetector::horizontal(Point3::origin(), sensor_half, sensor_half, 1, 1, false);
    let result = trace_illuminance(&scene, &cfg(200_000, 2), vec![plane]);
    let e = result.average_lux(0);

    let rel = (e - expected_lux).abs() / expected_lux;
    assert!(
        rel < 0.05,
        "sun-lit plane {:.0} lx should match DNI·sin(alt) {:.0} lx (rel {:.3})",
        e,
        expected_lux,
        rel
    );
}

#[test]
fn room_with_roof_opening_has_plausible_daylight_factor() {
    // A closed box room, floor at z=0, roof at z=3, walls 4×4. The roof has a
    // small opening (a smaller emitting sky patch above it). We approximate the
    // opening by placing the sky-dome emitter just above the roof hole and
    // black (absorbing) walls/floor so only geometry shapes the result.
    let sun = solar_position(2026, 6, 21, 12.0, 40.4, -3.7);
    let overcast = DaylightAvailability::overcast(&sun);
    let perez = PerezSky::new(sun, SkyParams::overcast());
    let radiance = SkyRadiance::perez_from_dhi(perez, overcast.dhi_lux);

    // Opening 1 m² at roof centre; flux through it = DHI × opening area.
    let opening_half = 0.5;
    let opening_area = (2.0 * opening_half) * (2.0 * opening_half);
    let flux_lm = overcast.dhi_lux * opening_area;

    let mut scene = Scene::new();
    // Emit the sky through the roof opening, from just above the roof.
    scene.add_source(Source::SkyDome(
        SkyDomeSource::new(&radiance, Point3::new(0.0, 0.0, 3.1), flux_lm, 16, 32)
            .with_spectrum(SourceSpectrum::from_cct(6500.0)),
    ));

    // Grey floor and walls (some inter-reflection) via a diffuse reflector.
    let wall = scene.add_material(MaterialParams {
        name: "grey wall".into(),
        reflectance_pct: 50.0,
        ior: 0.0,
        transmittance_pct: 0.0,
        thickness_mm: 0.0,
        diffusion_pct: 100.0,
    });
    // Four walls + floor as sheets forming a 4×4×3 box (roof open).
    let w = 2.0; // half-extent 2 m → 4 m walls
    scene.add_object(
        Primitive::Sheet {
            center: Point3::new(0.0, 0.0, 0.0),
            normal: Vector3::z_axis(),
            u_axis: Vector3::x_axis(),
            half_width: w,
            half_height: w,
            thickness: 0.01,
        },
        wall,
        "floor",
    );

    let interior = PlaneDetector::horizontal(Point3::new(0.0, 0.0, 0.01), w, w, 4, 4, false);
    let result = trace_illuminance(&scene, &cfg(500_000, 3), vec![interior]);

    let e_in = result.average_lux(0);
    let df = daylight_factor_grid(
        &result.planes[0],
        result.total_flux_lm,
        result.total_emitted_energy,
        overcast.dhi_lux,
    );
    let df_avg: f64 =
        df.iter().flatten().sum::<f64>() / (df.len() * df[0].len()) as f64;

    // The interior gets some light but far less than the exterior DHI.
    assert!(e_in > 0.0, "room floor should receive some daylight");
    assert!(
        e_in < overcast.dhi_lux,
        "interior {:.0} lx must be below exterior DHI {:.0} lx",
        e_in,
        overcast.dhi_lux
    );
    // Daylight factor for a small roof opening: a few percent, generously bounded.
    assert!(
        (0.05..50.0).contains(&df_avg),
        "daylight factor {:.2}% out of plausible band",
        df_avg
    );
}
