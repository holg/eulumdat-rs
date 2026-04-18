//! Apache Parquet export for eulumdat photometric files.
//!
//! Produces one row per luminaire. See the crate README for schema details.

mod schema;
mod writer;

pub use schema::build_schema;
pub use writer::EulumdatParquetWriter;
