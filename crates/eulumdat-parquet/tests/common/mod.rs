//! Shared test helpers.

use std::path::PathBuf;

use eulumdat::Eulumdat;

/// Load a template LDT by its file name (e.g., "fluorescent_luminaire.ldt").
pub fn load(rel: &str) -> Eulumdat {
    let path = format!("../eulumdat-wasm/templates/{rel}");
    let content = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    Eulumdat::parse(&content).unwrap_or_else(|e| panic!("parse {path}: {e}"))
}

/// Load via IesParser. Returns `None` if the file can't be parsed.
#[allow(dead_code)]
pub fn load_ies(rel: &str) -> Option<Eulumdat> {
    let path = format!("../eulumdat-wasm/templates/{rel}");
    let content = std::fs::read_to_string(&path).ok()?;
    eulumdat::IesParser::parse(&content).ok()
}

/// A unique tempfile path for the current test.
pub fn tmp_parquet(label: &str) -> PathBuf {
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("eulumdat-parquet-{label}-{pid}-{nanos}.parquet"))
}
