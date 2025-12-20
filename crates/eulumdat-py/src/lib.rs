//! Python bindings for the eulumdat photometric file library.
//!
//! This module provides PyO3-based Python bindings for parsing, writing,
//! and analyzing EULUMDAT (LDT) and IES photometric files, as well as
//! ATLA (Advanced Technical Lighting Application) format support.

pub mod atla_types;
pub mod batch;
pub mod bug_rating;
pub mod calculations;
pub mod diagram;
pub mod error;
pub mod types;
pub mod validation;

use pyo3::prelude::*;

use atla_types::{AtlaDocument, ColorRendering, Emitter, SpectralDistribution};
use batch::{BatchInput, BatchOutput, BatchStats, ConversionFormat, InputFormat};
use bug_rating::{BugRating, ZoneLumens};
use calculations::{
    CieFluxCodes, GldfPhotometricData, PhotometricSummary, UgrParams, ZonalLumens30,
};
use diagram::SvgTheme;
use types::{Eulumdat, LampSet, Symmetry, TypeIndicator};
use validation::ValidationWarning;

/// Python bindings for the eulumdat photometric file library.
#[pymodule]
fn eulumdat(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // ATLA types (primary data structure)
    m.add_class::<AtlaDocument>()?;
    m.add_class::<Emitter>()?;
    m.add_class::<SpectralDistribution>()?;
    m.add_class::<ColorRendering>()?;

    // Legacy Eulumdat types (for backward compatibility)
    m.add_class::<Eulumdat>()?;
    m.add_class::<TypeIndicator>()?;
    m.add_class::<Symmetry>()?;
    m.add_class::<LampSet>()?;

    // Diagram types
    m.add_class::<SvgTheme>()?;

    // Validation types
    m.add_class::<ValidationWarning>()?;

    // BUG rating types
    m.add_class::<BugRating>()?;
    m.add_class::<ZoneLumens>()?;

    // Photometric calculation types
    m.add_class::<PhotometricSummary>()?;
    m.add_class::<GldfPhotometricData>()?;
    m.add_class::<CieFluxCodes>()?;
    m.add_class::<ZonalLumens30>()?;
    m.add_class::<UgrParams>()?;

    // Batch conversion types
    m.add_class::<InputFormat>()?;
    m.add_class::<ConversionFormat>()?;
    m.add_class::<BatchInput>()?;
    m.add_class::<BatchOutput>()?;
    m.add_class::<BatchStats>()?;

    // Batch conversion function
    m.add_function(wrap_pyfunction!(batch::batch_convert, m)?)?;

    Ok(())
}
