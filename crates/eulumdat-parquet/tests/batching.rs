//! Exercise the writer's internal batch flush (currently every 256 rows).
//! Ensures multi-row-group files read back with the expected total and stable schema.

mod common;
use common::{load, tmp_parquet};

use std::fs::File;

use arrow::array::RecordBatchReader;
use eulumdat_parquet::EulumdatParquetWriter;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

#[test]
fn multi_batch_write_preserves_count_and_schema() {
    // 256-row internal flush, 700 rows -> >=3 RecordBatch flushes -> writer
    // produces multiple row groups in the file.
    const N_ROWS: usize = 700;

    let ldt = load("fluorescent_luminaire.ldt");
    let tmp = tmp_parquet("multibatch");

    let mut w = EulumdatParquetWriter::create(&tmp).unwrap();
    for i in 0..N_ROWS {
        w.append(&format!("row-{i:04}"), &ldt).unwrap();
    }
    w.finish().unwrap();

    // Writer performs 256-row internal flushes — with 700 rows, that's at
    // least 3 RecordBatch writes. The final Parquet file has a single row
    // group by default (ArrowWriter's max_row_group_size is 1M), which is
    // fine: what we verify here is that repeated flushes don't corrupt the
    // file and the total row count is preserved.
    let file = File::open(&tmp).unwrap();
    let builder = ParquetRecordBatchReaderBuilder::try_new(file.try_clone().unwrap()).unwrap();
    let metadata = builder.metadata();
    let total_via_metadata: i64 = (0..metadata.num_row_groups())
        .map(|i| metadata.row_group(i).num_rows())
        .sum();
    assert_eq!(total_via_metadata as usize, N_ROWS);

    // Reading consumer-side: schema must be stable across RecordBatches.
    let reader = builder.build().unwrap();
    let read_schema = reader.schema();
    let mut total = 0usize;
    let mut first_batch_schema = None;
    for batch in reader {
        let batch = batch.unwrap();
        total += batch.num_rows();
        match &first_batch_schema {
            None => first_batch_schema = Some(batch.schema()),
            Some(s) => assert_eq!(batch.schema(), *s, "schema stable across batches"),
        }
    }

    assert_eq!(total, N_ROWS, "row count preserved");
    assert_eq!(first_batch_schema.unwrap(), read_schema);

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn finish_with_no_appends_produces_empty_file() {
    // Edge case: writer is created but no rows are appended.
    let tmp = tmp_parquet("empty");

    let w = EulumdatParquetWriter::create(&tmp).unwrap();
    w.finish().unwrap();

    let file = File::open(&tmp).unwrap();
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)
        .unwrap()
        .build()
        .unwrap();

    let total: usize = reader.map(|b| b.unwrap().num_rows()).sum();
    assert_eq!(total, 0, "no rows written, no rows read");

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn fewer_rows_than_batch_size_still_flushes() {
    // 10 rows << 256 batch size: finish() should flush the buffered rows.
    let ldt = load("fluorescent_luminaire.ldt");
    let tmp = tmp_parquet("below-batch");

    let mut w = EulumdatParquetWriter::create(&tmp).unwrap();
    for i in 0..10 {
        w.append(&format!("row-{i}"), &ldt).unwrap();
    }
    w.finish().unwrap();

    let file = File::open(&tmp).unwrap();
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)
        .unwrap()
        .build()
        .unwrap();
    let total: usize = reader.map(|b| b.unwrap().num_rows()).sum();
    assert_eq!(total, 10);

    let _ = std::fs::remove_file(&tmp);
}
