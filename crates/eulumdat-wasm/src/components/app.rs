use atla::{Emitter, IntensityDistribution, LuminaireOpticalData};
use eulumdat::{Eulumdat, IesParser, UnitSystem};
use eulumdat_typst::{ReportGenerator, ReportOptions};
use leptos::ev;
use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

// JS binding for typst PDF compilation
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = compileTypstToPdf, catch)]
    async fn compile_typst_to_pdf_js(source: &str) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_name = getTemplateContent, catch)]
    async fn get_template_content_js(id: &str) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_name = isTemplatesLoaded)]
    fn is_templates_loaded() -> bool;
}

/// Compile Typst source to PDF using the WASM typst compiler.
/// SVGs are already embedded inline in the source.
async fn compile_typst_to_pdf(typst_source: &str) -> Result<Vec<u8>, String> {
    match compile_typst_to_pdf_js(typst_source).await {
        Ok(js_val) => {
            let array = js_sys::Uint8Array::new(&js_val);
            Ok(array.to_vec())
        }
        Err(e) => {
            let msg = e.as_string().unwrap_or_else(|| "Unknown error".to_string());
            Err(msg)
        }
    }
}

use crate::i18n::{use_locale, LanguageSelectorCompact};
use eulumdat_i18n::Locale;

use super::beam_angle_diagram::BeamAngleDiagram;
use super::bevy_scene::BevySceneViewer;
use super::bim_panel::{has_bim_data, BimPanel, BimPanelEmpty};
use super::bug_rating::BugRating;
use super::butterfly_3d::Butterfly3D;
use super::cartesian_diagram::CartesianDiagram;
use super::compare_panel::ComparePanel;
use super::cone_diagram::{ConeDiagramView, ConeIlluminanceTableView};
use super::data_table::DataTable;
use super::diagram_zoom::DiagramZoom;
use super::floodlight_cartesian::FloodlightCartesian;
use super::greenhouse_diagram::GreenhouseDiagramView;
use super::intensity_heatmap::IntensityHeatmap;
use super::isocandela_diagram::IsocandelaDiagramView;
use super::isolux_footprint::IsoluxFootprint;
use super::lcs_classification::LcsClassification;
use super::maps_designer::MapsDesigner;
use super::polar_diagram::PolarDiagram;
use super::spectral_diagram::SpectralDiagramView;
use super::tabs::{DimensionsTab, DirectRatiosTab, GeneralTab, LampSetsTab};
use super::templates::ALL_TEMPLATES;
use super::theme::{ThemeMode, ThemeProvider};
use super::validation_panel::ValidationPanel;

// Storage keys - must match eulumdat-bevy/src/viewer/wasm_sync.rs
const LDT_STORAGE_KEY: &str = "eulumdat_current_ldt";
const LDT_TIMESTAMP_KEY: &str = "eulumdat_ldt_timestamp";

/// All features (PDF/Typst export, Maps Designer) are enabled by default.
fn is_export_enabled() -> bool {
    true
}

/// Log color data (CCT/CRI) for debugging spectral synthesis
fn log_color_data(filename: &str, doc: &LuminaireOpticalData) {
    web_sys::console::group_collapsed_1(&format!("[Color Data] {}", filename).into());

    if doc.emitters.is_empty() {
        web_sys::console::warn_1(&"No emitters found".into());
    } else {
        for (i, emitter) in doc.emitters.iter().enumerate() {
            let has_spectral = emitter.spectral_distribution.is_some();
            let cct = emitter.cct;
            let cri = emitter.color_rendering.as_ref().and_then(|cr| cr.ra);

            let status = if has_spectral {
                "Direct SPD data available".to_string()
            } else if cct.is_some() {
                format!(
                    "Can synthesize from CCT={}K{}",
                    cct.unwrap() as i32,
                    cri.map(|c| format!(", CRI={}", c as i32))
                        .unwrap_or_default()
                )
            } else {
                "No color data - will show sample spectrum".to_string()
            };

            web_sys::console::log_1(&format!("Emitter {}: {}", i, status).into());

            // Show raw values for debugging
            if cct.is_none() && !has_spectral {
                web_sys::console::log_1(
                    &"  Tip: CCT parsed from LDT 'color_appearance' field (e.g., '3000K', 'tw/6500', 'warm white')".into()
                );
                web_sys::console::log_1(
                    &"  Tip: CRI parsed from LDT 'color_rendering_group' field (e.g., '1B/86', 'Ra>90', '80')".into()
                );
            }
        }
    }

    web_sys::console::group_end();
}

/// Log color data with raw LDT values
fn log_color_data_from_ldt(filename: &str, ldt: &Eulumdat, doc: &LuminaireOpticalData) {
    web_sys::console::group_1(&format!("[Color Data] {}", filename).into());

    for (i, lamp_set) in ldt.lamp_sets.iter().enumerate() {
        web_sys::console::log_1(&format!("Lamp Set {}:", i).into());
        web_sys::console::log_1(
            &format!("  Raw color_appearance: '{}'", lamp_set.color_appearance).into(),
        );
        web_sys::console::log_1(
            &format!(
                "  Raw color_rendering_group: '{}'",
                lamp_set.color_rendering_group
            )
            .into(),
        );
    }

    if let Some(emitter) = doc.emitters.first() {
        let cct = emitter.cct;
        let cri = emitter.color_rendering.as_ref().and_then(|cr| cr.ra);
        let has_spectral = emitter.spectral_distribution.is_some();

        if has_spectral {
            web_sys::console::log_1(&"  → Direct SPD data available".into());
        } else if let Some(cct_val) = cct {
            web_sys::console::log_1(
                &format!(
                    "  → Parsed: CCT={}K, CRI={:?}",
                    cct_val as i32,
                    cri.map(|c| c as i32)
                )
                .into(),
            );
            web_sys::console::log_1(&"  → Can synthesize spectrum!".into());
        } else {
            web_sys::console::warn_1(&"  → Could not parse CCT - showing sample spectrum".into());
            web_sys::console::log_1(&"  Supported formats:".into());
            web_sys::console::log_1(
                &"    CCT: '3000K', '4000', 'tw/6500', 'ww/2700', 'warm white', 'daylight'".into(),
            );
            web_sys::console::log_1(&"    CRI: '1B/86', 'Ra>90', '80', '1A', '1B', '2A'".into());
        }
    }

    web_sys::console::group_end();
}

/// Main tab groups
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum MainTab {
    #[default]
    Info,
    Data,
    Diagrams,
    Analysis,
    Validation,
    Compare,
    Bim,
    Scene3D,
    MapsDesigner,
}

/// Sub-tabs within each main tab group
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    // Info group
    #[default]
    General,
    Dimensions,
    LampSets,
    DirectRatios,
    // Data group
    Intensity,
    // Diagrams group
    Diagram2D,
    Diagram3D,
    Heatmap,
    Cone,
    FloodlightVH,
    FloodlightIsolux,
    FloodlightIsocandela,
    // Analysis group
    Spectral,
    Greenhouse,
    BugRating,
    Lcs,
    // Validation group (single tab, no sub-tabs)
    ValidationTab,
    // Compare group (single tab, no sub-tabs)
    CompareTab,
    // BIM group (single tab, no sub-tabs)
    BimTab,
    // Scene 3D group (single tab, no sub-tabs)
    Scene3DTab,
    // Maps Designer group (single tab, no sub-tabs)
    MapsDesignerTab,
}

impl Tab {
    /// Get the main tab group this sub-tab belongs to
    pub fn main_tab(&self) -> MainTab {
        match self {
            Tab::General | Tab::Dimensions | Tab::LampSets | Tab::DirectRatios => MainTab::Info,
            Tab::Intensity => MainTab::Data,
            Tab::Diagram2D
            | Tab::Diagram3D
            | Tab::Heatmap
            | Tab::Cone
            | Tab::FloodlightVH
            | Tab::FloodlightIsolux
            | Tab::FloodlightIsocandela => MainTab::Diagrams,
            Tab::Spectral | Tab::Greenhouse | Tab::BugRating | Tab::Lcs => MainTab::Analysis,
            Tab::ValidationTab => MainTab::Validation,
            Tab::CompareTab => MainTab::Compare,
            Tab::BimTab => MainTab::Bim,
            Tab::Scene3DTab => MainTab::Scene3D,
            Tab::MapsDesignerTab => MainTab::MapsDesigner,
        }
    }

    /// Get the default sub-tab for a main tab
    pub fn default_for_main(main: MainTab) -> Tab {
        match main {
            MainTab::Info => Tab::General,
            MainTab::Data => Tab::Intensity,
            MainTab::Diagrams => Tab::Diagram2D,
            MainTab::Analysis => Tab::Spectral,
            MainTab::Validation => Tab::ValidationTab,
            MainTab::Compare => Tab::CompareTab,
            MainTab::Bim => Tab::BimTab,
            MainTab::Scene3D => Tab::Scene3DTab,
            MainTab::MapsDesigner => Tab::MapsDesignerTab,
        }
    }

    /// Get all sub-tabs for a main tab
    pub fn tabs_for_main(main: MainTab) -> &'static [Tab] {
        match main {
            MainTab::Info => &[
                Tab::General,
                Tab::Dimensions,
                Tab::LampSets,
                Tab::DirectRatios,
            ],
            MainTab::Data => &[Tab::Intensity],
            MainTab::Diagrams => &[
                Tab::Diagram2D,
                Tab::Diagram3D,
                Tab::Heatmap,
                Tab::Cone,
                Tab::FloodlightVH,
                Tab::FloodlightIsolux,
                Tab::FloodlightIsocandela,
            ],
            MainTab::Analysis => &[Tab::Spectral, Tab::Greenhouse, Tab::BugRating, Tab::Lcs],
            MainTab::Validation => &[Tab::ValidationTab],
            MainTab::Compare => &[Tab::CompareTab],
            MainTab::Bim => &[Tab::BimTab],
            MainTab::Scene3D => &[Tab::Scene3DTab],
            MainTab::MapsDesigner => &[Tab::MapsDesignerTab],
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum DiagramType {
    #[default]
    Polar,
    Cartesian,
    BeamAngle,
}

/// Create a default ATLA document for new files
fn create_default_atla() -> LuminaireOpticalData {
    let mut doc = LuminaireOpticalData::new();
    doc.header.manufacturer = Some("New Luminaire".to_string());
    doc.header.description = Some("Default luminaire".to_string());

    // Create default emitter with intensity distribution
    let g_angles: Vec<f64> = (0..19).map(|i| i as f64 * 5.0).collect();
    let intensities = vec![vec![
        100.0, 99.0, 96.0, 91.0, 84.0, 75.0, 64.0, 51.0, 36.0, 25.0, 16.0, 9.0, 4.0, 2.0, 1.0, 0.5,
        0.2, 0.1, 0.0,
    ]];

    doc.emitters.push(Emitter {
        quantity: 1,
        description: Some("LED".to_string()),
        rated_lumens: Some(1000.0),
        input_watts: Some(10.0),
        cct: Some(3000.0),
        color_rendering: Some(atla::ColorRendering {
            ra: Some(80.0),
            ..Default::default()
        }),
        intensity_distribution: Some(IntensityDistribution {
            horizontal_angles: vec![0.0],
            vertical_angles: g_angles,
            intensities,
            ..Default::default()
        }),
        ..Default::default()
    });

    doc
}

fn detect_system_theme() -> ThemeMode {
    super::theme::detect_system_theme()
}

/// Signal type for the unit system context.
pub type UnitSystemSignal = (ReadSignal<UnitSystem>, WriteSignal<UnitSystem>);

/// Get the unit system signal from context.
pub fn use_unit_system() -> ReadSignal<UnitSystem> {
    let (unit_system, _) = use_context::<UnitSystemSignal>().expect("UnitSystem context not found");
    unit_system
}

/// Load unit system from localStorage, defaulting to Metric.
fn load_unit_system() -> UnitSystem {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(val)) = storage.get_item("eulumdat_unit_system") {
                if val == "imperial" {
                    return UnitSystem::Imperial;
                }
            }
        }
    }
    UnitSystem::Metric
}

/// Save unit system to localStorage.
fn save_unit_system(units: UnitSystem) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let val = match units {
                UnitSystem::Metric => "metric",
                UnitSystem::Imperial => "imperial",
            };
            let _ = storage.set_item("eulumdat_unit_system", val);
        }
    }
}

use super::templates::Template;

/// Load a template file (content is lazily fetched from templates WASM module)
fn load_template(
    template: &Template,
    set_atla_doc: WriteSignal<LuminaireOpticalData>,
    set_current_file: WriteSignal<Option<String>>,
    set_selected_lamp_set: WriteSignal<usize>,
    set_templates_loading: WriteSignal<bool>,
    rotate_c_planes: ReadSignal<bool>,
) {
    use super::templates::TemplateFormat;

    let ext = match template.format {
        TemplateFormat::Ldt => "ldt",
        TemplateFormat::IesLm63 => "ies",
        TemplateFormat::AtlaXml => "xml",
        TemplateFormat::AtlaJson => "json",
    };
    let filename = format!(
        "{}.{}",
        template
            .name
            .to_lowercase()
            .replace(' ', "_")
            .replace("(", "")
            .replace(")", ""),
        ext
    );

    let id = template.id.to_string();
    let format = template.format;

    set_templates_loading.set(true);

    wasm_bindgen_futures::spawn_local(async move {
        match get_template_content_js(&id).await {
            Ok(js_val) => {
                if let Some(content) = js_val.as_string() {
                    match format {
                        TemplateFormat::Ldt => {
                            if let Ok(ldt) = Eulumdat::parse(&content) {
                                let doc = LuminaireOpticalData::from_eulumdat(&ldt);
                                set_atla_doc.set(doc);
                                set_current_file.set(Some(filename));
                                set_selected_lamp_set.set(0);
                            }
                        }
                        TemplateFormat::IesLm63 => {
                            let opts = eulumdat::IesImportOptions {
                                rotate_c_planes: if rotate_c_planes.get_untracked() {
                                    90.0
                                } else {
                                    0.0
                                },
                            };
                            if let Ok(ldt) = IesParser::parse_with_options(&content, &opts) {
                                let doc = LuminaireOpticalData::from_eulumdat(&ldt);
                                set_atla_doc.set(doc);
                                set_current_file.set(Some(filename));
                                set_selected_lamp_set.set(0);
                            }
                        }
                        TemplateFormat::AtlaXml => {
                            if let Ok(doc) = atla::xml::parse(&content) {
                                set_atla_doc.set(doc);
                                set_current_file.set(Some(filename));
                                set_selected_lamp_set.set(0);
                            }
                        }
                        TemplateFormat::AtlaJson => {
                            if let Ok(doc) = atla::json::parse(&content) {
                                set_atla_doc.set(doc);
                                set_current_file.set(Some(filename));
                                set_selected_lamp_set.set(0);
                            }
                        }
                    }
                } else {
                    web_sys::console::error_1(
                        &format!("Template '{}' returned non-string value", id).into(),
                    );
                }
            }
            Err(e) => {
                let msg = e.as_string().unwrap_or_else(|| "Unknown error".to_string());
                web_sys::console::error_1(
                    &format!("Failed to load template '{}': {}", id, msg).into(),
                );
            }
        }
        set_templates_loading.set(false);
    });
}

/// Get localized template name
fn get_template_name(template_name: &str, locale: &Locale) -> String {
    match template_name {
        "Downlight" => locale.ui.template.downlight.clone(),
        "Projector" => locale.ui.template.projector.clone(),
        "Linear Luminaire" => locale.ui.template.linear.clone(),
        "Fluorescent Luminaire" => locale.ui.template.fluorescent.clone(),
        "Road Luminaire" => locale.ui.template.road.clone(),
        "Floor Uplight" => locale.ui.template.uplight.clone(),
        "_atla Fluorescent (XML)" => locale.ui.template.atla_fluorescent_xml.clone(),
        "_atla Fluorescent (JSON)" => locale.ui.template.atla_fluorescent_json.clone(),
        "_atla Grow Light (Full Spectrum)" => locale.ui.template.atla_grow_light_fs.clone(),
        "_atla Grow Light (Red/Blue)" => locale.ui.template.atla_grow_light_rb.clone(),
        "_atla Halogen Lamp (IR)" => locale.ui.template.halogen.clone(),
        "_atla Incandescent (IR)" => locale.ui.template.incandescent.clone(),
        "_atla Heat Lamp (High IR)" => locale.ui.template.heat_lamp.clone(),
        "_atla UV Blacklight (UV-A)" => locale.ui.template.uv_blacklight.clone(),
        _ => template_name.to_string(),
    }
}

/// Replace file extension with a new one
fn replace_extension(filename: &str, new_ext: &str) -> String {
    if let Some(dot_pos) = filename.rfind('.') {
        format!("{}.{}", &filename[..dot_pos], new_ext)
    } else {
        format!("{}.{}", filename, new_ext)
    }
}

/// Save ATLA document to localStorage (as LDT string for Bevy compatibility)
fn save_to_storage(doc: &LuminaireOpticalData) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            // Save as LDT for Bevy 3D viewer compatibility
            let ldt = doc.to_eulumdat();
            let ldt_string = ldt.to_ldt();
            let _ = storage.set_item(LDT_STORAGE_KEY, &ldt_string);
            let timestamp = js_sys::Date::now().to_string();
            let _ = storage.set_item(LDT_TIMESTAMP_KEY, &timestamp);

            // Save as JSON for Maps Designer (with intensities for calculations)
            if let Ok(json) = serde_json::to_string(&serde_json::json!({
                "intensities": ldt.intensities,
                "c_angles": ldt.c_angles,
                "g_angles": ldt.g_angles,
                "lumens": ldt.total_luminous_flux(),
                "luminaire_name": ldt.luminaire_name,
            })) {
                let _ = storage.set_item("eulumdat_current_json", &json);
            }
        }
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Primary state: ATLA document (source of truth)
    let (atla_doc, set_atla_doc) = signal(create_default_atla());

    // Secondary state: Eulumdat for existing components
    // This is derived from ATLA and syncs back to ATLA on changes
    let (ldt, set_ldt_internal) = signal(create_default_atla().to_eulumdat());

    // Sync ATLA → Eulumdat whenever ATLA changes
    Effect::new(move |_| {
        set_ldt_internal.set(atla_doc.get().to_eulumdat());
    });

    // Custom setter that syncs Eulumdat changes back to ATLA
    let set_ldt = set_ldt_internal;

    // Also sync Eulumdat → ATLA (for when child components modify ldt)
    Effect::new(move |_| {
        let current_ldt = ldt.get();
        let current_atla_as_ldt = atla_doc.get_untracked().to_eulumdat();
        // Only update if actually different (avoid infinite loop)
        if current_ldt.to_ldt() != current_atla_as_ldt.to_ldt() {
            set_atla_doc.set(LuminaireOpticalData::from_eulumdat(&current_ldt));
        }
    });

    let (current_file, set_current_file) = signal::<Option<String>>(None);
    let (active_tab, set_active_tab) = signal(Tab::default());
    let (selected_lamp_set, set_selected_lamp_set) = signal(0_usize);
    let (templates_loading, set_templates_loading) = signal(false);

    // Compare panel: File B state lives here so it persists across tab switches
    let (compare_ldt_b, set_compare_ldt_b) = signal::<Option<Eulumdat>>(None);
    let (compare_label_b, set_compare_label_b) = signal::<Option<String>>(None);

    // Derive the active main tab from the active sub-tab
    let active_main_tab = Memo::new(move |_| active_tab.get().main_tab());

    // i18n locale for UI strings
    let locale = use_locale();
    let (drag_active, set_drag_active) = signal(false);
    let (diagram_type, set_diagram_type) = signal(DiagramType::default());
    let (mounting_height, set_mounting_height) = signal(3.0_f64); // Default 3m mounting height for cone diagram
    let (selected_c_plane, set_selected_c_plane) = signal(None::<f64>); // None = overall, Some(angle) = specific C-plane
    let (greenhouse_height, set_greenhouse_height) = signal(2.0_f64); // Default 2m for greenhouse PPFD
    let (theme_mode, set_theme_mode) = signal(detect_system_theme());
    let (unit_system, set_unit_system) = signal(load_unit_system());
    // Save unit system to localStorage on change
    Effect::new(move |_| {
        save_unit_system(unit_system.get());
    });
    provide_context((unit_system, set_unit_system));
    let (show_about, set_show_about) = signal(false);

    // C-plane rotation for IES↔LDT axis correction (EU C0‖length vs US C0⊥length)
    let (rotate_c_planes, set_rotate_c_planes) = signal(false);

    // Check if PDF/Typst export is enabled (via secret URL)
    let export_enabled = is_export_enabled();

    // Save to localStorage whenever ATLA doc changes
    Effect::new(move |_| {
        save_to_storage(&atla_doc.get());
    });

    // File loading helper - ALL formats convert to ATLA (lossless)
    let load_file_content = move |name: String, content: String| {
        let lower_name = name.to_lowercase();
        let is_ies = lower_name.ends_with(".ies");
        let is_atla_xml = lower_name.ends_with(".xml");
        let is_atla_json = lower_name.ends_with(".json");
        let is_ldt = lower_name.ends_with(".ldt");
        let is_spdx = lower_name.ends_with(".spdx");

        // Parse to ATLA format (source of truth)
        if is_spdx {
            // SPDX (IES TM-27-14) → ATLA (spectral only, no photometric data)
            match atla::spdx::parse(&content) {
                Ok(spdx_data) => {
                    // Log warnings about missing data
                    let warnings = atla::spdx::get_warnings(&spdx_data);
                    for warning in &warnings {
                        web_sys::console::warn_1(&format!("SPDX: {}", warning).into());
                    }

                    let doc = atla::spdx::to_atla(&spdx_data);
                    set_atla_doc.set(doc);
                    set_current_file.set(Some(name));
                    set_selected_lamp_set.set(0);

                    // Show first warning to user
                    if let Some(first_warning) = warnings.first() {
                        web_sys::console::info_1(
                            &format!("Loaded SPDX file (spectral data only): {}", first_warning)
                                .into(),
                        );
                    }
                }
                Err(e) => {
                    web_sys::console::error_1(&format!("Failed to parse SPDX: {}", e).into());
                }
            }
        } else if is_ies {
            // IES → Eulumdat → ATLA (with optional C-plane rotation)
            let opts = eulumdat::IesImportOptions {
                rotate_c_planes: if rotate_c_planes.get_untracked() {
                    90.0
                } else {
                    0.0
                },
            };
            match IesParser::parse_with_options(&content, &opts) {
                Ok(ldt) => {
                    let doc = LuminaireOpticalData::from_eulumdat(&ldt);
                    log_color_data_from_ldt(&name, &ldt, &doc);
                    set_atla_doc.set(doc);
                    set_current_file.set(Some(name));
                    set_selected_lamp_set.set(0);
                }
                Err(e) => {
                    web_sys::console::error_1(&format!("Failed to parse IES: {}", e).into());
                }
            }
        } else if is_atla_xml {
            // ATLA XML → ATLA (direct, no conversion)
            match atla::xml::parse(&content) {
                Ok(doc) => {
                    log_color_data(&name, &doc);
                    set_atla_doc.set(doc);
                    set_current_file.set(Some(name));
                    set_selected_lamp_set.set(0);
                }
                Err(e) => {
                    web_sys::console::error_1(&format!("Failed to parse ATLA XML: {}", e).into());
                }
            }
        } else if is_atla_json {
            // ATLA JSON → ATLA (direct, no conversion)
            match atla::json::parse(&content) {
                Ok(doc) => {
                    log_color_data(&name, &doc);
                    set_atla_doc.set(doc);
                    set_current_file.set(Some(name));
                    set_selected_lamp_set.set(0);
                }
                Err(e) => {
                    web_sys::console::error_1(&format!("Failed to parse ATLA JSON: {}", e).into());
                }
            }
        } else if is_ldt {
            // LDT → Eulumdat → ATLA (with raw value logging)
            match Eulumdat::parse(&content) {
                Ok(ldt) => {
                    let doc = LuminaireOpticalData::from_eulumdat(&ldt);
                    log_color_data_from_ldt(&name, &ldt, &doc);
                    set_atla_doc.set(doc);
                    set_current_file.set(Some(name));
                    set_selected_lamp_set.set(0);
                }
                Err(e) => {
                    web_sys::console::error_1(&format!("Failed to parse LDT: {}", e).into());
                }
            }
        } else {
            web_sys::console::error_1(&"Unknown file format".into());
        }
    };

    // Handlers
    let on_new_file = move |_| {
        set_atla_doc.set(create_default_atla());
        set_current_file.set(None);
        set_selected_lamp_set.set(0);
    };

    let on_save_ldt = move |_| {
        // Export from ATLA → LDT
        let content = atla_doc.get().to_eulumdat().to_ldt();
        let filename = current_file
            .get()
            .map(|f| replace_extension(&f, "ldt"))
            .unwrap_or_else(|| "luminaire.ldt".to_string());
        super::file_handler::download_file(&filename, &content, "text/plain");
    };

    let on_export_ies = move |_| {
        // Export from ATLA → Eulumdat → IES
        let opts = eulumdat::IesExportOptions {
            rotate_c_planes: if rotate_c_planes.get() { -90.0 } else { 0.0 },
            ..Default::default()
        };
        let content =
            eulumdat::IesExporter::export_with_options(&atla_doc.get().to_eulumdat(), &opts);
        let filename = current_file
            .get()
            .map(|f| replace_extension(&f, "ies"))
            .unwrap_or_else(|| "luminaire.ies".to_string());
        super::file_handler::download_ies(&filename, &content);
    };

    let on_export_atla_xml = move |_| {
        // Export directly from ATLA (no conversion needed!)
        match atla::xml::write(&atla_doc.get()) {
            Ok(content) => {
                let filename = current_file
                    .get()
                    .map(|f| replace_extension(&f, "xml"))
                    .unwrap_or_else(|| "luminaire.xml".to_string());
                super::file_handler::download_atla_xml(&filename, &content);
            }
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to export ATLA XML: {}", e).into());
            }
        }
    };

    let on_export_atla_json = move |_| {
        // Export directly from ATLA (no conversion needed!)
        match atla::json::write(&atla_doc.get()) {
            Ok(content) => {
                let filename = current_file
                    .get()
                    .map(|f| replace_extension(&f, "json"))
                    .unwrap_or_else(|| "luminaire.json".to_string());
                super::file_handler::download_atla_json(&filename, &content);
            }
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to export ATLA JSON: {}", e).into());
            }
        }
    };

    let on_export_report_typ = move |_| {
        // Generate Typst report from the current LDT
        let ldt_val = ldt.get();
        let generator = ReportGenerator::new(&ldt_val);
        let content = generator.generate_typst(&ReportOptions::default());
        let filename = current_file
            .get()
            .map(|f| replace_extension(&f, "typ"))
            .unwrap_or_else(|| "photometric_report.typ".to_string());
        super::file_handler::download_file(&filename, &content, "text/plain");
    };

    // Signal to track PDF export loading state
    let (pdf_exporting, set_pdf_exporting) = signal(false);

    let on_export_report_pdf = move |_| {
        // Generate Typst report with inline SVGs and compile to PDF via WASM
        let ldt_val = ldt.get();
        let (typst_source, _) =
            eulumdat_typst::generate_typst_with_files(&ldt_val, &ReportOptions::default().sections);

        let filename = current_file
            .get()
            .map(|f| replace_extension(&f, "pdf"))
            .unwrap_or_else(|| "photometric_report.pdf".to_string());

        set_pdf_exporting.set(true);

        // Compile using typst WASM
        wasm_bindgen_futures::spawn_local(async move {
            match compile_typst_to_pdf(&typst_source).await {
                Ok(pdf_bytes) => {
                    super::file_handler::download_bytes(&filename, &pdf_bytes, "application/pdf");
                }
                Err(e) => {
                    web_sys::console::error_1(&format!("PDF export failed: {}", e).into());
                    // Fallback: download .typ file
                    web_sys::window()
                        .unwrap()
                        .alert_with_message(&format!(
                            "PDF export failed: {}. Downloading .typ file instead.",
                            e
                        ))
                        .ok();
                    let typ_filename = filename.replace(".pdf", ".typ");
                    super::file_handler::download_file(&typ_filename, &typst_source, "text/plain");
                }
            }
            set_pdf_exporting.set(false);
        });
    };

    // Helper to generate SVG for the current diagram
    let generate_current_svg = move || -> Option<(String, String)> {
        let current_tab = active_tab.get();
        let ldt_val = ldt.get();
        let theme = eulumdat::diagram::SvgTheme::light(); // Use light theme for export

        match current_tab {
            Tab::Diagram2D => {
                // Use the current diagram type (Polar, Cartesian, or BeamAngle)
                match diagram_type.get() {
                    DiagramType::Polar => {
                        let polar = eulumdat::diagram::PolarDiagram::from_eulumdat(&ldt_val);
                        let summary = eulumdat::PhotometricSummary::from_eulumdat(&ldt_val);
                        let svg = polar.to_svg_with_summary(600.0, 600.0, &theme, &summary);
                        Some((svg, "polar_diagram.svg".to_string()))
                    }
                    DiagramType::Cartesian => {
                        let cartesian = eulumdat::diagram::CartesianDiagram::from_eulumdat(
                            &ldt_val, 600.0, 450.0, 8,
                        );
                        let summary = eulumdat::PhotometricSummary::from_eulumdat(&ldt_val);
                        let svg = cartesian.to_svg_with_summary(600.0, 450.0, &theme, &summary);
                        Some((svg, "cartesian_diagram.svg".to_string()))
                    }
                    DiagramType::BeamAngle => {
                        let polar = eulumdat::diagram::PolarDiagram::from_eulumdat(&ldt_val);
                        let analysis =
                            eulumdat::PhotometricCalculations::beam_field_analysis(&ldt_val);
                        let show_both = analysis.is_batwing;
                        let svg = polar.to_svg_with_beam_field_angles(
                            600.0, 600.0, &theme, &analysis, show_both,
                        );
                        Some((svg, "beam_angle_diagram.svg".to_string()))
                    }
                }
            }
            Tab::Diagram3D => {
                // Butterfly/3D diagram
                let butterfly = eulumdat::diagram::ButterflyDiagram::from_eulumdat(
                    &ldt_val, 600.0, 500.0, 60.0,
                );
                let svg = butterfly.to_svg(600.0, 500.0, &theme);
                Some((svg, "butterfly_3d_diagram.svg".to_string()))
            }
            Tab::Heatmap => {
                let heatmap =
                    eulumdat::diagram::HeatmapDiagram::from_eulumdat(&ldt_val, 700.0, 500.0);
                let summary = eulumdat::PhotometricSummary::from_eulumdat(&ldt_val);
                let svg = heatmap.to_svg_with_summary(700.0, 500.0, &theme, &summary);
                Some((svg, "intensity_heatmap.svg".to_string()))
            }
            Tab::Cone => {
                let height = mounting_height.get();
                let cone = match selected_c_plane.get() {
                    Some(c) => {
                        eulumdat::diagram::ConeDiagram::from_eulumdat_for_plane(&ldt_val, height, c)
                    }
                    None => eulumdat::diagram::ConeDiagram::from_eulumdat(&ldt_val, height),
                };
                let loc = locale.get();
                let labels = eulumdat::diagram::ConeDiagramLabels {
                    beam_angle: loc.diagram.cone.beam_angle.clone(),
                    field_angle: loc.diagram.cone.field_angle.clone(),
                    mounting_height: loc.diagram.cone.mounting_height.clone(),
                    beam_diameter: loc.diagram.cone.beam_diameter.clone(),
                    field_diameter: loc.diagram.cone.field_diameter.clone(),
                    intensity_50: loc.diagram.cone.intensity_50.clone(),
                    intensity_10: loc.diagram.cone.intensity_10.clone(),
                    floor: loc.diagram.cone.floor.clone(),
                    meter: loc.diagram.cone.meter.clone(),
                    c_plane_label: loc.diagram.cone.c_plane.clone(),
                };
                let svg = cone.to_svg_with_units(600.0, 450.0, &theme, &labels, unit_system.get());
                Some((svg, "cone_diagram.svg".to_string()))
            }
            Tab::Spectral => {
                // Spectral diagram from ATLA doc
                let doc = atla_doc.get();
                let atla_theme = atla::spectral::SpectralTheme::light();

                // Try to get spectral data
                if let Some(spd) = doc
                    .emitters
                    .iter()
                    .filter_map(|e| e.spectral_distribution.as_ref())
                    .next()
                {
                    let diagram = atla::spectral::SpectralDiagram::from_spectral(spd);
                    let svg = diagram.to_svg(700.0, 400.0, &atla_theme);
                    Some((svg, "spectral_diagram.svg".to_string()))
                } else if let Some(emitter) = doc.emitters.first() {
                    if let Some(cct) = emitter.cct {
                        let cri = emitter.color_rendering.as_ref().and_then(|cr| cr.ra);
                        let spd = atla::spectral::synthesize_spectrum(cct, cri);
                        let diagram = atla::spectral::SpectralDiagram::from_spectral(&spd);
                        let svg = diagram.to_svg(700.0, 400.0, &atla_theme);
                        Some((svg, "spectral_diagram.svg".to_string()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Tab::Greenhouse => {
                let doc = atla_doc.get();
                let height = greenhouse_height.get();
                let diagram =
                    atla::greenhouse::GreenhouseDiagram::from_atla_with_height(&doc, height);
                let gh_theme = atla::greenhouse::GreenhouseTheme::light();
                let svg = diagram.to_svg(600.0, 450.0, &gh_theme);
                Some((svg, "greenhouse_ppfd.svg".to_string()))
            }
            Tab::BugRating => {
                let diagram = eulumdat::BugDiagram::from_eulumdat(&ldt_val);
                let svg = diagram.to_svg_with_details(600.0, 400.0, &theme);
                Some((svg, "bug_rating.svg".to_string()))
            }
            Tab::Lcs => {
                let diagram = eulumdat::BugDiagram::from_eulumdat(&ldt_val);
                let svg = diagram.to_lcs_svg(600.0, 400.0, &theme);
                Some((svg, "lcs_classification.svg".to_string()))
            }
            Tab::FloodlightVH => {
                let y_scale = eulumdat::diagram::YScale::Linear;
                let diagram = eulumdat::diagram::FloodlightCartesianDiagram::from_eulumdat(
                    &ldt_val, 600.0, 400.0, y_scale,
                );
                let svg = diagram.to_svg(600.0, 400.0, &theme);
                Some((svg, "floodlight_vh.svg".to_string()))
            }
            Tab::FloodlightIsolux => {
                let params = eulumdat::diagram::IsoluxParams::default();
                let diagram =
                    eulumdat::diagram::IsoluxDiagram::from_eulumdat(&ldt_val, 600.0, 500.0, params);
                let svg = diagram.to_svg(600.0, 500.0, &theme);
                Some((svg, "isolux_footprint.svg".to_string()))
            }
            Tab::FloodlightIsocandela => {
                let diagram =
                    eulumdat::diagram::IsocandelaDiagram::from_eulumdat(&ldt_val, 600.0, 500.0);
                let svg = diagram.to_svg(600.0, 500.0, &theme);
                Some((svg, "isocandela_contour.svg".to_string()))
            }
            // Non-diagram tabs - no SVG export available
            Tab::General
            | Tab::Dimensions
            | Tab::LampSets
            | Tab::DirectRatios
            | Tab::Intensity
            | Tab::ValidationTab
            | Tab::CompareTab
            | Tab::BimTab
            | Tab::Scene3DTab
            | Tab::MapsDesignerTab => None,
        }
    };

    let on_export_svg = move |_| {
        if let Some((svg_content, default_filename)) = generate_current_svg() {
            let filename = current_file
                .get()
                .map(|f| replace_extension(&f, "svg"))
                .unwrap_or(default_filename);
            super::file_handler::download_svg(&filename, &svg_content);
        } else {
            // No diagram to export - show alert or log
            web_sys::console::warn_1(&"No diagram to export on this tab".into());
        }
    };

    let on_toggle_theme = move |_| {
        set_theme_mode.update(|m| *m = m.toggle());
    };

    let on_file_input = move |ev: ev::Event| {
        let input: HtmlInputElement = ev.target().unwrap().unchecked_into();
        if let Some(files) = input.files() {
            if let Some(file) = files.get(0) {
                let name = file.name();
                let load_content = load_file_content;
                wasm_bindgen_futures::spawn_local(async move {
                    let text = gloo_file::futures::read_as_text(&file.into()).await;
                    if let Ok(content) = text {
                        load_content(name, content);
                    }
                });
            }
        }
    };

    let on_dragover = move |ev: ev::DragEvent| {
        ev.prevent_default();
        set_drag_active.set(true);
    };

    let on_dragleave = move |_: ev::DragEvent| {
        set_drag_active.set(false);
    };

    let on_drop = move |ev: ev::DragEvent| {
        ev.prevent_default();
        set_drag_active.set(false);

        if let Some(data_transfer) = ev.data_transfer() {
            if let Some(files) = data_transfer.files() {
                if let Some(file) = files.get(0) {
                    let name = file.name();
                    let load_content = load_file_content;
                    wasm_bindgen_futures::spawn_local(async move {
                        let text = gloo_file::futures::read_as_text(&file.into()).await;
                        if let Ok(content) = text {
                            load_content(name, content);
                        }
                    });
                }
            }
        }
    };

    // Note: Child components use the `ldt` Memo which derives from atla_doc.
    // When they call set_ldt, it converts back to ATLA internally.

    view! {
        <ThemeProvider mode=theme_mode>
            <div class=move || format!("app {}", theme_mode.get().class_name()) role="application" aria-label="Eulumdat Editor">
                // Skip to main content link for keyboard users
                <a href="#main-content" class="skip-link">"Skip to main content"</a>

                // Header with navigation landmark
                <header class="header" role="banner">
                    <h1>{move || locale.get().ui.header.title.clone()}</h1>
                    <div class="header-actions">
                        // File menu dropdown
                        <div class="file-menu">
                            <button class="btn btn-secondary file-menu-toggle">
                                {move || locale.get().ui.header.file.clone()}
                                <span class="dropdown-arrow">" ▾"</span>
                            </button>
                            <div class="file-menu-dropdown">
                                <button class="menu-item" on:click=on_new_file>
                                    {move || locale.get().ui.header.new.clone()}
                                </button>
                                <label class="menu-item" role="button" tabindex="0">
                                    {move || locale.get().ui.header.open.clone()}
                                    <input
                                        type="file"
                                        accept=".ldt,.LDT,.ies,.IES,.xml,.XML,.json,.JSON,.spdx,.SPDX"
                                        style="display: none;"
                                        on:change=on_file_input
                                        aria-label="Open LDT, IES, SPDX, XML, or JSON file"
                                    />
                                </label>
                                <div class="menu-divider"></div>
                                <label class="menu-item checkbox-item" title="Rotate C-planes ±90° when importing/exporting IES (fixes EU↔US axis orientation)">
                                    <input
                                        type="checkbox"
                                        prop:checked=move || rotate_c_planes.get()
                                        on:change=move |ev| {
                                            let checked = ev.target().unwrap().unchecked_into::<HtmlInputElement>().checked();
                                            set_rotate_c_planes.set(checked);
                                        }
                                    />
                                    " Rotate C0 ±90° (IES)"
                                </label>
                                <div class="menu-divider"></div>
                                <button class="menu-item" on:click=on_save_ldt>
                                    {move || locale.get().ui.header.save_ldt.clone()}
                                </button>
                                <button class="menu-item" on:click=on_export_ies>
                                    {move || locale.get().ui.header.export_ies.clone()}
                                </button>
                                <button class="menu-item" on:click=on_export_atla_xml>
                                    {move || locale.get().ui.header.atla_xml.clone()}
                                </button>
                                <button class="menu-item" on:click=on_export_atla_json>
                                    {move || locale.get().ui.header.atla_json.clone()}
                                </button>
                                <div class="menu-divider"></div>
                                <button
                                    class=move || {
                                        if generate_current_svg().is_some() { "menu-item" } else { "menu-item disabled" }
                                    }
                                    on:click=on_export_svg
                                    disabled=move || generate_current_svg().is_none()
                                >
                                    {move || {
                                        let tab = active_tab.get();
                                        match tab {
                                            Tab::Diagram2D => match diagram_type.get() {
                                                DiagramType::Polar => "Export SVG (Polar)",
                                                DiagramType::Cartesian => "Export SVG (Cartesian)",
                                                DiagramType::BeamAngle => "Export SVG (Beam Angle)",
                                            },
                                            Tab::Diagram3D => "Export SVG (3D Butterfly)",
                                            Tab::Heatmap => "Export SVG (Heatmap)",
                                            Tab::Cone => "Export SVG (Cone)",
                                            Tab::Spectral => "Export SVG (Spectral)",
                                            Tab::Greenhouse => "Export SVG (Greenhouse)",
                                            Tab::BugRating => "Export SVG (BUG Rating)",
                                            Tab::Lcs => "Export SVG (LCS)",
                                            Tab::FloodlightVH => "Export SVG (V-H Diagram)",
                                            Tab::FloodlightIsolux => "Export SVG (Isolux)",
                                            Tab::FloodlightIsocandela => "Export SVG (Isocandela)",
                                            _ => "Export SVG",
                                        }
                                    }}
                                </button>
                                // PDF/Typst export - only shown on secret URL
                                {move || export_enabled.then(|| view! {
                                    <button
                                        class="menu-item"
                                        on:click=on_export_report_pdf
                                        disabled=move || pdf_exporting.get()
                                        title="Export as PDF report (compiles in browser)"
                                    >
                                        {move || if pdf_exporting.get() { "Generating PDF..." } else { "Export Report (.pdf)" }}
                                    </button>
                                    <button
                                        class="menu-item"
                                        on:click=on_export_report_typ
                                        title="Export as Typst source file (.typ) - compile with 'typst compile report.typ'"
                                    >
                                        "Export Report (.typ)"
                                    </button>
                                })}
                                <div class="menu-divider"></div>
                                <a
                                    class="menu-item"
                                    href="https://github.com/holg/eulumdat-rs"
                                    target="_blank"
                                    rel="noopener noreferrer"
                                >
                                    "GitHub"
                                </a>
                                <button
                                    class="menu-item"
                                    on:click=move |_| set_show_about.set(true)
                                >
                                    "About"
                                </button>
                            </div>
                        </div>
                        // Templates dropdown (separate)
                        <div class="file-menu">
                            <button class="btn btn-secondary file-menu-toggle">
                                {move || locale.get().ui.header.templates.clone()}
                                <span class="dropdown-arrow">" ▾"</span>
                            </button>
                            <div class="file-menu-dropdown templates-dropdown">
                                {move || {
                                    let l = locale.get();
                                    ALL_TEMPLATES.iter().enumerate().map(|(i, t)| {
                                        let name = get_template_name(t.name, &l);
                                        let idx = i;
                                        view! {
                                            <button
                                                class="menu-item"
                                                title=t.description
                                                on:click=move |_| {
                                                    if let Some(template) = ALL_TEMPLATES.get(idx) {
                                                        load_template(template, set_atla_doc, set_current_file, set_selected_lamp_set, set_templates_loading, rotate_c_planes);
                                                    }
                                                }
                                            >
                                                {name}
                                            </button>
                                        }
                                    }).collect_view()
                                }}
                            </div>
                        </div>
                        // Templates loading indicator
                        {move || templates_loading.get().then(|| view! {
                            <span class="templates-loading">"Loading..."</span>
                        })}
                        // Settings
                        <button
                            class="btn btn-secondary theme-toggle"
                            on:click=on_toggle_theme
                            title=move || match theme_mode.get() {
                                ThemeMode::Light => locale.get().ui.header.switch_to_dark.clone(),
                                ThemeMode::Dark => locale.get().ui.header.switch_to_light.clone(),
                            }
                        >
                            {move || match theme_mode.get() {
                                ThemeMode::Light => "🌙",
                                ThemeMode::Dark => "☀️",
                            }}
                        </button>
                        <button
                            class="btn btn-secondary"
                            on:click=move |_| {
                                set_unit_system.update(|u| {
                                    *u = match u {
                                        UnitSystem::Metric => UnitSystem::Imperial,
                                        UnitSystem::Imperial => UnitSystem::Metric,
                                    };
                                });
                            }
                            title=move || match unit_system.get() {
                                UnitSystem::Metric => "Switch to Imperial (ft, fc, in)",
                                UnitSystem::Imperial => "Switch to Metric (m, lx, mm)",
                            }
                        >
                            {move || match unit_system.get() {
                                UnitSystem::Metric => "SI",
                                UnitSystem::Imperial => "IMP",
                            }}
                        </button>
                        <LanguageSelectorCompact />
                    </div>
                </header>

                // File info
                {move || current_file.get().map(|filename| view! {
                    <div class="file-info">
                        {locale.get().ui.dropzone.current_file.clone()}" "<strong>{filename}</strong>
                    </div>
                })}

                // Drop zone
                <div
                    class=move || format!("drop-zone{}", if drag_active.get() { " active" } else { "" })
                    on:dragover=on_dragover
                    on:dragleave=on_dragleave
                    on:drop=on_drop
                >
                    <p>{move || locale.get().ui.dropzone.text.clone()}</p>
                </div>

                // Main content
                <main id="main-content" class="main-content" role="main" aria-label="Editor content">
                    <div class="panel">
                        // Main Tabs - navigation landmark
                        <nav class="tabs main-tabs" role="tablist" aria-label="Editor sections">
                            <button
                                class=move || format!("tab{}", if active_main_tab.get() == MainTab::Info { " active" } else { "" })
                                on:click=move |_| set_active_tab.set(Tab::default_for_main(MainTab::Info))
                            >
                                {move || locale.get().ui.tabs.info.clone()}
                            </button>
                            <button
                                class=move || format!("tab{}", if active_main_tab.get() == MainTab::Data { " active" } else { "" })
                                on:click=move |_| set_active_tab.set(Tab::default_for_main(MainTab::Data))
                            >
                                {move || locale.get().ui.tabs.data.clone()}
                            </button>
                            <button
                                class=move || format!("tab{}", if active_main_tab.get() == MainTab::Diagrams { " active" } else { "" })
                                on:click=move |_| set_active_tab.set(Tab::default_for_main(MainTab::Diagrams))
                            >
                                {move || locale.get().ui.tabs.diagrams.clone()}
                            </button>
                            <button
                                class=move || format!("tab{}", if active_main_tab.get() == MainTab::Analysis { " active" } else { "" })
                                on:click=move |_| set_active_tab.set(Tab::default_for_main(MainTab::Analysis))
                            >
                                {move || locale.get().ui.tabs.analysis.clone()}
                            </button>
                            <button
                                class=move || format!("tab{}", if active_main_tab.get() == MainTab::Validation { " active" } else { "" })
                                on:click=move |_| set_active_tab.set(Tab::default_for_main(MainTab::Validation))
                            >
                                {move || locale.get().ui.tabs.validation.clone()}
                            </button>
                            <button
                                class=move || format!("tab{}", if active_main_tab.get() == MainTab::Compare { " active" } else { "" })
                                on:click=move |_| set_active_tab.set(Tab::default_for_main(MainTab::Compare))
                            >
                                {move || locale.get().ui.tabs.compare.clone()}
                            </button>
                            // BIM tab - only shown when file has BIM data
                            {move || {
                                if has_bim_data(&atla_doc.get()) {
                                    Some(view! {
                                        <button
                                            class=move || format!("tab{}", if active_main_tab.get() == MainTab::Bim { " active" } else { "" })
                                            on:click=move |_| set_active_tab.set(Tab::default_for_main(MainTab::Bim))
                                            title="TM-32-24 BIM Parameters"
                                        >
                                            "BIM"
                                        </button>
                                    })
                                } else {
                                    None
                                }
                            }}
                            <button
                                class=move || format!("tab{}", if active_main_tab.get() == MainTab::Scene3D { " active" } else { "" })
                                on:click=move |_| set_active_tab.set(Tab::default_for_main(MainTab::Scene3D))
                            >
                                {move || locale.get().ui.tabs.scene_3d.clone()}
                            </button>
                            // Maps Designer - only shown on secret URL (requires Google Maps API key)
                            {move || export_enabled.then(|| view! {
                                <button
                                    class=move || format!("tab{}", if active_main_tab.get() == MainTab::MapsDesigner { " active" } else { "" })
                                    on:click=move |_| set_active_tab.set(Tab::default_for_main(MainTab::MapsDesigner))
                                >
                                    "🗺️ Maps"
                                </button>
                            })}
                        </nav>

                        // Sub-tabs (shown only when main tab has multiple sub-tabs)
                        {move || {
                            let main = active_main_tab.get();
                            let sub_tabs = Tab::tabs_for_main(main);
                            if sub_tabs.len() > 1 {
                                Some(view! {
                                    <div class="tabs sub-tabs">
                                        {sub_tabs.iter().map(|&tab| {
                                            let label = match tab {
                                                Tab::General => locale.get().ui.tabs.general.clone(),
                                                Tab::Dimensions => locale.get().ui.tabs.dimensions.clone(),
                                                Tab::LampSets => locale.get().ui.tabs.lamp_sets.clone(),
                                                Tab::DirectRatios => locale.get().ui.tabs.direct_ratios.clone(),
                                                Tab::Intensity => locale.get().ui.tabs.intensity.clone(),
                                                Tab::Diagram2D => locale.get().ui.tabs.diagram_2d.clone(),
                                                Tab::Diagram3D => locale.get().ui.tabs.diagram_3d.clone(),
                                                Tab::Heatmap => locale.get().ui.tabs.heatmap.clone(),
                                                Tab::Cone => locale.get().ui.tabs.cone.clone(),
                                                Tab::Spectral => locale.get().ui.tabs.spectral.clone(),
                                                Tab::Greenhouse => locale.get().ui.tabs.greenhouse.clone(),
                                                Tab::BugRating => locale.get().ui.tabs.bug_rating.clone(),
                                                Tab::Lcs => locale.get().ui.tabs.lcs.clone(),
                                                Tab::FloodlightVH => locale.get().ui.tabs.floodlight_vh.clone(),
                                                Tab::FloodlightIsolux => locale.get().ui.tabs.floodlight_isolux.clone(),
                                                Tab::FloodlightIsocandela => locale.get().ui.tabs.floodlight_isocandela.clone(),
                                                Tab::ValidationTab => locale.get().ui.tabs.validation.clone(),
                                                Tab::CompareTab => locale.get().ui.tabs.compare.clone(),
                                                Tab::BimTab => "BIM".to_string(),
                                                Tab::Scene3DTab => locale.get().ui.tabs.scene_3d.clone(),
                                                Tab::MapsDesignerTab => "Maps Designer".to_string(),
                                            };
                                            view! {
                                                <button
                                                    class=move || format!("tab{}", if active_tab.get() == tab { " active" } else { "" })
                                                    on:click=move |_| set_active_tab.set(tab)
                                                >
                                                    {label}
                                                </button>
                                            }
                                        }).collect_view()}
                                    </div>
                                })
                            } else {
                                None
                            }
                        }}

                        // Tab content
                        <div class="tab-content">
                            {move || match active_tab.get() {
                                Tab::General => view! {
                                    <GeneralTab ldt=ldt set_ldt=set_ldt />
                                }.into_any(),
                                Tab::Dimensions => view! {
                                    <DimensionsTab ldt=ldt set_ldt=set_ldt />
                                }.into_any(),
                                Tab::LampSets => view! {
                                    <LampSetsTab ldt=ldt set_ldt=set_ldt selected=selected_lamp_set set_selected=set_selected_lamp_set />
                                }.into_any(),
                                Tab::DirectRatios => view! {
                                    <DirectRatiosTab ldt=ldt set_ldt=set_ldt />
                                }.into_any(),
                                Tab::Intensity => view! {
                                    <div class="intensity-tab">
                                        <div class="intensity-table-section">
                                            <div class="table-header">
                                                <span>{move || locale.get().ui.intensity.title.clone()}</span>
                                                <span class="table-info">
                                                    {move || {
                                                        let l = ldt.get();
                                                        let template = locale.get().ui.intensity.table_info.clone();
                                                        template.replace("{c_planes}", &l.c_angles.len().to_string())
                                                            .replace("{g_angles}", &l.g_angles.len().to_string())
                                                    }}
                                                </span>
                                            </div>
                                            <DataTable ldt=ldt set_ldt=set_ldt />
                                        </div>
                                    </div>
                                }.into_any(),
                                Tab::Diagram2D => view! {
                                    <div class="diagram-2d-tab">
                                        <div class="diagram-header">
                                            <span class="diagram-title">{move || locale.get().ui.diagram.title_2d.clone()}</span>
                                            <div class="diagram-controls">
                                                <div class="diagram-toggle">
                                                    <button
                                                        class=move || format!("btn-toggle{}", if diagram_type.get() == DiagramType::Polar { " active" } else { "" })
                                                        on:click=move |_| set_diagram_type.set(DiagramType::Polar)
                                                    >
                                                        {move || locale.get().ui.diagram.polar.clone()}
                                                    </button>
                                                    <button
                                                        class=move || format!("btn-toggle{}", if diagram_type.get() == DiagramType::Cartesian { " active" } else { "" })
                                                        on:click=move |_| set_diagram_type.set(DiagramType::Cartesian)
                                                    >
                                                        {move || locale.get().ui.diagram.cartesian.clone()}
                                                    </button>
                                                    <button
                                                        class=move || format!("btn-toggle{}", if diagram_type.get() == DiagramType::BeamAngle { " active" } else { "" })
                                                        on:click=move |_| set_diagram_type.set(DiagramType::BeamAngle)
                                                        title="IES vs CIE beam angle comparison (Wikipedia style)"
                                                    >
                                                        "Beam Angle"
                                                    </button>
                                                </div>
                                                <span class="zoom-hint">{move || locale.get().ui.diagram.zoom_hint.clone()}</span>
                                            </div>
                                        </div>
                                        <DiagramZoom>
                                            <div class="diagram-fullwidth">
                                                {move || match diagram_type.get() {
                                                    DiagramType::Polar => view! { <PolarDiagram ldt=ldt /> }.into_any(),
                                                    DiagramType::Cartesian => view! { <CartesianDiagram ldt=ldt /> }.into_any(),
                                                    DiagramType::BeamAngle => view! { <BeamAngleDiagram ldt=ldt /> }.into_any(),
                                                }}
                                            </div>
                                        </DiagramZoom>
                                    </div>
                                }.into_any(),
                                Tab::Diagram3D => view! {
                                    <div class="diagram-3d-tab">
                                        <div class="diagram-header">
                                            <span class="diagram-title">{move || locale.get().ui.diagram.title_3d.clone()}</span>
                                            <span class="text-muted">{move || locale.get().ui.diagram.rotate_hint.clone()}</span>
                                        </div>
                                        <DiagramZoom>
                                            <div class="diagram-fullwidth">
                                                <Butterfly3D ldt=ldt />
                                            </div>
                                        </DiagramZoom>
                                    </div>
                                }.into_any(),
                                Tab::Heatmap => view! {
                                    <div class="heatmap-tab">
                                        <div class="diagram-header">
                                            <span class="diagram-title">{move || locale.get().ui.diagram.title_heatmap.clone()}</span>
                                            <span class="text-muted">{move || locale.get().ui.diagram.zoom_hint.clone()}</span>
                                        </div>
                                        <DiagramZoom>
                                            <div class="diagram-fullwidth">
                                                <IntensityHeatmap ldt=ldt />
                                            </div>
                                        </DiagramZoom>
                                    </div>
                                }.into_any(),
                                Tab::Cone => {
                                    // Get expanded C-plane angles for the slider
                                    let c_angles = Memo::new(move |_| {
                                        let l = ldt.get();
                                        eulumdat::SymmetryHandler::expand_c_angles(&l)
                                    });
                                    let has_variation = Memo::new(move |_| {
                                        eulumdat::diagram::ConeDiagram::has_c_plane_variation(&ldt.get())
                                    });

                                    view! {
                                        <div class="cone-tab">
                                            <div class="diagram-header">
                                                <span class="diagram-title">{move || locale.get().diagram.title.cone.clone()}</span>
                                                <div class="mounting-height-control">
                                                    <label>{move || locale.get().diagram.cone.mounting_height.clone()}</label>
                                                    <input
                                                        type="range"
                                                        min="1"
                                                        max="15"
                                                        step="0.5"
                                                        prop:value=move || mounting_height.get()
                                                        on:input=move |ev| {
                                                            if let Ok(value) = event_target_value(&ev).parse::<f64>() {
                                                                set_mounting_height.set(value);
                                                            }
                                                        }
                                                    />
                                                    <span class="value-display">
                                                        {move || unit_system.get().format_distance(mounting_height.get())}
                                                    </span>
                                                </div>
                                            </div>
                                            // C-plane selector
                                            {move || {
                                                if has_variation.get() {
                                                    let angles = c_angles.get();
                                                    let max_idx = if angles.is_empty() { 0 } else { angles.len() - 1 };
                                                    view! {
                                                        <div class="c-plane-control">
                                                            <label>{move || locale.get().diagram.cone.c_plane.clone()}</label>
                                                            <input
                                                                type="range"
                                                                min="0"
                                                                max=max_idx.to_string()
                                                                step="1"
                                                                prop:value=move || {
                                                                    match selected_c_plane.get() {
                                                                        Some(cp) => {
                                                                            let a = c_angles.get();
                                                                            a.iter().position(|&x| (x - cp).abs() < 0.01)
                                                                                .unwrap_or(0)
                                                                                .to_string()
                                                                        }
                                                                        None => "0".to_string(),
                                                                    }
                                                                }
                                                                on:input=move |ev| {
                                                                    if let Ok(idx) = event_target_value(&ev).parse::<usize>() {
                                                                        let a = c_angles.get();
                                                                        if let Some(&angle) = a.get(idx) {
                                                                            set_selected_c_plane.set(Some(angle));
                                                                        }
                                                                    }
                                                                }
                                                            />
                                                            <span class="value-display">
                                                                {move || {
                                                                    match selected_c_plane.get() {
                                                                        Some(cp) => format!("C {cp:.0}°"),
                                                                        None => locale.get().diagram.cone.all_planes.clone(),
                                                                    }
                                                                }}
                                                            </span>
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <div class="c-plane-control symmetric-note">
                                                            <span class="text-muted">{move || locale.get().diagram.cone.symmetric_note.clone()}</span>
                                                        </div>
                                                    }.into_any()
                                                }
                                            }}
                                            <DiagramZoom>
                                                <div class="diagram-fullwidth">
                                                    <ConeDiagramView ldt=ldt mounting_height=mounting_height c_plane=selected_c_plane />
                                                </div>
                                            </DiagramZoom>
                                            <ConeIlluminanceTableView ldt=ldt mounting_height=mounting_height c_plane=selected_c_plane />
                                        </div>
                                    }.into_any()
                                },
                                Tab::Spectral => {
                                    let is_dark = Memo::new(move |_| theme_mode.get() == ThemeMode::Dark);
                                    view! {
                                        <div class="spectral-tab">
                                            <div class="diagram-header">
                                                <span class="diagram-title">{move || locale.get().ui.diagram.title_spectral.clone()}</span>
                                                <span class="text-muted">{move || locale.get().ui.spectral.subtitle.clone()}</span>
                                            </div>
                                            <DiagramZoom>
                                                <div class="diagram-fullwidth">
                                                    <SpectralDiagramView atla_doc=atla_doc dark=is_dark />
                                                </div>
                                            </DiagramZoom>
                                        </div>
                                    }.into_any()
                                },
                                Tab::Greenhouse => {
                                    let is_dark = Memo::new(move |_| theme_mode.get() == ThemeMode::Dark);
                                    view! {
                                        <div class="greenhouse-tab">
                                            <div class="diagram-header">
                                                <span class="diagram-title">{move || locale.get().ui.diagram.title_greenhouse.clone()}</span>
                                                <div class="mounting-height-control">
                                                    <label>{move || locale.get().diagram.greenhouse.max_height.clone()}</label>
                                                    <input
                                                        type="range"
                                                        min="0.5"
                                                        max="6"
                                                        step="0.5"
                                                        prop:value=move || greenhouse_height.get()
                                                        on:input=move |ev| {
                                                            if let Ok(value) = event_target_value(&ev).parse::<f64>() {
                                                                set_greenhouse_height.set(value);
                                                            }
                                                        }
                                                    />
                                                    <span class="value-display">
                                                        {move || unit_system.get().format_distance(greenhouse_height.get())}
                                                    </span>
                                                </div>
                                            </div>
                                            <DiagramZoom>
                                                <div class="diagram-fullwidth">
                                                    <GreenhouseDiagramView atla_doc=atla_doc dark=is_dark max_height=greenhouse_height />
                                                </div>
                                            </DiagramZoom>
                                        </div>
                                    }.into_any()
                                },
                                Tab::BugRating => view! {
                                    <div class="bug-rating-tab">
                                        <div class="diagram-header">
                                            <span class="diagram-title">{move || locale.get().ui.diagram.title_bug.clone()}</span>
                                            <span class="text-muted">{move || locale.get().ui.spectral.bug_subtitle.clone()}</span>
                                        </div>
                                        <DiagramZoom>
                                            <div class="diagram-fullwidth">
                                                <BugRating ldt=ldt />
                                            </div>
                                        </DiagramZoom>
                                    </div>
                                }.into_any(),
                                Tab::Lcs => view! {
                                    <div class="lcs-tab">
                                        <div class="diagram-header">
                                            <span class="diagram-title">{move || locale.get().ui.diagram.title_lcs.clone()}</span>
                                            <span class="text-muted">{move || locale.get().ui.spectral.lcs_subtitle.clone()}</span>
                                        </div>
                                        <DiagramZoom>
                                            <div class="diagram-fullwidth">
                                                <LcsClassification ldt=ldt />
                                            </div>
                                        </DiagramZoom>
                                    </div>
                                }.into_any(),
                                Tab::FloodlightVH => view! {
                                    <div class="floodlight-vh-tab">
                                        <div class="diagram-header">
                                            <span class="diagram-title">{move || locale.get().ui.diagram.title_floodlight_vh.clone()}</span>
                                            <span class="text-muted">{move || locale.get().ui.floodlight.vh_subtitle.clone()}</span>
                                        </div>
                                        <DiagramZoom>
                                            <div class="diagram-fullwidth">
                                                <FloodlightCartesian ldt=ldt />
                                            </div>
                                        </DiagramZoom>
                                    </div>
                                }.into_any(),
                                Tab::FloodlightIsolux => view! {
                                    <div class="isolux-tab">
                                        <div class="diagram-header">
                                            <span class="diagram-title">{move || locale.get().ui.diagram.title_isolux.clone()}</span>
                                            <span class="text-muted">{move || locale.get().ui.floodlight.isolux_subtitle.clone()}</span>
                                        </div>
                                        <IsoluxFootprint ldt=ldt />
                                    </div>
                                }.into_any(),
                                Tab::FloodlightIsocandela => view! {
                                    <div class="isocandela-tab">
                                        <div class="diagram-header">
                                            <span class="diagram-title">{move || locale.get().ui.diagram.title_isocandela.clone()}</span>
                                            <span class="text-muted">{move || locale.get().ui.floodlight.isocandela_subtitle.clone()}</span>
                                        </div>
                                        <DiagramZoom>
                                            <div class="diagram-fullwidth">
                                                <IsocandelaDiagramView ldt=ldt />
                                            </div>
                                        </DiagramZoom>
                                    </div>
                                }.into_any(),
                                Tab::ValidationTab => view! {
                                    <div class="validation-tab">
                                        <h3>{move || locale.get().ui.validation.title.clone()}</h3>
                                        <ValidationPanel ldt=ldt />
                                    </div>
                                }.into_any(),
                                Tab::CompareTab => view! {
                                    <div class="compare-tab">
                                        <ComparePanel
                                            ldt=ldt
                                            current_file=current_file
                                            ldt_b=compare_ldt_b
                                            set_ldt_b=set_compare_ldt_b
                                            label_b=compare_label_b
                                            set_label_b=set_compare_label_b
                                        />
                                    </div>
                                }.into_any(),
                                Tab::BimTab => view! {
                                    <div class="bim-tab">
                                        {move || {
                                            if has_bim_data(&atla_doc.get()) {
                                                view! { <BimPanel atla_doc=atla_doc /> }.into_any()
                                            } else {
                                                view! { <BimPanelEmpty /> }.into_any()
                                            }
                                        }}
                                    </div>
                                }.into_any(),
                                Tab::Scene3DTab => view! {
                                    <div class="scene-3d-tab">
                                        <div class="diagram-header">
                                            <span class="diagram-title">{move || locale.get().ui.diagram.title_scene.clone()}</span>
                                        </div>
                                        <div class="scene-container" style="height: 600px; position: relative;">
                                            <BevySceneViewer />
                                        </div>
                                        <div class="scene-controls">
                                            <p class="text-muted">
                                                {move || locale.get().ui.diagram.scene_controls.clone()}
                                            </p>
                                        </div>
                                    </div>
                                }.into_any(),
                                Tab::MapsDesignerTab => view! {
                                    <div class="maps-designer-tab">
                                        <div class="diagram-header">
                                            <span class="diagram-title">"Lighting Designer"</span>
                                        </div>
                                        <div class="maps-container" style="height: 600px; position: relative;">
                                            <MapsDesigner />
                                        </div>
                                    </div>
                                }.into_any(),
                            }}
                        </div>
                    </div>
                </main>
            </div>

            // About Modal
            {move || {
                if show_about.get() {
                    view! {
                        <div class="modal-overlay" on:click=move |_| set_show_about.set(false)>
                            <div class="modal-content about-modal" on:click=|e| e.stop_propagation()>
                                <h2>"Eulumdat"</h2>
                                <p class="about-subtitle">"Rust/WASM Lighting Data Toolkit"</p>
                                <div class="about-description">
                                    <p>"Parses EULUMDAT (.ldt), IES, TM-33, ATLA-S001, SPDX files."</p>
                                    <p>"Generates SVG diagrams: polar, cartesian, spectral, heatmap."</p>
                                    <p class="about-highlight">"One Rust codebase → Web, CLI, iOS, Android, Python"</p>
                                </div>
                                <div class="about-links">
                                    <a href="https://github.com/holg/eulumdat-rs" target="_blank" rel="noopener noreferrer">
                                        "GitHub"
                                    </a>
                                    <span class="about-version">"v0.4.0"</span>
                                </div>
                                <button class="btn btn-primary" on:click=move |_| set_show_about.set(false)>
                                    "Close"
                                </button>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    // Empty fragment when modal is hidden
                    view! { <div style="display:none"></div> }.into_any()
                }
            }}
        </ThemeProvider>
    }
}
