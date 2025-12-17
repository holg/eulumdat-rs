//! XML parser for ATLA S001 / TM-33 / UNI 11733 documents
//!
//! Parses the XML format as specified in ATLA S001 and its equivalent standards
//! ANSI/IES TM-33-18 and UNI 11733:2019.

use crate::error::{AtlaError, Result};
use crate::types::*;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;
use std::io::Cursor;

/// Parse ATLA XML document from string
pub fn parse(xml: &str) -> Result<LuminaireOpticalData> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut doc = LuminaireOpticalData::new();
    let mut buf = Vec::new();
    let mut current_path: Vec<String> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                current_path.push(name.clone());

                match name.as_str() {
                    "LuminaireOpticalData" => {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"version" {
                                doc.version = String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                    }
                    "Header" => {
                        doc.header = parse_header(&mut reader)?;
                        current_path.pop();
                    }
                    "Luminaire" => {
                        doc.luminaire = Some(parse_luminaire(&mut reader)?);
                        current_path.pop();
                    }
                    "Equipment" => {
                        doc.equipment = Some(parse_equipment(&mut reader)?);
                        current_path.pop();
                    }
                    "Emitter" => {
                        doc.emitters.push(parse_emitter(&mut reader)?);
                        current_path.pop();
                    }
                    _ => {}
                }
            }
            Ok(Event::End(_)) => {
                current_path.pop();
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(AtlaError::XmlParse(format!(
                    "Error at position {}: {:?}",
                    reader.buffer_position(),
                    e
                )))
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(doc)
}

/// Parse Header section
fn parse_header(reader: &mut Reader<&[u8]>) -> Result<Header> {
    let mut header = Header::default();
    let mut buf = Vec::new();
    let mut current_element = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                current_element = String::from_utf8_lossy(e.name().as_ref()).to_string();
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape().unwrap_or_default().to_string();
                match current_element.as_str() {
                    "Manufacturer" => header.manufacturer = Some(text),
                    "CatalogNumber" => header.catalog_number = Some(text),
                    "Description" => header.description = Some(text),
                    "GTIN" => header.gtin = Some(text),
                    "UUID" => header.uuid = Some(text),
                    "Reference" => header.reference = Some(text),
                    "MoreInfoURI" => header.more_info_uri = Some(text),
                    "Laboratory" => header.laboratory = Some(text),
                    "ReportNumber" => header.report_number = Some(text),
                    "TestDate" => header.test_date = Some(text),
                    "IssueDate" => header.issue_date = Some(text),
                    "LuminaireType" => header.luminaire_type = Some(text),
                    "Comments" => header.comments = Some(text),
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                if e.name().as_ref() == b"Header" {
                    break;
                }
                current_element.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }

    Ok(header)
}

/// Parse Luminaire section
fn parse_luminaire(reader: &mut Reader<&[u8]>) -> Result<Luminaire> {
    let mut luminaire = Luminaire::default();
    let mut buf = Vec::new();
    let mut current_element = String::new();
    let mut in_dimensions = false;
    let mut dims = Dimensions::default();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                current_element = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if current_element == "Dimensions" {
                    in_dimensions = true;
                }
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape().unwrap_or_default().to_string();
                if in_dimensions {
                    match current_element.as_str() {
                        "Length" => dims.length = text.parse().unwrap_or(0.0),
                        "Width" => dims.width = text.parse().unwrap_or(0.0),
                        "Height" => dims.height = text.parse().unwrap_or(0.0),
                        _ => {}
                    }
                } else {
                    match current_element.as_str() {
                        "Mounting" => luminaire.mounting = Some(text),
                        "NumEmitters" => luminaire.num_emitters = text.parse().ok(),
                        _ => {}
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let name_bytes = e.name();
                let name = name_bytes.as_ref();
                if name == b"Dimensions" {
                    in_dimensions = false;
                    luminaire.dimensions = Some(dims.clone());
                }
                if name == b"Luminaire" {
                    break;
                }
                current_element.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }

    Ok(luminaire)
}

/// Parse Equipment section
fn parse_equipment(reader: &mut Reader<&[u8]>) -> Result<Equipment> {
    let mut equipment = Equipment::default();
    let mut buf = Vec::new();
    let mut current_element = String::new();
    let mut in_goniometer = false;
    let mut in_integrating_sphere = false;
    let mut in_spectroradiometer = false;
    let mut in_accreditation = false;

    let mut goniometer = GoniometerInfo::default();
    let mut sphere = IntegratingSphereInfo::default();
    let mut spectro = SpectroradiometerInfo::default();
    let mut accred = Accreditation::default();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                current_element = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match current_element.as_str() {
                    "Goniometer" => in_goniometer = true,
                    "IntegratingSphere" => in_integrating_sphere = true,
                    "Spectroradiometer" => in_spectroradiometer = true,
                    "Accreditation" => in_accreditation = true,
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape().unwrap_or_default().to_string();
                if in_goniometer {
                    match current_element.as_str() {
                        "Manufacturer" => goniometer.manufacturer = Some(text),
                        "Model" => goniometer.model = Some(text),
                        "GoniometerType" | "Type" => goniometer.goniometer_type = Some(text),
                        "Distance" => goniometer.distance = text.parse().ok(),
                        _ => {}
                    }
                } else if in_integrating_sphere {
                    match current_element.as_str() {
                        "Manufacturer" => sphere.manufacturer = Some(text),
                        "Model" => sphere.model = Some(text),
                        "Diameter" => sphere.diameter = text.parse().ok(),
                        _ => {}
                    }
                } else if in_spectroradiometer {
                    match current_element.as_str() {
                        "Manufacturer" => spectro.manufacturer = Some(text),
                        "Model" => spectro.model = Some(text),
                        "WavelengthMin" => spectro.wavelength_min = text.parse().ok(),
                        "WavelengthMax" => spectro.wavelength_max = text.parse().ok(),
                        "Resolution" => spectro.resolution = text.parse().ok(),
                        _ => {}
                    }
                } else if in_accreditation {
                    match current_element.as_str() {
                        "Body" => accred.body = Some(text),
                        "Number" => accred.number = Some(text),
                        "Scope" => accred.scope = Some(text),
                        _ => {}
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let name_bytes = e.name();
                let name = name_bytes.as_ref();
                match name {
                    b"Goniometer" => {
                        in_goniometer = false;
                        equipment.goniometer = Some(goniometer.clone());
                    }
                    b"IntegratingSphere" => {
                        in_integrating_sphere = false;
                        equipment.integrating_sphere = Some(sphere.clone());
                    }
                    b"Spectroradiometer" => {
                        in_spectroradiometer = false;
                        equipment.spectroradiometer = Some(spectro.clone());
                    }
                    b"Accreditation" => {
                        in_accreditation = false;
                        equipment.accreditation = Some(accred.clone());
                    }
                    b"Equipment" => break,
                    _ => {}
                }
                current_element.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }

    Ok(equipment)
}

/// Parse Emitter section
fn parse_emitter(reader: &mut Reader<&[u8]>) -> Result<Emitter> {
    let mut emitter = Emitter {
        quantity: 1,
        ..Default::default()
    };
    let mut buf = Vec::new();
    let mut current_element = String::new();
    let mut in_color_rendering = false;
    let mut color_rendering = ColorRendering::default();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                current_element = name.clone();

                match name.as_str() {
                    "ColorRendering" => {
                        in_color_rendering = true;
                    }
                    "IntensityDistribution" => {
                        emitter.intensity_distribution =
                            Some(parse_intensity_distribution(reader)?);
                    }
                    "SpectralDistribution" => {
                        emitter.spectral_distribution = Some(parse_spectral_distribution(reader)?);
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape().unwrap_or_default().to_string();
                if in_color_rendering {
                    match current_element.as_str() {
                        "Ra" => color_rendering.ra = text.parse().ok(),
                        "R9" => color_rendering.r9 = text.parse().ok(),
                        "Rf" => color_rendering.rf = text.parse().ok(),
                        "Rg" => color_rendering.rg = text.parse().ok(),
                        _ => {}
                    }
                } else {
                    match current_element.as_str() {
                        "ID" | "Id" => emitter.id = Some(text),
                        "Description" => emitter.description = Some(text),
                        "Quantity" => emitter.quantity = text.parse().unwrap_or(1),
                        "RatedLumens" => emitter.rated_lumens = text.parse().ok(),
                        "MeasuredLumens" => emitter.measured_lumens = text.parse().ok(),
                        "InputWatts" => emitter.input_watts = text.parse().ok(),
                        "PowerFactor" => emitter.power_factor = text.parse().ok(),
                        "CCT" => emitter.cct = text.parse().ok(),
                        "SPRatio" => emitter.sp_ratio = text.parse().ok(),
                        _ => {}
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let name_bytes = e.name();
                let name = name_bytes.as_ref();
                if name == b"ColorRendering" {
                    in_color_rendering = false;
                    emitter.color_rendering = Some(color_rendering.clone());
                }
                if name == b"Emitter" {
                    break;
                }
                current_element.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }

    Ok(emitter)
}

/// Parse IntensityDistribution section
fn parse_intensity_distribution(reader: &mut Reader<&[u8]>) -> Result<IntensityDistribution> {
    let mut dist = IntensityDistribution::default();
    let mut buf = Vec::new();
    let mut current_element = String::new();

    // Temporary storage for intensity data points
    let mut intensity_data: Vec<(f64, f64, f64)> = Vec::new(); // (horz, vert, value)

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                current_element = name.clone();

                if name == "IntensityData" {
                    let mut horz = 0.0;
                    let mut vert = 0.0;
                    for attr in e.attributes().flatten() {
                        match attr.key.as_ref() {
                            b"horz" | b"horizontal" => {
                                horz = String::from_utf8_lossy(&attr.value).parse().unwrap_or(0.0);
                            }
                            b"vert" | b"vertical" => {
                                vert = String::from_utf8_lossy(&attr.value).parse().unwrap_or(0.0);
                            }
                            _ => {}
                        }
                    }
                    // Value will be parsed from text content
                    intensity_data.push((horz, vert, 0.0));
                }
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape().unwrap_or_default().to_string();
                match current_element.as_str() {
                    "IntensityData" => {
                        if let Some(last) = intensity_data.last_mut() {
                            last.2 = text.trim().parse().unwrap_or(0.0);
                        }
                    }
                    "PhotometryType" => {
                        dist.photometry_type = match text.trim() {
                            "A" | "TypeA" => PhotometryType::TypeA,
                            "B" | "TypeB" => PhotometryType::TypeB,
                            _ => PhotometryType::TypeC,
                        };
                    }
                    "Metric" => {
                        dist.metric = match text.trim() {
                            "Radiant" => IntensityMetric::Radiant,
                            "Photon" => IntensityMetric::Photon,
                            "Spectral" => IntensityMetric::Spectral,
                            _ => IntensityMetric::Luminous,
                        };
                    }
                    "HorizontalAngles" => {
                        dist.horizontal_angles = parse_angle_list(&text);
                    }
                    "VerticalAngles" => {
                        dist.vertical_angles = parse_angle_list(&text);
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                if e.name().as_ref() == b"IntensityDistribution" {
                    break;
                }
                current_element.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }

    // Convert intensity data points to 2D array
    if !intensity_data.is_empty() {
        // Extract unique angles if not already set
        if dist.horizontal_angles.is_empty() {
            let mut h_angles: Vec<f64> = intensity_data.iter().map(|(h, _, _)| *h).collect();
            h_angles.sort_by(|a, b| a.partial_cmp(b).unwrap());
            h_angles.dedup();
            dist.horizontal_angles = h_angles;
        }
        if dist.vertical_angles.is_empty() {
            let mut v_angles: Vec<f64> = intensity_data.iter().map(|(_, v, _)| *v).collect();
            v_angles.sort_by(|a, b| a.partial_cmp(b).unwrap());
            v_angles.dedup();
            dist.vertical_angles = v_angles;
        }

        // Build 2D intensity array
        let h_count = dist.horizontal_angles.len();
        let v_count = dist.vertical_angles.len();
        dist.intensities = vec![vec![0.0; v_count]; h_count];

        for (horz, vert, value) in intensity_data {
            if let Some(h_idx) = dist
                .horizontal_angles
                .iter()
                .position(|&a| (a - horz).abs() < 0.001)
            {
                if let Some(v_idx) = dist
                    .vertical_angles
                    .iter()
                    .position(|&a| (a - vert).abs() < 0.001)
                {
                    dist.intensities[h_idx][v_idx] = value;
                }
            }
        }
    }

    Ok(dist)
}

/// Parse SpectralDistribution section
fn parse_spectral_distribution(reader: &mut Reader<&[u8]>) -> Result<SpectralDistribution> {
    let mut dist = SpectralDistribution::default();
    let mut buf = Vec::new();
    let mut current_element = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                current_element = String::from_utf8_lossy(e.name().as_ref()).to_string();
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape().unwrap_or_default().to_string();
                match current_element.as_str() {
                    "Wavelengths" => {
                        dist.wavelengths = parse_value_list(&text);
                    }
                    "Values" => {
                        dist.values = parse_value_list(&text);
                    }
                    "StartWavelength" => {
                        dist.start_wavelength = text.parse().ok();
                    }
                    "WavelengthInterval" => {
                        dist.wavelength_interval = text.parse().ok();
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                if e.name().as_ref() == b"SpectralDistribution" {
                    break;
                }
                current_element.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }

    Ok(dist)
}

/// Parse a space or comma-separated list of angles
fn parse_angle_list(text: &str) -> Vec<f64> {
    text.split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect()
}

/// Parse a space or comma-separated list of values
fn parse_value_list(text: &str) -> Vec<f64> {
    parse_angle_list(text)
}

/// Write LuminaireOpticalData to pretty-printed XML string (default)
pub fn write(doc: &LuminaireOpticalData) -> Result<String> {
    write_with_indent(doc, Some(2))
}

/// Write LuminaireOpticalData to compact XML string (no whitespace)
pub fn write_compact(doc: &LuminaireOpticalData) -> Result<String> {
    write_with_indent(doc, None)
}

/// Write LuminaireOpticalData to XML string with optional indentation
fn write_with_indent(doc: &LuminaireOpticalData, indent: Option<usize>) -> Result<String> {
    let cursor = Cursor::new(Vec::new());
    let mut writer = match indent {
        Some(spaces) => Writer::new_with_indent(cursor, b' ', spaces),
        None => Writer::new(cursor),
    };

    // XML declaration
    writer
        .write_event(Event::Decl(quick_xml::events::BytesDecl::new(
            "1.0",
            Some("UTF-8"),
            None,
        )))
        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;

    // Root element
    let mut root = BytesStart::new("LuminaireOpticalData");
    root.push_attribute(("version", doc.version.as_str()));
    root.push_attribute(("xmlns", "http://www.ies.org/tm-33"));
    writer
        .write_event(Event::Start(root))
        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;

    // Header
    write_header(&mut writer, &doc.header)?;

    // Luminaire (optional)
    if let Some(ref luminaire) = doc.luminaire {
        write_luminaire(&mut writer, luminaire)?;
    }

    // Equipment (optional)
    if let Some(ref equipment) = doc.equipment {
        write_equipment(&mut writer, equipment)?;
    }

    // Emitters
    for emitter in &doc.emitters {
        write_emitter(&mut writer, emitter)?;
    }

    // Close root
    writer
        .write_event(Event::End(BytesEnd::new("LuminaireOpticalData")))
        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;

    let result = writer.into_inner().into_inner();
    String::from_utf8(result).map_err(|e| AtlaError::XmlParse(e.to_string()))
}

fn write_element<W: std::io::Write>(writer: &mut Writer<W>, name: &str, value: &str) -> Result<()> {
    writer
        .write_event(Event::Start(BytesStart::new(name)))
        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;
    writer
        .write_event(Event::Text(BytesText::new(value)))
        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;
    writer
        .write_event(Event::End(BytesEnd::new(name)))
        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;
    Ok(())
}

fn write_header<W: std::io::Write>(writer: &mut Writer<W>, header: &Header) -> Result<()> {
    writer
        .write_event(Event::Start(BytesStart::new("Header")))
        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;

    if let Some(ref v) = header.manufacturer {
        write_element(writer, "Manufacturer", v)?;
    }
    if let Some(ref v) = header.catalog_number {
        write_element(writer, "CatalogNumber", v)?;
    }
    if let Some(ref v) = header.description {
        write_element(writer, "Description", v)?;
    }
    if let Some(ref v) = header.gtin {
        write_element(writer, "GTIN", v)?;
    }
    if let Some(ref v) = header.uuid {
        write_element(writer, "UUID", v)?;
    }
    if let Some(ref v) = header.laboratory {
        write_element(writer, "Laboratory", v)?;
    }
    if let Some(ref v) = header.report_number {
        write_element(writer, "ReportNumber", v)?;
    }
    if let Some(ref v) = header.test_date {
        write_element(writer, "TestDate", v)?;
    }
    if let Some(ref v) = header.luminaire_type {
        write_element(writer, "LuminaireType", v)?;
    }
    if let Some(ref v) = header.comments {
        write_element(writer, "Comments", v)?;
    }

    writer
        .write_event(Event::End(BytesEnd::new("Header")))
        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;
    Ok(())
}

fn write_luminaire<W: std::io::Write>(writer: &mut Writer<W>, luminaire: &Luminaire) -> Result<()> {
    writer
        .write_event(Event::Start(BytesStart::new("Luminaire")))
        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;

    if let Some(ref dims) = luminaire.dimensions {
        writer
            .write_event(Event::Start(BytesStart::new("Dimensions")))
            .map_err(|e| AtlaError::XmlParse(e.to_string()))?;
        write_element(writer, "Length", &dims.length.to_string())?;
        write_element(writer, "Width", &dims.width.to_string())?;
        write_element(writer, "Height", &dims.height.to_string())?;
        writer
            .write_event(Event::End(BytesEnd::new("Dimensions")))
            .map_err(|e| AtlaError::XmlParse(e.to_string()))?;
    }

    if let Some(ref mounting) = luminaire.mounting {
        write_element(writer, "Mounting", mounting)?;
    }

    writer
        .write_event(Event::End(BytesEnd::new("Luminaire")))
        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;
    Ok(())
}

fn write_equipment<W: std::io::Write>(
    writer: &mut Writer<W>,
    _equipment: &Equipment,
) -> Result<()> {
    writer
        .write_event(Event::Start(BytesStart::new("Equipment")))
        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;
    // TODO: Implement equipment writing
    writer
        .write_event(Event::End(BytesEnd::new("Equipment")))
        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;
    Ok(())
}

fn write_emitter<W: std::io::Write>(writer: &mut Writer<W>, emitter: &Emitter) -> Result<()> {
    writer
        .write_event(Event::Start(BytesStart::new("Emitter")))
        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;

    if let Some(ref id) = emitter.id {
        write_element(writer, "ID", id)?;
    }
    if let Some(ref desc) = emitter.description {
        write_element(writer, "Description", desc)?;
    }
    write_element(writer, "Quantity", &emitter.quantity.to_string())?;

    if let Some(v) = emitter.rated_lumens {
        write_element(writer, "RatedLumens", &v.to_string())?;
    }
    if let Some(v) = emitter.measured_lumens {
        write_element(writer, "MeasuredLumens", &v.to_string())?;
    }
    if let Some(v) = emitter.input_watts {
        write_element(writer, "InputWatts", &v.to_string())?;
    }
    if let Some(v) = emitter.cct {
        write_element(writer, "CCT", &v.to_string())?;
    }

    // Write intensity distribution
    if let Some(ref dist) = emitter.intensity_distribution {
        write_intensity_distribution(writer, dist)?;
    }

    writer
        .write_event(Event::End(BytesEnd::new("Emitter")))
        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;
    Ok(())
}

fn write_intensity_distribution<W: std::io::Write>(
    writer: &mut Writer<W>,
    dist: &IntensityDistribution,
) -> Result<()> {
    writer
        .write_event(Event::Start(BytesStart::new("IntensityDistribution")))
        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;

    // Write each intensity data point with explicit angles
    for (h_idx, h_angle) in dist.horizontal_angles.iter().enumerate() {
        for (v_idx, v_angle) in dist.vertical_angles.iter().enumerate() {
            if let Some(row) = dist.intensities.get(h_idx) {
                if let Some(&value) = row.get(v_idx) {
                    let mut elem = BytesStart::new("IntensityData");
                    elem.push_attribute(("horz", h_angle.to_string().as_str()));
                    elem.push_attribute(("vert", v_angle.to_string().as_str()));
                    writer
                        .write_event(Event::Start(elem))
                        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;
                    writer
                        .write_event(Event::Text(BytesText::new(&value.to_string())))
                        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;
                    writer
                        .write_event(Event::End(BytesEnd::new("IntensityData")))
                        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;
                }
            }
        }
    }

    writer
        .write_event(Event::End(BytesEnd::new("IntensityDistribution")))
        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;
    Ok(())
}

/// Parse ATLA XML document from file
pub fn parse_file(path: &std::path::Path) -> Result<LuminaireOpticalData> {
    let content = std::fs::read_to_string(path)?;
    parse(&content)
}

/// Write LuminaireOpticalData to file
pub fn write_file(doc: &LuminaireOpticalData, path: &std::path::Path) -> Result<()> {
    let xml = write(doc)?;
    std::fs::write(path, xml)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<LuminaireOpticalData version="1.0">
    <Header>
        <Manufacturer>Test Corp</Manufacturer>
        <CatalogNumber>TC-001</CatalogNumber>
    </Header>
    <Emitter>
        <Quantity>1</Quantity>
        <RatedLumens>1000</RatedLumens>
    </Emitter>
</LuminaireOpticalData>"#;

        let doc = parse(xml).unwrap();
        assert_eq!(doc.version, "1.0");
        assert_eq!(doc.header.manufacturer, Some("Test Corp".to_string()));
        assert_eq!(doc.header.catalog_number, Some("TC-001".to_string()));
        assert_eq!(doc.emitters.len(), 1);
        assert_eq!(doc.emitters[0].rated_lumens, Some(1000.0));
    }

    #[test]
    fn test_parse_intensity_data() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<LuminaireOpticalData version="1.0">
    <Header/>
    <Emitter>
        <Quantity>1</Quantity>
        <IntensityDistribution>
            <IntensityData horz="0" vert="0">100</IntensityData>
            <IntensityData horz="0" vert="45">80</IntensityData>
            <IntensityData horz="0" vert="90">20</IntensityData>
            <IntensityData horz="90" vert="0">95</IntensityData>
            <IntensityData horz="90" vert="45">75</IntensityData>
            <IntensityData horz="90" vert="90">15</IntensityData>
        </IntensityDistribution>
    </Emitter>
</LuminaireOpticalData>"#;

        let doc = parse(xml).unwrap();
        let dist = doc.emitters[0].intensity_distribution.as_ref().unwrap();

        assert_eq!(dist.horizontal_angles, vec![0.0, 90.0]);
        assert_eq!(dist.vertical_angles, vec![0.0, 45.0, 90.0]);
        assert_eq!(dist.sample(0.0, 0.0), Some(100.0));
        assert_eq!(dist.sample(0.0, 45.0), Some(80.0));
        assert_eq!(dist.sample(90.0, 90.0), Some(15.0));
    }

    #[test]
    fn test_roundtrip() {
        let mut doc = LuminaireOpticalData::new();
        doc.header.manufacturer = Some("Roundtrip Test".to_string());
        doc.header.catalog_number = Some("RT-001".to_string());
        doc.emitters.push(Emitter {
            quantity: 1,
            rated_lumens: Some(500.0),
            cct: Some(3000.0),
            ..Default::default()
        });

        let xml = write(&doc).unwrap();
        let parsed = parse(&xml).unwrap();

        assert_eq!(parsed.header.manufacturer, doc.header.manufacturer);
        assert_eq!(parsed.header.catalog_number, doc.header.catalog_number);
        assert_eq!(
            parsed.emitters[0].rated_lumens,
            doc.emitters[0].rated_lumens
        );
        assert_eq!(parsed.emitters[0].cct, doc.emitters[0].cct);
    }

    #[test]
    fn test_parse_equipment() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<LuminaireOpticalData version="1.0">
    <Header/>
    <Equipment>
        <Goniometer>
            <Manufacturer>Test Equipment Co</Manufacturer>
            <Model>GP-5000</Model>
            <GoniometerType>Type C</GoniometerType>
            <Distance>10.0</Distance>
        </Goniometer>
        <IntegratingSphere>
            <Manufacturer>Sphere Inc</Manufacturer>
            <Model>IS-2000</Model>
            <Diameter>2.0</Diameter>
        </IntegratingSphere>
        <Accreditation>
            <Body>NVLAP</Body>
            <Number>200123-0</Number>
        </Accreditation>
    </Equipment>
    <Emitter>
        <Quantity>1</Quantity>
    </Emitter>
</LuminaireOpticalData>"#;

        let doc = parse(xml).unwrap();
        let equipment = doc.equipment.as_ref().unwrap();

        // Goniometer
        let gonio = equipment.goniometer.as_ref().unwrap();
        assert_eq!(gonio.manufacturer, Some("Test Equipment Co".to_string()));
        assert_eq!(gonio.model, Some("GP-5000".to_string()));
        assert_eq!(gonio.goniometer_type, Some("Type C".to_string()));
        assert_eq!(gonio.distance, Some(10.0));

        // Integrating Sphere
        let sphere = equipment.integrating_sphere.as_ref().unwrap();
        assert_eq!(sphere.manufacturer, Some("Sphere Inc".to_string()));
        assert_eq!(sphere.diameter, Some(2.0));

        // Accreditation
        let accred = equipment.accreditation.as_ref().unwrap();
        assert_eq!(accred.body, Some("NVLAP".to_string()));
        assert_eq!(accred.number, Some("200123-0".to_string()));
    }
}
