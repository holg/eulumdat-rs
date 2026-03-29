//! Benchmark: GPU vs CPU photon tracing.
//!
//! Usage: cargo run -p eulumdat-rt --bin rt_bench --release

use eulumdat_rt::*;
use std::time::Instant;

fn main() {
    env_logger::init();

    println!("=== eulumdat-rt GPU vs CPU Benchmark ===\n");

    let tracer = pollster::block_on(GpuTracer::new()).expect("Failed to create GPU tracer");

    // Warm up GPU
    let _ = pollster::block_on(tracer.trace_isotropic(1000, 10.0, 5.0));

    let photon_counts = [100_000u32, 1_000_000, 10_000_000];

    println!("{:<12} {:>10} {:>10} {:>8}", "Photons", "GPU (ms)", "CPU (ms)", "Speedup");
    println!("{}", "-".repeat(44));

    for &n in &photon_counts {
        // GPU
        let gpu_start = Instant::now();
        let gpu_result = pollster::block_on(tracer.trace_isotropic(n, 10.0, 5.0));
        let gpu_ms = gpu_start.elapsed().as_secs_f64() * 1000.0;

        // CPU
        let cpu_start = Instant::now();
        let cpu_scene = eulumdat_goniosim::bare_isotropic(1000.0);
        let cpu_config = eulumdat_goniosim::TracerConfig {
            num_photons: n as u64,
            detector_c_resolution: 10.0,
            detector_g_resolution: 5.0,
            seed: 42,
            ..eulumdat_goniosim::TracerConfig::default()
        };
        let cpu_result = eulumdat_goniosim::Tracer::trace(&cpu_scene, &cpu_config);
        let cpu_ms = cpu_start.elapsed().as_secs_f64() * 1000.0;

        let speedup = cpu_ms / gpu_ms;

        println!("{:<12} {:>9.1} {:>9.1} {:>7.1}x",
            format_num(n), gpu_ms, cpu_ms, speedup);
    }

    println!();

    // Opal cover benchmark
    println!("--- With Opal PMMA 3mm Cover ---\n");
    println!("{:<12} {:>10} {:>10} {:>8}", "Photons", "GPU (ms)", "CPU (ms)", "Speedup");
    println!("{}", "-".repeat(44));

    let cover = eulumdat_goniosim::catalog::opal_pmma_3mm();
    let gpu_mat = GpuMaterial::from_material_params(&cover);
    let gpu_prim = GpuPrimitive::sheet(
        [0.0, 0.0, -0.04], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0],
        0.5, 0.5, 0.003, 0,
    );

    for &n in &[100_000u32, 1_000_000] {
        // GPU
        let gpu_start = Instant::now();
        let _ = pollster::block_on(tracer.trace_with_scene(
            n, 10.0, 5.0, SourceType::Isotropic, 1000.0,
            &[gpu_prim], &[gpu_mat],
        ));
        let gpu_ms = gpu_start.elapsed().as_secs_f64() * 1000.0;

        // CPU
        let cpu_start = Instant::now();
        let mut cpu_scene = eulumdat_goniosim::Scene::new();
        cpu_scene.add_source(eulumdat_goniosim::Source::Isotropic {
            position: eulumdat_goniosim::nalgebra::Point3::origin(),
            flux_lm: 1000.0,
        });
        let mat_id = cpu_scene.add_material(cover.clone());
        cpu_scene.add_object(
            eulumdat_goniosim::Primitive::Sheet {
                center: eulumdat_goniosim::nalgebra::Point3::new(0.0, 0.0, -0.04),
                normal: eulumdat_goniosim::nalgebra::Vector3::z_axis(),
                u_axis: eulumdat_goniosim::nalgebra::Vector3::x_axis(),
                half_width: 0.5, half_height: 0.5, thickness: 0.003,
            },
            mat_id, "cover",
        );
        let cpu_config = eulumdat_goniosim::TracerConfig {
            num_photons: n as u64,
            detector_c_resolution: 10.0,
            detector_g_resolution: 5.0,
            seed: 42,
            ..eulumdat_goniosim::TracerConfig::default()
        };
        let _ = eulumdat_goniosim::Tracer::trace(&cpu_scene, &cpu_config);
        let cpu_ms = cpu_start.elapsed().as_secs_f64() * 1000.0;

        let speedup = cpu_ms / gpu_ms;
        println!("{:<12} {:>9.1} {:>9.1} {:>7.1}x",
            format_num(n), gpu_ms, cpu_ms, speedup);
    }
}

fn format_num(n: u32) -> String {
    if n >= 1_000_000 { format!("{}M", n / 1_000_000) }
    else if n >= 1_000 { format!("{}K", n / 1_000) }
    else { n.to_string() }
}
