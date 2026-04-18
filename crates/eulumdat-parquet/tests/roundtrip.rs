//! Round-trip test: write several sample LDTs to a Parquet file, read back,
//! verify row count and key column values.

use std::fs::File;
use std::sync::Arc;

use arrow::array::RecordBatchReader;
use eulumdat::Eulumdat;
use eulumdat_parquet::EulumdatParquetWriter;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

fn load(rel: &str) -> Eulumdat {
    let path = format!("../eulumdat-wasm/templates/{rel}");
    let content = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    Eulumdat::parse(&content).unwrap_or_else(|e| panic!("parse {path}: {e}"))
}

#[test]
fn write_and_read_sample_catalog() {
    let tmp = std::env::temp_dir().join("eulumdat-parquet-roundtrip.parquet");

    let samples = [
        ("fluorescent", load("fluorescent_luminaire.ldt")),
        ("road", load("road_luminaire.ldt")),
        ("uplight", load("floor_uplight.ldt")),
        ("projector", load("projector.ldt")),
    ];

    // Write
    {
        let mut w = EulumdatParquetWriter::create(&tmp).unwrap();
        for (name, ldt) in &samples {
            w.append(name, ldt).unwrap();
        }
        w.finish().unwrap();
    }

    // Read back
    let file = File::open(&tmp).unwrap();
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)
        .unwrap()
        .build()
        .unwrap();

    let mut total_rows = 0usize;
    let mut names: Vec<String> = Vec::new();
    let mut identifications: Vec<String> = Vec::new();

    for batch in reader {
        let batch = batch.unwrap();
        total_rows += batch.num_rows();

        // file_path is column 0
        let col = batch.column(0);
        let arr = col
            .as_any()
            .downcast_ref::<arrow::array::StringArray>()
            .unwrap();
        for i in 0..batch.num_rows() {
            names.push(arr.value(i).to_string());
        }

        // identification is column 1
        let col = batch.column(1);
        let arr = col
            .as_any()
            .downcast_ref::<arrow::array::StringArray>()
            .unwrap();
        for i in 0..batch.num_rows() {
            identifications.push(arr.value(i).to_string());
        }
    }

    assert_eq!(total_rows, samples.len(), "row count matches input");
    assert_eq!(names[0], "fluorescent");
    assert_eq!(names[3], "projector");

    // Every sample should have non-empty identification
    for (i, id) in identifications.iter().enumerate() {
        assert!(
            !id.is_empty(),
            "sample {i} has empty identification — parser bug or missing field"
        );
    }

    // Cleanup
    let _ = std::fs::remove_file(&tmp);
}

#[cfg(feature = "summary")]
#[test]
fn summary_columns_present() {
    let tmp = std::env::temp_dir().join("eulumdat-parquet-summary.parquet");

    let ldt = load("fluorescent_luminaire.ldt");
    {
        let mut w = EulumdatParquetWriter::create(&tmp).unwrap();
        w.append("fluorescent", &ldt).unwrap();
        w.finish().unwrap();
    }

    let file = File::open(&tmp).unwrap();
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)
        .unwrap()
        .build()
        .unwrap();
    let schema: Arc<arrow::datatypes::Schema> = reader.schema();

    // Sanity: summary columns must exist
    for name in [
        "beam_angle",
        "field_angle",
        "luminaire_efficacy",
        "is_batwing",
        "primary_direction",
    ] {
        assert!(
            schema.field_with_name(name).is_ok(),
            "schema missing summary column '{name}'"
        );
    }

    let _ = std::fs::remove_file(&tmp);
}
