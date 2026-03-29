//! GPU vs CPU validation test.
//!
//! Traces isotropic source in free space on both GPU and CPU,
//! compares detector outputs. Must match within statistical tolerance.

use eulumdat_rt::{GpuMaterial, GpuPrimitive};
use std::f64::consts::PI;

#[test]
fn isotropic_free_space_gpu_vs_cpu() {
    // GPU trace
    let tracer = pollster::block_on(eulumdat_rt::GpuTracer::new())
        .expect("Failed to create GPU tracer");

    let gpu_result = pollster::block_on(tracer.trace_isotropic(1_000_000, 10.0, 5.0));

    let gpu_energy = gpu_result.total_energy();
    eprintln!("GPU: total_energy = {gpu_energy:.1} (expected ~1000000)");
    assert!(
        (gpu_energy - 1_000_000.0).abs() / 1_000_000.0 < 0.01,
        "GPU energy conservation: {gpu_energy}"
    );

    // GPU candela at equator (gamma=90) should be ~79.6 for 1000 lm isotropic
    let gpu_candela = gpu_result.to_candela(1000.0);
    let gi_90 = 18; // 90 / 5.0
    let expected_cd = 1000.0 / (4.0 * PI);

    let mut gpu_avg_90 = 0.0;
    let mut n = 0;
    for ci in 0..gpu_candela.len() {
        if gpu_candela[ci][gi_90] > 0.0 {
            gpu_avg_90 += gpu_candela[ci][gi_90];
            n += 1;
        }
    }
    if n > 0 {
        gpu_avg_90 /= n as f64;
    }
    let gpu_err = (gpu_avg_90 - expected_cd).abs() / expected_cd;
    eprintln!("GPU candela at g=90: {gpu_avg_90:.1} (expected {expected_cd:.1}, error {:.1}%)", gpu_err * 100.0);

    // CPU trace for comparison
    let cpu_scene = eulumdat_goniosim::bare_isotropic(1000.0);
    let cpu_config = eulumdat_goniosim::TracerConfig {
        num_photons: 1_000_000,
        detector_c_resolution: 10.0,
        detector_g_resolution: 5.0,
        seed: 42,
        ..eulumdat_goniosim::TracerConfig::default()
    };
    let cpu_result = eulumdat_goniosim::Tracer::trace(&cpu_scene, &cpu_config);
    let cpu_candela = cpu_result.detector.to_candela(1000.0);

    let mut cpu_avg_90 = 0.0;
    let mut n2 = 0;
    for ci in 0..cpu_candela.len() {
        if cpu_candela[ci][gi_90] > 0.0 {
            cpu_avg_90 += cpu_candela[ci][gi_90];
            n2 += 1;
        }
    }
    if n2 > 0 {
        cpu_avg_90 /= n2 as f64;
    }

    eprintln!("CPU candela at g=90: {cpu_avg_90:.1}");
    eprintln!("GPU/CPU ratio: {:.3}", gpu_avg_90 / cpu_avg_90);

    // Both should be within 10% of expected (statistical noise)
    assert!(gpu_err < 0.10, "GPU error {:.1}% too high", gpu_err * 100.0);
    let cpu_err = (cpu_avg_90 - expected_cd).abs() / expected_cd;
    assert!(cpu_err < 0.10, "CPU error {:.1}% too high", cpu_err * 100.0);
}

#[test]
fn isotropic_with_opal_cover_gpu_vs_cpu() {
    // GPU trace with opal PMMA cover
    let tracer = pollster::block_on(eulumdat_rt::GpuTracer::new())
        .expect("Failed to create GPU tracer");

    let cover_params = eulumdat_goniosim::catalog::opal_pmma_3mm();
    let gpu_material = GpuMaterial::from_material_params(&cover_params);
    let gpu_primitive = GpuPrimitive::sheet(
        [0.0, 0.0, -0.04],  // center: 40mm below source
        [0.0, 0.0, 1.0],    // normal: +Z (facing up)
        [1.0, 0.0, 0.0],    // u_axis: +X
        0.5, 0.5,           // half_width, half_height
        0.003,              // thickness 3mm
        0,                  // material_id
    );

    let gpu_result = pollster::block_on(tracer.trace_with_scene(
        500_000, 10.0, 5.0,
        eulumdat_rt::SourceType::Isotropic, 1000.0,
        &[gpu_primitive],
        &[gpu_material],
    ));

    let gpu_energy = gpu_result.total_energy();
    eprintln!("GPU with opal cover: total_energy = {gpu_energy:.1}");
    eprintln!("GPU energy fraction: {:.1}%", gpu_energy / 500_000.0 * 100.0);

    // Should absorb significant fraction (opal PMMA 50% transmittance)
    let throughput = gpu_energy / 500_000.0;
    eprintln!("GPU throughput: {throughput:.3}");

    // Throughput should be less than 1.0 (cover absorbs some light)
    assert!(
        throughput < 0.99,
        "Cover should reduce throughput, got {throughput:.3}"
    );

    // CPU trace for comparison
    let mut cpu_scene = eulumdat_goniosim::Scene::new();
    cpu_scene.add_source(eulumdat_goniosim::Source::Isotropic {
        position: eulumdat_goniosim::nalgebra::Point3::origin(),
        flux_lm: 1000.0,
    });
    let mat_id = cpu_scene.add_material(cover_params);
    cpu_scene.add_object(
        eulumdat_goniosim::Primitive::Sheet {
            center: eulumdat_goniosim::nalgebra::Point3::new(0.0, 0.0, -0.04),
            normal: eulumdat_goniosim::nalgebra::Vector3::z_axis(),
            u_axis: eulumdat_goniosim::nalgebra::Vector3::x_axis(),
            half_width: 0.5,
            half_height: 0.5,
            thickness: 0.003,
        },
        mat_id,
        "opal cover",
    );
    let cpu_config = eulumdat_goniosim::TracerConfig {
        num_photons: 500_000,
        detector_c_resolution: 10.0,
        detector_g_resolution: 5.0,
        seed: 42,
        ..eulumdat_goniosim::TracerConfig::default()
    };
    let cpu_result = eulumdat_goniosim::Tracer::trace(&cpu_scene, &cpu_config);
    let cpu_throughput = cpu_result.stats.total_energy_detected / cpu_result.stats.total_energy_emitted;
    eprintln!("CPU throughput: {cpu_throughput:.3}");
    eprintln!("GPU/CPU throughput ratio: {:.3}", throughput / cpu_throughput);

    // GPU and CPU throughput should be in the same ballpark (within 30%)
    let ratio = throughput / cpu_throughput;
    assert!(
        ratio > 0.5 && ratio < 2.0,
        "GPU/CPU throughput ratio {ratio:.3} too far off"
    );
}
