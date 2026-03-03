//! SPDX (IES TM-27-14) file parser
//!
//! Parses spectral power distribution files in the IES TM-27-14 format (.spdx).
//! These files contain only spectral data, no photometric distribution.
//!
//! When loaded, creates a partial LuminaireOpticalData with spectral information
//! but missing intensity distribution data.

use crate::types::{Emitter, Header, LuminaireOpticalData, SpectralDistribution, SpectralUnits};
use anyhow::{anyhow, Result};
use quick_xml::events::Event;
use quick_xml::Reader;

/// Metadata from SPDX header
#[derive(Debug, Clone, Default)]
pub struct SpdxHeader {
    pub manufacturer: Option<String>,
    pub catalog_number: Option<String>,
    pub description: Option<String>,
    pub document_creator: Option<String>,
    pub laboratory: Option<String>,
    pub unique_identifier: Option<String>,
    pub report_number: Option<String>,
    pub report_date: Option<String>,
    pub document_creation_date: Option<String>,
    pub comments: Option<String>,
}

/// Spectral data from SPDX file
#[derive(Debug, Clone, Default)]
pub struct SpdxData {
    pub header: SpdxHeader,
    pub spectral_quantity: String,
    pub bandwidth_fwhm: Option<f64>,
    pub bandwidth_corrected: Option<bool>,
    pub wavelengths: Vec<f64>,
    pub values: Vec<f64>,
}

/// Parse an SPDX (IES TM-27-14) file
pub fn parse(content: &str) -> Result<SpdxData> {
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(true);

    let mut spdx = SpdxData::default();
    let mut current_element = String::new();
    let mut in_header = false;
    let mut in_spectral = false;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                current_element = name.clone();

                match name.as_str() {
                    "Header" => in_header = true,
                    "SpectralDistribution" => in_spectral = true,
                    "SpectralData" => {
                        // Parse wavelength attribute
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"wavelength" {
                                if let Ok(wl) = String::from_utf8_lossy(&attr.value).parse::<f64>()
                                {
                                    spdx.wavelengths.push(wl);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match name.as_str() {
                    "Header" => in_header = false,
                    "SpectralDistribution" => in_spectral = false,
                    _ => {}
                }
                current_element.clear();
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape().unwrap_or_default().trim().to_string();
                if text.is_empty() {
                    continue;
                }

                if in_header {
                    match current_element.as_str() {
                        "Manufacturer" => spdx.header.manufacturer = Some(text),
                        "CatalogNumber" => spdx.header.catalog_number = Some(text),
                        "Description" => spdx.header.description = Some(text),
                        "DocumentCreator" => spdx.header.document_creator = Some(text),
                        "Laboratory" => spdx.header.laboratory = Some(text),
                        "UniqueIdentifier" => spdx.header.unique_identifier = Some(text),
                        "ReportNumber" => spdx.header.report_number = Some(text),
                        "ReportDate" => spdx.header.report_date = Some(text),
                        "DocumentCreationDate" => spdx.header.document_creation_date = Some(text),
                        "Comments" => spdx.header.comments = Some(text),
                        _ => {}
                    }
                } else if in_spectral {
                    match current_element.as_str() {
                        "SpectralQuantity" => spdx.spectral_quantity = text,
                        "BandwidthFWHM" => spdx.bandwidth_fwhm = text.parse().ok(),
                        "BandwidthCorrected" => {
                            spdx.bandwidth_corrected = Some(text.to_lowercase() == "true")
                        }
                        "SpectralData" => {
                            // Parse value
                            if let Ok(val) = text.parse::<f64>() {
                                spdx.values.push(val);
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(anyhow!("XML parse error: {}", e)),
            _ => {}
        }
        buf.clear();
    }

    if spdx.wavelengths.is_empty() || spdx.values.is_empty() {
        return Err(anyhow!("No spectral data found in SPDX file"));
    }

    if spdx.wavelengths.len() != spdx.values.len() {
        return Err(anyhow!(
            "Wavelength count ({}) doesn't match value count ({})",
            spdx.wavelengths.len(),
            spdx.values.len()
        ));
    }

    Ok(spdx)
}

/// Convert SPDX data to SpectralDistribution
impl From<&SpdxData> for SpectralDistribution {
    fn from(spdx: &SpdxData) -> Self {
        let units = if spdx.spectral_quantity.to_lowercase().contains("relative") {
            SpectralUnits::Relative
        } else {
            SpectralUnits::WattsPerNanometer
        };

        SpectralDistribution {
            wavelengths: spdx.wavelengths.clone(),
            values: spdx.values.clone(),
            units,
            start_wavelength: None,
            wavelength_interval: None,
        }
    }
}

/// Convert SPDX to a partial LuminaireOpticalData
///
/// Creates a document with spectral data but NO photometric distribution.
/// The intensity distribution will be empty, which should trigger warnings
/// in the UI about missing data.
pub fn to_atla(spdx: &SpdxData) -> LuminaireOpticalData {
    let mut doc = LuminaireOpticalData {
        header: Header {
            manufacturer: spdx.header.manufacturer.clone(),
            catalog_number: spdx.header.catalog_number.clone(),
            description: spdx.header.description.clone(),
            document_creator: spdx.header.document_creator.clone(),
            laboratory: spdx.header.laboratory.clone(),
            unique_identifier: spdx.header.unique_identifier.clone(),
            report_number: spdx.header.report_number.clone(),
            report_date: spdx.header.report_date.clone(),
            document_creation_date: spdx.header.document_creation_date.clone(),
            ..Default::default()
        },
        ..Default::default()
    };

    // Create an emitter with spectral data but no intensity distribution
    let emitter = Emitter {
        description: spdx.header.description.clone(),
        spectral_distribution: Some(SpectralDistribution::from(spdx)),
        quantity: 1,
        // No intensity_distribution - this is spectral-only data
        ..Default::default()
    };

    doc.emitters = vec![emitter];

    doc
}

/// Check if content looks like SPDX format
pub fn is_spdx(content: &str) -> bool {
    content.contains("IESTM2714") || content.contains("SpectralDistribution")
}

/// Get warnings for SPDX-loaded data
pub fn get_warnings(spdx: &SpdxData) -> Vec<String> {
    let mut warnings = Vec::new();

    warnings.push(
        "SPDX file contains spectral data only - no photometric (intensity) distribution."
            .to_string(),
    );
    warnings.push(
        "Polar/cartesian diagrams will be empty. Only spectral diagram available.".to_string(),
    );

    if spdx.header.manufacturer.is_none() || spdx.header.manufacturer.as_deref() == Some("Unknown")
    {
        warnings.push("Manufacturer information missing or unknown.".to_string());
    }

    if spdx.wavelengths.len() < 20 {
        warnings.push(format!(
            "Limited spectral resolution: only {} data points.",
            spdx.wavelengths.len()
        ));
    }

    // Check wavelength range
    let min_wl = spdx.wavelengths.iter().copied().fold(f64::MAX, f64::min);
    let max_wl = spdx.wavelengths.iter().copied().fold(f64::MIN, f64::max);

    if min_wl > 400.0 {
        warnings.push(format!(
            "Spectral data starts at {:.0}nm (missing blue/violet region).",
            min_wl
        ));
    }
    if max_wl < 700.0 {
        warnings.push(format!(
            "Spectral data ends at {:.0}nm (missing red region).",
            max_wl
        ));
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_SPDX: &str = r#"<?xml version="1.0"?>
<IESTM2714 xmlns="iestm2714" version="1.0">
    <Header>
        <Manufacturer>Test Corp</Manufacturer>
        <Description>Test LED</Description>
    </Header>
    <SpectralDistribution>
        <SpectralQuantity>relative</SpectralQuantity>
        <SpectralData wavelength="450.0">0.5</SpectralData>
        <SpectralData wavelength="550.0">1.0</SpectralData>
        <SpectralData wavelength="650.0">0.3</SpectralData>
    </SpectralDistribution>
</IESTM2714>"#;

    #[test]
    fn test_parse_spdx() {
        let result = parse(SAMPLE_SPDX);
        assert!(result.is_ok());

        let spdx = result.unwrap();
        assert_eq!(spdx.header.manufacturer, Some("Test Corp".to_string()));
        assert_eq!(spdx.wavelengths.len(), 3);
        assert_eq!(spdx.values.len(), 3);
        assert_eq!(spdx.wavelengths[1], 550.0);
        assert_eq!(spdx.values[1], 1.0);
    }

    #[test]
    fn test_is_spdx() {
        assert!(is_spdx(SAMPLE_SPDX));
        assert!(!is_spdx("<LuminaireOpticalData>"));
    }

    #[test]
    fn test_to_atla() {
        let spdx = parse(SAMPLE_SPDX).unwrap();
        let atla = to_atla(&spdx);

        assert_eq!(atla.header.manufacturer, Some("Test Corp".to_string()));
        assert_eq!(atla.emitters.len(), 1);
        assert!(atla.emitters[0].spectral_distribution.is_some());
    }

    #[test]
    fn test_get_warnings() {
        let spdx = parse(SAMPLE_SPDX).unwrap();
        let warnings = get_warnings(&spdx);

        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.contains("spectral data only")));
    }
}
