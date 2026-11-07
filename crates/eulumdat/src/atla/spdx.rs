//! SPDX (IES TM-27) reader and writer
//!
//! Spectral data documents in the IES TM-27 format (.spdx, "IES TM-27 SPDX
//! document"; TM-27-20 keeps the TM-27-14 layout: root `IESTM2714`,
//! namespace `http://www.ies.org/iestm2714`, and adds the optional
//! `MeasurementEquipment` header element). [`write()`] exports the spectrum
//! of a luminaire.
//! These files contain only spectral data, no photometric distribution.
//!
//! When loaded, creates a partial LuminaireOpticalData with spectral information
//! but missing intensity distribution data.

use crate::atla::types::{
    Emitter, Equipment, Header, LuminaireOpticalData, SpectralDistribution, SpectralUnits,
    SpectroradiometerInfo,
};
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
    /// TM-27-20: the equipment used to measure the spectrum.
    pub measurement_equipment: Option<String>,
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
                        "MeasurementEquipment" => spdx.header.measurement_equipment = Some(text),
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
            comments_list: spdx.header.comments.iter().cloned().collect(),
            ..Default::default()
        },
        equipment: (spdx.header.measurement_equipment.is_some() || spdx.bandwidth_fwhm.is_some())
            .then(|| Equipment {
                spectroradiometer: Some(SpectroradiometerInfo {
                    model: spdx.header.measurement_equipment.clone(),
                    resolution: spdx.bandwidth_fwhm,
                    ..Default::default()
                }),
                ..Default::default()
            }),
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

/// Options for [`write()`].
#[derive(Debug, Clone, Default)]
pub struct WriteOptions {
    /// Today's date (`YYYY-MM-DD`): the required `DocumentCreationDate`
    /// (else the source's creation date, when it is a date).
    pub today: Option<String>,
}

/// The spectrum of emitter `emitter` of `doc` as an IES TM-27-20 SPDX
/// document ("IES-TM-27", `.spdx`), `None` when it has no spectrum.
///
/// Relative spectra are scaled to a peak of 1 (`relative`: TM-27-20 asks for
/// 0 to 1.0), absolute ones are written as `flux` (W/nm). Required header
/// fields the source lacks: `Description` = the emitter or "Spectral power
/// distribution", `DocumentCreator` = laboratory or manufacturer, else
/// "Not specified".
pub fn write(doc: &LuminaireOpticalData, emitter: usize, opts: &WriteOptions) -> Option<String> {
    use crate::atla::tm33::{esc, iso_date, sig, spectrum_points, NOT_SPECIFIED};
    let em = doc.emitters.get(emitter)?;
    let spd = em.spectral_distribution.as_ref()?;
    let (w, v) = spectrum_points(spd);
    if w.is_empty() {
        return None;
    }
    let h = &doc.header;
    let ne = |v: &Option<String>| {
        v.as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    let spectro = doc
        .equipment
        .as_ref()
        .and_then(|e| e.spectroradiometer.as_ref());
    let mut x = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<IESTM2714 xmlns=\"http://www.ies.org/iestm2714\" version=\"1.0\">\n  <Header>\n",
    );
    let mut el = |name: &str, value: Option<String>| {
        if let Some(v) = value {
            x.push_str(&format!("    <{name}>{}</{name}>\n", esc(&v, false)));
        }
    };
    el("Manufacturer", ne(&h.manufacturer));
    el("CatalogNumber", ne(&h.catalog_number));
    el(
        "Description",
        Some(
            ne(&h.description)
                .or_else(|| ne(&em.description))
                .unwrap_or_else(|| "Spectral power distribution".to_string()),
        ),
    );
    el(
        "DocumentCreator",
        Some(
            ne(&h.document_creator)
                .or_else(|| ne(&h.laboratory))
                .or_else(|| ne(&h.manufacturer))
                .unwrap_or_else(|| NOT_SPECIFIED.to_string()),
        ),
    );
    el(
        "UniqueIdentifier",
        ne(&h.unique_identifier).or_else(|| ne(&h.uuid)),
    );
    el(
        "MeasurementEquipment",
        spectro.and_then(|s| ne(&s.model).or_else(|| ne(&s.manufacturer))),
    );
    el("Laboratory", ne(&h.laboratory));
    el("ReportNumber", ne(&h.report_number));
    el(
        "ReportDate",
        [&h.report_date, &h.test_date, &h.issue_date]
            .into_iter()
            .find_map(|d| ne(d).as_deref().and_then(iso_date)),
    );
    el(
        "DocumentCreationDate",
        opts.today
            .clone()
            .or_else(|| ne(&h.document_creation_date).as_deref().and_then(iso_date)),
    );
    let comments: Vec<String> = h
        .comments_list
        .iter()
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty())
        .collect();
    el(
        "Comments",
        if comments.is_empty() {
            ne(&h.comments)
        } else {
            Some(comments.join("; "))
        },
    );
    let relative = spd.units == SpectralUnits::Relative;
    let peak = v.iter().cloned().fold(0.0f64, f64::max);
    x.push_str("  </Header>\n  <SpectralDistribution>\n");
    x.push_str(&format!(
        "    <SpectralQuantity>{}</SpectralQuantity>\n",
        if relative { "relative" } else { "flux" }
    ));
    if let Some(r) = spectro.and_then(|s| s.resolution).filter(|r| *r > 0.0) {
        x.push_str(&format!(
            "    <BandwidthFWHM>{}</BandwidthFWHM>\n",
            sig(r, 9)
        ));
    }
    for (wl, val) in w.iter().zip(&v) {
        let val = if relative && peak > 0.0 {
            val / peak
        } else {
            *val
        };
        x.push_str(&format!(
            "    <SpectralData wavelength=\"{}\">{}</SpectralData>\n",
            sig(*wl, 9),
            sig(val, 6)
        ));
    }
    x.push_str("  </SpectralDistribution>\n</IESTM2714>\n");
    Some(x)
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
