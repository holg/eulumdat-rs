//! EULUMDAT CLI - Command-line tool for photometric files.
//!
//! Supports EULUMDAT (.ldt) and IES (.ies) photometric file formats.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use eulumdat::{diagram::SvgTheme, BugDiagram, Eulumdat, IesExporter, IesParser};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "eulumdat")]
#[command(author, version, about = "EULUMDAT/IES photometric file tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Display information about a photometric file
    Info {
        /// Input file (.ldt or .ies)
        file: PathBuf,

        /// Show detailed intensity data
        #[arg(short, long)]
        verbose: bool,
    },

    /// Validate a photometric file
    Validate {
        /// Input file (.ldt or .ies)
        file: PathBuf,

        /// Use strict validation (fail on warnings)
        #[arg(short, long)]
        strict: bool,
    },

    /// Convert between LDT and IES formats
    Convert {
        /// Input file (.ldt or .ies)
        input: PathBuf,

        /// Output file (.ldt or .ies)
        output: PathBuf,
    },

    /// Generate SVG diagram
    Diagram {
        /// Input file (.ldt or .ies)
        input: PathBuf,

        /// Output SVG file
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Diagram type
        #[arg(short = 't', long, value_enum, default_value = "polar")]
        diagram_type: DiagramType,

        /// Use dark theme
        #[arg(short, long)]
        dark: bool,

        /// Width in pixels
        #[arg(short = 'W', long, default_value = "500")]
        width: f64,

        /// Height in pixels
        #[arg(short = 'H', long, default_value = "500")]
        height: f64,
    },

    /// Calculate BUG rating (outdoor luminaires)
    Bug {
        /// Input file (.ldt or .ies)
        file: PathBuf,

        /// Generate BUG diagram SVG
        #[arg(short, long)]
        svg: Option<PathBuf>,

        /// Use dark theme for SVG
        #[arg(short, long)]
        dark: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
enum DiagramType {
    /// Polar intensity diagram (C0-C180, C90-C270)
    Polar,
    /// Butterfly diagram (3D isometric)
    Butterfly,
    /// Cartesian diagram (intensity vs gamma)
    Cartesian,
    /// Heatmap diagram (2D intensity grid)
    Heatmap,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Info { file, verbose } => cmd_info(&file, verbose),
        Commands::Validate { file, strict } => cmd_validate(&file, strict),
        Commands::Convert { input, output } => cmd_convert(&input, &output),
        Commands::Diagram {
            input,
            output,
            diagram_type,
            dark,
            width,
            height,
        } => cmd_diagram(&input, output.as_ref(), diagram_type, dark, width, height),
        Commands::Bug { file, svg, dark } => cmd_bug(&file, svg.as_ref(), dark),
    }
}

fn load_file(path: &PathBuf) -> Result<Eulumdat> {
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

fn cmd_info(file: &PathBuf, verbose: bool) -> Result<()> {
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

fn cmd_validate(file: &PathBuf, strict: bool) -> Result<()> {
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

fn cmd_convert(input: &PathBuf, output: &PathBuf) -> Result<()> {
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

fn cmd_diagram(
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

fn cmd_bug(file: &PathBuf, svg: Option<&PathBuf>, dark: bool) -> Result<()> {
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
