//! Command implementations for the CLI

use anyhow::{Context, Result};
use eulumdat::{
    batch::{self, BatchInput, ConversionFormat},
    diagram::SvgTheme,
    BugDiagram, Eulumdat, IesExporter, IesParser,
};
use std::path::PathBuf;

use crate::cli::{DiagramType, OutputFormat};

pub fn load_file(path: &PathBuf) -> Result<Eulumdat> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "ldt" => Eulumdat::from_file(path).context("Failed to parse LDT file"),
        "ies" => IesParser::parse_file(path).context("Failed to parse IES file"),
        _ => anyhow::bail!("Unknown file extension: .{ext} (expected .ldt or .ies)"),
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

pub fn convert(input: &PathBuf, output: &PathBuf) -> Result<()> {
    let ldt = load_file(input)?;

    let out_ext = output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let content = match out_ext.as_str() {
        "ldt" => ldt.to_ldt(),
        "ies" => IesExporter::export(&ldt),
        _ => anyhow::bail!("Unknown output extension: .{out_ext} (expected .ldt or .ies)"),
    };

    std::fs::write(output, &content).context("Failed to write output file")?;

    let in_ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_uppercase();
    let out_ext_upper = out_ext.to_uppercase();

    println!(
        "Converted {} → {} ({} → {})",
        input.display(),
        output.display(),
        in_ext,
        out_ext_upper
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
) -> Result<()> {
    use eulumdat::diagram::*;

    let ldt = load_file(input)?;
    let theme = if dark {
        SvgTheme::dark()
    } else {
        SvgTheme::light()
    };

    let svg = match diagram_type {
        DiagramType::Polar => {
            let diagram = PolarDiagram::from_eulumdat(&ldt);
            diagram.to_svg(width, height, &theme)
        }
        DiagramType::Butterfly => {
            let diagram = ButterflyDiagram::from_eulumdat(&ldt, width, height, 60.0);
            diagram.to_svg(width, height, &theme)
        }
        DiagramType::Cartesian => {
            let diagram = CartesianDiagram::from_eulumdat(&ldt, width, height, 8);
            diagram.to_svg(width, height, &theme)
        }
        DiagramType::Heatmap => {
            let diagram = HeatmapDiagram::from_eulumdat(&ldt, width, height);
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
