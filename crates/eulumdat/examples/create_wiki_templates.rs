//! Create LDT template files for Wikipedia beam angle demonstrations
//!
//! Run with: cargo run -p eulumdat --example create_wiki_templates

#![allow(clippy::field_reassign_with_default)]

use eulumdat::{Eulumdat, LampSet, Symmetry, TypeIndicator};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = "crates/eulumdat-wasm/templates";

    // 1. Wiki Batwing - shows IES vs CIE difference
    println!("Creating wiki-batwing.ldt...");
    let batwing = create_wiki_batwing();
    fs::write(format!("{}/wiki-batwing.ldt", output_dir), batwing.to_ldt())?;

    // 2. Wiki Spotlight - narrow beam, standard distribution
    println!("Creating wiki-spotlight.ldt...");
    let spotlight = create_wiki_spotlight();
    fs::write(
        format!("{}/wiki-spotlight.ldt", output_dir),
        spotlight.to_ldt(),
    )?;

    // 3. Wiki Wide Flood - wide beam distribution
    println!("Creating wiki-flood.ldt...");
    let flood = create_wiki_flood();
    fs::write(format!("{}/wiki-flood.ldt", output_dir), flood.to_ldt())?;

    println!("\n✓ Templates created in {}/", output_dir);
    println!("  - wiki-batwing.ldt (IES vs CIE beam angle demo)");
    println!("  - wiki-spotlight.ldt (narrow beam ~30°)");
    println!("  - wiki-flood.ldt (wide flood ~120°)");

    Ok(())
}

/// Batwing distribution - center intensity lower than maximum
/// Demonstrates the difference between IES and CIE beam angle definitions
fn create_wiki_batwing() -> Eulumdat {
    let mut ldt = Eulumdat::default();

    // Metadata
    ldt.identification = "Wikipedia Beam Angle Demo".to_string();
    ldt.luminaire_name = "Batwing Distribution".to_string();
    ldt.luminaire_number = "WIKI-BAT-01".to_string();
    ldt.file_name = "wiki-batwing.ldt".to_string();
    ldt.date_user = "IES vs CIE Demo".to_string();

    // Type and symmetry
    ldt.type_indicator = TypeIndicator::PointSourceSymmetric;
    ldt.symmetry = Symmetry::VerticalAxis;

    // Dimensions (mm)
    ldt.length = 600.0;
    ldt.width = 600.0;
    ldt.height = 80.0;

    // Photometric properties
    ldt.downward_flux_fraction = 100.0;
    ldt.light_output_ratio = 85.0;

    // Lamp data
    ldt.lamp_sets = vec![LampSet {
        num_lamps: 1,
        lamp_type: "LED Panel 40W".to_string(),
        total_luminous_flux: 4000.0,
        color_appearance: "4000K".to_string(),
        color_rendering_group: "1A".to_string(),
        wattage_with_ballast: 40.0,
    }];

    // Angles - single C-plane (rotationally symmetric)
    ldt.num_c_planes = 1;
    ldt.c_plane_distance = 0.0;
    ldt.c_angles = vec![0.0];

    ldt.num_g_planes = 19;
    ldt.g_plane_distance = 5.0;
    ldt.g_angles = (0..=90).step_by(5).map(|a| a as f64).collect();

    // Batwing intensity distribution:
    // - Center (0°) is about 58% of maximum
    // - Peak around 40°
    // - Shows clear IES vs CIE difference
    ldt.intensities = vec![vec![
        450.0, // 0° - center (dip)
        470.0, // 5°
        510.0, // 10°
        560.0, // 15°
        620.0, // 20°
        680.0, // 25°
        730.0, // 30°
        760.0, // 35°
        780.0, // 40° - peak
        770.0, // 45°
        740.0, // 50°
        680.0, // 55°
        590.0, // 60°
        470.0, // 65°
        340.0, // 70°
        200.0, // 75°
        90.0,  // 80°
        30.0,  // 85°
        5.0,   // 90°
    ]];

    ldt
}

/// Narrow spotlight - standard center-peak distribution
fn create_wiki_spotlight() -> Eulumdat {
    let mut ldt = Eulumdat::default();

    ldt.identification = "Wikipedia Beam Angle Demo".to_string();
    ldt.luminaire_name = "Narrow Spotlight".to_string();
    ldt.luminaire_number = "WIKI-SPOT-01".to_string();
    ldt.file_name = "wiki-spotlight.ldt".to_string();
    ldt.date_user = "30deg Beam".to_string();

    ldt.type_indicator = TypeIndicator::PointSourceSymmetric;
    ldt.symmetry = Symmetry::VerticalAxis;

    ldt.length = 100.0;
    ldt.width = 100.0;
    ldt.height = 150.0;

    ldt.downward_flux_fraction = 100.0;
    ldt.light_output_ratio = 90.0;

    ldt.lamp_sets = vec![LampSet {
        num_lamps: 1,
        lamp_type: "LED COB 15W".to_string(),
        total_luminous_flux: 1500.0,
        color_appearance: "3000K".to_string(),
        color_rendering_group: "1A".to_string(),
        wattage_with_ballast: 15.0,
    }];

    ldt.num_c_planes = 1;
    ldt.c_plane_distance = 0.0;
    ldt.c_angles = vec![0.0];

    ldt.num_g_planes = 19;
    ldt.g_plane_distance = 5.0;
    ldt.g_angles = (0..=90).step_by(5).map(|a| a as f64).collect();

    // Narrow beam - Gaussian-like, ~30° beam angle
    ldt.intensities = vec![vec![
        1000.0, // 0° - maximum at center
        980.0,  // 5°
        920.0,  // 10°
        820.0,  // 15° - 50% threshold around here
        680.0,  // 20°
        520.0,  // 25°
        360.0,  // 30°
        220.0,  // 35°
        120.0,  // 40°
        60.0,   // 45°
        30.0,   // 50°
        15.0,   // 55°
        8.0,    // 60°
        4.0,    // 65°
        2.0,    // 70°
        1.0,    // 75°
        0.5,    // 80°
        0.2,    // 85°
        0.0,    // 90°
    ]];

    ldt
}

/// Wide flood - cosine-like distribution
fn create_wiki_flood() -> Eulumdat {
    let mut ldt = Eulumdat::default();

    ldt.identification = "Wikipedia Beam Angle Demo".to_string();
    ldt.luminaire_name = "Wide Flood".to_string();
    ldt.luminaire_number = "WIKI-FLOOD-01".to_string();
    ldt.file_name = "wiki-flood.ldt".to_string();
    ldt.date_user = "120deg Field".to_string();

    ldt.type_indicator = TypeIndicator::PointSourceSymmetric;
    ldt.symmetry = Symmetry::VerticalAxis;

    ldt.length = 300.0;
    ldt.width = 300.0;
    ldt.height = 50.0;

    ldt.downward_flux_fraction = 100.0;
    ldt.light_output_ratio = 88.0;

    ldt.lamp_sets = vec![LampSet {
        num_lamps: 1,
        lamp_type: "LED Array 30W".to_string(),
        total_luminous_flux: 3000.0,
        color_appearance: "4000K".to_string(),
        color_rendering_group: "1B".to_string(),
        wattage_with_ballast: 30.0,
    }];

    ldt.num_c_planes = 1;
    ldt.c_plane_distance = 0.0;
    ldt.c_angles = vec![0.0];

    ldt.num_g_planes = 19;
    ldt.g_plane_distance = 5.0;
    ldt.g_angles = (0..=90).step_by(5).map(|a| a as f64).collect();

    // Wide flood - cosine distribution, ~60° beam, ~120° field
    ldt.intensities = vec![vec![
        800.0, // 0°
        795.0, // 5°
        780.0, // 10°
        755.0, // 15°
        720.0, // 20°
        675.0, // 25°
        620.0, // 30° - 50% around 60°
        555.0, // 35°
        480.0, // 40°
        400.0, // 45°
        315.0, // 50°
        230.0, // 55°
        150.0, // 60°
        85.0,  // 65°
        40.0,  // 70°
        15.0,  // 75°
        5.0,   // 80°
        1.0,   // 85°
        0.0,   // 90°
    ]];

    ldt
}
