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
    assert_eq!(
        xml_doc.header.catalog_number,
        json_doc.header.catalog_number
    );
    assert_eq!(xml_doc.emitters.len(), json_doc.emitters.len());
    assert_eq!(
        xml_doc.emitters[0].rated_lumens,
        json_doc.emitters[0].rated_lumens
    );
    assert_eq!(xml_doc.emitters[0].cct, json_doc.emitters[0].cct);

    // Intensity at same point should match
    let xml_dist = xml_doc.emitters[0].intensity_distribution.as_ref().unwrap();
    let json_dist = json_doc.emitters[0]
        .intensity_distribution
        .as_ref()
        .unwrap();
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

    let spd = halogen.emitters[0]
        .spectral_distribution
        .as_ref()
        .expect("Halogen should have spectral data");
    let metrics = SpectralMetrics::from_spd(spd);

    assert!(metrics.has_ir, "Halogen should have IR data");
    assert!(
        metrics.wavelength_max >= 1000.0,
        "Halogen should extend to 1000nm"
    );
    assert!(
        metrics.nir_percent > 5.0,
        "Halogen should have significant IR content"
    );
    println!(
        "Halogen: {:.1}% visible, {:.1}% NIR",
        metrics.visible_percent, metrics.nir_percent
    );

    // Test heat lamp with high IR
    let heat_path = samples_dir().join("heat_lamp.xml");
    let heat = atla::parse_file(&heat_path).expect("Failed to parse heat lamp");

    let spd = heat.emitters[0]
        .spectral_distribution
        .as_ref()
        .expect("Heat lamp should have spectral data");
    let metrics = SpectralMetrics::from_spd(spd);

    assert!(metrics.has_ir, "Heat lamp should have IR data");
    assert!(
        metrics.thermal_warning,
        "Heat lamp should trigger thermal warning"
    );
    assert!(metrics.nir_percent > 25.0, "Heat lamp should have >25% IR");
    println!(
        "Heat lamp: {:.1}% visible, {:.1}% NIR (thermal warning: {})",
        metrics.visible_percent, metrics.nir_percent, metrics.thermal_warning
    );
}

#[test]
fn test_uv_spectral_template() {
    use atla::SpectralMetrics;

    let uv_path = samples_dir().join("uv_blacklight.xml");
    let uv = atla::parse_file(&uv_path).expect("Failed to parse UV blacklight");

    let spd = uv.emitters[0]
        .spectral_distribution
        .as_ref()
        .expect("UV lamp should have spectral data");
    let metrics = SpectralMetrics::from_spd(spd);

    assert!(metrics.has_uv, "UV lamp should have UV data");
    assert!(
        metrics.wavelength_min <= 320.0,
        "UV lamp should have data below 320nm"
    );
    assert!(
        metrics.uv_a_percent > 5.0,
        "UV lamp should have significant UV-A content"
    );
    assert!(metrics.uv_warning, "UV lamp should trigger UV warning");
    println!(
        "UV blacklight: {:.1}% UV-A, {:.1}% visible (UV warning: {})",
        metrics.uv_a_percent, metrics.visible_percent, metrics.uv_warning
    );
}

// ===========================================
// TM-33-23 Schema Tests
// ===========================================

fn tm33_samples_dir() -> PathBuf {
    samples_dir().join("tm33-23")
}

#[test]
fn test_schema_detection() {
    use atla::SchemaVersion;

    // ATLA S001 format
    let s001_xml = r#"<LuminaireOpticalData version="1.0"></LuminaireOpticalData>"#;
    assert_eq!(
        atla::detect_schema_version(s001_xml),
        SchemaVersion::AtlaS001
    );

    // TM-33-23 format
    let tm33_xml = r#"<IESTM33-22><Version>1.1</Version></IESTM33-22>"#;
    assert_eq!(atla::detect_schema_version(tm33_xml), SchemaVersion::Tm3323);
}

#[test]
fn test_parse_tm33_23_minimal() {
    let path = tm33_samples_dir().join("minimal.xml");
    if !path.exists() {
        eprintln!(
            "Skipping test: TM-33-23 minimal sample not found at {:?}",
            path
        );
        return;
    }

    let doc = atla::parse_file(&path).expect("Failed to parse TM-33-23 minimal file");

    // Check schema version was detected
    assert_eq!(doc.schema_version, atla::SchemaVersion::Tm3323);

    // Check version string
    assert_eq!(doc.version, "1.1");

    // Check header fields
    assert_eq!(
        doc.header.manufacturer,
        Some("ATLA Test Manufacturer".to_string())
    );
    assert_eq!(
        doc.header.description,
        Some("Minimal TM-33-23 Test Luminaire".to_string())
    );
    assert_eq!(doc.header.laboratory, Some("Test Laboratory".to_string()));
    assert_eq!(doc.header.report_number, Some("RPT-2024-001".to_string()));
    assert_eq!(doc.header.report_date, Some("2024-01-15".to_string()));
    assert!(doc.header.unique_identifier.is_some());

    // Check emitter
    assert_eq!(doc.emitters.len(), 1);
    let emitter = &doc.emitters[0];
    assert_eq!(emitter.description, Some("LED Panel Module".to_string()));
    assert_eq!(emitter.quantity, 1);
    assert_eq!(emitter.rated_lumens, Some(4000.0));
    assert_eq!(emitter.input_watts, Some(40.0));
    assert_eq!(emitter.cct, Some(4000.0));

    // Check intensity distribution
    let dist = emitter.intensity_distribution.as_ref().unwrap();
    assert_eq!(dist.symmetry, Some(atla::SymmetryType::Full));
    assert_eq!(dist.multiplier, Some(1.0));
    assert!(!dist.horizontal_angles.is_empty());
    assert!(!dist.vertical_angles.is_empty());
}

#[test]
fn test_parse_tm33_23_with_custom_data() {
    let path = tm33_samples_dir().join("with_custom_data.xml");
    if !path.exists() {
        eprintln!(
            "Skipping test: TM-33-23 custom data sample not found at {:?}",
            path
        );
        return;
    }

    let doc = atla::parse_file(&path).expect("Failed to parse TM-33-23 with custom data");

    // Check schema version
    assert_eq!(doc.schema_version, atla::SchemaVersion::Tm3323);

    // Check header with multiple comments
    assert!(
        !doc.header.comments_list.is_empty(),
        "Should have multiple comments"
    );
    assert!(doc.header.document_creator.is_some());

    // Check emitter has TM-33-23 specific fields
    let emitter = &doc.emitters[0];
    assert!(emitter.ballast_factor.is_some());
    assert!(emitter.duv.is_some());
    assert!(emitter.catalog_number.is_some());

    // Check CustomData items
    assert!(
        !doc.custom_data_items.is_empty(),
        "Should have CustomData items"
    );
    for item in &doc.custom_data_items {
        assert!(!item.name.is_empty(), "CustomData.Name should not be empty");
        assert!(
            !item.unique_identifier.is_empty(),
            "CustomData.UniqueIdentifier should not be empty"
        );
    }
}

#[test]
fn test_tm33_23_validation() {
    use atla::validate::{validate_with_schema, ValidationSchema};

    let path = tm33_samples_dir().join("minimal.xml");
    if !path.exists() {
        eprintln!("Skipping test: TM-33-23 minimal sample not found");
        return;
    }

    let doc = atla::parse_file(&path).unwrap();

    // Should pass TM-33-23 validation
    let result = validate_with_schema(&doc, ValidationSchema::Tm3323);
    assert!(
        result.is_valid(),
        "TM-33-23 minimal should be valid: {:?}",
        result.errors
    );

    // Should also pass auto-detection (since schema_version is Tm3323)
    let result_auto = validate_with_schema(&doc, ValidationSchema::Auto);
    assert!(result_auto.is_valid());
}

#[test]
fn test_tm33_23_to_s001_conversion() {
    let path = tm33_samples_dir().join("minimal.xml");
    if !path.exists() {
        eprintln!("Skipping test: TM-33-23 minimal sample not found");
        return;
    }

    let doc = atla::parse_file(&path).unwrap();
    assert_eq!(doc.schema_version, atla::SchemaVersion::Tm3323);

    // Convert to S001
    #[cfg(feature = "eulumdat")]
    {
        use atla::convert::tm33_to_atla;
        let (s001_doc, log) = tm33_to_atla(&doc);

        // Should have S001 schema version
        assert_eq!(s001_doc.schema_version, atla::SchemaVersion::AtlaS001);

        // Core data should be preserved
        assert_eq!(s001_doc.header.manufacturer, doc.header.manufacturer);
        assert_eq!(s001_doc.emitters.len(), doc.emitters.len());
        assert_eq!(
            s001_doc.emitters[0].rated_lumens,
            doc.emitters[0].rated_lumens
        );

        println!("Conversion log entries: {}", log.len());
        for entry in &log {
            println!("  {}: {:?}", entry.field, entry.action);
        }
    }
}

#[test]
fn test_s001_to_tm33_23_conversion() {
    let path = samples_dir().join("fluorescent.xml");
    let doc = atla::parse_file(&path).unwrap();

    #[cfg(feature = "eulumdat")]
    {
        use atla::convert::{atla_to_tm33, ConversionPolicy};

        // Convert with compatible policy (apply defaults)
        let (tm33_doc, log) = atla_to_tm33(&doc, ConversionPolicy::Compatible).unwrap();

        // Should have TM-33-23 schema version
        assert_eq!(tm33_doc.schema_version, atla::SchemaVersion::Tm3323);

        // Core data should be preserved
        assert_eq!(tm33_doc.header.manufacturer, doc.header.manufacturer);
        assert_eq!(tm33_doc.emitters.len(), doc.emitters.len());
        assert_eq!(
            tm33_doc.emitters[0].rated_lumens,
            doc.emitters[0].rated_lumens
        );

        // Should have defaults applied for required TM-33-23 fields
        assert!(
            tm33_doc.header.report_date.is_some(),
            "ReportDate should be defaulted"
        );
        assert!(
            tm33_doc.emitters[0].description.is_some(),
            "Emitter description should be preserved/defaulted"
        );

        println!("Conversion log entries: {}", log.len());
        for entry in &log {
            println!(
                "  {}: {:?} ({:?} -> {:?})",
                entry.field, entry.action, entry.original_value, entry.new_value
            );
        }
    }
}

#[test]
fn test_tm33_23_write_roundtrip() {
    let path = tm33_samples_dir().join("minimal.xml");
    if !path.exists() {
        eprintln!("Skipping test: TM-33-23 minimal sample not found");
        return;
    }

    let original = atla::parse_file(&path).unwrap();

    // Write as TM-33-23
    let xml_output =
        atla::xml::write_with_schema(&original, atla::SchemaVersion::Tm3323, Some(2)).unwrap();

    // Should have TM-33-23 root element
    assert!(
        xml_output.contains("<IESTM33-22>"),
        "Output should have TM-33-23 root element"
    );
    assert!(
        xml_output.contains("<Version>1.1</Version>"),
        "Output should have version 1.1"
    );

    // Parse back
    let reparsed = atla::xml::parse(&xml_output).unwrap();

    // Schema version should be preserved
    assert_eq!(reparsed.schema_version, atla::SchemaVersion::Tm3323);

    // Core data should match
    assert_eq!(original.header.manufacturer, reparsed.header.manufacturer);
    assert_eq!(original.emitters.len(), reparsed.emitters.len());
    assert_eq!(
        original.emitters[0].rated_lumens,
        reparsed.emitters[0].rated_lumens
    );
}

// ===========================================
// TM-33-23 Horticultural Tests
// ===========================================

#[test]
fn test_parse_horticultural_led() {
    let path = tm33_samples_dir().join("horticultural_led.xml");
    if !path.exists() {
        eprintln!(
            "Skipping test: horticultural_led.xml not found at {:?}",
            path
        );
        return;
    }

    let doc = atla::parse_file(&path).expect("Failed to parse horticultural LED file");

    // Check schema version
    assert_eq!(doc.schema_version, atla::SchemaVersion::Tm3323);
    assert_eq!(doc.version, "1.1");

    // Check header
    assert_eq!(
        doc.header.manufacturer,
        Some("GrowLight Technologies".to_string())
    );
    assert!(doc
        .header
        .description
        .as_ref()
        .unwrap()
        .contains("Horticultural"));

    // Check emitter
    let emitter = &doc.emitters[0];
    assert_eq!(emitter.rated_lumens, Some(42000.0));
    assert_eq!(emitter.input_watts, Some(600.0));
    assert_eq!(emitter.cct, Some(4000.0));
    assert!(emitter.ballast_factor.is_some());
    assert!(emitter.duv.is_some());

    // Check intensity distribution with symmetry
    let dist = emitter.intensity_distribution.as_ref().unwrap();
    assert_eq!(dist.symmetry, Some(atla::SymmetryType::Quad));
    assert_eq!(dist.multiplier, Some(1.0));

    // Check spectral distribution is present
    assert!(
        emitter.spectral_distribution.is_some(),
        "Should have spectral data"
    );
    let spd = emitter.spectral_distribution.as_ref().unwrap();
    assert!(!spd.wavelengths.is_empty(), "Should have wavelength data");

    // Check custom data for horticultural metrics
    assert!(
        !doc.custom_data_items.is_empty(),
        "Should have CustomData items"
    );
    let hort_metrics = doc
        .custom_data_items
        .iter()
        .find(|c| c.name == "HorticulturalMetrics");
    assert!(
        hort_metrics.is_some(),
        "Should have HorticulturalMetrics CustomData"
    );
}

#[test]
fn test_parse_far_red_supplemental() {
    let path = tm33_samples_dir().join("far_red_supplemental.xml");
    if !path.exists() {
        eprintln!("Skipping test: far_red_supplemental.xml not found");
        return;
    }

    let doc = atla::parse_file(&path).expect("Failed to parse far-red supplemental file");

    // Check schema version
    assert_eq!(doc.schema_version, atla::SchemaVersion::Tm3323);

    // Check emitter
    let emitter = &doc.emitters[0];
    assert_eq!(emitter.input_watts, Some(120.0));

    // Check bilateral symmetry
    let dist = emitter.intensity_distribution.as_ref().unwrap();
    assert_eq!(dist.symmetry, Some(atla::SymmetryType::Bi90));

    // Check spectral distribution for far-red peak
    let spd = emitter.spectral_distribution.as_ref().unwrap();
    assert!(!spd.wavelengths.is_empty());

    // Find peak wavelength (should be around 730nm)
    let max_idx = spd
        .values
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(i, _)| i);
    if let Some(idx) = max_idx {
        let peak_wavelength = spd.wavelengths[idx];
        assert!(
            (peak_wavelength - 730.0).abs() < 10.0,
            "Peak wavelength should be near 730nm, got {}",
            peak_wavelength
        );
    }

    // Check multiple comments
    assert!(
        doc.header.comments_list.len() >= 2,
        "Should have multiple comments"
    );
}

#[test]
fn test_parse_uv_supplemental() {
    use atla::SpectralMetrics;

    let path = tm33_samples_dir().join("uv_supplemental.xml");
    if !path.exists() {
        eprintln!("Skipping test: uv_supplemental.xml not found");
        return;
    }

    let doc = atla::parse_file(&path).expect("Failed to parse UV supplemental file");

    // Check schema version
    assert_eq!(doc.schema_version, atla::SchemaVersion::Tm3323);

    // Check full symmetry
    let dist = doc.emitters[0].intensity_distribution.as_ref().unwrap();
    assert_eq!(dist.symmetry, Some(atla::SymmetryType::Full));

    // Check spectral distribution has UV
    let spd = doc.emitters[0].spectral_distribution.as_ref().unwrap();
    let metrics = SpectralMetrics::from_spd(spd);

    assert!(metrics.has_uv, "UV supplemental should have UV data");
    assert!(
        metrics.wavelength_min < 400.0,
        "Should have wavelengths below 400nm"
    );
    assert!(metrics.uv_a_percent > 0.0, "Should have UV-A content");

    // Check for safety CustomData
    let safety_data = doc
        .custom_data_items
        .iter()
        .find(|c| c.name == "UVSafetyMetrics");
    assert!(
        safety_data.is_some(),
        "Should have UVSafetyMetrics CustomData"
    );
}

#[test]
fn test_parse_seedling_propagation() {
    let path = tm33_samples_dir().join("seedling_propagation.xml");
    if !path.exists() {
        eprintln!("Skipping test: seedling_propagation.xml not found");
        return;
    }

    let doc = atla::parse_file(&path).expect("Failed to parse seedling propagation file");

    // Check schema version
    assert_eq!(doc.schema_version, atla::SchemaVersion::Tm3323);

    // Check emitter with high CCT (blue-shifted)
    let emitter = &doc.emitters[0];
    assert_eq!(emitter.cct, Some(6500.0));
    assert_eq!(emitter.input_watts, Some(200.0));

    // Check quadrilateral symmetry
    let dist = emitter.intensity_distribution.as_ref().unwrap();
    assert_eq!(dist.symmetry, Some(atla::SymmetryType::Quad));

    // Verify spectral data has blue peak
    let spd = emitter.spectral_distribution.as_ref().unwrap();

    // Find peak wavelength (should be around 450nm for high blue)
    let max_idx = spd
        .values
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(i, _)| i);
    if let Some(idx) = max_idx {
        let peak_wavelength = spd.wavelengths[idx];
        assert!(
            peak_wavelength < 500.0,
            "Peak wavelength should be in blue range (<500nm), got {}",
            peak_wavelength
        );
    }

    // Check for multiple CustomData blocks
    assert!(
        doc.custom_data_items.len() >= 2,
        "Should have multiple CustomData items"
    );
}

#[test]
fn test_horticultural_validation() {
    use atla::validate::{validate_with_schema, ValidationSchema};

    let files = [
        "horticultural_led.xml",
        "far_red_supplemental.xml",
        "uv_supplemental.xml",
        "seedling_propagation.xml",
    ];

    for filename in &files {
        let path = tm33_samples_dir().join(filename);
        if !path.exists() {
            continue;
        }

        let doc = atla::parse_file(&path).unwrap();
        let result = validate_with_schema(&doc, ValidationSchema::Tm3323);

        assert!(
            result.is_valid(),
            "Horticultural file {} should be valid TM-33-23, errors: {:?}",
            filename,
            result.errors
        );
    }
}

// ===========================================
// TM-32-24 BIM Parameters Tests
// ===========================================

#[test]
fn test_bim_parameters_from_atla() {
    use atla::BimParameters;

    let path = samples_dir().join("fluorescent.xml");
    let doc = atla::parse_file(&path).expect("Failed to parse XML file");

    let bim = BimParameters::from_atla(&doc);

    // Check general parameters
    assert_eq!(bim.manufacturer, Some("ATLA Test Manufacturer".to_string()));
    assert_eq!(bim.catalog_number, Some("FL-T16-54W".to_string()));

    // Check photometric parameters
    assert_eq!(bim.cct_kelvin, Some(6500));
    assert!(bim.has_photometric_data());

    // Check lamp quantity
    assert_eq!(bim.lamp_quantity, Some(2));

    // Check total flux (from emitter)
    assert!(bim.total_luminous_flux.is_some());

    // Check summary generation
    let summary = bim.summary();
    assert!(summary.contains("6500K"));
}

#[test]
fn test_bim_parameters_from_tm33_horticultural() {
    use atla::BimParameters;

    let path = tm33_samples_dir().join("horticultural_led.xml");
    if !path.exists() {
        eprintln!("Skipping test: horticultural_led.xml not found");
        return;
    }

    let doc = atla::parse_file(&path).expect("Failed to parse horticultural file");
    let bim = BimParameters::from_atla(&doc);

    // Check manufacturer
    assert_eq!(bim.manufacturer, Some("GrowLight Technologies".to_string()));

    // Check photometric data
    assert_eq!(bim.cct_kelvin, Some(4000));

    // Check electrical data
    assert_eq!(bim.watts, Some(600.0));
    assert!(bim.has_electrical_data());

    // Check power factor if available
    if let Some(pf) = bim.power_factor {
        assert!(pf > 0.0 && pf <= 1.0);
        // Should have calculated apparent power
        assert!(bim.apparent_power_va.is_some());
    }
}

#[test]
fn test_bim_enum_parsing() {
    use atla::{HousingShape, LedDriveType, MountingType, VoltageType};

    // VoltageType
    assert_eq!(VoltageType::parse("AC"), Some(VoltageType::AC));
    assert_eq!(VoltageType::parse("dc"), Some(VoltageType::DC));
    assert_eq!(VoltageType::parse("Universal"), Some(VoltageType::UC));

    // MountingType
    assert_eq!(
        MountingType::parse("Recessed"),
        Some(MountingType::Recessed)
    );
    assert_eq!(MountingType::parse("pendant"), Some(MountingType::Pendant));
    assert_eq!(
        MountingType::parse("IN-GROUND"),
        Some(MountingType::InGround)
    );

    // LedDriveType
    assert_eq!(
        LedDriveType::parse("CC"),
        Some(LedDriveType::ConstantCurrent)
    );
    assert_eq!(
        LedDriveType::parse("CV"),
        Some(LedDriveType::ConstantVoltage)
    );

    // HousingShape
    assert_eq!(HousingShape::parse("cuboid"), Some(HousingShape::Cuboid));
    assert_eq!(
        HousingShape::parse("circular"),
        Some(HousingShape::Cylinder)
    );
}

// ==========================================================================
// TM-32-24 Sample File Validation Tests
// ==========================================================================

fn tm32_24_samples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/samples/tm32-24")
}

#[test]
fn test_tm32_24_office_downlight_validation() {
    use atla::validate::{validate_with_schema, ValidationSchema};
    use atla::BimParameters;

    let path = tm32_24_samples_dir().join("office_downlight_bim.xml");
    let doc = atla::parse_file(&path).expect("Failed to parse office_downlight_bim.xml");

    // Should be detected as TM-33-24 (BIM)
    assert_eq!(doc.schema_version, atla::SchemaVersion::Tm3323);

    // Validate against TM-33-23 rules (base requirements)
    let result_tm33 = validate_with_schema(&doc, ValidationSchema::Tm3323);
    assert!(
        result_tm33.is_valid(),
        "TM-33-23 validation failed: {:?}",
        result_tm33.errors
    );

    // Validate against TM-32-24 rules (BIM requirements)
    let result_tm32 = validate_with_schema(&doc, ValidationSchema::Tm3224);
    assert!(
        result_tm32.is_valid(),
        "TM-32-24 validation failed: {:?}",
        result_tm32.errors
    );

    // Check BIM parameters are populated
    let bim = BimParameters::from_atla(&doc);
    assert!(
        bim.populated_count() >= 10,
        "Expected at least 10 BIM parameters, got {}",
        bim.populated_count()
    );

    // Verify key BIM parameters
    assert!(bim.manufacturer.is_some(), "Manufacturer should be set");
    assert!(bim.cct_kelvin.is_some(), "CCT should be set");
    assert!(bim.cri.is_some(), "CRI should be set");
    assert!(bim.watts.is_some(), "Watts should be set");
}

#[test]
fn test_tm32_24_road_luminaire_validation() {
    use atla::validate::{validate_with_schema, ValidationSchema};
    use atla::BimParameters;

    let path = tm32_24_samples_dir().join("road_luminaire_bim.xml");
    let doc = atla::parse_file(&path).expect("Failed to parse road_luminaire_bim.xml");

    // Validate against TM-33-23 rules
    let result_tm33 = validate_with_schema(&doc, ValidationSchema::Tm3323);
    assert!(
        result_tm33.is_valid(),
        "TM-33-23 validation failed: {:?}",
        result_tm33.errors
    );

    // Validate against TM-32-24 rules
    let result_tm32 = validate_with_schema(&doc, ValidationSchema::Tm3224);
    assert!(
        result_tm32.is_valid(),
        "TM-32-24 validation failed: {:?}",
        result_tm32.errors
    );

    // Check BIM parameters are populated
    let bim = BimParameters::from_atla(&doc);
    assert!(
        bim.populated_count() >= 15,
        "Expected at least 15 BIM parameters for road luminaire, got {}",
        bim.populated_count()
    );

    // Road luminaire should have BUG rating
    assert!(bim.bug_rating.is_some(), "BUG rating should be set");

    // Road luminaire should have mounting type
    assert!(bim.mounting_type.is_some(), "Mounting type should be set");
}

#[test]
fn test_tm32_24_samples_have_required_header_fields() {
    // Both samples should have all required TM-33-23 header fields
    for filename in ["office_downlight_bim.xml", "road_luminaire_bim.xml"] {
        let path = tm32_24_samples_dir().join(filename);
        let doc =
            atla::parse_file(&path).unwrap_or_else(|_| panic!("Failed to parse {}", filename));

        // Required TM-33-23 fields
        assert!(
            doc.header.description.is_some(),
            "{}: Missing Header.Description",
            filename
        );
        assert!(
            doc.header.laboratory.is_some(),
            "{}: Missing Header.Laboratory",
            filename
        );
        assert!(
            doc.header.report_number.is_some(),
            "{}: Missing Header.ReportNumber",
            filename
        );
        assert!(
            doc.header.report_date.is_some(),
            "{}: Missing Header.ReportDate",
            filename
        );

        // Required TM-32-24 fields
        assert!(
            doc.header.manufacturer.is_some(),
            "{}: Missing Header.Manufacturer",
            filename
        );
        assert!(
            doc.header.catalog_number.is_some(),
            "{}: Missing Header.CatalogNumber",
            filename
        );
    }
}
