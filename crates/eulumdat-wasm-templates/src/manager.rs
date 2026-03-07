//! Shared template management logic used by both CLI and egui binaries.
//!
//! Reads/writes `templates-metadata.toml` and generates:
//! - `crates/eulumdat-wasm-templates/src/lib.rs`
//! - `crates/eulumdat-wasm/src/components/templates.rs`

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemplateFormat {
    Ldt,
    IesLm63,
    AtlaXml,
    AtlaJson,
}

impl TemplateFormat {
    pub fn label(&self) -> &'static str {
        match self {
            TemplateFormat::Ldt => "Ldt",
            TemplateFormat::IesLm63 => "IesLm63",
            TemplateFormat::AtlaXml => "AtlaXml",
            TemplateFormat::AtlaJson => "AtlaJson",
        }
    }

    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "ldt" => Some(TemplateFormat::Ldt),
            "ies" => Some(TemplateFormat::IesLm63),
            "xml" => Some(TemplateFormat::AtlaXml),
            "json" => Some(TemplateFormat::AtlaJson),
            _ => None,
        }
    }
}

impl std::fmt::Display for TemplateFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateMeta {
    pub id: String,
    pub name: String,
    pub description: String,
    pub file: String,
    pub format: TemplateFormat,
    pub sort: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MetadataFile {
    template: Vec<TemplateMeta>,
}

// ---------------------------------------------------------------------------
// Verify result
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct VerifyMessage {
    pub is_error: bool,
    pub message: String,
}

impl VerifyMessage {
    fn error(msg: impl Into<String>) -> Self {
        Self {
            is_error: true,
            message: msg.into(),
        }
    }
    fn warning(msg: impl Into<String>) -> Self {
        Self {
            is_error: false,
            message: msg.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Core operations
// ---------------------------------------------------------------------------

/// Resolve the templates directory (where template files + metadata live).
pub fn default_templates_dir() -> PathBuf {
    // Try to find workspace root by looking for the root Cargo.toml
    let mut dir = std::env::current_dir().unwrap_or_default();
    loop {
        let candidate = dir.join("crates/eulumdat-wasm/templates");
        if candidate.is_dir() {
            return candidate;
        }
        if !dir.pop() {
            break;
        }
    }
    // Fallback
    PathBuf::from("crates/eulumdat-wasm/templates")
}

fn metadata_path(templates_dir: &Path) -> PathBuf {
    templates_dir.join("templates-metadata.toml")
}

/// Load metadata from disk.
pub fn load_metadata(templates_dir: &Path) -> Result<Vec<TemplateMeta>> {
    let path = metadata_path(templates_dir);
    let content =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let file: MetadataFile =
        toml::from_str(&content).with_context(|| format!("parsing {}", path.display()))?;
    Ok(file.template)
}

/// Save metadata to disk.
pub fn save_metadata(templates_dir: &Path, templates: &[TemplateMeta]) -> Result<()> {
    let file = MetadataFile {
        template: templates.to_vec(),
    };
    let content = toml::to_string_pretty(&file).context("serializing metadata")?;
    let path = metadata_path(templates_dir);
    std::fs::write(&path, content).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// Generate an ID from a filename: strip extension, lowercase, replace non-alphanumeric with `-`,
/// collapse runs of `-`, strip leading/trailing `-`.
pub fn id_from_filename(filename: &str) -> String {
    let stem = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(filename);
    let raw: String = stem
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    // Collapse runs of '-' and strip leading/trailing
    let mut result = String::new();
    for ch in raw.chars() {
        if ch == '-' {
            if !result.is_empty() && !result.ends_with('-') {
                result.push('-');
            }
        } else {
            result.push(ch);
        }
    }
    result.trim_end_matches('-').to_string()
}

/// Convert id to Rust const name: replace non-alphanumeric with `_`, uppercase, collapse runs.
/// Prefixes with `T_` if the name starts with a digit (Rust identifiers can't start with digits).
pub fn id_to_const_name(id: &str) -> String {
    let raw: String = id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    // Collapse runs of '_' and strip leading/trailing
    let mut result = String::new();
    for ch in raw.chars() {
        if ch == '_' {
            if !result.is_empty() && !result.ends_with('_') {
                result.push('_');
            }
        } else {
            result.push(ch);
        }
    }
    let result = result.trim_end_matches('_').to_string();
    // Rust identifiers can't start with a digit
    if result.starts_with(|c: char| c.is_ascii_digit()) {
        format!("T_{}", result)
    } else {
        result
    }
}

/// Validate an ID: must contain only lowercase alphanumeric and hyphens, no spaces,
/// must not be empty, and must not start/end with a hyphen.
pub fn validate_id(id: &str) -> Result<()> {
    if id.is_empty() {
        bail!("ID cannot be empty");
    }
    if id.contains(' ') {
        bail!("ID cannot contain spaces (use hyphens instead): '{}'", id);
    }
    if id.contains(|c: char| !c.is_ascii_alphanumeric() && c != '-') {
        bail!(
            "ID must contain only lowercase alphanumeric characters and hyphens: '{}'",
            id
        );
    }
    if id.starts_with('-') || id.ends_with('-') {
        bail!("ID must not start or end with a hyphen: '{}'", id);
    }
    if id.contains("--") {
        bail!("ID must not contain consecutive hyphens: '{}'", id);
    }
    Ok(())
}

/// Add a new template from a file path.
pub fn add_template(
    templates: &mut Vec<TemplateMeta>,
    templates_dir: &Path,
    file_path: &Path,
    id: Option<&str>,
    name: Option<&str>,
    description: Option<&str>,
) -> Result<TemplateMeta> {
    let filename = file_path
        .file_name()
        .and_then(|s| s.to_str())
        .context("invalid filename")?;

    let ext = file_path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let format = TemplateFormat::from_extension(ext)
        .with_context(|| format!("unknown extension: .{ext}"))?;

    let gen_id = id
        .map(String::from)
        .unwrap_or_else(|| id_from_filename(filename));

    // Check for duplicate ID
    if templates.iter().any(|t| t.id == gen_id) {
        bail!("template with id '{}' already exists", gen_id);
    }

    // Copy file to templates dir if not already there
    let dest = templates_dir.join(filename);
    if !dest.exists() {
        std::fs::copy(file_path, &dest)
            .with_context(|| format!("copying {} to {}", file_path.display(), dest.display()))?;
    }

    let next_sort = templates.iter().map(|t| t.sort).max().map_or(0, |m| m + 1);

    let meta = TemplateMeta {
        id: gen_id,
        name: name.unwrap_or(filename).to_string(),
        description: description.unwrap_or("").to_string(),
        file: filename.to_string(),
        format,
        sort: next_sort,
    };
    templates.push(meta.clone());
    Ok(meta)
}

/// Remove a template by ID. Returns the removed entry.
pub fn remove_template(
    templates: &mut Vec<TemplateMeta>,
    id: &str,
    delete_file: bool,
    templates_dir: &Path,
) -> Result<TemplateMeta> {
    let idx = templates
        .iter()
        .position(|t| t.id == id)
        .with_context(|| format!("template '{}' not found", id))?;
    let removed = templates.remove(idx);
    reindex_sorts(templates);

    if delete_file {
        let path = templates_dir.join(&removed.file);
        if path.exists() {
            std::fs::remove_file(&path).with_context(|| format!("deleting {}", path.display()))?;
        }
    }
    Ok(removed)
}

/// Rename a template ID.
pub fn rename_template(templates: &mut [TemplateMeta], old_id: &str, new_id: &str) -> Result<()> {
    if templates.iter().any(|t| t.id == new_id) {
        bail!("template with id '{}' already exists", new_id);
    }
    let entry = templates
        .iter_mut()
        .find(|t| t.id == old_id)
        .with_context(|| format!("template '{}' not found", old_id))?;
    entry.id = new_id.to_string();
    Ok(())
}

/// Move a template in the sort order.
pub enum SortAction {
    Up,
    Down,
    Top,
    Bottom,
    Position(usize),
}

pub fn sort_template(
    templates: &mut Vec<TemplateMeta>,
    id: &str,
    action: SortAction,
) -> Result<()> {
    // Sort by current sort index first
    templates.sort_by_key(|t| t.sort);

    let idx = templates
        .iter()
        .position(|t| t.id == id)
        .with_context(|| format!("template '{}' not found", id))?;

    let new_idx = match action {
        SortAction::Up => idx.saturating_sub(1),
        SortAction::Down => (idx + 1).min(templates.len() - 1),
        SortAction::Top => 0,
        SortAction::Bottom => templates.len() - 1,
        SortAction::Position(pos) => pos.min(templates.len() - 1),
    };

    if new_idx != idx {
        let entry = templates.remove(idx);
        templates.insert(new_idx, entry);
    }
    reindex_sorts(templates);
    Ok(())
}

fn reindex_sorts(templates: &mut [TemplateMeta]) {
    for (i, t) in templates.iter_mut().enumerate() {
        t.sort = i;
    }
}

/// Verify all templates. Returns a list of messages (errors and warnings).
pub fn verify(templates: &[TemplateMeta], templates_dir: &Path) -> Vec<VerifyMessage> {
    let mut msgs = Vec::new();

    // 1. Duplicate IDs + ID validation
    let mut seen_ids = HashSet::new();
    for t in templates {
        if !seen_ids.insert(&t.id) {
            msgs.push(VerifyMessage::error(format!("duplicate id: '{}'", t.id)));
        }
        if let Err(e) = validate_id(&t.id) {
            msgs.push(VerifyMessage::error(format!("'{}': {}", t.id, e)));
        }
    }

    // 2. Contiguous sort indices
    let mut sorts: Vec<usize> = templates.iter().map(|t| t.sort).collect();
    sorts.sort();
    let expected: Vec<usize> = (0..templates.len()).collect();
    if sorts != expected {
        msgs.push(VerifyMessage::error(format!(
            "sort indices not contiguous 0..{}: got {:?}",
            templates.len() - 1,
            sorts
        )));
    }

    // 3. Valid format + extension match
    for t in templates {
        let ext = Path::new(&t.file)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let expected_format = TemplateFormat::from_extension(ext);
        match &expected_format {
            Some(fmt) if *fmt == t.format => {}
            Some(fmt) => {
                msgs.push(VerifyMessage::error(format!(
                    "'{}': format {} doesn't match extension .{} (expected {})",
                    t.id, t.format, ext, fmt
                )));
            }
            None => {
                msgs.push(VerifyMessage::error(format!(
                    "'{}': unknown extension .{}",
                    t.id, ext
                )));
            }
        }
    }

    // 4. Referenced files exist
    for t in templates {
        let path = templates_dir.join(&t.file);
        if !path.exists() {
            msgs.push(VerifyMessage::error(format!(
                "'{}': file not found: {}",
                t.id,
                path.display()
            )));
        }
    }

    // 5. Orphan files (warning)
    let referenced: HashSet<&str> = templates.iter().map(|t| t.file.as_str()).collect();
    if let Ok(entries) = std::fs::read_dir(templates_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            // Skip metadata file, hidden files, directories
            if name_str == "templates-metadata.toml"
                || name_str.starts_with('.')
                || entry.path().is_dir()
            {
                continue;
            }
            // Skip non-template extensions
            let ext = Path::new(&*name_str)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            if !["ldt", "xml", "json", "ies"].contains(&ext) {
                continue;
            }
            if !referenced.contains(name_str.as_ref()) {
                msgs.push(VerifyMessage::warning(format!(
                    "orphan file not in metadata: {}",
                    name_str
                )));
            }
        }
    }

    // 6. Parse each template
    for t in templates {
        let path = templates_dir.join(&t.file);
        if !path.exists() {
            continue; // Already reported above
        }
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                msgs.push(VerifyMessage::error(format!(
                    "'{}': failed to read: {}",
                    t.id, e
                )));
                continue;
            }
        };
        match t.format {
            TemplateFormat::Ldt => {
                if let Err(e) = eulumdat::Eulumdat::parse(&content) {
                    msgs.push(VerifyMessage::error(format!(
                        "'{}': LDT parse error: {}",
                        t.id, e
                    )));
                }
            }
            TemplateFormat::IesLm63 => {
                if let Err(e) = eulumdat::IesParser::parse(&content) {
                    msgs.push(VerifyMessage::error(format!(
                        "'{}': IES parse error: {}",
                        t.id, e
                    )));
                }
            }
            TemplateFormat::AtlaXml => {
                if let Err(e) = atla::xml::parse(&content) {
                    msgs.push(VerifyMessage::error(format!(
                        "'{}': ATLA XML parse error: {}",
                        t.id, e
                    )));
                }
            }
            TemplateFormat::AtlaJson => {
                if let Err(e) = atla::json::parse(&content) {
                    msgs.push(VerifyMessage::error(format!(
                        "'{}': ATLA JSON parse error: {}",
                        t.id, e
                    )));
                }
            }
        }
    }

    msgs
}

// ---------------------------------------------------------------------------
// Code generation
// ---------------------------------------------------------------------------

/// Generate the content for `crates/eulumdat-wasm-templates/src/lib.rs`.
pub fn generate_lib_rs(templates: &[TemplateMeta]) -> String {
    let mut sorted: Vec<&TemplateMeta> = templates.iter().collect();
    sorted.sort_by_key(|t| t.sort);

    let mut out = String::new();
    out.push_str("//! Auto-generated by `eulumdat-templates build` -- DO NOT EDIT\n\n");
    out.push_str("use wasm_bindgen::prelude::*;\n\n");
    out.push_str("/// Get template file content by ID.\n");
    out.push_str("/// Returns None if the ID is not recognized.\n");
    out.push_str("#[wasm_bindgen]\n");
    out.push_str("pub fn get_template_content(id: &str) -> Option<String> {\n");
    out.push_str("    let content = match id {\n");

    for t in &sorted {
        out.push_str(&format!(
            "        \"{}\" => include_str!(\"../../eulumdat-wasm/templates/{}\"),\n",
            t.id, t.file
        ));
    }

    out.push_str("        _ => return None,\n");
    out.push_str("    };\n");
    out.push_str("    Some(content.to_string())\n");
    out.push_str("}\n");
    out
}

/// Generate the content for `crates/eulumdat-wasm/src/components/templates.rs`.
pub fn generate_templates_rs(templates: &[TemplateMeta]) -> String {
    let mut sorted: Vec<&TemplateMeta> = templates.iter().collect();
    sorted.sort_by_key(|t| t.sort);

    let mut out = String::new();
    out.push_str("//! Auto-generated by `eulumdat-templates build` -- DO NOT EDIT\n");
    out.push_str("//! Built-in templates for photometric files\n");
    out.push_str("//! Template content is lazily loaded from a separate WASM module (eulumdat-wasm-templates).\n\n");

    // TemplateFormat enum
    out.push_str("/// Template format\n");
    out.push_str("#[derive(Clone, Copy, PartialEq, Eq)]\n");
    out.push_str("pub enum TemplateFormat {\n");
    out.push_str("    Ldt,\n");
    out.push_str("    IesLm63,\n");
    out.push_str("    AtlaXml,\n");
    out.push_str("    AtlaJson,\n");
    out.push_str("}\n\n");

    // Template struct
    out.push_str("/// Template definition with name, format, and lazy-loaded content ID\n");
    out.push_str("pub struct Template {\n");
    out.push_str("    pub name: &'static str,\n");
    out.push_str("    pub description: &'static str,\n");
    out.push_str("    pub format: TemplateFormat,\n");
    out.push_str("    pub id: &'static str,\n");
    out.push_str("}\n\n");

    // Const definitions
    for t in &sorted {
        let const_name = id_to_const_name(&t.id);
        let format_variant = match t.format {
            TemplateFormat::Ldt => "Ldt",
            TemplateFormat::IesLm63 => "IesLm63",
            TemplateFormat::AtlaXml => "AtlaXml",
            TemplateFormat::AtlaJson => "AtlaJson",
        };
        out.push_str(&format!(
            "pub const {}: Template = Template {{\n",
            const_name
        ));
        out.push_str(&format!("    name: \"{}\",\n", escape_str(&t.name)));
        out.push_str(&format!(
            "    description: \"{}\",\n",
            escape_str(&t.description)
        ));
        out.push_str(&format!(
            "    format: TemplateFormat::{},\n",
            format_variant
        ));
        out.push_str(&format!("    id: \"{}\",\n", t.id));
        out.push_str("};\n\n");
    }

    // ALL_TEMPLATES array
    out.push_str("/// All available templates\n");
    out.push_str("pub const ALL_TEMPLATES: &[&Template] = &[\n");
    for t in &sorted {
        let const_name = id_to_const_name(&t.id);
        out.push_str(&format!("    &{},\n", const_name));
    }
    out.push_str("];\n");
    out
}

fn escape_str(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Write generated files to disk.
pub fn build(templates: &[TemplateMeta], workspace_root: &Path) -> Result<(PathBuf, PathBuf)> {
    let lib_rs_path = workspace_root.join("crates/eulumdat-wasm-templates/src/lib.rs");
    let templates_rs_path = workspace_root.join("crates/eulumdat-wasm/src/components/templates.rs");

    let lib_rs = generate_lib_rs(templates);
    let templates_rs = generate_templates_rs(templates);

    std::fs::write(&lib_rs_path, lib_rs)
        .with_context(|| format!("writing {}", lib_rs_path.display()))?;
    std::fs::write(&templates_rs_path, templates_rs)
        .with_context(|| format!("writing {}", templates_rs_path.display()))?;

    Ok((lib_rs_path, templates_rs_path))
}

/// Resolve workspace root from a templates directory path.
pub fn workspace_root_from_templates_dir(templates_dir: &Path) -> Result<PathBuf> {
    // templates_dir is typically <root>/crates/eulumdat-wasm/templates
    let root = templates_dir
        .ancestors()
        .find(|p| p.join("Cargo.toml").exists() && p.join("crates").is_dir())
        .context("could not find workspace root")?;
    Ok(root.to_path_buf())
}
