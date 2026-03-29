//! GPU vs CPU validation test.
//!
//! Traces isotropic source in free space on both GPU and CPU,
//! compares detector outputs. Must match within statistical tolerance.

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
