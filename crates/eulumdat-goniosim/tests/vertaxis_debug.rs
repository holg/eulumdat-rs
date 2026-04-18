use eulumdat::{Eulumdat, PhotometricSummary};
use eulumdat_goniosim::*;

#[test]
fn vertaxis_flux_debug() {
    let content = include_str!("../../eulumdat-wasm/templates/wiki-batwing.ldt");
    let ldt = Eulumdat::parse(content).unwrap();
    let lamp_flux = ldt.total_luminous_flux();
    let lor = ldt.light_output_ratio / 100.0;
    let flux = lamp_flux * lor;

    eprintln!("Original LDT:");
    eprintln!(
        "  symmetry: {:?}, num_c: {}, num_g: {}",
        ldt.symmetry, ldt.num_c_planes, ldt.num_g_planes
    );
    eprintln!("  c_angles: {:?}", ldt.c_angles);
    eprintln!("  lamp_flux: {lamp_flux}, LOR: {lor}, luminaire_flux: {flux}");
    let orig_summary = PhotometricSummary::from_eulumdat(&ldt);
    eprintln!("  calculated_flux: {:.1}", orig_summary.calculated_flux);
    eprintln!(
        "  intensities[0] (first few): {:?}",
        &ldt.intensities[0][..5.min(ldt.intensities[0].len())]
    );

    // Trace
    let mut scene = Scene::new();
    scene.add_source(Source::from_lvk(
        nalgebra::Point3::origin(),
        nalgebra::Rotation3::identity(),
        ldt.clone(),
        flux,
    ));
    let config = TracerConfig {
        num_photons: 1_000_000,
        detector_c_resolution: 15.0,
        detector_g_resolution: 5.0,
        seed: 42,
        ..TracerConfig::default()
    };
    let result = Tracer::trace(&scene, &config);

    // Export as Symmetry::None (24 C-planes)
    let cfg_none = ExportConfig {
        c_step_deg: 15.0,
        g_step_deg: 5.0,
        symmetry: None,
        luminaire_name: "test_none".into(),
        ..ExportConfig::default()
    };
    let mut ldt_none =
        detector_to_eulumdat_with_lamp_flux(&result.detector, flux, lamp_flux, &cfg_none);
    ldt_none.lamp_sets = ldt.lamp_sets.clone();
    let sum_none = PhotometricSummary::from_eulumdat(&ldt_none);
    eprintln!("\nExport as Symmetry::None:");
    eprintln!(
        "  num_c: {}, symmetry: {:?}",
        ldt_none.num_c_planes, ldt_none.symmetry
    );
    eprintln!("  calculated_flux: {:.1}", sum_none.calculated_flux);
    eprintln!(
        "  intensities[0] (first few): {:?}",
        &ldt_none.intensities[0][..5.min(ldt_none.intensities[0].len())]
    );

    // Export as VerticalAxis (1 C-plane)
    let cfg_vert = ExportConfig {
        c_step_deg: 15.0,
        g_step_deg: 5.0,
        symmetry: Some(eulumdat::Symmetry::VerticalAxis),
        luminaire_name: "test_vert".into(),
        ..ExportConfig::default()
    };
    let mut ldt_vert =
        detector_to_eulumdat_with_lamp_flux(&result.detector, flux, lamp_flux, &cfg_vert);
    ldt_vert.lamp_sets = ldt.lamp_sets.clone();
    let sum_vert = PhotometricSummary::from_eulumdat(&ldt_vert);
    eprintln!("\nExport as VerticalAxis:");
    eprintln!(
        "  num_c: {}, symmetry: {:?}",
        ldt_vert.num_c_planes, ldt_vert.symmetry
    );
    eprintln!("  calculated_flux: {:.1}", sum_vert.calculated_flux);
    eprintln!(
        "  intensities[0] (first few): {:?}",
        &ldt_vert.intensities[0][..5.min(ldt_vert.intensities[0].len())]
    );

    // Raw candela from detector
    let raw_cd = result.detector.to_candela(flux);
    eprintln!("\nRaw candela at g=40 (to_candela({flux})):");
    let gi40 = (40.0 / 5.0) as usize;
    let cd_avg: f64 = raw_cd.iter().map(|c| c[gi40]).sum::<f64>() / raw_cd.len() as f64;
    let expected_cd = 780.0 * lamp_flux / 1000.0; // 780 cdklm * 4/1 = 3120 cd
    eprintln!(
        "  avg over C-planes: {:.1} cd (expected {:.1} cd, ratio {:.3})",
        cd_avg,
        expected_cd,
        cd_avg / expected_cd
    );
    eprintln!("  scale factor 1000/lamp_flux = {:.4}", 1000.0 / lamp_flux);
    eprintln!(
        "  cd_avg * scale = {:.1} cd/klm (expected 780)",
        cd_avg * 1000.0 / lamp_flux
    );

    // Compare the intensity values
    eprintln!("\nIntensity comparison at C0:");
    eprintln!("  gamma | original | sym_none | sym_vert | none/orig | vert/orig");
    for gi in 0..ldt.intensities[0]
        .len()
        .min(ldt_none.intensities[0].len())
        .min(ldt_vert.intensities[0].len())
    {
        let g = gi as f64 * 5.0;
        let orig = ldt.intensities[0][gi];
        let none = ldt_none.intensities[0][gi];
        let vert = ldt_vert.intensities[0][gi];
        eprintln!(
            "  {:5.0} | {:8.1} | {:8.1} | {:8.1} | {:9.3} | {:9.3}",
            g,
            orig,
            none,
            vert,
            none / orig.max(0.01),
            vert / orig.max(0.01)
        );
    }
}
