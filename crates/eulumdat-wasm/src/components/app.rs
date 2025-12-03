use eulumdat::{Eulumdat, LampSet, Symmetry, TypeIndicator};
use gloo::file::callbacks::FileReader;
use std::collections::HashMap;
use web_sys::{DragEvent, Event, HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;

use super::bug_rating::BugRating;
use super::butterfly_3d::Butterfly3D;
use super::cartesian_diagram::CartesianDiagram;
use super::data_table::DataTable;
use super::diagram_zoom::DiagramZoom;
use super::file_handler::{download_file, download_ies};
use super::intensity_heatmap::IntensityHeatmap;
use super::polar_diagram::PolarDiagram;
use super::tabs::{DimensionsTab, DirectRatiosTab, GeneralTab, LampSetsTab};
use super::templates::ALL_TEMPLATES;
use super::theme::{ThemeMode, ThemeProvider};
use super::validation_panel::ValidationPanel;

#[derive(Clone, PartialEq)]
pub enum Tab {
    General,
    Dimensions,
    LampSets,
    DirectRatios,
    Intensity,
    Diagram2D,
    Diagram3D,
    Heatmap,
    BugRating,
    Validation,
}

#[derive(Clone, PartialEq)]
pub enum DiagramType {
    Polar,
    Cartesian,
}

#[allow(dead_code)]
pub enum Msg {
    // File operations
    FileLoaded(String, String),
    LoadFile(web_sys::File),
    LoadTemplate(usize),
    NewFile,
    SaveLdt,
    ExportIes,

    // Tab navigation
    SetTab(Tab),

    // Data updates
    UpdateIdentification(String),
    UpdateTypeIndicator(i32),
    UpdateSymmetry(i32),
    UpdateLuminaireName(String),
    UpdateLuminaireNumber(String),
    UpdateMeasurementReport(String),
    UpdateFileName(String),
    UpdateDateUser(String),

    // Dimensions
    UpdateLength(f64),
    UpdateWidth(f64),
    UpdateHeight(f64),
    UpdateLuminousAreaLength(f64),
    UpdateLuminousAreaWidth(f64),
    UpdateHeightC0(f64),
    UpdateHeightC90(f64),
    UpdateHeightC180(f64),
    UpdateHeightC270(f64),

    // Optical
    UpdateDownwardFlux(f64),
    UpdateLightOutputRatio(f64),
    UpdateConversionFactor(f64),
    UpdateTiltAngle(f64),

    // Lamp sets
    SelectLampSet(usize),
    UpdateLampSetNumLamps(i32),
    UpdateLampSetType(String),
    UpdateLampSetFlux(f64),
    UpdateLampSetColorAppearance(String),
    UpdateLampSetColorRendering(String),
    UpdateLampSetWattage(f64),
    AddLampSet,
    RemoveLampSet,

    // Direct ratios
    UpdateDirectRatio(usize, f64),

    // Intensity data
    UpdateIntensity(usize, usize, f64),

    // Drag and drop
    DragOver(DragEvent),
    DragLeave(DragEvent),
    Drop(DragEvent),

    // Diagram type
    SetDiagramType(DiagramType),

    // Theme
    ToggleTheme,
    SetTheme(ThemeMode),

    // Misc
    Noop,
}

pub struct App {
    ldt: Eulumdat,
    current_file: Option<String>,
    active_tab: Tab,
    selected_lamp_set: usize,
    drag_active: bool,
    diagram_type: DiagramType,
    theme_mode: ThemeMode,
    readers: HashMap<String, FileReader>,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        // Detect system color scheme preference
        let theme_mode = Self::detect_system_theme();

        Self {
            ldt: Self::create_default_ldt(),
            current_file: None,
            active_tab: Tab::General,
            selected_lamp_set: 0,
            drag_active: false,
            diagram_type: DiagramType::Polar,
            theme_mode,
            readers: HashMap::new(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::FileLoaded(name, content) => {
                match Eulumdat::parse(&content) {
                    Ok(ldt) => {
                        self.ldt = ldt;
                        self.current_file = Some(name);
                        self.selected_lamp_set = 0;
                    }
                    Err(e) => {
                        gloo::console::error!("Failed to parse file:", e.to_string());
                    }
                }
                true
            }

            Msg::LoadFile(file) => {
                let link = ctx.link().clone();
                let name = file.name();

                let task = gloo_file::callbacks::read_as_text(
                    &gloo_file::File::from(file),
                    move |result| {
                        if let Ok(content) = result {
                            link.send_message(Msg::FileLoaded(name, content));
                        }
                    },
                );

                self.readers.insert("file".to_string(), task);
                false
            }

            Msg::NewFile => {
                self.ldt = Self::create_default_ldt();
                self.current_file = None;
                self.selected_lamp_set = 0;
                true
            }

            Msg::LoadTemplate(idx) => {
                if let Some(template) = ALL_TEMPLATES.get(idx) {
                    match Eulumdat::parse(template.content) {
                        Ok(ldt) => {
                            self.ldt = ldt;
                            self.current_file = Some(format!(
                                "{}.ldt",
                                template.name.to_lowercase().replace(' ', "_")
                            ));
                            self.selected_lamp_set = 0;
                        }
                        Err(e) => {
                            gloo::console::error!("Failed to parse template:", e.to_string());
                        }
                    }
                }
                true
            }

            Msg::SaveLdt => {
                let content = self.ldt.to_ldt();
                let filename = self
                    .current_file
                    .clone()
                    .unwrap_or_else(|| "luminaire.ldt".to_string());
                download_file(&filename, &content, "text/plain");
                false
            }

            Msg::ExportIes => {
                let content = eulumdat::IesExporter::export(&self.ldt);
                let filename = self
                    .current_file
                    .clone()
                    .map(|f| f.replace(".ldt", ".ies"))
                    .unwrap_or_else(|| "luminaire.ies".to_string());
                download_ies(&filename, &content);
                false
            }

            Msg::SetTab(tab) => {
                self.active_tab = tab;
                true
            }

            // Identification updates
            Msg::UpdateIdentification(v) => {
                self.ldt.identification = v;
                true
            }
            Msg::UpdateTypeIndicator(v) => {
                if let Ok(t) = TypeIndicator::from_int(v) {
                    self.ldt.type_indicator = t;
                }
                true
            }
            Msg::UpdateSymmetry(v) => {
                if let Ok(s) = Symmetry::from_int(v) {
                    self.ldt.symmetry = s;
                }
                true
            }
            Msg::UpdateLuminaireName(v) => {
                self.ldt.luminaire_name = v;
                true
            }
            Msg::UpdateLuminaireNumber(v) => {
                self.ldt.luminaire_number = v;
                true
            }
            Msg::UpdateMeasurementReport(v) => {
                self.ldt.measurement_report_number = v;
                true
            }
            Msg::UpdateFileName(v) => {
                self.ldt.file_name = v;
                true
            }
            Msg::UpdateDateUser(v) => {
                self.ldt.date_user = v;
                true
            }

            // Dimension updates
            Msg::UpdateLength(v) => {
                self.ldt.length = v;
                true
            }
            Msg::UpdateWidth(v) => {
                self.ldt.width = v;
                true
            }
            Msg::UpdateHeight(v) => {
                self.ldt.height = v;
                true
            }
            Msg::UpdateLuminousAreaLength(v) => {
                self.ldt.luminous_area_length = v;
                true
            }
            Msg::UpdateLuminousAreaWidth(v) => {
                self.ldt.luminous_area_width = v;
                true
            }
            Msg::UpdateHeightC0(v) => {
                self.ldt.height_c0 = v;
                true
            }
            Msg::UpdateHeightC90(v) => {
                self.ldt.height_c90 = v;
                true
            }
            Msg::UpdateHeightC180(v) => {
                self.ldt.height_c180 = v;
                true
            }
            Msg::UpdateHeightC270(v) => {
                self.ldt.height_c270 = v;
                true
            }

            // Optical updates
            Msg::UpdateDownwardFlux(v) => {
                self.ldt.downward_flux_fraction = v;
                true
            }
            Msg::UpdateLightOutputRatio(v) => {
                self.ldt.light_output_ratio = v;
                true
            }
            Msg::UpdateConversionFactor(v) => {
                self.ldt.conversion_factor = v;
                true
            }
            Msg::UpdateTiltAngle(v) => {
                self.ldt.tilt_angle = v;
                true
            }

            // Lamp set updates
            Msg::SelectLampSet(idx) => {
                self.selected_lamp_set = idx;
                true
            }
            Msg::UpdateLampSetNumLamps(v) => {
                if let Some(ls) = self.ldt.lamp_sets.get_mut(self.selected_lamp_set) {
                    ls.num_lamps = v;
                }
                true
            }
            Msg::UpdateLampSetType(v) => {
                if let Some(ls) = self.ldt.lamp_sets.get_mut(self.selected_lamp_set) {
                    ls.lamp_type = v;
                }
                true
            }
            Msg::UpdateLampSetFlux(v) => {
                if let Some(ls) = self.ldt.lamp_sets.get_mut(self.selected_lamp_set) {
                    ls.total_luminous_flux = v;
                }
                true
            }
            Msg::UpdateLampSetColorAppearance(v) => {
                if let Some(ls) = self.ldt.lamp_sets.get_mut(self.selected_lamp_set) {
                    ls.color_appearance = v;
                }
                true
            }
            Msg::UpdateLampSetColorRendering(v) => {
                if let Some(ls) = self.ldt.lamp_sets.get_mut(self.selected_lamp_set) {
                    ls.color_rendering_group = v;
                }
                true
            }
            Msg::UpdateLampSetWattage(v) => {
                if let Some(ls) = self.ldt.lamp_sets.get_mut(self.selected_lamp_set) {
                    ls.wattage_with_ballast = v;
                }
                true
            }
            Msg::AddLampSet => {
                if self.ldt.lamp_sets.len() < 20 {
                    self.ldt.lamp_sets.push(LampSet::default());
                    self.selected_lamp_set = self.ldt.lamp_sets.len() - 1;
                }
                true
            }
            Msg::RemoveLampSet => {
                if self.ldt.lamp_sets.len() > 1 {
                    self.ldt.lamp_sets.remove(self.selected_lamp_set);
                    if self.selected_lamp_set >= self.ldt.lamp_sets.len() {
                        self.selected_lamp_set = self.ldt.lamp_sets.len() - 1;
                    }
                }
                true
            }

            // Direct ratio updates
            Msg::UpdateDirectRatio(idx, v) => {
                if idx < 10 {
                    self.ldt.direct_ratios[idx] = v;
                }
                true
            }

            // Intensity updates
            Msg::UpdateIntensity(c, g, v) => {
                if let Some(row) = self.ldt.intensities.get_mut(c) {
                    if let Some(cell) = row.get_mut(g) {
                        *cell = v;
                    }
                }
                true
            }

            // Drag and drop
            Msg::DragOver(e) => {
                e.prevent_default();
                self.drag_active = true;
                true
            }
            Msg::DragLeave(_) => {
                self.drag_active = false;
                true
            }
            Msg::Drop(e) => {
                e.prevent_default();
                self.drag_active = false;

                if let Some(data_transfer) = e.data_transfer() {
                    if let Some(files) = data_transfer.files() {
                        if let Some(file) = files.get(0) {
                            ctx.link().send_message(Msg::LoadFile(file));
                        }
                    }
                }
                true
            }

            Msg::SetDiagramType(dt) => {
                self.diagram_type = dt;
                true
            }

            Msg::ToggleTheme => {
                self.theme_mode = self.theme_mode.toggle();
                true
            }

            Msg::SetTheme(mode) => {
                self.theme_mode = mode;
                true
            }

            Msg::Noop => false,
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let theme_class = self.theme_mode.class_name();
        let theme_icon = match self.theme_mode {
            ThemeMode::Light => "🌙", // Moon for switching to dark
            ThemeMode::Dark => "☀️",  // Sun for switching to light
        };

        html! {
            <ThemeProvider mode={self.theme_mode}>
            <div class={classes!("app", theme_class)}>
                // Header
                <header class="header">
                    <h1>{"Eulumdat Editor"}</h1>
                    <div class="header-actions">
                        <button class="btn btn-secondary" onclick={link.callback(|_| Msg::NewFile)}>
                            {"New"}
                        </button>
                        <select
                            class="btn btn-secondary"
                            onchange={link.callback(|e: Event| {
                                let select: HtmlSelectElement = e.target_unchecked_into();
                                let idx = select.selected_index();
                                select.set_selected_index(0); // Reset to "Templates..."
                                if idx > 0 {
                                    Msg::LoadTemplate((idx - 1) as usize)
                                } else {
                                    Msg::Noop
                                }
                            })}
                        >
                            <option value="">{"Templates..."}</option>
                            {for ALL_TEMPLATES.iter().enumerate().map(|(i, t)| {
                                html! {
                                    <option value={i.to_string()} title={t.description}>
                                        {t.name}
                                    </option>
                                }
                            })}
                        </select>
                        <label class="btn btn-secondary">
                            {"Open"}
                            <input
                                type="file"
                                accept=".ldt,.LDT"
                                style="display: none;"
                                onchange={link.callback(|e: Event| {
                                    let input: HtmlInputElement = e.target_unchecked_into();
                                    if let Some(files) = input.files() {
                                        if let Some(file) = files.get(0) {
                                            return Msg::LoadFile(file);
                                        }
                                    }
                                    Msg::Noop
                                })}
                            />
                        </label>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::SaveLdt)}>
                            {"Save LDT"}
                        </button>
                        <button class="btn btn-success" onclick={link.callback(|_| Msg::ExportIes)}>
                            {"Export IES"}
                        </button>
                        <button
                            class="btn btn-secondary theme-toggle"
                            onclick={link.callback(|_| Msg::ToggleTheme)}
                            title={match self.theme_mode {
                                ThemeMode::Light => "Switch to dark mode",
                                ThemeMode::Dark => "Switch to light mode",
                            }}
                        >
                            {theme_icon}
                        </button>
                    </div>
                </header>

                // File info
                if let Some(filename) = &self.current_file {
                    <div class="file-info">
                        {"Current file: "}<strong>{filename}</strong>
                    </div>
                }

                // Drop zone
                <div
                    class={classes!("drop-zone", self.drag_active.then_some("active"))}
                    ondragover={link.callback(Msg::DragOver)}
                    ondragleave={link.callback(Msg::DragLeave)}
                    ondrop={link.callback(Msg::Drop)}
                >
                    <p>{"Drag and drop an LDT file here, or use the Open button above"}</p>
                </div>

                // Main content
                <div class="main-content">
                    // Left panel - Editor
                    <div class="panel">
                        // Tabs
                        <div class="tabs">
                            {self.render_tab(link, Tab::General, "General")}
                            {self.render_tab(link, Tab::Dimensions, "Dimensions")}
                            {self.render_tab(link, Tab::LampSets, "Lamp Sets")}
                            {self.render_tab(link, Tab::DirectRatios, "Direct Ratios")}
                            {self.render_tab(link, Tab::Intensity, "Intensity")}
                            {self.render_tab(link, Tab::Diagram2D, "2D Diagram")}
                            {self.render_tab(link, Tab::Diagram3D, "3D Diagram")}
                            {self.render_tab(link, Tab::Heatmap, "Heatmap")}
                            {self.render_tab(link, Tab::BugRating, "BUG Rating")}
                            {self.render_tab(link, Tab::Validation, "Validation")}
                        </div>

                        // Tab content
                        <div class="tab-content">
                            {match self.active_tab {
                                Tab::General => html! {
                                    <GeneralTab
                                        ldt={self.ldt.clone()}
                                        on_update={link.callback(|msg| msg)}
                                    />
                                },
                                Tab::Dimensions => html! {
                                    <DimensionsTab
                                        ldt={self.ldt.clone()}
                                        on_update={link.callback(|msg| msg)}
                                    />
                                },
                                Tab::LampSets => html! {
                                    <LampSetsTab
                                        ldt={self.ldt.clone()}
                                        selected={self.selected_lamp_set}
                                        on_update={link.callback(|msg| msg)}
                                    />
                                },
                                Tab::DirectRatios => html! {
                                    <DirectRatiosTab
                                        ldt={self.ldt.clone()}
                                        on_update={link.callback(|msg| msg)}
                                    />
                                },
                                Tab::Intensity => html! {
                                    <div class="intensity-tab">
                                        // Data table section only
                                        <div class="intensity-table-section">
                                            <div class="table-header">
                                                <span>{"Luminous Intensity (cd/klm)"}</span>
                                                <span class="table-info">
                                                    {format!("{} C-planes × {} γ-angles", self.ldt.c_angles.len(), self.ldt.g_angles.len())}
                                                </span>
                                            </div>
                                            <DataTable
                                                ldt={self.ldt.clone()}
                                                on_update={link.callback(|msg| msg)}
                                            />
                                        </div>
                                    </div>
                                },
                                Tab::Diagram2D => html! {
                                    <div class="diagram-2d-tab">
                                        <div class="diagram-header">
                                            <span class="diagram-title">{"2D Intensity Distribution"}</span>
                                            <div class="diagram-controls">
                                                <div class="diagram-toggle">
                                                    <button
                                                        class={classes!("btn-toggle", (self.diagram_type == DiagramType::Polar).then_some("active"))}
                                                        onclick={link.callback(|_| Msg::SetDiagramType(DiagramType::Polar))}
                                                    >
                                                        {"Polar"}
                                                    </button>
                                                    <button
                                                        class={classes!("btn-toggle", (self.diagram_type == DiagramType::Cartesian).then_some("active"))}
                                                        onclick={link.callback(|_| Msg::SetDiagramType(DiagramType::Cartesian))}
                                                    >
                                                        {"Cartesian"}
                                                    </button>
                                                </div>
                                                <span class="zoom-hint">{"Scroll to zoom | Drag to pan"}</span>
                                            </div>
                                        </div>
                                        <DiagramZoom>
                                            <div class="diagram-fullwidth">
                                                {match self.diagram_type {
                                                    DiagramType::Polar => html! { <PolarDiagram ldt={self.ldt.clone()} /> },
                                                    DiagramType::Cartesian => html! { <CartesianDiagram ldt={self.ldt.clone()} /> },
                                                }}
                                            </div>
                                        </DiagramZoom>
                                    </div>
                                },
                                Tab::Diagram3D => html! {
                                    <div class="diagram-3d-tab">
                                        <div class="diagram-header">
                                            <span class="diagram-title">{"3D Butterfly Diagram"}</span>
                                            <span class="text-muted">{"Drag to rotate | Auto-rotates on load"}</span>
                                        </div>
                                        <div class="diagram-fullwidth">
                                            <Butterfly3D ldt={self.ldt.clone()} />
                                        </div>
                                    </div>
                                },
                                Tab::Heatmap => html! {
                                    <div class="heatmap-tab">
                                        <div class="diagram-header">
                                            <span class="diagram-title">{"Intensity Heatmap"}</span>
                                            <span class="text-muted">{"Candela distribution across C-planes and gamma angles"}</span>
                                        </div>
                                        <div class="diagram-fullwidth">
                                            <IntensityHeatmap ldt={self.ldt.clone()} />
                                        </div>
                                    </div>
                                },
                                Tab::BugRating => html! {
                                    <div class="bug-rating-tab">
                                        <div class="diagram-header">
                                            <span class="diagram-title">{"BUG Rating Analysis"}</span>
                                            <span class="text-muted">{"IES TM-15-11 Backlight, Uplight, Glare"}</span>
                                        </div>
                                        <div class="diagram-fullwidth">
                                            <BugRating ldt={self.ldt.clone()} />
                                        </div>
                                    </div>
                                },
                                Tab::Validation => html! {
                                    <div class="validation-tab">
                                        <div class="validation-section">
                                            <h3>{"Validation Results"}</h3>
                                            <ValidationPanel ldt={self.ldt.clone()} />
                                        </div>
                                        <div class="summary-section">
                                            <h3>{"Summary"}</h3>
                                            <div class="info-grid-wide">
                                                <div class="info-item">
                                                    <div class="info-label">{"Total Flux"}</div>
                                                    <div class="info-value">{format!("{:.0} lm", self.ldt.total_luminous_flux())}</div>
                                                </div>
                                                <div class="info-item">
                                                    <div class="info-label">{"Total Wattage"}</div>
                                                    <div class="info-value">{format!("{:.1} W", self.ldt.total_wattage())}</div>
                                                </div>
                                                <div class="info-item">
                                                    <div class="info-label">{"Efficacy"}</div>
                                                    <div class="info-value">{format!("{:.1} lm/W", self.ldt.luminous_efficacy())}</div>
                                                </div>
                                                <div class="info-item">
                                                    <div class="info-label">{"Max Intensity"}</div>
                                                    <div class="info-value">{format!("{:.1} cd/klm", self.ldt.max_intensity())}</div>
                                                </div>
                                                <div class="info-item">
                                                    <div class="info-label">{"Symmetry"}</div>
                                                    <div class="info-value">{self.ldt.symmetry.description()}</div>
                                                </div>
                                                <div class="info-item">
                                                    <div class="info-label">{"C-Planes"}</div>
                                                    <div class="info-value">{format!("{} (Mc={})", self.ldt.num_c_planes, self.ldt.actual_c_planes())}</div>
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                },
                            }}
                        </div>
                    </div>
                </div>
            </div>
            </ThemeProvider>
        }
    }
}

impl App {
    /// Detect system color scheme preference
    fn detect_system_theme() -> ThemeMode {
        super::theme::detect_system_theme()
    }

    fn render_tab(&self, link: &yew::html::Scope<Self>, tab: Tab, label: &str) -> Html {
        let is_active = self.active_tab == tab;
        let tab_clone = tab.clone();
        html! {
            <button
                class={classes!("tab", is_active.then_some("active"))}
                onclick={link.callback(move |_| Msg::SetTab(tab_clone.clone()))}
            >
                {label}
            </button>
        }
    }

    fn create_default_ldt() -> Eulumdat {
        let mut ldt = Eulumdat::new();
        ldt.identification = "New Luminaire".to_string();
        ldt.type_indicator = TypeIndicator::PointSourceSymmetric;
        ldt.symmetry = Symmetry::VerticalAxis;
        ldt.num_c_planes = 1;
        ldt.c_plane_distance = 0.0;
        ldt.num_g_planes = 19;
        ldt.g_plane_distance = 5.0;
        ldt.light_output_ratio = 100.0;
        ldt.conversion_factor = 1.0;

        // Default lamp set
        ldt.lamp_sets.push(LampSet {
            num_lamps: 1,
            lamp_type: "LED".to_string(),
            total_luminous_flux: 1000.0,
            color_appearance: "3000K".to_string(),
            color_rendering_group: "80".to_string(),
            wattage_with_ballast: 10.0,
        });

        // Default C angles
        ldt.c_angles = vec![0.0];

        // Default G angles (0 to 90 in 5° steps)
        ldt.g_angles = (0..19).map(|i| i as f64 * 5.0).collect();

        // Default intensities (simple downlight pattern)
        ldt.intensities = vec![vec![
            100.0, 99.0, 96.0, 91.0, 84.0, 75.0, 64.0, 51.0, 36.0, 25.0, 16.0, 9.0, 4.0, 2.0, 1.0,
            0.5, 0.2, 0.1, 0.0,
        ]];

        ldt
    }
}
