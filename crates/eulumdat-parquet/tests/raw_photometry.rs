//! Verify raw-photometry columns (c_angles, g_angles, intensities) round-trip
//! exactly when the `raw-photometry` feature is enabled.

#![cfg(feature = "raw-photometry")]

mod common;
use common::{load, tmp_parquet};

use std::fs::File;

use arrow::array::{Array, Float64Array, ListArray};
use eulumdat_parquet::EulumdatParquetWriter;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

fn read_list_f64(batch: &arrow::record_batch::RecordBatch, name: &str, row: usize) -> Vec<f64> {
    let idx = batch.schema().index_of(name).unwrap();
    let list = batch
        .column(idx)
        .as_any()
        .downcast_ref::<ListArray>()
        .expect("list column");
    let sub = list.value(row);
    let values = sub
        .as_any()
        .downcast_ref::<Float64Array>()
        .expect("f64 in list");
    (0..values.len()).map(|i| values.value(i)).collect()
}

#[test]
fn angles_and_intensities_roundtrip() {
    let ldt = load("fluorescent_luminaire.ldt");
    let tmp = tmp_parquet("raw-photometry");

    let mut w = EulumdatParquetWriter::create(&tmp).unwrap();
    w.append("fluorescent", &ldt).unwrap();
    w.finish().unwrap();

    let file = File::open(&tmp).unwrap();
    let mut reader = ParquetRecordBatchReaderBuilder::try_new(file)
        .unwrap()
        .build()
        .unwrap();
    let batch = reader.next().unwrap().unwrap();
    assert_eq!(batch.num_rows(), 1);

    // c_angles and g_angles exact match
    let c = read_list_f64(&batch, "c_angles", 0);
    let g = read_list_f64(&batch, "g_angles", 0);
    assert_eq!(c, ldt.c_angles, "c_angles exact match");
    assert_eq!(g, ldt.g_angles, "g_angles exact match");

    // intensities: list<list<double>>
    let idx = batch.schema().index_of("intensities").unwrap();
    let outer = batch
        .column(idx)
        .as_any()
        .downcast_ref::<ListArray>()
        .expect("outer list");
    let c_planes = outer.value(0);
    let c_planes = c_planes
        .as_any()
        .downcast_ref::<ListArray>()
        .expect("list<list>");
    assert_eq!(
        c_planes.len(),
        ldt.intensities.len(),
        "c-plane count matches"
    );

    for ci in 0..ldt.intensities.len() {
        let inner = c_planes.value(ci);
        let vals = inner
            .as_any()
            .downcast_ref::<Float64Array>()
            .expect("f64 inner");
        let got: Vec<f64> = (0..vals.len()).map(|i| vals.value(i)).collect();
        assert_eq!(got, ldt.intensities[ci], "intensities[{ci}] exact match");
    }

    let _ = std::fs::remove_file(&tmp);
}
