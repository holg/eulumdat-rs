//! CLI tool for managing eulumdat-wasm templates.
//!
//! Build with: `cargo build -p eulumdat-wasm-templates --features cli`
//! Run with:   `cargo run -p eulumdat-wasm-templates --features cli -- <command>`

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[path = "manager.rs"]
mod manager;
use manager::*;

#[derive(Parser)]
#[command(name = "eulumdat-templates", about = "Manage WASM editor templates")]
struct Cli {
    /// Path to the templates directory
    #[arg(long, default_value_os_t = default_templates_dir())]
    templates_dir: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List all templates
    List,
    /// Show details for a specific template
    Info {
        /// Template ID
        id: String,
    },
    /// Add a new template file
    Add {
        /// Path to the template file
        file: PathBuf,
        /// Override generated ID
        #[arg(long)]
        id: Option<String>,
        /// Display name
        #[arg(long)]
        name: Option<String>,
        /// Description
        #[arg(long)]
        description: Option<String>,
    },
    /// Remove a template
    Remove {
        /// Template ID
        id: String,
        /// Also delete the template file
        #[arg(long)]
        delete_file: bool,
    },
    /// Rename a template ID
    Rename {
        /// Current ID
        old_id: String,
        /// New ID
        new_id: String,
    },
    /// Reorder a template
    Sort {
        /// Template ID
        id: String,
        /// Direction or position
        action: String,
    },
    /// Verify all templates
    Verify,
    /// Regenerate lib.rs and templates.rs
    Build,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let dir = &cli.templates_dir;

    match cli.command {
        Command::List => {
            let templates = load_metadata(dir)?;
            let mut sorted = templates.clone();
            sorted.sort_by_key(|t| t.sort);
            println!(
                "{:<4} {:<25} {:<12} {:<30} {}",
                "Sort", "ID", "Format", "Name", "File"
            );
            println!("{}", "-".repeat(100));
            for t in &sorted {
                println!(
                    "{:<4} {:<25} {:<12} {:<30} {}",
                    t.sort, t.id, t.format, t.name, t.file
                );
            }
            println!("\n{} templates total", sorted.len());
        }
        Command::Info { id } => {
            let templates = load_metadata(dir)?;
            let t = templates
                .iter()
                .find(|t| t.id == id)
                .with_context(|| format!("template '{}' not found", id))?;
            println!("ID:          {}", t.id);
            println!("Name:        {}", t.name);
            println!("Description: {}", t.description);
            println!("File:        {}", t.file);
            println!("Format:      {}", t.format);
            println!("Sort:        {}", t.sort);
            println!("Const:       {}", id_to_const_name(&t.id));

            // Try parsing
            let path = dir.join(&t.file);
            if path.exists() {
                let content = std::fs::read_to_string(&path)?;
                match t.format {
                    TemplateFormat::Ldt => match eulumdat::Eulumdat::parse(&content) {
                        Ok(ldt) => println!(
                            "Parse:       OK ({} name: {})",
                            t.format, ldt.luminaire_name
                        ),
                        Err(e) => println!("Parse:       FAIL - {}", e),
                    },
                    TemplateFormat::IesLm63 => match eulumdat::IesParser::parse(&content) {
                        Ok(ldt) => println!("Parse:       OK (IES name: {})", ldt.luminaire_name),
                        Err(e) => println!("Parse:       FAIL - {}", e),
                    },
                    TemplateFormat::AtlaXml => match atla::xml::parse(&content) {
                        Ok(_) => println!("Parse:       OK (ATLA XML)"),
                        Err(e) => println!("Parse:       FAIL - {}", e),
                    },
                    TemplateFormat::AtlaJson => match atla::json::parse(&content) {
                        Ok(_) => println!("Parse:       OK (ATLA JSON)"),
                        Err(e) => println!("Parse:       FAIL - {}", e),
                    },
                }
            } else {
                println!("Parse:       SKIP (file not found)");
            }
        }
        Command::Add {
            file,
            id,
            name,
            description,
        } => {
            let mut templates = load_metadata(dir)?;
            let meta = add_template(
                &mut templates,
                dir,
                &file,
                id.as_deref(),
                name.as_deref(),
                description.as_deref(),
            )?;
            save_metadata(dir, &templates)?;
            println!("Added template '{}' (sort: {})", meta.id, meta.sort);
        }
        Command::Remove { id, delete_file } => {
            let mut templates = load_metadata(dir)?;
            let removed = remove_template(&mut templates, &id, delete_file, dir)?;
            save_metadata(dir, &templates)?;
            println!("Removed template '{}'", removed.id);
            if delete_file {
                println!("Deleted file: {}", removed.file);
            }
        }
        Command::Rename { old_id, new_id } => {
            let mut templates = load_metadata(dir)?;
            rename_template(&mut templates, &old_id, &new_id)?;
            save_metadata(dir, &templates)?;
            println!("Renamed '{}' -> '{}'", old_id, new_id);
        }
        Command::Sort { id, action } => {
            let mut templates = load_metadata(dir)?;
            let sort_action = match action.as_str() {
                "up" => SortAction::Up,
                "down" => SortAction::Down,
                "top" => SortAction::Top,
                "bottom" => SortAction::Bottom,
                n => SortAction::Position(
                    n.parse::<usize>()
                        .with_context(|| format!("invalid sort action: '{}'", action))?,
                ),
            };
            sort_template(&mut templates, &id, sort_action)?;
            save_metadata(dir, &templates)?;
            println!("Reordered '{}'", id);
        }
        Command::Verify => {
            let templates = load_metadata(dir)?;
            let msgs = verify(&templates, dir);
            let errors = msgs.iter().filter(|m| m.is_error).count();
            let warnings = msgs.iter().filter(|m| !m.is_error).count();

            for m in &msgs {
                let prefix = if m.is_error { "ERROR" } else { "WARN " };
                println!("[{}] {}", prefix, m.message);
            }

            println!(
                "\n{} templates, {} errors, {} warnings",
                templates.len(),
                errors,
                warnings
            );
            if errors > 0 {
                std::process::exit(1);
            }
        }
        Command::Build => {
            let templates = load_metadata(dir)?;
            let root = workspace_root_from_templates_dir(dir)?;
            let (lib_path, tmpl_path) = build(&templates, &root)?;
            println!("Generated:");
            println!("  {}", lib_path.display());
            println!("  {}", tmpl_path.display());
        }
    }
    Ok(())
}
