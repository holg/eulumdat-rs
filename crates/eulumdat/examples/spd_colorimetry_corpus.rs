//! Walk every SPD file under docs/SPDs/, compute CIE colorimetry, and compare
//! to the Signify reference prelude where available.
//!
//! Run: cargo run -q -p eulumdat --example spd_colorimetry_corpus

use eulumdat::{analyze_spd, load_spd};

fn main() {
    let root = format!("{}/../../docs/SPDs", env!("CARGO_MANIFEST_DIR"));

    println!(
        "{:<32} {:<7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>6}",
        "file", "src", "x", "y", "u'", "v'", "CCT", "Duv", "peak"
    );
    println!("{}", "-".repeat(108));

    for sub in ["Luxeon_SPD_fixed", "luxeon_95CRI", "Youji-Nite", "Signify"] {
        let dir = format!("{root}/{sub}");
        let Ok(mut entries) = std::fs::read_dir(&dir) else { continue };
        let mut paths: Vec<_> = entries
            .by_ref()
            .flatten()
            .map(|e| e.path())
            .filter(|p| matches!(p.extension().and_then(|s| s.to_str()), Some("spd") | Some("csv")))
            .collect();
        paths.sort();

        for path in paths {
            let loaded = match load_spd(&path) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("load {}: {e}", path.display());
                    continue;
                }
            };
            let c = analyze_spd(&loaded.spd);
            let name = path.file_name().unwrap().to_string_lossy().into_owned();

            // Our line.
            println!(
                "{:<32} {:<7} {:>7.4} {:>7.4} {:>7.4} {:>7.4} {:>7.0} {:>7.4} {:>6.0}",
                truncate(&name, 32),
                "ours",
                c.x_1931, c.y_1931, c.u_prime, c.v_prime, c.cct_k, c.duv, c.peak_wavelength_nm
            );

            // Reference line (Signify only).
            if let Some(r) = loaded.reference {
                println!(
                    "{:<32} {:<7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>6}",
                    "",
                    "ref",
                    fmt(r.chromaticity_x, 4),
                    fmt(r.chromaticity_y, 4),
                    fmt(r.cie1976_u_prime, 4),
                    fmt(r.cie1976_v_prime, 4),
                    fmt(r.cct_k, 0),
                    fmt(r.duv, 4),
                    fmt(r.peak_wavelength_nm, 0),
                );
            }
        }
        println!();
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n { s.to_string() } else { format!("{}…", &s[..n - 1]) }
}

fn fmt(v: Option<f64>, dp: usize) -> String {
    match v {
        Some(x) => format!("{x:.*}", dp),
        None => "—".to_string(),
    }
}
