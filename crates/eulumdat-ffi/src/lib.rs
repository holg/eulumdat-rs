//! FFI bindings for eulumdat-core
//!
//! This crate provides UniFFI bindings to expose the eulumdat-core library
//! to Swift, Kotlin, Python, and other languages.
//!
//! # Generating Bindings
//!
//! To generate bindings for different languages:
//!
//! ```bash
//! # Swift
//! cargo run --bin uniffi-bindgen generate --library target/release/libeulumdat_ffi.dylib --language swift --out-dir ./bindings/swift
//!
//! # Kotlin
//! cargo run --bin uniffi-bindgen generate --library target/release/libeulumdat_ffi.dylib --language kotlin --out-dir ./bindings/kotlin
//!
//! # Python
//! cargo run --bin uniffi-bindgen generate --library target/release/libeulumdat_ffi.dylib --language python --out-dir ./bindings/python
//! ```

pub mod batch;
pub mod bug_rating;
pub mod diagram;
pub mod error;
pub mod types;
pub mod validation;

// Re-export all public types and functions
pub use batch::{
    batch_convert_contents, batch_convert_to_ies, convert_ldt_to_ies, convert_ldt_to_ldt,
    BatchConversionStats, BatchInputFile, BatchOutputFile, ConversionFormat, ConversionResult,
    InputFormat,
};
pub use bug_rating::{
    calculate_bug_rating, generate_bug_diagram, generate_bug_svg, generate_lcs_svg, BugDiagramData,
    BugRatingData, ZoneLumens,
};
pub use diagram::{
    generate_butterfly_diagram, generate_butterfly_svg, generate_cartesian_diagram,
    generate_cartesian_svg, generate_heatmap_diagram, generate_heatmap_svg, generate_polar_diagram,
    generate_polar_svg, ButterflyDiagramData, ButterflyWing, CPlaneDirection, CartesianCurve,
    CartesianDiagramData, CartesianPoint, Color, DiagramScale, HeatmapCell, HeatmapDiagramData,
    LegendEntry, Point2D, PolarCurve, PolarDiagramData, PolarPoint, SvgThemeType,
};
pub use error::EulumdatError;
pub use types::{
    export_ies, export_ldt, parse_ies, parse_ldt, Eulumdat, LampSet, Symmetry, TypeIndicator,
};
pub use validation::{
    get_validation_errors, validate_ldt, validate_ldt_strict, ValidationError, ValidationWarning,
};

uniffi::setup_scaffolding!();
