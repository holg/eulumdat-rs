//! Batch conversion functionality for Python bindings

use pyo3::prelude::*;

use eulumdat::batch::{
    self, BatchInput as CoreBatchInput, ConversionFormat as CoreConversionFormat,
};

/// Input file format for batch conversion
#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputFormat {
    /// EULUMDAT (.ldt) format
    Ldt = 0,
    /// IES (.ies) format
    Ies = 1,
}

#[pymethods]
impl InputFormat {
    fn __repr__(&self) -> String {
        match self {
            Self::Ldt => "InputFormat.Ldt".to_string(),
            Self::Ies => "InputFormat.Ies".to_string(),
        }
    }
}

/// Output format for batch conversion
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ConversionFormat {
    /// Convert to EULUMDAT (.ldt) format
    Ldt = 0,
    /// Convert to IES (.ies) format
    Ies = 1,
}

#[pymethods]
impl ConversionFormat {
    fn __repr__(&self) -> String {
        match self {
            Self::Ldt => "ConversionFormat.Ldt".to_string(),
            Self::Ies => "ConversionFormat.Ies".to_string(),
        }
    }
}

/// Input file for batch conversion
#[pyclass]
#[derive(Clone)]
pub struct BatchInput {
    /// File name
    #[pyo3(get, set)]
    pub name: String,
    /// File content
    #[pyo3(get, set)]
    pub content: String,
    /// Optional input format (auto-detected if None)
    #[pyo3(get, set)]
    pub format: Option<InputFormat>,
}

#[pymethods]
impl BatchInput {
    #[new]
    #[pyo3(signature = (name, content, format=None))]
    fn new(name: String, content: String, format: Option<InputFormat>) -> Self {
        Self {
            name,
            content,
            format,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "BatchInput(name='{}', content_len={}, format={:?})",
            self.name,
            self.content.len(),
            self.format
        )
    }
}

/// Output file from batch conversion
#[pyclass]
#[derive(Clone)]
pub struct BatchOutput {
    /// Original input file name
    #[pyo3(get)]
    pub input_name: String,
    /// Generated output file name
    #[pyo3(get)]
    pub output_name: String,
    /// Converted content (None if error)
    #[pyo3(get)]
    pub content: Option<String>,
    /// Error message (None if successful)
    #[pyo3(get)]
    pub error: Option<String>,
}

#[pymethods]
impl BatchOutput {
    fn __repr__(&self) -> String {
        if let Some(err) = &self.error {
            format!("BatchOutput(input='{}', error='{}')", self.input_name, err)
        } else {
            format!(
                "BatchOutput(input='{}', output='{}', content_len={})",
                self.input_name,
                self.output_name,
                self.content.as_ref().map(|c| c.len()).unwrap_or(0)
            )
        }
    }

    /// Check if the conversion was successful
    #[getter]
    fn success(&self) -> bool {
        self.error.is_none()
    }
}

/// Statistics from batch conversion
#[pyclass]
#[derive(Clone)]
pub struct BatchStats {
    /// Total number of files processed
    #[pyo3(get)]
    pub total: usize,
    /// Number of successful conversions
    #[pyo3(get)]
    pub successful: usize,
    /// Number of failed conversions
    #[pyo3(get)]
    pub failed: usize,
}

#[pymethods]
impl BatchStats {
    fn __repr__(&self) -> String {
        format!(
            "BatchStats(total={}, successful={}, failed={})",
            self.total, self.successful, self.failed
        )
    }

    /// Success rate as a percentage
    #[getter]
    fn success_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.successful as f64 / self.total as f64) * 100.0
        }
    }
}

/// Batch convert multiple photometric files.
///
/// This function is significantly faster than converting files one-by-one in Python
/// because the conversion happens entirely in Rust.
///
/// Args:
///     inputs: List of BatchInput objects containing file names and contents
///     format: Target conversion format (Ldt or Ies)
///
/// Returns:
///     Tuple of (list of BatchOutput, BatchStats)
///
/// Example:
///     >>> inputs = [
///     ...     BatchInput("file1.ldt", ldt_content1),
///     ...     BatchInput("file2.ldt", ldt_content2),
///     ... ]
///     >>> outputs, stats = batch_convert(inputs, ConversionFormat.Ies)
///     >>> print(f"Converted {stats.successful}/{stats.total} files")
#[pyfunction]
#[pyo3(signature = (inputs, format))]
pub fn batch_convert(
    inputs: Vec<BatchInput>,
    format: ConversionFormat,
) -> (Vec<BatchOutput>, BatchStats) {
    // Convert Python types to Rust types
    let core_inputs: Vec<CoreBatchInput> = inputs
        .into_iter()
        .map(|input| CoreBatchInput {
            name: input.name,
            content: input.content,
            format: input.format.map(|f| match f {
                InputFormat::Ldt => eulumdat::InputFormat::Ldt,
                InputFormat::Ies => eulumdat::InputFormat::Ies,
            }),
        })
        .collect();

    let core_format = match format {
        ConversionFormat::Ldt => CoreConversionFormat::Ldt,
        ConversionFormat::Ies => CoreConversionFormat::Ies,
    };

    // Perform batch conversion
    let (core_outputs, core_stats) = batch::batch_convert_with_stats(&core_inputs, core_format);

    // Convert results back to Python types
    let outputs: Vec<BatchOutput> = core_outputs
        .into_iter()
        .map(|output| BatchOutput {
            input_name: output.input_name,
            output_name: output.output_name,
            content: output.content,
            error: output.error,
        })
        .collect();

    let stats = BatchStats {
        total: core_stats.total,
        successful: core_stats.successful,
        failed: core_stats.failed,
    };

    (outputs, stats)
}
