//! WASM benchmark module for performance comparison with native
//!
//! Run in browser console after loading the app:
//! ```js
//! window.runBenchmark();        // Simple benchmark
//! window.runBenchmarkFull();    // Both simple and challenging
//! ```

use wasm_bindgen::prelude::*;

// Simple LDT for quick benchmarks
const SAMPLE_LDT: &str = include_str!("../../eulumdat-wasm/templates/fluorescent_luminaire.ldt");

// Challenging IES file: 263 vertical angles × 361 horizontal planes = ~95,000 intensity values!
const CHALLENGING_IES: &str =
    include_str!("../../../tests/files/Preliminar Interlab IPT_fotometria 1.ies");

/// Benchmark results structure
#[wasm_bindgen]
pub struct BenchmarkResults {
    pub iterations: u32,
    pub parse_us: f64,
    pub polar_us: f64,
    pub cartesian_us: f64,
    pub ies_export_us: f64,
    pub ldt_export_us: f64,
}

#[wasm_bindgen]
impl BenchmarkResults {
    #[wasm_bindgen(getter)]
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"platform":"wasm","iterations":{},"parse_us":{:.2},"polar_us":{:.2},"cartesian_us":{:.2},"ies_export_us":{:.2},"ldt_export_us":{:.2}}}"#,
            self.iterations,
            self.parse_us,
            self.polar_us,
            self.cartesian_us,
            self.ies_export_us,
            self.ldt_export_us
        )
    }
}

/// Run the benchmark and return results
#[wasm_bindgen(js_name = runBenchmark)]
pub fn run_benchmark() -> BenchmarkResults {
    let iterations = 1000u32;
    let window = web_sys::window().expect("no window");
    let performance = window.performance().expect("no performance");

    // Benchmark parsing
    let start = performance.now();
    for _ in 0..iterations {
        let _ = eulumdat::Eulumdat::parse(SAMPLE_LDT);
    }
    let parse_total = performance.now() - start;
    let parse_us = (parse_total * 1000.0) / iterations as f64;

    // Parse once for other benchmarks
    let ldt = eulumdat::Eulumdat::parse(SAMPLE_LDT).expect("parse failed");

    // Benchmark polar diagram
    let start = performance.now();
    for _ in 0..iterations {
        let polar = eulumdat::diagram::PolarDiagram::from_eulumdat(&ldt);
        let _ = polar.to_svg(500.0, 500.0, &eulumdat::diagram::SvgTheme::light());
    }
    let polar_total = performance.now() - start;
    let polar_us = (polar_total * 1000.0) / iterations as f64;

    // Benchmark cartesian diagram
    let start = performance.now();
    for _ in 0..iterations {
        let cart = eulumdat::diagram::CartesianDiagram::from_eulumdat(&ldt, 600.0, 400.0, 8);
        let _ = cart.to_svg(600.0, 400.0, &eulumdat::diagram::SvgTheme::light());
    }
    let cartesian_total = performance.now() - start;
    let cartesian_us = (cartesian_total * 1000.0) / iterations as f64;

    // Benchmark IES export
    let start = performance.now();
    for _ in 0..iterations {
        let _ = eulumdat::IesExporter::export(&ldt);
    }
    let ies_total = performance.now() - start;
    let ies_export_us = (ies_total * 1000.0) / iterations as f64;

    // Benchmark LDT export
    let start = performance.now();
    for _ in 0..iterations {
        let _ = ldt.to_ldt();
    }
    let ldt_total = performance.now() - start;
    let ldt_export_us = (ldt_total * 1000.0) / iterations as f64;

    // Log results to console
    web_sys::console::log_1(&"=== Eulumdat WASM Benchmark ===".into());
    web_sys::console::log_1(&format!("Iterations: {}", iterations).into());
    web_sys::console::log_1(&format!("Parse LDT:         {:.2} µs", parse_us).into());
    web_sys::console::log_1(&format!("Polar diagram:     {:.2} µs", polar_us).into());
    web_sys::console::log_1(&format!("Cartesian diagram: {:.2} µs", cartesian_us).into());
    web_sys::console::log_1(&format!("IES export:        {:.2} µs", ies_export_us).into());
    web_sys::console::log_1(&format!("LDT export:        {:.2} µs", ldt_export_us).into());

    BenchmarkResults {
        iterations,
        parse_us,
        polar_us,
        cartesian_us,
        ies_export_us,
        ldt_export_us,
    }
}

/// Compare WASM results with native baseline
#[wasm_bindgen(js_name = compareBenchmark)]
pub fn compare_benchmark(native_json: &str) -> String {
    let wasm = run_benchmark();

    // Parse native results (simple JSON parsing)
    let native_parse: f64 = extract_json_field(native_json, "parse_us");
    let native_polar: f64 = extract_json_field(native_json, "polar_us");
    let native_cartesian: f64 = extract_json_field(native_json, "cartesian_us");
    let native_ies: f64 = extract_json_field(native_json, "ies_export_us");
    let native_ldt: f64 = extract_json_field(native_json, "ldt_export_us");

    let compare = |wasm_val: f64, native_val: f64| -> String {
        let ratio = wasm_val / native_val;
        if ratio > 1.0 {
            format!("{:.1}x slower", ratio)
        } else {
            format!("{:.1}x faster", 1.0 / ratio)
        }
    };

    let result = format!(
        r#"
=== Native vs WASM Comparison ===

Operation          Native (µs)  WASM (µs)    Ratio
─────────────────────────────────────────────────────
Parse LDT          {:>8.2}     {:>8.2}     {}
Polar diagram      {:>8.2}     {:>8.2}     {}
Cartesian diagram  {:>8.2}     {:>8.2}     {}
IES export         {:>8.2}     {:>8.2}     {}
LDT export         {:>8.2}     {:>8.2}     {}
"#,
        native_parse,
        wasm.parse_us,
        compare(wasm.parse_us, native_parse),
        native_polar,
        wasm.polar_us,
        compare(wasm.polar_us, native_polar),
        native_cartesian,
        wasm.cartesian_us,
        compare(wasm.cartesian_us, native_cartesian),
        native_ies,
        wasm.ies_export_us,
        compare(wasm.ies_export_us, native_ies),
        native_ldt,
        wasm.ldt_export_us,
        compare(wasm.ldt_export_us, native_ldt),
    );

    web_sys::console::log_1(&JsValue::from_str(&result));

    result
}

fn extract_json_field(json: &str, field: &str) -> f64 {
    let pattern = format!("\"{}\":", field);
    if let Some(pos) = json.find(&pattern) {
        let start = pos + pattern.len();
        let rest = &json[start..];
        let end = rest.find([',', '}']).unwrap_or(rest.len());
        rest[..end].trim().parse().unwrap_or(0.0)
    } else {
        0.0
    }
}

/// Run challenging benchmark with large IES file (95,000 intensity values)
#[wasm_bindgen(js_name = runBenchmarkChallenging)]
pub fn run_benchmark_challenging() -> BenchmarkResults {
    let iterations = 100u32;
    let window = web_sys::window().expect("no window");
    let performance = window.performance().expect("no performance");

    web_sys::console::log_1(
        &"=== Challenging IES Benchmark (361 × 263 = 95,000 values) ===".into(),
    );

    // Benchmark IES parsing
    let start = performance.now();
    for _ in 0..iterations {
        let _ = eulumdat::IesParser::parse(CHALLENGING_IES);
    }
    let parse_total = performance.now() - start;
    let parse_us = (parse_total * 1000.0) / iterations as f64;

    // Parse once for other benchmarks
    let ldt = eulumdat::IesParser::parse(CHALLENGING_IES).expect("parse failed");

    web_sys::console::log_1(
        &format!(
            "C-planes: {}, Gamma angles: {}, Total: {}",
            ldt.c_angles.len(),
            ldt.g_angles.len(),
            ldt.c_angles.len() * ldt.g_angles.len()
        )
        .into(),
    );

    // Benchmark polar diagram
    let start = performance.now();
    for _ in 0..iterations {
        let polar = eulumdat::diagram::PolarDiagram::from_eulumdat(&ldt);
        let _ = polar.to_svg(500.0, 500.0, &eulumdat::diagram::SvgTheme::light());
    }
    let polar_total = performance.now() - start;
    let polar_us = (polar_total * 1000.0) / iterations as f64;

    // Benchmark cartesian diagram
    let start = performance.now();
    for _ in 0..iterations {
        let cart = eulumdat::diagram::CartesianDiagram::from_eulumdat(&ldt, 600.0, 400.0, 8);
        let _ = cart.to_svg(600.0, 400.0, &eulumdat::diagram::SvgTheme::light());
    }
    let cartesian_total = performance.now() - start;
    let cartesian_us = (cartesian_total * 1000.0) / iterations as f64;

    // Benchmark IES export
    let start = performance.now();
    for _ in 0..iterations {
        let _ = eulumdat::IesExporter::export(&ldt);
    }
    let ies_total = performance.now() - start;
    let ies_export_us = (ies_total * 1000.0) / iterations as f64;

    // Benchmark LDT export
    let start = performance.now();
    for _ in 0..iterations {
        let _ = ldt.to_ldt();
    }
    let ldt_total = performance.now() - start;
    let ldt_export_us = (ldt_total * 1000.0) / iterations as f64;

    // Log results to console
    web_sys::console::log_1(&format!("Iterations: {}", iterations).into());
    web_sys::console::log_1(
        &format!(
            "Parse IES:         {:.2} µs ({:.2} ms)",
            parse_us,
            parse_us / 1000.0
        )
        .into(),
    );
    web_sys::console::log_1(
        &format!(
            "Polar diagram:     {:.2} µs ({:.2} ms)",
            polar_us,
            polar_us / 1000.0
        )
        .into(),
    );
    web_sys::console::log_1(
        &format!(
            "Cartesian diagram: {:.2} µs ({:.2} ms)",
            cartesian_us,
            cartesian_us / 1000.0
        )
        .into(),
    );
    web_sys::console::log_1(
        &format!(
            "IES export:        {:.2} µs ({:.2} ms)",
            ies_export_us,
            ies_export_us / 1000.0
        )
        .into(),
    );
    web_sys::console::log_1(
        &format!(
            "LDT export:        {:.2} µs ({:.2} ms)",
            ldt_export_us,
            ldt_export_us / 1000.0
        )
        .into(),
    );

    let json = format!(
        r#"{{"platform":"wasm","type":"challenging","iterations":{},"parse_us":{:.2},"polar_us":{:.2},"cartesian_us":{:.2},"ies_export_us":{:.2},"ldt_export_us":{:.2}}}"#,
        iterations, parse_us, polar_us, cartesian_us, ies_export_us, ldt_export_us
    );
    web_sys::console::log_1(&format!("JSON: {}", json).into());

    BenchmarkResults {
        iterations,
        parse_us,
        polar_us,
        cartesian_us,
        ies_export_us,
        ldt_export_us,
    }
}

/// Run full benchmark (both simple and challenging)
#[wasm_bindgen(js_name = runBenchmarkFull)]
pub fn run_benchmark_full() {
    web_sys::console::log_1(&"\n========== FULL BENCHMARK ==========\n".into());
    web_sys::console::log_1(&"--- Simple LDT ---".into());
    let _simple = run_benchmark();
    web_sys::console::log_1(&"\n--- Challenging IES ---".into());
    let _challenging = run_benchmark_challenging();
}
