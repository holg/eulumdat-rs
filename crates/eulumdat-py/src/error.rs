//! Error handling for Python bindings

use pyo3::exceptions::{PyIOError, PyValueError};
use pyo3::PyErr;

/// Convert eulumdat::Error to PyErr
pub fn to_py_err(err: eulumdat::Error) -> PyErr {
    PyValueError::new_err(err.to_string())
}

/// Convert atla::AtlaError to PyErr
pub fn atla_to_py_err(err: atla::AtlaError) -> PyErr {
    PyValueError::new_err(err.to_string())
}

/// Convert std::io::Error to PyErr
pub fn io_to_py_err(err: std::io::Error) -> PyErr {
    PyIOError::new_err(err.to_string())
}

/// Helper trait for converting Results to Python-compatible Results
pub trait ToPyResult<T> {
    fn to_py(self) -> pyo3::PyResult<T>;
}

impl<T> ToPyResult<T> for Result<T, eulumdat::Error> {
    fn to_py(self) -> pyo3::PyResult<T> {
        self.map_err(to_py_err)
    }
}

impl<T> ToPyResult<T> for Result<T, atla::AtlaError> {
    fn to_py(self) -> pyo3::PyResult<T> {
        self.map_err(atla_to_py_err)
    }
}

impl<T> ToPyResult<T> for Result<T, std::io::Error> {
    fn to_py(self) -> pyo3::PyResult<T> {
        self.map_err(io_to_py_err)
    }
}
