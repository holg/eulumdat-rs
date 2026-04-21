//! Verify numeric columns, lists, and nested structs round-trip exactly.

mod common;
use common::{load, tmp_parquet};

use std::fs::File;

use arrow::array::{
    Array, Float64Array, Int32Array, ListArray, StringArray, StructArray, UInt32Array, UInt8Array,
};
use eulumdat_parquet::EulumdatParquetWriter;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

/// Write one LDT, read back as a single batch, return the batch.
fn write_one_and_read(ldt: &eulumdat::Eulumdat, label: &str) -> arrow::record_batch::RecordBatch {
    let tmp = tmp_parquet(label);
    let mut w = EulumdatParquetWriter::create(&tmp).unwrap();
    w.append(label, ldt).unwrap();
    w.finish().unwrap();

    let file = File::open(&tmp).unwrap();
    let mut reader = ParquetRecordBatchReaderBuilder::try_new(file)
        .unwrap()
        .build()
        .unwrap();
    let batch = reader
        .next()
        .expect("at least one batch")
        .expect("batch read ok");
    assert_eq!(batch.num_rows(), 1);

    let _ = std::fs::remove_file(&tmp);
    batch
}

fn col_str<'a>(batch: &'a arrow::record_batch::RecordBatch, name: &str) -> &'a str {
    let idx = batch.schema().index_of(name).expect("column exists");
    let arr = batch
        .column(idx)
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("string column");
    arr.value(0)
}

fn col_f64(batch: &arrow::record_batch::RecordBatch, name: &str) -> f64 {
    let idx = batch.schema().index_of(name).expect("column exists");
    let arr = batch
        .column(idx)
        .as_any()
        .downcast_ref::<Float64Array>()
        .expect("f64 column");
    arr.value(0)
}

fn col_u32(batch: &arrow::record_batch::RecordBatch, name: &str) -> u32 {
    let idx = batch.schema().index_of(name).expect("column exists");
    let arr = batch
        .column(idx)
        .as_any()
        .downcast_ref::<UInt32Array>()
        .expect("u32 column");
    arr.value(0)
}

fn col_u8(batch: &arrow::record_batch::RecordBatch, name: &str) -> u8 {
    let idx = batch.schema().index_of(name).expect("column exists");
    let arr = batch
        .column(idx)
        .as_any()
        .downcast_ref::<UInt8Array>()
        .expect("u8 column");
    arr.value(0)
}

#[test]
fn identity_and_classification_fields() {
    // fluorescent template has known values we can assert exactly
    let ldt = load("fluorescent_luminaire.ldt");
    let batch = write_one_and_read(&ldt, "identity");

    assert_eq!(col_str(&batch, "identification"), &ldt.identification);
    assert_eq!(col_str(&batch, "luminaire_name"), &ldt.luminaire_name);
    assert_eq!(col_str(&batch, "luminaire_number"), &ldt.luminaire_number);
    assert_eq!(col_str(&batch, "date_user"), &ldt.date_user);
    assert_eq!(
        col_str(&batch, "measurement_report_number"),
        &ldt.measurement_report_number
    );

    // Spec enums: stable integer discriminants from the EULUMDAT spec
    // (Ityp and Isym), not Rust variant names.
    assert_eq!(col_u8(&batch, "type_indicator"), ldt.type_indicator as u8);
    assert_eq!(col_u8(&batch, "symmetry"), ldt.symmetry as u8);
}

#[test]
fn grid_and_dimension_scalars() {
    let ldt = load("fluorescent_luminaire.ldt");
    let batch = write_one_and_read(&ldt, "grid");

    assert_eq!(col_u32(&batch, "num_c_planes"), ldt.num_c_planes as u32);
    assert_eq!(col_u32(&batch, "num_g_planes"), ldt.num_g_planes as u32);
    assert_eq!(col_f64(&batch, "c_plane_distance"), ldt.c_plane_distance);
    assert_eq!(col_f64(&batch, "g_plane_distance"), ldt.g_plane_distance);

    assert_eq!(col_f64(&batch, "length"), ldt.length);
    assert_eq!(col_f64(&batch, "width"), ldt.width);
    assert_eq!(col_f64(&batch, "height"), ldt.height);
    assert_eq!(
        col_f64(&batch, "luminous_area_length"),
        ldt.luminous_area_length
    );
}

#[test]
fn optical_scalars() {
    let ldt = load("fluorescent_luminaire.ldt");
    let batch = write_one_and_read(&ldt, "optical");

    assert_eq!(
        col_f64(&batch, "downward_flux_fraction"),
        ldt.downward_flux_fraction
    );
    assert_eq!(
        col_f64(&batch, "light_output_ratio"),
        ldt.light_output_ratio
    );
    assert_eq!(col_f64(&batch, "conversion_factor"), ldt.conversion_factor);
    assert_eq!(col_f64(&batch, "tilt_angle"), ldt.tilt_angle);
}

#[test]
fn direct_ratios_list_roundtrip() {
    let ldt = load("fluorescent_luminaire.ldt");
    let batch = write_one_and_read(&ldt, "direct-ratios");

    let idx = batch.schema().index_of("direct_ratios").unwrap();
    let list = batch
        .column(idx)
        .as_any()
        .downcast_ref::<ListArray>()
        .expect("list column");

    // Row 0's sub-array
    let sub = list.value(0);
    let values = sub
        .as_any()
        .downcast_ref::<Float64Array>()
        .expect("f64 inside list");

    assert_eq!(values.len(), 10, "direct_ratios is always 10 values");
    for i in 0..10 {
        assert_eq!(
            values.value(i),
            ldt.direct_ratios[i],
            "direct_ratios[{i}] mismatch"
        );
    }
}

#[test]
fn lamp_sets_nested_struct_roundtrip() {
    let ldt = load("fluorescent_luminaire.ldt");
    assert!(
        !ldt.lamp_sets.is_empty(),
        "fluorescent template has lamp sets"
    );

    let batch = write_one_and_read(&ldt, "lamp-sets");
    let idx = batch.schema().index_of("lamp_sets").unwrap();
    let list = batch
        .column(idx)
        .as_any()
        .downcast_ref::<ListArray>()
        .expect("list<struct>");
    let sub = list.value(0);
    let s = sub
        .as_any()
        .downcast_ref::<StructArray>()
        .expect("struct values");

    assert_eq!(s.len(), ldt.lamp_sets.len(), "lamp_sets count matches");

    let num_lamps = s
        .column_by_name("num_lamps")
        .unwrap()
        .as_any()
        .downcast_ref::<Int32Array>()
        .unwrap();
    let lamp_type = s
        .column_by_name("lamp_type")
        .unwrap()
        .as_any()
        .downcast_ref::<StringArray>()
        .unwrap();
    let flux = s
        .column_by_name("total_luminous_flux")
        .unwrap()
        .as_any()
        .downcast_ref::<Float64Array>()
        .unwrap();
    let color = s
        .column_by_name("color_appearance")
        .unwrap()
        .as_any()
        .downcast_ref::<StringArray>()
        .unwrap();
    let cri = s
        .column_by_name("color_rendering_group")
        .unwrap()
        .as_any()
        .downcast_ref::<StringArray>()
        .unwrap();
    let watt = s
        .column_by_name("wattage_with_ballast")
        .unwrap()
        .as_any()
        .downcast_ref::<Float64Array>()
        .unwrap();

    for (i, lamp) in ldt.lamp_sets.iter().enumerate() {
        assert_eq!(num_lamps.value(i), lamp.num_lamps);
        assert_eq!(lamp_type.value(i), lamp.lamp_type);
        assert_eq!(flux.value(i), lamp.total_luminous_flux);
        assert_eq!(color.value(i), lamp.color_appearance);
        assert_eq!(cri.value(i), lamp.color_rendering_group);
        assert_eq!(watt.value(i), lamp.wattage_with_ballast);
    }
}

#[cfg(feature = "summary")]
#[test]
fn summary_matches_photometric_summary() {
    // The summary columns should contain exactly what PhotometricSummary produces.
    use eulumdat::PhotometricSummary;

    let ldt = load("fluorescent_luminaire.ldt");
    let expected = PhotometricSummary::from_eulumdat(&ldt);
    let batch = write_one_and_read(&ldt, "summary-fidelity");

    assert_eq!(col_f64(&batch, "total_lamp_flux"), expected.total_lamp_flux);
    assert_eq!(col_f64(&batch, "calculated_flux"), expected.calculated_flux);
    assert_eq!(col_f64(&batch, "lor"), expected.lor);
    assert_eq!(col_f64(&batch, "dlor"), expected.dlor);
    assert_eq!(col_f64(&batch, "ulor"), expected.ulor);
    assert_eq!(col_f64(&batch, "lamp_efficacy"), expected.lamp_efficacy);
    assert_eq!(
        col_f64(&batch, "luminaire_efficacy"),
        expected.luminaire_efficacy
    );
    assert_eq!(col_f64(&batch, "beam_angle"), expected.beam_angle);
    assert_eq!(col_f64(&batch, "field_angle"), expected.field_angle);
    assert_eq!(col_f64(&batch, "max_intensity"), expected.max_intensity);

    // Stable snake_case constants (not Rust Debug output).
    use eulumdat::{DistributionType, LightDirection};
    let expected_dir = match expected.primary_direction {
        LightDirection::Downward => "downward",
        LightDirection::Upward => "upward",
    };
    let expected_dist = match expected.distribution_type {
        DistributionType::Direct => "direct",
        DistributionType::Indirect => "indirect",
        DistributionType::DirectIndirect => "direct_indirect",
        DistributionType::IndirectDirect => "indirect_direct",
    };
    assert_eq!(col_str(&batch, "primary_direction"), expected_dir);
    assert_eq!(col_str(&batch, "distribution_type"), expected_dist);
}
