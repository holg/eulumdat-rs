//! Integration tests for eulumdat crate.

use eulumdat::{Eulumdat, IesExporter, PhotometricCalculations, Symmetry, TypeIndicator};

const TEST_LDT: &str = r#"RUCO Licht GmbH
1
1
1
0
19
5
0
SY-D-1500-195-30-5-ACW
Supersky
SY-D-1500-195-30-5-ACW
22.11.2021
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
LED 830
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
386.8106
384.3521
377.1348
365.4063
349.7785
330.3969
307.8906
283.0269
256.5533
228.7582
200.5965
172.3095
144.3752
116.2986
88.6199
62.3291
38.4389
17.8507
0
"#;

#[test]
fn test_parse_real_ldt() {
    let ldt = Eulumdat::parse(TEST_LDT).unwrap();

    assert_eq!(ldt.identification, "RUCO Licht GmbH");
    assert_eq!(ldt.type_indicator, TypeIndicator::PointSourceSymmetric);
    assert_eq!(ldt.symmetry, Symmetry::VerticalAxis);
    assert_eq!(ldt.num_c_planes, 1);
    assert_eq!(ldt.num_g_planes, 19);
    assert_eq!(ldt.luminaire_name, "SY-D-1500-195-30-5-ACW");
    assert_eq!(ldt.luminaire_number, "Supersky");
    assert_eq!(ldt.lamp_sets.len(), 1);
    assert_eq!(ldt.lamp_sets[0].lamp_type, "LED 830");
    assert!((ldt.lamp_sets[0].total_luminous_flux - 19800.0).abs() < 0.001);
    assert!((ldt.lamp_sets[0].wattage_with_ballast - 195.0).abs() < 0.001);

    // Check dimensions
    assert!((ldt.length - 1500.0).abs() < 0.001);
    assert!((ldt.height - 129.0).abs() < 0.001);

    // Check intensity data
    assert_eq!(ldt.intensities.len(), 1); // 1 C-plane for vertical symmetry
    assert_eq!(ldt.intensities[0].len(), 19); // 19 G-planes
    assert!((ldt.intensities[0][0] - 386.8106).abs() < 0.001); // First intensity
    assert!((ldt.intensities[0][18] - 0.0).abs() < 0.001); // Last intensity (0 at 90°)
}

#[test]
fn test_validation() {
    let ldt = Eulumdat::parse(TEST_LDT).unwrap();
    let warnings = ldt.validate();

    // Print warnings for debugging
    for w in &warnings {
        println!("Warning: {}", w);
    }

    // Should have no critical errors
    assert!(ldt.validate_strict().is_ok());
}

#[test]
fn test_roundtrip() {
    let ldt = Eulumdat::parse(TEST_LDT).unwrap();
    let output = ldt.to_ldt();

    // Parse the output again
    let ldt2 = Eulumdat::parse(&output).unwrap();

    // Compare key fields
    assert_eq!(ldt.identification, ldt2.identification);
    assert_eq!(ldt.type_indicator, ldt2.type_indicator);
    assert_eq!(ldt.symmetry, ldt2.symmetry);
    assert_eq!(ldt.luminaire_name, ldt2.luminaire_name);
    assert_eq!(ldt.lamp_sets.len(), ldt2.lamp_sets.len());
    assert_eq!(ldt.intensities.len(), ldt2.intensities.len());

    // Check intensity data matches
    for (row1, row2) in ldt.intensities.iter().zip(ldt2.intensities.iter()) {
        for (v1, v2) in row1.iter().zip(row2.iter()) {
            assert!(
                (v1 - v2).abs() < 0.01,
                "Intensity mismatch: {} vs {}",
                v1,
                v2
            );
        }
    }
}

#[test]
fn test_calculations() {
    let ldt = Eulumdat::parse(TEST_LDT).unwrap();

    // Total luminous flux
    let flux = ldt.total_luminous_flux();
    assert!((flux - 19800.0).abs() < 0.001);

    // Wattage
    let wattage = ldt.total_wattage();
    assert!((wattage - 195.0).abs() < 0.001);

    // Luminous efficacy
    let efficacy = ldt.luminous_efficacy();
    assert!((efficacy - 101.538).abs() < 1.0); // ~101.5 lm/W

    // Max intensity
    let max = ldt.max_intensity();
    assert!((max - 386.8106).abs() < 0.001);

    // Beam angle (full angle per CIE S 017:2020)
    let beam = PhotometricCalculations::beam_angle(&ldt);
    assert!(beam > 0.0 && beam < 180.0, "Beam angle: {}", beam);
}

#[test]
fn test_ies_export() {
    let ldt = Eulumdat::parse(TEST_LDT).unwrap();
    let ies = IesExporter::export(&ldt);

    // Default export is now LM-63-2019
    assert!(ies.contains("IES:LM-63-2019"));
    assert!(ies.contains("[LUMINAIRE] SY-D-1500-195-30-5-ACW"));
    assert!(ies.contains("[ISSUEDATE]")); // Required in LM-63-2019
    assert!(ies.contains("TILT=NONE"));

    // Check that intensity data is present (absolute candela: 386.8 cd/klm * 19.8 = 7658.65 cd)
    assert!(
        ies.contains("7658"),
        "IES should contain absolute candela values"
    );

    // Test legacy 2002 export
    let ies_2002 = IesExporter::export_2002(&ldt);
    assert!(ies_2002.contains("IESNA:LM-63-2002"));
    assert!(!ies_2002.contains("[ISSUEDATE]")); // Not required in 2002
}

#[test]
fn test_direct_ratios() {
    let ldt = Eulumdat::parse(TEST_LDT).unwrap();

    // Check stored direct ratios
    assert!((ldt.direct_ratios[0] - 0.358).abs() < 0.001);
    assert!((ldt.direct_ratios[9] - 0.874).abs() < 0.001);
}

#[test]
fn test_symmetry_calc() {
    // Vertical axis symmetry should use only 1 C-plane
    let ldt = Eulumdat::parse(TEST_LDT).unwrap();
    assert_eq!(ldt.actual_c_planes(), 1);

    // Test symmetry calculation
    assert_eq!(Symmetry::None.calc_mc(36), 36);
    assert_eq!(Symmetry::VerticalAxis.calc_mc(36), 1);
    assert_eq!(Symmetry::PlaneC0C180.calc_mc(36), 19);
    assert_eq!(Symmetry::PlaneC90C270.calc_mc(36), 19);
    assert_eq!(Symmetry::BothPlanes.calc_mc(36), 10);
}

#[test]
fn test_european_number_format() {
    // Test file with European comma as decimal separator
    let content = r#"Test
1
1
1
0
3
5
Report
Luminaire
LUM-001
test.ldt
2024-01-01
100,5
50,25
30
80
40
0
0
0
0
100
85
1,0
0
1
1
LED
1000,5
3000K
80
10,5
0,5
0,55
0,6
0,65
0,7
0,75
0,8
0,82
0,85
0,88
0
0
45
90
100,5
80,25
50
"#;

    let ldt = Eulumdat::parse(content).unwrap();
    assert!((ldt.length - 100.5).abs() < 0.001);
    assert!((ldt.width - 50.25).abs() < 0.001);
    assert!((ldt.lamp_sets[0].total_luminous_flux - 1000.5).abs() < 0.001);
    assert!((ldt.intensities[0][0] - 100.5).abs() < 0.001);
}

#[test]
fn test_italo_ies_isolux() {
    // Test the real ITALO IES file that uses absolute photometry
    let content = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../eulumdat-wasm/templates/ITALO 1 5P5 S05 3.140-3M.IES"
    ))
    .unwrap();
    let ldt = eulumdat::IesParser::parse(&content).unwrap();

    eprintln!("=== ITALO IES parsed ===");
    eprintln!("num_lamps: {}", ldt.lamp_sets[0].num_lamps);
    eprintln!(
        "total_luminous_flux: {:.1}",
        ldt.lamp_sets[0].total_luminous_flux
    );
    eprintln!(
        "c_angles: {} ({:.0}..{:.0})",
        ldt.c_angles.len(),
        ldt.c_angles.first().unwrap_or(&0.0),
        ldt.c_angles.last().unwrap_or(&0.0)
    );
    eprintln!(
        "g_angles: {} ({:.0}..{:.0})",
        ldt.g_angles.len(),
        ldt.g_angles.first().unwrap_or(&0.0),
        ldt.g_angles.last().unwrap_or(&0.0)
    );
    eprintln!(
        "intensities: {}x{}",
        ldt.intensities.len(),
        ldt.intensities.first().map(|v| v.len()).unwrap_or(0)
    );
    eprintln!("symmetry: {:?}", ldt.symmetry);
    eprintln!(
        "sample(0,0)={:.2} sample(0,45)={:.2} sample(0,90)={:.2}",
        ldt.sample(0.0, 0.0),
        ldt.sample(0.0, 45.0),
        ldt.sample(0.0, 90.0)
    );
    eprintln!("max_intensity: {:.2}", ldt.max_intensity());

    let total_flux: f64 = ldt
        .lamp_sets
        .iter()
        .map(|ls| ls.total_luminous_flux * ls.num_lamps.unsigned_abs() as f64)
        .sum();
    eprintln!(
        "total_flux: {:.1}, flux_scale: {:.4}",
        total_flux,
        total_flux / 1000.0
    );

    use eulumdat::diagram::{IsoluxDiagram, IsoluxParams};
    let params = IsoluxParams {
        mounting_height: 10.0,
        tilt_angle: 0.0,
        area_half_width: 20.0,
        area_half_depth: 20.0,
        grid_resolution: 60,
    };
    let diagram = IsoluxDiagram::from_eulumdat(&ldt, 600.0, 500.0, params);

    eprintln!("max_lux: {:.4}", diagram.max_lux);
    eprintln!("cells: {}", diagram.cells.len());
    let nonzero = diagram.cells.iter().filter(|c| c.lux > 0.001).count();
    eprintln!("cells with lux>0.001: {}", nonzero);
    eprintln!("contours: {}", diagram.contours.len());

    assert!(
        diagram.max_lux > 0.0,
        "ITALO isolux must have positive illuminance"
    );
}
