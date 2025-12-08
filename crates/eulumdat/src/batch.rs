//! Batch conversion utilities for processing multiple EULUMDAT files at once.
//!
//! This module provides efficient batch processing of LDT files, allowing
//! conversion of multiple files in a single operation.

use crate::{Eulumdat, IesExporter};

/// Input file for batch conversion
#[derive(Debug, Clone)]
pub struct BatchInput {
    /// Name/identifier for this file (e.g., filename)
    pub name: String,
    /// Raw file content (LDT or IES format)
    pub content: String,
    /// Input format (auto-detected if not specified)
    pub format: Option<InputFormat>,
}

/// Input file format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputFormat {
    /// EULUMDAT (.ldt) format
    Ldt,
    /// IES (.ies) format
    Ies,
}

/// Output from batch conversion
#[derive(Debug, Clone)]
pub struct BatchOutput {
    /// Original input name
    pub input_name: String,
    /// Suggested output name (with converted extension)
    pub output_name: String,
    /// Converted content (if successful)
    pub content: Option<String>,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Target format for batch conversion
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionFormat {
    /// Convert to IES format
    Ies,
    /// Convert to LDT format (normalize/validate)
    Ldt,
}

/// Statistics from a batch conversion operation
#[derive(Debug, Clone)]
pub struct BatchStats {
    /// Total number of files processed
    pub total: usize,
    /// Number of successful conversions
    pub successful: usize,
    /// Number of failed conversions
    pub failed: usize,
}

/// Batch convert multiple LDT files to the specified format.
///
/// This function processes all files in a single operation, parsing each
/// LDT file and converting it to the target format. Failed conversions
/// are captured with error messages rather than stopping the batch.
///
/// # Arguments
///
/// * `inputs` - Vector of input files with names and contents
/// * `format` - Target format (IES or LDT)
///
/// # Returns
///
/// Vector of outputs, one per input, in the same order. Each output contains
/// either the converted content or an error message.
///
/// # Example
///
/// ```no_run
/// use eulumdat::batch::{BatchInput, ConversionFormat, batch_convert};
///
/// let inputs = vec![
///     BatchInput {
///         name: "lamp1.ldt".to_string(),
///         content: std::fs::read_to_string("lamp1.ldt").unwrap(),
///         format: None, // auto-detect
///     },
/// ];
///
/// let outputs = batch_convert(&inputs, ConversionFormat::Ies);
/// for output in outputs {
///     if let Some(content) = output.content {
///         std::fs::write(&output.output_name, content).unwrap();
///     }
/// }
/// ```
pub fn batch_convert(inputs: &[BatchInput], format: ConversionFormat) -> Vec<BatchOutput> {
    inputs
        .iter()
        .map(|input| convert_single(input, format))
        .collect()
}

/// Batch convert with statistics.
///
/// Same as `batch_convert` but also returns summary statistics.
pub fn batch_convert_with_stats(
    inputs: &[BatchInput],
    format: ConversionFormat,
) -> (Vec<BatchOutput>, BatchStats) {
    let outputs = batch_convert(inputs, format);
    let stats = BatchStats {
        total: outputs.len(),
        successful: outputs.iter().filter(|o| o.content.is_some()).count(),
        failed: outputs.iter().filter(|o| o.error.is_some()).count(),
    };
    (outputs, stats)
}

/// Convert a single file
fn convert_single(input: &BatchInput, output_format: ConversionFormat) -> BatchOutput {
    // Detect input format if not specified
    let input_format = input
        .format
        .unwrap_or_else(|| detect_format(&input.content));

    // Parse based on input format
    let parse_result = match input_format {
        InputFormat::Ldt => Eulumdat::parse(&input.content),
        InputFormat::Ies => crate::IesParser::parse(&input.content),
    };

    match parse_result {
        Ok(ldt) => {
            let content = match output_format {
                ConversionFormat::Ies => IesExporter::export(&ldt),
                ConversionFormat::Ldt => ldt.to_ldt(),
            };

            let extension = match output_format {
                ConversionFormat::Ies => ".ies",
                ConversionFormat::Ldt => ".ldt",
            };

            // Strip old extension and add new one
            let output_name = input
                .name
                .trim_end_matches(".ldt")
                .trim_end_matches(".LDT")
                .trim_end_matches(".ies")
                .trim_end_matches(".IES")
                .to_string()
                + extension;

            BatchOutput {
                input_name: input.name.clone(),
                output_name,
                content: Some(content),
                error: None,
            }
        }
        Err(e) => BatchOutput {
            input_name: input.name.clone(),
            output_name: String::new(),
            content: None,
            error: Some(e.to_string()),
        },
    }
}

/// Auto-detect file format based on content
fn detect_format(content: &str) -> InputFormat {
    // IES files typically start with IESNA header
    if content.trim_start().starts_with("IESNA") {
        InputFormat::Ies
    } else {
        // Default to LDT
        InputFormat::Ldt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_LDT: &str = r#"Test Company
1
1
1
0
19
5
0
Test Lamp
Product
Test
2021
1500
0
129
1486
0
0
0
0
0
100
100.0
1.0
0.0
1
1
LED
19800
3000
80
195
0.358
0.468
0.545
0.619
0.675
0.733
0.776
0.802
0.847
0.874
0
0
5
10
15
20
25
30
35
40
45
50
55
60
65
70
75
80
85
90
386.8
384.3
377.1
365.4
349.7
330.3
307.8
283.0
256.5
228.7
200.5
172.3
144.3
116.2
88.6
62.3
38.4
17.8
0
"#;

    const TEST_IES: &str = r#"IESNA:LM-63-2002
[TEST] Test
[MANUFAC] Test Manufacturer
TILT=NONE
1 19800 1 19 1 1 2 1 1 0
1.0 1.0 195
0 5 10 15 20 25 30 35 40 45 50 55 60 65 70 75 80 85 90
0
386.8 384.3 377.1 365.4 349.7 330.3 307.8 283.0 256.5 228.7 200.5 172.3 144.3 116.2 88.6 62.3 38.4 17.8 0
"#;

    #[test]
    fn test_ldt_to_ies() {
        let inputs = vec![BatchInput {
            name: "test.ldt".to_string(),
            content: TEST_LDT.to_string(),
            format: Some(InputFormat::Ldt),
        }];

        let outputs = batch_convert(&inputs, ConversionFormat::Ies);
        assert_eq!(outputs.len(), 1);
        assert!(outputs[0].content.is_some());
        assert!(outputs[0].error.is_none());
        assert_eq!(outputs[0].output_name, "test.ies");
    }

    #[test]
    fn test_ies_to_ldt() {
        let inputs = vec![BatchInput {
            name: "test.ies".to_string(),
            content: TEST_IES.to_string(),
            format: Some(InputFormat::Ies),
        }];

        let outputs = batch_convert(&inputs, ConversionFormat::Ldt);
        assert_eq!(outputs.len(), 1);
        assert!(outputs[0].content.is_some());
        assert!(outputs[0].error.is_none());
        assert_eq!(outputs[0].output_name, "test.ldt");
    }

    #[test]
    fn test_auto_detect_format() {
        let ies_input = BatchInput {
            name: "test.ies".to_string(),
            content: TEST_IES.to_string(),
            format: None, // Auto-detect
        };

        let ldt_input = BatchInput {
            name: "test.ldt".to_string(),
            content: TEST_LDT.to_string(),
            format: None, // Auto-detect
        };

        let outputs = batch_convert(&[ies_input, ldt_input], ConversionFormat::Ldt);
        assert_eq!(outputs.len(), 2);
        assert!(outputs[0].content.is_some(), "IES should parse");
        assert!(outputs[1].content.is_some(), "LDT should parse");
    }

    #[test]
    fn test_batch_convert_with_errors() {
        let inputs = vec![
            BatchInput {
                name: "good.ldt".to_string(),
                content: TEST_LDT.to_string(),
                format: Some(InputFormat::Ldt),
            },
            BatchInput {
                name: "bad.ldt".to_string(),
                content: "invalid content".to_string(),
                format: Some(InputFormat::Ldt),
            },
        ];

        let (outputs, stats) = batch_convert_with_stats(&inputs, ConversionFormat::Ies);
        assert_eq!(stats.total, 2);
        assert_eq!(stats.successful, 1);
        assert_eq!(stats.failed, 1);
        assert!(outputs[0].content.is_some());
        assert!(outputs[1].error.is_some());
    }
}
