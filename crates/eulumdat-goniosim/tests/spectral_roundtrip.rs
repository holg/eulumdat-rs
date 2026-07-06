//! S1 spectral trunk end-to-end: emit a known spectrum, trace it through free
//! space, and recover the colour from the weighted-channel detector.
//!
//! These are the load-bearing guarantees of the spectral tracer:
//! 1. A synthesized CCT round-trips through the full MC pipeline within ±40 K.
//! 2. Adding a spectrum does not change the photopic (Y) distribution — the LDT
//!    channel is invariant to whether we carried wavelengths.
//! 3. Warm vs cool spectra land on the correct side of the mesopic S/P divide.

use eulumdat_goniosim::*;
use nalgebra::{Point3, Vector3};

fn free_space_scene(cct_k: f64, flux_lm: f64) -> Scene {
    let mut scene = Scene::new();
    scene.add_source_with_spectrum(
        Source::Isotropic {
            position: Point3::origin(),
            flux_lm,
        },
        SourceSpectrum::from_cct(cct_k),
    );
    scene
}

fn trace_cfg(seed: u64) -> TracerConfig {
    TracerConfig {
        num_photons: 400_000,
        max_bounces: 4,
        detector_c_resolution: 15.0,
        detector_g_resolution: 5.0,
        seed,
        ..TracerConfig::default()
    }
}

#[test]
fn synthesized_3000k_round_trips_through_tracer() {
    let scene = free_space_scene(3000.0, 1000.0);
    let result = Tracer::trace(&scene, &trace_cfg(11));

    let channels = result
        .channels
        .expect("spectral scene must produce weighted channels");
    let col = channels
        .integrated_colorimetry()
        .expect("light was collected");

    assert!(
        (col.cct_k - 3000.0).abs() < 40.0,
        "3000 K spectrum should recover ~3000 K through the tracer, got {:.0} K",
        col.cct_k
    );
    // Warm white sits just below the Planckian locus (slightly negative Duv is
    // fine); |Duv| should be small.
    assert!(col.duv.abs() < 0.01, "warm white Duv should be small, got {}", col.duv);
}

#[test]
fn spectrum_does_not_change_photopic_distribution() {
    // Same isotropic source, same seed, with and without a spectrum. The
    // photopic (radiant-proportional) detector should match within MC noise:
    // isotropic emission is wavelength-independent, so carrying wavelengths must
    // not perturb where energy lands.
    let flux = 1000.0;

    let mut mono = Scene::new();
    mono.add_source(Source::Isotropic {
        position: Point3::origin(),
        flux_lm: flux,
    });

    let spectral = free_space_scene(4000.0, flux);

    let cfg = trace_cfg(7);
    let r_mono = Tracer::trace(&mono, &cfg);
    let r_spec = Tracer::trace(&spectral, &cfg);

    // Total detected flux equal (energy conservation, spectrum-independent).
    let f_mono = r_mono.detector.total_flux(flux);
    let f_spec = r_spec.detector.total_flux(flux);
    assert!(
        (f_mono - f_spec).abs() / f_mono < 1e-9,
        "total flux must be spectrum-independent: {} vs {}",
        f_mono,
        f_spec
    );

    // Isotropic → every gamma bin roughly equal candela in both runs. Compare
    // the candela at nadir; both should be near flux/(4π) per steradian scaled.
    let cd_mono = r_mono.detector.candela_at(0.0, 90.0, flux);
    let cd_spec = r_spec.detector.candela_at(0.0, 90.0, flux);
    let rel = (cd_mono - cd_spec).abs() / cd_mono.max(1e-9);
    assert!(
        rel < 0.05,
        "photopic distribution should be unchanged by spectrum (rel diff {:.3})",
        rel
    );
}

#[test]
fn warm_and_cool_straddle_the_mesopic_divide() {
    let warm = Tracer::trace(&free_space_scene(2700.0, 1000.0), &trace_cfg(3));
    let cool = Tracer::trace(&free_space_scene(6500.0, 1000.0), &trace_cfg(3));

    let sp_warm = warm.channels.unwrap().integrated_sp_ratio();
    let sp_cool = cool.channels.unwrap().integrated_sp_ratio();

    assert!(
        sp_cool > sp_warm,
        "cool light must have higher S/P than warm: {:.2} vs {:.2}",
        sp_cool,
        sp_warm
    );
    assert!(sp_warm > 1.0 && sp_warm < 1.7, "warm S/P envelope, got {:.2}", sp_warm);
    assert!(sp_cool > 1.8 && sp_cool < 2.7, "cool S/P envelope, got {:.2}", sp_cool);
}
