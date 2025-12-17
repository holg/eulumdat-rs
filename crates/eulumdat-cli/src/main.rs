//! EULUMDAT CLI - Command-line tool for photometric files.
//!
//! Supports EULUMDAT (.ldt) and IES (.ies) photometric file formats.

mod cli;
mod commands;

use anyhow::Result;
use clap::Parser;

use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Info { file, verbose } => commands::info(&file, verbose),
        Commands::Validate { file, strict } => commands::validate(&file, strict),
        Commands::Convert {
            input,
            output,
            compact,
        } => commands::convert(&input, &output, compact),
        Commands::Diagram {
            input,
            output,
            diagram_type,
            dark,
            width,
            height,
        } => commands::diagram(&input, output.as_ref(), diagram_type, dark, width, height),
        Commands::Bug { file, svg, dark } => commands::bug(&file, svg.as_ref(), dark),
        Commands::Batch {
            input_dir,
            output_dir,
            format,
            recursive,
            overwrite,
        } => commands::batch(
            &input_dir,
            output_dir.as_ref(),
            format,
            recursive,
            overwrite,
        ),
        Commands::Summary {
            file,
            format,
            output,
        } => commands::summary(&file, format, output.as_ref()),
        Commands::Gldf {
            file,
            output,
            pretty,
        } => commands::gldf(&file, output.as_ref(), pretty),
        Commands::Calc { file, calc_type } => commands::calc(&file, calc_type),
        Commands::ValidateAtla { file, schema, xsd } => {
            commands::validate_atla(&file, schema.as_ref(), xsd)
        }
    }
}
