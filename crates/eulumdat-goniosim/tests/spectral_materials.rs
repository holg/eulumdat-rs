//! S2 spectral materials end-to-end.
//!
//! 1. Sellmeier dispersion: a clear PMMA/glass cover bends blue more than red,
//!    so a broadband source picks up a chromatic signature through it.
//! 2. Beer–Lambert tint: a warm-white source behind an amber transmittance
//!    curve shifts the transmitted CCT downward (fewer blue photons survive).

use eulumdat_goniosim::*;
use eulumdat_spectrum::{Sellmeier, Spd};
use nalgebra::{Point3, Unit, Vector3};

/// A downward LED behind a flat cover at `distance_mm`, with the given
/// spectral override on the cover material.
fn led_through_cover(
    cct_k: f64,
    cover: MaterialParams,
    spectral: SpectralOverride,
) -> Scene {
    let mut scene = Scene::new();
    scene.add_source_with_spectrum(
        Source::Led {
            position: Point3::origin(),
            direction: Unit::new_unchecked(Vector3::new(0.0, 0.0, -1.0)),
            half_angle_deg: 20.0,
            flux_lm: 1000.0,
        },
        SourceSpectrum::from_cct(cct_k),
    );
    let mat = scene.add_material_spectral(cover, spectral);
    // Flat cover 40 mm below the LED, facing up.
    scene.add_object(
        eulumdat_goniosim::geometry::Primitive::Sheet {
            center: Point3::new(0.0, 0.0, -0.04),
            normal: Vector3::z_axis(),
            u_axis: Vector3::x_axis(),
            half_width: 0.1,
            half_height: 0.1,
            thickness: 0.003,
        },
        mat,
        "cover",
    );
    scene
}

fn cfg() -> TracerConfig {
    TracerConfig {
        num_photons: 300_000,
        max_bounces: 8,
        detector_c_resolution: 15.0,
        detector_g_resolution: 5.0,
        seed: 91,
        ..TracerConfig::default()
    }
}

/// An amber absorbing filter: transmits red fully, blocks blue. A step-ish
/// curve, 0.05 below 500 nm rising to 0.9 above 600 nm.
fn amber_filter() -> Spd {
    let wl: Vec<f64> = (380..=780).step_by(10).map(|w| w as f64).collect();
    let vals: Vec<f64> = wl
        .iter()
        .map(|&w| {
            if w < 500.0 {
                0.03
            } else if w > 600.0 {
                0.90
            } else {
                0.03 + (w - 500.0) / 100.0 * 0.87
            }
        })
        .collect();
    Spd::new(&wl, &vals)
}

#[test]
fn amber_filter_shifts_cct_downward() {
    // Same 5000 K source; one behind clear glass, one behind an amber filter.
    // The amber-filtered transmitted light must be warmer (lower CCT).
    let clear = MaterialParams {
        name: "clear".into(),
        reflectance_pct: 0.0,
        ior: 1.5,
        transmittance_pct: 90.0,
        thickness_mm: 3.0,
        diffusion_pct: 0.0,
    };
    let amber = clear.clone();

    let r_clear = Tracer::trace(
        &led_through_cover(5000.0, clear, SpectralOverride::default()),
        &cfg(),
    );
    let r_amber = Tracer::trace(
        &led_through_cover(
            5000.0,
            amber,
            SpectralOverride {
                dispersion: None,
                transmittance_curve: Some(amber_filter()),
            },
        ),
        &cfg(),
    );

    let cct_clear = r_clear
        .channels
        .unwrap()
        .integrated_colorimetry()
        .unwrap()
        .cct_k;
    let cct_amber = r_amber
        .channels
        .unwrap()
        .integrated_colorimetry()
        .unwrap()
        .cct_k;

    assert!(
        cct_amber < cct_clear - 300.0,
        "amber filter should warm the light substantially: clear {:.0} K vs amber {:.0} K",
        cct_clear,
        cct_amber
    );
    // Sanity: clear cover stays near the source colour.
    assert!(
        (cct_clear - 5000.0).abs() < 500.0,
        "clear cover should preserve ~5000 K, got {:.0}",
        cct_clear
    );
}

#[test]
fn sellmeier_ior_is_used_for_transmission() {
    // With a Sellmeier override, the cover uses n(λ); the trace must still
    // conserve energy and transmit light (a smoke test that the dispersion path
    // executes and does not, e.g., produce NaNs or total absorption).
    let clear = MaterialParams {
        name: "pmma".into(),
        reflectance_pct: 0.0,
        ior: 1.49,
        transmittance_pct: 92.0,
        thickness_mm: 3.0,
        diffusion_pct: 0.0,
    };
    let r = Tracer::trace(
        &led_through_cover(
            4000.0,
            clear,
            SpectralOverride {
                dispersion: Some(Sellmeier::PMMA),
                transmittance_curve: None,
            },
        ),
        &cfg(),
    );
    let col = r.channels.unwrap().integrated_colorimetry().unwrap();
    assert!(col.cct_k.is_finite() && col.cct_k > 2000.0 && col.cct_k < 10000.0);
    assert!(r.stats.photons_detected > 0);
}
