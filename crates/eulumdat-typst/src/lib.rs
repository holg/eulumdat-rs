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

mod error;
mod generator;
pub mod template;
#[cfg(feature = "compile")]
mod world;

pub use error::{ReportError, Result};
pub use generator::{PaperSize, ReportGenerator, ReportOptions, ReportSection};
pub use template::{generate_comparison_report, generate_typst_with_files};
