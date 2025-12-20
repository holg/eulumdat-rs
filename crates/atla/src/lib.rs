//! ATLA S001 / ANSI/IES TM-33 / UNI 11733 luminaire optical data library
//!
//! This crate provides parsing, writing, and conversion for the ATLA S001 standard
//! and its equivalent standards ANSI/IES TM-33-18, TM-33-23, and UNI 11733:2019.
//!
//! # Overview
//!
//! The ATLA S001 standard (also published as TM-33 and UNI 11733) is a modern XML/JSON
//! format for luminaire optical data, designed to replace the legacy IES LM-63 and
//! EULUMDAT formats. Key features include:
//!
//! - **Spectral data support** - Full spectral power distribution (SPD)
//! - **Multiple intensity metrics** - Luminous, radiant, photon, and spectral
//! - **Data provenance** - Track whether data is measured or simulated
//! - **Color metrics** - CCT, CRI (Ra, R9), and TM-30 (Rf, Rg)
//! - **Extensible** - Custom data fields for application-specific needs
//! - **TM-33-23 support** - Symmetry types, multipliers, angular spectral/color data
//!
//! # Schema Version Support
//!
//! | Schema | Root Element | Version |
//! |--------|--------------|---------|
//! | ATLA S001 / TM-33-18 | `LuminaireOpticalData` | 1.0 |
//! | TM-33-23 (IESTM33-22) | `IESTM33-22` | 1.1 |
//!
//! # Format Support
//!
//! | Feature | XML | JSON |
//! |---------|-----|------|
//! | ATLA S001 | ✅ | ✅ |
//! | TM-33-18 | ✅ | - |
//! | TM-33-23 | ✅ | ✅ |
//! | UNI 11733 | ✅ | - |
//! | ATLA S001-A | ✅ | ✅ |
//!
//! # Example
//!
//! ```rust,ignore
//! use atla::{LuminaireOpticalData, xml};
//!
//! // Parse from XML
//! let xml_content = std::fs::read_to_string("luminaire.xml")?;
//! let doc = xml::parse(&xml_content)?;
//!
//! // Access data
//! println!("Manufacturer: {:?}", doc.header.manufacturer);
//! println!("Total flux: {} lm", doc.total_luminous_flux());
//!
//! // With the 'json' feature, convert to JSON (90% smaller)
//! #[cfg(feature = "json")]
//! let json_output = atla::json::write(&doc)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Conversion to/from EULUMDAT
//!
//! With the `eulumdat` feature enabled, you can convert between formats:
//!
//! ```rust,ignore
//! use atla::LuminaireOpticalData;
//! use eulumdat::Eulumdat;
//!
//! // LDT -> ATLA
//! let ldt = Eulumdat::from_file("luminaire.ldt")?;
//! let atla = LuminaireOpticalData::from_eulumdat(&ldt);
//!
//! // ATLA -> LDT
//! let ldt_back = atla.to_eulumdat();
//! ```
//!
//! # Feature Flags
//!
//! - `xml` (default) - XML parsing with `quick-xml`
//! - `json` - JSON parsing with `serde_json`
//! - `serde` - Serde derive for all types
//! - `eulumdat` - Conversion to/from EULUMDAT format

pub mod error;
pub mod greenhouse;
pub mod labels;
pub mod spectral;
pub mod tm30;
pub mod types;
pub mod validate;

#[cfg(feature = "xml")]
pub mod xml;

#[cfg(feature = "json")]
pub mod json;

#[cfg(feature = "eulumdat")]
pub mod convert;

// Re-exports
pub use error::{AtlaError, Result};
pub use greenhouse::{GreenhouseDiagram, GreenhouseLabels, GreenhouseTheme};
pub use labels::SpectralLabels;
pub use spectral::{
    synthesize_spectrum, SpectralDiagram, SpectralMetrics, SpectralSvgLabels, SpectralTheme,
};
pub use tm30::{calculate_tm30, Tm30Result, Tm30Theme};
pub use types::*;
pub use validate::{
    validate, validate_with_schema, ValidationMessage, ValidationResult, ValidationSchema,
};

/// Detect schema version from XML content
///
/// Checks for known root elements:
/// - `<IESTM33-22>` → TM-33-23 (SchemaVersion::Tm3323)
/// - `<LuminaireOpticalData>` → ATLA S001 (SchemaVersion::AtlaS001)
///
/// # Example
/// ```rust
/// use atla::{detect_schema_version, SchemaVersion};
///
/// let xml = r#"<IESTM33-22><Version>1.1</Version></IESTM33-22>"#;
/// assert_eq!(detect_schema_version(xml), SchemaVersion::Tm3323);
///
/// let xml2 = r#"<LuminaireOpticalData version="1.0"></LuminaireOpticalData>"#;
/// assert_eq!(detect_schema_version(xml2), SchemaVersion::AtlaS001);
/// ```
pub fn detect_schema_version(content: &str) -> SchemaVersion {
    let trimmed = content.trim();

    // Check for TM-33-23 root element
    if trimmed.contains("<IESTM33-22") || trimmed.contains("<IESTM33-22>") {
        return SchemaVersion::Tm3323;
    }

    // Check for ATLA S001 root element
    if trimmed.contains("<LuminaireOpticalData") {
        return SchemaVersion::AtlaS001;
    }

    // Default to S001 for unknown formats
    SchemaVersion::AtlaS001
}

/// Parse ATLA document from string, auto-detecting format (XML or JSON)
pub fn parse(content: &str) -> Result<LuminaireOpticalData> {
    let trimmed = content.trim();

    if trimmed.starts_with('{') {
        // JSON format
        #[cfg(feature = "json")]
        {
            json::parse(content)
        }
        #[cfg(not(feature = "json"))]
        {
            Err(AtlaError::JsonParse(
                "JSON support not enabled. Enable the 'json' feature.".to_string(),
            ))
        }
    } else if trimmed.starts_with('<') {
        // XML format
        #[cfg(feature = "xml")]
        {
            xml::parse(content)
        }
        #[cfg(not(feature = "xml"))]
        {
            Err(AtlaError::XmlParse(
                "XML support not enabled. Enable the 'xml' feature.".to_string(),
            ))
        }
    } else {
        Err(AtlaError::XmlParse(
            "Unknown format: content must start with '<' (XML) or '{' (JSON)".to_string(),
        ))
    }
}

/// Parse ATLA document from file, auto-detecting format from extension
pub fn parse_file(path: impl AsRef<std::path::Path>) -> Result<LuminaireOpticalData> {
    let path = path.as_ref();
    let content = std::fs::read_to_string(path)?;

    // Try to detect from extension first
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        match ext.to_lowercase().as_str() {
            "json" => {
                #[cfg(feature = "json")]
                return json::parse(&content);
                #[cfg(not(feature = "json"))]
                return Err(AtlaError::JsonParse(
                    "JSON support not enabled. Enable the 'json' feature.".to_string(),
                ));
            }
            "xml" => {
                #[cfg(feature = "xml")]
                return xml::parse(&content);
                #[cfg(not(feature = "xml"))]
                return Err(AtlaError::XmlParse(
                    "XML support not enabled. Enable the 'xml' feature.".to_string(),
                ));
            }
            _ => {}
        }
    }

    // Fall back to content-based detection
    parse(&content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_detect_xml() {
        let xml = r#"<?xml version="1.0"?>
<LuminaireOpticalData version="1.0">
    <Header><Manufacturer>Test</Manufacturer></Header>
    <Emitter><Quantity>1</Quantity></Emitter>
</LuminaireOpticalData>"#;

        let doc = parse(xml).unwrap();
        assert_eq!(doc.header.manufacturer, Some("Test".to_string()));
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_auto_detect_json() {
        let json =
            r#"{"version":"1.0","header":{"manufacturer":"Test"},"emitters":[{"quantity":1}]}"#;

        let doc = parse(json).unwrap();
        assert_eq!(doc.header.manufacturer, Some("Test".to_string()));
    }
}
