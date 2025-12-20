//! CLI argument parsing and type definitions

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "eulumdat")]
#[command(author, version, about = "EULUMDAT/IES photometric file tool", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
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

    /// Convert between photometric formats (LDT, IES, ATLA XML/JSON)
    Convert {
        /// Input file (.ldt, .ies, .xml, or .json)
        input: PathBuf,

        /// Output file (.ldt, .ies, .xml, or .json)
        output: PathBuf,

        /// Output compact format (no indentation) for XML/JSON
        #[arg(short, long)]
        compact: bool,
    },

    /// Generate SVG diagram
    Diagram {
        /// Input file (.ldt, .ies, .xml, or .json for ATLA)
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

        /// Mounting height in meters (for cone/greenhouse diagrams)
        #[arg(short = 'm', long, default_value = "3.0")]
        mounting_height: f64,
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

    /// Batch convert multiple files
    Batch {
        /// Input directory containing .ldt or .ies files
        input_dir: PathBuf,

        /// Output directory (defaults to input directory)
        #[arg(short, long)]
        output_dir: Option<PathBuf>,

        /// Output format
        #[arg(short = 'f', long, value_enum)]
        format: OutputFormat,

        /// Process subdirectories recursively
        #[arg(short, long)]
        recursive: bool,

        /// Overwrite existing files
        #[arg(long)]
        overwrite: bool,
    },

    /// Display photometric summary with calculated values
    Summary {
        /// Input file (.ldt or .ies)
        file: PathBuf,

        /// Output format
        #[arg(short = 'f', long, value_enum, default_value = "text")]
        format: SummaryFormat,

        /// Output to file instead of stdout
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Export GLDF-compatible photometric data
    Gldf {
        /// Input file (.ldt or .ies)
        file: PathBuf,

        /// Output JSON file (stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Pretty-print JSON output
        #[arg(short, long)]
        pretty: bool,
    },

    /// Calculate specific photometric values
    Calc {
        /// Input file (.ldt or .ies)
        file: PathBuf,

        /// Calculation type
        #[arg(short = 't', long, value_enum)]
        calc_type: CalcType,
    },

    /// Validate ATLA XML file against XSD schema
    ValidateAtla {
        /// Input ATLA XML file (.xml)
        file: PathBuf,

        /// Custom XSD schema file (uses embedded schema if not specified)
        #[arg(short, long)]
        schema: Option<PathBuf>,

        /// Schema version to validate against
        #[arg(long, value_enum, default_value = "auto")]
        schema_type: AtlaSchemaType,

        /// Use xmllint for full XSD validation (requires libxml2)
        #[arg(long)]
        xsd: bool,
    },

    /// Convert ATLA between schema versions (S001 <-> TM-33-23)
    AtlaConvert {
        /// Input ATLA file (.xml or .json)
        input: PathBuf,

        /// Output file (.xml or .json)
        output: PathBuf,

        /// Target schema version
        #[arg(short, long, value_enum)]
        target: AtlaSchemaType,

        /// Conversion policy
        #[arg(long, value_enum, default_value = "compatible")]
        policy: ConversionPolicyArg,

        /// Show conversion log (field-by-field changes)
        #[arg(short, long)]
        verbose: bool,

        /// Output compact format (no indentation)
        #[arg(short, long)]
        compact: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum DiagramType {
    /// Polar intensity diagram (C0-C180, C90-C270)
    Polar,
    /// Butterfly diagram (3D isometric)
    Butterfly,
    /// Cartesian diagram (intensity vs gamma)
    Cartesian,
    /// Heatmap diagram (2D intensity grid)
    Heatmap,
    /// Cone diagram (beam/field angle spread at mounting height)
    Cone,
    /// Beam angle diagram (IES vs CIE comparison)
    BeamAngle,
    /// LCS classification diagram (IES TM-15-07)
    Lcs,
    /// Spectral power distribution (requires ATLA input with spectral data)
    Spectral,
    /// Greenhouse PPFD diagram (horticultural lighting)
    Greenhouse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    /// Convert to LDT format (common in Europe)
    Ldt,
    /// Convert to IES format (common in North America)
    Ies,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum SummaryFormat {
    /// Human-readable text report
    Text,
    /// Single-line compact summary
    Compact,
    /// JSON output
    Json,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum CalcType {
    /// CIE flux codes (N1-N5)
    CieCodes,
    /// Beam angle (50%) and field angle (10%)
    BeamAngles,
    /// Spacing criteria (S/H ratios)
    Spacing,
    /// Zonal lumens in 30° zones
    ZonalLumens,
    /// All calculations
    All,
}

/// ATLA schema version for validation
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum AtlaSchemaType {
    /// Auto-detect from document content
    #[default]
    Auto,
    /// ATLA S001 / TM-33-18 / UNI 11733
    S001,
    /// TM-33-23 (IESTM33-22 v1.1)
    Tm3323,
}

/// Conversion policy for ATLA schema conversion
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum ConversionPolicyArg {
    /// Error on missing required fields
    Strict,
    /// Apply defaults for missing fields where possible
    #[default]
    Compatible,
}
