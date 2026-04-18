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
        Commands::Info {
            file,
            verbose,
            units,
        } => commands::info(&file, verbose, units.to_unit_system()),
        Commands::Validate { file, strict } => commands::validate(&file, strict),
        Commands::Convert {
            input,
            output,
            compact,
            rotate,
        } => commands::convert(&input, &output, compact, rotate),
        Commands::Diagram {
            input,
            output,
            diagram_type,
            dark,
            width,
            height,
            mounting_height,
            tilt,
            log_scale,
            units,
        } => commands::diagram(
            &input,
            output.as_ref(),
            diagram_type,
            dark,
            width,
            height,
            mounting_height,
            tilt,
            log_scale,
            units.to_unit_system(),
        ),
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
        #[cfg(feature = "parquet")]
        Commands::ExportParquet {
            input_dir,
            output,
            recursive,
        } => commands::export_parquet(&input_dir, &output, recursive),
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
        Commands::ValidateAtla {
            file,
            schema,
            schema_type,
            xsd,
        } => commands::validate_atla(&file, schema.as_ref(), schema_type, xsd),
        Commands::AtlaConvert {
            input,
            output,
            target,
            policy,
            verbose,
            compact,
        } => commands::atla_convert(&input, &output, target, policy, verbose, compact),
        Commands::Compare {
            file_a,
            file_b,
            format,
            diagram,
            output,
            dark,
            significant_only,
            units,
        } => commands::compare(
            &file_a,
            &file_b,
            format,
            diagram,
            output.as_ref(),
            dark,
            significant_only,
            units.to_unit_system(),
        ),
        Commands::Report {
            input,
            output,
            paper,
            compact,
            cu_table,
            ugr_table,
            candela_table,
        } => commands::report(
            &input,
            &output,
            paper,
            compact,
            cu_table,
            ugr_table,
            candela_table,
        ),
        Commands::Interpolate {
            inputs,
            steps,
            range,
            count,
            at,
            format,
            output_dir,
            param_name,
            overwrite,
        } => commands::interpolate(
            &inputs,
            steps.as_deref(),
            range.as_deref(),
            count,
            at,
            format,
            &output_dir,
            &param_name,
            overwrite,
        ),
    }
}
