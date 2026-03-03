//! Native benchmark for eulumdat parsing and diagram generation
//!
//! Run with: cargo bench -p eulumdat
//!
//! Compare with WASM benchmark by running in browser console.

use std::time::Instant;

use eulumdat::{CuTable, IesParser, UgrTable};

// Simple LDT for quick benchmarks
const SAMPLE_LDT: &str =
    include_str!("../../../crates/eulumdat-wasm/templates/fluorescent_luminaire.ldt");

// File paths (can't use include_str! due to encoding issues in some files)
// These paths are relative to the workspace root where `cargo bench` is run
const CHALLENGING_IES_PATH: &str = "tests/files/Preliminar Interlab IPT_fotometria 1.ies";
const INDOOR_IES_PATH: &str = "tests/files/Indoor_60W_120G_5300LM_5000K_OVNI.ies";

// Get workspace root path
fn get_workspace_path(relative: &str) -> std::path::PathBuf {
    // Try current directory first (when run from workspace root)
    let path = std::path::Path::new(relative);
    if path.exists() {
        return path.to_path_buf();
    }
    // Try from CARGO_MANIFEST_DIR (when run from crate directory)
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let ws_root = std::path::Path::new(&manifest_dir)
            .parent()
            .and_then(|p| p.parent());
        if let Some(root) = ws_root {
            let full_path = root.join(relative);
            if full_path.exists() {
                return full_path;
            }
        }
    }
    // Fallback to relative path
    path.to_path_buf()
}

fn main() {
    println!("=== Eulumdat Native Benchmark ===\n");

    // Run all benchmark categories
    run_simple_benchmark();
    println!("\n{}\n", "=".repeat(60));
    run_challenging_benchmark();
    println!("\n{}\n", "=".repeat(60));
    run_cu_ugr_benchmark();
}

fn run_simple_benchmark() {
    println!("--- Simple LDT (24 C-planes × 19 gamma angles) ---\n");
    let iterations = 1000;

    // Benchmark parsing
    println!("Parsing {} iterations...", iterations);
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = eulumdat::Eulumdat::parse(SAMPLE_LDT);
    }
    let parse_duration = start.elapsed();
    let parse_per_iter = parse_duration.as_micros() as f64 / iterations as f64;
    println!(
        "  Total: {:?}, Per iteration: {:.2} µs",
        parse_duration, parse_per_iter
    );

    // Parse once for diagram benchmarks
    let ldt = eulumdat::Eulumdat::parse(SAMPLE_LDT).expect("Parse failed");

    // Benchmark polar diagram generation
    println!("\nPolar diagram {} iterations...", iterations);
    let start = Instant::now();
    for _ in 0..iterations {
        let polar = eulumdat::diagram::PolarDiagram::from_eulumdat(&ldt);
        let _ = polar.to_svg(500.0, 500.0, &eulumdat::diagram::SvgTheme::light());
    }
    let polar_duration = start.elapsed();
    let polar_per_iter = polar_duration.as_micros() as f64 / iterations as f64;
    println!(
        "  Total: {:?}, Per iteration: {:.2} µs",
        polar_duration, polar_per_iter
    );

    // Benchmark cartesian diagram generation
    println!("\nCartesian diagram {} iterations...", iterations);
    let start = Instant::now();
    for _ in 0..iterations {
        let cart = eulumdat::diagram::CartesianDiagram::from_eulumdat(&ldt, 600.0, 400.0, 8);
        let _ = cart.to_svg(600.0, 400.0, &eulumdat::diagram::SvgTheme::light());
    }
    let cartesian_duration = start.elapsed();
    let cartesian_per_iter = cartesian_duration.as_micros() as f64 / iterations as f64;
    println!(
        "  Total: {:?}, Per iteration: {:.2} µs",
        cartesian_duration, cartesian_per_iter
    );

    // Benchmark IES export
    println!("\nIES export {} iterations...", iterations);
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = eulumdat::IesExporter::export(&ldt);
    }
    let ies_duration = start.elapsed();
    let ies_per_iter = ies_duration.as_micros() as f64 / iterations as f64;
    println!(
        "  Total: {:?}, Per iteration: {:.2} µs",
        ies_duration, ies_per_iter
    );

    // Benchmark LDT export (roundtrip)
    println!("\nLDT export {} iterations...", iterations);
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = ldt.to_ldt();
    }
    let ldt_duration = start.elapsed();
    let ldt_per_iter = ldt_duration.as_micros() as f64 / iterations as f64;
    println!(
        "  Total: {:?}, Per iteration: {:.2} µs",
        ldt_duration, ldt_per_iter
    );

    // Summary
    println!("\n=== Summary - Simple (µs per operation) ===");
    println!("Parse LDT:         {:>8.2} µs", parse_per_iter);
    println!("Polar diagram:     {:>8.2} µs", polar_per_iter);
    println!("Cartesian diagram: {:>8.2} µs", cartesian_per_iter);
    println!("IES export:        {:>8.2} µs", ies_per_iter);
    println!("LDT export:        {:>8.2} µs", ldt_per_iter);

    // Output as JSON for comparison
    println!("\n=== JSON - Simple (for WASM comparison) ===");
    println!(
        r#"{{"platform":"native","type":"simple","iterations":{},"parse_us":{:.2},"polar_us":{:.2},"cartesian_us":{:.2},"ies_export_us":{:.2},"ldt_export_us":{:.2}}}"#,
        iterations, parse_per_iter, polar_per_iter, cartesian_per_iter, ies_per_iter, ldt_per_iter
    );
}

fn run_challenging_benchmark() {
    println!("--- Challenging IES (361 C-planes × 263 gamma angles = 95,000 values) ---\n");

    // Check if file exists
    let path = get_workspace_path(CHALLENGING_IES_PATH);
    if !path.exists() {
        println!(
            "Warning: {} not found, skipping challenging benchmark",
            CHALLENGING_IES_PATH
        );
        return;
    }

    // Fewer iterations for the large file
    let iterations = 100;

    // Benchmark IES parsing from file
    println!("Parsing IES {} iterations...", iterations);
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = IesParser::parse_file(&path);
    }
    let parse_duration = start.elapsed();
    let parse_per_iter = parse_duration.as_micros() as f64 / iterations as f64;
    println!(
        "  Total: {:?}, Per iteration: {:.2} µs ({:.2} ms)",
        parse_duration,
        parse_per_iter,
        parse_per_iter / 1000.0
    );

    // Parse once for diagram benchmarks
    let ldt = IesParser::parse_file(&path).expect("Parse failed");

    println!("\nFile stats:");
    println!("  C-planes: {}", ldt.c_angles.len());
    println!("  Gamma angles: {}", ldt.g_angles.len());
    println!(
        "  Total intensity values: {}",
        ldt.c_angles.len() * ldt.g_angles.len()
    );

    // Benchmark polar diagram generation
    println!("\nPolar diagram {} iterations...", iterations);
    let start = Instant::now();
    for _ in 0..iterations {
        let polar = eulumdat::diagram::PolarDiagram::from_eulumdat(&ldt);
        let _ = polar.to_svg(500.0, 500.0, &eulumdat::diagram::SvgTheme::light());
    }
    let polar_duration = start.elapsed();
    let polar_per_iter = polar_duration.as_micros() as f64 / iterations as f64;
    println!(
        "  Total: {:?}, Per iteration: {:.2} µs ({:.2} ms)",
        polar_duration,
        polar_per_iter,
        polar_per_iter / 1000.0
    );

    // Benchmark cartesian diagram generation
    println!("\nCartesian diagram {} iterations...", iterations);
    let start = Instant::now();
    for _ in 0..iterations {
        let cart = eulumdat::diagram::CartesianDiagram::from_eulumdat(&ldt, 600.0, 400.0, 8);
        let _ = cart.to_svg(600.0, 400.0, &eulumdat::diagram::SvgTheme::light());
    }
    let cartesian_duration = start.elapsed();
    let cartesian_per_iter = cartesian_duration.as_micros() as f64 / iterations as f64;
    println!(
        "  Total: {:?}, Per iteration: {:.2} µs ({:.2} ms)",
        cartesian_duration,
        cartesian_per_iter,
        cartesian_per_iter / 1000.0
    );

    // Benchmark IES export (roundtrip)
    println!("\nIES export {} iterations...", iterations);
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = eulumdat::IesExporter::export(&ldt);
    }
    let ies_duration = start.elapsed();
    let ies_per_iter = ies_duration.as_micros() as f64 / iterations as f64;
    println!(
        "  Total: {:?}, Per iteration: {:.2} µs ({:.2} ms)",
        ies_duration,
        ies_per_iter,
        ies_per_iter / 1000.0
    );

    // Benchmark LDT export
    println!("\nLDT export {} iterations...", iterations);
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = ldt.to_ldt();
    }
    let ldt_duration = start.elapsed();
    let ldt_per_iter = ldt_duration.as_micros() as f64 / iterations as f64;
    println!(
        "  Total: {:?}, Per iteration: {:.2} µs ({:.2} ms)",
        ldt_duration,
        ldt_per_iter,
        ldt_per_iter / 1000.0
    );

    // Summary
    println!("\n=== Summary - Challenging (µs per operation) ===");
    println!(
        "Parse IES:         {:>10.2} µs ({:.2} ms)",
        parse_per_iter,
        parse_per_iter / 1000.0
    );
    println!(
        "Polar diagram:     {:>10.2} µs ({:.2} ms)",
        polar_per_iter,
        polar_per_iter / 1000.0
    );
    println!(
        "Cartesian diagram: {:>10.2} µs ({:.2} ms)",
        cartesian_per_iter,
        cartesian_per_iter / 1000.0
    );
    println!(
        "IES export:        {:>10.2} µs ({:.2} ms)",
        ies_per_iter,
        ies_per_iter / 1000.0
    );
    println!(
        "LDT export:        {:>10.2} µs ({:.2} ms)",
        ldt_per_iter,
        ldt_per_iter / 1000.0
    );

    // Output as JSON for comparison
    println!("\n=== JSON - Challenging (for WASM comparison) ===");
    println!(
        r#"{{"platform":"native","type":"challenging","iterations":{},"parse_us":{:.2},"polar_us":{:.2},"cartesian_us":{:.2},"ies_export_us":{:.2},"ldt_export_us":{:.2}}}"#,
        iterations, parse_per_iter, polar_per_iter, cartesian_per_iter, ies_per_iter, ldt_per_iter
    );
}

fn run_cu_ugr_benchmark() {
    println!("--- CU/UGR Table Calculation: Simple vs Sophisticated ---\n");

    // Check if file exists
    let path = get_workspace_path(INDOOR_IES_PATH);
    if !path.exists() {
        println!(
            "Warning: {} not found, skipping CU/UGR benchmark",
            INDOOR_IES_PATH
        );
        return;
    }

    // Parse the indoor IES file (has proper luminaire characteristics)
    let ldt = IesParser::parse_file(&path).expect("Failed to parse Indoor IES");

    println!("File: Indoor_60W_120G_5300LM_5000K_OVNI.ies");
    println!(
        "  Flux: {} lm",
        ldt.lamp_sets
            .iter()
            .map(|l| l.total_luminous_flux)
            .sum::<f64>()
    );
    println!(
        "  Luminous area: {} mm × {} mm\n",
        ldt.luminous_area_length, ldt.luminous_area_width
    );

    let iterations = 100;

    // ========================================================================
    // CU Table Benchmarks
    // ========================================================================
    println!("=== CU Table (11 RCR × 18 reflectance combinations = 198 values) ===\n");

    // Benchmark simple CU calculation
    println!("Simple CU calculation {} iterations...", iterations);
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = CuTable::calculate_simple(&ldt);
    }
    let cu_simple_duration = start.elapsed();
    let cu_simple_per_iter = cu_simple_duration.as_micros() as f64 / iterations as f64;
    println!(
        "  Total: {:?}, Per iteration: {:.2} µs",
        cu_simple_duration, cu_simple_per_iter
    );

    // Benchmark sophisticated CU calculation
    println!(
        "\nSophisticated CU calculation {} iterations...",
        iterations
    );
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = CuTable::calculate_sophisticated(&ldt);
    }
    let cu_soph_duration = start.elapsed();
    let cu_soph_per_iter = cu_soph_duration.as_micros() as f64 / iterations as f64;
    println!(
        "  Total: {:?}, Per iteration: {:.2} µs",
        cu_soph_duration, cu_soph_per_iter
    );

    let cu_speedup = cu_soph_per_iter / cu_simple_per_iter;
    println!(
        "\n  Simple/Sophisticated ratio: {:.2}x (simple is {:.1}% of sophisticated time)",
        cu_speedup,
        100.0 / cu_speedup
    );

    // ========================================================================
    // UGR Table Benchmarks
    // ========================================================================
    println!("\n=== UGR Table (19 rooms × 5 reflectances × 2 views = 190 values) ===\n");

    // Benchmark simple UGR calculation
    println!("Simple UGR calculation {} iterations...", iterations);
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = UgrTable::calculate_simple(&ldt);
    }
    let ugr_simple_duration = start.elapsed();
    let ugr_simple_per_iter = ugr_simple_duration.as_micros() as f64 / iterations as f64;
    println!(
        "  Total: {:?}, Per iteration: {:.2} µs",
        ugr_simple_duration, ugr_simple_per_iter
    );

    // Benchmark sophisticated UGR calculation
    println!(
        "\nSophisticated UGR calculation {} iterations...",
        iterations
    );
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = UgrTable::calculate_sophisticated(&ldt);
    }
    let ugr_soph_duration = start.elapsed();
    let ugr_soph_per_iter = ugr_soph_duration.as_micros() as f64 / iterations as f64;
    println!(
        "  Total: {:?}, Per iteration: {:.2} µs",
        ugr_soph_duration, ugr_soph_per_iter
    );

    let ugr_speedup = ugr_soph_per_iter / ugr_simple_per_iter;
    println!(
        "\n  Simple/Sophisticated ratio: {:.2}x (simple is {:.1}% of sophisticated time)",
        ugr_speedup,
        100.0 / ugr_speedup
    );

    // ========================================================================
    // Accuracy comparison
    // ========================================================================
    println!("\n=== Accuracy Comparison (vs Photometric Toolbox reference) ===\n");

    // CU accuracy
    let cu_simple = CuTable::calculate_simple(&ldt);
    let cu_soph = CuTable::calculate_sophisticated(&ldt);

    // Reference from PT: RCR=0, RC=80, RW=70 → 119
    let pt_cu_r0 = 119.0;
    let simple_cu_r0 = cu_simple.values[0][0];
    let soph_cu_r0 = cu_soph.values[0][0];

    // Reference from PT: RCR=5, RC=80, RW=50 → 64
    let pt_cu_r5 = 64.0;
    let simple_cu_r5 = cu_simple.values[5][1];
    let soph_cu_r5 = cu_soph.values[5][1];

    println!("CU Table Accuracy:");
    println!("  RCR=0, RC=80/RW=70:");
    println!("    Photometric Toolbox: {:.0}", pt_cu_r0);
    println!(
        "    Simple:              {:.0} (error: {:+.1}%)",
        simple_cu_r0,
        (simple_cu_r0 - pt_cu_r0) / pt_cu_r0 * 100.0
    );
    println!(
        "    Sophisticated:       {:.0} (error: {:+.1}%)",
        soph_cu_r0,
        (soph_cu_r0 - pt_cu_r0) / pt_cu_r0 * 100.0
    );
    println!("  RCR=5, RC=80/RW=50:");
    println!("    Photometric Toolbox: {:.0}", pt_cu_r5);
    println!(
        "    Simple:              {:.0} (error: {:+.1}%)",
        simple_cu_r5,
        (simple_cu_r5 - pt_cu_r5) / pt_cu_r5 * 100.0
    );
    println!(
        "    Sophisticated:       {:.0} (error: {:+.1}%)",
        soph_cu_r5,
        (soph_cu_r5 - pt_cu_r5) / pt_cu_r5 * 100.0
    );

    // UGR accuracy
    let ugr_simple = UgrTable::calculate_simple(&ldt);
    let ugr_soph = UgrTable::calculate_sophisticated(&ldt);

    // Reference from PT: 2H×2H, RC=70/RW=50 → 22.4
    let pt_ugr_2x2 = 22.4;
    let simple_ugr_2x2 = ugr_simple.crosswise[0][0];
    let soph_ugr_2x2 = ugr_soph.crosswise[0][0];

    // Reference from PT: 8H×8H, RC=70/RW=50 → 25.2
    let pt_ugr_8x8 = 25.2;
    let simple_ugr_8x8 = ugr_simple.crosswise[14][0]; // Index 14 is 8H×8H
    let soph_ugr_8x8 = ugr_soph.crosswise[14][0];

    println!("\nUGR Table Accuracy:");
    println!("  2H×2H, RC=70/RW=50:");
    println!("    Photometric Toolbox: {:.1}", pt_ugr_2x2);
    println!(
        "    Simple:              {:.1} (error: {:+.1} points)",
        simple_ugr_2x2,
        simple_ugr_2x2 - pt_ugr_2x2
    );
    println!(
        "    Sophisticated:       {:.1} (error: {:+.1} points)",
        soph_ugr_2x2,
        soph_ugr_2x2 - pt_ugr_2x2
    );
    println!("  8H×8H, RC=70/RW=50:");
    println!("    Photometric Toolbox: {:.1}", pt_ugr_8x8);
    println!(
        "    Simple:              {:.1} (error: {:+.1} points)",
        simple_ugr_8x8,
        simple_ugr_8x8 - pt_ugr_8x8
    );
    println!(
        "    Sophisticated:       {:.1} (error: {:+.1} points)",
        soph_ugr_8x8,
        soph_ugr_8x8 - pt_ugr_8x8
    );

    // ========================================================================
    // Summary
    // ========================================================================
    println!("\n=== Summary - CU/UGR Benchmarks ===");
    println!("CU Table:");
    println!("  Simple:        {:>8.2} µs", cu_simple_per_iter);
    println!(
        "  Sophisticated: {:>8.2} µs ({:.1}x slower)",
        cu_soph_per_iter, cu_speedup
    );
    println!("UGR Table:");
    println!("  Simple:        {:>8.2} µs", ugr_simple_per_iter);
    println!(
        "  Sophisticated: {:>8.2} µs ({:.1}x slower)",
        ugr_soph_per_iter, ugr_speedup
    );

    // Output as JSON
    println!("\n=== JSON - CU/UGR (for comparison) ===");
    println!(
        r#"{{"platform":"native","type":"cu_ugr","iterations":{},"cu_simple_us":{:.2},"cu_soph_us":{:.2},"cu_speedup":{:.2},"ugr_simple_us":{:.2},"ugr_soph_us":{:.2},"ugr_speedup":{:.2}}}"#,
        iterations,
        cu_simple_per_iter,
        cu_soph_per_iter,
        cu_speedup,
        ugr_simple_per_iter,
        ugr_soph_per_iter,
        ugr_speedup
    );
}
