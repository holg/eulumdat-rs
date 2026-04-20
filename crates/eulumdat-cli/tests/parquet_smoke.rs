//! Smoke test for `eulumdat export-parquet`.
//!
//! Built only when the parquet feature is enabled on eulumdat-cli.

#![cfg(feature = "parquet")]

use std::fs;

use assert_cmd::Command;

#[test]
fn export_parquet_produces_readable_file() {
    // Copy a handful of sample LDTs into a temp dir (keeps the test hermetic,
    // independent of anything else that might be under templates/).
    let tmp = tempfile::tempdir().unwrap();
    let input_dir = tmp.path().join("input");
    fs::create_dir_all(&input_dir).unwrap();

    let templates = [
        "fluorescent_luminaire.ldt",
        "road_luminaire.ldt",
        "projector.ldt",
        "floor_uplight.ldt",
    ];
    for t in &templates {
        let src = format!("../eulumdat-wasm/templates/{t}");
        let bytes = fs::read(&src).unwrap_or_else(|e| panic!("read {src}: {e}"));
        fs::write(input_dir.join(t), bytes).unwrap();
    }

    let output = tmp.path().join("catalog.parquet");

    Command::cargo_bin("eulumdat")
        .unwrap()
        .arg("export-parquet")
        .arg(&input_dir)
        .arg(&output)
        .assert()
        .success();

    assert!(output.exists(), "output parquet file was not created");
    let size = fs::metadata(&output).unwrap().len();
    assert!(size > 0, "output parquet file is empty");

    // Sanity: it's a valid parquet file with the expected row count.
    let file = fs::File::open(&output).unwrap();
    let builder =
        parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder::try_new(file).unwrap();
    let metadata = builder.metadata();
    let total: i64 = (0..metadata.num_row_groups())
        .map(|i| metadata.row_group(i).num_rows())
        .sum();
    assert_eq!(
        total as usize,
        templates.len(),
        "row count should match number of input files"
    );
}
