//! JSON parser for ATLA S001-A documents
//!
//! Parses the JSON format as specified in ATLA S001-A, which provides
//! approximately 90% file size reduction compared to XML.

use crate::error::Result;
use crate::types::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON representation of ATLA document (for serde)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AtlaJson {
    version: String,
    header: HeaderJson,
    #[serde(skip_serializing_if = "Option::is_none")]
    luminaire: Option<LuminaireJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    equipment: Option<EquipmentJson>,
    emitters: Vec<EmitterJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    custom_data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct HeaderJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    manufacturer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    catalog_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    gtin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    more_info_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    laboratory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    report_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    test_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    issue_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    luminaire_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    comments: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct LuminaireJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    dimensions: Option<DimensionsJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mounting: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_emitters: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct DimensionsJson {
    length: f64,
    width: f64,
    height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct EquipmentJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    goniometer: Option<GoniometerJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    integrating_sphere: Option<IntegratingSphereJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    spectroradiometer: Option<SpectroradiometerJson>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GoniometerJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    manufacturer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    goniometer_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    distance: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct IntegratingSphereJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    manufacturer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diameter: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct SpectroradiometerJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    manufacturer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct EmitterJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(default = "default_quantity")]
    quantity: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    rated_lumens: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    measured_lumens: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    input_watts: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    power_factor: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cct: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    color_rendering: Option<ColorRenderingJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sp_ratio: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    intensity_distribution: Option<IntensityDistributionJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    spectral_distribution: Option<SpectralDistributionJson>,
}

fn default_quantity() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ColorRenderingJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    ra: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    r9: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rf: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rg: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct IntensityDistributionJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    photometry_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metric: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    units: Option<String>,
    /// Horizontal (C-plane) angles
    horizontal_angles: Vec<f64>,
    /// Vertical (gamma) angles
    vertical_angles: Vec<f64>,
    /// Flattened intensity values (row-major: h0v0, h0v1, ..., h1v0, h1v1, ...)
    intensities: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct SpectralDistributionJson {
    wavelengths: Vec<f64>,
    values: Vec<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    units: Option<String>,
}

/// Parse ATLA JSON document from string
pub fn parse(json: &str) -> Result<LuminaireOpticalData> {
    let atla_json: AtlaJson = serde_json::from_str(json)?;
    Ok(from_json(atla_json))
}

/// Parse ATLA JSON document from file
pub fn parse_file(path: &std::path::Path) -> Result<LuminaireOpticalData> {
    let content = std::fs::read_to_string(path)?;
    parse(&content)
}

/// Write LuminaireOpticalData to JSON string
pub fn write(doc: &LuminaireOpticalData) -> Result<String> {
    let atla_json = to_json(doc);
    Ok(serde_json::to_string_pretty(&atla_json)?)
}

/// Write LuminaireOpticalData to JSON string (compact, no whitespace)
pub fn write_compact(doc: &LuminaireOpticalData) -> Result<String> {
    let atla_json = to_json(doc);
    Ok(serde_json::to_string(&atla_json)?)
}

/// Write LuminaireOpticalData to file
pub fn write_file(doc: &LuminaireOpticalData, path: &std::path::Path) -> Result<()> {
    let json = write(doc)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Convert JSON representation to domain types
fn from_json(json: AtlaJson) -> LuminaireOpticalData {
    LuminaireOpticalData {
        version: json.version,
        schema_version: SchemaVersion::AtlaS001,
        header: Header {
            manufacturer: json.header.manufacturer,
            catalog_number: json.header.catalog_number,
            description: json.header.description,
            gtin: json.header.gtin,
            gtin_int: None,
            uuid: json.header.uuid,
            unique_identifier: None,
            reference: json.header.reference,
            references: vec![],
            more_info_uri: json.header.more_info_uri,
            laboratory: json.header.laboratory,
            report_number: json.header.report_number,
            report_date: None,
            test_date: json.header.test_date,
            issue_date: json.header.issue_date,
            luminaire_type: json.header.luminaire_type,
            comments: json.header.comments,
            comments_list: vec![],
            document_creator: None,
            document_creation_date: None,
        },
        luminaire: json.luminaire.map(|l| Luminaire {
            dimensions: l.dimensions.map(|d| Dimensions {
                length: d.length,
                width: d.width,
                height: d.height,
            }),
            mounting: l.mounting,
            num_emitters: l.num_emitters,
            luminous_openings: vec![],
        }),
        equipment: json.equipment.map(|e| Equipment {
            goniometer: e.goniometer.map(|g| GoniometerInfo {
                manufacturer: g.manufacturer,
                model: g.model,
                goniometer_type: g.goniometer_type,
                distance: g.distance,
            }),
            integrating_sphere: e.integrating_sphere.map(|s| IntegratingSphereInfo {
                manufacturer: s.manufacturer,
                model: s.model,
                diameter: s.diameter,
            }),
            spectroradiometer: e.spectroradiometer.map(|s| SpectroradiometerInfo {
                manufacturer: s.manufacturer,
                model: s.model,
                wavelength_min: None,
                wavelength_max: None,
                resolution: None,
            }),
            accreditation: None,
        }),
        emitters: json.emitters.into_iter().map(emitter_from_json).collect(),
        custom_data: json.custom_data.map(|v| CustomData {
            namespace: None,
            data: v.to_string(),
        }),
        custom_data_items: vec![],
    }
}

fn emitter_from_json(json: EmitterJson) -> Emitter {
    Emitter {
        id: json.id,
        description: json.description,
        catalog_number: None,
        quantity: json.quantity,
        rated_lumens: json.rated_lumens,
        measured_lumens: json.measured_lumens,
        input_watts: json.input_watts,
        power_factor: json.power_factor,
        ballast_factor: None,
        cct: json.cct,
        color_rendering: json.color_rendering.map(|cr| ColorRendering {
            ra: cr.ra,
            r9: cr.r9,
            rf: cr.rf,
            rg: cr.rg,
        }),
        duv: None,
        sp_ratio: json.sp_ratio,
        data_generation: None,
        intensity_distribution: json.intensity_distribution.map(|id| {
            let h_count = id.horizontal_angles.len();
            let v_count = id.vertical_angles.len();

            // Convert flat array to 2D
            let mut intensities = vec![vec![0.0; v_count]; h_count];
            for (i, &val) in id.intensities.iter().enumerate() {
                let h_idx = i / v_count;
                let v_idx = i % v_count;
                if h_idx < h_count {
                    intensities[h_idx][v_idx] = val;
                }
            }

            IntensityDistribution {
                photometry_type: id
                    .photometry_type
                    .map(|t| match t.as_str() {
                        "A" | "TypeA" => PhotometryType::TypeA,
                        "B" | "TypeB" => PhotometryType::TypeB,
                        _ => PhotometryType::TypeC,
                    })
                    .unwrap_or_default(),
                metric: id
                    .metric
                    .map(|m| match m.as_str() {
                        "Radiant" => IntensityMetric::Radiant,
                        "Photon" => IntensityMetric::Photon,
                        "Spectral" => IntensityMetric::Spectral,
                        _ => IntensityMetric::Luminous,
                    })
                    .unwrap_or_default(),
                units: IntensityUnits::CandelaPerKilolumen,
                horizontal_angles: id.horizontal_angles,
                vertical_angles: id.vertical_angles,
                intensities,
                symmetry: None,
                multiplier: None,
                absolute_photometry: None,
                number_measured: None,
            }
        }),
        spectral_distribution: json.spectral_distribution.map(|sd| SpectralDistribution {
            wavelengths: sd.wavelengths,
            values: sd.values,
            units: SpectralUnits::default(),
            start_wavelength: None,
            wavelength_interval: None,
        }),
        angular_spectral: None,
        angular_color: None,
        tilt_angles: None,
        regulatory: None,
    }
}

/// Convert domain types to JSON representation
fn to_json(doc: &LuminaireOpticalData) -> AtlaJson {
    AtlaJson {
        version: doc.version.clone(),
        header: HeaderJson {
            manufacturer: doc.header.manufacturer.clone(),
            catalog_number: doc.header.catalog_number.clone(),
            description: doc.header.description.clone(),
            gtin: doc.header.gtin.clone(),
            uuid: doc.header.uuid.clone(),
            reference: doc.header.reference.clone(),
            more_info_uri: doc.header.more_info_uri.clone(),
            laboratory: doc.header.laboratory.clone(),
            report_number: doc.header.report_number.clone(),
            test_date: doc.header.test_date.clone(),
            issue_date: doc.header.issue_date.clone(),
            luminaire_type: doc.header.luminaire_type.clone(),
            comments: doc.header.comments.clone(),
        },
        luminaire: doc.luminaire.as_ref().map(|l| LuminaireJson {
            dimensions: l.dimensions.as_ref().map(|d| DimensionsJson {
                length: d.length,
                width: d.width,
                height: d.height,
            }),
            mounting: l.mounting.clone(),
            num_emitters: l.num_emitters,
        }),
        equipment: doc.equipment.as_ref().map(|e| EquipmentJson {
            goniometer: e.goniometer.as_ref().map(|g| GoniometerJson {
                manufacturer: g.manufacturer.clone(),
                model: g.model.clone(),
                goniometer_type: g.goniometer_type.clone(),
                distance: g.distance,
            }),
            integrating_sphere: e
                .integrating_sphere
                .as_ref()
                .map(|s| IntegratingSphereJson {
                    manufacturer: s.manufacturer.clone(),
                    model: s.model.clone(),
                    diameter: s.diameter,
                }),
            spectroradiometer: e.spectroradiometer.as_ref().map(|s| SpectroradiometerJson {
                manufacturer: s.manufacturer.clone(),
                model: s.model.clone(),
            }),
        }),
        emitters: doc.emitters.iter().map(emitter_to_json).collect(),
        custom_data: doc
            .custom_data
            .as_ref()
            .and_then(|c| serde_json::from_str(&c.data).ok()),
    }
}

fn emitter_to_json(emitter: &Emitter) -> EmitterJson {
    EmitterJson {
        id: emitter.id.clone(),
        description: emitter.description.clone(),
        quantity: emitter.quantity,
        rated_lumens: emitter.rated_lumens,
        measured_lumens: emitter.measured_lumens,
        input_watts: emitter.input_watts,
        power_factor: emitter.power_factor,
        cct: emitter.cct,
        color_rendering: emitter
            .color_rendering
            .as_ref()
            .map(|cr| ColorRenderingJson {
                ra: cr.ra,
                r9: cr.r9,
                rf: cr.rf,
                rg: cr.rg,
            }),
        sp_ratio: emitter.sp_ratio,
        intensity_distribution: emitter.intensity_distribution.as_ref().map(|id| {
            // Flatten 2D array to 1D (row-major)
            let intensities: Vec<f64> = id.intensities.iter().flatten().copied().collect();

            IntensityDistributionJson {
                photometry_type: Some(match id.photometry_type {
                    PhotometryType::TypeA => "A".to_string(),
                    PhotometryType::TypeB => "B".to_string(),
                    PhotometryType::TypeC => "C".to_string(),
                }),
                metric: Some(match id.metric {
                    IntensityMetric::Luminous => "Luminous".to_string(),
                    IntensityMetric::Radiant => "Radiant".to_string(),
                    IntensityMetric::Photon => "Photon".to_string(),
                    IntensityMetric::Spectral => "Spectral".to_string(),
                }),
                units: None,
                horizontal_angles: id.horizontal_angles.clone(),
                vertical_angles: id.vertical_angles.clone(),
                intensities,
            }
        }),
        spectral_distribution: emitter.spectral_distribution.as_ref().map(|sd| {
            SpectralDistributionJson {
                wavelengths: sd.wavelengths.clone(),
                values: sd.values.clone(),
                units: None,
            }
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal() {
        let json = r#"{
            "version": "1.0",
            "header": {
                "manufacturer": "Test Corp",
                "catalogNumber": "TC-001"
            },
            "emitters": [{
                "quantity": 1,
                "ratedLumens": 1000
            }]
        }"#;

        let doc = parse(json).unwrap();
        assert_eq!(doc.version, "1.0");
        assert_eq!(doc.header.manufacturer, Some("Test Corp".to_string()));
        assert_eq!(doc.header.catalog_number, Some("TC-001".to_string()));
        assert_eq!(doc.emitters.len(), 1);
        assert_eq!(doc.emitters[0].rated_lumens, Some(1000.0));
    }

    #[test]
    fn test_parse_with_intensity() {
        let json = r#"{
            "version": "1.0",
            "header": {},
            "emitters": [{
                "quantity": 1,
                "intensityDistribution": {
                    "horizontalAngles": [0, 90],
                    "verticalAngles": [0, 45, 90],
                    "intensities": [100, 80, 20, 95, 75, 15]
                }
            }]
        }"#;

        let doc = parse(json).unwrap();
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

        let json = write(&doc).unwrap();
        let parsed = parse(&json).unwrap();

        assert_eq!(parsed.header.manufacturer, doc.header.manufacturer);
        assert_eq!(parsed.header.catalog_number, doc.header.catalog_number);
        assert_eq!(
            parsed.emitters[0].rated_lumens,
            doc.emitters[0].rated_lumens
        );
        assert_eq!(parsed.emitters[0].cct, doc.emitters[0].cct);
    }
}
