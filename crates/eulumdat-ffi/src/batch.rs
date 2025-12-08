//! Batch conversion types and functions for FFI

use eulumdat::Eulumdat as CoreEulumdat;

use crate::error::EulumdatError;

/// Result of converting a single file
#[derive(Debug, Clone, uniffi::Record)]
pub struct ConversionResult {
    pub input_path: String,
    pub output_path: String,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Statistics for batch conversion
#[derive(Debug, Clone, uniffi::Record)]
pub struct BatchConversionStats {
    pub total_files: u32,
    pub successful: u32,
    pub failed: u32,
    pub results: Vec<ConversionResult>,
}

/// Output format for batch conversion
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ConversionFormat {
    Ies,
    Ldt,
}

/// Input file for batch conversion
#[derive(Debug, Clone, uniffi::Record)]
pub struct BatchInputFile {
    pub name: String,
    pub content: String,
    /// Optional input format (auto-detected if None)
    pub format: Option<InputFormat>,
}

/// Input file format
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum InputFormat {
    Ldt,
    Ies,
}

/// Output file from batch conversion
#[derive(Debug, Clone, uniffi::Record)]
pub struct BatchOutputFile {
    pub input_name: String,
    pub output_name: String,
    pub content: Option<String>,
    pub error: Option<String>,
}

/// Convert a single LDT file to IES format
#[uniffi::export]
pub fn convert_ldt_to_ies(ldt_content: String) -> Result<String, EulumdatError> {
    let ldt =
        CoreEulumdat::parse(&ldt_content).map_err(|e| EulumdatError::ParseError(e.to_string()))?;
    Ok(eulumdat::IesExporter::export(&ldt))
}

/// Convert a single LDT file content to another LDT (normalize/clean)
#[uniffi::export]
pub fn convert_ldt_to_ldt(ldt_content: String) -> Result<String, EulumdatError> {
    let ldt =
        CoreEulumdat::parse(&ldt_content).map_err(|e| EulumdatError::ParseError(e.to_string()))?;
    Ok(ldt.to_ldt())
}

/// Batch convert multiple LDT contents to IES format
/// Returns a list of (original_name, ies_content or error)
#[uniffi::export]
pub fn batch_convert_to_ies(files: Vec<BatchInputFile>) -> BatchConversionStats {
    let mut results = Vec::new();
    let mut successful = 0u32;
    let mut failed = 0u32;

    for file in &files {
        match CoreEulumdat::parse(&file.content) {
            Ok(ldt) => {
                let _ies_content = eulumdat::IesExporter::export(&ldt);
                let output_path = file.name.replace(".ldt", ".ies").replace(".LDT", ".ies");
                results.push(ConversionResult {
                    input_path: file.name.clone(),
                    output_path,
                    success: true,
                    error_message: None,
                });
                successful += 1;
            }
            Err(e) => {
                results.push(ConversionResult {
                    input_path: file.name.clone(),
                    output_path: String::new(),
                    success: false,
                    error_message: Some(e.to_string()),
                });
                failed += 1;
            }
        }
    }

    BatchConversionStats {
        total_files: files.len() as u32,
        successful,
        failed,
        results,
    }
}

/// Batch convert and return the converted contents
///
/// This is a thin FFI wrapper around eulumdat::batch::batch_convert()
#[uniffi::export]
pub fn batch_convert_contents(
    files: Vec<BatchInputFile>,
    format: ConversionFormat,
) -> Vec<BatchOutputFile> {
    // Convert FFI types to core types
    let core_inputs: Vec<eulumdat::BatchInput> = files
        .into_iter()
        .map(|f| eulumdat::BatchInput {
            name: f.name,
            content: f.content,
            format: f.format.map(|fmt| match fmt {
                InputFormat::Ldt => eulumdat::InputFormat::Ldt,
                InputFormat::Ies => eulumdat::InputFormat::Ies,
            }),
        })
        .collect();

    let core_format = match format {
        ConversionFormat::Ies => eulumdat::ConversionFormat::Ies,
        ConversionFormat::Ldt => eulumdat::ConversionFormat::Ldt,
    };

    // Call core batch conversion
    let core_outputs = eulumdat::batch::batch_convert(&core_inputs, core_format);

    // Convert core types back to FFI types
    core_outputs
        .into_iter()
        .map(|o| BatchOutputFile {
            input_name: o.input_name,
            output_name: o.output_name,
            content: o.content,
            error: o.error,
        })
        .collect()
}
