//! Diagram-related types for Python bindings

use pyo3::prelude::*;

use ::eulumdat as core;
use core::diagram::SvgTheme as CoreSvgTheme;

/// SVG theme for diagram rendering.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SvgTheme {
    /// Light theme with white background
    Light = 0,
    /// Dark theme with dark background
    Dark = 1,
    /// CSS variables for dynamic theming
    CssVariables = 2,
}

#[pymethods]
impl SvgTheme {
    fn __repr__(&self) -> String {
        match self {
            Self::Light => "SvgTheme.Light".to_string(),
            Self::Dark => "SvgTheme.Dark".to_string(),
            Self::CssVariables => "SvgTheme.CssVariables".to_string(),
        }
    }
}

impl SvgTheme {
    pub(crate) fn to_core(self) -> CoreSvgTheme {
        match self {
            Self::Light => CoreSvgTheme::light(),
            Self::Dark => CoreSvgTheme::dark(),
            Self::CssVariables => CoreSvgTheme::css_variables(),
        }
    }
}
