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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    /// Convert to LDT format (common in Europe)
    Ldt,
    /// Convert to IES format (common in North America)
    Ies,
}
