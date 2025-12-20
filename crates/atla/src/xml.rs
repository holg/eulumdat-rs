//! XML parser for ATLA S001 / TM-33 / UNI 11733 / TM-33-23 documents
//!
//! Parses the XML format as specified in ATLA S001 and its equivalent standards
//! ANSI/IES TM-33-18, TM-33-23, and UNI 11733:2019.
//!
//! # Schema Version Detection
//!
//! The parser automatically detects the schema version based on the root element:
//! - `<LuminaireOpticalData>` → ATLA S001 / TM-33-18
//! - `<IESTM33-22>` → TM-33-23

use crate::error::{AtlaError, Result};
use crate::types::*;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;
use std::io::Cursor;

/// Parse ATLA XML document from string, auto-detecting schema version
pub fn parse(xml: &str) -> Result<LuminaireOpticalData> {
    // Detect schema version from content
    let schema_version = crate::detect_schema_version(xml);

    match schema_version {
        SchemaVersion::Tm3323 | SchemaVersion::Tm3324 => parse_tm33_23(xml),
        SchemaVersion::AtlaS001 => parse_s001(xml),
    }
}

/// Parse ATLA S001 / TM-33-18 format (LuminaireOpticalData root)
fn parse_s001(xml: &str) -> Result<LuminaireOpticalData> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut doc = LuminaireOpticalData::new();
    doc.schema_version = SchemaVersion::AtlaS001;
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
                    "CustomData" => {
                        doc.custom_data = Some(parse_custom_data_s001(&mut reader)?);
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

/// Parse TM-33-23 format (IESTM33-22 root)
fn parse_tm33_23(xml: &str) -> Result<LuminaireOpticalData> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut doc = LuminaireOpticalData::new();
    doc.schema_version = SchemaVersion::Tm3323;
    doc.version = "1.1".to_string(); // TM-33-23 fixed version
    let mut buf = Vec::new();
    let mut current_path: Vec<String> = Vec::new();
    let mut in_root = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                current_path.push(name.clone());

                match name.as_str() {
                    "IESTM33-22" => {
                        in_root = true;
                    }
                    "Version" if in_root && current_path.len() == 2 => {
                        // Version element at root level - will be read as text
                    }
                    "Header" if in_root => {
                        doc.header = parse_header_tm33_23(&mut reader)?;
                        current_path.pop();
                    }
                    "Luminaire" if in_root => {
                        doc.luminaire = Some(parse_luminaire(&mut reader)?);
                        current_path.pop();
                    }
                    "Equipment" if in_root => {
                        doc.equipment = Some(parse_equipment_tm33_23(&mut reader)?);
                        current_path.pop();
                    }
                    "Emitter" if in_root => {
                        doc.emitters.push(parse_emitter_tm33_23(&mut reader)?);
                        current_path.pop();
                    }
                    "CustomData" if in_root => {
                        doc.custom_data_items
                            .push(parse_custom_data_item(&mut reader)?);
                        current_path.pop();
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => {
                if current_path.last().map(|s| s.as_str()) == Some("Version")
                    && current_path.len() == 2
                {
                    let text = e.unescape().unwrap_or_default().to_string();
                    doc.version = text;
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

/// Parse TM-33-23 Header section with required fields
fn parse_header_tm33_23(reader: &mut Reader<&[u8]>) -> Result<Header> {
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
                    "GTIN" => {
                        // TM-33-23 uses xs:integer for GTIN
                        if let Ok(gtin_int) = text.parse::<i64>() {
                            header.gtin_int = Some(gtin_int);
                        }
                        header.gtin = Some(text);
                    }
                    "Laboratory" => header.laboratory = Some(text),
                    "ReportNumber" => header.report_number = Some(text),
                    "ReportDate" => header.report_date = Some(text),
                    "DocumentCreator" => header.document_creator = Some(text),
                    "DocumentCreationDate" => header.document_creation_date = Some(text),
                    "UniqueIdentifier" => header.unique_identifier = Some(text),
                    // Support both "Comment" (TM-33-23 standard) and "Comments" (common usage)
                    "Comment" | "Comments" => header.comments_list.push(text),
                    "Reference" => header.references.push(text),
                    "MoreInfoURI" => header.more_info_uri = Some(text),
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

/// Parse TM-33-23 Equipment section with Gonioradiometer
fn parse_equipment_tm33_23(reader: &mut Reader<&[u8]>) -> Result<Equipment> {
    let mut equipment = Equipment::default();
    let mut buf = Vec::new();
    let mut current_element = String::new();
    let mut in_gonioradiometer = false;
    let mut goniometer = GoniometerInfo::default();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                current_element = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if current_element.as_str() == "Gonioradiometer" {
                    in_gonioradiometer = true
                }
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape().unwrap_or_default().to_string();
                if in_gonioradiometer {
                    match current_element.as_str() {
                        "Type" => {
                            // Parse TM-33-23 format "CIE _ A" etc
                            let gonio_type = GoniometerTypeEnum::parse(&text);
                            goniometer.goniometer_type = Some(gonio_type.to_atla_str().to_string());
                        }
                        "MeasurementEquipment" => {
                            goniometer.model = Some(text);
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let name = e.name();
                if name.as_ref() == b"Gonioradiometer" {
                    in_gonioradiometer = false;
                    equipment.goniometer = Some(goniometer.clone());
                }
                if name.as_ref() == b"Equipment" {
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

    Ok(equipment)
}

/// Parse TM-33-23 Emitter section with extended fields
fn parse_emitter_tm33_23(reader: &mut Reader<&[u8]>) -> Result<Emitter> {
    let mut emitter = Emitter::default();
    let mut buf = Vec::new();
    let mut current_element = String::new();
    let mut in_color_rendering = false;
    let mut in_luminous_intensity = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                current_element = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match current_element.as_str() {
                    "ColorRendering" => in_color_rendering = true,
                    "LuminousData" => {
                        // TM-33-23 uses LuminousData with IntensityData children
                        emitter.intensity_distribution = Some(parse_luminous_data_tm33_23(reader)?);
                    }
                    "LuminousIntensity" => in_luminous_intensity = true,
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
                match current_element.as_str() {
                    "ID" => emitter.id = Some(text.clone()),
                    "Quantity" => emitter.quantity = text.parse().unwrap_or(1),
                    "Description" => emitter.description = Some(text.clone()),
                    "CatalogNumber" => emitter.catalog_number = Some(text.clone()),
                    "RatedLumens" => emitter.rated_lumens = text.parse().ok(),
                    // TM-33-23 uses InputWattage instead of InputWatts
                    "InputWattage" | "InputWatts" => emitter.input_watts = text.parse().ok(),
                    "PowerFactor" => emitter.power_factor = text.parse().ok(),
                    "BallastFactor" => emitter.ballast_factor = text.parse().ok(),
                    "Duv" => emitter.duv = text.parse().ok(),
                    "SPRatio" => emitter.sp_ratio = text.parse().ok(),
                    // Handle CCT - both FixedCCT (TM-33-23) and CCT (S001)
                    "FixedCCT" | "CCT" => emitter.cct = text.parse().ok(),
                    _ => {}
                }

                // Handle color rendering sub-elements
                if in_color_rendering {
                    let cr = emitter
                        .color_rendering
                        .get_or_insert_with(ColorRendering::default);
                    match current_element.as_str() {
                        "Ra" => cr.ra = text.parse().ok(),
                        "R9" => cr.r9 = text.parse().ok(),
                        "Rf" => cr.rf = text.parse().ok(),
                        "Rg" => cr.rg = text.parse().ok(),
                        _ => {}
                    }
                }

                // Handle TM-33-23 specific intensity fields
                if in_luminous_intensity {
                    if let Some(ref mut dist) = emitter.intensity_distribution {
                        match current_element.as_str() {
                            "AbsolutePhotometry" => {
                                dist.absolute_photometry = text.parse().ok();
                            }
                            "Symm" => {
                                dist.symmetry = Some(SymmetryType::from_tm33_str(&text));
                            }
                            "Multiplier" => {
                                dist.multiplier = text.parse().ok();
                            }
                            "NumberMeasured" => {
                                dist.number_measured = text.parse().ok();
                            }
                            _ => {}
                        }
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let name = e.name();
                if name.as_ref() == b"ColorRendering" {
                    in_color_rendering = false;
                }
                if name.as_ref() == b"LuminousIntensity" {
                    in_luminous_intensity = false;
                }
                if name.as_ref() == b"Emitter" {
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

/// Parse TM-33-23 LuminousData section with IntensityData children
fn parse_luminous_data_tm33_23(reader: &mut Reader<&[u8]>) -> Result<IntensityDistribution> {
    let mut dist = IntensityDistribution::default();
    let mut buf = Vec::new();
    let mut current_element = String::new();
    let mut intensity_data: Vec<(f64, f64, f64)> = Vec::new(); // (horz, vert, value)

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                current_element = String::from_utf8_lossy(e.name().as_ref()).to_string();

                if current_element == "IntensityData" {
                    let mut horz = 0.0;
                    let mut vert = 0.0;
                    for attr in e.attributes().flatten() {
                        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                        let val = attr.unescape_value().unwrap_or_default().to_string();
                        match key.as_str() {
                            "horz" => horz = val.parse().unwrap_or(0.0),
                            "vert" => vert = val.parse().unwrap_or(0.0),
                            _ => {}
                        }
                    }
                    intensity_data.push((horz, vert, 0.0));
                }
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape().unwrap_or_default().to_string();
                match current_element.as_str() {
                    "PhotometryType" => {
                        dist.photometry_type = match text.as_str() {
                            "CIE _ A" | "A" | "TypeA" => PhotometryType::TypeA,
                            "CIE _ B" | "B" | "TypeB" => PhotometryType::TypeB,
                            _ => PhotometryType::TypeC,
                        };
                    }
                    "Metric" => {
                        dist.metric = match text.as_str() {
                            "Radiant" => IntensityMetric::Radiant,
                            "Photon" => IntensityMetric::Photon,
                            "Spectral" => IntensityMetric::Spectral,
                            _ => IntensityMetric::Luminous,
                        };
                    }
                    "SymmType" | "Symm" => {
                        dist.symmetry = Some(SymmetryType::from_tm33_str(&text));
                    }
                    "Multiplier" => {
                        dist.multiplier = text.parse().ok();
                    }
                    "AbsolutePhotometry" => {
                        dist.absolute_photometry = text.parse().ok();
                    }
                    "NumberMeasured" => {
                        dist.number_measured = text.parse().ok();
                    }
                    "IntensityData" => {
                        if let Some(last) = intensity_data.last_mut() {
                            last.2 = text.trim().parse().unwrap_or(0.0);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                if e.name().as_ref() == b"LuminousData" {
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

    // Convert intensity data to 2D array
    if !intensity_data.is_empty() {
        // Extract unique angles
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
                .position(|&h| (h - horz).abs() < 0.001)
            {
                if let Some(v_idx) = dist
                    .vertical_angles
                    .iter()
                    .position(|&v| (v - vert).abs() < 0.001)
                {
                    dist.intensities[h_idx][v_idx] = value;
                }
            }
        }
    }

    Ok(dist)
}

/// Parse TM-33-23 CustomData item with Name and UniqueIdentifier
fn parse_custom_data_item(reader: &mut Reader<&[u8]>) -> Result<CustomDataItem> {
    let mut item = CustomDataItem::default();
    let mut buf = Vec::new();
    let mut current_element = String::new();
    let mut raw_content = String::new();
    let mut depth = 1; // Start at 1 because we're inside CustomData

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                current_element = String::from_utf8_lossy(e.name().as_ref()).to_string();
                depth += 1;
                // Capture raw XML for unknown elements
                if depth > 1 && current_element != "Name" && current_element != "UniqueIdentifier" {
                    raw_content.push_str(&format!("<{}>", current_element));
                }
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape().unwrap_or_default().to_string();
                match current_element.as_str() {
                    "Name" => item.name = text,
                    "UniqueIdentifier" => item.unique_identifier = text,
                    _ => {
                        // Preserve unknown content
                        raw_content.push_str(&text);
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                depth -= 1;
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name != "Name" && name != "UniqueIdentifier" && name != "CustomData" {
                    raw_content.push_str(&format!("</{}>", name));
                }
                if e.name().as_ref() == b"CustomData" {
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

    item.raw_content = raw_content.trim().to_string();
    Ok(item)
}

/// Parse S001 CustomData (single, with namespace attribute)
fn parse_custom_data_s001(reader: &mut Reader<&[u8]>) -> Result<CustomData> {
    let mut custom_data = CustomData::default();
    let mut buf = Vec::new();
    let mut raw_content = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                raw_content.push_str(&format!("<{}>", name));
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape().unwrap_or_default().to_string();
                raw_content.push_str(&text);
            }
            Ok(Event::End(ref e)) => {
                if e.name().as_ref() == b"CustomData" {
                    break;
                }
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                raw_content.push_str(&format!("</{}>", name));
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }

    custom_data.data = raw_content.trim().to_string();
    Ok(custom_data)
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
/// Uses the document's schema_version to determine output format
pub fn write(doc: &LuminaireOpticalData) -> Result<String> {
    write_with_schema(doc, doc.schema_version, Some(2))
}

/// Write LuminaireOpticalData to compact XML string (no whitespace)
pub fn write_compact(doc: &LuminaireOpticalData) -> Result<String> {
    write_with_schema(doc, doc.schema_version, None)
}

/// Write LuminaireOpticalData to XML with specified schema version
pub fn write_with_schema(
    doc: &LuminaireOpticalData,
    schema: SchemaVersion,
    indent: Option<usize>,
) -> Result<String> {
    match schema {
        SchemaVersion::Tm3323 | SchemaVersion::Tm3324 => write_tm33_23_with_indent(doc, indent),
        SchemaVersion::AtlaS001 => write_with_indent(doc, indent),
    }
}

/// Write LuminaireOpticalData to TM-33-23 format (IESTM33-22 root)
fn write_tm33_23_with_indent(doc: &LuminaireOpticalData, indent: Option<usize>) -> Result<String> {
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

    // Root element: IESTM33-22
    let root = BytesStart::new("IESTM33-22");
    writer
        .write_event(Event::Start(root))
        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;

    // Version element (fixed at 1.1 for TM-33-23)
    write_element(&mut writer, "Version", "1.1")?;

    // Header (TM-33-23 style)
    write_header_tm33_23(&mut writer, &doc.header)?;

    // Luminaire (optional)
    if let Some(ref luminaire) = doc.luminaire {
        write_luminaire(&mut writer, luminaire)?;
    }

    // Equipment (TM-33-23 style with Gonioradiometer)
    if let Some(ref equipment) = doc.equipment {
        write_equipment_tm33_23(&mut writer, equipment)?;
    }

    // Emitters (TM-33-23 style)
    for emitter in &doc.emitters {
        write_emitter_tm33_23(&mut writer, emitter)?;
    }

    // CustomData items (TM-33-23 allows multiple)
    for item in &doc.custom_data_items {
        write_custom_data_item(&mut writer, item)?;
    }

    // Close root
    writer
        .write_event(Event::End(BytesEnd::new("IESTM33-22")))
        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;

    let result = writer.into_inner().into_inner();
    String::from_utf8(result).map_err(|e| AtlaError::XmlParse(e.to_string()))
}

/// Write TM-33-23 style Header
fn write_header_tm33_23<W: std::io::Write>(writer: &mut Writer<W>, header: &Header) -> Result<()> {
    writer
        .write_event(Event::Start(BytesStart::new("Header")))
        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;

    if let Some(ref v) = header.manufacturer {
        write_element(writer, "Manufacturer", v)?;
    }
    if let Some(ref v) = header.catalog_number {
        write_element(writer, "CatalogNumber", v)?;
    }
    // Description is required in TM-33-23
    if let Some(ref v) = header.description {
        write_element(writer, "Description", v)?;
    }
    // GTIN as integer in TM-33-23
    if let Some(gtin_int) = header.gtin_int {
        write_element(writer, "GTIN", &gtin_int.to_string())?;
    } else if let Some(ref v) = header.gtin {
        write_element(writer, "GTIN", v)?;
    }
    if let Some(ref v) = header.unique_identifier {
        write_element(writer, "UniqueIdentifier", v)?;
    }
    // Laboratory is required in TM-33-23
    if let Some(ref v) = header.laboratory {
        write_element(writer, "Laboratory", v)?;
    }
    // ReportNumber is required in TM-33-23
    if let Some(ref v) = header.report_number {
        write_element(writer, "ReportNumber", v)?;
    }
    // ReportDate is required in TM-33-23 (xs:date format)
    if let Some(ref v) = header.report_date {
        write_element(writer, "ReportDate", v)?;
    }
    if let Some(ref v) = header.document_creator {
        write_element(writer, "DocumentCreator", v)?;
    }
    if let Some(ref v) = header.document_creation_date {
        write_element(writer, "DocumentCreationDate", v)?;
    }
    // Multiple comments (TM-33-23)
    for comment in &header.comments_list {
        write_element(writer, "Comment", comment)?;
    }
    // Multiple references (TM-33-23)
    for reference in &header.references {
        write_element(writer, "Reference", reference)?;
    }
    if let Some(ref v) = header.more_info_uri {
        write_element(writer, "MoreInfoURI", v)?;
    }

    writer
        .write_event(Event::End(BytesEnd::new("Header")))
        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;
    Ok(())
}

/// Write TM-33-23 style Equipment with Gonioradiometer
fn write_equipment_tm33_23<W: std::io::Write>(
    writer: &mut Writer<W>,
    equipment: &Equipment,
) -> Result<()> {
    writer
        .write_event(Event::Start(BytesStart::new("Equipment")))
        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;

    // TM-33-23 uses Gonioradiometer instead of Goniometer
    if let Some(ref goniometer) = equipment.goniometer {
        writer
            .write_event(Event::Start(BytesStart::new("Gonioradiometer")))
            .map_err(|e| AtlaError::XmlParse(e.to_string()))?;

        // Convert type to TM-33-23 format "CIE _ A"
        if let Some(ref t) = goniometer.goniometer_type {
            let gonio_type = GoniometerTypeEnum::parse(t);
            write_element(writer, "Type", gonio_type.to_tm33_str())?;
        }
        if let Some(ref v) = goniometer.model {
            write_element(writer, "MeasurementEquipment", v)?;
        }

        writer
            .write_event(Event::End(BytesEnd::new("Gonioradiometer")))
            .map_err(|e| AtlaError::XmlParse(e.to_string()))?;
    }

    writer
        .write_event(Event::End(BytesEnd::new("Equipment")))
        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;
    Ok(())
}

/// Write TM-33-23 style Emitter with extended fields
fn write_emitter_tm33_23<W: std::io::Write>(
    writer: &mut Writer<W>,
    emitter: &Emitter,
) -> Result<()> {
    writer
        .write_event(Event::Start(BytesStart::new("Emitter")))
        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;

    // Quantity is required in TM-33-23
    write_element(writer, "Quantity", &emitter.quantity.to_string())?;

    // Description is required in TM-33-23
    if let Some(ref v) = emitter.description {
        write_element(writer, "Description", v)?;
    }
    if let Some(ref v) = emitter.catalog_number {
        write_element(writer, "CatalogNumber", v)?;
    }
    if let Some(v) = emitter.rated_lumens {
        write_element(writer, "RatedLumens", &v.to_string())?;
    }
    // TM-33-23 uses InputWattage (required)
    if let Some(v) = emitter.input_watts {
        write_element(writer, "InputWattage", &v.to_string())?;
    }
    if let Some(v) = emitter.power_factor {
        write_element(writer, "PowerFactor", &v.to_string())?;
    }
    if let Some(v) = emitter.ballast_factor {
        write_element(writer, "BallastFactor", &v.to_string())?;
    }

    // Color Temperature section
    if emitter.cct.is_some() {
        writer
            .write_event(Event::Start(BytesStart::new("ColorTemperature")))
            .map_err(|e| AtlaError::XmlParse(e.to_string()))?;
        if let Some(v) = emitter.cct {
            write_element(writer, "FixedCCT", &v.to_string())?;
        }
        writer
            .write_event(Event::End(BytesEnd::new("ColorTemperature")))
            .map_err(|e| AtlaError::XmlParse(e.to_string()))?;
    }

    // Color Rendering section
    if let Some(ref cr) = emitter.color_rendering {
        writer
            .write_event(Event::Start(BytesStart::new("ColorRendering")))
            .map_err(|e| AtlaError::XmlParse(e.to_string()))?;
        if let Some(v) = cr.ra {
            write_element(writer, "Ra", &v.to_string())?;
        }
        if let Some(v) = cr.r9 {
            write_element(writer, "R9", &v.to_string())?;
        }
        if let Some(v) = cr.rf {
            write_element(writer, "Rf", &v.to_string())?;
        }
        if let Some(v) = cr.rg {
            write_element(writer, "Rg", &v.to_string())?;
        }
        writer
            .write_event(Event::End(BytesEnd::new("ColorRendering")))
            .map_err(|e| AtlaError::XmlParse(e.to_string()))?;
    }

    if let Some(v) = emitter.duv {
        write_element(writer, "Duv", &v.to_string())?;
    }
    if let Some(v) = emitter.sp_ratio {
        write_element(writer, "SPRatio", &v.to_string())?;
    }

    // LuminousData section with TM-33-23 specific fields
    if let Some(ref dist) = emitter.intensity_distribution {
        writer
            .write_event(Event::Start(BytesStart::new("LuminousData")))
            .map_err(|e| AtlaError::XmlParse(e.to_string()))?;

        writer
            .write_event(Event::Start(BytesStart::new("LuminousIntensity")))
            .map_err(|e| AtlaError::XmlParse(e.to_string()))?;

        // TM-33-23 specific fields
        if let Some(v) = dist.absolute_photometry {
            write_element(
                writer,
                "AbsolutePhotometry",
                if v { "true" } else { "false" },
            )?;
        }
        if let Some(ref symm) = dist.symmetry {
            write_element(writer, "Symm", symm.to_tm33_str())?;
        }
        if let Some(v) = dist.multiplier {
            write_element(writer, "Multiplier", &v.to_string())?;
        }
        if let Some(v) = dist.number_measured {
            write_element(writer, "NumberMeasured", &v.to_string())?;
        }

        // Write intensity data (reuse existing function)
        write_intensity_data(writer, dist)?;

        writer
            .write_event(Event::End(BytesEnd::new("LuminousIntensity")))
            .map_err(|e| AtlaError::XmlParse(e.to_string()))?;
        writer
            .write_event(Event::End(BytesEnd::new("LuminousData")))
            .map_err(|e| AtlaError::XmlParse(e.to_string()))?;
    }

    // Spectral Distribution
    if let Some(ref spectral) = emitter.spectral_distribution {
        write_spectral_distribution(writer, spectral)?;
    }

    writer
        .write_event(Event::End(BytesEnd::new("Emitter")))
        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;
    Ok(())
}

/// Write TM-33-23 style CustomData item
fn write_custom_data_item<W: std::io::Write>(
    writer: &mut Writer<W>,
    item: &CustomDataItem,
) -> Result<()> {
    writer
        .write_event(Event::Start(BytesStart::new("CustomData")))
        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;

    write_element(writer, "Name", &item.name)?;
    write_element(writer, "UniqueIdentifier", &item.unique_identifier)?;

    // Write raw content (preserved from parsing)
    if !item.raw_content.is_empty() {
        writer
            .write_event(Event::Text(BytesText::new(&item.raw_content)))
            .map_err(|e| AtlaError::XmlParse(e.to_string()))?;
    }

    writer
        .write_event(Event::End(BytesEnd::new("CustomData")))
        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;
    Ok(())
}

/// Write intensity data (horizontal/vertical angles and values)
fn write_intensity_data<W: std::io::Write>(
    writer: &mut Writer<W>,
    dist: &IntensityDistribution,
) -> Result<()> {
    write_element(
        writer,
        "NumberHorz",
        &dist.horizontal_angles.len().to_string(),
    )?;
    write_element(
        writer,
        "NumberVert",
        &dist.vertical_angles.len().to_string(),
    )?;

    // Write angle lists and intensity data points
    for (h_idx, h_angle) in dist.horizontal_angles.iter().enumerate() {
        for (v_idx, v_angle) in dist.vertical_angles.iter().enumerate() {
            if let Some(row) = dist.intensities.get(h_idx) {
                if let Some(value) = row.get(v_idx) {
                    let mut elem = BytesStart::new("IntData");
                    elem.push_attribute(("h", h_angle.to_string().as_str()));
                    elem.push_attribute(("v", v_angle.to_string().as_str()));
                    writer
                        .write_event(Event::Start(elem))
                        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;
                    writer
                        .write_event(Event::Text(BytesText::new(&value.to_string())))
                        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;
                    writer
                        .write_event(Event::End(BytesEnd::new("IntData")))
                        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;
                }
            }
        }
    }

    Ok(())
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

/// Write SpectralDistribution section
fn write_spectral_distribution<W: std::io::Write>(
    writer: &mut Writer<W>,
    dist: &SpectralDistribution,
) -> Result<()> {
    writer
        .write_event(Event::Start(BytesStart::new("SpectralDistribution")))
        .map_err(|e| AtlaError::XmlParse(e.to_string()))?;

    // Write units
    let units_str = match dist.units {
        SpectralUnits::WattsPerNanometer => "WattsPerNanometer",
        SpectralUnits::Relative => "Relative",
    };
    write_element(writer, "Units", units_str)?;

    // Write start wavelength and interval if present
    if let Some(start) = dist.start_wavelength {
        write_element(writer, "StartWavelength", &start.to_string())?;
    }
    if let Some(interval) = dist.wavelength_interval {
        write_element(writer, "WavelengthInterval", &interval.to_string())?;
    }

    // Write wavelengths as space-separated list
    if !dist.wavelengths.is_empty() {
        let wavelengths_str: Vec<String> = dist.wavelengths.iter().map(|v| v.to_string()).collect();
        write_element(writer, "Wavelengths", &wavelengths_str.join(" "))?;
    }

    // Write values as space-separated list
    if !dist.values.is_empty() {
        let values_str: Vec<String> = dist.values.iter().map(|v| v.to_string()).collect();
        write_element(writer, "Values", &values_str.join(" "))?;
    }

    writer
        .write_event(Event::End(BytesEnd::new("SpectralDistribution")))
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
