//! Generate Wikipedia-quality SVG diagrams for beam angle article
//!
//! This example generates several SVGs demonstrating:
//! 1. Polar diagram with beam/field angle overlays (IES definition)
//! 2. Polar diagram showing IES vs CIE definitions for batwing distribution
//! 3. Cone diagram showing beam spread
//!
//! Run with: cargo run --example generate_wikipedia_svgs

#![allow(clippy::field_reassign_with_default)]

use eulumdat::{
    diagram::{ConeDiagram, PolarDiagram, SvgTheme},
    Eulumdat, PhotometricCalculations,
};
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = Path::new("screenshots/wikipedia");
    fs::create_dir_all(output_dir)?;

    // Create themes
    let light_theme = SvgTheme::light();
    let _dark_theme = SvgTheme::dark();

    println!("Generating Wikipedia beam angle illustrations...\n");

    // 1. Standard spotlight/downlight example
    println!("1. Generating standard spotlight diagram...");
    let spotlight = create_spotlight_distribution();
    generate_spotlight_diagrams(&spotlight, &light_theme, output_dir)?;

    // 2. Batwing distribution example (shows IES vs CIE difference)
    println!("2. Generating batwing distribution diagram...");
    let batwing = create_batwing_distribution();
    generate_batwing_diagrams(&batwing, &light_theme, output_dir)?;

    // 3. Wide flood example
    println!("3. Generating wide flood diagram...");
    let wide_flood = create_wide_flood_distribution();
    generate_flood_diagrams(&wide_flood, &light_theme, output_dir)?;

    // 4. Cone diagram example
    println!("4. Generating cone diagram...");
    generate_cone_diagram(&light_theme, output_dir)?;

    println!("\n✓ All SVGs generated in: {}", output_dir.display());
    println!("\nFiles generated:");
    for entry in fs::read_dir(output_dir)? {
        let entry = entry?;
        println!("  - {}", entry.file_name().to_string_lossy());
    }

    Ok(())
}

/// Create a standard spotlight/downlight distribution
fn create_spotlight_distribution() -> Eulumdat {
    let mut ldt = Eulumdat::default();
    ldt.luminaire_name = "Standard LED Spotlight".to_string();
    ldt.symmetry = eulumdat::Symmetry::VerticalAxis;

    // Standard narrow beam spotlight
    ldt.g_angles = (0..=90).step_by(5).map(|a| a as f64).collect();
    ldt.c_angles = vec![0.0];

    // Gaussian-like intensity distribution
    let intensities: Vec<f64> = ldt
        .g_angles
        .iter()
        .map(|&angle| {
            let sigma: f64 = 15.0; // Narrow beam
            let intensity = 1000.0 * (-angle.powi(2) / (2.0 * sigma.powi(2))).exp();
            intensity.max(0.0)
        })
        .collect();

    ldt.intensities = vec![intensities];
    ldt
}

/// Create a batwing distribution (center-beam < max)
///
/// This creates a distribution similar to the Wikipedia article examples:
/// - Center intensity is about 60-70% of maximum
/// - Peak intensity at around 35-45° gamma
/// - Results in IES beam angle ~90-100° and CIE beam angle ~110-130°
fn create_batwing_distribution() -> Eulumdat {
    let mut ldt = Eulumdat::default();
    ldt.luminaire_name = "Batwing Office Luminaire".to_string();
    ldt.symmetry = eulumdat::Symmetry::VerticalAxis;

    ldt.g_angles = (0..=90).step_by(5).map(|a| a as f64).collect();
    ldt.c_angles = vec![0.0];

    // Batwing distribution matching Wikipedia examples:
    // - Center (0°) intensity is lower than maximum
    // - Peak intensity around 40°
    // - Gradual drop-off to 90°
    let intensities: Vec<f64> = vec![
        450.0, // 0° - center (lower than peak)
        470.0, // 5°
        510.0, // 10°
        560.0, // 15°
        620.0, // 20°
        680.0, // 25°
        730.0, // 30°
        760.0, // 35°
        780.0, // 40° - near peak
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
    ];

    ldt.intensities = vec![intensities];
    ldt
}

/// Create a wide flood distribution
fn create_wide_flood_distribution() -> Eulumdat {
    let mut ldt = Eulumdat::default();
    ldt.luminaire_name = "Wide Flood Light".to_string();
    ldt.symmetry = eulumdat::Symmetry::VerticalAxis;

    ldt.g_angles = (0..=90).step_by(5).map(|a| a as f64).collect();
    ldt.c_angles = vec![0.0];

    // Wide cosine-like distribution
    let intensities: Vec<f64> = ldt
        .g_angles
        .iter()
        .map(|&angle| {
            let cos_val = angle.to_radians().cos();
            800.0 * cos_val.max(0.0)
        })
        .collect();

    ldt.intensities = vec![intensities];
    ldt
}

fn generate_spotlight_diagrams(
    ldt: &Eulumdat,
    theme: &SvgTheme,
    output_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let polar = PolarDiagram::from_eulumdat(ldt);
    let analysis = PhotometricCalculations::beam_field_analysis(ldt);

    // Standard polar with beam/field overlay
    let svg = polar.to_svg_with_beam_field_angles(600.0, 600.0, theme, &analysis, false);
    fs::write(output_dir.join("polar_spotlight_beam_field.svg"), &svg)?;

    // Print analysis
    println!("   Spotlight analysis:");
    println!("   - Beam angle (IES): {:.1}°", analysis.beam_angle_ies);
    println!("   - Field angle (IES): {:.1}°", analysis.field_angle_ies);
    println!(
        "   - Center/Max ratio: {:.1}%",
        analysis.center_to_max_ratio() * 100.0
    );

    Ok(())
}

fn generate_batwing_diagrams(
    ldt: &Eulumdat,
    theme: &SvgTheme,
    output_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let polar = PolarDiagram::from_eulumdat(ldt);
    let analysis = PhotometricCalculations::beam_field_analysis(ldt);

    // Polar showing both IES and CIE definitions
    let svg = polar.to_svg_with_beam_field_angles(600.0, 600.0, theme, &analysis, true);
    fs::write(output_dir.join("polar_batwing_ies_vs_cie.svg"), &svg)?;

    // Print analysis
    println!("   Batwing analysis:");
    println!("   - Beam angle (IES): {:.1}°", analysis.beam_angle_ies);
    println!("   - Beam angle (CIE): {:.1}°", analysis.beam_angle_cie);
    println!("   - Difference: {:.1}°", analysis.beam_angle_difference());
    println!(
        "   - Center/Max ratio: {:.1}%",
        analysis.center_to_max_ratio() * 100.0
    );
    println!("   - Distribution type: {}", analysis.distribution_type());

    Ok(())
}

fn generate_flood_diagrams(
    ldt: &Eulumdat,
    theme: &SvgTheme,
    output_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let polar = PolarDiagram::from_eulumdat(ldt);
    let analysis = PhotometricCalculations::beam_field_analysis(ldt);

    let svg = polar.to_svg_with_beam_field_angles(600.0, 600.0, theme, &analysis, false);
    fs::write(output_dir.join("polar_wide_flood.svg"), &svg)?;

    println!("   Wide flood analysis:");
    println!("   - Beam angle: {:.1}°", analysis.beam_angle_ies);
    println!("   - Field angle: {:.1}°", analysis.field_angle_ies);

    Ok(())
}

fn generate_cone_diagram(
    theme: &SvgTheme,
    output_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create a sample luminaire and generate cone diagram from it
    let mut ldt = Eulumdat::default();
    ldt.luminaire_name = "Sample Downlight".to_string();
    ldt.symmetry = eulumdat::Symmetry::VerticalAxis;
    ldt.g_angles = (0..=90).step_by(5).map(|a| a as f64).collect();
    ldt.c_angles = vec![0.0];

    // Create distribution that gives ~36° beam angle and ~60° field angle
    let intensities: Vec<f64> = ldt
        .g_angles
        .iter()
        .map(|&angle| {
            let sigma: f64 = 22.0;
            let intensity = 500.0 * (-angle.powi(2) / (2.0 * sigma.powi(2))).exp();
            intensity.max(0.0)
        })
        .collect();
    ldt.intensities = vec![intensities];

    let cone = ConeDiagram::from_eulumdat(&ldt, 3.0);

    // Standard cone diagram
    let svg = cone.to_svg(500.0, 450.0, theme);
    fs::write(output_dir.join("cone_diagram_standard.svg"), &svg)?;

    // Wikipedia-enhanced version
    let svg = cone.to_svg_wikipedia(600.0, 550.0, theme);
    fs::write(output_dir.join("cone_diagram_wikipedia.svg"), &svg)?;

    println!("   Cone diagram:");
    println!("   - Beam angle: {:.1}°", cone.beam_angle);
    println!("   - Field angle: {:.1}°", cone.field_angle);
    println!("   - Beam diameter at 3m: {:.2}m", cone.beam_diameter);
    println!("   - Classification: {}", cone.beam_classification());

    Ok(())
}
