//! eulumdat-rt CLI: GPU tracing, benchmarks, GPU vs CPU comparison.
//!
//! Usage:
//!   rt_bench bench                    # GPU vs CPU speed comparison
//!   rt_bench trace <input.ldt> [--cover <preset>] [--photons <n>]
//!   rt_bench compare <input.ldt> [--cover <preset>] [--photons <n>]

use eulumdat::{Eulumdat, PhotometricComparison};
use eulumdat_goniosim::*;
use eulumdat_rt::*;
use std::time::Instant;
use std::{env, fs, process};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    match args[1].as_str() {
        "bench" => cmd_bench(),
        "trace" => cmd_trace(&args[2..]),
        "compare" => cmd_compare(&args[2..]),
        "render" => cmd_render(&args[2..]),
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_usage();
            process::exit(1);
        }
    }
}

fn print_usage() {
    eprintln!("eulumdat-rt — GPU ray tracing engine\n");
    eprintln!("Usage:");
    eprintln!("  rt_bench bench");
    eprintln!("  rt_bench trace <input.ldt> [--cover <preset>] [--photons <n>]");
    eprintln!("  rt_bench compare <input.ldt> [--cover <preset>] [--photons <n>]");
    eprintln!("  rt_bench render [--cover <preset>] [--width <w>] [--height <h>] [--spp <n>] [-o output.ppm]");
}

fn load_ldt(path: &str) -> Eulumdat {
    let content = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("Cannot read {path}: {e}");
        process::exit(1);
    });
    Eulumdat::parse(&content)
        .or_else(|_| eulumdat::IesParser::parse(&content))
        .unwrap_or_else(|e| {
            eprintln!("Cannot parse {path}: {e}");
            process::exit(1);
        })
}

fn parse_flag(args: &[String], flag: &str, default: &str) -> String {
    for i in 0..args.len() {
        if args[i] == flag && i + 1 < args.len() {
            return args[i + 1].clone();
        }
    }
    default.to_string()
}

fn get_cover(name: &str) -> Option<MaterialParams> {
    match name {
        "" | "none" => None,
        "clear_pmma_3mm" => Some(catalog::clear_pmma_3mm()),
        "satin_pmma_3mm" => Some(catalog::satin_pmma_3mm()),
        "opal_pmma_3mm" => Some(catalog::opal_pmma_3mm()),
        "opal_light_pmma_3mm" => Some(catalog::opal_light_pmma_3mm()),
        "clear_glass_4mm" => Some(catalog::clear_glass_4mm()),
        "satin_glass_4mm" => Some(catalog::satin_glass_4mm()),
        "matte_black" => Some(catalog::matte_black()),
        other => {
            eprintln!("Unknown cover: {other}");
            process::exit(1);
        }
    }
}

// ============================================================================
// Benchmark
// ============================================================================

fn cmd_bench() {
    let tracer = pollster::block_on(GpuTracer::new()).expect("Failed to create GPU tracer");

    // Warm up
    let _ = pollster::block_on(tracer.trace_isotropic(1000, 10.0, 5.0));

    println!("=== Free Space (Isotropic) ===\n");
    println!("{:<12} {:>10} {:>10} {:>8}", "Photons", "GPU (ms)", "CPU (ms)", "Speedup");
    println!("{}", "-".repeat(44));

    for &n in &[100_000u32, 1_000_000, 10_000_000] {
        let gpu_start = Instant::now();
        let _ = pollster::block_on(tracer.trace_isotropic(n, 10.0, 5.0));
        let gpu_ms = gpu_start.elapsed().as_secs_f64() * 1000.0;

        let cpu_start = Instant::now();
        let scene = bare_isotropic(1000.0);
        let config = TracerConfig {
            num_photons: n as u64, detector_c_resolution: 10.0,
            detector_g_resolution: 5.0, seed: 42, ..TracerConfig::default()
        };
        let _ = eulumdat_goniosim::Tracer::trace(&scene, &config);
        let cpu_ms = cpu_start.elapsed().as_secs_f64() * 1000.0;

        println!("{:<12} {:>9.1} {:>9.1} {:>7.1}x",
            fmt_num(n), gpu_ms, cpu_ms, cpu_ms / gpu_ms);
    }

    println!("\n=== With Opal PMMA Cover ===\n");
    println!("{:<12} {:>10} {:>10} {:>8}", "Photons", "GPU (ms)", "CPU (ms)", "Speedup");
    println!("{}", "-".repeat(44));

    let cover = catalog::opal_pmma_3mm();
    let gpu_mat = GpuMaterial::from_material_params(&cover);
    let gpu_prim = GpuPrimitive::sheet(
        [0.0, 0.0, -0.04], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0],
        0.5, 0.5, 0.003, 0,
    );

    for &n in &[100_000u32, 1_000_000] {
        let gpu_start = Instant::now();
        let _ = pollster::block_on(tracer.trace_with_scene(
            n, 10.0, 5.0, SourceType::Isotropic, 1000.0,
            &[gpu_prim], &[gpu_mat],
        ));
        let gpu_ms = gpu_start.elapsed().as_secs_f64() * 1000.0;

        let cpu_start = Instant::now();
        let mut scene = Scene::new();
        scene.add_source(Source::Isotropic {
            position: nalgebra::Point3::origin(), flux_lm: 1000.0,
        });
        let mat_id = scene.add_material(cover.clone());
        scene.add_object(Primitive::Sheet {
            center: nalgebra::Point3::new(0.0, 0.0, -0.04),
            normal: nalgebra::Vector3::z_axis(),
            u_axis: nalgebra::Vector3::x_axis(),
            half_width: 0.5, half_height: 0.5, thickness: 0.003,
        }, mat_id, "cover");
        let config = TracerConfig {
            num_photons: n as u64, detector_c_resolution: 10.0,
            detector_g_resolution: 5.0, seed: 42, ..TracerConfig::default()
        };
        let _ = eulumdat_goniosim::Tracer::trace(&scene, &config);
        let cpu_ms = cpu_start.elapsed().as_secs_f64() * 1000.0;

        println!("{:<12} {:>9.1} {:>9.1} {:>7.1}x",
            fmt_num(n), gpu_ms, cpu_ms, cpu_ms / gpu_ms);
    }
}

// ============================================================================
// GPU trace with LDT input
// ============================================================================

fn cmd_trace(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: rt_bench trace <input.ldt> [--cover <preset>] [--photons <n>]");
        process::exit(1);
    }

    let ldt = load_ldt(&args[0]);
    let cover_name = parse_flag(args, "--cover", "");
    let num_photons: u32 = parse_flag(args, "--photons", "1000000").parse().unwrap_or(1_000_000);
    let cover = get_cover(&cover_name);

    let tracer = pollster::block_on(GpuTracer::new()).expect("Failed to create GPU tracer");

    println!("GPU trace: {} ({:.0} lm)", ldt.luminaire_name, ldt.total_luminous_flux());

    let (gpu_prims, gpu_mats) = if let Some(ref c) = cover {
        println!("Cover: {}", c.name);
        let mat = GpuMaterial::from_material_params(c);
        let prim = GpuPrimitive::sheet(
            [0.0, 0.0, -0.04], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0],
            0.5, 0.5, c.thickness_mm as f32 / 1000.0, 0,
        );
        (vec![prim], vec![mat])
    } else {
        println!("Cover: none");
        (vec![], vec![])
    };

    let start = Instant::now();
    let result = pollster::block_on(tracer.trace_with_scene(
        num_photons, 10.0, 5.0, SourceType::Isotropic, 1000.0,
        &gpu_prims, &gpu_mats,
    ));
    let elapsed = start.elapsed();

    let throughput = result.total_energy() / num_photons as f64;
    println!("Traced {} photons in {:.1}ms ({:.1}M photons/sec)",
        fmt_num(num_photons),
        elapsed.as_secs_f64() * 1000.0,
        num_photons as f64 / elapsed.as_secs_f64() / 1_000_000.0);
    println!("Energy throughput: {:.1}%", throughput * 100.0);
}

// ============================================================================
// GPU vs CPU comparison
// ============================================================================

fn cmd_compare(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: rt_bench compare <input.ldt> [--cover <preset>] [--photons <n>]");
        process::exit(1);
    }

    let ldt = load_ldt(&args[0]);
    let cover_name = parse_flag(args, "--cover", "");
    let num_photons: u32 = parse_flag(args, "--photons", "500000").parse().unwrap_or(500_000);
    let cover = get_cover(&cover_name);

    println!("=== GPU vs CPU Comparison ===\n");
    println!("Input: {} ({:.0} lm)", ldt.luminaire_name, ldt.total_luminous_flux());
    if let Some(ref c) = cover {
        println!("Cover: {}", c.name);
    }
    println!();

    // GPU
    let tracer = pollster::block_on(GpuTracer::new()).expect("GPU tracer");
    let (gpu_prims, gpu_mats) = if let Some(ref c) = cover {
        let mat = GpuMaterial::from_material_params(c);
        let prim = GpuPrimitive::sheet(
            [0.0, 0.0, -0.04], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0],
            0.5, 0.5, c.thickness_mm as f32 / 1000.0, 0,
        );
        (vec![prim], vec![mat])
    } else {
        (vec![], vec![])
    };

    let gpu_start = Instant::now();
    let gpu_result = pollster::block_on(tracer.trace_with_scene(
        num_photons, 10.0, 5.0, SourceType::Isotropic, 1000.0,
        &gpu_prims, &gpu_mats,
    ));
    let gpu_ms = gpu_start.elapsed().as_secs_f64() * 1000.0;
    let gpu_throughput = gpu_result.total_energy() / num_photons as f64;

    // CPU
    let mut cpu_scene = Scene::new();
    cpu_scene.add_source(Source::Isotropic {
        position: nalgebra::Point3::origin(), flux_lm: 1000.0,
    });
    if let Some(ref c) = cover {
        let mat_id = cpu_scene.add_material(c.clone());
        cpu_scene.add_object(Primitive::Sheet {
            center: nalgebra::Point3::new(0.0, 0.0, -0.04),
            normal: nalgebra::Vector3::z_axis(),
            u_axis: nalgebra::Vector3::x_axis(),
            half_width: 0.5, half_height: 0.5, thickness: c.thickness_mm / 1000.0,
        }, mat_id, "cover");
    }
    let cpu_config = TracerConfig {
        num_photons: num_photons as u64,
        detector_c_resolution: 10.0,
        detector_g_resolution: 5.0,
        seed: 42,
        ..TracerConfig::default()
    };
    let cpu_start = Instant::now();
    let cpu_result = eulumdat_goniosim::Tracer::trace(&cpu_scene, &cpu_config);
    let cpu_ms = cpu_start.elapsed().as_secs_f64() * 1000.0;
    let cpu_throughput = cpu_result.stats.total_energy_detected / cpu_result.stats.total_energy_emitted;

    // Compare candela at key angles
    let gpu_cd = gpu_result.to_candela(1000.0);
    let cpu_cd = cpu_result.detector.to_candela(1000.0);

    println!("{:<8} {:>10} {:>10}", "", "GPU", "CPU");
    println!("{}", "-".repeat(30));
    println!("{:<8} {:>9.1}ms {:>9.1}ms", "Time", gpu_ms, cpu_ms);
    println!("{:<8} {:>9.1}% {:>9.1}%", "Through", gpu_throughput * 100.0, cpu_throughput * 100.0);
    println!("{:<8} {:>9.1}x", "Speedup", cpu_ms / gpu_ms);
    println!();

    println!("Candela at key gamma angles (avg over C-planes):");
    println!("{:<8} {:>10} {:>10} {:>8}", "gamma", "GPU cd", "CPU cd", "ratio");
    println!("{}", "-".repeat(40));
    for gi in (0..gpu_cd[0].len()).step_by(3) {
        let g = gi as f64 * 5.0;
        let gpu_avg: f64 = gpu_cd.iter().map(|c| c[gi]).sum::<f64>() / gpu_cd.len() as f64;
        let cpu_avg: f64 = cpu_cd.iter().map(|c| c[gi]).sum::<f64>() / cpu_cd.len() as f64;
        let ratio = if cpu_avg > 0.1 { gpu_avg / cpu_avg } else { 0.0 };
        println!("{:>5.0}° {:>10.1} {:>10.1} {:>8.3}", g, gpu_avg, cpu_avg, ratio);
    }
}

fn fmt_num(n: u32) -> String {
    if n >= 1_000_000 { format!("{}M", n / 1_000_000) }
    else if n >= 1_000 { format!("{}K", n / 1_000) }
    else { n.to_string() }
}

// ============================================================================
// Render
// ============================================================================

fn cmd_render(args: &[String]) {
    let cover_name = parse_flag(args, "--cover", "opal_pmma_3mm");
    let width: u32 = parse_flag(args, "--width", "512").parse().unwrap_or(512);
    let height: u32 = parse_flag(args, "--height", "512").parse().unwrap_or(512);
    let spp: u32 = parse_flag(args, "--spp", "32").parse().unwrap_or(32);
    let output = parse_flag(args, "-o", "render.ppm");
    let exposure: f32 = parse_flag(args, "--exposure", "2.0").parse().unwrap_or(2.0);

    let cover = get_cover(&cover_name);

    println!("=== eulumdat-rt Render ===\n");
    println!("Resolution: {width}x{height}, SPP: {spp}");

    let camera = pollster::block_on(eulumdat_rt::GpuCamera::new()).expect("GPU camera");

    // Build scene: floor + cover + walls
    let mut prims = Vec::new();
    let mut mats = Vec::new();

    let room_w = 2.0f32;
    let room_h = 1.5f32;

    // Floor (y=0, white diffuse)
    mats.push(GpuMaterial {
        mtype: 1, _pad0: 0, _pad1: 0, _pad2: 0,
        reflectance: 0.75, ior: 1.0, transmittance: 0.0, min_reflectance: 0.0,
        absorption_coeff: 0.0, scattering_coeff: 0.0, asymmetry: 0.0, thickness: 0.0,
    });
    prims.push(GpuPrimitive::sheet(
        [0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0],
        room_w, room_w, 0.001, 0,
    ));

    // Ceiling (y=room_h)
    mats.push(GpuMaterial {
        mtype: 1, _pad0: 0, _pad1: 0, _pad2: 0,
        reflectance: 0.5, ior: 1.0, transmittance: 0.0, min_reflectance: 0.0,
        absorption_coeff: 0.0, scattering_coeff: 0.0, asymmetry: 0.0, thickness: 0.0,
    });
    prims.push(GpuPrimitive::sheet(
        [0.0, room_h, 0.0], [0.0, -1.0, 0.0], [1.0, 0.0, 0.0],
        room_w, room_w, 0.001, 1,
    ));

    // Back wall (z=-room_w)
    mats.push(GpuMaterial {
        mtype: 1, _pad0: 0, _pad1: 0, _pad2: 0,
        reflectance: 0.6, ior: 1.0, transmittance: 0.0, min_reflectance: 0.0,
        absorption_coeff: 0.0, scattering_coeff: 0.0, asymmetry: 0.0, thickness: 0.0,
    });
    prims.push(GpuPrimitive::sheet(
        [0.0, room_h * 0.5, -room_w], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0],
        room_w, room_h * 0.5, 0.001, 2,
    ));

    // Left wall (x=-room_w)
    prims.push(GpuPrimitive::sheet(
        [-room_w, room_h * 0.5, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0],
        room_w, room_h * 0.5, 0.001, 2,
    ));

    // Cover sheet near ceiling (if specified)
    if let Some(ref c) = cover {
        println!("Cover: {}", c.name);
        let cover_mat = GpuMaterial::from_material_params(c);
        let cover_mat_id = mats.len() as u32;
        mats.push(cover_mat);
        prims.push(GpuPrimitive::sheet(
            [0.0, room_h - 0.04, 0.0], [0.0, -1.0, 0.0], [1.0, 0.0, 0.0],
            0.4, 0.4, c.thickness_mm as f32 / 1000.0, cover_mat_id,
        ));
    } else {
        println!("Cover: none");
    }

    println!("Primitives: {}, Materials: {}", prims.len(), mats.len());
    println!();

    let denoise: u32 = parse_flag(args, "--denoise", "0").parse().unwrap_or(0);

    let start = Instant::now();
    // Source at ceiling (near cover position)
    let source_pos = [0.0, room_h - 0.04, 0.0];
    let mut image = pollster::block_on(camera.render(
        width, height, spp,
        [1.8, 0.8, 2.2],
        [0.0, 0.5, 0.0],
        55.0,
        &prims, &mats,
        500.0,
        source_pos,
    ));
    let render_ms = start.elapsed().as_secs_f64() * 1000.0;

    println!("Rendered in {:.1}ms ({:.1}M rays/sec)",
        render_ms,
        (width * height * spp) as f64 / (render_ms / 1000.0) / 1_000_000.0);

    if denoise > 0 {
        let dn_start = Instant::now();
        image.denoise(denoise);
        println!("Denoised (radius={denoise}) in {:.1}ms", dn_start.elapsed().as_secs_f64() * 1000.0);
    }

    // Write PPM
    let ppm_bytes: Vec<u8> = image.to_srgb_bytes_with_exposure(exposure)
        .chunks(4)
        .flat_map(|rgba| [rgba[0], rgba[1], rgba[2]])
        .collect();

    let mut data = format!("P6\n{} {}\n255\n", width, height).into_bytes();
    data.extend_from_slice(&ppm_bytes);

    fs::write(&output, &data).unwrap_or_else(|e| {
        eprintln!("Cannot write {output}: {e}");
        process::exit(1);
    });
    println!("Written: {output} ({} bytes)", data.len());
}
