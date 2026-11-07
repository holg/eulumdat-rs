//! Typst report generation for photometric data.
//!
//! This crate generates Typst source files from EULUMDAT/IES photometric files,
//! including all available SVG diagrams and comprehensive textual information.
//! The generated Typst file can be compiled to PDF using the `typst` CLI.
//!
//! # Example
//!
//! ```ignore
//! use eulumdat::Eulumdat;
//! use eulumdat_typst::{ReportGenerator, ReportOptions};
//!
//! let ldt = Eulumdat::from_file("luminaire.ldt")?;
//! let generator = ReportGenerator::new(&ldt);
//! let typst_source = generator.generate_typst(&ReportOptions::default());
//! std::fs::write("report.typ", typst_source)?;
//! // Then compile with: typst compile report.typ report.pdf
//! ```
//!
//! # PDF Compilation (requires `compile` feature)
//!
//! With the `compile` feature enabled, you can compile directly to PDF:
//!
//! ```ignore
//! use eulumdat_typst::compile_to_pdf;
//!
//! let pdf_bytes = compile_to_pdf(&typst_source)?;
//! std::fs::write("report.pdf", pdf_bytes)?;
//! ```

pub mod colour_report;
mod error;
mod generator;
pub mod sensor_compare_report;
pub mod sensor_report;
pub mod template;
pub mod tm25_report;
#[cfg(feature = "compile")]
mod world;

pub use colour_report::{append_colour_quality_chapter, colour_quality_chapter};
pub use error::{ReportError, Result};
pub use generator::{PaperSize, ReportGenerator, ReportOptions, ReportSection};
pub use sensor_compare_report::{
    generate_sensor_comparison_report, CompareRoom, SensorCompareOptions,
};
pub use sensor_report::{
    append_sensor_chapter, generate_sensor_report, SensorReportOptions, SensorSource,
};
pub use template::{generate_comparison_report, generate_typst_with_files};
pub use tm25_report::{generate_tm25_report, Tm25ReportData};
