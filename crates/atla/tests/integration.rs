//! Integration tests for the atla crate

use std::path::PathBuf;

fn samples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/samples")
}

fn templates_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("eulumdat-wasm/templates")
}

#[test]
fn test_parse_xml_sample() {
    let path = samples_dir().join("fluorescent.xml");
    let doc = atla::parse_file(&path).expect("Failed to parse XML file");

    assert_eq!(doc.version, "1.0");
    assert_eq!(
        doc.header.manufacturer,
        Some("ATLA Test Manufacturer".to_string())
    );
    assert_eq!(doc.header.catalog_number, Some("FL-T16-54W".to_string()));
    assert_eq!(doc.emitters.len(), 1);

    let emitter = &doc.emitters[0];
    assert_eq!(emitter.quantity, 2);
    assert_eq!(emitter.rated_lumens, Some(8100.0));
    assert_eq!(emitter.cct, Some(6500.0));

    // Check intensity distribution
    let dist = emitter.intensity_distribution.as_ref().unwrap();
    assert_eq!(dist.horizontal_angles.len(), 2);
    assert_eq!(dist.vertical_angles.len(), 19);
    assert_eq!(dist.sample(0.0, 0.0), Some(136.0));
    assert_eq!(dist.sample(90.0, 90.0), Some(1.0));
}

#[cfg(feature = "json")]
#[test]
fn test_parse_json_sample() {
    let path = samples_dir().join("fluorescent.json");
    let doc = atla::parse_file(&path).expect("Failed to parse JSON file");

    assert_eq!(doc.version, "1.0");
    assert_eq!(
        doc.header.manufacturer,
        Some("ATLA Test Manufacturer".to_string())
    );
    assert_eq!(doc.emitters.len(), 1);

    let emitter = &doc.emitters[0];
    assert_eq!(emitter.quantity, 2);
    assert_eq!(emitter.rated_lumens, Some(8100.0));

    // Check intensity distribution
    let dist = emitter.intensity_distribution.as_ref().unwrap();
    assert_eq!(dist.sample(0.0, 0.0), Some(136.0));
}

#[cfg(feature = "json")]
#[test]
fn test_xml_json_equivalence() {
    let xml_path = samples_dir().join("fluorescent.xml");
    let json_path = samples_dir().join("fluorescent.json");

    let xml_doc = atla::parse_file(&xml_path).unwrap();
    let json_doc = atla::parse_file(&json_path).unwrap();

    // Both should have same core data
    assert_eq!(xml_doc.header.manufacturer, json_doc.header.manufacturer);
    assert_eq!(xml_doc.header.catalog_number, json_doc.header.catalog_number);
    assert_eq!(xml_doc.emitters.len(), json_doc.emitters.len());
    assert_eq!(
        xml_doc.emitters[0].rated_lumens,
        json_doc.emitters[0].rated_lumens
    );
    assert_eq!(xml_doc.emitters[0].cct, json_doc.emitters[0].cct);

    // Intensity at same point should match
    let xml_dist = xml_doc.emitters[0].intensity_distribution.as_ref().unwrap();
    let json_dist = json_doc.emitters[0].intensity_distribution.as_ref().unwrap();
    assert_eq!(xml_dist.sample(0.0, 0.0), json_dist.sample(0.0, 0.0));
}

#[test]
fn test_xml_roundtrip() {
    let path = samples_dir().join("fluorescent.xml");
    let original = atla::parse_file(&path).unwrap();

    // Write to XML
    let xml_output = atla::xml::write(&original).unwrap();

    // Parse back
    let reparsed = atla::xml::parse(&xml_output).unwrap();

    // Core data should match
    assert_eq!(original.version, reparsed.version);
    assert_eq!(original.header.manufacturer, reparsed.header.manufacturer);
    assert_eq!(original.emitters.len(), reparsed.emitters.len());
    assert_eq!(
        original.emitters[0].rated_lumens,
        reparsed.emitters[0].rated_lumens
    );
}

#[cfg(feature = "json")]
#[test]
fn test_json_roundtrip() {
    let path = samples_dir().join("fluorescent.json");
    let original = atla::parse_file(&path).unwrap();

    // Write to JSON
    let json_output = atla::json::write(&original).unwrap();

    // Parse back
    let reparsed = atla::json::parse(&json_output).unwrap();

    // Core data should match
    assert_eq!(original.version, reparsed.version);
    assert_eq!(original.header.manufacturer, reparsed.header.manufacturer);
    assert_eq!(original.emitters.len(), reparsed.emitters.len());
}

#[cfg(feature = "json")]
#[test]
fn test_xml_to_json_conversion() {
    let xml_path = samples_dir().join("fluorescent.xml");
    let xml_doc = atla::parse_file(&xml_path).unwrap();

    // Convert to JSON
    let json_output = atla::json::write(&xml_doc).unwrap();

    // Verify JSON is smaller than XML
    let xml_content = std::fs::read_to_string(&xml_path).unwrap();
    println!("XML size: {} bytes", xml_content.len());
    println!("JSON size: {} bytes", json_output.len());

    // JSON should be smaller (compact would be even smaller)
    let compact_json = atla::json::write_compact(&xml_doc).unwrap();
    println!("Compact JSON size: {} bytes", compact_json.len());
    assert!(compact_json.len() < xml_content.len());
}

#[cfg(feature = "eulumdat")]
#[test]
fn test_ldt_to_atla_conversion() {
    let ldt_path = templates_dir().join("fluorescent_luminaire.ldt");
    if !ldt_path.exists() {
        eprintln!("Skipping test: LDT file not found at {:?}", ldt_path);
        return;
    }

    let ldt = eulumdat::Eulumdat::from_file(&ldt_path).expect("Failed to parse LDT file");

    // Convert to ATLA
    let atla_doc = atla::LuminaireOpticalData::from_eulumdat(&ldt);

    // Verify conversion
    assert!(!atla_doc.emitters.is_empty());

    let emitter = &atla_doc.emitters[0];

    // LDT has 2 lamps with 8100 lumens total
    assert_eq!(emitter.rated_lumens, Some(8100.0));

    // Check intensity distribution was preserved
    let dist = emitter.intensity_distribution.as_ref().unwrap();
    assert!(!dist.horizontal_angles.is_empty());
    assert!(!dist.vertical_angles.is_empty());
    assert!(!dist.intensities.is_empty());

    // First intensity value should be 136 cd/klm
    assert_eq!(dist.intensities[0][0], 136.0);
}

#[cfg(feature = "eulumdat")]
#[test]
fn test_atla_to_ldt_conversion() {
    let xml_path = samples_dir().join("fluorescent.xml");
    let atla_doc = atla::parse_file(&xml_path).unwrap();

    // Convert to LDT
    let ldt = atla_doc.to_eulumdat();

    // Verify conversion
    assert!(!ldt.lamp_sets.is_empty());
    assert_eq!(ldt.lamp_sets[0].total_luminous_flux, 8100.0);
    assert!(!ldt.c_angles.is_empty());
    assert!(!ldt.g_angles.is_empty());
    assert!(!ldt.intensities.is_empty());

    // Dimensions should be preserved
    assert_eq!(ldt.length, 1170.0);
    assert_eq!(ldt.width, 90.0);
}

#[cfg(feature = "eulumdat")]
#[test]
fn test_ldt_roundtrip_via_atla() {
    let ldt_path = templates_dir().join("fluorescent_luminaire.ldt");
    if !ldt_path.exists() {
        eprintln!("Skipping test: LDT file not found at {:?}", ldt_path);
        return;
    }

    let original_ldt = eulumdat::Eulumdat::from_file(&ldt_path).unwrap();

    // LDT -> ATLA -> LDT
    let atla_doc = atla::LuminaireOpticalData::from_eulumdat(&original_ldt);
    let converted_ldt = atla_doc.to_eulumdat();

    // Core photometric data should be preserved
    assert_eq!(original_ldt.c_angles.len(), converted_ldt.c_angles.len());
    assert_eq!(original_ldt.g_angles.len(), converted_ldt.g_angles.len());
    assert_eq!(
        original_ldt.intensities.len(),
        converted_ldt.intensities.len()
    );

    // Intensity values should match
    for (i, (orig_plane, conv_plane)) in original_ldt
        .intensities
        .iter()
        .zip(converted_ldt.intensities.iter())
        .enumerate()
    {
        for (j, (orig_val, conv_val)) in orig_plane.iter().zip(conv_plane.iter()).enumerate() {
            assert!(
                (orig_val - conv_val).abs() < 0.001,
                "Intensity mismatch at C[{}] G[{}]: {} vs {}",
                i,
                j,
                orig_val,
                conv_val
            );
        }
    }
}

#[test]
fn test_luminaire_calculations() {
    let path = samples_dir().join("fluorescent.xml");
    let doc = atla::parse_file(&path).unwrap();

    // Total luminous flux
    assert_eq!(doc.total_luminous_flux(), 8100.0);

    // Total input power
    assert_eq!(doc.total_input_watts(), 120.0);

    // Efficacy
    let efficacy = doc.efficacy().unwrap();
    assert!((efficacy - 67.5).abs() < 0.1); // 8100 / 120 = 67.5 lm/W
}

#[test]
fn test_intensity_sampling() {
    let path = samples_dir().join("fluorescent.xml");
    let doc = atla::parse_file(&path).unwrap();

    let dist = doc.emitters[0].intensity_distribution.as_ref().unwrap();

    // Sample at known points
    assert_eq!(dist.sample(0.0, 0.0), Some(136.0));
    assert_eq!(dist.sample(0.0, 45.0), Some(90.0));
    assert_eq!(dist.sample(0.0, 90.0), Some(10.0));
    assert_eq!(dist.sample(90.0, 0.0), Some(136.0));
    assert_eq!(dist.sample(90.0, 90.0), Some(1.0));

    // Max intensity
    assert_eq!(dist.max_intensity(), 136.0);
}

#[test]
fn test_ir_spectral_templates() {
    use atla::SpectralMetrics;

    // Test halogen lamp with IR
    let halogen_path = samples_dir().join("halogen_lamp.xml");
    let halogen = atla::parse_file(&halogen_path).expect("Failed to parse halogen lamp");

    let spd = halogen.emitters[0].spectral_distribution.as_ref()
        .expect("Halogen should have spectral data");
    let metrics = SpectralMetrics::from_spd(spd);

    assert!(metrics.has_ir, "Halogen should have IR data");
    assert!(metrics.wavelength_max >= 1000.0, "Halogen should extend to 1000nm");
    assert!(metrics.nir_percent > 5.0, "Halogen should have significant IR content");
    println!("Halogen: {:.1}% visible, {:.1}% NIR", metrics.visible_percent, metrics.nir_percent);

    // Test heat lamp with high IR
    let heat_path = samples_dir().join("heat_lamp.xml");
    let heat = atla::parse_file(&heat_path).expect("Failed to parse heat lamp");

    let spd = heat.emitters[0].spectral_distribution.as_ref()
        .expect("Heat lamp should have spectral data");
    let metrics = SpectralMetrics::from_spd(spd);

    assert!(metrics.has_ir, "Heat lamp should have IR data");
    assert!(metrics.thermal_warning, "Heat lamp should trigger thermal warning");
    assert!(metrics.nir_percent > 25.0, "Heat lamp should have >25% IR");
    println!("Heat lamp: {:.1}% visible, {:.1}% NIR (thermal warning: {})",
        metrics.visible_percent, metrics.nir_percent, metrics.thermal_warning);
}

#[test]
fn test_uv_spectral_template() {
    use atla::SpectralMetrics;

    let uv_path = samples_dir().join("uv_blacklight.xml");
    let uv = atla::parse_file(&uv_path).expect("Failed to parse UV blacklight");

    let spd = uv.emitters[0].spectral_distribution.as_ref()
        .expect("UV lamp should have spectral data");
    let metrics = SpectralMetrics::from_spd(spd);

    assert!(metrics.has_uv, "UV lamp should have UV data");
    assert!(metrics.wavelength_min <= 320.0, "UV lamp should have data below 320nm");
    assert!(metrics.uv_a_percent > 5.0, "UV lamp should have significant UV-A content");
    assert!(metrics.uv_warning, "UV lamp should trigger UV warning");
    println!("UV blacklight: {:.1}% UV-A, {:.1}% visible (UV warning: {})",
        metrics.uv_a_percent, metrics.visible_percent, metrics.uv_warning);
}
