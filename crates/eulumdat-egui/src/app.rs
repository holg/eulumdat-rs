//! Main application state and UI

use eframe::egui::{self, Color32, Margin, RichText, Rounding, TextureHandle, Vec2};
use eulumdat::{Eulumdat, IesExporter};
use std::path::PathBuf;

use crate::diagram::Butterfly3DRenderer;
use crate::templates::{self, Template};
use crate::ui::{
    diagram_panel::{generate_svg, render_diagram_selector},
    render_diagram_panel, render_info_panel, render_tab_bar,
    tabs::{
        render_dimensions_tab, render_general_tab, render_intensity_tab, render_lamps_tab,
        render_optical_tab, render_validation_tab, IntensityTabState,
    },
    AppTab, DiagramType,
};

/// Application state
pub struct EulumdatApp {
    /// Currently loaded file
    pub current_file: Option<PathBuf>,
    /// Parsed Eulumdat data
    pub eulumdat: Option<Eulumdat>,
    /// Error message if parsing failed
    pub error: Option<String>,
    /// Selected diagram type
    pub diagram_type: DiagramType,
    /// Use dark theme for diagrams
    pub dark_theme: bool,
    /// Cached texture for current diagram
    texture: Option<TextureHandle>,
    /// Whether texture needs refresh
    texture_dirty: bool,
    /// Show info panel
    pub show_info: bool,
    /// Current tab
    pub current_tab: AppTab,
    /// 3D renderer
    butterfly_3d: Butterfly3DRenderer,
    /// Show colors in intensity table
    pub intensity_show_colors: bool,
}

impl EulumdatApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            current_file: None,
            eulumdat: None,
            error: None,
            diagram_type: DiagramType::Polar,
            dark_theme: false,
            texture: None,
            texture_dirty: true,
            show_info: true,
            current_tab: AppTab::Diagram,
            butterfly_3d: Butterfly3DRenderer::new(),
            intensity_show_colors: true,
        }
    }

    /// Load a file from path
    pub fn load_file(&mut self, path: PathBuf) {
        self.error = None;
        self.eulumdat = None;
        self.texture = None;
        self.texture_dirty = true;

        let content = match std::fs::read(&path) {
            Ok(bytes) => {
                let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes);
                decoded.into_owned()
            }
            Err(e) => {
                self.error = Some(format!("Failed to read file: {}", e));
                return;
            }
        };

        let is_ies = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase() == "ies")
            .unwrap_or(false);

        if is_ies {
            match eulumdat::IesParser::parse(&content) {
                Ok(ldt) => {
                    self.eulumdat = Some(ldt);
                    self.current_file = Some(path);
                    self.butterfly_3d
                        .update_from_eulumdat(self.eulumdat.as_ref());
                }
                Err(e) => {
                    self.error = Some(format!("Failed to parse IES: {}", e));
                }
            }
        } else {
            match Eulumdat::parse(&content) {
                Ok(ldt) => {
                    self.eulumdat = Some(ldt);
                    self.current_file = Some(path);
                    self.butterfly_3d
                        .update_from_eulumdat(self.eulumdat.as_ref());
                }
                Err(e) => match eulumdat::IesParser::parse(&content) {
                    Ok(ldt) => {
                        self.eulumdat = Some(ldt);
                        self.current_file = Some(path);
                        self.butterfly_3d
                            .update_from_eulumdat(self.eulumdat.as_ref());
                    }
                    Err(ies_err) => {
                        self.error = Some(format!("LDT: {}\nIES: {}", e, ies_err));
                    }
                },
            }
        }
    }

    /// Load from template
    pub fn load_template(&mut self, template: &Template) {
        self.error = None;
        self.texture = None;
        self.texture_dirty = true;

        match template.parse() {
            Ok(ldt) => {
                self.eulumdat = Some(ldt);
                self.current_file = Some(PathBuf::from(format!("{}.ldt", template.id)));
                self.butterfly_3d
                    .update_from_eulumdat(self.eulumdat.as_ref());
            }
            Err(e) => {
                self.error = Some(format!("Failed to parse template: {}", e));
            }
        }
    }

    /// Export current diagram as SVG
    pub fn export_svg(&self) -> Option<String> {
        self.eulumdat
            .as_ref()
            .and_then(|ldt| generate_svg(ldt, self.diagram_type, 800.0, 800.0, self.dark_theme))
    }

    /// Export as IES
    pub fn export_ies(&self) -> Option<String> {
        self.eulumdat.as_ref().map(IesExporter::export)
    }

    /// Export as LDT
    pub fn export_ldt(&self) -> Option<String> {
        self.eulumdat.as_ref().map(Eulumdat::to_ldt)
    }

    fn open_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Photometric Files", &["ldt", "ies", "LDT", "IES"])
            .add_filter("EULUMDAT", &["ldt", "LDT"])
            .add_filter("IES", &["ies", "IES"])
            .add_filter("All Files", &["*"])
            .pick_file()
        {
            self.load_file(path);
        }
    }

    /// Render the welcome/empty state
    fn render_welcome(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);

                // Logo/Title
                ui.label(
                    RichText::new("EULUMDAT")
                        .size(32.0)
                        .strong()
                        .color(Color32::from_rgb(59, 130, 246)),
                );
                ui.label(
                    RichText::new("Photometric Data Viewer")
                        .size(14.0)
                        .color(Color32::GRAY),
                );

                ui.add_space(30.0);

                // Open file button
                let button = egui::Button::new(RichText::new("  Open File...").size(16.0))
                    .min_size(Vec2::new(200.0, 40.0))
                    .rounding(Rounding::same(8.0));

                if ui.add(button).clicked() {
                    self.open_file_dialog();
                }

                ui.add_space(10.0);
                ui.label(
                    RichText::new("or drag & drop LDT/IES files")
                        .size(12.0)
                        .color(Color32::GRAY),
                );

                ui.add_space(40.0);
                ui.separator();
                ui.add_space(20.0);

                // Templates section
                ui.label(RichText::new("Sample Templates").size(16.0).strong());
                ui.add_space(15.0);

                // Template cards in a grid
                let card_width = 280.0;
                let cards_per_row = ((available.x - 40.0) / (card_width + 10.0))
                    .floor()
                    .max(1.0) as usize;

                let templates = templates::all_templates();
                let mut template_to_load: Option<&Template> = None;

                egui::Grid::new("template_grid")
                    .num_columns(cards_per_row)
                    .spacing([10.0, 10.0])
                    .show(ui, |ui| {
                        for (i, template) in templates.iter().enumerate() {
                            if self.render_template_card(ui, template, card_width) {
                                template_to_load = Some(template);
                            }
                            if (i + 1) % cards_per_row == 0 {
                                ui.end_row();
                            }
                        }
                    });

                if let Some(template) = template_to_load {
                    self.load_template(template);
                }

                ui.add_space(40.0);
            });
        });
    }

    /// Render a template card, returns true if clicked
    fn render_template_card(&self, ui: &mut egui::Ui, template: &Template, width: f32) -> bool {
        let mut clicked = false;

        egui::Frame::none()
            .fill(Color32::from_rgb(248, 250, 252))
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(226, 232, 240)))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::same(12.0))
            .show(ui, |ui| {
                ui.set_width(width - 24.0);

                ui.horizontal(|ui| {
                    // Icon placeholder
                    egui::Frame::none()
                        .fill(Color32::from_rgb(59, 130, 246))
                        .rounding(Rounding::same(6.0))
                        .inner_margin(Margin::same(8.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(get_template_icon(template.id))
                                    .size(16.0)
                                    .color(Color32::WHITE),
                            );
                        });

                    ui.vertical(|ui| {
                        ui.label(RichText::new(template.name).strong());
                        ui.label(
                            RichText::new(template.description)
                                .size(11.0)
                                .color(Color32::GRAY),
                        );
                    });
                });

                if ui
                    .interact(
                        ui.min_rect(),
                        ui.id().with(template.id),
                        egui::Sense::click(),
                    )
                    .clicked()
                {
                    clicked = true;
                }
            });

        clicked
    }
}

/// Get icon for template type
fn get_template_icon(id: &str) -> &'static str {
    match id {
        "downlight" => "v",
        "linear" => "=",
        "fluorescent" => "-",
        "projector" => ">",
        "road" => "*",
        "uplight" => "^",
        _ => "o",
    }
}

impl eframe::App for EulumdatApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Style customization
        let mut style = (*ctx.style()).clone();
        style.spacing.button_padding = Vec2::new(8.0, 4.0);
        style.visuals.widgets.inactive.rounding = Rounding::same(4.0);
        style.visuals.widgets.hovered.rounding = Rounding::same(4.0);
        style.visuals.widgets.active.rounding = Rounding::same(4.0);
        ctx.set_style(style);

        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open...").clicked() {
                        self.open_file_dialog();
                        ui.close_menu();
                    }

                    ui.menu_button("New from Template", |ui| {
                        for template in templates::all_templates() {
                            if ui.button(template.name).clicked() {
                                self.load_template(template);
                                ui.close_menu();
                            }
                        }
                    });

                    ui.separator();

                    if self.eulumdat.is_some() {
                        ui.menu_button("Export", |ui| {
                            if ui.button("Export SVG...").clicked() {
                                if let Some(svg) = self.export_svg() {
                                    if let Some(path) = rfd::FileDialog::new()
                                        .add_filter("SVG", &["svg"])
                                        .set_file_name("diagram.svg")
                                        .save_file()
                                    {
                                        let _ = std::fs::write(path, svg);
                                    }
                                }
                                ui.close_menu();
                            }
                            if ui.button("Export IES...").clicked() {
                                if let Some(ies) = self.export_ies() {
                                    if let Some(path) = rfd::FileDialog::new()
                                        .add_filter("IES", &["ies"])
                                        .set_file_name("export.ies")
                                        .save_file()
                                    {
                                        let _ = std::fs::write(path, ies);
                                    }
                                }
                                ui.close_menu();
                            }
                            if ui.button("Export LDT...").clicked() {
                                if let Some(ldt_content) = self.export_ldt() {
                                    if let Some(path) = rfd::FileDialog::new()
                                        .add_filter("LDT", &["ldt"])
                                        .set_file_name("export.ldt")
                                        .save_file()
                                    {
                                        let _ = std::fs::write(path, ldt_content);
                                    }
                                }
                                ui.close_menu();
                            }
                        });
                        ui.separator();
                    }

                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.show_info, "Info Panel");
                    ui.separator();
                    if ui
                        .checkbox(&mut self.dark_theme, "Dark Diagram Theme")
                        .changed()
                    {
                        self.texture_dirty = true;
                    }
                });
            });
        });

        // Only show tab bar when we have data
        if self.eulumdat.is_some() {
            egui::TopBottomPanel::top("tab_bar")
                .frame(
                    egui::Frame::none()
                        .fill(Color32::from_rgb(248, 250, 252))
                        .inner_margin(Margin::symmetric(8.0, 4.0)),
                )
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        render_tab_bar(ui, &mut self.current_tab, true);

                        if self.current_tab == AppTab::Diagram {
                            ui.separator();
                            if render_diagram_selector(ui, &mut self.diagram_type) {
                                self.texture_dirty = true;
                            }
                        }
                    });
                });
        }

        // Status bar
        egui::TopBottomPanel::bottom("status_bar")
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgb(248, 250, 252))
                    .inner_margin(Margin::symmetric(8.0, 4.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if let Some(path) = &self.current_file {
                        ui.label(RichText::new(format!("{}", path.display())).size(11.0));
                    } else {
                        ui.label(
                            RichText::new("No file loaded")
                                .size(11.0)
                                .color(Color32::GRAY),
                        );
                    }

                    if let Some(ldt) = &self.eulumdat {
                        ui.separator();
                        ui.label(
                            RichText::new(format!("{:.0} cd/klm", ldt.max_intensity())).size(11.0),
                        );
                        ui.separator();
                        ui.label(
                            RichText::new(format!("{:.0} lm", ldt.total_luminous_flux()))
                                .size(11.0),
                        );
                    }
                });
            });

        // Info panel (right side)
        if self.show_info && self.eulumdat.is_some() {
            egui::SidePanel::right("info_panel")
                .default_width(260.0)
                .frame(
                    egui::Frame::none()
                        .fill(Color32::from_rgb(248, 250, 252))
                        .inner_margin(Margin::same(12.0)),
                )
                .show(ctx, |ui| {
                    if let Some(ldt) = &self.eulumdat {
                        render_info_panel(ui, ldt);
                    }
                });
        }

        // Central panel
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(error) = &self.error {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    ui.label(
                        RichText::new("Error")
                            .size(18.0)
                            .color(Color32::from_rgb(239, 68, 68)),
                    );
                    ui.add_space(10.0);
                    egui::Frame::none()
                        .fill(Color32::from_rgb(254, 242, 242))
                        .rounding(Rounding::same(8.0))
                        .inner_margin(Margin::same(16.0))
                        .show(ui, |ui| {
                            ui.label(RichText::new(error).color(Color32::from_rgb(185, 28, 28)));
                        });
                });
            } else if let Some(ldt) = &mut self.eulumdat {
                match self.current_tab {
                    AppTab::Diagram => {
                        render_diagram_panel(
                            ui,
                            ldt,
                            self.diagram_type,
                            self.dark_theme,
                            &mut self.texture,
                            &mut self.texture_dirty,
                            &mut self.butterfly_3d,
                        );
                    }
                    AppTab::General => render_general_tab(ui, ldt),
                    AppTab::Dimensions => render_dimensions_tab(ui, ldt),
                    AppTab::Lamps => render_lamps_tab(ui, ldt),
                    AppTab::Optical => render_optical_tab(ui, ldt),
                    AppTab::Intensity => {
                        let mut state = IntensityTabState {
                            show_colors: self.intensity_show_colors,
                        };
                        render_intensity_tab(ui, ldt, &mut state);
                        self.intensity_show_colors = state.show_colors;
                    }
                    AppTab::Validation => render_validation_tab(ui, ldt),
                }
            } else {
                self.render_welcome(ui);
            }
        });

        // Handle file drops
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                if let Some(path) = i.raw.dropped_files[0].path.clone() {
                    self.load_file(path);
                }
            }
        });
    }
}
