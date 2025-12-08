//! Python bindings for the eulumdat photometric file library.
//!
//! This module provides PyO3-based Python bindings for parsing, writing,
//! and analyzing EULUMDAT (LDT) and IES photometric files.

pub mod batch;
pub mod bug_rating;
pub mod diagram;
pub mod error;
pub mod types;
pub mod validation;

use pyo3::prelude::*;

use batch::{BatchInput, BatchOutput, BatchStats, ConversionFormat, InputFormat};
use bug_rating::{BugRating, ZoneLumens};
use diagram::SvgTheme;
use types::{Eulumdat, LampSet, Symmetry, TypeIndicator};
use validation::ValidationWarning;

/// Python bindings for the eulumdat photometric file library.
#[pymodule]
fn eulumdat(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Core types
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
