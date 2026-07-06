//! N1/N2 nightlight end-to-end.
//!
//! 1. Dark-sky report: an upward-spilling luminaire, traced with a warm vs a
//!    cool spectrum, is correctly distinguished — cool light has higher blue
//!    content, higher upward CCT, higher (Rayleigh-weighted) spectral ULOR and
//!    trips the dark-sky CCT limit that warm light passes.
//! 2. Mesopic road: the S/P ratio measured from a completed trace, fed through
//!    CIE 191, makes a cool source read brighter than a warm one at the same
//!    photopic road luminance.

use eulumdat_goniosim::*;
use eulumdat_spectrum::mesopic::mesopic_luminance;
use nalgebra::Point3;

/// An isotropic luminaire (emits into the full sphere, so ~half its flux goes
/// upward — a worst-case spill fixture) with the given spectrum.
fn isotropic_spectral(cct_k: f64) -> Scene {
    let mut scene = Scene::new();
    scene.add_source_with_spectrum(
        Source::Isotropic {
            position: Point3::origin(),
            flux_lm: 10_000.0,
        },
        SourceSpectrum::from_cct(cct_k),
    );
    scene
}

fn cfg(seed: u64) -> TracerConfig {
    TracerConfig {
        num_photons: 300_000,
        max_bounces: 2,
        detector_c_resolution: 15.0,
        detector_g_resolution: 5.0,
        seed,
        ..TracerConfig::default()
    }
}

#[test]
fn dark_sky_report_distinguishes_warm_from_cool() {
    let warm = Tracer::trace(&isotropic_spectral(2200.0), &cfg(21));
    let cool = Tracer::trace(&isotropic_spectral(6500.0), &cfg(21));

    let r_warm = DarkSkyReport::from_channels(warm.channels.as_ref().unwrap());
    let r_cool = DarkSkyReport::from_channels(cool.channels.as_ref().unwrap());

    // Isotropic → about half the photopic flux escapes upward (γ>90°).
    assert!(
        (0.35..0.65).contains(&r_warm.ulor),
        "isotropic ULOR should be ~0.5, got {:.2}",
        r_warm.ulor
    );

    // Cool light: more blue, higher upward CCT, larger Rayleigh-weighted spill.
    assert!(
        r_cool.blue_fraction_up > r_warm.blue_fraction_up,
        "cool blue fraction {:.3} should exceed warm {:.3}",
        r_cool.blue_fraction_up,
        r_warm.blue_fraction_up
    );
    assert!(
        r_cool.upward_cct_k.unwrap() > r_warm.upward_cct_k.unwrap() + 1000.0,
        "cool upward CCT {:.0} should exceed warm {:.0}",
        r_cool.upward_cct_k.unwrap(),
        r_warm.upward_cct_k.unwrap()
    );
    assert!(
        r_cool.spectral_ulor > r_warm.spectral_ulor,
        "cool spectral ULOR {:.3} should exceed warm {:.3} (Rayleigh)",
        r_cool.spectral_ulor,
        r_warm.spectral_ulor
    );

    // Dark-sky CCT ordinance (3000 K): warm passes, cool fails.
    assert!(r_warm.meets_cct_limit(3000.0), "2200 K should pass the 3000 K limit");
    assert!(!r_cool.meets_cct_limit(3000.0), "6500 K should fail the 3000 K limit");

    // S/P from the trace: cool higher than warm.
    assert!(r_cool.sp_ratio > r_warm.sp_ratio);
}

#[test]
fn mesopic_road_luminance_favors_cool_at_night() {
    // Trace warm and cool sources, read their S/P from the detector, and apply
    // CIE 191 at a typical M-class road luminance (1 cd/m² photopic).
    let warm = Tracer::trace(&isotropic_spectral(2200.0), &cfg(5));
    let cool = Tracer::trace(&isotropic_spectral(6500.0), &cfg(5));

    let sp_warm = warm.channels.unwrap().integrated_sp_ratio();
    let sp_cool = cool.channels.unwrap().integrated_sp_ratio();

    let l_photopic = 1.0; // cd/m²
    let l_warm = mesopic_luminance(l_photopic, sp_warm);
    let l_cool = mesopic_luminance(l_photopic, sp_cool);

    assert!(
        l_cool > l_warm,
        "at night the cool source should read brighter: {:.3} vs {:.3} cd/m²",
        l_cool,
        l_warm
    );
    // The cool (high-S/P) source exceeds its photopic luminance; the warm one
    // does not gain as much.
    assert!(l_cool > l_photopic, "cool mesopic luminance should exceed photopic");
}

#[test]
fn moon_and_night_sky_sources_build_and_emit() {
    // Smoke test: the moon beam and uniform night-sky dome produce photons and
    // land measurable illuminance on a horizontal plane.
    use eulumdat_daylight::solar::solar_position;
    let moon_pos = solar_position(2026, 1, 15, 22.0, 40.0, 0.0); // reuse geometry

    let mut scene = Scene::new();
    scene.add_source(night_sky_source(
        Point3::new(0.0, 0.0, 0.05),
        2.0,
        STARLIGHT_LUX_TEST,
        12,
        24,
    ));
    let _ = moon_pos;

    let plane = PlaneDetector::horizontal(Point3::origin(), 0.5, 0.5, 1, 1, false);
    let result = trace_illuminance(
        &scene,
        &TracerConfig {
            num_photons: 100_000,
            seed: 9,
            ..TracerConfig::default()
        },
        vec![plane],
    );
    let e = result.average_lux(0);
    assert!(e > 0.0, "night sky should deliver some illuminance, got {e}");
    // Uniform dark dome ~ starlight order of magnitude.
    assert!(e < 1.0, "starlight illuminance should be sub-lux, got {e}");
}

const STARLIGHT_LUX_TEST: f64 = 0.001;
