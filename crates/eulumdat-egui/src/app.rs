//! Main application state and UI

use atla::LuminaireOpticalData;
use eframe::egui::{self, Color32, DragValue, Margin, RichText, Rounding, TextureHandle, Vec2};
use eulumdat::compare::{PhotometricComparison, Significance};
use eulumdat::diagram::{CartesianDiagram, ConeDiagram, PolarDiagram};
use eulumdat::{Eulumdat, IesExporter, PhotometricCalculations};
use eulumdat_i18n::{Language, Locale};
use std::path::PathBuf;

use crate::diagram::Butterfly3DRenderer;
use crate::templates::{self, Template};
use crate::ui::{
    diagram_panel::{generate_svg_with_height, DiagramParams},
    render_info_panel, render_main_tab_bar, render_sub_tab_bar,
    tabs::{
        render_dimensions_tab, render_general_tab, render_intensity_tab, render_lamps_tab,
        render_optical_tab, render_validation_tab, IntensityTabState,
    },
    DiagramType, MainTab, SubTab,
};

/// Compare display mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompareMode {
    #[default]
    PolarOverlay,
    CartesianOverlay,
    PolarSideBySide,
    CartesianSideBySide,
    HeatmapSideBySide,
    ConeSideBySide,
    IsoluxSideBySide,
    IsocandlelaSideBySide,
    FloodlightSideBySide,
    MetricsOnly,
}

impl CompareMode {
    pub fn label(&self) -> &'static str {
        match self {
            CompareMode::PolarOverlay => "Polar Overlay",
            CompareMode::CartesianOverlay => "Cartesian Overlay",
            CompareMode::PolarSideBySide => "Polar Side-by-Side",
            CompareMode::CartesianSideBySide => "Cartesian Side-by-Side",
            CompareMode::HeatmapSideBySide => "Heatmap Side-by-Side",
            CompareMode::ConeSideBySide => "Cone Side-by-Side",
            CompareMode::IsoluxSideBySide => "Isolux Side-by-Side",
            CompareMode::IsocandlelaSideBySide => "Isocandela Side-by-Side",
            CompareMode::FloodlightSideBySide => "Floodlight Side-by-Side",
            CompareMode::MetricsOnly => "Metrics Only",
        }
    }

    pub fn all() -> &'static [CompareMode] {
        &[
            CompareMode::PolarOverlay,
            CompareMode::CartesianOverlay,
            CompareMode::PolarSideBySide,
            CompareMode::CartesianSideBySide,
            CompareMode::HeatmapSideBySide,
            CompareMode::ConeSideBySide,
            CompareMode::IsoluxSideBySide,
            CompareMode::IsocandlelaSideBySide,
            CompareMode::FloodlightSideBySide,
            CompareMode::MetricsOnly,
        ]
    }
}

/// Application state
pub struct EulumdatApp {
    /// Currently loaded file
    pub current_file: Option<PathBuf>,
    /// ATLA document (primary data structure)
    pub atla_doc: Option<LuminaireOpticalData>,
    /// Parsed Eulumdat data (derived from ATLA)
    pub eulumdat: Option<Eulumdat>,
    /// Error message if parsing failed
    pub error: Option<String>,
    /// Use dark theme for diagrams
    pub dark_theme: bool,
    /// Cached texture for current diagram
    texture: Option<TextureHandle>,
    /// Whether texture needs refresh
    texture_dirty: bool,
    /// Show info panel
    pub show_info: bool,
    /// Current main tab
    pub main_tab: MainTab,
    /// Current sub-tab
    pub sub_tab: SubTab,
    /// 3D renderer
    butterfly_3d: Butterfly3DRenderer,
    /// Show colors in intensity table
    pub intensity_show_colors: bool,
    /// Mounting height for cone diagram (meters)
    pub mounting_height: f64,
    /// Max height for greenhouse diagram (meters)
    pub greenhouse_height: f64,
    /// Tilt angle for isolux diagram (degrees)
    pub tilt_angle: f64,
    /// Area size for isolux diagram (meters, half-width)
    pub area_size: f64,
    /// Log scale for floodlight diagram
    pub log_scale: bool,
    /// Selected C-plane for per-plane diagrams (None = all)
    pub selected_c_plane: Option<f64>,
    /// Compare file B
    pub compare_ldt: Option<Eulumdat>,
    /// Compare file B name
    pub compare_file_name: String,
    /// Compare mode
    pub compare_mode: CompareMode,
    /// Compare C-plane A
    pub compare_c_plane_a: f64,
    /// Compare C-plane B
    pub compare_c_plane_b: f64,
    /// Link compare sliders
    pub compare_link_sliders: bool,
    /// Compare texture
    pub compare_texture: Option<TextureHandle>,
    /// Compare texture dirty flag
    pub compare_texture_dirty: bool,
    /// Current language
    pub language: Language,
    /// Current locale for translations (derived from language)
    pub locale: Locale,
}

impl EulumdatApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Configure fonts to support CJK characters
        configure_fonts(&cc.egui_ctx);

        Self {
            current_file: None,
            atla_doc: None,
            eulumdat: None,
            error: None,
            dark_theme: false,
            texture: None,
            texture_dirty: true,
            show_info: true,
            main_tab: MainTab::Diagrams,
            sub_tab: SubTab::Polar,
            butterfly_3d: Butterfly3DRenderer::new(),
            intensity_show_colors: true,
            mounting_height: 3.0,
            greenhouse_height: 2.0,
            tilt_angle: 0.0,
            area_size: 20.0,
            log_scale: false,
            selected_c_plane: None,
            compare_ldt: None,
            compare_file_name: String::new(),
            compare_mode: CompareMode::default(),
            compare_c_plane_a: 0.0,
            compare_c_plane_b: 0.0,
            compare_link_sliders: true,
            compare_texture: None,
            compare_texture_dirty: true,
            language: Language::default(),
            locale: Locale::default(), // English by default
        }
    }

    /// Set the current language and update locale
    pub fn set_language(&mut self, lang: Language) {
        self.language = lang;
        self.locale = Locale::for_language(lang);
        self.texture_dirty = true; // Refresh diagrams with new locale
    }

    /// Load a file from path
    pub fn load_file(&mut self, path: PathBuf) {
        self.error = None;
        self.eulumdat = None;
        self.atla_doc = None;
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

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Parse based on file extension
        match ext.as_str() {
            "xml" => {
                // ATLA XML format
                match atla::xml::parse(&content) {
                    Ok(doc) => {
                        self.eulumdat = Some(doc.to_eulumdat());
                        self.atla_doc = Some(doc);
                        self.current_file = Some(path);
                        self.butterfly_3d
                            .update_from_eulumdat(self.eulumdat.as_ref());
                    }
                    Err(e) => {
                        self.error = Some(format!("Failed to parse ATLA XML: {}", e));
                    }
                }
            }
            "json" => {
                // ATLA JSON format
                match atla::json::parse(&content) {
                    Ok(doc) => {
                        self.eulumdat = Some(doc.to_eulumdat());
                        self.atla_doc = Some(doc);
                        self.current_file = Some(path);
                        self.butterfly_3d
                            .update_from_eulumdat(self.eulumdat.as_ref());
                    }
                    Err(e) => {
                        self.error = Some(format!("Failed to parse ATLA JSON: {}", e));
                    }
                }
            }
            "ies" => {
                // IES format
                match eulumdat::IesParser::parse(&content) {
                    Ok(ldt) => {
                        self.atla_doc = Some(LuminaireOpticalData::from_eulumdat(&ldt));
                        self.eulumdat = Some(ldt);
                        self.current_file = Some(path);
                        self.butterfly_3d
                            .update_from_eulumdat(self.eulumdat.as_ref());
                    }
                    Err(e) => {
                        self.error = Some(format!("Failed to parse IES: {}", e));
                    }
                }
            }
            _ => {
                // Try LDT first, then IES
                match Eulumdat::parse(&content) {
                    Ok(ldt) => {
                        self.atla_doc = Some(LuminaireOpticalData::from_eulumdat(&ldt));
                        self.eulumdat = Some(ldt);
                        self.current_file = Some(path);
                        self.butterfly_3d
                            .update_from_eulumdat(self.eulumdat.as_ref());
                    }
                    Err(e) => match eulumdat::IesParser::parse(&content) {
                        Ok(ldt) => {
                            self.atla_doc = Some(LuminaireOpticalData::from_eulumdat(&ldt));
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
    }

    /// Load from template
    pub fn load_template(&mut self, template: &Template) {
        self.error = None;
        self.texture = None;
        self.texture_dirty = true;

        // Parse both ATLA and Eulumdat formats
        match template.parse_atla() {
            Ok(atla) => {
                self.eulumdat = Some(atla.to_eulumdat());
                self.atla_doc = Some(atla);
                let ext = if template.format == templates::TemplateFormat::AtlaXml {
                    "xml"
                } else {
                    "ldt"
                };
                self.current_file = Some(PathBuf::from(format!("{}.{}", template.id, ext)));
                self.butterfly_3d
                    .update_from_eulumdat(self.eulumdat.as_ref());
            }
            Err(e) => {
                self.error = Some(format!("Failed to parse template: {}", e));
            }
        }
    }

    /// Generate SVG for current diagram
    fn generate_current_svg(&self) -> Option<String> {
        let ldt = self.eulumdat.as_ref()?;
        let atla = self.atla_doc.as_ref()?;

        match self.sub_tab {
            SubTab::Polar => {
                let diagram = eulumdat::diagram::PolarDiagram::from_eulumdat(ldt);
                let summary = eulumdat::PhotometricSummary::from_eulumdat(ldt);
                let theme = self.svg_theme();
                Some(diagram.to_svg_with_summary(800.0, 800.0, &theme, &summary))
            }
            SubTab::Cartesian => {
                let diagram =
                    eulumdat::diagram::CartesianDiagram::from_eulumdat(ldt, 800.0, 600.0, 8);
                let summary = eulumdat::PhotometricSummary::from_eulumdat(ldt);
                let theme = self.svg_theme();
                Some(diagram.to_svg_with_summary(800.0, 600.0, &theme, &summary))
            }
            SubTab::BeamAngle => {
                let diagram = eulumdat::diagram::PolarDiagram::from_eulumdat(ldt);
                let analysis = eulumdat::PhotometricCalculations::beam_field_analysis(ldt);
                let theme = self.svg_theme();
                Some(diagram.to_svg_with_beam_field_angles(
                    800.0,
                    800.0,
                    &theme,
                    &analysis,
                    analysis.is_batwing,
                ))
            }
            SubTab::Butterfly3D => {
                let diagram =
                    eulumdat::diagram::ButterflyDiagram::from_eulumdat(ldt, 800.0, 640.0, 60.0);
                let theme = self.svg_theme();
                Some(diagram.to_svg(800.0, 640.0, &theme))
            }
            SubTab::Heatmap => {
                let diagram = eulumdat::diagram::HeatmapDiagram::from_eulumdat(ldt, 800.0, 560.0);
                let theme = self.svg_theme();
                Some(diagram.to_svg(800.0, 560.0, &theme))
            }
            SubTab::Cone => {
                let diagram =
                    eulumdat::diagram::ConeDiagram::from_eulumdat(ldt, self.mounting_height);
                let theme = self.svg_theme();
                Some(diagram.to_svg(800.0, 600.0, &theme))
            }
            SubTab::Spectral => {
                let theme = if self.dark_theme {
                    atla::spectral::SpectralTheme::dark_with_locale(&self.locale)
                } else {
                    atla::spectral::SpectralTheme::light_with_locale(&self.locale)
                };
                // Try to get spectral data from emitters
                if let Some(spd) = atla
                    .emitters
                    .iter()
                    .filter_map(|e| e.spectral_distribution.as_ref())
                    .next()
                {
                    let diagram = atla::spectral::SpectralDiagram::from_spectral(spd);
                    Some(diagram.to_svg(800.0, 480.0, &theme))
                } else if let Some(emitter) = atla.emitters.first() {
                    if let Some(cct) = emitter.cct {
                        let cri = emitter.color_rendering.as_ref().and_then(|cr| cr.ra);
                        let spd = atla::spectral::synthesize_spectrum(cct, cri);
                        let diagram = atla::spectral::SpectralDiagram::from_spectral(&spd);
                        Some(diagram.to_svg(800.0, 480.0, &theme))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            SubTab::Greenhouse => {
                let theme = if self.dark_theme {
                    atla::greenhouse::GreenhouseTheme::dark()
                } else {
                    atla::greenhouse::GreenhouseTheme::light()
                };
                let diagram = atla::greenhouse::GreenhouseDiagram::from_atla_with_height(
                    atla,
                    self.greenhouse_height,
                );
                Some(diagram.to_svg(800.0, 600.0, &theme))
            }
            SubTab::BugRating => {
                let diagram = eulumdat::BugDiagram::from_eulumdat(ldt);
                let theme = self.svg_theme();
                Some(diagram.to_svg_with_details(800.0, 560.0, &theme))
            }
            SubTab::Lcs => {
                let diagram = eulumdat::BugDiagram::from_eulumdat(ldt);
                let theme = self.svg_theme();
                Some(diagram.to_lcs_svg(800.0, 504.0, &theme))
            }
            _ => None,
        }
    }

    fn svg_theme(&self) -> eulumdat::diagram::SvgTheme {
        if self.dark_theme {
            eulumdat::diagram::SvgTheme::dark_with_locale(&self.locale)
        } else {
            eulumdat::diagram::SvgTheme::light_with_locale(&self.locale)
        }
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
            .add_filter(
                "All Photometric",
                &["ldt", "ies", "xml", "json", "LDT", "IES"],
            )
            .add_filter("EULUMDAT", &["ldt", "LDT"])
            .add_filter("IES", &["ies", "IES"])
            .add_filter("ATLA", &["xml", "json"])
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
                let open_label = format!("  {}...", self.locale.ui.header.open);
                let button = egui::Button::new(RichText::new(&open_label).size(16.0))
                    .min_size(Vec2::new(200.0, 40.0))
                    .rounding(Rounding::same(8.0));

                if ui.add(button).clicked() {
                    self.open_file_dialog();
                }

                ui.add_space(10.0);
                ui.label(
                    RichText::new(&self.locale.ui.dropzone.text)
                        .size(12.0)
                        .color(Color32::GRAY),
                );

                ui.add_space(40.0);
                ui.separator();
                ui.add_space(20.0);

                // Templates section
                ui.label(
                    RichText::new(&self.locale.ui.header.templates)
                        .size(16.0)
                        .strong(),
                );
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

    /// Render the diagram panel for the current sub-tab
    fn render_diagram(&mut self, ui: &mut egui::Ui) {
        let ldt = match &self.eulumdat {
            Some(ldt) => ldt,
            None => {
                ui.centered_and_justified(|ui| {
                    ui.label("No data loaded");
                });
                return;
            }
        };

        // Handle 3D butterfly specially (interactive)
        if self.sub_tab == SubTab::Butterfly3D {
            self.render_3d_butterfly(ui);
            return;
        }

        // For other diagrams, render SVG to texture
        let available_size = ui.available_size();
        let size = available_size.min_elem() * 0.95;

        if self.texture_dirty || self.texture.is_none() {
            let params = DiagramParams {
                mounting_height: self.mounting_height,
                tilt_angle: self.tilt_angle,
                area_size: self.area_size,
                log_scale: self.log_scale,
                c_plane: self.selected_c_plane,
            };
            if let Some(svg) = generate_svg_with_height(
                ldt,
                self.sub_tab_to_diagram_type(),
                size as f64,
                size as f64,
                self.dark_theme,
                self.mounting_height,
                &self.locale,
                &params,
            ) {
                match crate::render::render_svg_to_rgba(&svg, size as u32, size as u32) {
                    Ok((pixels, w, h)) => {
                        let image = crate::render::rgba_to_color_image(pixels, w, h);
                        self.texture = Some(ui.ctx().load_texture(
                            "diagram",
                            image,
                            egui::TextureOptions::LINEAR,
                        ));
                        self.texture_dirty = false;
                    }
                    Err(e) => {
                        ui.colored_label(Color32::RED, format!("Render error: {}", e));
                    }
                }
            } else if let Some(svg) = self.generate_current_svg() {
                // Try ATLA-based diagrams
                match crate::render::render_svg_to_rgba(&svg, size as u32, size as u32) {
                    Ok((pixels, w, h)) => {
                        let image = crate::render::rgba_to_color_image(pixels, w, h);
                        self.texture = Some(ui.ctx().load_texture(
                            "diagram",
                            image,
                            egui::TextureOptions::LINEAR,
                        ));
                        self.texture_dirty = false;
                    }
                    Err(e) => {
                        ui.colored_label(Color32::RED, format!("Render error: {}", e));
                    }
                }
            }
        }

        if let Some(tex) = &self.texture {
            let texture_size = tex.size_vec2();
            let scale = (available_size.x / texture_size.x).min(available_size.y / texture_size.y);
            let display_size = texture_size * scale;

            // Center the image but only occupy the diagram's actual size (not the whole panel)
            ui.allocate_new_ui(
                egui::UiBuilder::new().max_rect(egui::Rect::from_center_size(
                    ui.available_rect_before_wrap().center(),
                    display_size,
                )),
                |ui| {
                    ui.image((tex.id(), display_size));
                },
            );
        }
    }

    fn sub_tab_to_diagram_type(&self) -> DiagramType {
        match self.sub_tab {
            SubTab::Polar => DiagramType::Polar,
            SubTab::Cartesian => DiagramType::Cartesian,
            SubTab::BeamAngle => DiagramType::BeamAngle,
            SubTab::Butterfly3D => DiagramType::Butterfly3D,
            SubTab::Heatmap => DiagramType::Heatmap,
            SubTab::Cone => DiagramType::Cone,
            SubTab::BugRating => DiagramType::Bug,
            SubTab::Lcs => DiagramType::Lcs,
            SubTab::Spectral => DiagramType::Spectral,
            SubTab::Greenhouse => DiagramType::Greenhouse,
            SubTab::Isolux => DiagramType::Isolux,
            SubTab::Isocandela => DiagramType::Isocandela,
            SubTab::Floodlight => DiagramType::Floodlight,
            _ => DiagramType::Polar,
        }
    }

    /// Render the BIM parameters panel
    fn render_bim_panel(&mut self, ui: &mut egui::Ui) {
        let atla_doc = match &self.atla_doc {
            Some(doc) => doc,
            None => {
                ui.centered_and_justified(|ui| {
                    ui.label("No data loaded");
                });
                return;
            }
        };

        let bim = atla::bim::BimParameters::from_atla(atla_doc);

        if bim.populated_count() < 3 {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.label(RichText::new("BIM Parameters").size(20.0).strong());
                ui.add_space(10.0);
                ui.label(
                    RichText::new("No BIM parameters available for this luminaire")
                        .color(Color32::GRAY),
                );
                ui.add_space(5.0);
                ui.label(
                    RichText::new("BIM data is extracted from ATLA XML files")
                        .small()
                        .color(Color32::GRAY),
                );
            });
            return;
        }

        // Header
        ui.horizontal(|ui| {
            ui.heading("BIM Parameters");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Copy CSV").clicked() {
                    ui.output_mut(|o| o.copied_text = bim.to_csv());
                }
            });
        });
        ui.separator();
        ui.label(
            RichText::new(format!("{} parameters populated", bim.populated_count()))
                .small()
                .color(Color32::GRAY),
        );
        ui.add_space(5.0);

        // Table of parameters
        let rows = bim.to_table_rows();
        let mut current_group = "";

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("bim_grid")
                .num_columns(3)
                .spacing([20.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    for (group, name, value, unit) in &rows {
                        if *group != current_group {
                            current_group = group;
                            ui.label(
                                RichText::new(*group)
                                    .strong()
                                    .color(Color32::from_rgb(59, 130, 246)),
                            );
                            ui.label("");
                            ui.label("");
                            ui.end_row();
                        }
                        ui.label(RichText::new(*name).small());
                        ui.label(RichText::new(value).small());
                        ui.label(RichText::new(*unit).small().color(Color32::GRAY));
                        ui.end_row();
                    }
                });
        });
    }

    /// Render the compare panel
    fn render_compare_panel(&mut self, ui: &mut egui::Ui) {
        let ldt_a = match &self.eulumdat {
            Some(ldt) => ldt.clone(),
            None => {
                ui.centered_and_justified(|ui| {
                    ui.label("No data loaded");
                });
                return;
            }
        };

        // If no compare file loaded, show empty state
        if self.compare_ldt.is_none() {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.label(RichText::new("Compare Luminaires").size(20.0).strong());
                ui.add_space(10.0);
                ui.label(
                    RichText::new("Load a second file to compare photometric data")
                        .color(Color32::GRAY),
                );
                ui.add_space(20.0);

                if ui
                    .add(
                        egui::Button::new(RichText::new("  Load File B...").size(14.0))
                            .min_size(Vec2::new(180.0, 36.0))
                            .rounding(Rounding::same(8.0)),
                    )
                    .clicked()
                {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter(
                            "All Photometric",
                            &["ldt", "ies", "xml", "json", "LDT", "IES"],
                        )
                        .pick_file()
                    {
                        self.load_compare_file(path);
                    }
                }

                // Templates shortcut
                ui.add_space(15.0);
                ui.label(
                    RichText::new("Or use a template:")
                        .small()
                        .color(Color32::GRAY),
                );
                ui.add_space(5.0);
                let templates = templates::all_templates();
                let mut template_to_load: Option<&Template> = None;
                ui.horizontal_wrapped(|ui| {
                    for template in templates {
                        if ui.small_button(template.name).clicked() {
                            template_to_load = Some(template);
                        }
                    }
                });
                if let Some(template) = template_to_load {
                    if let Ok(ldt) = template.parse() {
                        self.compare_file_name = template.name.to_string();
                        self.compare_ldt = Some(ldt);
                        self.compare_texture_dirty = true;
                    }
                }
            });
            return;
        }

        let ldt_b = self.compare_ldt.as_ref().unwrap().clone();
        let label_a = self
            .current_file
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("File A")
            .to_string();
        let label_b = self.compare_file_name.clone();

        // Header: file B info + clear + mode selector
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("B: {}", label_b)).strong());
            if ui.small_button("x").clicked() {
                self.compare_ldt = None;
                self.compare_file_name.clear();
                self.compare_texture = None;
                return;
            }
            ui.separator();
            if ui.button("Load different...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter(
                        "All Photometric",
                        &["ldt", "ies", "xml", "json", "LDT", "IES"],
                    )
                    .pick_file()
                {
                    self.load_compare_file(path);
                }
            }
        });
        ui.separator();

        // Mode selector
        ui.horizontal_wrapped(|ui| {
            for mode in CompareMode::all() {
                if ui
                    .selectable_label(self.compare_mode == *mode, mode.label())
                    .clicked()
                {
                    self.compare_mode = *mode;
                    self.compare_texture_dirty = true;
                }
            }
        });
        ui.separator();

        // C-plane controls for overlay modes
        if matches!(
            self.compare_mode,
            CompareMode::PolarOverlay | CompareMode::CartesianOverlay
        ) {
            ui.horizontal(|ui| {
                ui.label("C-Plane A:");
                if ui
                    .add(
                        DragValue::new(&mut self.compare_c_plane_a)
                            .speed(15.0)
                            .range(0.0..=345.0)
                            .suffix("°"),
                    )
                    .changed()
                {
                    if self.compare_link_sliders {
                        self.compare_c_plane_b = self.compare_c_plane_a;
                    }
                    self.compare_texture_dirty = true;
                }
                ui.label("C-Plane B:");
                if ui
                    .add(
                        DragValue::new(&mut self.compare_c_plane_b)
                            .speed(15.0)
                            .range(0.0..=345.0)
                            .suffix("°"),
                    )
                    .changed()
                {
                    self.compare_texture_dirty = true;
                }
                if ui
                    .checkbox(&mut self.compare_link_sliders, "Link")
                    .changed()
                    && self.compare_link_sliders
                {
                    self.compare_c_plane_b = self.compare_c_plane_a;
                    self.compare_texture_dirty = true;
                }
            });
            ui.separator();
        }

        // Render diagram area
        let available_size = ui.available_size();

        match self.compare_mode {
            CompareMode::PolarOverlay => {
                let size = available_size.min_elem() * 0.8;
                if self.compare_texture_dirty || self.compare_texture.is_none() {
                    let polar_a =
                        PolarDiagram::from_eulumdat_for_plane(&ldt_a, self.compare_c_plane_a);
                    let polar_b =
                        PolarDiagram::from_eulumdat_for_plane(&ldt_b, self.compare_c_plane_b);
                    let theme = self.svg_theme();
                    let svg = PolarDiagram::to_overlay_svg(
                        &polar_a,
                        &polar_b,
                        size as f64,
                        size as f64,
                        &theme,
                        &label_a,
                        &label_b,
                    );
                    self.load_compare_texture(ui, &svg, size as u32, size as u32);
                }
                self.show_compare_texture(ui, available_size);
            }
            CompareMode::CartesianOverlay => {
                let w = available_size.x * 0.9;
                let h = available_size.y * 0.5;
                if self.compare_texture_dirty || self.compare_texture.is_none() {
                    let cart_a = CartesianDiagram::from_eulumdat_for_plane(
                        &ldt_a,
                        self.compare_c_plane_a,
                        w as f64,
                        h as f64,
                    );
                    let cart_b = CartesianDiagram::from_eulumdat_for_plane(
                        &ldt_b,
                        self.compare_c_plane_b,
                        w as f64,
                        h as f64,
                    );
                    let theme = self.svg_theme();
                    let svg = CartesianDiagram::to_overlay_svg(
                        &cart_a, &cart_b, w as f64, h as f64, &theme, &label_a, &label_b,
                    );
                    self.load_compare_texture(ui, &svg, w as u32, h as u32);
                }
                self.show_compare_texture(ui, available_size);
            }
            CompareMode::MetricsOnly => {
                // Just show metrics table below
            }
            _ => {
                // Side-by-side: render two diagrams
                let half_w = (available_size.x * 0.48) as f64;
                let half_h = (available_size.y * 0.5) as f64;
                let diagram_type = match self.compare_mode {
                    CompareMode::PolarSideBySide => DiagramType::Polar,
                    CompareMode::CartesianSideBySide => DiagramType::Cartesian,
                    CompareMode::HeatmapSideBySide => DiagramType::Heatmap,
                    CompareMode::ConeSideBySide => DiagramType::Cone,
                    CompareMode::IsoluxSideBySide => DiagramType::Isolux,
                    CompareMode::IsocandlelaSideBySide => DiagramType::Isocandela,
                    CompareMode::FloodlightSideBySide => DiagramType::Floodlight,
                    _ => DiagramType::Polar,
                };
                let params = DiagramParams {
                    mounting_height: self.mounting_height,
                    tilt_angle: self.tilt_angle,
                    area_size: self.area_size,
                    log_scale: self.log_scale,
                    c_plane: None,
                };

                ui.columns(2, |cols| {
                    // File A
                    cols[0].label(RichText::new(&label_a).small().strong());
                    if let Some(svg_a) = generate_svg_with_height(
                        &ldt_a,
                        diagram_type,
                        half_w,
                        half_h,
                        self.dark_theme,
                        self.mounting_height,
                        &self.locale,
                        &params,
                    ) {
                        if let Ok((pixels, w, h)) =
                            crate::render::render_svg_to_rgba(&svg_a, half_w as u32, half_h as u32)
                        {
                            let image = crate::render::rgba_to_color_image(pixels, w, h);
                            let tex = cols[0].ctx().load_texture(
                                "compare_a",
                                image,
                                egui::TextureOptions::LINEAR,
                            );
                            cols[0].image((tex.id(), tex.size_vec2()));
                        }
                    }

                    // File B
                    cols[1].label(RichText::new(&label_b).small().strong());
                    if let Some(svg_b) = generate_svg_with_height(
                        &ldt_b,
                        diagram_type,
                        half_w,
                        half_h,
                        self.dark_theme,
                        self.mounting_height,
                        &self.locale,
                        &params,
                    ) {
                        if let Ok((pixels, w, h)) =
                            crate::render::render_svg_to_rgba(&svg_b, half_w as u32, half_h as u32)
                        {
                            let image = crate::render::rgba_to_color_image(pixels, w, h);
                            let tex = cols[1].ctx().load_texture(
                                "compare_b",
                                image,
                                egui::TextureOptions::LINEAR,
                            );
                            cols[1].image((tex.id(), tex.size_vec2()));
                        }
                    }
                });
            }
        }

        // Metrics table
        ui.separator();
        let comparison = PhotometricComparison::from_eulumdat_with_locale(
            &ldt_a,
            &ldt_b,
            &label_a,
            &label_b,
            &self.locale,
        );

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "Similarity: {:.0}%",
                    comparison.similarity_score * 100.0
                ))
                .strong(),
            );
            ui.separator();
            if ui.button("Copy CSV").clicked() {
                ui.output_mut(|o| o.copied_text = comparison.to_csv());
            }
        });
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("compare_metrics")
                .num_columns(6)
                .spacing([12.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    // Header
                    ui.label(RichText::new("Metric").small().strong());
                    ui.label(RichText::new(&label_a).small().strong());
                    ui.label(RichText::new(&label_b).small().strong());
                    ui.label(RichText::new("Delta").small().strong());
                    ui.label(RichText::new("%").small().strong());
                    ui.label(RichText::new("Sig.").small().strong());
                    ui.end_row();

                    for metric in &comparison.metrics {
                        let sig_color = match metric.significance {
                            Significance::Negligible => Color32::GRAY,
                            Significance::Minor => Color32::from_rgb(59, 130, 246),
                            Significance::Moderate => Color32::from_rgb(245, 158, 11),
                            Significance::Major => Color32::from_rgb(239, 68, 68),
                        };

                        ui.label(
                            RichText::new(format!("{} ({})", metric.name, metric.unit)).small(),
                        );
                        ui.label(RichText::new(format!("{:.2}", metric.value_a)).small());
                        ui.label(RichText::new(format!("{:.2}", metric.value_b)).small());
                        ui.label(RichText::new(format!("{:.2}", metric.delta)).small());
                        ui.label(
                            RichText::new(format!("{:.1}%", metric.delta_percent))
                                .small()
                                .color(sig_color),
                        );
                        ui.label(
                            RichText::new(format!("{:?}", metric.significance))
                                .small()
                                .color(sig_color),
                        );
                        ui.end_row();
                    }
                });
        });
    }

    /// Load compare texture from SVG
    fn load_compare_texture(&mut self, ui: &mut egui::Ui, svg: &str, w: u32, h: u32) {
        match crate::render::render_svg_to_rgba(svg, w, h) {
            Ok((pixels, pw, ph)) => {
                let image = crate::render::rgba_to_color_image(pixels, pw, ph);
                self.compare_texture = Some(ui.ctx().load_texture(
                    "compare_diagram",
                    image,
                    egui::TextureOptions::LINEAR,
                ));
                self.compare_texture_dirty = false;
            }
            Err(e) => {
                ui.colored_label(Color32::RED, format!("Render error: {}", e));
            }
        }
    }

    /// Show compare texture
    fn show_compare_texture(&self, ui: &mut egui::Ui, available_size: Vec2) {
        if let Some(tex) = &self.compare_texture {
            let texture_size = tex.size_vec2();
            let scale = (available_size.x / texture_size.x).min(available_size.y / texture_size.y);
            let display_size = texture_size * scale;
            // Center the image but only occupy the diagram's actual size (not the whole panel)
            ui.allocate_new_ui(
                egui::UiBuilder::new().max_rect(egui::Rect::from_center_size(
                    ui.available_rect_before_wrap().center(),
                    display_size,
                )),
                |ui| {
                    ui.image((tex.id(), display_size));
                },
            );
        }
    }

    /// Load a compare file
    fn load_compare_file(&mut self, path: PathBuf) {
        let content = match std::fs::read(&path) {
            Ok(bytes) => {
                let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes);
                decoded.into_owned()
            }
            Err(_) => return,
        };

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let result: Result<Eulumdat, String> = match ext.as_str() {
            "xml" => atla::xml::parse(&content)
                .map(|doc| doc.to_eulumdat())
                .map_err(|e| e.to_string()),
            "json" => atla::json::parse(&content)
                .map(|doc| doc.to_eulumdat())
                .map_err(|e| e.to_string()),
            "ies" => eulumdat::IesParser::parse(&content).map_err(|e| e.to_string()),
            _ => Eulumdat::parse(&content)
                .or_else(|_| eulumdat::IesParser::parse(&content))
                .map_err(|e| e.to_string()),
        };

        if let Ok(ldt) = result {
            self.compare_file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("File B")
                .to_string();
            self.compare_ldt = Some(ldt);
            self.compare_texture_dirty = true;
        }
    }

    fn render_3d_butterfly(&mut self, ui: &mut egui::Ui) {
        let size = ui.available_size().min_elem() * 0.95;
        let bg_color = if self.dark_theme {
            Color32::from_rgb(26, 26, 46)
        } else {
            Color32::WHITE
        };

        egui::Frame::none()
            .fill(bg_color)
            .rounding(8.0)
            .show(ui, |ui| {
                let (response, painter) =
                    ui.allocate_painter(egui::vec2(size, size * 0.8), egui::Sense::drag());

                let rect = response.rect;

                // Handle drag for rotation
                if response.dragged() {
                    let delta = response.drag_delta();
                    self.butterfly_3d.rotation_y += delta.x as f64 * 0.01;
                    self.butterfly_3d.rotation_x += delta.y as f64 * 0.01;
                    self.butterfly_3d.rotation_x = self.butterfly_3d.rotation_x.clamp(-1.5, 1.5);
                }

                // Auto-rotate
                if self.butterfly_3d.auto_rotate && !response.dragged() {
                    self.butterfly_3d.rotation_y += 0.005;
                    ui.ctx().request_repaint();
                }

                // Render
                self.butterfly_3d.render(&painter, rect, self.dark_theme);
            });

        // Controls
        ui.horizontal(|ui| {
            if ui
                .button(if self.butterfly_3d.auto_rotate {
                    &self.locale.ui.butterfly.pause
                } else {
                    &self.locale.ui.butterfly.auto
                })
                .clicked()
            {
                self.butterfly_3d.auto_rotate = !self.butterfly_3d.auto_rotate;
            }
            if ui.button(&self.locale.ui.butterfly.reset).clicked() {
                self.butterfly_3d.reset_view();
            }
            ui.label(&self.locale.ui.butterfly.drag_hint);
        });
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
        "wiki-batwing" => "W",
        "wiki-spotlight" => "S",
        "wiki-flood" => "F",
        "atla-grow-light" | "atla-grow-light-rb" => "G",
        "atla-fluorescent" => "T",
        "atla-halogen" => "H",
        "atla-incandescent" => "I",
        "atla-heat-lamp" => "R",
        "atla-uv-blacklight" => "U",
        _ => "o",
    }
}

/// Configure fonts to support CJK (Chinese, Japanese, Korean) and other unicode characters
fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Try to load system CJK fonts
    let cjk_font_paths: &[&str] = if cfg!(target_os = "macos") {
        &[
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/STHeiti Light.ttc",
            "/Library/Fonts/Arial Unicode.ttf",
        ]
    } else if cfg!(target_os = "windows") {
        &[
            "C:\\Windows\\Fonts\\msyh.ttc",   // Microsoft YaHei
            "C:\\Windows\\Fonts\\simhei.ttf", // SimHei
            "C:\\Windows\\Fonts\\simsun.ttc", // SimSun
        ]
    } else {
        // Linux
        &[
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/wenquanyi/wqy-microhei/wqy-microhei.ttc",
        ]
    };

    // Try to load a CJK font
    for path in cjk_font_paths {
        if let Ok(font_data) = std::fs::read(path) {
            fonts
                .font_data
                .insert("cjk_font".to_owned(), egui::FontData::from_owned(font_data));

            // Add to font families as fallback
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push("cjk_font".to_owned());

            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push("cjk_font".to_owned());

            break; // Use first found font
        }
    }

    ctx.set_fonts(fonts);
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

        // Clone locale strings to avoid borrow issues in closures
        let file_label = self.locale.ui.header.file.clone();
        let open_label = self.locale.ui.header.open.clone();
        let templates_label = self.locale.ui.header.templates.clone();
        let export_label = self.locale.ui.tabs.export.clone();
        let export_svg_label = self.locale.ui.file.export_svg.clone();
        let export_ies_label = self.locale.ui.file.export_ies.clone();
        let export_ldt_label = self.locale.ui.file.export_ldt.clone();
        let close_label = self.locale.ui.actions.close.clone();
        let info_panel_label = self.locale.ui.header.title.clone();
        let dark_theme_label = self.locale.ui.theme.dark.clone();
        let language_label = self.locale.ui.language.select.clone();

        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button(&file_label, |ui| {
                    if ui.button(&open_label).clicked() {
                        self.open_file_dialog();
                        ui.close_menu();
                    }

                    ui.menu_button(&templates_label, |ui| {
                        for template in templates::all_templates() {
                            if ui.button(template.name).clicked() {
                                self.load_template(template);
                                ui.close_menu();
                            }
                        }
                    });

                    ui.separator();

                    if self.eulumdat.is_some() {
                        ui.menu_button(&export_label, |ui| {
                            if ui.button(&export_svg_label).clicked() {
                                if let Some(svg) = self.generate_current_svg() {
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
                            if ui.button(&export_ies_label).clicked() {
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
                            if ui.button(&export_ldt_label).clicked() {
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

                    if ui.button(&close_label).clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.show_info, &info_panel_label);
                    ui.separator();
                    if ui
                        .checkbox(&mut self.dark_theme, &dark_theme_label)
                        .changed()
                    {
                        self.texture_dirty = true;
                    }
                    ui.separator();

                    // Language selector
                    ui.menu_button(&language_label, |ui| {
                        for lang in Language::all() {
                            let is_selected = self.language == *lang;
                            let label = if is_selected {
                                format!("✓ {}", lang.native_name())
                            } else {
                                format!("   {}", lang.native_name())
                            };
                            if ui.button(label).clicked() {
                                self.set_language(*lang);
                                ui.close_menu();
                            }
                        }
                    });
                });
            });
        });

        // Main tab bar (only show when we have data)
        if self.eulumdat.is_some() {
            egui::TopBottomPanel::top("main_tab_bar")
                .frame(
                    egui::Frame::none()
                        .fill(Color32::from_rgb(248, 250, 252))
                        .inner_margin(Margin::symmetric(8.0, 4.0)),
                )
                .show(ctx, |ui| {
                    let old_main = self.main_tab;
                    let old_sub = self.sub_tab;
                    render_main_tab_bar(ui, &mut self.main_tab, &mut self.sub_tab);
                    if old_main != self.main_tab || old_sub != self.sub_tab {
                        self.texture_dirty = true;
                    }
                });

            // Sub-tab bar
            egui::TopBottomPanel::top("sub_tab_bar")
                .frame(
                    egui::Frame::none()
                        .fill(Color32::from_rgb(241, 245, 249))
                        .inner_margin(Margin::symmetric(8.0, 4.0)),
                )
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        let old_sub = self.sub_tab;
                        render_sub_tab_bar(ui, self.main_tab, &mut self.sub_tab);
                        if old_sub != self.sub_tab {
                            self.texture_dirty = true;
                        }

                        // Add height controls for relevant tabs
                        if self.sub_tab == SubTab::Cone {
                            ui.separator();
                            ui.label(&self.locale.diagram.cone.mounting_height);
                            if ui
                                .add(
                                    DragValue::new(&mut self.mounting_height)
                                        .speed(0.1)
                                        .range(0.5..=20.0)
                                        .suffix(" m"),
                                )
                                .changed()
                            {
                                self.texture_dirty = true;
                            }
                        } else if self.sub_tab == SubTab::Greenhouse {
                            ui.separator();
                            ui.label(&self.locale.diagram.greenhouse.max_height);
                            if ui
                                .add(
                                    DragValue::new(&mut self.greenhouse_height)
                                        .speed(0.1)
                                        .range(0.5..=6.0)
                                        .suffix(" m"),
                                )
                                .changed()
                            {
                                self.texture_dirty = true;
                            }
                        } else if self.sub_tab == SubTab::Isolux {
                            ui.separator();
                            ui.label("Height:");
                            if ui
                                .add(
                                    DragValue::new(&mut self.mounting_height)
                                        .speed(0.1)
                                        .range(3.0..=30.0)
                                        .suffix(" m"),
                                )
                                .changed()
                            {
                                self.texture_dirty = true;
                            }
                            ui.label("Tilt:");
                            if ui
                                .add(
                                    DragValue::new(&mut self.tilt_angle)
                                        .speed(0.5)
                                        .range(0.0..=80.0)
                                        .suffix("°"),
                                )
                                .changed()
                            {
                                self.texture_dirty = true;
                            }
                            ui.label("Area:");
                            if ui
                                .add(
                                    DragValue::new(&mut self.area_size)
                                        .speed(0.5)
                                        .range(5.0..=100.0)
                                        .suffix(" m"),
                                )
                                .changed()
                            {
                                self.texture_dirty = true;
                            }
                        } else if self.sub_tab == SubTab::Floodlight {
                            ui.separator();
                            if ui.checkbox(&mut self.log_scale, "Log scale").changed() {
                                self.texture_dirty = true;
                            }
                            if let Some(ldt) = &self.eulumdat {
                                let nema = PhotometricCalculations::nema_classification(ldt);
                                ui.separator();
                                ui.label(
                                    RichText::new(&nema.designation)
                                        .small()
                                        .color(Color32::GRAY),
                                );
                            }
                        }

                        // C-plane selector for Polar/Cartesian/Cone
                        if matches!(
                            self.sub_tab,
                            SubTab::Polar | SubTab::Cartesian | SubTab::Cone
                        ) {
                            if let Some(ldt) = &self.eulumdat {
                                if ConeDiagram::has_c_plane_variation(ldt) {
                                    ui.separator();
                                    if let Some(cp) = &mut self.selected_c_plane {
                                        ui.label(RichText::new(format!("C {:.0}°", cp)).small());
                                        let step = if ldt.c_angles.len() > 1 {
                                            ldt.c_angles[1] - ldt.c_angles[0]
                                        } else {
                                            15.0
                                        };
                                        if ui
                                            .add(
                                                DragValue::new(cp)
                                                    .speed(step)
                                                    .range(0.0..=345.0)
                                                    .suffix("°"),
                                            )
                                            .changed()
                                        {
                                            self.texture_dirty = true;
                                        }
                                        if ui.small_button("x").clicked() {
                                            self.selected_c_plane = None;
                                            self.texture_dirty = true;
                                        }
                                    } else if ui.small_button("C-Plane").clicked() {
                                        self.selected_c_plane = Some(0.0);
                                        self.texture_dirty = true;
                                    }
                                }
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

                    if let Some(atla) = &self.atla_doc {
                        if let Some(emitter) = atla.emitters.first() {
                            if let Some(cct) = emitter.cct {
                                ui.separator();
                                ui.label(RichText::new(format!("{:.0}K", cct)).size(11.0));
                            }
                            if let Some(cr) = &emitter.color_rendering {
                                if let Some(ra) = cr.ra {
                                    ui.separator();
                                    ui.label(RichText::new(format!("Ra {:.0}", ra)).size(11.0));
                                }
                            }
                        }
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
            } else if let Some(ldt) = &mut self.eulumdat.clone() {
                match self.sub_tab {
                    // Info tabs
                    SubTab::General => render_general_tab(ui, ldt),
                    SubTab::Dimensions => render_dimensions_tab(ui, ldt),
                    SubTab::LampSets => render_lamps_tab(ui, ldt),
                    SubTab::Optical => render_optical_tab(ui, ldt),

                    // Data tabs
                    SubTab::Intensity => {
                        let mut state = IntensityTabState {
                            show_colors: self.intensity_show_colors,
                        };
                        render_intensity_tab(ui, ldt, &mut state);
                        self.intensity_show_colors = state.show_colors;
                    }

                    // Diagram tabs
                    SubTab::Polar
                    | SubTab::Cartesian
                    | SubTab::BeamAngle
                    | SubTab::Butterfly3D
                    | SubTab::Heatmap
                    | SubTab::Cone
                    | SubTab::Isolux
                    | SubTab::Isocandela
                    | SubTab::Floodlight
                    | SubTab::Spectral
                    | SubTab::Greenhouse
                    | SubTab::BugRating
                    | SubTab::Lcs => {
                        self.render_diagram(ui);
                    }

                    // BIM Parameters
                    SubTab::Bim => {
                        self.render_bim_panel(ui);
                    }

                    // Compare
                    SubTab::ComparePanel => {
                        self.render_compare_panel(ui);
                    }

                    // Validation
                    SubTab::ValidationPanel => render_validation_tab(ui, ldt),
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
