//! WASM benchmark for eulumdat - runs in wasmer/wasmtime for fair comparison
//!
//! Build:
//!   cargo build --release --target wasm32-wasip1 -p eulumdat --example wasm_benchmark
//!
//! Run with wasmer:
//!   wasmer run target/wasm32-wasip1/release/examples/wasm_benchmark.wasm
//!
//! This gives a fairer comparison than browser WASM because:
//! - No DOM/browser overhead
//! - No V8 JIT warmup variance
//! - Pure WASM execution performance

use std::time::Instant;

// Sample LDT content for benchmarking (valid EULUMDAT format)
const SAMPLE_LDT: &str =
    include_str!("../../../crates/eulumdat-wasm/templates/fluorescent_luminaire.ldt");

// Challenging IES file
const CHALLENGING_IES: &str =
    include_str!("../../../tests/files/Preliminar Interlab IPT_fotometria 1.ies");

fn main() {
    println!("=== Eulumdat WASM Benchmark (wasmer/wasmtime) ===\n");

    run_simple_benchmark();
    println!("\n{}\n", "=".repeat(60));
    run_challenging_benchmark();
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

    println!("\n=== JSON - Simple ===");
    println!(
        r#"{{"platform":"wasm-wasi","type":"simple","iterations":{},"parse_us":{:.2},"polar_us":{:.2},"cartesian_us":{:.2},"ies_export_us":{:.2},"ldt_export_us":{:.2}}}"#,
        iterations, parse_per_iter, polar_per_iter, cartesian_per_iter, ies_per_iter, ldt_per_iter
    );
}

fn run_challenging_benchmark() {
    println!("--- Challenging IES (361 C-planes × 263 gamma angles = 95,000 values) ---\n");

    let iterations = 100;

    // Benchmark IES parsing
    println!("Parsing IES {} iterations...", iterations);
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = eulumdat::IesParser::parse(CHALLENGING_IES);
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
    let ldt = eulumdat::IesParser::parse(CHALLENGING_IES).expect("Parse failed");

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

    // Benchmark IES export
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

    println!("\n=== JSON - Challenging ===");
    println!(
        r#"{{"platform":"wasm-wasi","type":"challenging","iterations":{},"parse_us":{:.2},"polar_us":{:.2},"cartesian_us":{:.2},"ies_export_us":{:.2},"ldt_export_us":{:.2}}}"#,
        iterations, parse_per_iter, polar_per_iter, cartesian_per_iter, ies_per_iter, ldt_per_iter
    );
}
