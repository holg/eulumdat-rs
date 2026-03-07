//! Standalone egui app for managing WASM editor templates.
//!
//! All changes are in-memory until explicitly saved.
//!
//! Build with: `cargo build -p eulumdat-wasm-templates --features egui-app`
//! Run with:   `cargo run -p eulumdat-wasm-templates --features egui-app --bin eulumdat-templates-gui`

use eframe::egui::{self, Id};
use std::path::PathBuf;

#[path = "manager.rs"]
mod manager;
use manager::*;

/// Reveal a file or directory in the native file manager (Finder / Explorer / xdg-open).
fn reveal_in_file_manager(path: &std::path::Path) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg("-R")
            .arg(path)
            .spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer")
            .arg("/select,")
            .arg(path)
            .spawn();
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(parent) = path.parent() {
            let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 700.0])
            .with_min_inner_size([700.0, 400.0])
            .with_title("Eulumdat Templates Manager")
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "Eulumdat Templates Manager",
        options,
        Box::new(|_cc| Ok(Box::new(TemplatesApp::new()))),
    )
}

// ---------------------------------------------------------------------------
// Drag payload for row reordering
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct DragRow {
    sort: usize,
}

// ---------------------------------------------------------------------------
// Format colors (shared between table and legend)
// ---------------------------------------------------------------------------

fn format_color(format: &TemplateFormat) -> egui::Color32 {
    match format {
        TemplateFormat::Ldt => egui::Color32::from_rgb(70, 140, 70),
        TemplateFormat::IesLm63 => egui::Color32::from_rgb(140, 100, 180),
        TemplateFormat::AtlaXml => egui::Color32::from_rgb(70, 100, 180),
        TemplateFormat::AtlaJson => egui::Color32::from_rgb(180, 120, 50),
    }
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

struct TemplatesApp {
    templates_dir: PathBuf,
    templates: Vec<TemplateMeta>,
    dirty: bool, // unsaved changes
    selected: Option<usize>,
    status_message: Option<(String, bool)>, // (message, is_error)
    verify_results: Vec<VerifyMessage>,
    show_verify: bool,
    // Inline editing
    editing: Option<EditState>,
    // OS file drop overlay
    hovering_files: bool,
    // Confirm remove dialog
    confirm_remove: Option<String>,
}

struct EditState {
    index: usize,
    id: String,
    name: String,
    description: String,
    file: String,
}

impl TemplatesApp {
    fn new() -> Self {
        let templates_dir = default_templates_dir();
        let templates = load_metadata(&templates_dir).unwrap_or_default();
        let status = if templates.is_empty() {
            Some((
                "No templates loaded. Check templates directory path.".into(),
                true,
            ))
        } else {
            Some((format!("Loaded {} templates", templates.len()), false))
        };
        Self {
            templates_dir,
            templates,
            dirty: false,
            selected: None,
            status_message: status,
            verify_results: Vec::new(),
            show_verify: false,
            editing: None,
            hovering_files: false,
            confirm_remove: None,
        }
    }

    /// Reload from disk, discarding all unsaved changes.
    fn reload(&mut self) {
        match load_metadata(&self.templates_dir) {
            Ok(t) => {
                self.templates = t;
                self.dirty = false;
                self.set_status(
                    format!("Reloaded {} templates from disk", self.templates.len()),
                    false,
                );
            }
            Err(e) => self.set_status(format!("Load error: {}", e), true),
        }
        self.selected = None;
        self.editing = None;
    }

    /// Write current in-memory state to disk.
    fn save_to_disk(&mut self) {
        if let Err(e) = save_metadata(&self.templates_dir, &self.templates) {
            self.set_status(format!("Save error: {}", e), true);
        } else {
            self.dirty = false;
            self.set_status("Saved to disk".into(), false);
        }
    }

    /// Mark in-memory state as changed.
    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    fn set_status(&mut self, msg: String, is_error: bool) {
        self.status_message = Some((msg, is_error));
    }

    fn add_files_from_paths(&mut self, paths: Vec<PathBuf>) {
        let mut added = 0;
        let mut errors = Vec::new();
        for path in &paths {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            if !["ldt", "xml", "json", "ies"].contains(&ext.as_str()) {
                continue;
            }
            match add_template(
                &mut self.templates,
                &self.templates_dir,
                path,
                None,
                None,
                None,
            ) {
                Ok(_) => added += 1,
                Err(e) => {
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    errors.push(format!("{}: {}", name, e));
                }
            }
        }
        if added > 0 {
            self.mark_dirty();
        }
        if errors.is_empty() && added > 0 {
            self.set_status(format!("Added {} template(s) (unsaved)", added), false);
        } else if !errors.is_empty() {
            self.set_status(
                format!(
                    "Added {}, {} failed: {}",
                    added,
                    errors.len(),
                    errors.join("; ")
                ),
                true,
            );
        }
    }

    fn start_editing(&mut self, idx: usize) {
        let t = &self.templates[idx];
        self.selected = Some(idx);
        self.editing = Some(EditState {
            index: idx,
            id: t.id.clone(),
            name: t.name.clone(),
            description: t.description.clone(),
            file: t.file.clone(),
        });
    }
}

impl eframe::App for TemplatesApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle OS file drops (add new templates)
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            self.hovering_files = !i.raw.hovered_files.is_empty();
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        if !dropped.is_empty() {
            self.add_files_from_paths(dropped);
        }

        // Top panel: toolbar
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Dir:");
                let dir_str = self.templates_dir.to_string_lossy().to_string();
                let mut dir_edit = dir_str.clone();
                if ui
                    .add(egui::TextEdit::singleline(&mut dir_edit).desired_width(300.0))
                    .changed()
                {
                    self.templates_dir = PathBuf::from(&dir_edit);
                }
                if ui.button("Browse...").clicked() {
                    if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                        self.templates_dir = dir;
                        self.reload();
                    }
                }

                ui.separator();

                if ui
                    .button("Reload")
                    .on_hover_text("Discard changes, re-read from disk")
                    .clicked()
                {
                    self.reload();
                }

                // Save button — highlighted when dirty
                let save_btn = if self.dirty {
                    egui::Button::new(egui::RichText::new("Save").strong())
                } else {
                    egui::Button::new("Save")
                };
                if ui
                    .add_enabled(self.dirty, save_btn)
                    .on_hover_text("Write changes to templates-metadata.toml")
                    .clicked()
                {
                    self.save_to_disk();
                }

                ui.separator();

                if ui.button("Add...").clicked() {
                    if let Some(paths) = rfd::FileDialog::new()
                        .add_filter("Photometric", &["ldt", "xml", "json", "ies"])
                        .pick_files()
                    {
                        self.add_files_from_paths(paths);
                    }
                }

                if ui.button("Verify").clicked() {
                    self.verify_results = verify(&self.templates, &self.templates_dir);
                    self.show_verify = true;
                    let errors = self.verify_results.iter().filter(|m| m.is_error).count();
                    let warnings = self.verify_results.iter().filter(|m| !m.is_error).count();
                    self.set_status(
                        format!("Verify: {} errors, {} warnings", errors, warnings),
                        errors > 0,
                    );
                }

                if ui
                    .button("Build")
                    .on_hover_text(
                        "Regenerate lib.rs + templates.rs from current state (saves first)",
                    )
                    .clicked()
                {
                    // Save first so generated code matches metadata
                    if self.dirty {
                        self.save_to_disk();
                    }
                    match workspace_root_from_templates_dir(&self.templates_dir) {
                        Ok(root) => match build(&self.templates, &root) {
                            Ok((lib_path, tmpl_path)) => {
                                self.set_status(
                                    format!(
                                        "Built: {}, {}",
                                        lib_path.file_name().unwrap_or_default().to_string_lossy(),
                                        tmpl_path.file_name().unwrap_or_default().to_string_lossy()
                                    ),
                                    false,
                                );
                                reveal_in_file_manager(&lib_path);
                            }
                            Err(e) => self.set_status(format!("Build error: {}", e), true),
                        },
                        Err(e) => {
                            self.set_status(format!("Can't find workspace: {}", e), true);
                        }
                    }
                }
            });
        });

        // Bottom panel: status bar + format legend
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Dirty indicator
                if self.dirty {
                    ui.colored_label(egui::Color32::from_rgb(200, 170, 0), "* unsaved");
                    ui.separator();
                }

                // Status message
                if let Some((msg, is_error)) = &self.status_message {
                    let color = if *is_error {
                        egui::Color32::from_rgb(220, 50, 50)
                    } else {
                        ui.visuals().text_color()
                    };
                    ui.colored_label(color, msg);
                }

                // Right side: legend + count
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("{} templates", self.templates.len()));
                    ui.separator();
                    for (fmt, label) in [
                        (TemplateFormat::Ldt, "LDT"),
                        (TemplateFormat::IesLm63, "IES"),
                        (TemplateFormat::AtlaXml, "ATLA XML"),
                        (TemplateFormat::AtlaJson, "ATLA JSON"),
                    ] {
                        ui.colored_label(format_color(&fmt), label);
                    }
                });
            });
        });

        // Right panel: details / edit
        egui::SidePanel::right("details")
            .min_width(280.0)
            .default_width(340.0)
            .show(ctx, |ui| {
                self.show_detail_panel(ui);
            });

        // Verify results window
        if self.show_verify {
            let mut open = self.show_verify;
            egui::Window::new("Verify Results")
                .open(&mut open)
                .default_width(500.0)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for m in &self.verify_results {
                            let (icon, color) = if m.is_error {
                                ("ERR", egui::Color32::from_rgb(220, 50, 50))
                            } else {
                                ("WARN", egui::Color32::from_rgb(200, 170, 0))
                            };
                            ui.horizontal(|ui| {
                                ui.colored_label(color, icon);
                                ui.label(&m.message);
                            });
                        }
                        if self.verify_results.is_empty() {
                            ui.label("All checks passed.");
                        }
                    });
                });
            self.show_verify = open;
        }

        // Confirm remove dialog
        if self.confirm_remove.is_some() {
            let id = self.confirm_remove.clone().unwrap();
            let label = self
                .templates
                .iter()
                .find(|t| t.id == id)
                .map(|t| format!("{} ({})", t.name, t.id))
                .unwrap_or_else(|| id.clone());
            let mut do_remove = false;
            let mut do_cancel = false;

            egui::Window::new("Confirm Remove")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(format!("Remove \"{}\"?", label));
                    ui.weak("The file on disk will not be deleted.");
                    ui.weak("Change is in-memory until you Save.");
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        do_remove = ui.button("Remove").clicked();
                        do_cancel = ui.button("Cancel").clicked();
                    });
                });

            if do_remove {
                if let Err(e) =
                    remove_template(&mut self.templates, &id, false, &self.templates_dir)
                {
                    self.set_status(format!("Remove error: {}", e), true);
                } else {
                    self.selected = None;
                    self.editing = None;
                    self.mark_dirty();
                    self.set_status(format!("Removed '{}' (unsaved)", id), false);
                }
                self.confirm_remove = None;
            }
            if do_cancel {
                self.confirm_remove = None;
            }
        }

        // Central panel: template table with drag-and-drop reordering
        egui::CentralPanel::default().show(ctx, |ui| {
            // Drop overlay when hovering files from OS
            if self.hovering_files {
                let rect = ui.available_rect_before_wrap();
                ui.painter().rect_filled(
                    rect,
                    8.0,
                    egui::Color32::from_rgba_premultiplied(40, 80, 200, 60),
                );
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Drop files here to add templates",
                    egui::FontId::proportional(24.0),
                    egui::Color32::WHITE,
                );
            }

            self.show_template_table(ui);
        });
    }
}

impl TemplatesApp {
    fn show_template_table(&mut self, ui: &mut egui::Ui) {
        // Build sorted view
        let mut sorted_indices: Vec<usize> = (0..self.templates.len()).collect();
        sorted_indices.sort_by_key(|&i| self.templates[i].sort);

        // Track drag-and-drop reorder
        let mut drag_from: Option<usize> = None;
        let mut drop_to: Option<usize> = None;
        let mut action: Option<TableAction> = None;

        egui::ScrollArea::both().show(ui, |ui| {
            // Header row
            ui.horizontal(|ui| {
                ui.add_space(28.0); // drag handle width
                ui.strong("#");
                ui.add_space(8.0);
                ui.strong("Format");
                ui.add_space(8.0);
                ui.strong("ID");
                ui.add_space(60.0);
                ui.strong("Name");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.weak("\u{2717} Remove");
                    ui.weak("\u{270e} Edit");
                });
            });
            ui.separator();

            for (display_row, &idx) in sorted_indices.iter().enumerate() {
                let t = &self.templates[idx];
                let is_selected = self.selected == Some(idx);
                let row_id = Id::new("template_row").with(display_row);

                // Each row is a drop zone
                let (_, dropped_payload) =
                    ui.dnd_drop_zone::<DragRow, ()>(egui::Frame::default(), |ui| {
                        ui.horizontal(|ui| {
                            // Only the drag handle is a drag source
                            let handle_resp = ui
                                .dnd_drag_source(row_id, DragRow { sort: t.sort }, |ui| {
                                    ui.label(egui::RichText::new("\u{2630}").weak().size(14.0));
                                })
                                .response;

                            // Sort number
                            ui.label(
                                egui::RichText::new(format!("{:>2}", t.sort))
                                    .monospace()
                                    .weak(),
                            );

                            // Format badge
                            ui.colored_label(format_color(&t.format), t.format.label());

                            // ID (clickable to select)
                            let id_resp = ui.selectable_label(is_selected, &t.id);
                            if id_resp.clicked() {
                                self.selected = Some(idx);
                                self.editing = None;
                            }

                            // Name (dimmer)
                            ui.weak(&t.name);

                            // Right-aligned actions (RTL: first in code = rightmost)
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .small_button("\u{2717}")
                                        .on_hover_text("Remove (with confirmation)")
                                        .clicked()
                                    {
                                        action = Some(TableAction::ConfirmRemove(t.id.clone()));
                                    }
                                    if ui
                                        .small_button("\u{270e}")
                                        .on_hover_text("Edit in detail panel")
                                        .clicked()
                                    {
                                        action = Some(TableAction::Edit(idx));
                                    }
                                },
                            );

                            // Drop indicator when dragging over this row
                            if ui.ctx().dragged_id().is_some() {
                                let is_being_dragged = ui.ctx().is_being_dragged(row_id);
                                if !is_being_dragged && handle_resp.hovered() {
                                    let rect = ui.min_rect();
                                    ui.painter().hline(
                                        rect.x_range(),
                                        rect.top(),
                                        egui::Stroke::new(
                                            2.0,
                                            egui::Color32::from_rgb(80, 160, 255),
                                        ),
                                    );
                                }
                            }
                        });
                    });

                // Check if something was dropped on this row
                if let Some(payload) = dropped_payload {
                    drag_from = Some(payload.sort);
                    drop_to = Some(t.sort);
                }
            }
        });

        // Process deferred reorder from drag-and-drop
        if let (Some(from_sort), Some(to_sort)) = (drag_from, drop_to) {
            if from_sort != to_sort {
                if let Some(t) = self.templates.iter().find(|t| t.sort == from_sort) {
                    let id = t.id.clone();
                    if sort_template(&mut self.templates, &id, SortAction::Position(to_sort))
                        .is_ok()
                    {
                        self.mark_dirty();
                    }
                }
            }
        }

        // Process deferred button actions
        if let Some(act) = action {
            match act {
                TableAction::ConfirmRemove(id) => {
                    self.confirm_remove = Some(id);
                }
                TableAction::Edit(idx) => {
                    self.start_editing(idx);
                }
            }
        }
    }

    fn show_detail_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Details");
        ui.separator();

        let sel = match self.selected {
            Some(i) if i < self.templates.len() => i,
            _ => {
                ui.label("Select a template from the table.");
                ui.add_space(16.0);
                ui.weak("Drag files from Finder to add templates.\nDrag \u{2630} handles to reorder.\n\nAll changes are in-memory.\nUse Save to write to disk.");
                return;
            }
        };

        let is_editing = self.editing.as_ref().is_some_and(|e| e.index == sel);
        let mut do_apply = false;
        let mut do_cancel = false;
        let mut do_start_edit = false;

        if is_editing {
            let edit = self.editing.as_mut().unwrap();

            let id_err = validate_id(&edit.id).err();

            egui::Grid::new("detail_edit_grid")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    ui.label("ID:");
                    ui.text_edit_singleline(&mut edit.id);
                    ui.end_row();

                    // Show ID validation hint
                    if let Some(ref e) = id_err {
                        ui.label("");
                        ui.colored_label(egui::Color32::from_rgb(220, 50, 50), format!("{}", e));
                        ui.end_row();
                    }

                    ui.label("File:");
                    ui.text_edit_singleline(&mut edit.file);
                    ui.end_row();

                    ui.label("Format:");
                    ui.label(self.templates[sel].format.label());
                    ui.end_row();

                    ui.label("Sort:");
                    ui.label(self.templates[sel].sort.to_string());
                    ui.end_row();

                    ui.label("Const:");
                    ui.label(egui::RichText::new(id_to_const_name(&edit.id)).monospace());
                    ui.end_row();
                });

            ui.weak("ID: lowercase alphanumeric and hyphens only");

            ui.separator();

            ui.label("Name:");
            ui.text_edit_singleline(&mut edit.name);

            ui.add_space(4.0);
            ui.label("Description:");
            ui.add(
                egui::TextEdit::multiline(&mut edit.description)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY),
            );

            ui.add_space(8.0);
            let can_apply = id_err.is_none();
            ui.horizontal(|ui| {
                do_apply = ui
                    .add_enabled(can_apply, egui::Button::new("Apply"))
                    .on_hover_text("Apply to in-memory state (not yet saved to disk)")
                    .clicked();
                do_cancel = ui.button("Cancel").clicked();
            });
        } else {
            egui::Grid::new("detail_view_grid")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    ui.label("ID:");
                    ui.label(&self.templates[sel].id);
                    ui.end_row();

                    ui.label("File:");
                    ui.label(&self.templates[sel].file);
                    ui.end_row();

                    ui.label("Format:");
                    ui.colored_label(
                        format_color(&self.templates[sel].format),
                        self.templates[sel].format.label(),
                    );
                    ui.end_row();

                    ui.label("Sort:");
                    ui.label(self.templates[sel].sort.to_string());
                    ui.end_row();

                    ui.label("Const:");
                    ui.label(
                        egui::RichText::new(id_to_const_name(&self.templates[sel].id)).monospace(),
                    );
                    ui.end_row();

                    ui.label("Name:");
                    ui.label(&self.templates[sel].name);
                    ui.end_row();

                    ui.label("Desc:");
                    ui.label(&self.templates[sel].description);
                    ui.end_row();
                });

            ui.add_space(8.0);
            do_start_edit = ui.button("Edit").clicked();
        }

        // Deferred mutations
        if do_apply {
            if let Some(edit) = self.editing.take() {
                self.templates[sel].id = edit.id;
                self.templates[sel].name = edit.name;
                self.templates[sel].description = edit.description;
                self.templates[sel].file = edit.file;
                self.mark_dirty();
            }
        }
        if do_cancel {
            self.editing = None;
        }
        if do_start_edit {
            self.start_editing(sel);
        }
    }
}

enum TableAction {
    ConfirmRemove(String),
    Edit(usize),
}
