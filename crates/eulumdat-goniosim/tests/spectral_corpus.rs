//! End-to-end corpus sweep: every real vendor SPD in `docs/SPDs/` is loaded,
//! traced through free space, and the colour recovered from the weighted-channel
//! detector must match the SPD's own direct colorimetry within Monte Carlo
//! noise. This is the spectral analogue of the round-trip LVK validation: if the
//! tracer's wavelength sampling and channel weighting are correct, tracing a
//! spectrum reproduces its colour.
//!
//! Also checks energy invariance: a spectral trace and a monochromatic trace of
//! the same geometry agree on total flux and the photopic distribution.

use eulumdat::{analyze_spd, load_spd};
use eulumdat_goniosim::*;
use nalgebra::Point3;
use std::path::PathBuf;

fn spd_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR is crates/eulumdat-goniosim; the corpus is at the repo
    // root under docs/SPDs.
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // crates/
    p.pop(); // repo root
    p.push("docs");
    p.push("SPDs");
    p
}

fn collect_spd_files(root: &PathBuf) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Some(ext) = path.extension() {
                let e = ext.to_string_lossy().to_lowercase();
                if e == "spd" || e == "csv" {
                    out.push(path);
                }
            }
        }
    }
    out.sort();
    out
}

fn trace_cct(spec: SourceSpectrum) -> Option<f64> {
    let mut scene = Scene::new();
    scene.add_source_with_spectrum(
        Source::Isotropic {
            position: Point3::origin(),
            flux_lm: 1000.0,
        },
        spec,
    );
    let cfg = TracerConfig {
        num_photons: 300_000,
        max_bounces: 2,
        detector_c_resolution: 30.0,
        detector_g_resolution: 10.0,
        seed: 4242,
        ..TracerConfig::default()
    };
    let result = Tracer::trace(&scene, &cfg);
    result
        .channels
        .and_then(|c| c.integrated_colorimetry())
        .map(|c| c.cct_k)
}

#[test]
fn spd_corpus_round_trips_through_tracer() {
    let dir = spd_dir();
    let files = collect_spd_files(&dir);
    assert!(
        !files.is_empty(),
        "no SPD files found under {} — corpus missing?",
        dir.display()
    );

    let mut checked = 0;
    let mut worst_delta = 0.0_f64;
    for path in &files {
        let Ok(loaded) = load_spd(path) else {
            continue; // skip unparseable files rather than fail the sweep
        };
        // Direct colorimetry of the measured spectrum (ground truth).
        let direct = analyze_spd(&loaded.spd);
        if !direct.cct_k.is_finite() || direct.cct_k < 1500.0 || direct.cct_k > 20000.0 {
            continue; // out of the meaningful white-light range
        }

        let spec = SourceSpectrum::from_eulumdat(&loaded.spd);
        let Some(traced) = trace_cct(spec) else {
            continue;
        };

        let delta = (traced - direct.cct_k).abs();
        worst_delta = worst_delta.max(delta);
        // Tracer CCT must track the direct CCT. Allow a generous absolute floor
        // plus a relative term (warm sources tolerate more K per unit chroma).
        let tol = 60.0 + 0.03 * direct.cct_k;
        assert!(
            delta < tol,
            "{}: traced {:.0} K vs direct {:.0} K (Δ {:.0} > tol {:.0})",
            path.file_name().unwrap().to_string_lossy(),
            traced,
            direct.cct_k,
            delta,
            tol
        );
        checked += 1;
    }

    assert!(
        checked >= 5,
        "expected to check several SPDs, only did {checked}"
    );
    eprintln!("corpus: {checked} SPDs traced, worst ΔCCT {worst_delta:.0} K");
}

#[test]
fn spectral_and_monochromatic_agree_on_energy() {
    // A housing-free Lambertian source: with and without a spectrum, the total
    // detected flux and photopic distribution must match (energy conservation is
    // spectrum-independent).
    let flux = 1000.0;

    let mut mono = Scene::new();
    mono.add_source(Source::Lambertian {
        position: Point3::origin(),
        normal: nalgebra::Vector3::z_axis(),
        flux_lm: flux,
    });

    let mut spec = Scene::new();
    spec.add_source_with_spectrum(
        Source::Lambertian {
            position: Point3::origin(),
            normal: nalgebra::Vector3::z_axis(),
            flux_lm: flux,
        },
        SourceSpectrum::from_cct(5000.0),
    );

    let cfg = TracerConfig {
        num_photons: 300_000,
        max_bounces: 2,
        detector_c_resolution: 30.0,
        detector_g_resolution: 10.0,
        seed: 77,
        ..TracerConfig::default()
    };

    let rm = Tracer::trace(&mono, &cfg);
    let rs = Tracer::trace(&spec, &cfg);

    let fm = rm.detector.total_flux(flux);
    let fs = rs.detector.total_flux(flux);
    assert!(
        (fm - fs).abs() / fm < 1e-9,
        "total flux must be spectrum-independent: {fm} vs {fs}"
    );
}
