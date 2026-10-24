//! Command implementations for the CLI

use anyhow::{Context, Result};
use eulumdat::atla::LuminaireOpticalData;
use eulumdat::{
    batch::{self, BatchInput, ConversionFormat},
    diagram::{CartesianDiagram, PolarDiagram, SvgTheme},
    BugDiagram, Eulumdat, GldfPhotometricData, IesExporter, IesParser, PhotometricCalculations,
    PhotometricComparison, PhotometricSummary, Significance,
};
use std::path::{Path, PathBuf};

use crate::cli::CurveStyle;
use crate::cli::{
    AtlaSchemaType, CalcType, CompareDiagramType, CompareFormat, ConversionPolicyArg, DiagramType,
    OutputFormat, SummaryFormat,
};
use std::fs;

pub fn load_file(path: &PathBuf) -> Result<Eulumdat> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "ldt" => Eulumdat::from_file(path).context("Failed to parse LDT file"),
        "ies" => IesParser::parse_file(path).context("Failed to parse IES file"),
        "xml" | "json" => {
            // Parse ATLA format and convert to Eulumdat
            let atla_doc = eulumdat::atla::parse_file(path).context("Failed to parse ATLA file")?;
            Ok(atla_doc.to_eulumdat())
        }
        _ => anyhow::bail!("Unknown file extension: .{ext} (expected .ldt, .ies, .xml, or .json)"),
    }
}

/// Load file as ATLA document (preserves spectral data)
pub fn load_atla(path: &PathBuf) -> Result<LuminaireOpticalData> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "xml" | "json" => eulumdat::atla::parse_file(path).context("Failed to parse ATLA file"),
        "oxl" | "oxc" => {
            // OXL/OXC are OxyTech LITESTAR multi-luminaire catalogs.
            // `atla::parse_file` returns the first luminaire; callers
            // wanting all of them can use `eulumdat::atla::oxl::parse`.
            eulumdat::atla::parse_file(path).context("Failed to parse OXL/OXC file")
        }
        "ldt" => {
            let ldt = Eulumdat::from_file(path).context("Failed to parse LDT file")?;
            Ok(LuminaireOpticalData::from_eulumdat(&ldt))
        }
        "ies" => {
            let ldt = IesParser::parse_file(path).context("Failed to parse IES file")?;
            Ok(LuminaireOpticalData::from_eulumdat(&ldt))
        }
        _ => anyhow::bail!(
            "Unknown file extension: .{ext} (expected .ldt, .ies, .xml, .json, .oxl, or .oxc)"
        ),
    }
}

pub fn info(file: &PathBuf, verbose: bool, units: eulumdat::UnitSystem) -> Result<()> {
    let ldt = load_file(file)?;

    println!("File: {}", file.display());
    println!();
    println!("=== Luminaire Information ===");
    println!("Name:           {}", ldt.luminaire_name);
    println!("Number:         {}", ldt.luminaire_number);
    println!("Manufacturer:   {}", ldt.identification);
    println!("Date:           {}", ldt.date_user);
    println!();
    let dim_label = units.dimension_label();
    println!("=== Dimensions ({dim_label}) ===");
    println!("Length:         {}", units.format_dimension(ldt.length));
    println!("Width:          {}", units.format_dimension(ldt.width));
    println!("Height:         {}", units.format_dimension(ldt.height));
    println!();
    println!("=== Photometric Data ===");
    println!("Type:           {:?}", ldt.type_indicator);
    println!("Symmetry:       {:?}", ldt.symmetry);
    println!(
        "C-planes:       {} ({}° spacing)",
        ldt.c_angles.len(),
        ldt.c_plane_distance
    );
    println!(
        "Gamma angles:   {} ({}° spacing)",
        ldt.g_angles.len(),
        ldt.g_plane_distance
    );
    println!();
    println!("=== Lamp Data ===");
    for (i, lamp) in ldt.lamp_sets.iter().enumerate() {
        if ldt.lamp_sets.len() > 1 {
            println!("Lamp set {}:", i + 1);
        }
        println!("Type:           {}", lamp.lamp_type);
        println!("Quantity:       {}", lamp.num_lamps);
        println!("Luminous flux:  {:.0} lm", lamp.total_luminous_flux);
        println!("Color temp:     {}", lamp.color_appearance);
        println!("CRI:            {}", lamp.color_rendering_group);
        println!("Wattage:        {:.1} W", lamp.wattage_with_ballast);
    }
    println!();
    println!("=== Calculated Values ===");
    println!("Total flux:     {:.0} lm", ldt.luminous_flux());
    println!("Total wattage:  {:.1} W", ldt.wattage());
    println!("Efficacy:       {:.1} lm/W", ldt.luminous_efficacy());
    println!("Max intensity:  {:.1} cd/klm", ldt.max_intensity());
    println!("DFF:            {}%", ldt.downward_flux_fraction);
    println!("LORL:           {}%", ldt.light_output_ratio);

    if verbose {
        println!();
        println!("=== C-plane Angles ===");
        println!("{:?}", ldt.c_angles);
        println!();
        println!("=== Gamma Angles ===");
        println!("{:?}", ldt.g_angles);
        println!();
        println!("=== Intensity Data (cd/klm) ===");
        for (i, row) in ldt.intensities.iter().enumerate() {
            println!("C={:>5.1}°: {:?}", ldt.c_angles.get(i).unwrap_or(&0.0), row);
        }
    }

    Ok(())
}

pub fn validate(file: &PathBuf, strict: bool) -> Result<()> {
    let ldt = load_file(file)?;
    let warnings = ldt.validate();

    if warnings.is_empty() {
        println!("✓ {} is valid", file.display());
        return Ok(());
    }

    println!("Validation results for {}:", file.display());
    println!();

    for warning in &warnings {
        println!("[{}] {}", warning.code, warning.message);
    }

    println!();
    println!("Found {} warning(s)", warnings.len());

    if strict {
        ldt.validate_strict().map_err(|errors| {
            let msgs: Vec<_> = errors
                .iter()
                .map(|e| format!("[{}] {}", e.code, e.message))
                .collect();
            anyhow::anyhow!("Strict validation failed:\n{}", msgs.join("\n"))
        })?;
    }

    Ok(())
}

/// Load any luminaire file (.ldt / .ies / ATLA .xml / .json) as an ATLA document.
fn load_luminaire_doc(path: &Path, rotate: f64) -> Result<LuminaireOpticalData> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "xml" | "json" => eulumdat::atla::parse_file(path).context("Failed to parse ATLA file"),
        "ies" if rotate.abs() > 0.001 => {
            let opts = eulumdat::IesImportOptions {
                rotate_c_planes: rotate,
            };
            let ldt = IesParser::parse_file_with_options(path, &opts)
                .context("Failed to parse IES file")?;
            Ok(LuminaireOpticalData::from_eulumdat(&ldt))
        }
        _ => Ok(LuminaireOpticalData::from_eulumdat(&load_file(
            &path.to_path_buf(),
        )?)),
    }
}

/// Load a spectral file (.spd / .csv / .spdx) and return its SPD.
fn load_spd_file(path: &Path) -> Result<eulumdat::atla::SpectralDistribution> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext == "spdx" {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let spdx = eulumdat::atla::spdx::parse(&content).context("Failed to parse SPDX")?;
        eulumdat::atla::spdx::to_atla(&spdx)
            .emitters
            .into_iter()
            .find_map(|e| e.spectral_distribution)
            .ok_or_else(|| anyhow::anyhow!("SPDX file carries no spectral data"))
    } else {
        Ok(eulumdat::atla::spd_loader::load(path)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .with_context(|| format!("Failed to load SPD {}", path.display()))?
            .spd)
    }
}

/// Serialise an ATLA document to .xml or .json according to the output extension.
fn write_atla(doc: &LuminaireOpticalData, output: &Path, compact: bool) -> Result<()> {
    let ext = output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let content = match (ext.as_str(), compact) {
        ("xml", true) => {
            eulumdat::atla::xml::write_compact(doc).context("Failed to write ATLA XML")?
        }
        ("xml", false) => eulumdat::atla::xml::write(doc).context("Failed to write ATLA XML")?,
        ("json", true) => {
            eulumdat::atla::json::write_compact(doc).context("Failed to write ATLA JSON")?
        }
        ("json", false) => eulumdat::atla::json::write(doc).context("Failed to write ATLA JSON")?,
        _ => anyhow::bail!(
            "a spectrum can only be stored in ATLA output (.xml or .json), not .{ext}"
        ),
    };
    fs::write(output, content).with_context(|| format!("Failed to write {}", output.display()))?;
    Ok(())
}

pub fn convert(
    input: &PathBuf,
    output: &PathBuf,
    compact: bool,
    rotate: f64,
    spd: Option<&Path>,
    emitter: Option<usize>,
) -> Result<()> {
    let in_ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let out_ext = output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Attach a spectrum: any luminaire input → ATLA output carrying the SPD.
    if let Some(spd_path) = spd {
        let mut doc = load_luminaire_doc(input, rotate)?;
        let spd = load_spd_file(spd_path)?;
        let n = eulumdat::atla::spd_loader::attach_spd(&mut doc, &spd, emitter);
        write_atla(&doc, output, compact)?;
        println!(
            "Attached {} ({} samples) to {} emitter(s) → {}",
            spd_path.display(),
            spd.wavelengths.len(),
            n,
            output.display()
        );
        return Ok(());
    }

    // Helper: load file with optional C-plane rotation on IES import
    let load_with_rotation = |path: &PathBuf, rotation: f64| -> Result<Eulumdat> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        match ext.as_str() {
            "ies" if rotation.abs() > 0.001 => {
                let opts = eulumdat::IesImportOptions {
                    rotate_c_planes: rotation,
                };
                IesParser::parse_file_with_options(path, &opts).context("Failed to parse IES file")
            }
            _ => load_file(path),
        }
    };

    // Load the source data
    let content = match (in_ext.as_str(), out_ext.as_str()) {
        // ATLA input -> ATLA output (direct conversion)
        ("xml" | "json", "xml") => {
            let atla_doc =
                eulumdat::atla::parse_file(input).context("Failed to parse ATLA file")?;
            if compact {
                eulumdat::atla::xml::write_compact(&atla_doc).context("Failed to write ATLA XML")?
            } else {
                eulumdat::atla::xml::write(&atla_doc).context("Failed to write ATLA XML")?
            }
        }
        ("xml" | "json", "json") => {
            let atla_doc =
                eulumdat::atla::parse_file(input).context("Failed to parse ATLA file")?;
            if compact {
                eulumdat::atla::json::write_compact(&atla_doc)
                    .context("Failed to write ATLA JSON")?
            } else {
                eulumdat::atla::json::write(&atla_doc).context("Failed to write ATLA JSON")?
            }
        }
        // LDT/IES input -> ATLA output
        ("ldt" | "ies", "xml") => {
            let ldt = load_with_rotation(input, rotate)?;
            let atla_doc = eulumdat::atla::LuminaireOpticalData::from_eulumdat(&ldt);
            if compact {
                eulumdat::atla::xml::write_compact(&atla_doc).context("Failed to write ATLA XML")?
            } else {
                eulumdat::atla::xml::write(&atla_doc).context("Failed to write ATLA XML")?
            }
        }
        ("ldt" | "ies", "json") => {
            let ldt = load_with_rotation(input, rotate)?;
            let atla_doc = eulumdat::atla::LuminaireOpticalData::from_eulumdat(&ldt);
            if compact {
                eulumdat::atla::json::write_compact(&atla_doc)
                    .context("Failed to write ATLA JSON")?
            } else {
                eulumdat::atla::json::write(&atla_doc).context("Failed to write ATLA JSON")?
            }
        }
        // Any input -> LDT output (via Eulumdat)
        (_, "ldt") => {
            let ldt = load_with_rotation(input, rotate)?;
            ldt.to_ldt()
        }
        // Any input -> IES output (rotation applied on export for LDT sources)
        (_, "ies") => {
            let ldt = load_file(input)?;
            let opts = eulumdat::IesExportOptions {
                rotate_c_planes: rotate,
                ..Default::default()
            };
            IesExporter::export_with_options(&ldt, &opts)
        }
        _ => anyhow::bail!(
            "Unknown output extension: .{out_ext} (expected .ldt, .ies, .xml, or .json)"
        ),
    };

    std::fs::write(output, &content).context("Failed to write output file")?;

    let in_ext_upper = in_ext.to_uppercase();
    let out_ext_upper = out_ext.to_uppercase();
    let mut notes = Vec::new();
    if compact && (out_ext == "xml" || out_ext == "json") {
        notes.push("compact");
    }
    if rotate.abs() > 0.001 {
        notes.push("C-planes rotated");
    }
    let format_note = if notes.is_empty() {
        String::new()
    } else {
        format!(" [{}]", notes.join(", "))
    };

    println!(
        "Converted {} → {} ({} → {}){}",
        input.display(),
        output.display(),
        in_ext_upper,
        out_ext_upper,
        format_note
    );

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn diagram(
    input: &PathBuf,
    output: Option<&PathBuf>,
    diagram_type: DiagramType,
    dark: bool,
    theme_file: Option<&PathBuf>,
    width: f64,
    height: f64,
    mounting_height: f64,
    tilt: f64,
    log_scale: bool,
    units: eulumdat::UnitSystem,
) -> Result<()> {
    use eulumdat::diagram::*;

    // Base preset from --dark; if a --theme file is given, its overrides are
    // layered on top (and --dark still wins as the base it customises).
    let base = if dark { "dark" } else { "light" };
    let theme = match theme_file {
        Some(path) => SvgTheme::from_file(path, Some(base))
            .map_err(|e| anyhow::anyhow!("failed to load theme {}: {e}", path.display()))?,
        None => {
            if dark {
                SvgTheme::dark()
            } else {
                SvgTheme::light()
            }
        }
    };

    let svg = match diagram_type {
        DiagramType::Polar => {
            let ldt = load_file(input)?;
            let diagram = PolarDiagram::from_eulumdat(&ldt);
            diagram.to_svg(width, height, &theme)
        }
        DiagramType::Butterfly => {
            let ldt = load_file(input)?;
            let diagram = ButterflyDiagram::from_eulumdat(&ldt, width, height, 60.0);
            diagram.to_svg(width, height, &theme)
        }
        DiagramType::Cartesian => {
            let ldt = load_file(input)?;
            let diagram = CartesianDiagram::from_eulumdat(&ldt, width, height, 8);
            diagram.to_svg(width, height, &theme)
        }
        DiagramType::Heatmap => {
            let ldt = load_file(input)?;
            let diagram = HeatmapDiagram::from_eulumdat(&ldt, width, height);
            diagram.to_svg(width, height, &theme)
        }
        DiagramType::Cone => {
            let ldt = load_file(input)?;
            let diagram = ConeDiagram::from_eulumdat(&ldt, mounting_height);
            diagram.to_svg_with_units(width, height, &theme, &ConeDiagramLabels::default(), units)
        }
        DiagramType::BeamAngle => {
            let ldt = load_file(input)?;
            let diagram = PolarDiagram::from_eulumdat(&ldt);
            let analysis = PhotometricCalculations::beam_field_analysis(&ldt);
            let show_both = analysis.is_batwing;
            diagram.to_svg_with_beam_field_angles(width, height, &theme, &analysis, show_both)
        }
        DiagramType::Lcs => {
            let ldt = load_file(input)?;
            let diagram = BugDiagram::from_eulumdat(&ldt);
            diagram.to_lcs_svg(width, height, &theme)
        }
        DiagramType::Spectral => {
            let atla_doc = load_atla(input)?;
            let atla_theme = if dark {
                eulumdat::atla::spectral::SpectralTheme::dark()
            } else {
                eulumdat::atla::spectral::SpectralTheme::light()
            };

            // Try to get spectral data from emitters
            if let Some(spd) = atla_doc
                .emitters
                .iter()
                .filter_map(|e| e.spectral_distribution.as_ref())
                .next()
            {
                let diagram = eulumdat::atla::spectral::SpectralDiagram::from_spectral(spd);
                diagram.to_svg(width, height, &atla_theme)
            } else if let Some(emitter) = atla_doc.emitters.first() {
                // Try to synthesize from CCT/CRI
                if let Some(cct) = emitter.cct {
                    let cri = emitter.color_rendering.as_ref().and_then(|cr| cr.ra);
                    let spd = eulumdat::atla::spectral::synthesize_spectrum(cct, cri);
                    let diagram = eulumdat::atla::spectral::SpectralDiagram::from_spectral(&spd);
                    diagram.to_svg(width, height, &atla_theme)
                } else {
                    anyhow::bail!("No spectral data or CCT found in file. Spectral diagram requires spectral distribution or CCT.")
                }
            } else {
                anyhow::bail!("No emitter data found in file. Spectral diagram requires emitter with spectral distribution or CCT.")
            }
        }
        DiagramType::Greenhouse => {
            let atla_doc = load_atla(input)?;
            let gh_theme = if dark {
                eulumdat::atla::greenhouse::GreenhouseTheme::dark()
            } else {
                eulumdat::atla::greenhouse::GreenhouseTheme::light()
            };
            let diagram = eulumdat::atla::greenhouse::GreenhouseDiagram::from_atla_with_height(
                &atla_doc,
                mounting_height,
            );
            diagram.to_svg(width, height, &gh_theme)
        }
        DiagramType::FloodlightVh => {
            let ldt = load_file(input)?;
            let y_scale = if log_scale {
                YScale::Logarithmic
            } else {
                YScale::Linear
            };
            let diagram = FloodlightCartesianDiagram::from_eulumdat(&ldt, width, height, y_scale);
            diagram.to_svg(width, height, &theme)
        }
        DiagramType::Isolux => {
            let ldt = load_file(input)?;
            let params = IsoluxParams {
                mounting_height,
                tilt_angle: tilt,
                area_half_width: 20.0,
                area_half_depth: 20.0,
                grid_resolution: 80,
            };
            let diagram =
                IsoluxDiagram::from_eulumdat_with_units(&ldt, width, height, params, units);
            diagram.to_svg_with_units(width, height, &theme, units)
        }
        DiagramType::Isocandela => {
            let ldt = load_file(input)?;
            let diagram = IsocandelaDiagram::from_eulumdat(&ldt, width, height);
            diagram.to_svg(width, height, &theme)
        }
    };

    if let Some(out_path) = output {
        std::fs::write(out_path, &svg).context("Failed to write SVG file")?;
        println!(
            "Generated {:?} diagram: {}",
            diagram_type,
            out_path.display()
        );
    } else {
        println!("{svg}");
    }

    Ok(())
}

pub fn bug(file: &PathBuf, svg: Option<&PathBuf>, dark: bool) -> Result<()> {
    let ldt = load_file(file)?;
    let bug = BugDiagram::from_eulumdat(&ldt);

    println!("BUG Rating for {}:", file.display());
    println!();
    println!("Rating: {}", bug.rating);
    println!();
    println!("=== Zone Lumens ===");
    println!(
        "Backlight:  BL={:.1}  BM={:.1}  BH={:.1}  BVH={:.1}",
        bug.zones.bl, bug.zones.bm, bug.zones.bh, bug.zones.bvh
    );
    println!(
        "Forward:    FL={:.1}  FM={:.1}  FH={:.1}  FVH={:.1}",
        bug.zones.fl, bug.zones.fm, bug.zones.fh, bug.zones.fvh
    );
    println!("Uplight:    UL={:.1}  UH={:.1}", bug.zones.ul, bug.zones.uh);

    if let Some(out_path) = svg {
        let theme = if dark {
            SvgTheme::dark()
        } else {
            SvgTheme::light()
        };
        let svg_content = bug.to_svg(400.0, 350.0, &theme);
        std::fs::write(out_path, &svg_content).context("Failed to write SVG file")?;
        println!();
        println!("Generated BUG diagram: {}", out_path.display());
    }

    Ok(())
}

pub fn batch(
    input_dir: &PathBuf,
    output_dir: Option<&PathBuf>,
    format: OutputFormat,
    recursive: bool,
    overwrite: bool,
) -> Result<()> {
    use std::fs;

    if !input_dir.is_dir() {
        anyhow::bail!("Input path is not a directory: {}", input_dir.display());
    }

    let output_dir = output_dir.unwrap_or(input_dir);
    fs::create_dir_all(output_dir).context("Failed to create output directory")?;

    // Collect input files
    let mut batch_inputs = Vec::new();
    let walker = if recursive {
        walkdir::WalkDir::new(input_dir)
    } else {
        walkdir::WalkDir::new(input_dir).max_depth(1)
    };

    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext != "ldt" && ext != "ies" {
            continue;
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;

        batch_inputs.push((
            path.to_owned(),
            BatchInput {
                name: path.file_name().unwrap().to_string_lossy().to_string(),
                content,
                format: None, // Auto-detect
            },
        ));
    }

    if batch_inputs.is_empty() {
        println!("No .ldt or .ies files found in {}", input_dir.display());
        return Ok(());
    }

    println!("Found {} files to convert", batch_inputs.len());
    println!(
        "Converting to {} format...",
        match format {
            OutputFormat::Ldt => "LDT",
            OutputFormat::Ies => "IES",
        }
    );
    println!();

    // Batch convert
    let conversion_format = match format {
        OutputFormat::Ldt => ConversionFormat::Ldt,
        OutputFormat::Ies => ConversionFormat::Ies,
    };

    let inputs: Vec<_> = batch_inputs
        .iter()
        .map(|(_, input)| input.clone())
        .collect();
    let (outputs, stats) = batch::batch_convert_with_stats(&inputs, conversion_format);

    // Write outputs
    for (output, (original_path, _)) in outputs.iter().zip(batch_inputs.iter()) {
        if let Some(error) = &output.error {
            eprintln!("✗ {}: {}", output.input_name, error);
            continue;
        }

        let content = output.content.as_ref().unwrap();
        let mut out_path = output_dir.join(&output.output_name);

        // Preserve subdirectory structure if recursive
        if recursive {
            if let Ok(rel_path) = original_path.strip_prefix(input_dir) {
                if let Some(parent) = rel_path.parent() {
                    let out_subdir = output_dir.join(parent);
                    fs::create_dir_all(&out_subdir)?;
                    out_path = out_subdir.join(&output.output_name);
                }
            }
        }

        if out_path.exists() && !overwrite {
            eprintln!("✗ {}: Already exists (use --overwrite)", out_path.display());
            continue;
        }

        fs::write(&out_path, content)
            .with_context(|| format!("Failed to write {}", out_path.display()))?;

        println!("✓ {} → {}", output.input_name, out_path.display());
    }

    println!();
    println!("=== Batch Conversion Summary ===");
    println!("Total:      {}", stats.total);
    println!("Successful: {}", stats.successful);
    println!("Failed:     {}", stats.failed);

    if stats.failed > 0 {
        anyhow::bail!("{} file(s) failed to convert", stats.failed);
    }

    Ok(())
}

pub fn summary(file: &PathBuf, format: SummaryFormat, output: Option<&PathBuf>) -> Result<()> {
    let ldt = load_file(file)?;
    let summary = PhotometricSummary::from_eulumdat(&ldt);

    let content = match format {
        SummaryFormat::Text => {
            let mut s = format!("File: {}\n\n", file.display());
            s.push_str(&summary.to_text());
            s
        }
        SummaryFormat::Compact => summary.to_compact(),
        SummaryFormat::Json => {
            let kv = summary.to_key_value();
            let mut json = String::from("{\n");
            for (i, (key, value)) in kv.iter().enumerate() {
                let comma = if i < kv.len() - 1 { "," } else { "" };
                // Try to parse as number for proper JSON formatting
                if let Ok(num) = value.parse::<f64>() {
                    json.push_str(&format!("  \"{}\": {}{}\n", key, num, comma));
                } else {
                    json.push_str(&format!("  \"{}\": \"{}\"{}\n", key, value, comma));
                }
            }
            json.push('}');
            json
        }
    };

    if let Some(out_path) = output {
        std::fs::write(out_path, &content).context("Failed to write output file")?;
        println!("Summary written to: {}", out_path.display());
    } else {
        println!("{content}");
    }

    Ok(())
}

pub fn gldf(file: &PathBuf, output: Option<&PathBuf>, pretty: bool) -> Result<()> {
    let ldt = load_file(file)?;
    let gldf = GldfPhotometricData::from_eulumdat(&ldt);
    let props = gldf.to_gldf_properties();

    let json = if pretty {
        let mut s = String::from("{\n");
        for (i, (key, value)) in props.iter().enumerate() {
            let comma = if i < props.len() - 1 { "," } else { "" };
            // Try to parse as number for proper JSON formatting
            if let Ok(num) = value.parse::<f64>() {
                s.push_str(&format!("  \"{}\": {}{}\n", key, num, comma));
            } else {
                s.push_str(&format!("  \"{}\": \"{}\"{}\n", key, value, comma));
            }
        }
        s.push('}');
        s
    } else {
        let pairs: Vec<String> = props
            .iter()
            .map(|(k, v)| {
                if let Ok(num) = v.parse::<f64>() {
                    format!("\"{}\":{}", k, num)
                } else {
                    format!("\"{}\":\"{}\"", k, v)
                }
            })
            .collect();
        format!("{{{}}}", pairs.join(","))
    };

    if let Some(out_path) = output {
        std::fs::write(out_path, &json).context("Failed to write JSON file")?;
        println!("GLDF data exported to: {}", out_path.display());
    } else {
        println!("{json}");
    }

    Ok(())
}

pub fn calc(file: &PathBuf, calc_type: CalcType) -> Result<()> {
    let ldt = load_file(file)?;

    println!("Photometric calculations for: {}", file.display());
    println!();

    match calc_type {
        CalcType::CieCodes => {
            let codes = PhotometricCalculations::cie_flux_codes(&ldt);
            println!("=== CIE Flux Codes ===");
            println!("N1 (0-90°, DLOR):      {:.1}%", codes.n1);
            println!("N2 (0-60°):            {:.1}%", codes.n2);
            println!("N3 (0-40°):            {:.1}%", codes.n3);
            println!("N4 (90-180°, ULOR):    {:.1}%", codes.n4);
            println!("N5 (90-120°):          {:.1}%", codes.n5);
            println!();
            println!("CIE Flux Code: {}", codes);
        }
        CalcType::BeamAngles => {
            let a = PhotometricCalculations::beam_field_analysis(&ldt);
            let cut_off = PhotometricCalculations::cut_off_angle(&ldt);
            let fmt = eulumdat::format_angle;

            println!("=== Beam Characteristics (CIE S 017 / IES: 50 % / 10 % of peak) ===");
            println!(
                "Peak:                  {:.1} cd/klm at C{:.0} γ{:.1}°",
                a.max_intensity, a.max_intensity_c, a.max_intensity_gamma
            );
            println!(
                "Beam Angle (50%):      {} (section {})",
                fmt(a.beam_angle_ies),
                a.peak_section.label()
            );
            println!("Field Angle (10%):     {}", fmt(a.field_angle_ies));
            println!("Cut-off Angle (2.5%):  {:.1}°", cut_off);
            println!();
            println!("=== Per-Section Angles (50 % / 10 % of the section peak) ===");
            println!(
                "C0–C180  beam / field: {} / {}",
                fmt(a.c0_c180.beam),
                fmt(a.c0_c180.field)
            );
            println!(
                "C90–C270 beam / field: {} / {}",
                fmt(a.c90_c270.beam),
                fmt(a.c90_c270.field)
            );
            println!();
            println!("=== Centre-Beam Angles (50 % / 10 % of I(γ=0), C0–C180) ===");
            println!(
                "Beam / Field:          {} / {}",
                fmt(a.beam_angle_cie),
                fmt(a.field_angle_cie)
            );
        }
        CalcType::Spacing => {
            let (s_c0, s_c90) = PhotometricCalculations::spacing_criteria(&ldt);
            let (sc_0_180, sc_90_270, sc_diag) =
                PhotometricCalculations::spacing_criteria_ies(&ldt);
            let code = PhotometricCalculations::photometric_code(&ldt);

            println!("=== Spacing Criteria (European/Intensity-based) ===");
            println!("S/H ratio (C0):        {:.2}", s_c0);
            println!("S/H ratio (C90):       {:.2}", s_c90);
            println!();
            println!("=== Spacing Criteria (IES/Illuminance-based) ===");
            println!("SC (0-180):            {:.2}", sc_0_180);
            println!("SC (90-270):           {:.2}", sc_90_270);
            println!("SC (Diagonal):         {:.2}", sc_diag);
            println!();
            println!("Photometric Code:      {}", code);
            println!();
            println!("Note: IES method accounts for illuminance uniformity on work plane.");
            println!("      European method uses 50% intensity drop-off angle.");
        }
        CalcType::ZonalLumens => {
            let zones = PhotometricCalculations::zonal_lumens_30deg(&ldt);
            let flux_90 = PhotometricCalculations::downward_flux(&ldt, 90.0);
            let flux_60 = PhotometricCalculations::downward_flux(&ldt, 60.0);
            let flux_40 = PhotometricCalculations::downward_flux(&ldt, 40.0);

            println!("=== Zonal Lumens (30° zones) ===");
            println!("0-30°:                 {:.1}%", zones.zone_0_30);
            println!("30-60°:                {:.1}%", zones.zone_30_60);
            println!("60-90°:                {:.1}%", zones.zone_60_90);
            println!("90-120°:               {:.1}%", zones.zone_90_120);
            println!("120-150°:              {:.1}%", zones.zone_120_150);
            println!("150-180°:              {:.1}%", zones.zone_150_180);
            println!();
            println!("=== Downward Totals ===");
            println!("Downward (0-90°):      {:.1}%", zones.downward_total());
            println!("Upward (90-180°):      {:.1}%", zones.upward_total());
            println!();
            println!("=== Cumulative Flux ===");
            println!("Within 40°:            {:.1}%", flux_40);
            println!("Within 60°:            {:.1}%", flux_60);
            println!("Within 90°:            {:.1}%", flux_90);
        }
        CalcType::CuTable => {
            let cu = PhotometricCalculations::cu_table(&ldt);
            println!("{}", cu.to_text());
        }
        CalcType::UgrTable => {
            let ugr = PhotometricCalculations::ugr_table(&ldt);
            println!("{}", ugr.to_text());
        }
        CalcType::CandelaTable => {
            let tab = PhotometricCalculations::candela_tabulation(&ldt);
            println!("{}", tab.to_text());
        }
        CalcType::Nema => {
            let nema = PhotometricCalculations::nema_classification(&ldt);
            println!("=== NEMA Floodlight Classification ===");
            println!("Designation:           {}", nema.designation);
            println!(
                "Beam Axis (Type B):    H {:.0}° / V {:.0}°",
                nema.peak_h, nema.peak_v
            );
            println!(
                "Horizontal Spread:     {} (Type {}, 10 % of I_max)",
                eulumdat::format_angle(nema.horizontal_spread),
                nema.horizontal_type
            );
            println!(
                "Vertical Spread:       {} (Type {}, 10 % of I_max)",
                eulumdat::format_angle(nema.vertical_spread),
                nema.vertical_type
            );
            println!("Peak Intensity (I_max):{:.0} cd/klm", nema.i_max);
        }
        CalcType::All => {
            // Print all calculations
            let summary = PhotometricSummary::from_eulumdat(&ldt);
            println!("{}", summary.to_text());
        }
    }

    Ok(())
}

pub fn validate_atla(
    file: &PathBuf,
    schema: Option<&PathBuf>,
    schema_type: AtlaSchemaType,
    use_xsd: bool,
) -> Result<()> {
    use eulumdat::atla::validate::{self, ValidationSchema};

    let ext = file
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Parse the file first
    let content = std::fs::read_to_string(file).context("Failed to read file")?;

    // For XML files, we can do XSD validation
    if ext == "xml" && use_xsd {
        let schema_name = match schema_type {
            AtlaSchemaType::Auto => "auto-detected",
            AtlaSchemaType::S001 => "ATLA S001",
            AtlaSchemaType::Tm3323 => "TM-33-23",
        };
        println!(
            "Validating {} against {} XSD schema...",
            file.display(),
            schema_name
        );
        println!();

        // Check if xmllint is available
        if !validate::is_xmllint_available() {
            eprintln!("Warning: xmllint not found. Install libxml2 for full XSD validation.");
            eprintln!("Falling back to structural validation only.");
            eprintln!();
        } else {
            // Do XSD validation
            let xsd_result = if let Some(schema_path) = schema {
                let schema_content =
                    std::fs::read_to_string(schema_path).context("Failed to read schema file")?;
                validate::validate_xsd_with_schema(&content, &schema_content)?
            } else {
                validate::validate_xsd(&content)?
            };

            if xsd_result.is_valid() {
                println!("XSD validation: PASSED");
            } else {
                println!("XSD validation: FAILED");
                for err in &xsd_result.errors {
                    println!("  {}", err);
                }
            }

            for warn in &xsd_result.warnings {
                println!("  Warning: {}", warn);
            }
            println!();
        }
    }

    // Parse and do structural validation
    let doc = eulumdat::atla::parse(&content).context("Failed to parse ATLA file")?;

    // Display detected schema version
    let detected_version = match doc.schema_version {
        eulumdat::atla::SchemaVersion::AtlaS001 => "ATLA S001 / TM-33-18",
        eulumdat::atla::SchemaVersion::Tm3323 => "TM-33-23 (IESTM33-22)",
        eulumdat::atla::SchemaVersion::Tm3324 => "TM-33-24",
    };

    println!("Structural validation for {}:", file.display());
    println!("  Detected schema: {}", detected_version);
    println!();

    // For XML files with Auto mode, validate against BOTH schemas and show results
    if ext == "xml" && schema_type == AtlaSchemaType::Auto {
        println!("=== ATLA S001 Validation ===");
        let s001_result = validate::validate_with_schema(&doc, ValidationSchema::AtlaS001);
        print_validation_result(&s001_result, "ATLA S001");

        println!();
        println!("=== TM-33-23 Validation ===");
        let tm33_result = validate::validate_with_schema(&doc, ValidationSchema::Tm3323);
        print_validation_result(&tm33_result, "TM-33-23");

        println!();
        println!("=== Summary ===");
        println!("  Version: {}", doc.version);
        println!("  Detected schema: {}", detected_version);
        println!("  Emitters: {}", doc.emitters.len());
        println!("  Total flux: {:.0} lm", doc.total_luminous_flux());
        println!("  Total power: {:.1} W", doc.total_input_watts());
        println!();

        let s001_valid = s001_result.is_valid();
        let tm33_valid = tm33_result.is_valid();

        println!("Results:");
        println!(
            "  ATLA S001: {}",
            if s001_valid { "VALID" } else { "INVALID" }
        );
        println!(
            "  TM-33-23:  {}",
            if tm33_valid { "VALID" } else { "INVALID" }
        );

        // Return success if valid for at least one schema (the detected one)
        let detected_valid = match doc.schema_version {
            eulumdat::atla::SchemaVersion::AtlaS001 => s001_valid,
            eulumdat::atla::SchemaVersion::Tm3323 | eulumdat::atla::SchemaVersion::Tm3324 => {
                tm33_valid
            }
        };

        if detected_valid {
            Ok(())
        } else {
            anyhow::bail!(
                "Validation failed for detected schema ({})",
                detected_version
            )
        }
    } else {
        // Single schema validation (explicit schema type or JSON input)
        let validation_schema = match schema_type {
            AtlaSchemaType::Auto => ValidationSchema::Auto,
            AtlaSchemaType::S001 => ValidationSchema::AtlaS001,
            AtlaSchemaType::Tm3323 => ValidationSchema::Tm3323,
        };

        let result = validate::validate_with_schema(&doc, validation_schema);

        if result.errors.is_empty() && result.warnings.is_empty() {
            println!("  All checks passed!");
        }

        if !result.errors.is_empty() {
            println!("Errors:");
            for err in &result.errors {
                println!("  {}", err);
            }
        }

        if !result.warnings.is_empty() {
            println!("Warnings:");
            for warn in &result.warnings {
                println!("  {}", warn);
            }
        }

        println!();
        println!("Summary:");
        println!("  Version: {}", doc.version);
        println!("  Schema: {}", detected_version);
        println!("  Emitters: {}", doc.emitters.len());
        println!("  Total flux: {:.0} lm", doc.total_luminous_flux());
        println!("  Total power: {:.1} W", doc.total_input_watts());

        if result.is_valid() {
            println!();
            println!("Result: VALID");
            Ok(())
        } else {
            println!();
            println!("Result: INVALID ({} errors)", result.errors.len());
            anyhow::bail!("Validation failed with {} errors", result.errors.len())
        }
    }
}

/// Helper function to print validation results
fn print_validation_result(result: &eulumdat::atla::validate::ValidationResult, schema_name: &str) {
    if result.errors.is_empty() && result.warnings.is_empty() {
        println!("  All {} checks passed!", schema_name);
    } else {
        if !result.errors.is_empty() {
            println!("  Errors ({}):", result.errors.len());
            for err in &result.errors {
                println!("    {}", err);
            }
        }
        if !result.warnings.is_empty() {
            println!("  Warnings ({}):", result.warnings.len());
            for warn in &result.warnings {
                println!("    {}", warn);
            }
        }
    }

    let status = if result.is_valid() {
        "VALID"
    } else {
        "INVALID"
    };
    println!("  Status: {}", status);
}

/// Convert ATLA document between schema versions
pub fn atla_convert(
    input: &PathBuf,
    output: &PathBuf,
    target: AtlaSchemaType,
    policy: ConversionPolicyArg,
    verbose: bool,
    compact: bool,
) -> Result<()> {
    use eulumdat::atla::convert::{atla_to_tm33, tm33_to_atla, ConversionPolicy};

    // Parse input file
    let content = std::fs::read_to_string(input).context("Failed to read input file")?;
    let doc = eulumdat::atla::parse(&content).context("Failed to parse ATLA file")?;

    // Get source and target schema names
    let source_name = match doc.schema_version {
        eulumdat::atla::SchemaVersion::AtlaS001 => "ATLA S001",
        eulumdat::atla::SchemaVersion::Tm3323 => "TM-33-23",
        eulumdat::atla::SchemaVersion::Tm3324 => "TM-33-24",
    };

    let target_schema = match target {
        AtlaSchemaType::Auto => {
            // Auto: convert to the "other" format
            match doc.schema_version {
                eulumdat::atla::SchemaVersion::AtlaS001 => eulumdat::atla::SchemaVersion::Tm3323,
                _ => eulumdat::atla::SchemaVersion::AtlaS001,
            }
        }
        AtlaSchemaType::S001 => eulumdat::atla::SchemaVersion::AtlaS001,
        AtlaSchemaType::Tm3323 => eulumdat::atla::SchemaVersion::Tm3323,
    };

    let target_name = match target_schema {
        eulumdat::atla::SchemaVersion::AtlaS001 => "ATLA S001",
        eulumdat::atla::SchemaVersion::Tm3323 => "TM-33-23",
        eulumdat::atla::SchemaVersion::Tm3324 => "TM-33-24",
    };

    println!("Converting {} → {}", source_name, target_name);
    println!("  Input: {}", input.display());
    println!("  Output: {}", output.display());
    println!();

    // Perform conversion
    let (converted_doc, log) = match (doc.schema_version, target_schema) {
        (
            eulumdat::atla::SchemaVersion::AtlaS001,
            eulumdat::atla::SchemaVersion::Tm3323 | eulumdat::atla::SchemaVersion::Tm3324,
        ) => {
            let conversion_policy = match policy {
                ConversionPolicyArg::Strict => ConversionPolicy::Strict,
                ConversionPolicyArg::Compatible => ConversionPolicy::Compatible,
            };
            atla_to_tm33(&doc, conversion_policy)?
        }
        (
            eulumdat::atla::SchemaVersion::Tm3323 | eulumdat::atla::SchemaVersion::Tm3324,
            eulumdat::atla::SchemaVersion::AtlaS001,
        ) => tm33_to_atla(&doc),
        _ => {
            // Same schema - just copy
            println!("  Note: Source and target schemas are the same.");
            (doc.clone(), vec![])
        }
    };

    // Show conversion log if verbose
    if verbose && !log.is_empty() {
        println!("Conversion log:");
        for entry in &log {
            let action_str = match entry.action {
                eulumdat::atla::convert::ConversionAction::Preserved => "Preserved",
                eulumdat::atla::convert::ConversionAction::DefaultApplied => "Default applied",
                eulumdat::atla::convert::ConversionAction::Renamed => "Renamed",
                eulumdat::atla::convert::ConversionAction::TypeConverted => "Type converted",
                eulumdat::atla::convert::ConversionAction::Dropped => "Dropped",
                eulumdat::atla::convert::ConversionAction::Warning => "Warning",
            };
            println!(
                "  [{}] {}: {} → {}",
                action_str,
                entry.field,
                entry.original_value.as_deref().unwrap_or("(none)"),
                entry.new_value.as_deref().unwrap_or("(none)")
            );
        }
        println!();
    }

    // Determine output format from extension
    let out_ext = output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("xml")
        .to_lowercase();

    let indent = if compact { None } else { Some(2) };

    let output_content = match out_ext.as_str() {
        "json" => {
            if compact {
                eulumdat::atla::json::write_compact(&converted_doc)?
            } else {
                eulumdat::atla::json::write(&converted_doc)?
            }
        }
        _ => {
            // XML output - use target schema format
            eulumdat::atla::xml::write_with_schema(&converted_doc, target_schema, indent)?
        }
    };

    std::fs::write(output, &output_content).context("Failed to write output file")?;

    // Summary
    let defaults_count = log
        .iter()
        .filter(|e| {
            matches!(
                e.action,
                eulumdat::atla::convert::ConversionAction::DefaultApplied
            )
        })
        .count();
    let dropped_count = log
        .iter()
        .filter(|e| matches!(e.action, eulumdat::atla::convert::ConversionAction::Dropped))
        .count();

    println!("Conversion complete!");
    if defaults_count > 0 {
        println!("  {} default value(s) applied", defaults_count);
    }
    if dropped_count > 0 {
        println!(
            "  {} field(s) dropped (not supported in target schema)",
            dropped_count
        );
    }

    Ok(())
}

/// Generate a photometric report
pub fn report(
    input: &PathBuf,
    output: &PathBuf,
    paper: crate::cli::PaperSize,
    compact: bool,
    cu_table: bool,
    ugr_table: bool,
    candela_table: bool,
) -> Result<()> {
    use eulumdat_typst::{ReportGenerator, ReportOptions, ReportSection};

    let ldt = load_file(input)?;
    let generator = ReportGenerator::new(&ldt);

    let mut sections = if compact {
        ReportSection::compact()
    } else {
        ReportSection::all()
    };

    // Add optional sections based on flags
    if cu_table {
        sections.push(ReportSection::CuTable);
    }
    if ugr_table {
        sections.push(ReportSection::UgrTable);
    }
    if candela_table {
        sections.push(ReportSection::CandelaTable);
    }

    let paper_size = match paper {
        crate::cli::PaperSize::A4 => eulumdat_typst::PaperSize::A4,
        crate::cli::PaperSize::Letter => eulumdat_typst::PaperSize::Letter,
        crate::cli::PaperSize::A3 => eulumdat_typst::PaperSize::A3,
    };

    let options = ReportOptions {
        sections,
        include_dark_theme: false,
        paper_size,
        language: "en".to_string(),
    };

    let out_ext = output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("pdf")
        .to_lowercase();

    match out_ext.as_str() {
        "typ" => {
            // Generate Typst source
            let source = generator.generate_typst(&options);
            std::fs::write(output, source).context("Failed to write Typst file")?;
            println!("Typst source written to: {}", output.display());
            println!(
                "Compile with: typst compile {} {}",
                output.display(),
                output.with_extension("pdf").display()
            );
        }
        "pdf" => {
            // Generate PDF directly
            let pdf = generator
                .generate_pdf(&options)
                .context("Failed to generate PDF")?;
            std::fs::write(output, pdf).context("Failed to write PDF file")?;
            println!("PDF report written to: {}", output.display());
        }
        _ => {
            anyhow::bail!(
                "Unknown output format: .{ext} (expected .typ or .pdf)",
                ext = out_ext
            );
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn compare(
    file_a: &PathBuf,
    file_b: &PathBuf,
    format: CompareFormat,
    diagram: Option<CompareDiagramType>,
    output: Option<&PathBuf>,
    dark: bool,
    significant_only: bool,
    units: eulumdat::UnitSystem,
) -> Result<()> {
    let ldt_a = load_file(file_a)?;
    let ldt_b = load_file(file_b)?;

    let label_a = file_a
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("A")
        .to_string();
    let label_b = file_b
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("B")
        .to_string();

    let comparison =
        PhotometricComparison::from_eulumdat_with_units(&ldt_a, &ldt_b, &label_a, &label_b, units);

    // Print comparison table
    match format {
        CompareFormat::Text => {
            if significant_only {
                let sig = comparison.significant_metrics(Significance::Minor);
                println!(
                    "COMPARISON: {} vs {} (significant only)",
                    comparison.label_a, comparison.label_b
                );
                println!("Similarity: {:.1}%", comparison.similarity_score * 100.0);
                println!("{}", "=".repeat(80));
                println!(
                    "{:<28} {:>12} {:>12} {:>10} {:>8}  Significance",
                    "Metric", "A", "B", "Delta", "%"
                );
                println!("{}", "-".repeat(80));
                for m in &sig {
                    let unit = if m.unit.is_empty() {
                        String::new()
                    } else {
                        format!(" {}", m.unit)
                    };
                    println!(
                        "{:<28} {:>10.1}{:<2} {:>10.1}{:<2} {:>+9.1} {:>+7.1}%  {}",
                        m.name,
                        m.value_a,
                        unit,
                        m.value_b,
                        unit,
                        m.delta,
                        m.delta_percent,
                        m.significance,
                    );
                }
                println!("{}", "-".repeat(80));
            } else {
                print!("{}", comparison.to_text());
            }
        }
        CompareFormat::Json => {
            // Build a simple JSON output
            println!("{{");
            println!("  \"label_a\": \"{}\",", comparison.label_a);
            println!("  \"label_b\": \"{}\",", comparison.label_b);
            println!(
                "  \"similarity_score\": {:.4},",
                comparison.similarity_score
            );
            println!("  \"metrics\": [");
            for (i, m) in comparison.metrics.iter().enumerate() {
                let comma = if i < comparison.metrics.len() - 1 {
                    ","
                } else {
                    ""
                };
                println!(
                    "    {{\"name\":\"{}\",\"key\":\"{}\",\"unit\":\"{}\",\"value_a\":{:.4},\"value_b\":{:.4},\"delta\":{:.4},\"delta_percent\":{:.4},\"significance\":\"{}\"}}{}",
                    m.name, m.key, m.unit, m.value_a, m.value_b, m.delta, m.delta_percent, m.significance, comma
                );
            }
            println!("  ]");
            println!("}}");
        }
        CompareFormat::Csv => {
            print!("{}", comparison.to_csv());
        }
    }

    // Generate overlay diagram if requested
    if let Some(diagram_type) = diagram {
        let theme = if dark {
            SvgTheme::dark()
        } else {
            SvgTheme::light()
        };

        let svg = match diagram_type {
            CompareDiagramType::Polar => {
                let polar_a = PolarDiagram::from_eulumdat(&ldt_a);
                let polar_b = PolarDiagram::from_eulumdat(&ldt_b);
                PolarDiagram::to_overlay_svg(
                    &polar_a, &polar_b, 500.0, 500.0, &theme, &label_a, &label_b,
                )
            }
            CompareDiagramType::Cartesian => {
                let cart_a = CartesianDiagram::from_eulumdat(&ldt_a, 600.0, 400.0, 4);
                let cart_b = CartesianDiagram::from_eulumdat(&ldt_b, 600.0, 400.0, 4);
                CartesianDiagram::to_overlay_svg(
                    &cart_a, &cart_b, 600.0, 400.0, &theme, &label_a, &label_b,
                )
            }
        };

        if let Some(out_path) = output {
            std::fs::write(out_path, &svg).context("Failed to write SVG file")?;
            println!("Overlay SVG written to: {}", out_path.display());
        } else {
            println!("{}", svg);
        }
    }

    Ok(())
}

/// Interpolate between photometric files at different LED operating points.
#[allow(clippy::too_many_arguments)]
pub fn interpolate(
    inputs: &[String],
    steps: Option<&[f64]>,
    range: Option<&str>,
    count: usize,
    at: Option<f64>,
    format: OutputFormat,
    output_dir: &PathBuf,
    param_name: &str,
    overwrite: bool,
) -> Result<()> {
    // Parse "file.ies:350" pairs
    let mut parsed_inputs: Vec<(Eulumdat, f64)> = Vec::new();
    let mut base_name = String::new();

    for (i, spec) in inputs.iter().enumerate() {
        let (path_str, value) = spec
            .rsplit_once(':')
            .with_context(|| format!("invalid input '{}' — expected format: file.ies:350", spec))?;

        let value: f64 = value
            .parse()
            .with_context(|| format!("cannot parse '{}' as a number in '{}'", value, spec))?;

        let path = PathBuf::from(path_str);
        let ldt = load_file(&path).with_context(|| format!("failed to load '{}'", path_str))?;

        if i == 0 {
            base_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("luminaire")
                .to_string();
            // Strip trailing operating-point suffix if present (e.g., "_350mA")
            if let Some(pos) = base_name.rfind('_') {
                let suffix = &base_name[pos + 1..];
                if suffix
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false)
                {
                    base_name.truncate(pos);
                }
            }
        }

        println!(
            "  Loaded {} ({:.0} lm, {:.0} W) @ {}{}",
            path_str,
            ldt.luminous_flux(),
            ldt.wattage(),
            eulumdat::interpolate::format_value(value),
            param_name,
        );

        parsed_inputs.push((ldt, value));
    }

    // Determine target values
    let targets: Vec<f64> = if let Some(val) = at {
        vec![val]
    } else if let Some(step_vals) = steps {
        step_vals.to_vec()
    } else if let Some(range_str) = range {
        let (start_str, end_str) = range_str.split_once(':').with_context(|| {
            format!("invalid --range '{}' — expected format: 350:700", range_str)
        })?;
        let start: f64 = start_str.parse().context("invalid range start")?;
        let end: f64 = end_str.parse().context("invalid range end")?;
        eulumdat::interpolate::linspace(start, end, count)
    } else {
        // Default: infer range from inputs, generate `count` steps
        let min = parsed_inputs
            .iter()
            .map(|(_, v)| *v)
            .fold(f64::INFINITY, f64::min);
        let max = parsed_inputs
            .iter()
            .map(|(_, v)| *v)
            .fold(f64::NEG_INFINITY, f64::max);
        eulumdat::interpolate::linspace(min, max, count)
    };

    if targets.is_empty() {
        anyhow::bail!("no target values specified — use --at, --steps, or --range");
    }

    println!("\n  Generating {} file(s)...\n", targets.len());

    // Generate interpolated series
    let series = eulumdat::interpolate::generate_series(&parsed_inputs, &targets)
        .context("interpolation failed")?;

    // Ensure output directory exists
    fs::create_dir_all(output_dir)
        .with_context(|| format!("cannot create output directory '{}'", output_dir.display()))?;

    let ext = match format {
        OutputFormat::Ies => "ies",
        OutputFormat::Ldt => "ldt",
    };

    let mut written = 0;
    for (value, ldt) in &series {
        let val_str = eulumdat::interpolate::format_value(*value);
        let filename = format!("{}_{}{}.{}", base_name, val_str, param_name, ext);
        let out_path = output_dir.join(&filename);

        if out_path.exists() && !overwrite {
            println!("  SKIP {} (exists, use --overwrite)", filename);
            continue;
        }

        let content = match format {
            OutputFormat::Ies => IesExporter::export(ldt),
            OutputFormat::Ldt => ldt.to_ldt(),
        };

        fs::write(&out_path, &content)
            .with_context(|| format!("failed to write '{}'", out_path.display()))?;

        println!(
            "  \u{2713} {} ({:.0} lm, {:.0} W)",
            filename,
            ldt.luminous_flux(),
            ldt.wattage(),
        );
        written += 1;
    }

    println!(
        "\n  Done: {} file(s) written to {}",
        written,
        output_dir.display()
    );
    Ok(())
}

// ─── Street lighting classification / compliance (feature = "street") ────────

/// Print luminaire classifications: IES Type, BUG, CCT, DLOR/ULOR, efficacy.
#[cfg(feature = "street")]
pub fn classify(
    file: &std::path::PathBuf,
    format: crate::cli::ClassifyFormat,
) -> anyhow::Result<()> {
    use eulumdat::bug_rating::{BugRating, ZoneLumens};
    use eulumdat::iesna_classification;

    let ldt = load_file(file)?;
    let zones = ZoneLumens::from_eulumdat(&ldt);
    let bug = BugRating::from_zone_lumens(&zones);
    let ies = iesna_classification::classify(&ldt);
    let summary = eulumdat::PhotometricSummary::from_eulumdat(&ldt);

    match format {
        crate::cli::ClassifyFormat::Text => {
            println!("File: {}", file.display());
            println!("Luminaire: {}", ldt.luminaire_name);
            println!();
            println!("=== IES Classification (RP-8 / TM-15) ===");
            println!("Lateral type:    {}", ies.lateral_type);
            println!("Longitudinal:    {}", ies.longitudinal);
            println!("Cutoff class:    {}", ies.cutoff);
            println!("BUG rating:      {bug}");
            println!();
            println!("=== Photometric ===");
            println!("DLOR:            {:.1}%", summary.dlor);
            println!("ULOR:            {:.1}%", summary.ulor);
            println!("Efficacy:        {:.1} lm/W", summary.luminaire_efficacy);
            println!("Max intensity:   {:.0} cd/klm", summary.max_intensity);
            println!("Beam angle:      {:.1}°", summary.beam_angle);
            println!();
            println!("=== Dark-Sky Zones (MLO) ===");
            for z in bug.compliant_zones() {
                println!("  {z}: ✓ compliant");
            }
        }
        crate::cli::ClassifyFormat::Json => {
            let report = serde_json::json!({
                "file": file.display().to_string(),
                "luminaire_name": ldt.luminaire_name,
                "ies": {
                    "lateral_type": ies.lateral_type.to_string(),
                    "longitudinal": ies.longitudinal.to_string(),
                    "cutoff": ies.cutoff.to_string(),
                },
                "bug": {
                    "b": bug.b,
                    "u": bug.u,
                    "g": bug.g,
                    "display": bug.to_string(),
                },
                "photometric": {
                    "dlor": summary.dlor,
                    "ulor": summary.ulor,
                    "efficacy_lm_per_w": summary.luminaire_efficacy,
                    "max_intensity_cd_klm": summary.max_intensity,
                    "beam_angle_deg": summary.beam_angle,
                },
                "mlo_compliant_zones": bug
                    .compliant_zones()
                    .iter()
                    .map(|z| z.to_string())
                    .collect::<Vec<_>>(),
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }

    Ok(())
}

/// Check a photometric file against a file-level compliance standard (MLO).
///
/// Design-level standards (RP-8, EN 13201, CJJ 45) require a layout and
/// belong in the WASM street-designer, not a single-file CLI invocation.
#[cfg(feature = "street")]
pub fn compliance(
    file: &std::path::PathBuf,
    standard: crate::cli::ComplianceStandard,
    zone: Option<&str>,
) -> anyhow::Result<()> {
    use eulumdat::bug_rating::LightingZone;
    use eulumdat::standards::{mlo::MloStandard, LightingStandard};

    let ldt = load_file(file)?;

    match standard {
        crate::cli::ComplianceStandard::Mlo => {
            let zone = zone.ok_or_else(|| {
                anyhow::anyhow!(
                    "--zone is required for MLO (e.g. --zone lz2). \
                     Valid values: lz0, lz1, lz2, lz3, lz4."
                )
            })?;
            let zone = match zone.to_ascii_lowercase().as_str() {
                "lz0" => LightingZone::LZ0,
                "lz1" => LightingZone::LZ1,
                "lz2" => LightingZone::LZ2,
                "lz3" => LightingZone::LZ3,
                "lz4" => LightingZone::LZ4,
                other => anyhow::bail!("unknown lighting zone: {other}"),
            };

            let result = MloStandard
                .check_file(&zone, &ldt)
                .ok_or_else(|| anyhow::anyhow!("MLO check returned no result — internal error"))?;

            println!("Standard: {}", result.standard);
            println!("Region:   {}", result.region);
            println!();
            println!(
                "{:<22} {:<15} {:<15} {}",
                "Criterion", "Required", "Achieved", "Status"
            );
            println!("{}", "-".repeat(60));
            for item in &result.items {
                let status = if item.passed { "✓" } else { "✗" };
                println!(
                    "{:<22} {:<15} {:<15} {status}",
                    item.parameter, item.required, item.achieved
                );
            }
            println!();
            if result.passed() {
                println!("Overall: ✓ COMPLIANT");
            } else {
                println!(
                    "Overall: ✗ NON-COMPLIANT ({} failures)",
                    result.failure_count()
                );
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

/// Arguments of `eulumdat digitize`.
pub struct DigitizeArgs {
    pub image: PathBuf,
    pub output: Option<PathBuf>,
    pub x_range: Option<String>,
    pub x_ticks: Option<String>,
    pub x_calibration: Option<String>,
    pub mode: CurveStyle,
    pub step: f64,
    pub label: Option<String>,
    pub source: Option<String>,
    pub debug: Option<PathBuf>,
    pub attach: Option<PathBuf>,
    pub emitter: Option<usize>,
}

/// Digitize a spectrum chart image into a relative `.spd` file.
pub fn digitize(args: &DigitizeArgs) -> Result<()> {
    use eulumdat_digitize::{CurveMode, ImageRef, Options};

    let img = image::open(&args.image)
        .with_context(|| format!("Failed to open image {}", args.image.display()))?
        .to_rgb8();
    let (w, h) = (img.width() as usize, img.height() as usize);

    let mut opts = Options {
        step_nm: args.step,
        mode: match args.mode {
            CurveStyle::Auto => CurveMode::Auto,
            CurveStyle::Fill => CurveMode::Fill,
            CurveStyle::Line => CurveMode::Line,
        },
        ..Options::default()
    };
    if let Some(r) = &args.x_range {
        opts.x_range = Some(Options::parse_range(r)?);
    }
    if let Some(t) = &args.x_ticks {
        opts.x_ticks = Some(Options::parse_ticks(t)?);
    }
    if let Some(c) = &args.x_calibration {
        opts.x_calibration = Some(Options::parse_calibration(c)?);
    }
    if opts.x_range.is_none() && opts.x_ticks.is_none() && opts.x_calibration.is_none() {
        anyhow::bail!(
            "the x-axis scale is required: --x-range FIRST:LAST (e.g. 380:780) or --x-ticks"
        );
    }

    let d = eulumdat_digitize::digitize(ImageRef::new(w, h, 3, img.as_raw())?, &opts)?;

    let c = &d.calibration;
    eprintln!(
        "image {w}×{h}: background #{}, axis #{}, curve #{}, mode {:?}",
        hex(c.background),
        hex(c.axis),
        hex(c.fill),
        c.mode
    );
    eprintln!(
        "x-axis rows {}–{}, y-axis cols {}–{}, {} calibration points ({}), max residual {:.3} nm, {:.3} nm/px",
        c.x_axis_rows.0,
        c.x_axis_rows.1,
        c.y_axis_cols.0,
        c.y_axis_cols.1,
        c.x_points.len(),
        if c.x_from_labels { "label positions" } else { "tick marks" },
        c.x_residual,
        c.nm_per_px
    );
    for wmsg in &d.warnings {
        eprintln!("warning: {wmsg}");
    }
    for p in &d.peaks {
        eprintln!(
            "peak {:6.1} nm  rel {:5.3}  FWHM ≈ {:.0} nm",
            p.wavelength_nm, p.relative, p.fwhm_nm
        );
    }

    let stem = args
        .image
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "spectrum".into());
    let label = args.label.clone().unwrap_or_else(|| stem.clone());
    let image_name = args
        .image
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let spd_text = d.to_spd(&label, &image_name, args.source.as_deref());

    if let Some(lum_path) = &args.attach {
        // Attach to a luminaire and write ATLA; the .spd itself goes next to the image.
        let spd_out = args.image.with_extension("spd");
        fs::write(&spd_out, &spd_text)
            .with_context(|| format!("Failed to write {}", spd_out.display()))?;
        eprintln!("wrote {}", spd_out.display());
        let spd = eulumdat::atla::spd_loader::parse(&spd_text)
            .map_err(|e| anyhow::anyhow!("{e}"))?
            .spd;
        let mut doc = load_luminaire_doc(lum_path, 0.0)?;
        let n = eulumdat::atla::spd_loader::attach_spd(&mut doc, &spd, args.emitter);
        let out = args
            .output
            .clone()
            .unwrap_or_else(|| lum_path.with_extension("xml"));
        write_atla(&doc, &out, false)?;
        eprintln!(
            "attached spectrum to {} emitter(s) of {} → {}",
            n,
            lum_path.display(),
            out.display()
        );
    } else {
        let out = args
            .output
            .clone()
            .unwrap_or_else(|| args.image.with_extension("spd"));
        fs::write(&out, &spd_text).with_context(|| format!("Failed to write {}", out.display()))?;
        eprintln!(
            "wrote {} ({} rows, {}–{} nm @ {} nm)",
            out.display(),
            d.wavelengths.len(),
            d.wavelengths.first().copied().unwrap_or(0.0),
            d.wavelengths.last().copied().unwrap_or(0.0),
            args.step
        );
    }

    if let Some(dbg_path) = &args.debug {
        let mut dbg = img.clone();
        let magenta = image::Rgb([255, 0, 255]);
        let green = image::Rgb([0, 200, 0]);
        let yellow = image::Rgb([255, 220, 0]);
        for y in c.x_axis_rows.0..=c.x_axis_rows.1 {
            for x in 0..w {
                dbg.put_pixel(x as u32, y as u32, magenta);
            }
        }
        for x in c.y_axis_cols.0..=c.y_axis_cols.1 {
            for y in 0..h {
                dbg.put_pixel(x as u32, y as u32, magenta);
            }
        }
        for (px, _) in &c.x_points {
            let x = px.round() as usize;
            for dx in 0..2usize {
                let xx = (x + dx).min(w - 1);
                for y in c.x_axis_rows.0.saturating_sub(15)..(c.x_axis_rows.1 + 15).min(h) {
                    dbg.put_pixel(xx as u32, y as u32, green);
                }
            }
        }
        let zero = c.x_axis_rows.0 as f64;
        for (col, hpx) in &d.trace {
            if *hpx > 0.0 {
                let y = (zero - hpx).round().clamp(0.0, h as f64 - 1.0) as u32;
                for dy in 0..2u32 {
                    dbg.put_pixel(*col as u32, y.saturating_sub(dy), yellow);
                }
            }
        }
        dbg.save(dbg_path)
            .with_context(|| format!("Failed to write debug overlay {}", dbg_path.display()))?;
        eprintln!("wrote debug overlay {}", dbg_path.display());
    }
    Ok(())
}

fn hex(c: [u8; 3]) -> String {
    format!("{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

/// Inspect a TM-25 ray file; optionally convert its far field (+ spectrum)
/// to ATLA or EULUMDAT.
pub fn tm25(
    file: &Path,
    output: Option<&Path>,
    c_step: f64,
    g_step: f64,
    smooth: usize,
) -> Result<()> {
    use eulumdat_tm25::{FarFieldBuilder, Tm25Reader};
    let f = fs::File::open(file).with_context(|| format!("Failed to open {}", file.display()))?;
    let total_size = f.metadata().map(|m| m.len()).unwrap_or(0);
    let mut reader = Tm25Reader::new(std::io::BufReader::with_capacity(1 << 20, f))
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let h = reader.header().clone();
    eulumdat_tm25::check_file_size(&h, total_size).map_err(|e| anyhow::anyhow!("{e}"))?;
    let kind = h.flux_kind();
    println!("{}", file.display());
    println!(
        "  {} rays, {:?}, radiant {} W, luminous {} lm, created {}",
        h.n_rays, kind, h.radiant_flux_w, h.luminous_flux_lm, h.date_time
    );
    println!(
        "  {} / {} / {} / {}",
        h.text.name(),
        h.text.manufacturer(),
        h.text.operating_condition(),
        h.text.data_reference()
    );
    println!("  spectral id {:?}, columns {:?}", h.spectral_id, h.columns);
    let mut ff = FarFieldBuilder::new(c_step, g_step, kind);
    let mut n = 0u64;
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    loop {
        let chunk = reader
            .read_chunk(1 << 16)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        if chunk.is_empty() {
            break;
        }
        n += chunk.len() as u64;
        for r in &chunk {
            ff.add(r);
            let p = r.position();
            for i in 0..3 {
                lo[i] = lo[i].min(p[i]);
                hi[i] = hi[i].max(p[i]);
            }
        }
    }
    println!(
        "  start points: x {:.2}..{:.2}, y {:.2}..{:.2}, z {:.2}..{:.2} mm",
        lo[0], hi[0], lo[1], hi[1], lo[2], hi[2]
    );
    let ff = ff.finish();
    let ff = if smooth > 0 {
        ff.smoothed(smooth, smooth, 2.0 * g_step)
    } else {
        ff
    };
    let prof = ff.azimuthal_average();
    let max = prof.iter().cloned().fold(0.0, f64::max);
    println!(
        "  far field from {n} rays: peak {:.4} {:?}, half intensity at {:?}°, forward fraction {:.3}",
        max,
        ff.unit,
        ff.half_intensity_gamma().map(|g| g.round()),
        ff.forward_fraction()
    );
    if let Some(t) = h.shared_spectrum() {
        let spd = eulumdat_spectrum::Spd::new(
            &t.wavelengths_nm
                .iter()
                .map(|w| *w as f64)
                .collect::<Vec<_>>(),
            &t.values.iter().map(|v| *v as f64).collect::<Vec<_>>(),
        );
        let m = eulumdat_spectrum::SpectralMetrics::from_spd(&spd);
        println!(
            "  spectrum: {} points, peak {:?}, LER {:.1} lm/W, PPFD {:.2} µmol/J{}",
            t.len(),
            t.peak(),
            m.ler_lm_per_w,
            m.ppfd_per_watt,
            if t.is_ultraviolet() {
                ", ultraviolet"
            } else {
                ""
            }
        );
        if m.ppfd_per_watt > 0.0 && h.radiant_flux_w > 0.0 {
            println!(
                "  PPF {:.2} µmol/s",
                h.radiant_flux_w as f64 * m.ppfd_per_watt
            );
        }
    }
    let Some(out) = output else { return Ok(()) };
    let ldt = eulumdat_tm25::luminaire_from_far_field(&ff, &h, Some((lo, hi)));
    let ext = out
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "ldt" => {
            fs::write(out, ldt.to_ldt())
                .with_context(|| format!("Failed to write {}", out.display()))?;
        }
        "xml" | "json" => {
            let doc = eulumdat_tm25::atla_from_luminaire(&ldt, &h);
            write_atla(&doc, out, false)?;
        }
        _ => anyhow::bail!("output must be .ldt, .xml or .json"),
    }
    println!("wrote {}", out.display());
    Ok(())
}
