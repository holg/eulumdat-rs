//! Basic round-trip: write several sample LDTs, read them back, verify row
//! count and string-column values.

mod common;
use common::{load, tmp_parquet};

use std::fs::File;
use std::sync::Arc;

use arrow::array::RecordBatchReader;
use eulumdat_parquet::EulumdatParquetWriter;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

#[test]
fn write_and_read_sample_catalog() {
    let tmp = tmp_parquet("roundtrip");

    let samples = [
        ("fluorescent", load("fluorescent_luminaire.ldt")),
        ("road", load("road_luminaire.ldt")),
        ("uplight", load("floor_uplight.ldt")),
        ("projector", load("projector.ldt")),
    ];

    let mut w = EulumdatParquetWriter::create(&tmp).unwrap();
    for (name, ldt) in &samples {
        w.append(name, ldt).unwrap();
    }
    w.finish().unwrap();

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

        let arr = batch
            .column(0)
            .as_any()
            .downcast_ref::<arrow::array::StringArray>()
            .unwrap();
        for i in 0..batch.num_rows() {
            names.push(arr.value(i).to_string());
        }

        let arr = batch
            .column(1)
            .as_any()
            .downcast_ref::<arrow::array::StringArray>()
            .unwrap();
        for i in 0..batch.num_rows() {
            identifications.push(arr.value(i).to_string());
        }
    }

    assert_eq!(total_rows, samples.len());
    assert_eq!(names[0], "fluorescent");
    assert_eq!(names[3], "projector");
    for (i, id) in identifications.iter().enumerate() {
        assert!(!id.is_empty(), "sample {i} has empty identification");
    }

    let _ = std::fs::remove_file(&tmp);
}

#[cfg(feature = "summary")]
#[test]
fn summary_columns_present() {
    let tmp = tmp_parquet("summary-schema");

    let ldt = load("fluorescent_luminaire.ldt");
    let mut w = EulumdatParquetWriter::create(&tmp).unwrap();
    w.append("fluorescent", &ldt).unwrap();
    w.finish().unwrap();

    let file = File::open(&tmp).unwrap();
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)
        .unwrap()
        .build()
        .unwrap();
    let schema: Arc<arrow::datatypes::Schema> = reader.schema();

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
