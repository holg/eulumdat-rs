//! Inputs with varied symmetry types and from different parsers (LDT, IES).

mod common;
use common::{load, load_ies, tmp_parquet};

use std::fs::File;

use arrow::array::{RecordBatchReader, StringArray};
use eulumdat_parquet::EulumdatParquetWriter;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

/// Read the `symmetry` column across all rows.
fn collect_symmetries(path: &std::path::Path) -> Vec<String> {
    let file = File::open(path).unwrap();
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)
        .unwrap()
        .build()
        .unwrap();
    let mut out = Vec::new();
    for batch in reader {
        let batch = batch.unwrap();
        let idx = batch.schema().index_of("symmetry").unwrap();
        let arr = batch
            .column(idx)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        for i in 0..batch.num_rows() {
            out.push(arr.value(i).to_string());
        }
    }
    out
}

#[test]
fn mixed_symmetry_inputs_serialize() {
    // Templates with file-name pattern `<sym>-<something>-<something>.ldt`
    // cover Symmetry variants None(0), VerticalAxis(1), PlaneC0C180(2),
    // PlaneC90C270(3), BothPlanes(4).
    let samples = [
        ("sym-0", load("0-0-0.ldt")),
        ("sym-1", load("1-0-0.ldt")),
        ("sym-2", load("0-2-0.ldt")),
        ("sym-3", load("0-3-0.ldt")),
        ("sym-4", load("0-4-0.ldt")),
    ];

    let tmp = tmp_parquet("mixed-sym");
    let mut w = EulumdatParquetWriter::create(&tmp).unwrap();
    for (label, ldt) in &samples {
        w.append(label, ldt).unwrap();
    }
    w.finish().unwrap();

    let syms = collect_symmetries(&tmp);
    assert_eq!(syms.len(), samples.len());

    // Each row's symmetry string must match the source LDT's Debug repr.
    for (i, (_, ldt)) in samples.iter().enumerate() {
        assert_eq!(syms[i], format!("{:?}", ldt.symmetry));
    }

    // We should see at least 3 distinct symmetry values across the 5 samples.
    let mut unique: Vec<_> = syms.clone();
    unique.sort();
    unique.dedup();
    assert!(
        unique.len() >= 3,
        "expected varied symmetries, got {unique:?}"
    );

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn ies_parsed_input_writes_same_schema() {
    // Real IES file, parsed via IesParser into an Eulumdat, then written
    // through the same pipeline. Schema must be identical.
    let ies = match load_ies(
        "S01.01.02.354_MAXWELL-8-T4 LUXEON 5050 Square with glass-MEASURED_IESNA.ies",
    ) {
        Some(l) => l,
        None => {
            eprintln!("SKIP: IES template unavailable");
            return;
        }
    };
    let ldt = load("fluorescent_luminaire.ldt");

    let tmp_ies = tmp_parquet("ies");
    let tmp_ldt = tmp_parquet("ldt");

    let mut w = EulumdatParquetWriter::create(&tmp_ies).unwrap();
    w.append("ies", &ies).unwrap();
    w.finish().unwrap();

    let mut w = EulumdatParquetWriter::create(&tmp_ldt).unwrap();
    w.append("ldt", &ldt).unwrap();
    w.finish().unwrap();

    // Compare schemas — must match byte for byte.
    let schema_ies = ParquetRecordBatchReaderBuilder::try_new(File::open(&tmp_ies).unwrap())
        .unwrap()
        .build()
        .unwrap()
        .schema();
    let schema_ldt = ParquetRecordBatchReaderBuilder::try_new(File::open(&tmp_ldt).unwrap())
        .unwrap()
        .build()
        .unwrap()
        .schema();
    assert_eq!(
        schema_ies, schema_ldt,
        "schema must be identical regardless of input source format"
    );

    let _ = std::fs::remove_file(&tmp_ies);
    let _ = std::fs::remove_file(&tmp_ldt);
}
