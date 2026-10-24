//! Error types for the eulumdat crate.

#[allow(unused_imports)]
use crate::prelude::*;
pub use anyhow::{anyhow, Error, Result};

/// Create a parse error at a specific line.
pub fn parse_error(line: usize, message: impl core::fmt::Display) -> Error {
    anyhow!("Parse error at line {}: {}", line, message)
}

/// Create an invalid value error.
pub fn invalid_value(field: impl core::fmt::Display, message: impl core::fmt::Display) -> Error {
    anyhow!("Invalid value for {}: {}", field, message)
}
