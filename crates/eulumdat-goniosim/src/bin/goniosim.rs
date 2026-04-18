//! CLI for eulumdat-goniosim: trace photons, compare LDTs.
//!
//! Usage:
//!   goniosim trace input.ldt -o output.ldt [--cover opal_pmma_3mm] [--photons 1000000]
//!   goniosim roundtrip input.ldt                  # trace through free space, compare
//!   goniosim compare a.ldt b.ldt                  # compare two files

use eulumdat::{Eulumdat, PhotometricComparison};
use eulumdat_goniosim::*;
use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    match args[1].as_str() {
        "trace" => cmd_trace(&args[2..]),
        "roundtrip" => cmd_roundtrip(&args[2..]),
        "compare" => cmd_compare(&args[2..]),
        "catalog" => cmd_catalog(),
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_usage();
            process::exit(1);
        }
    }
}

fn print_usage() {
    eprintln!("eulumdat-goniosim CLI — CIE 171:2006 validated Monte Carlo tracer\n");
    eprintln!("Usage:");
    eprintln!("  goniosim trace <input.ldt> [-o output.ldt] [--cover <preset>] [--distance <mm>] [--photons <n>]");
    eprintln!("  goniosim roundtrip <input.ldt> [--photons <n>]");
    eprintln!("  goniosim compare <a.ldt> <b.ldt>");
    eprintln!("  goniosim catalog");
    eprintln!();
    eprintln!("Cover presets: clear_pmma_3mm, satin_pmma_3mm, opal_pmma_3mm, clear_glass_4mm, ...");
    eprintln!("Use 'goniosim catalog' to list all materials.");
}

fn load_ldt(path: &str) -> Eulumdat {
    let content = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("Cannot read {path}: {e}");
        process::exit(1);
    });
    // Try LDT first, then IES
    Eulumdat::parse(&content)
        .or_else(|_| eulumdat::IesParser::parse(&content))
        .unwrap_or_else(|e| {
            eprintln!("Cannot parse {path}: {e}");
            process::exit(1);
        })
}

fn cmd_roundtrip(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: goniosim roundtrip <input.ldt> [--photons <n>]");
        process::exit(1);
    }

    let input_path = &args[0];
    let num_photons = parse_flag_u64(args, "--photons", 2_000_000);

    let ldt = load_ldt(input_path);
    let lamp_flux = ldt.total_luminous_flux();
    // Use the calculated flux from intensity integration — this is the actual
    // luminous output implied by the cd/klm data, which may differ from
    // lamp_flux * LOR for some files.
    // calculated_luminous_flux returns the integrated flux from intensity data.
    // This is the value to use as source flux for FromLvk — it represents the
    // actual luminous output encoded in the cd/klm distribution.
    let calculated_flux = eulumdat::PhotometricCalculations::calculated_luminous_flux(&ldt);
    let c_res = if ldt.c_plane_distance > 0.0 {
        ldt.c_plane_distance
    } else {
        15.0
    };
    let g_res = if ldt.g_plane_distance > 0.0 {
        ldt.g_plane_distance
    } else {
        5.0
    };

    println!("Input:  {input_path}");
    println!("  Name: {}", ldt.luminaire_name);
    println!(
        "  Lamp flux: {:.0} lm, LOR: {:.1}%, Calculated flux: {:.1} lm",
        lamp_flux, ldt.light_output_ratio, calculated_flux
    );
    println!("  Max I: {:.1} cd/klm", ldt.max_intensity());
    println!(
        "  Grid: C={c_res}deg x G={g_res}deg ({} x {} planes)",
        ldt.num_c_planes, ldt.num_g_planes
    );
    println!();

    // Trace with the calculated flux (actual output from intensity data)
    println!("Tracing {num_photons} photons through free space (no cover)...");
    let mut scene = Scene::new();
    scene.add_source(Source::from_lvk(
        nalgebra::Point3::origin(),
        nalgebra::Rotation3::identity(),
        ldt.clone(),
        calculated_flux,
    ));
    let config = TracerConfig {
        num_photons,
        detector_c_resolution: c_res,
        detector_g_resolution: g_res,
        seed: 42,
        ..TracerConfig::default()
    };

    let result = Tracer::trace(&scene, &config);
    println!(
        "  Detected: {} ({:.1}%)",
        result.stats.photons_detected,
        result.stats.photons_detected as f64 / result.stats.photons_traced as f64 * 100.0
    );
    println!("  Elapsed: {:.2}s", result.stats.elapsed.as_secs_f64());
    println!();

    // Export — use source LDT's exact C/G angles for perfect grid matching
    let export_cfg = ExportConfig {
        c_step_deg: c_res,
        g_step_deg: g_res,
        symmetry: Some(ldt.symmetry),
        luminaire_name: format!("{} (roundtrip)", ldt.luminaire_name),
        manufacturer: ldt.identification.clone(),
        luminaire_dimensions_mm: (ldt.length, ldt.width, ldt.height),
        luminous_area_mm: (ldt.luminous_area_length, ldt.luminous_area_width),
    };
    let mut sim_ldt = detector_to_eulumdat_at_angles(
        &result.detector,
        calculated_flux,
        lamp_flux,
        Some(&ldt.c_angles),
        Some(&ldt.g_angles),
        &export_cfg,
    );
    sim_ldt.lamp_sets = ldt.lamp_sets.clone();
    sim_ldt.type_indicator = ldt.type_indicator;
    sim_ldt.light_output_ratio = ldt.light_output_ratio;

    // Compare
    println!("Comparison (C0 plane):");
    println!("  gamma | original | simulated | ratio");
    println!("  ------+----------+-----------+------");
    for g in (0..=180).step_by(g_res.max(5.0) as usize) {
        let orig = ldt.sample(0.0, g as f64);
        let sim = sim_ldt.sample(0.0, g as f64);
        let ratio = if orig > 0.1 { sim / orig } else { 0.0 };
        println!("  {:5} | {:8.1} | {:9.1} | {:.3}", g, orig, sim, ratio);
    }

    println!();
    let cmp = PhotometricComparison::from_eulumdat(&ldt, &sim_ldt, "Original", "Simulated");
    println!("Similarity: {:.1}%", cmp.similarity_score * 100.0);
    println!("{}", cmp.to_text());
}

fn cmd_trace(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: goniosim trace <input.ldt> [-o output.ldt] [--cover <preset>] [--distance <mm>] [--photons <n>]");
        process::exit(1);
    }

    let input_path = &args[0];
    let output_path = parse_flag_str(args, "-o", "");
    let cover_name = parse_flag_str(args, "--cover", "");
    let distance_mm = parse_flag_f64(args, "--distance", 40.0);
    let num_photons = parse_flag_u64(args, "--photons", 1_000_000);

    let ldt = load_ldt(input_path);
    let lamp_flux = ldt.total_luminous_flux();
    // calculated_luminous_flux returns the integrated flux from intensity data.
    // This is the value to use as source flux for FromLvk — it represents the
    // actual luminous output encoded in the cd/klm distribution.
    let calculated_flux = eulumdat::PhotometricCalculations::calculated_luminous_flux(&ldt);
    let c_res = if ldt.c_plane_distance > 0.0 {
        ldt.c_plane_distance
    } else {
        15.0
    };
    let g_res = if ldt.g_plane_distance > 0.0 {
        ldt.g_plane_distance
    } else {
        5.0
    };

    println!(
        "Input: {input_path} ({}, lamp={:.0} lm, calculated={:.1} lm)",
        ldt.luminaire_name, lamp_flux, calculated_flux
    );

    // Build scene
    let mut scene = Scene::new();
    scene.add_source(Source::from_lvk(
        nalgebra::Point3::origin(),
        nalgebra::Rotation3::identity(),
        ldt.clone(),
        calculated_flux,
    ));

    if !cover_name.is_empty() {
        let cover_mat = match cover_name.as_str() {
            "clear_pmma_3mm" => catalog::clear_pmma_3mm(),
            "satin_pmma_3mm" => catalog::satin_pmma_3mm(),
            "opal_pmma_3mm" => catalog::opal_pmma_3mm(),
            "opal_light_pmma_3mm" => catalog::opal_light_pmma_3mm(),
            "clear_glass_4mm" => catalog::clear_glass_4mm(),
            "satin_glass_4mm" => catalog::satin_glass_4mm(),
            "clear_polycarbonate_3mm" => catalog::clear_polycarbonate_3mm(),
            "opal_polycarbonate_3mm" => catalog::opal_polycarbonate_3mm(),
            "white_paint" => catalog::white_paint(),
            "anodized_aluminum" => catalog::anodized_aluminum(),
            "mirror_aluminum" => catalog::mirror_aluminum(),
            "matte_black" => catalog::matte_black(),
            other => {
                eprintln!("Unknown cover preset: {other}. Use 'goniosim catalog' to list.");
                process::exit(1);
            }
        };
        println!("Cover: {} at {distance_mm}mm", cover_mat.name);
        let d = distance_mm / 1000.0;
        let mat_id = scene.add_material(cover_mat);
        scene.add_object(
            Primitive::Sheet {
                center: nalgebra::Point3::new(0.0, 0.0, -d),
                normal: nalgebra::Vector3::z_axis(),
                u_axis: nalgebra::Vector3::x_axis(),
                half_width: 0.5,
                half_height: 0.5,
                thickness: 0.003,
            },
            mat_id,
            "cover",
        );
    } else {
        println!("Cover: none (free space)");
    }

    println!("Tracing {num_photons} photons...");
    let config = TracerConfig {
        num_photons,
        detector_c_resolution: c_res,
        detector_g_resolution: g_res,
        seed: 42,
        ..TracerConfig::default()
    };
    let result = Tracer::trace(&scene, &config);

    // Use energy ratio, not photon count — ClearTransmitter attenuates energy without killing photons
    let energy_frac = result.stats.total_energy_detected / result.stats.total_energy_emitted;
    println!(
        "  Energy throughput: {:.1}%, Elapsed: {:.2}s",
        energy_frac * 100.0,
        result.stats.elapsed.as_secs_f64()
    );

    let export_cfg = ExportConfig {
        c_step_deg: c_res,
        g_step_deg: g_res,
        symmetry: None, // full 360° output
        luminaire_name: format!("{} (goniosim)", ldt.luminaire_name),
        manufacturer: ldt.identification.clone(),
        luminaire_dimensions_mm: (ldt.length, ldt.width, ldt.height),
        luminous_area_mm: (ldt.luminous_area_length, ldt.luminous_area_width),
    };
    // LOR: scale by energy throughput (not photon count — ClearTransmitter attenuates energy)
    let sim_lor = ldt.light_output_ratio * energy_frac;

    let mut sim_ldt = detector_to_eulumdat_with_lamp_flux(
        &result.detector,
        calculated_flux,
        lamp_flux,
        &export_cfg,
    );
    sim_ldt.lamp_sets = ldt.lamp_sets.clone();
    sim_ldt.type_indicator = ldt.type_indicator;
    sim_ldt.light_output_ratio = ldt.light_output_ratio * energy_frac;

    println!(
        "  Simulated max I: {:.1} cd/klm (original: {:.1})",
        sim_ldt.max_intensity(),
        ldt.max_intensity()
    );
    println!(
        "  Simulated LOR: {:.1}% (original: {:.1}%)",
        sim_lor, ldt.light_output_ratio
    );
    println!();

    // Always print comparison
    let cmp = PhotometricComparison::from_eulumdat(&ldt, &sim_ldt, "Original", "Simulated");
    println!("Similarity: {:.1}%", cmp.similarity_score * 100.0);
    println!("{}", cmp.to_text());

    if !output_path.is_empty() {
        let ldt_str = sim_ldt.to_ldt();
        fs::write(&output_path, &ldt_str).unwrap_or_else(|e| {
            eprintln!("Cannot write {output_path}: {e}");
            process::exit(1);
        });
        println!("Written: {output_path}");
    }
}

fn cmd_compare(args: &[String]) {
    if args.len() < 2 {
        eprintln!("Usage: goniosim compare <a.ldt> <b.ldt>");
        process::exit(1);
    }

    let a = load_ldt(&args[0]);
    let b = load_ldt(&args[1]);

    let cmp = PhotometricComparison::from_eulumdat(&a, &b, &args[0], &args[1]);
    println!("Similarity: {:.1}%", cmp.similarity_score * 100.0);
    println!("{}", cmp.to_text());
}

fn cmd_catalog() {
    println!("Material Catalog:");
    println!(
        "{:<28} {:>6} {:>5} {:>6} {:>6} {:>8}",
        "Name", "Refl%", "IOR", "Trans%", "Thick", "Diff%"
    );
    println!("{}", "-".repeat(70));
    for m in material_catalog() {
        println!(
            "{:<28} {:>5.0}% {:>5.2} {:>5.0}% {:>5.1}mm {:>5.0}%",
            m.name, m.reflectance_pct, m.ior, m.transmittance_pct, m.thickness_mm, m.diffusion_pct
        );
    }
}

fn parse_flag_str(args: &[String], flag: &str, default: &str) -> String {
    for i in 0..args.len() {
        if args[i] == flag && i + 1 < args.len() {
            return args[i + 1].clone();
        }
    }
    default.to_string()
}

fn parse_flag_f64(args: &[String], flag: &str, default: f64) -> f64 {
    let s = parse_flag_str(args, flag, "");
    if s.is_empty() {
        default
    } else {
        s.parse().unwrap_or(default)
    }
}

fn parse_flag_u64(args: &[String], flag: &str, default: u64) -> u64 {
    let s = parse_flag_str(args, flag, "");
    if s.is_empty() {
        default
    } else {
        s.parse().unwrap_or(default)
    }
}
