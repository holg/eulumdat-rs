//! Validation types for Python bindings

use pyo3::prelude::*;

/// Validation warning from the EULUMDAT specification.
#[pyclass]
#[derive(Clone)]
pub struct ValidationWarning {
    /// Warning code (e.g., "W001").
    #[pyo3(get)]
    pub code: String,
    /// Warning message.
    #[pyo3(get)]
    pub message: String,
}

#[pymethods]
impl ValidationWarning {
    fn __repr__(&self) -> String {
        format!("[{}] {}", self.code, self.message)
    }
}
