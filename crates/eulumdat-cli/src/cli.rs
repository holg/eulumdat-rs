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

        /// Unit system for dimensions (mm/in)
        #[arg(short = 'U', long, value_enum, default_value = "metric")]
        units: UnitArg,
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

        /// Rotate C-planes by N degrees during conversion.
        /// Use 90 when converting IES→LDT to fix EU/US axis orientation.
        /// Use -90 when converting LDT→IES.
        #[arg(short, long, default_value = "0.0")]
        rotate: f64,
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

        /// Mounting height in meters (for cone/greenhouse/isolux diagrams)
        #[arg(short = 'm', long, default_value = "3.0")]
        mounting_height: f64,

        /// Tilt angle in degrees (for isolux diagram, 0=down, 90=horizontal)
        #[arg(long, default_value = "0.0")]
        tilt: f64,

        /// Use logarithmic Y-axis (for floodlight-vh diagram)
        #[arg(long)]
        log_scale: bool,

        /// Unit system for isolux/cone labels (lx/fc, m/ft)
        #[arg(short = 'U', long, value_enum, default_value = "metric")]
        units: UnitArg,
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

    /// Compare two photometric files side-by-side
    Compare {
        /// First input file (.ldt, .ies, .xml, or .json)
        file_a: PathBuf,

        /// Second input file (.ldt, .ies, .xml, or .json)
        file_b: PathBuf,

        /// Output format for comparison table
        #[arg(short = 'f', long, value_enum, default_value = "text")]
        format: CompareFormat,

        /// Generate overlay diagram SVG
        #[arg(short = 'd', long, value_enum)]
        diagram: Option<CompareDiagramType>,

        /// Output file for SVG diagram (stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Use dark theme for diagram
        #[arg(long)]
        dark: bool,

        /// Show only significant differences (>= 5% delta)
        #[arg(long)]
        significant_only: bool,

        /// Unit system for dimension metrics (mm/in)
        #[arg(short = 'U', long, value_enum, default_value = "metric")]
        units: UnitArg,
    },

    /// Generate photometric report (Typst source or PDF)
    Report {
        /// Input file (.ldt, .ies, .xml, or .json)
        input: PathBuf,

        /// Output file (.typ for Typst source, .pdf for PDF)
        output: PathBuf,

        /// Paper size
        #[arg(short, long, value_enum, default_value = "a4")]
        paper: PaperSize,

        /// Use compact report (fewer sections)
        #[arg(short, long)]
        compact: bool,

        /// Include CU (Coefficient of Utilization) table
        #[arg(long)]
        cu_table: bool,

        /// Include UGR (Unified Glare Rating) table
        #[arg(long)]
        ugr_table: bool,

        /// Include full candela tabulation (like Photometric Toolbox)
        #[arg(long)]
        candela_table: bool,
    },

    /// Interpolate between photometric files at different LED operating points
    ///
    /// Takes 2+ photometric files measured at known operating values (e.g.,
    /// driver currents) and generates new files at intermediate points.
    ///
    /// Input format: file.ies:350  (filepath:operating_value)
    ///
    /// Examples:
    ///   eulumdat interpolate fixture_350mA.ies:350 fixture_700mA.ies:700 --at 500
    ///   eulumdat interpolate lo.ldt:350 hi.ldt:700 --range 350:700 --count 8
    ///   eulumdat interpolate a.ies:350 b.ies:500 c.ies:700 --steps 400,450,550,650
    Interpolate {
        /// Input files with operating point values (format: file.ies:350)
        #[arg(required = true, num_args = 2..)]
        inputs: Vec<String>,

        /// Specific operating point values to generate (comma-separated)
        #[arg(long, value_delimiter = ',')]
        steps: Option<Vec<f64>>,

        /// Generate evenly spaced points in range (format: start:end)
        #[arg(long)]
        range: Option<String>,

        /// Number of evenly spaced points (used with --range)
        #[arg(long, default_value = "8")]
        count: usize,

        /// Generate a single interpolated file at this value
        #[arg(long)]
        at: Option<f64>,

        /// Output format
        #[arg(short = 'f', long, value_enum, default_value = "ies")]
        format: OutputFormat,

        /// Output directory
        #[arg(short, long, default_value = ".")]
        output_dir: PathBuf,

        /// Parameter name for output filenames (e.g., "mA", "K", "W", "pct")
        #[arg(long, default_value = "mA")]
        param_name: String,

        /// Allow overwriting existing files
        #[arg(long)]
        overwrite: bool,
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
    /// Floodlight V-H Cartesian diagram (Type B coordinates)
    FloodlightVh,
    /// Isolux ground footprint (illuminance contours)
    Isolux,
    /// Isocandela contour plot (equal-intensity lines)
    Isocandela,
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
    /// Coefficient of Utilization (CU) table
    CuTable,
    /// Unified Glare Rating (UGR) table
    UgrTable,
    /// Candela tabulation
    CandelaTable,
    /// NEMA floodlight beam classification
    Nema,
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

/// Paper size for report generation
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum PaperSize {
    /// A4 (210 × 297 mm)
    #[default]
    A4,
    /// US Letter (8.5 × 11 in)
    Letter,
    /// A3 (297 × 420 mm)
    A3,
}

/// Output format for the compare command
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum CompareFormat {
    /// Human-readable text table
    #[default]
    Text,
    /// JSON output
    Json,
    /// CSV output
    Csv,
}

/// Diagram type for compare overlay
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum CompareDiagramType {
    /// Polar overlay diagram
    Polar,
    /// Cartesian overlay diagram
    Cartesian,
}

/// Unit system for output display
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum UnitArg {
    /// SI / Metric (lux, meters, millimeters)
    #[default]
    Metric,
    /// Imperial (foot-candles, feet, inches)
    Imperial,
}

impl UnitArg {
    /// Convert to the core library's `UnitSystem`.
    pub fn to_unit_system(self) -> eulumdat::UnitSystem {
        match self {
            Self::Metric => eulumdat::UnitSystem::Metric,
            Self::Imperial => eulumdat::UnitSystem::Imperial,
        }
    }
}
