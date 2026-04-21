//! Catalog-scale analytics over a Parquet file.
//!
//! These tests double as usage documentation: they show the kind of queries
//! a lighting designer would run against a manufacturer catalog — filtering
//! by beam angle, ranking by efficacy, aggregating flux per lamp type — all
//! through the Arrow column APIs, without pulling in DuckDB/Polars.

#![cfg(feature = "summary")]

mod common;
use common::{load, tmp_parquet};

use std::fs::File;

use arrow::array::{Array, BooleanArray, Float64Array, StringArray};
use arrow::record_batch::RecordBatch;
use eulumdat_parquet::EulumdatParquetWriter;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

/// Build a small "catalog" from the bundled templates, covering a realistic
/// mix of luminaire types: fluorescent, road, uplight, projector.
fn write_test_catalog() -> std::path::PathBuf {
    let entries = [
        ("fluorescent_luminaire.ldt", "Osram T16 54W"),
        ("road_luminaire.ldt", "Osram Street Light"),
        ("floor_uplight.ldt", "Osram Floor Uplight"),
        ("projector.ldt", "Osram Projector"),
    ];

    let tmp = tmp_parquet("analytics-catalog");
    let mut w = EulumdatParquetWriter::create(&tmp).unwrap();
    for (file, label) in &entries {
        let ldt = load(file);
        w.append(label, &ldt).unwrap();
    }
    w.finish().unwrap();
    tmp
}

fn read_all(path: &std::path::Path) -> Vec<RecordBatch> {
    let file = File::open(path).unwrap();
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)
        .unwrap()
        .build()
        .unwrap();
    reader.map(|b| b.unwrap()).collect()
}

fn f64_col(batch: &RecordBatch, name: &str) -> Vec<f64> {
    let idx = batch.schema().index_of(name).unwrap();
    let arr = batch
        .column(idx)
        .as_any()
        .downcast_ref::<Float64Array>()
        .unwrap();
    (0..arr.len()).map(|i| arr.value(i)).collect()
}

fn string_col(batch: &RecordBatch, name: &str) -> Vec<String> {
    let idx = batch.schema().index_of(name).unwrap();
    let arr = batch
        .column(idx)
        .as_any()
        .downcast_ref::<StringArray>()
        .unwrap();
    (0..arr.len()).map(|i| arr.value(i).to_string()).collect()
}

fn bool_col(batch: &RecordBatch, name: &str) -> Vec<bool> {
    let idx = batch.schema().index_of(name).unwrap();
    let arr = batch
        .column(idx)
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap();
    (0..arr.len()).map(|i| arr.value(i)).collect()
}

/// Query: "Rank luminaires by beam angle — tightest first."
/// This demonstrates sorting a numeric column. The projector template has
/// a narrower beam than the others, but we don't hard-code an expected
/// value — PhotometricCalculations may evolve its definition.
#[test]
fn rank_by_beam_angle() {
    let tmp = write_test_catalog();
    let batches = read_all(&tmp);
    let batch = &batches[0];

    let names = string_col(batch, "luminaire_name");
    let beam = f64_col(batch, "beam_angle");

    let mut ranked: Vec<(String, f64)> = names.into_iter().zip(beam).collect();
    ranked.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    eprintln!("Beam angle ranking (narrowest first):");
    for (n, v) in &ranked {
        eprintln!("  {v:>6.1}°  {n}");
    }

    // Sanity: beam angle is finite. Some luminaires have a sentinel of 360°
    // (no defined beam, e.g., diffuse distributions), which is also valid.
    for (_, v) in &ranked {
        assert!(v.is_finite(), "beam angle must be a finite number, got {v}");
    }

    let _ = std::fs::remove_file(&tmp);
}

/// Query: "Rank luminaires by efficacy (lm/W) and return the top one."
/// This is the kind of comparison a lighting designer does at spec time.
#[test]
fn rank_by_luminaire_efficacy() {
    let tmp = write_test_catalog();
    let batches = read_all(&tmp);
    let batch = &batches[0];

    let names = string_col(batch, "luminaire_name");
    let efficacy = f64_col(batch, "luminaire_efficacy");

    let (top_idx, top_val) = efficacy
        .iter()
        .enumerate()
        .filter(|(_, v)| v.is_finite() && **v > 0.0)
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .expect("at least one luminaire has positive efficacy");

    eprintln!("Top efficacy: {} @ {:.1} lm/W", names[top_idx], top_val);
    assert!(
        *top_val > 0.0 && top_val.is_finite(),
        "top efficacy must be a real positive number"
    );
}

/// Query: "Total catalog flux and average downward flux fraction."
/// Aggregations are the most common analytics workload.
#[test]
fn aggregate_total_flux_and_dlor() {
    let tmp = write_test_catalog();
    let batches = read_all(&tmp);
    let batch = &batches[0];

    let fluxes = f64_col(batch, "total_lamp_flux");
    let dlor = f64_col(batch, "dlor");

    let total_flux: f64 = fluxes.iter().sum();
    let avg_dlor: f64 = dlor.iter().sum::<f64>() / dlor.len() as f64;

    eprintln!(
        "Catalog: {} luminaires, total flux {:.0} lm, avg DLOR {:.1}%",
        fluxes.len(),
        total_flux,
        avg_dlor
    );

    // Sanity: >0 flux, DLOR in [0,100]
    assert!(total_flux > 0.0, "catalog has positive total flux");
    assert!(
        (0.0..=100.0).contains(&avg_dlor),
        "DLOR must be a percentage"
    );
}

/// Query: "Group luminaires by primary direction (Downward/Upward)."
/// Demonstrates the categorical column being queryable as a string partition key.
#[test]
fn partition_by_primary_direction() {
    let tmp = write_test_catalog();
    let batches = read_all(&tmp);
    let batch = &batches[0];

    let names = string_col(batch, "luminaire_name");
    let dir = string_col(batch, "primary_direction");

    let mut downward = Vec::new();
    let mut upward = Vec::new();
    for (n, d) in names.iter().zip(&dir) {
        match d.as_str() {
            "downward" => downward.push(n.clone()),
            "upward" => upward.push(n.clone()),
            _ => {}
        }
    }

    eprintln!("Downward: {downward:?}");
    eprintln!("Upward:   {upward:?}");

    // The floor_uplight template is literally an uplight — it should be in
    // the upward bucket. Other samples are downlights.
    assert!(
        upward.iter().any(|n| n.to_lowercase().contains("uplight")),
        "floor_uplight should partition as Upward (got {upward:?})"
    );
    assert!(
        !downward.is_empty(),
        "at least one luminaire must be Downward"
    );
}

/// Query: "Find batwing distributions."
/// Boolean column filtering — a trivial DuckDB WHERE clause.
#[test]
fn filter_batwing_distributions() {
    let tmp = write_test_catalog();
    let batches = read_all(&tmp);
    let batch = &batches[0];

    let names = string_col(batch, "luminaire_name");
    let batwing = bool_col(batch, "is_batwing");

    let batwings: Vec<_> = names
        .iter()
        .zip(&batwing)
        .filter_map(|(n, b)| b.then_some(n.clone()))
        .collect();

    eprintln!("Batwing luminaires in catalog: {batwings:?}");
    // We don't assert a specific count — just that the column is queryable
    // and yields sensible results.
    for n in &batwings {
        assert!(!n.is_empty(), "batwing entry has a name");
    }
}
