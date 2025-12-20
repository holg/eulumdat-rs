//! Command implementations for the CLI

use anyhow::{Context, Result};
use atla::LuminaireOpticalData;
use eulumdat::{
    batch::{self, BatchInput, ConversionFormat},
    diagram::SvgTheme,
    BugDiagram, Eulumdat, GldfPhotometricData, IesExporter, IesParser, PhotometricCalculations,
    PhotometricSummary,
};
use std::path::PathBuf;

use crate::cli::{
    AtlaSchemaType, CalcType, ConversionPolicyArg, DiagramType, OutputFormat, SummaryFormat,
};

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
            let atla_doc = atla::parse_file(path).context("Failed to parse ATLA file")?;
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
        "xml" | "json" => atla::parse_file(path).context("Failed to parse ATLA file"),
        "ldt" => {
            let ldt = Eulumdat::from_file(path).context("Failed to parse LDT file")?;
            Ok(LuminaireOpticalData::from_eulumdat(&ldt))
        }
        "ies" => {
            let ldt = IesParser::parse_file(path).context("Failed to parse IES file")?;
            Ok(LuminaireOpticalData::from_eulumdat(&ldt))
        }
        _ => anyhow::bail!("Unknown file extension: .{ext} (expected .ldt, .ies, .xml, or .json)"),
    }
}

pub fn info(file: &PathBuf, verbose: bool) -> Result<()> {
    let ldt = load_file(file)?;

    println!("File: {}", file.display());
    println!();
    println!("=== Luminaire Information ===");
    println!("Name:           {}", ldt.luminaire_name);
    println!("Number:         {}", ldt.luminaire_number);
    println!("Manufacturer:   {}", ldt.identification);
    println!("Date:           {}", ldt.date_user);
    println!();
    println!("=== Dimensions (mm) ===");
    println!("Length:         {:.1}", ldt.length);
    println!("Width:          {:.1}", ldt.width);
    println!("Height:         {:.1}", ldt.height);
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
    println!("Total flux:     {:.0} lm", ldt.total_luminous_flux());
    println!("Total wattage:  {:.1} W", ldt.total_wattage());
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

pub fn convert(input: &PathBuf, output: &PathBuf, compact: bool) -> Result<()> {
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

    // Load the source data
    let content = match (in_ext.as_str(), out_ext.as_str()) {
        // ATLA input -> ATLA output (direct conversion)
        ("xml" | "json", "xml") => {
            let atla_doc = atla::parse_file(input).context("Failed to parse ATLA file")?;
            if compact {
                atla::xml::write_compact(&atla_doc).context("Failed to write ATLA XML")?
            } else {
                atla::xml::write(&atla_doc).context("Failed to write ATLA XML")?
            }
        }
        ("xml" | "json", "json") => {
            let atla_doc = atla::parse_file(input).context("Failed to parse ATLA file")?;
            if compact {
                atla::json::write_compact(&atla_doc).context("Failed to write ATLA JSON")?
            } else {
                atla::json::write(&atla_doc).context("Failed to write ATLA JSON")?
            }
        }
        // LDT/IES input -> ATLA output
        ("ldt" | "ies", "xml") => {
            let ldt = load_file(input)?;
            let atla_doc = atla::LuminaireOpticalData::from_eulumdat(&ldt);
            if compact {
                atla::xml::write_compact(&atla_doc).context("Failed to write ATLA XML")?
            } else {
                atla::xml::write(&atla_doc).context("Failed to write ATLA XML")?
            }
        }
        ("ldt" | "ies", "json") => {
            let ldt = load_file(input)?;
            let atla_doc = atla::LuminaireOpticalData::from_eulumdat(&ldt);
            if compact {
                atla::json::write_compact(&atla_doc).context("Failed to write ATLA JSON")?
            } else {
                atla::json::write(&atla_doc).context("Failed to write ATLA JSON")?
            }
        }
        // Any input -> LDT/IES output (via Eulumdat)
        (_, "ldt") => {
            let ldt = load_file(input)?;
            ldt.to_ldt()
        }
        (_, "ies") => {
            let ldt = load_file(input)?;
            IesExporter::export(&ldt)
        }
        _ => anyhow::bail!(
            "Unknown output extension: .{out_ext} (expected .ldt, .ies, .xml, or .json)"
        ),
    };

    std::fs::write(output, &content).context("Failed to write output file")?;

    let in_ext_upper = in_ext.to_uppercase();
    let out_ext_upper = out_ext.to_uppercase();
    let format_note = if compact && (out_ext == "xml" || out_ext == "json") {
        " [compact]"
    } else {
        ""
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

pub fn diagram(
    input: &PathBuf,
    output: Option<&PathBuf>,
    diagram_type: DiagramType,
    dark: bool,
    width: f64,
    height: f64,
    mounting_height: f64,
) -> Result<()> {
    use eulumdat::diagram::*;

    let theme = if dark {
        SvgTheme::dark()
    } else {
        SvgTheme::light()
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
            diagram.to_svg(width, height, &theme)
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
                atla::spectral::SpectralTheme::dark()
            } else {
                atla::spectral::SpectralTheme::light()
            };

            // Try to get spectral data from emitters
            if let Some(spd) = atla_doc
                .emitters
                .iter()
                .filter_map(|e| e.spectral_distribution.as_ref())
                .next()
            {
                let diagram = atla::spectral::SpectralDiagram::from_spectral(spd);
                diagram.to_svg(width, height, &atla_theme)
            } else if let Some(emitter) = atla_doc.emitters.first() {
                // Try to synthesize from CCT/CRI
                if let Some(cct) = emitter.cct {
                    let cri = emitter.color_rendering.as_ref().and_then(|cr| cr.ra);
                    let spd = atla::spectral::synthesize_spectrum(cct, cri);
                    let diagram = atla::spectral::SpectralDiagram::from_spectral(&spd);
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
                atla::greenhouse::GreenhouseTheme::dark()
            } else {
                atla::greenhouse::GreenhouseTheme::light()
            };
            let diagram = atla::greenhouse::GreenhouseDiagram::from_atla_with_height(
                &atla_doc,
                mounting_height,
            );
            diagram.to_svg(width, height, &gh_theme)
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
            let beam = PhotometricCalculations::beam_angle(&ldt);
            let field = PhotometricCalculations::field_angle(&ldt);
            let beam_c0 = PhotometricCalculations::beam_angle_for_plane(&ldt, 0.0);
            let beam_c90 = PhotometricCalculations::beam_angle_for_plane(&ldt, 90.0);
            let field_c0 = PhotometricCalculations::field_angle_for_plane(&ldt, 0.0);
            let field_c90 = PhotometricCalculations::field_angle_for_plane(&ldt, 90.0);
            let cut_off = PhotometricCalculations::cut_off_angle(&ldt);

            println!("=== Beam Characteristics ===");
            println!("Beam Angle (50%):      {:.1}°", beam);
            println!("Field Angle (10%):     {:.1}°", field);
            println!("Cut-off Angle (2.5%):  {:.1}°", cut_off);
            println!();
            println!("=== Per-Plane Angles ===");
            println!("Beam C0 / C90:         {:.1}° / {:.1}°", beam_c0, beam_c90);
            println!(
                "Field C0 / C90:        {:.1}° / {:.1}°",
                field_c0, field_c90
            );
        }
        CalcType::Spacing => {
            let (s_c0, s_c90) = PhotometricCalculations::spacing_criteria(&ldt);
            let code = PhotometricCalculations::photometric_code(&ldt);

            println!("=== Spacing Criteria ===");
            println!("S/H ratio (C0):        {:.2}", s_c0);
            println!("S/H ratio (C90):       {:.2}", s_c90);
            println!();
            println!("Photometric Code:      {}", code);
            println!();
            println!("Note: S/H ratio indicates maximum spacing-to-height");
            println!("      ratio for reasonably uniform illumination.");
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
    use atla::validate::{self, ValidationSchema};

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
    let doc = atla::parse(&content).context("Failed to parse ATLA file")?;

    // Display detected schema version
    let detected_version = match doc.schema_version {
        atla::SchemaVersion::AtlaS001 => "ATLA S001 / TM-33-18",
        atla::SchemaVersion::Tm3323 => "TM-33-23 (IESTM33-22)",
        atla::SchemaVersion::Tm3324 => "TM-33-24",
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
            atla::SchemaVersion::AtlaS001 => s001_valid,
            atla::SchemaVersion::Tm3323 | atla::SchemaVersion::Tm3324 => tm33_valid,
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
fn print_validation_result(result: &atla::validate::ValidationResult, schema_name: &str) {
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
    use atla::convert::{atla_to_tm33, tm33_to_atla, ConversionPolicy};

    // Parse input file
    let content = std::fs::read_to_string(input).context("Failed to read input file")?;
    let doc = atla::parse(&content).context("Failed to parse ATLA file")?;

    // Get source and target schema names
    let source_name = match doc.schema_version {
        atla::SchemaVersion::AtlaS001 => "ATLA S001",
        atla::SchemaVersion::Tm3323 => "TM-33-23",
        atla::SchemaVersion::Tm3324 => "TM-33-24",
    };

    let target_schema = match target {
        AtlaSchemaType::Auto => {
            // Auto: convert to the "other" format
            match doc.schema_version {
                atla::SchemaVersion::AtlaS001 => atla::SchemaVersion::Tm3323,
                _ => atla::SchemaVersion::AtlaS001,
            }
        }
        AtlaSchemaType::S001 => atla::SchemaVersion::AtlaS001,
        AtlaSchemaType::Tm3323 => atla::SchemaVersion::Tm3323,
    };

    let target_name = match target_schema {
        atla::SchemaVersion::AtlaS001 => "ATLA S001",
        atla::SchemaVersion::Tm3323 => "TM-33-23",
        atla::SchemaVersion::Tm3324 => "TM-33-24",
    };

    println!("Converting {} → {}", source_name, target_name);
    println!("  Input: {}", input.display());
    println!("  Output: {}", output.display());
    println!();

    // Perform conversion
    let (converted_doc, log) = match (doc.schema_version, target_schema) {
        (
            atla::SchemaVersion::AtlaS001,
            atla::SchemaVersion::Tm3323 | atla::SchemaVersion::Tm3324,
        ) => {
            let conversion_policy = match policy {
                ConversionPolicyArg::Strict => ConversionPolicy::Strict,
                ConversionPolicyArg::Compatible => ConversionPolicy::Compatible,
            };
            atla_to_tm33(&doc, conversion_policy)?
        }
        (
            atla::SchemaVersion::Tm3323 | atla::SchemaVersion::Tm3324,
            atla::SchemaVersion::AtlaS001,
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
                atla::convert::ConversionAction::Preserved => "Preserved",
                atla::convert::ConversionAction::DefaultApplied => "Default applied",
                atla::convert::ConversionAction::Renamed => "Renamed",
                atla::convert::ConversionAction::TypeConverted => "Type converted",
                atla::convert::ConversionAction::Dropped => "Dropped",
                atla::convert::ConversionAction::Warning => "Warning",
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
                atla::json::write_compact(&converted_doc)?
            } else {
                atla::json::write(&converted_doc)?
            }
        }
        _ => {
            // XML output - use target schema format
            atla::xml::write_with_schema(&converted_doc, target_schema, indent)?
        }
    };

    std::fs::write(output, &output_content).context("Failed to write output file")?;

    // Summary
    let defaults_count = log
        .iter()
        .filter(|e| matches!(e.action, atla::convert::ConversionAction::DefaultApplied))
        .count();
    let dropped_count = log
        .iter()
        .filter(|e| matches!(e.action, atla::convert::ConversionAction::Dropped))
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
