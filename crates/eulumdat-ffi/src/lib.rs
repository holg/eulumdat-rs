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

pub mod atla_types;
pub mod batch;
pub mod bim;
pub mod bug_rating;
pub mod compare;
pub mod diagram;
pub mod error;
pub mod schema_validation;
pub mod types;
pub mod validation;

// Re-export all public types and functions
// ATLA types (primary data structure)
pub use atla_types::{
    generate_atla_beam_angle_svg, generate_atla_bug_svg, generate_atla_butterfly_svg,
    generate_atla_cartesian_svg, generate_atla_cone_svg, generate_atla_heatmap_svg,
    generate_atla_lcs_svg, generate_atla_polar_svg, generate_greenhouse_svg,
    generate_greenhouse_svg_localized, generate_spectral_svg, generate_spectral_svg_localized,
    AtlaDocument, ColorRendering, Emitter, SpectralDistribution,
};
pub use batch::{
    batch_convert_contents, batch_convert_to_ies, convert_ldt_to_ies, convert_ldt_to_ldt,
    BatchConversionStats, BatchInputFile, BatchOutputFile, ConversionFormat, ConversionResult,
    InputFormat,
};
pub use bim::{get_bim_parameters, has_bim_data, BimData, BimParameterRow};
pub use bug_rating::{
    calculate_bug_rating, generate_bug_diagram, generate_bug_svg, generate_bug_svg_localized,
    generate_lcs_svg, generate_lcs_svg_localized, BugDiagramData, BugRatingData, ZoneLumens,
};
pub use compare::{
    compare_photometric, compare_photometric_localized, ComparisonMetricFfi,
    PhotometricComparisonResult, SignificanceLevel,
};
pub use diagram::{
    generate_beam_angle_svg, generate_beam_angle_svg_localized, generate_butterfly_diagram,
    generate_butterfly_svg, generate_butterfly_svg_localized, generate_cartesian_diagram,
    generate_cartesian_overlay_svg, generate_cartesian_svg, generate_cartesian_svg_for_plane,
    generate_cartesian_svg_localized, generate_cone_svg, generate_cone_svg_for_plane,
    generate_cone_svg_localized, generate_floodlight_cartesian_svg,
    generate_floodlight_cartesian_svg_localized, generate_heatmap_diagram, generate_heatmap_svg,
    generate_heatmap_svg_localized, generate_isocandela_svg, generate_isocandela_svg_localized,
    generate_isolux_svg, generate_isolux_svg_localized, generate_polar_diagram,
    generate_polar_overlay_svg, generate_polar_svg, generate_polar_svg_for_plane,
    generate_polar_svg_localized, get_expanded_c_angles, has_c_plane_variation,
    ButterflyDiagramData, ButterflyWing, CPlaneDirection, CartesianCurve, CartesianDiagramData,
    CartesianPoint, Color, DiagramScale, HeatmapCell, HeatmapDiagramData, Language, LegendEntry,
    Point2D, PolarCurve, PolarDiagramData, PolarPoint, SvgThemeType,
};
pub use error::EulumdatError;
pub use schema_validation::{
    validate_schema_s001, validate_schema_tm32, validate_schema_tm33, SchemaValidationMessage,
    SchemaValidationResult,
};
pub use types::{
    export_ies, export_ldt, parse_ies, parse_ldt, Eulumdat, LampSet, Symmetry, TypeIndicator,
};
pub use validation::{
    get_validation_errors, get_validation_errors_localized, validate_ldt, validate_ldt_localized,
    validate_ldt_strict, ValidationError, ValidationWarning,
};

uniffi::setup_scaffolding!();
