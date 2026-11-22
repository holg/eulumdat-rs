use eulumdat::atla::{Emitter, IntensityDistribution, LuminaireOpticalData};
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
}

/// Compile Typst source to PDF using the WASM typst compiler.
/// SVGs are already embedded inline in the source.
pub(crate) async fn compile_typst_to_pdf(typst_source: &str) -> Result<Vec<u8>, String> {
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

use super::area_designer::AreaDesigner;
use super::beam_angle_diagram::BeamAngleDiagram;
use super::bevy_scene::BevySceneViewer;
use super::bim_panel::{has_bim_data, BimPanel, BimPanelEmpty};
use super::bug_rating::BugRating;
use super::butterfly_3d::Butterfly3D;
use super::cartesian_diagram::CartesianDiagram;
use super::compare_panel::ComparePanel;
use super::cone_diagram::{ConeDiagramView, ConeIlluminanceTableView};
use super::dashboard::{Dashboard, Feature};
use super::data_table::DataTable;
use super::daylight_sky::DaylightSky;
use super::diagram_zoom::DiagramZoom;
use super::floodlight_cartesian::FloodlightCartesian;
use super::goniosim::GonioSimDemo;
use super::greenhouse_demo::GreenhouseDemo;
use super::greenhouse_diagram::GreenhouseDiagramView;
use super::intensity_heatmap::IntensityHeatmap;
use super::isocandela_diagram::IsocandelaDiagramView;
use super::isolux_footprint::IsoluxFootprint;
use super::isolux_isometric::IsoluxIsometric;
use super::lcs_classification::LcsClassification;
use super::polar_diagram::PolarDiagram;
use super::rt_scene::RtSceneViewer;
use super::sensor_view::{
    report_typst, RecognisedSensorLdt, SensorFile, SensorFiles, SensorUi, SensorView,
};
use super::skyglow_demo::SkyglowDemo;
use super::spectral_diagram::{SpectralDiagramView, SpectralViewRequest};
use super::spectrum_demo::SpectrumDemo;
use super::street_launcher::StreetLauncher;
use super::tabs::{DimensionsTab, DirectRatiosTab, GeneralTab, LampSetsTab};
use super::templates::ALL_TEMPLATES;
use super::theme::{ThemeMode, ThemeProvider};
use super::validation_panel::ValidationPanel;
use super::zonal_designer::ZonalDesigner;

// Storage keys - must match eulumdat-bevy/src/viewer/wasm_sync.rs
const LDT_STORAGE_KEY: &str = "eulumdat_current_ldt";
const LDT_TIMESTAMP_KEY: &str = "eulumdat_ldt_timestamp";
/// One-shot hand-over of a file from another page (see `tm25_viewer`).
pub const PENDING_FILE_KEY: &str = "eulumdat_pending_file";

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

/// Log color data with raw values from the LDC side (`Eulumdat` lamp sets)
/// alongside the derived ATLA emitter view.
fn log_color_data_from_ldc(filename: &str, ldc: &Eulumdat, doc: &LuminaireOpticalData) {
    web_sys::console::group_1(&format!("[Color Data] {}", filename).into());

    for (i, lamp_set) in ldc.lamp_sets.iter().enumerate() {
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

/// Top-level view mode: Dashboard overview vs full Editor
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    Dashboard,
    Editor,
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
    AreaDesigner,
    ZonalDesigner,
    GonioSim,
    SpectrumDemo,
    DaylightSky,
    StreetDesign,
    Sensor,
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
    FloodlightIsoView,
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
    // Scene 3D group: Bevy PBR scene and the path-traced light scene
    Scene3DTab,
    SceneRtTab,
    // Area Designer group (single tab, no sub-tabs)
    AreaDesignerTab,
    // Zonal Designer group (single tab, no sub-tabs)
    ZonalDesignerTab,
    // GonioSim group (single tab, no sub-tabs)
    GonioSimTab,
    // Spectrum Lab group (single tab, no sub-tabs)
    SpectrumDemoTab,
    // Daylight & Sky group (single tab, no sub-tabs)
    DaylightSkyTab,
    // Street Design group (single tab, no sub-tabs) — lazy-loaded companion
    StreetDesignTab,
    // Sensors group (single tab): presence / motion detectors
    SensorTab,
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
            | Tab::FloodlightIsoView
            | Tab::FloodlightIsocandela => MainTab::Diagrams,
            Tab::Spectral | Tab::Greenhouse | Tab::BugRating | Tab::Lcs => MainTab::Analysis,
            Tab::ValidationTab => MainTab::Validation,
            Tab::CompareTab => MainTab::Compare,
            Tab::BimTab => MainTab::Bim,
            Tab::Scene3DTab | Tab::SceneRtTab => MainTab::Scene3D,
            Tab::AreaDesignerTab => MainTab::AreaDesigner,
            Tab::ZonalDesignerTab => MainTab::ZonalDesigner,
            Tab::GonioSimTab => MainTab::GonioSim,
            Tab::SpectrumDemoTab => MainTab::SpectrumDemo,
            Tab::DaylightSkyTab => MainTab::DaylightSky,
            Tab::StreetDesignTab => MainTab::StreetDesign,
            Tab::SensorTab => MainTab::Sensor,
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
            MainTab::Scene3D => Tab::SceneRtTab,
            MainTab::AreaDesigner => Tab::AreaDesignerTab,
            MainTab::ZonalDesigner => Tab::ZonalDesignerTab,
            MainTab::GonioSim => Tab::GonioSimTab,
            MainTab::SpectrumDemo => Tab::SpectrumDemoTab,
            MainTab::DaylightSky => Tab::DaylightSkyTab,
            MainTab::StreetDesign => Tab::StreetDesignTab,
            MainTab::Sensor => Tab::SensorTab,
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
                Tab::FloodlightIsoView,
                Tab::FloodlightIsocandela,
            ],
            MainTab::Analysis => &[Tab::Spectral, Tab::Greenhouse, Tab::BugRating, Tab::Lcs],
            MainTab::Validation => &[Tab::ValidationTab],
            MainTab::Compare => &[Tab::CompareTab],
            MainTab::Bim => &[Tab::BimTab],
            MainTab::Scene3D => &[Tab::SceneRtTab, Tab::Scene3DTab],
            MainTab::AreaDesigner => &[Tab::AreaDesignerTab],
            MainTab::ZonalDesigner => &[Tab::ZonalDesignerTab],
            MainTab::GonioSim => &[Tab::GonioSimTab],
            MainTab::SpectrumDemo => &[Tab::SpectrumDemoTab],
            MainTab::DaylightSky => &[Tab::DaylightSkyTab],
            MainTab::StreetDesign => &[Tab::StreetDesignTab],
            MainTab::Sensor => &[Tab::SensorTab],
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
        color_rendering: Some(eulumdat::atla::ColorRendering {
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

/// Load unit system from URL param (`?units=imperial`), then localStorage, defaulting to Metric.
fn load_unit_system() -> UnitSystem {
    // URL param takes priority
    if let Some(val) = crate::i18n::get_url_param("units") {
        if val.eq_ignore_ascii_case("imperial") || val.eq_ignore_ascii_case("imp") {
            return UnitSystem::Imperial;
        }
        if val.eq_ignore_ascii_case("metric") || val.eq_ignore_ascii_case("si") {
            return UnitSystem::Metric;
        }
    }
    // Then localStorage
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

/// Fetch a text resource over the network, refusing responses larger
/// than `max_bytes`. Used by the `?url=…` startup loader so a hostile
/// or accidentally-huge URL can't hang the tab.
async fn fetch_text_capped(url: &str, max_bytes: usize) -> Result<String, JsValue> {
    use wasm_bindgen_futures::JsFuture;
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;

    let opts = web_sys::RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(web_sys::RequestMode::Cors);

    let request = web_sys::Request::new_with_str_and_init(url, &opts)?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let response: web_sys::Response = resp_value.dyn_into()?;
    if !response.ok() {
        return Err(JsValue::from_str(&format!(
            "HTTP {} when fetching {}",
            response.status(),
            url
        )));
    }
    // Cheap upper bound check via Content-Length when the server provides it.
    if let Ok(Some(len_str)) = response.headers().get("content-length") {
        if let Ok(len) = len_str.parse::<usize>() {
            if len > max_bytes {
                return Err(JsValue::from_str(&format!(
                    "response too large: {} > {} bytes",
                    len, max_bytes
                )));
            }
        }
    }
    let text = JsFuture::from(response.text()?).await?;
    let s = text
        .as_string()
        .ok_or_else(|| JsValue::from_str("not a string"))?;
    if s.len() > max_bytes {
        return Err(JsValue::from_str(&format!(
            "response too large after read: {} > {} bytes",
            s.len(),
            max_bytes
        )));
    }
    Ok(s)
}

/// Return `name` unchanged if it already carries a recognized
/// photometric extension; otherwise sniff the first non-blank line of
/// `content` and append a synthetic extension so the routing in
/// `load_file_content` picks the right parser. Lets `?url=` work
/// against endpoints that return raw bodies without a `.ldt` suffix
/// (e.g. gldf-search's `/api/ldc/<hash>/<idx>`).
fn ensure_extension(name: &str, content: &str) -> String {
    let lower = name.to_lowercase();
    const KNOWN: &[&str] = &[".ldt", ".ies", ".xml", ".json", ".spdx", ".oxl", ".oxc"];
    if KNOWN.iter().any(|ext| lower.ends_with(ext)) {
        return name.to_string();
    }
    let head = content.trim_start();
    let ext = if head.starts_with("IESNA") || head.starts_with("IES:") {
        ".ies"
    } else if head.starts_with("<?xml") || head.starts_with('<') {
        ".xml"
    } else if head.starts_with('{') {
        ".json"
    } else {
        // EULUMDAT files have no fixed magic — line 1 is the manufacturer
        // string, line 2 is the type indicator (1..=3). If we can't
        // identify anything else, assume LDT (the most common case).
        ".ldt"
    };
    format!("{}{}", name, ext)
}

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
    let base = template
        .name
        .to_lowercase()
        .replace(' ', "_")
        .replace("(", "")
        .replace(")", "");
    // Some template names are file names already ("road_luminaire.ldt").
    let filename = if base.ends_with(&format!(".{ext}")) {
        base
    } else {
        format!("{base}.{ext}")
    };

    let id = template.id.to_string();
    let format = template.format;
    // The Demo project remembers its last template (opened again after a
    // reload).
    if super::projects_ui::ctx()
        .is_some_and(|p| p.current_id.get_untracked() == crate::projects::DEMO_ID)
    {
        crate::projects::save_demo_luminaire(&id);
    }

    set_templates_loading.set(true);

    web_sys::console::log_1(
        &format!("[Editor] load_template: id='{}', file='{}'", id, filename).into(),
    );

    if let Some(content) = eulumdat_wasm_templates::get_template_content(&id) {
        web_sys::console::log_1(
            &format!("[Editor] template content loaded: {} chars", content.len()).into(),
        );
        match format {
            TemplateFormat::Ldt => {
                match Eulumdat::parse(&content) {
                    Ok(ldc) => {
                        web_sys::console::log_1(
                            &format!("[Editor] LDT parsed: '{}'", ldc.luminaire_name).into(),
                        );
                        // Build the atla doc from the lossless LDC, then
                        // register the LDC as the pristine source so the
                        // save-Effect emits the lossless LDT text (instead
                        // of the lossy atla-round-trip) until the user
                        // actually edits the document.
                        let doc = LuminaireOpticalData::from_eulumdat(&ldc);
                        record_pristine_ldc(&ldc, &doc.to_eulumdat());
                        set_atla_doc.set(doc);
                        set_current_file.set(Some(filename));
                        set_selected_lamp_set.set(0);
                    }
                    Err(e) => {
                        web_sys::console::error_1(
                            &format!("[Editor] LDT parse failed: {e}").into(),
                        );
                    }
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
                if let Ok(ldc) = IesParser::parse_with_options(&content, &opts) {
                    let doc = LuminaireOpticalData::from_eulumdat(&ldc);
                    record_pristine_ldc(&ldc, &doc.to_eulumdat());
                    set_atla_doc.set(doc);
                    set_current_file.set(Some(filename));
                    set_selected_lamp_set.set(0);
                }
            }
            TemplateFormat::AtlaXml => {
                if let Ok(doc) = eulumdat::atla::xml::parse(&content) {
                    clear_pristine_ldc();
                    set_atla_doc.set(doc);
                    set_current_file.set(Some(filename));
                    set_selected_lamp_set.set(0);
                }
            }
            TemplateFormat::AtlaJson => {
                if let Ok(doc) = eulumdat::atla::json::parse(&content) {
                    clear_pristine_ldc();
                    set_atla_doc.set(doc);
                    set_current_file.set(Some(filename));
                    set_selected_lamp_set.set(0);
                }
            }
        }
    } else {
        web_sys::console::error_1(
            &format!(
                "[Editor] Template '{}' not found — templates module may not be loaded yet",
                id
            )
            .into(),
        );
    }
    set_templates_loading.set(false);
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
/// localStorage: the File menu's "with the saved Light scenes" choice.
const EXPORT_SCENES_KEY: &str = "eulumdat_export_scenes";

/// A file name without folder separators or characters browsers refuse.
fn safe_file_name(name: &str) -> String {
    let s: String = name
        .trim()
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    if s.is_empty() {
        "luminaire".to_string()
    } else {
        s
    }
}

/// Today's date as `YYYY-MM-DD` (UTC).
fn today_iso() -> String {
    String::from(js_sys::Date::new_0().to_iso_string())
        .chars()
        .take(10)
        .collect()
}

/// What the "Current file" line names: luminaire, manufacturer, file.
fn current_file_parts(file: &str, ldc: &Eulumdat, maker: Option<String>) -> (String, Vec<String>) {
    // GLDF parts are named `x.gldf/ldc/y.ldt`
    let file_label = match file.split_once(".gldf/") {
        Some((g, rest)) => format!("{g}.gldf › {}", rest.rsplit('/').next().unwrap_or(rest)),
        None => file.to_string(),
    };
    let name = ldc.luminaire_name.trim();
    let maker = maker
        .filter(|m| !m.trim().is_empty())
        .or_else(|| Some(ldc.identification.trim().to_string()).filter(|m| !m.is_empty()));
    let title = if name.is_empty() {
        file_label.clone()
    } else {
        name.to_string()
    };
    let mut rest = Vec::new();
    if let Some(m) = maker {
        if m.trim() != title {
            rest.push(m.trim().to_string());
        }
    }
    if !name.is_empty() && file_label != title {
        rest.push(file_label);
    }
    (title, rest)
}

// Save-to-storage machinery.
//
// The atla→ldc converter is lossy for some real files (e.g.
// `road_luminaire.ldt` — extra intensity rows, wrong symmetry). To protect
// consumers (Bevy 3D viewer, Street designer), callers that have a
// lossless `Eulumdat` already (freshly-parsed template or uploaded LDT
// file) register it via `record_pristine_ldc`. The ATLA Effect then
// prefers the pristine value while the atla doc still represents the
// same luminaire.
thread_local! {
    /// Last known lossless LDC snapshot + the atla-round-tripped LDT
    /// serialization it maps to. While the current atla_doc still
    /// serializes to that same string, we keep writing the pristine
    /// version to storage. A genuine user edit (which *changes* the atla
    /// serialization) invalidates this cache and we fall back to writing
    /// whatever the atla-derived Eulumdat produces.
    static PRISTINE_LDC: std::cell::RefCell<Option<PristineLdc>> =
        const { std::cell::RefCell::new(None) };
}

struct PristineLdc {
    /// Lossless LDT text — what we actually want to store.
    lossless_text: String,
    /// The atla-round-tripped text at the time we recorded the pristine
    /// value. Used as the cache-validity key.
    atla_roundtrip_text: String,
    /// For diagnostic logging.
    luminaire_name: String,
}

/// Record a freshly-parsed, lossless LDC. The ATLA save-Effect will use
/// this snapshot instead of the lossy atla-round-trip as long as the
/// user hasn't edited the document.
fn record_pristine_ldc(ldc: &Eulumdat, atla_derived: &Eulumdat) {
    let pristine = PristineLdc {
        lossless_text: ldc.to_ldt(),
        atla_roundtrip_text: atla_derived.to_ldt(),
        luminaire_name: ldc.luminaire_name.clone(),
    };
    PRISTINE_LDC.with(|cell| *cell.borrow_mut() = Some(pristine));
}

/// The lossless (as-loaded) form of the current luminaire, if the user has
/// not edited it since loading. The ATLA round trip recomputes values such
/// as the direct ratios, which sensor LDTs use as their parameter block.
pub fn lossless_ldc(atla_derived: &Eulumdat) -> Option<Eulumdat> {
    let text = atla_derived.to_ldt();
    PRISTINE_LDC.with(|cell| {
        cell.borrow()
            .as_ref()
            .filter(|p| p.atla_roundtrip_text == text)
            .and_then(|p| Eulumdat::parse(&p.lossless_text).ok())
    })
}

/// Clear the pristine cache — call on `New File` / default-reset paths
/// so we don't keep sourcing the previous luminaire.
fn clear_pristine_ldc() {
    PRISTINE_LDC.with(|cell| cell.borrow_mut().take());
}

/// Write the current LDC to localStorage.
///
/// If a pristine snapshot is on file AND the atla-derived value currently
/// round-trips to the same text we recorded, we write the pristine text
/// instead — preserving fidelity through the atla round-trip.
fn save_to_storage(atla_derived: &Eulumdat) {
    let atla_text = atla_derived.to_ldt();

    let (ldt_string, source_label, luminaire_name) = PRISTINE_LDC.with(|cell| {
        let guard = cell.borrow();
        match guard.as_ref() {
            Some(p) if p.atla_roundtrip_text == atla_text => (
                p.lossless_text.clone(),
                "pristine",
                p.luminaire_name.clone(),
            ),
            _ => (
                atla_text,
                "atla-derived",
                atla_derived.luminaire_name.clone(),
            ),
        }
    });

    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item(LDT_STORAGE_KEY, &ldt_string);
            let timestamp = js_sys::Date::now().to_string();
            let _ = storage.set_item(LDT_TIMESTAMP_KEY, &timestamp);
            web_sys::console::log_1(
                &format!(
                    "[Editor] saved to storage ({source_label}): '{}' ({} chars, ts={})",
                    luminaire_name,
                    ldt_string.len(),
                    timestamp
                )
                .into(),
            );

            // Save as JSON for Maps Designer (with intensities for calculations)
            if let Ok(json) = serde_json::to_string(&serde_json::json!({
                "intensities": atla_derived.intensities,
                "c_angles": atla_derived.c_angles,
                "g_angles": atla_derived.g_angles,
                "lumens": atla_derived.luminous_flux(),
                "luminaire_name": atla_derived.luminaire_name,
            })) {
                let _ = storage.set_item("eulumdat_current_json", &json);
            }
        }
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Check for ?wasm= query parameter — render standalone WASM demos
    if let Some(demo) = crate::i18n::get_url_param("wasm") {
        // `obscura_demo` is the legacy URL kept for backwards compatibility
        // with existing links / videos. New links use `skyglow_demo`.
        if demo == "skyglow_demo" || demo == "obscura_demo" {
            return view! { <SkyglowDemo /> }.into_any();
        }
        if demo == "goniosim" {
            return view! { <GonioSimDemo /> }.into_any();
        }
        if demo == "greenhouse_demo" || demo == "greenhouse" {
            return view! { <GreenhouseDemo /> }.into_any();
        }
        if demo == "tm25" {
            return view! { <super::tm25_viewer::Tm25Viewer /> }.into_any();
        }
        if demo == "rt_viewer" || demo == "rt" {
            return view! { <super::rt_viewer::RtViewer /> }.into_any();
        }
    }

    // Primary state: ATLA document (source of truth)
    let (atla_doc, set_atla_doc) = signal(create_default_atla());

    // Secondary state: light distribution curve (LDC) view for existing
    // components. Currently an `Eulumdat` value, derived from the ATLA
    // source of truth and synced back to ATLA on downstream edits.
    let (ldc, set_ldc_internal) = signal(create_default_atla().to_eulumdat());

    // Sync ATLA → LDC whenever ATLA changes
    Effect::new(move |_| {
        set_ldc_internal.set(atla_doc.get().to_eulumdat());
    });

    // Custom setter that syncs LDC changes back to ATLA
    let set_ldc = set_ldc_internal;

    // Also sync LDC → ATLA (for when child components modify ldc)
    Effect::new(move |_| {
        let current_ldc = ldc.get();
        let current_atla_as_ldc = atla_doc.get_untracked().to_eulumdat();
        // Only update if actually different (avoid infinite loop)
        if current_ldc.to_ldt() != current_atla_as_ldc.to_ldt() {
            set_atla_doc.set(LuminaireOpticalData::from_eulumdat(&current_ldc));
        }
    });

    // An Outdoor designer share link (`#designer?…`) opens that designer
    // (in the editor, not on the dashboard)
    let designer_link = web_sys::window()
        .and_then(|w| w.location().hash().ok())
        .is_some_and(|h| h.starts_with("#designer"));
    let (view_mode, set_view_mode) = signal(if designer_link {
        ViewMode::Editor
    } else {
        ViewMode::default()
    });
    let (current_file, set_current_file) = signal::<Option<String>>(None);
    let initial_tab = if designer_link {
        Tab::AreaDesignerTab
    } else {
        Tab::default()
    };
    let (active_tab, set_active_tab) = signal(initial_tab);
    let (selected_lamp_set, set_selected_lamp_set) = signal(0_usize);
    let (templates_loading, set_templates_loading) = signal(false);

    // Opened files are kept in projects (`projects_ui`); the old library
    // (`crate::library`) is only read once to move it into a project.

    // Compare panel: File B state lives here so it persists across tab switches
    let (compare_ldc_b, set_compare_ldc_b) = signal::<Option<Eulumdat>>(None);
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

    // Deep-link parameters from the URL fragment (`#tab=light-scene&scene=
    // street&mode=points&sample=road-luminaire`), parsed once so the Light
    // scene tab can take its first settings from them.
    let link_params = StoredValue::new(parse_hash_params());
    provide_context(RtLinkParams(link_params));

    // A plan from the Interior / Outdoor designer for the Light scene
    // ("Open in Light scene"): kept here so the Light scene finds it when it
    // mounts.
    let light_scene_plan = LightScenePlan(RwSignal::new(None));
    provide_context(light_scene_plan);
    // A saved Light scene to show (opened from the project menu).
    let light_scene_doc = LightSceneDoc(RwSignal::new(None));
    provide_context(light_scene_doc);
    // Only what changes is set (a signal set to the same value still
    // notifies, and a second rebuild of the tab content would leave the
    // first Light scene instance reacting to the plan after it is gone).
    let open_light_scene = Callback::new(move |plan: eulumdat_scene::scene::ScenePlan| {
        if view_mode.get_untracked() != ViewMode::Editor {
            set_view_mode.set(ViewMode::Editor);
        }
        if active_tab.get_untracked() != Tab::SceneRtTab {
            set_active_tab.set(Tab::SceneRtTab);
        }
        light_scene_plan.0.set(Some(std::sync::Arc::new(plan)));
    });
    provide_context(OpenLightScene(open_light_scene));
    // The Street designer is its own bundle (no contexts): it hands its
    // layout over through `window.candelabraOpenStreetScene(json)`.
    register_street_scene_hook(open_light_scene, ldc);
    // Back from the Light scene to the designer a plan came from; the
    // Interior / Outdoor designers restore themselves from their link, the
    // Street designer keeps its state in its own bundle.
    provide_context(OpenDesigner(Callback::new(move |designer: Designer| {
        if view_mode.get_untracked() != ViewMode::Editor {
            set_view_mode.set(ViewMode::Editor);
        }
        let tab = match designer {
            Designer::Interior => Tab::ZonalDesignerTab,
            Designer::Outdoor => Tab::AreaDesignerTab,
            Designer::Street => Tab::StreetDesignTab,
        };
        if active_tab.get_untracked() != tab {
            set_active_tab.set(tab);
        }
    })));

    // lux → PPFD factor of the loaded spectrum (ATLA / TM-33 emitter SPD), for
    // the greenhouse scene; `None` lets the viewer assume a white LED.
    let ppfd_factor = RwSignal::new(None::<f32>);
    provide_context(PpfdFactor(ppfd_factor));
    Effect::new(move |_| {
        let doc = atla_doc.get();
        let f = doc
            .emitters
            .iter()
            .filter_map(|e| e.spectral_distribution.as_ref())
            .next()
            .and_then(|spd| {
                eulumdat_scene::photometry::ppfd_per_lux(&spd.wavelengths, &spd.values)
            });
        if ppfd_factor.get_untracked() != f {
            ppfd_factor.set(f);
        }
    });

    // Melanopic DER (CIE S 026) of the loaded spectrum, for the Light scene's
    // melanopic EDI at eye level; `None` when the luminaire has no spectrum.
    let mel_der = RwSignal::new(None::<f32>);
    provide_context(MelanopicDer(mel_der));
    Effect::new(move |_| {
        let doc = atla_doc.get();
        let f = doc
            .emitters
            .iter()
            .filter_map(|e| e.spectral_distribution.as_ref())
            .next()
            .and_then(|spd| {
                eulumdat_scene::photometry::melanopic_der(&spd.wavelengths, &spd.values)
            });
        if mel_der.get_untracked() != f {
            mel_der.set(f);
        }
    });

    // Luminaire body from a GLDF container's L3D model, for the Light scene.
    // Cleared whenever a luminaire that did not come from that container is
    // loaded; GLDF-derived library rows keep the body that is current.
    let body_mesh: RwSignal<Option<std::sync::Arc<eulumdat_scene::mesh::LuminaireMesh>>> =
        RwSignal::new(None);
    provide_context(body_mesh);
    // Presence / motion sensors from GLDF containers and SensXML files.
    let sensor_files: RwSignal<Vec<SensorFile>> = RwSignal::new(Vec::new());
    provide_context(SensorFiles(sensor_files));
    // Sensors tab state (app-wide, so the PDF report can include the sensor).
    let sensor_ui = SensorUi::new(sensor_files);
    provide_context(sensor_ui);
    // Sub-tab requests for the spectral view (dashboard "Colour quality" tile).
    let spectral_view = SpectralViewRequest(RwSignal::new(None));
    provide_context(spectral_view);
    // Measured spectrum of the luminaire in the editor, for the report's
    // colour-quality chapter (also used by the Sensors tab's PDF button).
    provide_context(ReportSpectrum(Signal::derive(move || {
        report_spectrum(&atla_doc.get())
    })));
    // Projects: the dashboard shows the current one; opened files join it.
    // A link that asks for demo sensors shows the Demo project.
    let force_demo = link_params.with_value(|v| {
        v.iter()
            .any(|(k, val)| matches!(k.as_str(), "sensor" | "sa" | "sb") && val.starts_with("demo"))
    });
    // After a reload the current project's luminaire opens again, unless
    // the link brings its own (`sample=`, `?url=`, a file handed over by
    // another page).
    let link_brings_luminaire = link_params.with_value(|v| v.iter().any(|(k, _)| k == "sample"))
        || web_sys::window()
            .and_then(|w| w.location().href().ok())
            .and_then(|h| web_sys::Url::new(&h).ok())
            .is_some_and(|u| u.search_params().get("url").is_some())
        || web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item(PENDING_FILE_KEY).ok().flatten())
            .is_some();
    let projects = super::projects_ui::ProjectsCtx::init(force_demo, !link_brings_luminaire);
    provide_context(projects);
    Effect::new(move |_| {
        let l = locale.get();
        projects.names.set((
            l.dashboard.projects.untitled.clone(),
            l.dashboard.projects.my_files.clone(),
            l.dashboard.projects.copy_suffix.clone(),
        ));
    });
    // Requests to the Compare tab (sensor mode, select an opened sensor).
    let compare_requests = super::sensor_compare::CompareRequests {
        sensors_mode: RwSignal::new(super::sensor_compare::link_wants_sensors()),
        pick: RwSignal::new(None),
        pair: RwSignal::new(None),
    };
    provide_context(compare_requests);
    let interior_requests = super::zonal_sensors::InteriorRequests {
        pick: RwSignal::new(None),
    };
    provide_context(interior_requests);
    // The dashboard's file rows: the luminaires of the current project.
    let project_library = Memo::new(move |_| crate::library::Library {
        entries: projects
            .with_current(|p| {
                p.luminaires()
                    .map(|i| crate::library::LibraryEntry {
                        id: i.id.clone(),
                        name: i.name.clone(),
                        content: i.content.clone(),
                        source: i.source.clone(),
                        added_at: i.added_at,
                    })
                    .collect()
            })
            .unwrap_or_default(),
    });
    // The app's sensor list follows the current project.
    let sensor_key = Memo::new(move |_| projects.sensor_key());
    Effect::new(move |_| {
        // read, not just `track()`: a memo that is never read never computes
        // and so never sees the project change
        let _key = sensor_key.get();
        sensor_files.set(projects.sensor_files());
    });
    sensor_ui.apply_link(&link_params.get_value());
    Effect::new(move |_| {
        sensor_ui.luminaire_loaded.set(current_file.get().is_some());
    });
    let (show_about, set_show_about) = signal(false);

    // C-plane rotation for IES↔LDT axis correction (EU C0‖length vs US C0⊥length)
    let (rotate_c_planes, set_rotate_c_planes) = signal(false);

    // Check if PDF/Typst export is enabled (via secret URL)
    let export_enabled = is_export_enabled();

    // Save to localStorage whenever the atla document changes. Converts
    // back to Eulumdat at the boundary — callers that *know* they have a
    // lossless Eulumdat already (e.g. freshly-parsed template file) should
    // call `save_to_storage` directly with that value first, so consumers
    // see the lossless form even if this Effect fires a moment later.
    Effect::new(move |_| {
        save_to_storage(&atla_doc.get().to_eulumdat());
    });

    // File loading helper - ALL formats convert to ATLA (lossless)
    //
    // `source` records provenance for the library entry — pass
    // `LibrarySource::LocalFile` for drag-drop / file picker,
    // `LibrarySource::Url(href)` for the `?url=…` startup loader,
    // `LibrarySource::Template` / `Bundle` for the template buttons.
    // Every successful parse appends (or refreshes, by content-hash
    // dedup) an entry in the persistent library.
    // An `.ldt` with the sensor LDT fingerprint opens on the Sensors tab;
    // "open as luminaire instead" sets `force_luminaire` for one load.
    let recognised_sensor = RecognisedSensorLdt {
        notice: RwSignal::new(None),
        open_as_luminaire: RwSignal::new(None),
    };
    provide_context(recognised_sensor);
    let force_luminaire = StoredValue::new(false);
    // Opening an item of the project: it is in the project already (no
    // re-adding, no reordering).
    let from_project = StoredValue::new(false);

    let load_file_content =
        move |name: String, content: String, source: crate::library::LibrarySource| {
            let as_luminaire = force_luminaire.get_value();
            force_luminaire.set_value(false);
            let already_in_project = from_project.get_value();
            from_project.set_value(false);
            if !name.contains(".gldf/") {
                body_mesh.set(None);
            }
            // Append to library before parsing so even files that fail to
            // dispatch through any branch still leave a breadcrumb? No —
            // unknown formats already log to console; we only want valid
            // photometric files in the library. Push at the end, but
            // capture the entry skeleton up front for clarity.
            let entry = crate::library::LibraryEntry {
                id: crate::library::content_id(&content),
                name: name.clone(),
                content: content.clone(),
                source,
                added_at: js_sys::Date::now(),
            };
            let lower_name = name.to_lowercase();
            // A project file opens as a new project.
            if lower_name.ends_with(crate::projects::FILE_SUFFIX)
                || (lower_name.ends_with(".json") && content.contains("\"candelabra-project\""))
            {
                if let Err(e) = projects.import_text(&content) {
                    web_sys::console::error_1(&format!("{name}: {e}").into());
                }
                set_view_mode.set(ViewMode::Dashboard);
                return;
            }
            // A sensor goes into the project and opens on the sensor page;
            // on the Compare tab it stays there (the sensor comparison picks
            // it up).
            let sensor_source = entry.source.clone();
            let show_sensor = move |name: &str, content: &str, kind_note: bool| {
                projects.add(
                    name,
                    content.to_string(),
                    crate::projects::ItemKind::Sensor,
                    sensor_source.clone(),
                );
                if active_tab.get_untracked() == Tab::CompareTab
                    && view_mode.get_untracked() == ViewMode::Editor
                {
                    if !compare_requests.sensors_mode.get_untracked() {
                        compare_requests.sensors_mode.set(true);
                    }
                    compare_requests.pick.set(Some(name.to_string()));
                    return;
                }
                sensor_ui.use_loaded.set(false);
                sensor_ui.selected.set(0);
                if kind_note {
                    recognised_sensor
                        .notice
                        .set(Some((name.to_string(), content.to_string())));
                }
                set_view_mode.set(ViewMode::Editor);
                set_active_tab.set(Tab::SensorTab);
            };
            // A sensor LDT (ranges in metres, mounting code in lines 33–42) is
            // not a luminaire either; GLDF photometries were sorted already.
            if lower_name.ends_with(".ldt") && !name.contains(".gldf/") && !as_luminaire {
                let is_sensor =
                    Eulumdat::parse(&content).is_ok_and(|l| eulumdat::sensor::is_sensor_ldt(&l));
                if is_sensor
                    && SensorFile::from_bytes(&name, "sensor/sensldt", content.as_bytes()).is_some()
                {
                    web_sys::console::info_1(&format!("{name}: recognised as a sensor LDT").into());
                    show_sensor(&name, &content, true);
                    return;
                }
            }
            // A SensXML sensor file is not a luminaire either.
            if lower_name.ends_with(".xml") && content.contains("<SensXML") {
                if SensorFile::from_bytes(&name, "sensor/sensxml", content.as_bytes()).is_some() {
                    show_sensor(&name, &content, false);
                } else {
                    web_sys::console::error_1(&format!("SensXML: could not read {name}").into())
                }
                return;
            }
            let is_ies = lower_name.ends_with(".ies");
            let is_atla_xml = lower_name.ends_with(".xml");
            let is_atla_json = lower_name.ends_with(".json");
            let is_ldt = lower_name.ends_with(".ldt");
            let is_spdx = lower_name.ends_with(".spdx");
            // Raw SPD inputs: Luxeon `.spd`, generic `wavelength_nm,intensity` CSV,
            // Signify lab CSV with its metric prelude — auto-detected by content.
            let is_spd = lower_name.ends_with(".spd") || lower_name.ends_with(".csv");
            // OxyTech LITESTAR exports — `.oxl` carries photometry, `.oxc`
            // is a commercial-only sibling. Both share the LitePack XML
            // schema; our parser handles them identically.
            let is_oxl = lower_name.ends_with(".oxl") || lower_name.ends_with(".oxc");

            // Parse to ATLA format (source of truth)
            if is_spdx {
                // SPDX (IES TM-27-14) → ATLA (spectral only, no photometric data)
                match eulumdat::atla::spdx::parse(&content) {
                    Ok(spdx_data) => {
                        // Log warnings about missing data
                        let warnings = eulumdat::atla::spdx::get_warnings(&spdx_data);
                        for warning in &warnings {
                            web_sys::console::warn_1(&format!("SPDX: {}", warning).into());
                        }

                        let doc = eulumdat::atla::spdx::to_atla(&spdx_data);
                        clear_pristine_ldc();
                        set_atla_doc.set(doc);
                        set_current_file.set(Some(name));
                        set_selected_lamp_set.set(0);

                        // Show first warning to user
                        if let Some(first_warning) = warnings.first() {
                            web_sys::console::info_1(
                                &format!(
                                    "Loaded SPDX file (spectral data only): {}",
                                    first_warning
                                )
                                .into(),
                            );
                        }
                    }
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to parse SPDX: {}", e).into());
                    }
                }
            } else if is_spd {
                // Raw SPD (.spd / .csv): Luxeon datasheet `.spd`, generic
                // `wavelength_nm,intensity` CSV, or Signify lab CSV with metric
                // prelude. Auto-detected by content; produces a spectral-only
                // ATLA doc (no intensity distribution) just like SPDX.
                match eulumdat::atla::spd_loader::parse(&content) {
                    Ok(loaded) => {
                        for warning in eulumdat::atla::spd_loader::get_warnings(&loaded) {
                            web_sys::console::warn_1(&format!("SPD: {}", warning).into());
                        }
                        let doc = eulumdat::atla::spd_loader::to_atla(&loaded);
                        clear_pristine_ldc();
                        set_atla_doc.set(doc);
                        set_current_file.set(Some(name));
                        set_selected_lamp_set.set(0);
                    }
                    Err(e) => {
                        web_sys::console::error_1(
                            &format!("Failed to parse SPD/CSV: {}", e).into(),
                        );
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
                    Ok(ldc) => {
                        let doc = LuminaireOpticalData::from_eulumdat(&ldc);
                        log_color_data_from_ldc(&name, &ldc, &doc);
                        record_pristine_ldc(&ldc, &doc.to_eulumdat());
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
                match eulumdat::atla::xml::parse(&content) {
                    Ok(doc) => {
                        log_color_data(&name, &doc);
                        clear_pristine_ldc();
                        set_atla_doc.set(doc);
                        set_current_file.set(Some(name));
                        set_selected_lamp_set.set(0);
                    }
                    Err(e) => {
                        web_sys::console::error_1(
                            &format!("Failed to parse ATLA XML: {}", e).into(),
                        );
                    }
                }
            } else if is_atla_json {
                // ATLA JSON → ATLA (direct, no conversion)
                match eulumdat::atla::json::parse(&content) {
                    Ok(doc) => {
                        log_color_data(&name, &doc);
                        clear_pristine_ldc();
                        set_atla_doc.set(doc);
                        set_current_file.set(Some(name));
                        set_selected_lamp_set.set(0);
                    }
                    Err(e) => {
                        web_sys::console::error_1(
                            &format!("Failed to parse ATLA JSON: {}", e).into(),
                        );
                    }
                }
            } else if is_ldt {
                // LDT file → LDC (in-memory distribution) → ATLA. Register
                // the pristine LDC so the atla save-Effect emits the lossless
                // LDT text until the user edits the document.
                match Eulumdat::parse(&content) {
                    Ok(ldc) => {
                        let doc = LuminaireOpticalData::from_eulumdat(&ldc);
                        log_color_data_from_ldc(&name, &ldc, &doc);
                        record_pristine_ldc(&ldc, &doc.to_eulumdat());
                        set_atla_doc.set(doc);
                        set_current_file.set(Some(name));
                        set_selected_lamp_set.set(0);
                    }
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to parse LDT: {}", e).into());
                    }
                }
            } else if is_oxl {
                // OXL/OXC → multi-luminaire LitePack package → ATLA. We
                // import the first luminaire of the package; if the package
                // carries more, the others are reachable today only via the
                // library API (`eulumdat::atla::oxl::parse`). A "pick which
                // luminaire" picker is a deferred UX task.
                match eulumdat::atla::oxl::parse(&content) {
                    Ok(pkg) => {
                        if pkg.luminaires.is_empty() {
                            web_sys::console::warn_1(
                                &"OXL/OXC contains no luminaires (commercial-only file?)".into(),
                            );
                        } else {
                            if pkg.luminaires.len() > 1 {
                                web_sys::console::warn_1(
                                    &format!(
                                    "OXL package contains {} luminaires; loading the first only.",
                                    pkg.luminaires.len()
                                )
                                    .into(),
                                );
                            }
                            let doc = pkg.luminaires.into_iter().next().unwrap();
                            log_color_data(&name, &doc);
                            clear_pristine_ldc();
                            set_atla_doc.set(doc);
                            set_current_file.set(Some(name));
                            set_selected_lamp_set.set(0);
                        }
                    }
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to parse OXL: {}", e).into());
                    }
                }
            } else {
                web_sys::console::error_1(&"Unknown file format".into());
                return;
            }
            // Reached only when one of the format branches above produced a
            // valid ATLA document: the file joins the current project (the
            // bundled templates are in the Demo project already).
            // An item of the project is the one the editor shows now.
            if already_in_project {
                projects.set_active(&entry.id);
            } else if !matches!(
                entry.source,
                crate::library::LibrarySource::Template | crate::library::LibrarySource::Bundle
            ) {
                projects.add(
                    &entry.name,
                    entry.content,
                    crate::projects::ItemKind::Luminaire,
                    entry.source,
                );
            }
        };

    // ── `#tab=<id>` — open the editor on a tab (landing-page deep links) ──
    //
    // The landing page links straight into a tool, e.g. `app/#tab=light-scene`.
    // Runs once at startup; unknown ids are ignored. The hash is cleared so a
    // reload lands on the dashboard again.
    Effect::new(move |_| {
        let Some(window) = web_sys::window() else {
            return;
        };
        let Ok(hash) = window.location().hash() else {
            return;
        };
        // The interior designer's share link (`#interior?preset=…&…`) opens
        // its tab and keeps the hash: the designer reads its values from it.
        let interior_link = hash.starts_with("#interior?");
        let pairs = link_params.get_value();
        let get = |key: &str| pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone());
        let tab_id = get("tab").or_else(|| interior_link.then(|| "interior".to_string()));
        let sample = get("sample");
        // `sample=<template id>` loads one of the bundled luminaires first,
        // so a landing-page link can show the street with the road light.
        if let Some(sample) = sample {
            if let Some(template) = super::templates::ALL_TEMPLATES
                .iter()
                .find(|t| t.id == sample)
            {
                load_template(
                    template,
                    set_atla_doc,
                    set_current_file,
                    set_selected_lamp_set,
                    set_templates_loading,
                    rotate_c_planes,
                );
            }
        }
        let Some(id) = tab_id else {
            return;
        };
        // `compare=<template id>` puts a second bundled luminaire into the
        // Compare tab next to the loaded one.
        if let Some(cmp) = get("compare") {
            if let Some(template) = super::templates::ALL_TEMPLATES.iter().find(|t| t.id == cmp) {
                if let Some(ldt) = template_to_eulumdat(template) {
                    set_compare_label_b.set(Some(template.name.to_string()));
                    set_compare_ldc_b.set(Some(ldt));
                }
            }
        }
        let tab = match id.as_str() {
            "light-scene" | "scene" => Some(Tab::SceneRtTab),
            "scene-3d" => Some(Tab::Scene3DTab),
            "diagrams" | "polar" | "cartesian" => Some(Tab::Diagram2D),
            "3d" | "butterfly" => Some(Tab::Diagram3D),
            "heatmap" => Some(Tab::Heatmap),
            "cone" => Some(Tab::Cone),
            "isolux" => Some(Tab::FloodlightIsolux),
            "isocandela" => Some(Tab::FloodlightIsocandela),
            "iso-view" => Some(Tab::FloodlightIsoView),
            "vh" | "floodlight" => Some(Tab::FloodlightVH),
            "lcs" => Some(Tab::Lcs),
            "greenhouse-tab" | "ppfd" => Some(Tab::Greenhouse),
            "spectrum-lab" => Some(Tab::SpectrumDemoTab),
            "intensity" | "data" => Some(Tab::Intensity),
            "info" | "general" => Some(Tab::General),
            "validation" => Some(Tab::ValidationTab),
            "bug" => Some(Tab::BugRating),
            "spectral" => Some(Tab::Spectral),
            "compare" => Some(Tab::CompareTab),
            "bim" | "export" => Some(Tab::BimTab),
            "area" => Some(Tab::AreaDesignerTab),
            "interior" => Some(Tab::ZonalDesignerTab),
            // the map designer: the Outdoor designer on its site map
            "maps" => {
                super::site_map::request_site_map();
                Some(Tab::AreaDesignerTab)
            }
            "street" => Some(Tab::StreetDesignTab),
            "goniosim" => Some(Tab::GonioSimTab),
            "daylight" => Some(Tab::DaylightSkyTab),
            "sensor" | "sensors" => Some(Tab::SensorTab),
            _ => None,
        };
        if let Some(tab) = tab {
            set_view_mode.set(ViewMode::Editor);
            set_active_tab.set(tab);
            if !interior_link {
                let _ = window.location().set_hash("");
            }
        }
    });

    // ── `?url=…` query param — fetch and load a remote LDT/IES/ATLA ──────
    //
    // Drives sharing from sibling services (gldf-search.de, iesna.eu's own
    // catalog, anyone with a CORS-friendly host). Same shape as a normal
    // file open: we hand the fetched body to `load_file_content`, which
    // routes by extension. Fires once at app startup; ignored if absent.
    //
    // The remote host MUST send `Access-Control-Allow-Origin` for the
    // fetch to succeed. We require `https://` in production (HTTPS
    // pages would block mixed-content `http://` fetches anyway), but
    // permit `http://` when the editor itself runs over `http://`
    // (local dev: `127.0.0.1:8042` → `127.0.0.1:3090` gldf-search etc).
    // Response is capped at 5 MiB to keep a malicious URL from hanging
    // the tab.
    Effect::new(move |_| {
        let Some(window) = web_sys::window() else {
            return;
        };
        let Ok(href) = window.location().href() else {
            return;
        };
        let Ok(url) = web_sys::Url::new(&href) else {
            return;
        };
        let Some(src) = url.search_params().get("url") else {
            return;
        };
        let page_is_http = url.protocol() == "http:";
        let src_ok = src.starts_with("https://") || (page_is_http && src.starts_with("http://"));
        if !src_ok {
            web_sys::console::warn_1(
                &format!("ignoring url param (scheme not allowed): {}", src).into(),
            );
            return;
        }
        // Best-effort filename from the URL path. Won't always have an
        // extension (e.g. gldf-search's `/api/ldc/<hash>/<idx>` endpoint
        // returns raw LDT bytes with no `.ldt` suffix); we'll sniff the
        // payload after fetch and synthesize one when needed.
        let url_basename = src
            .rsplit('/')
            .next()
            .and_then(|s| s.split('?').next())
            .filter(|s| !s.is_empty())
            .unwrap_or("shared")
            .to_string();

        let src_for_source = src.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_text_capped(&src, 5 * 1024 * 1024).await {
                Ok(content) => {
                    let filename = ensure_extension(&url_basename, &content);
                    // Stay on the dashboard so the library row appears
                    // and the user can keep loading more `?url=…` links.
                    // The library list shows everything they've opened.
                    load_file_content(
                        filename,
                        content,
                        crate::library::LibrarySource::Url(src_for_source),
                    );
                }
                Err(e) => {
                    web_sys::console::error_1(&format!("failed to load {}: {:?}", src, e).into());
                }
            }
        });
    });

    // "Open as luminaire instead" on the Sensors tab: load the recognised
    // sensor LDT as a luminaire after all.
    Effect::new(move |_| {
        let Some((name, content)) = recognised_sensor.open_as_luminaire.get() else {
            return;
        };
        recognised_sensor.open_as_luminaire.set(None);
        projects.remove(&crate::library::content_id(&content));
        force_luminaire.set_value(true);
        load_file_content(name, content, crate::library::LibrarySource::LocalFile);
        set_active_tab.set(Tab::Diagram2D);
    });

    // Handlers
    let on_new_file = move |_| {
        clear_pristine_ldc();
        set_atla_doc.set(create_default_atla());
        set_current_file.set(None);
        set_selected_lamp_set.set(0);
    };

    // Base name of exported files: the opened file (without folders and
    // extension), else the luminaire's name or number.
    let export_base = move || -> String {
        let from_file = current_file.get_untracked().and_then(|f| {
            let last = f.rsplit('/').next().unwrap_or(&f).to_string();
            let stem = match last.rfind('.') {
                Some(i) if i > 0 => last[..i].to_string(),
                _ => last,
            };
            (!stem.trim().is_empty()).then_some(stem)
        });
        let l = ldc.get_untracked();
        let name = from_file
            .or_else(|| {
                [l.luminaire_name.clone(), l.luminaire_number.clone()]
                    .into_iter()
                    .find(|s| !s.trim().is_empty())
            })
            .unwrap_or_else(|| "luminaire".to_string());
        safe_file_name(&name)
    };
    let tm33_options = move || eulumdat::atla::tm33::WriteOptions {
        today: Some(today_iso()),
        creator: None,
    };
    // What a TM-33-23 file of the current luminaire carries (menu labels).
    let tm33_contents = Memo::new(move |_| eulumdat::atla::tm33::contents(&atla_doc.get()));

    let on_save_ldt = move |_| {
        // Export from ATLA → LDT
        let content = atla_doc.get().to_eulumdat().to_ldt();
        super::file_handler::download_file(
            &format!("{}.ldt", export_base()),
            &content,
            "text/plain",
        );
    };

    let on_export_ies = move |_| {
        // Export from ATLA → Eulumdat → IES
        let opts = eulumdat::IesExportOptions {
            rotate_c_planes: if rotate_c_planes.get() { -90.0 } else { 0.0 },
            ..Default::default()
        };
        let content =
            eulumdat::IesExporter::export_with_options(&atla_doc.get().to_eulumdat(), &opts);
        super::file_handler::download_ies(&format!("{}.ies", export_base()), &content);
    };

    // IES TM-33-23 (with the TM-32-24 BIM blocks when the luminaire has them)
    let on_export_tm33_xml = move |_| match eulumdat::atla::tm33::write_xml(
        &atla_doc.get_untracked(),
        &tm33_options(),
    ) {
        Ok(content) => {
            super::file_handler::download_atla_xml(&format!("{}.xml", export_base()), &content)
        }
        Err(e) => web_sys::console::error_1(&format!("TM-33-23 XML export failed: {e}").into()),
    };
    let on_export_tm33_json =
        move |_| match eulumdat::atla::tm33::write_json(&atla_doc.get_untracked(), &tm33_options())
        {
            Ok(content) => super::file_handler::download_atla_json(
                &format!("{}.json", export_base()),
                &content,
            ),
            Err(e) => {
                web_sys::console::error_1(&format!("TM-33-23 JSON export failed: {e}").into())
            }
        };

    // IES TM-27 SPDX: the spectrum alone
    let on_export_spdx = move |_| {
        let opts = eulumdat::atla::spdx::WriteOptions {
            today: Some(today_iso()),
        };
        let doc = atla_doc.get_untracked();
        let k = selected_lamp_set
            .get_untracked()
            .min(doc.emitters.len().saturating_sub(1));
        let text = eulumdat::atla::spdx::write(&doc, k, &opts)
            .or_else(|| eulumdat::atla::spdx::write(&doc, 0, &opts));
        match text {
            Some(content) => super::file_handler::download_file(
                &format!("{}.spdx", export_base()),
                &content,
                "application/xml",
            ),
            None => web_sys::console::warn_1(&"No spectrum to export".into()),
        }
    };

    // ATLA S001 / UNI 11733 (the older European layout of the same data)
    let on_export_atla_xml = move |_| match eulumdat::atla::xml::write_with_schema(
        &atla_doc.get_untracked(),
        eulumdat::atla::SchemaVersion::AtlaS001,
        Some(2),
    ) {
        Ok(content) => {
            super::file_handler::download_atla_xml(&format!("{}.atla.xml", export_base()), &content)
        }
        Err(e) => {
            web_sys::console::error_1(&format!("Failed to export ATLA XML: {}", e).into());
        }
    };

    let on_export_atla_json = move |_| match eulumdat::atla::json::write(&atla_doc.get_untracked())
    {
        Ok(content) => super::file_handler::download_atla_json(
            &format!("{}.atla.json", export_base()),
            &content,
        ),
        Err(e) => {
            web_sys::console::error_1(&format!("Failed to export ATLA JSON: {}", e).into());
        }
    };

    // Project file: the saved Light scenes only when asked for (remembered).
    let export_scenes = RwSignal::new(
        web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item(EXPORT_SCENES_KEY).ok().flatten())
            .is_some_and(|v| v == "1"),
    );

    let on_export_report_typ = move |_| {
        // Generate Typst report from the current LDT
        let ldc_val = ldc.get();
        let generator = ReportGenerator::new(&ldc_val);
        let content = generator.generate_typst(&ReportOptions::default());
        let filename = current_file
            .get()
            .map(|_| format!("{}.typ", export_base()))
            .unwrap_or_else(|| "photometric_report.typ".to_string());
        super::file_handler::download_file(&filename, &content, "text/plain");
    };

    // Signal to track PDF export loading state
    let (pdf_exporting, set_pdf_exporting) = signal(false);

    let on_export_report_pdf = move |_| {
        // Generate Typst report with inline SVGs and compile to PDF via WASM
        let ldc_val = ldc.get();
        let default_name = current_file
            .get()
            .map(|_| format!("{}.pdf", export_base()))
            .unwrap_or_else(|| "photometric_report.pdf".to_string());
        // photometric report, with the selected sensor (Sensors tab) as a
        // chapter, or the sensor report alone when no luminaire is loaded;
        // a measured spectrum adds the colour-quality chapter
        let doc = atla_doc.get_untracked();
        let (typst_source, filename) = report_typst(
            &sensor_ui,
            &ldc_val,
            &ldc_val,
            &locale.get_untracked().ui.sensor.loaded_ldt,
            default_name,
            report_spectrum(&doc)
                .as_ref()
                .map(|(spd, n)| (spd, n.as_str())),
        );

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
        let ldc_val = ldc.get();
        let theme = eulumdat::diagram::SvgTheme::light(); // Use light theme for export

        match current_tab {
            Tab::Diagram2D => {
                // Use the current diagram type (Polar, Cartesian, or BeamAngle)
                match diagram_type.get() {
                    DiagramType::Polar => {
                        let polar = eulumdat::diagram::PolarDiagram::from_eulumdat(&ldc_val);
                        let summary = eulumdat::PhotometricSummary::from_eulumdat(&ldc_val);
                        let svg = polar.to_svg_with_summary(600.0, 600.0, &theme, &summary);
                        Some((svg, "polar_diagram.svg".to_string()))
                    }
                    DiagramType::Cartesian => {
                        let cartesian = eulumdat::diagram::CartesianDiagram::from_eulumdat(
                            &ldc_val, 600.0, 450.0, 8,
                        );
                        let summary = eulumdat::PhotometricSummary::from_eulumdat(&ldc_val);
                        let svg = cartesian.to_svg_with_summary(600.0, 450.0, &theme, &summary);
                        Some((svg, "cartesian_diagram.svg".to_string()))
                    }
                    DiagramType::BeamAngle => {
                        let polar = eulumdat::diagram::PolarDiagram::from_eulumdat(&ldc_val);
                        let analysis =
                            eulumdat::PhotometricCalculations::beam_field_analysis(&ldc_val);
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
                    &ldc_val, 600.0, 500.0, 60.0,
                );
                let svg = butterfly.to_svg(600.0, 500.0, &theme);
                Some((svg, "butterfly_3d_diagram.svg".to_string()))
            }
            Tab::Heatmap => {
                let heatmap =
                    eulumdat::diagram::HeatmapDiagram::from_eulumdat(&ldc_val, 700.0, 500.0);
                let summary = eulumdat::PhotometricSummary::from_eulumdat(&ldc_val);
                let svg = heatmap.to_svg_with_summary(700.0, 500.0, &theme, &summary);
                Some((svg, "intensity_heatmap.svg".to_string()))
            }
            Tab::Cone => {
                let height = mounting_height.get();
                let cone = match selected_c_plane.get() {
                    Some(c) => {
                        eulumdat::diagram::ConeDiagram::from_eulumdat_for_plane(&ldc_val, height, c)
                    }
                    None => eulumdat::diagram::ConeDiagram::from_eulumdat(&ldc_val, height),
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
                let atla_theme = eulumdat::atla::spectral::SpectralTheme::light();

                // Try to get spectral data
                if let Some(spd) = doc
                    .emitters
                    .iter()
                    .filter_map(|e| e.spectral_distribution.as_ref())
                    .next()
                {
                    let diagram = eulumdat::atla::spectral::SpectralDiagram::from_spectral(spd);
                    let svg = diagram.to_svg(700.0, 400.0, &atla_theme);
                    Some((svg, "spectral_diagram.svg".to_string()))
                } else if let Some(emitter) = doc.emitters.first() {
                    if let Some(cct) = emitter.cct {
                        let cri = emitter.color_rendering.as_ref().and_then(|cr| cr.ra);
                        let spd = eulumdat::atla::spectral::synthesize_spectrum(cct, cri);
                        let diagram =
                            eulumdat::atla::spectral::SpectralDiagram::from_spectral(&spd);
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
                let diagram = eulumdat::atla::greenhouse::GreenhouseDiagram::from_atla_with_height(
                    &doc, height,
                );
                let gh_theme = eulumdat::atla::greenhouse::GreenhouseTheme::light();
                let svg = diagram.to_svg(600.0, 450.0, &gh_theme);
                Some((svg, "greenhouse_ppfd.svg".to_string()))
            }
            Tab::BugRating => {
                let diagram = eulumdat::BugDiagram::from_eulumdat(&ldc_val);
                let svg = diagram.to_svg_with_details(600.0, 400.0, &theme);
                Some((svg, "bug_rating.svg".to_string()))
            }
            Tab::Lcs => {
                let diagram = eulumdat::BugDiagram::from_eulumdat(&ldc_val);
                let svg = diagram.to_lcs_svg(600.0, 400.0, &theme);
                Some((svg, "lcs_classification.svg".to_string()))
            }
            Tab::FloodlightVH => {
                let y_scale = eulumdat::diagram::YScale::Linear;
                let diagram = eulumdat::diagram::FloodlightCartesianDiagram::from_eulumdat(
                    &ldc_val, 600.0, 400.0, y_scale,
                );
                let svg = diagram.to_svg(600.0, 400.0, &theme);
                Some((svg, "floodlight_vh.svg".to_string()))
            }
            Tab::FloodlightIsolux => {
                let params = eulumdat::diagram::IsoluxParams::default();
                let diagram =
                    eulumdat::diagram::IsoluxDiagram::from_eulumdat(&ldc_val, 600.0, 500.0, params);
                let svg = diagram.to_svg(600.0, 500.0, &theme);
                Some((svg, "isolux_footprint.svg".to_string()))
            }
            Tab::FloodlightIsoView => {
                let params = eulumdat::diagram::IsoluxParams {
                    area_half_width: 30.0,
                    area_half_depth: 15.0,
                    ..eulumdat::diagram::IsoluxParams::default()
                };
                let diagram = eulumdat::diagram::IsoluxDiagram::from_eulumdat(
                    &ldc_val, 1200.0, 500.0, params,
                );
                let title = if !ldc_val.luminaire_name.is_empty() {
                    format!("ISO view {}", ldc_val.luminaire_name)
                } else {
                    "ISO view".to_string()
                };
                let svg = diagram.to_svg_isometric(
                    1200.0,
                    500.0,
                    &theme,
                    &eulumdat::diagram::IsometricConfig::default(),
                    UnitSystem::default(),
                    &ldc_val,
                    &title,
                );
                Some((svg, "iso_view.svg".to_string()))
            }
            Tab::FloodlightIsocandela => {
                let diagram =
                    eulumdat::diagram::IsocandelaDiagram::from_eulumdat(&ldc_val, 600.0, 500.0);
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
            | Tab::SceneRtTab
            | Tab::AreaDesignerTab
            | Tab::ZonalDesignerTab
            | Tab::GonioSimTab
            | Tab::SpectrumDemoTab
            | Tab::DaylightSkyTab
            | Tab::StreetDesignTab
            | Tab::SensorTab => None,
        }
    };

    let on_export_svg = move |_| {
        if let Some((svg_content, default_filename)) = generate_current_svg() {
            let filename = current_file
                .get()
                .map(|_| format!("{}.svg", export_base()))
                .unwrap_or(default_filename);
            super::file_handler::download_svg(&filename, &svg_content);
        } else {
            web_sys::console::warn_1(&"No diagram to export on this tab".into());
        }
    };

    let on_export_png = move |_| {
        if let Some((svg_content, default_filename)) = generate_current_svg() {
            let filename = current_file
                .get()
                .map(|_| format!("{}.png", export_base()))
                .unwrap_or_else(|| default_filename.replace(".svg", ".png"));
            super::file_handler::export_svg_as_png(&filename, &svg_content, 2.0);
        }
    };

    let on_export_jpeg = move |_| {
        if let Some((svg_content, default_filename)) = generate_current_svg() {
            let filename = current_file
                .get()
                .map(|_| format!("{}.jpg", export_base()))
                .unwrap_or_else(|| default_filename.replace(".svg", ".jpg"));
            super::file_handler::export_svg_as_jpeg(&filename, &svg_content, 2.0, 0.92);
        }
    };

    let on_toggle_theme = move |_| {
        set_theme_mode.update(|m| *m = m.toggle());
    };

    // Chart images (PNG/JPEG/...) are digitized into an SPD via a dialog
    // instead of being parsed as text.
    let pending_chart = RwSignal::new(None::<super::spd_digitize::PendingChart>);

    // A file handed over by another page (the TM-25 viewer's "Open in editor"):
    // `localStorage["eulumdat_pending_file"]` = {"name","content"}, consumed once.
    Effect::new(move |_| {
        let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) else {
            return;
        };
        let Ok(Some(raw)) = storage.get_item(PENDING_FILE_KEY) else {
            return;
        };
        let _ = storage.remove_item(PENDING_FILE_KEY);
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
            let name = v
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("pending.ldt");
            if let Some(content) = v.get("content").and_then(|c| c.as_str()) {
                load_file_content(
                    name.to_string(),
                    content.to_string(),
                    crate::library::LibrarySource::LocalFile,
                );
            }
        }
    });

    // Every way of opening files (file picker, drop zones, the Compare
    // tab's sensor drop zone) ends here; several files at once are fine.
    let open_file_list = move |files: Vec<web_sys::File>| {
        let mut viewer_opened = false;
        let mut plain = Vec::new();
        for file in files {
            let name = file.name();
            if super::tm25_viewer::is_tm25_file(&name) {
                if !viewer_opened {
                    viewer_opened = true;
                    super::tm25_viewer::open_in_viewer(file);
                }
                continue;
            }
            if super::spd_digitize::is_chart_image(&name) {
                if !viewer_opened {
                    viewer_opened = true;
                    super::spd_digitize::start(file, pending_chart);
                }
                continue;
            }
            if is_gldf_file(&name) {
                open_gldf(
                    file,
                    body_mesh,
                    projects,
                    load_file_content,
                    move || {
                        set_view_mode.set(ViewMode::Editor);
                        set_active_tab.set(Tab::SensorTab);
                    },
                    move |sensor: String| {
                        if active_tab.get_untracked() == Tab::CompareTab
                            && view_mode.get_untracked() == ViewMode::Editor
                        {
                            if !compare_requests.sensors_mode.get_untracked() {
                                compare_requests.sensors_mode.set(true);
                            }
                            compare_requests.pick.set(Some(sensor));
                        }
                    },
                );
                continue;
            }
            plain.push(file);
        }
        if !plain.is_empty() {
            let load_content = load_file_content;
            wasm_bindgen_futures::spawn_local(async move {
                for file in plain {
                    let name = file.name();
                    if let Ok(content) = gloo_file::futures::read_as_text(&file.into()).await {
                        load_content(name, content, crate::library::LibrarySource::LocalFile);
                    }
                }
            });
        }
    };
    provide_context(OpenFiles(StoredValue::new_local(
        Box::new(open_file_list) as OpenFilesFn
    )));

    // A sensor of the project, picked from the project menu: the Interior /
    // outdoor designer or the Compare tab use it, elsewhere its sensor page
    // opens.
    let open_sensor_named = move |name: String| {
        if view_mode.get_untracked() == ViewMode::Editor {
            match active_tab.get_untracked() {
                // Interior or outdoor designer: whichever is shown uses it.
                Tab::ZonalDesignerTab | Tab::AreaDesignerTab => {
                    interior_requests.pick.set(Some(name));
                    return;
                }
                Tab::CompareTab => {
                    if !compare_requests.sensors_mode.get_untracked() {
                        compare_requests.sensors_mode.set(true);
                    }
                    compare_requests.pick.set(Some(name));
                    return;
                }
                _ => {}
            }
        }
        let idx = sensor_files.with_untracked(|v| v.iter().position(|f| f.name == name));
        sensor_ui.use_loaded.set(false);
        sensor_ui.selected.set(idx.unwrap_or(0));
        set_view_mode.set(ViewMode::Editor);
        set_active_tab.set(Tab::SensorTab);
    };
    // A luminaire of the project into the editor: its kind is known, it is
    // in the project already.
    let open_item = move |it: crate::projects::ProjectItem| {
        force_luminaire.set_value(true);
        from_project.set_value(true);
        load_file_content(it.name, it.content, it.source);
    };
    // The current project's luminaire (the one open last), on request:
    // after a reload, a project picked or a project file opened. The Demo
    // opens the template opened last.
    Effect::new(move |_| {
        if projects.open_active.get() == 0 {
            return;
        }
        untrack(|| {
            if projects.current_id.get_untracked() == crate::projects::DEMO_ID {
                let id = crate::projects::load_demo_luminaire();
                if let Some(t) = ALL_TEMPLATES.iter().find(|t| Some(t.id) == id.as_deref()) {
                    load_template(
                        t,
                        set_atla_doc,
                        set_current_file,
                        set_selected_lamp_set,
                        set_templates_loading,
                        rotate_c_planes,
                    );
                }
            } else if let Some(it) = projects.active_item() {
                open_item(it);
            }
        });
    });
    // An entry of the project menu: its kind flag decides, nothing is
    // detected again.
    let open_entry = move |e: super::projects_ui::ProjectEntry| {
        use super::projects_ui::EntrySource;
        // The menu stays open while it has the focus.
        if let Some(el) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.active_element())
        {
            if let Ok(el) = el.dyn_into::<web_sys::HtmlElement>() {
                let _ = el.blur();
            }
        }
        match (e.kind, e.source) {
            (crate::projects::ItemKind::Luminaire, EntrySource::Template(i)) => {
                if let Some(t) = ALL_TEMPLATES.get(i) {
                    load_template(
                        t,
                        set_atla_doc,
                        set_current_file,
                        set_selected_lamp_set,
                        set_templates_loading,
                        rotate_c_planes,
                    );
                }
            }
            (crate::projects::ItemKind::Luminaire, EntrySource::Item(id)) => {
                if let Some(it) = projects.item(&id) {
                    open_item(it);
                }
            }
            (crate::projects::ItemKind::Sensor, _) => open_sensor_named(e.name),
            // A saved Light scene: its luminaire (if it is in the project),
            // then the scene on the Light scene tab.
            (crate::projects::ItemKind::Scene, EntrySource::Item(id)) => {
                let Some(doc) = projects.item(&id).and_then(|it| {
                    serde_json::from_str::<eulumdat_scene::doc::SceneDoc>(&it.content).ok()
                }) else {
                    return;
                };
                if let Some(it) = doc.luminaire.as_deref().and_then(|l| projects.item(l)) {
                    open_item(it);
                }
                if view_mode.get_untracked() != ViewMode::Editor {
                    set_view_mode.set(ViewMode::Editor);
                }
                if active_tab.get_untracked() != Tab::SceneRtTab {
                    set_active_tab.set(Tab::SceneRtTab);
                }
                light_scene_doc.0.set(Some(std::sync::Arc::new(doc)));
            }
            _ => {}
        }
    };

    let on_file_input = move |ev: ev::Event| {
        let input: HtmlInputElement = ev.target().unwrap().unchecked_into();
        if let Some(list) = input.files() {
            open_file_list((0..list.length()).filter_map(|i| list.get(i)).collect());
        }
        input.set_value("");
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
        if let Some(list) = ev.data_transfer().and_then(|d| d.files()) {
            open_file_list((0..list.length()).filter_map(|i| list.get(i)).collect());
        }
    };

    // Note: Child components use the `ldc` Memo which derives from atla_doc.
    // When they call set_ldc, it converts back to ATLA internally.

    view! {
        <ThemeProvider mode=theme_mode>
            <div class=move || format!("app {}", theme_mode.get().class_name()) role="application" aria-label="Candelabra">
                // Skip to main content link for keyboard users
                <a href="#main-content" class="skip-link">{move || locale.get().ui.header.skip_link.clone()}</a>

                // Header with navigation landmark
                <header class="header" role="banner">
                    <h1><a class="brand-link" href="../" title="Candelabra">{move || locale.get().ui.header.title.clone()}</a></h1>
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
                                        accept=".ldt,.LDT,.ies,.IES,.gldf,.GLDF,.xml,.XML,.json,.JSON,.spdx,.SPDX,.spd,.SPD,.csv,.CSV,.oxl,.OXL,.oxc,.OXC,.png,.PNG,.jpg,.JPG,.jpeg,.JPEG,.webp,.WEBP,.tm25ray,.TM25RAY,.ray,.RAY"
                                        multiple
                                        style="display: none;"
                                        on:change=on_file_input
                                        aria-label="Open LDT, IES, SPDX, XML, or JSON file"
                                    />
                                </label>
                                // Project: the whole project as one file
                                <div class="menu-divider"></div>
                                <div class="menu-section">
                                    {move || {
                                        let l = locale.get();
                                        format!(
                                            "{} · {}",
                                            l.ui.header.file_menu.section_project,
                                            projects.current_name().unwrap_or_else(|| l.dashboard.projects.demo.clone())
                                        )
                                    }}
                                </div>
                                <button
                                    class=move || if projects.is_demo() { "menu-item disabled" } else { "menu-item" }
                                    disabled=move || projects.is_demo()
                                    title=move || {
                                        let fm = locale.get().ui.header.file_menu;
                                        if projects.is_demo() { fm.project_demo_note } else { fm.export_project_tooltip }
                                    }
                                    on:click=move |_| projects.export_current_with(export_scenes.get_untracked())
                                >
                                    {move || format!("💾 {}", locale.get().ui.header.file_menu.export_project)}
                                </button>
                                {move || {
                                    let n = projects.scene_count();
                                    (n > 0 && !projects.is_demo()).then(|| view! {
                                        <label class="menu-item checkbox-item menu-sub">
                                            <input
                                                type="checkbox"
                                                prop:checked=move || export_scenes.get()
                                                on:change=move |ev| {
                                                    let on = ev.target().unwrap().unchecked_into::<HtmlInputElement>().checked();
                                                    export_scenes.set(on);
                                                    if let Some(st) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
                                                        let _ = st.set_item(EXPORT_SCENES_KEY, if on { "1" } else { "0" });
                                                    }
                                                }
                                            />
                                            {move || format!(" {}", locale.get().ui.header.file_menu.include_scenes.replace("{n}", &n.to_string()))}
                                        </label>
                                    })
                                }}
                                // Luminaire: the one in the editor, in every format
                                <div class="menu-divider"></div>
                                <div class="menu-section">
                                    {move || {
                                        let l = locale.get();
                                        let lum = ldc.get();
                                        let name = [lum.luminaire_name.trim(), lum.luminaire_number.trim()]
                                            .into_iter()
                                            .find(|s| !s.is_empty())
                                            .map(str::to_string)
                                            .or_else(|| current_file.get());
                                        match name {
                                            Some(n) => format!("{} · {}", l.ui.header.file_menu.section_luminaire, n),
                                            None => l.ui.header.file_menu.section_luminaire.clone(),
                                        }
                                    }}
                                </div>
                                <button class="menu-item" on:click=on_save_ldt>
                                    {move || locale.get().ui.header.file_menu.ldt.clone()}
                                </button>
                                <button class="menu-item" on:click=on_export_ies>
                                    {move || locale.get().ui.header.file_menu.ies.clone()}
                                </button>
                                <label class="menu-item checkbox-item menu-sub" title=move || locale.get().ui.header.rotate_c_planes_tooltip.clone()>
                                    <input
                                        type="checkbox"
                                        prop:checked=move || rotate_c_planes.get()
                                        on:change=move |ev| {
                                            let checked = ev.target().unwrap().unchecked_into::<HtmlInputElement>().checked();
                                            set_rotate_c_planes.set(checked);
                                        }
                                    />
                                    {move || format!(" {}", locale.get().ui.header.rotate_c_planes)}
                                </label>
                                {move || {
                                    let l = locale.get();
                                    let fm = l.ui.header.file_menu.clone();
                                    let c = tm33_contents.get();
                                    let mut carries: Vec<String> = Vec::new();
                                    if c.photometry { carries.push(fm.has_photometry.clone()); }
                                    if c.spectrum { carries.push(fm.has_spectrum.clone()); }
                                    if c.colour { carries.push(fm.has_colour.clone()); }
                                    if c.bim { carries.push(fm.has_bim.clone()); }
                                    if c.custom { carries.push(fm.has_custom.clone()); }
                                    let hint = carries.join(" · ");
                                    let tip = if c.bim { fm.tm32_tooltip.clone() } else { fm.tm33_tooltip.clone() };
                                    let (xml, json) = if c.bim {
                                        (fm.tm32_xml.clone(), fm.tm32_json.clone())
                                    } else {
                                        (fm.tm33_xml.clone(), fm.tm33_json.clone())
                                    };
                                    let hint2 = hint.clone();
                                    let tip2 = tip.clone();
                                    view! {
                                        <button class="menu-item menu-item-2line" title=tip on:click=on_export_tm33_xml>
                                            <span>{xml}</span>
                                            <span class="menu-hint">{hint}</span>
                                        </button>
                                        <button class="menu-item menu-item-2line" title=tip2 on:click=on_export_tm33_json>
                                            <span>{json}</span>
                                            <span class="menu-hint">{hint2}</span>
                                        </button>
                                        {c.spectrum.then(|| view! {
                                            <button class="menu-item menu-item-2line" title=fm.tm27_tooltip.clone() on:click=on_export_spdx>
                                                <span>{fm.tm27.clone()}</span>
                                                <span class="menu-hint">{fm.tm27_hint.clone()}</span>
                                            </button>
                                        })}
                                        <div class="menu-item menu-row" title=fm.atla_tooltip.clone()>
                                            <span class="menu-row-label">{fm.atla.clone()}</span>
                                            <span class="menu-row-actions">
                                                <button class="menu-chip" on:click=on_export_atla_xml>"XML"</button>
                                                <button class="menu-chip" on:click=on_export_atla_json>"JSON"</button>
                                            </span>
                                        </div>
                                    }
                                }}
                                <div class="menu-divider"></div>
                                <div class="menu-section">{move || locale.get().ui.header.file_menu.section_diagram.clone()}</div>
                                <button
                                    class=move || {
                                        if generate_current_svg().is_some() { "menu-item" } else { "menu-item disabled" }
                                    }
                                    on:click=on_export_svg
                                    disabled=move || generate_current_svg().is_none()
                                >
                                    {move || {
                                        let tab = active_tab.get();
                                        let loc = locale.get();
                                        let prefix = loc.ui.header.export_svg_prefix.clone();
                                        match tab {
                                            Tab::Diagram2D => match diagram_type.get() {
                                                DiagramType::Polar => format!("{} ({})", prefix, loc.ui.tabs.polar),
                                                DiagramType::Cartesian => format!("{} ({})", prefix, loc.ui.tabs.cartesian),
                                                DiagramType::BeamAngle => format!("{} ({})", prefix, loc.ui.tabs.beam_angle),
                                            },
                                            Tab::Diagram3D => format!("{} ({})", prefix, loc.ui.tabs.diagram_3d),
                                            Tab::Heatmap => format!("{} ({})", prefix, loc.ui.tabs.heatmap),
                                            Tab::Cone => format!("{} ({})", prefix, loc.ui.tabs.cone),
                                            Tab::Spectral => format!("{} ({})", prefix, loc.ui.tabs.spectral),
                                            Tab::Greenhouse => format!("{} ({})", prefix, loc.ui.tabs.greenhouse),
                                            Tab::BugRating => format!("{} ({})", prefix, loc.ui.tabs.bug_rating),
                                            Tab::Lcs => format!("{} ({})", prefix, loc.ui.tabs.lcs),
                                            Tab::FloodlightVH => format!("{} ({})", prefix, loc.ui.tabs.floodlight_vh),
                                            Tab::FloodlightIsolux => format!("{} ({})", prefix, loc.ui.tabs.floodlight_isolux),
                                            Tab::FloodlightIsoView => format!("{} ({})", prefix, loc.ui.tabs.iso_view),
                                            Tab::FloodlightIsocandela => format!("{} ({})", prefix, loc.ui.tabs.floodlight_isocandela),
                                            _ => prefix,
                                        }
                                    }}
                                </button>
                                <button
                                    class=move || {
                                        if generate_current_svg().is_some() { "menu-item" } else { "menu-item disabled" }
                                    }
                                    on:click=on_export_png
                                    disabled=move || generate_current_svg().is_none()
                                >
                                    {move || locale.get().ui.header.export_png.clone()}
                                </button>
                                <button
                                    class=move || {
                                        if generate_current_svg().is_some() { "menu-item" } else { "menu-item disabled" }
                                    }
                                    on:click=on_export_jpeg
                                    disabled=move || generate_current_svg().is_none()
                                >
                                    {move || locale.get().ui.header.export_jpeg.clone()}
                                </button>
                                // PDF/Typst export - only shown on secret URL
                                {move || export_enabled.then(|| view! {
                                    <button
                                        class="menu-item"
                                        on:click=on_export_report_pdf
                                        disabled=move || pdf_exporting.get()
                                        title=move || locale.get().ui.header.export_pdf_tooltip.clone()
                                    >
                                        {move || if pdf_exporting.get() { locale.get().ui.header.generating_pdf.clone() } else { locale.get().ui.header.export_pdf.clone() }}
                                    </button>
                                    <button
                                        class="menu-item"
                                        on:click=on_export_report_typ
                                        title=move || locale.get().ui.header.export_typ_tooltip.clone()
                                    >
                                        {move || locale.get().ui.header.export_typ.clone()}
                                    </button>
                                })}
                                <div class="menu-divider"></div>
                                <a
                                    class="menu-item"
                                    href="https://github.com/holg/eulumdat-rs"
                                    target="_blank"
                                    rel="noopener noreferrer"
                                >
                                    {move || locale.get().ui.header.github.clone()}
                                </a>
                                <button
                                    class="menu-item"
                                    on:click=move |_| set_show_about.set(true)
                                >
                                    {move || locale.get().ui.header.about.clone()}
                                </button>
                            </div>
                        </div>
                        // Project menu: the current project's luminaires and sensors
                        // (the Demo project = the bundled templates + demo sensors)
                        <div class="file-menu">
                            <button class="btn btn-secondary file-menu-toggle">
                                {move || format!("📁 {}", projects.current_name()
                                    .unwrap_or_else(|| locale.get().dashboard.projects.demo.clone()))}
                                <span class="dropdown-arrow">" ▾"</span>
                            </button>
                            <div class="file-menu-dropdown templates-dropdown">
                                {move || {
                                    let l = locale.get();
                                    use crate::projects::ItemKind;
                                    let entries = projects.entries();
                                    let of = |k: ItemKind| -> Vec<_> {
                                        entries.iter().filter(|e| e.kind == k).cloned().collect()
                                    };
                                    let (lums, sens, scenes) =
                                        (of(ItemKind::Luminaire), of(ItemKind::Sensor), of(ItemKind::Scene));
                                    let item = move |e: super::projects_ui::ProjectEntry, l: &Locale| {
                                        let (label, title) = match &e.source {
                                            super::projects_ui::EntrySource::Template(i) => {
                                                let t = ALL_TEMPLATES[*i];
                                                (get_template_name(t.name, l), t.description.to_string())
                                            }
                                            _ => (e.name.clone(), e.name.clone()),
                                        };
                                        view! {
                                            <button class="menu-item" title=title
                                                on:click=move |_| open_entry(e.clone())
                                            >{label}</button>
                                        }
                                    };
                                    let n_sens = sens.len();
                                    let empty = (lums.is_empty() && sens.is_empty())
                                        .then(|| l.dashboard.projects.empty.clone());
                                    view! {
                                        <div class="menu-section">{format!("💡 {}", l.dashboard.projects.luminaires_title)}</div>
                                        {empty.map(|t| view! { <div class="menu-note">{t}</div> })}
                                        {lums.into_iter().map(|e| item(e, &l)).collect_view()}
                                        {(n_sens > 0).then(|| view! {
                                            <div class="menu-section">{format!("📡 {}", l.dashboard.projects.sensors_title)}</div>
                                        })}
                                        {sens.into_iter().map(|e| item(e, &l)).collect_view()}
                                        {(!scenes.is_empty()).then(|| view! {
                                            <div class="menu-section">{format!("🎬 {}", l.dashboard.projects.scenes_title)}</div>
                                        })}
                                        {scenes.into_iter().map(|e| item(e, &l)).collect_view()}
                                    }
                                }}
                            </div>
                        </div>
                        // Templates loading indicator
                        {move || templates_loading.get().then(|| view! {
                            <span class="templates-loading">{move || locale.get().ui.header.loading.clone()}</span>
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
                                UnitSystem::Metric => locale.get().ui.header.switch_to_imperial.clone(),
                                UnitSystem::Imperial => locale.get().ui.header.switch_to_metric.clone(),
                            }
                        >
                            {move || match unit_system.get() {
                                UnitSystem::Metric => locale.get().ui.header.unit_si.clone(),
                                UnitSystem::Imperial => locale.get().ui.header.unit_imp.clone(),
                            }}
                        </button>
                        <LanguageSelectorCompact />
                    </div>
                </header>

                // File info
                {move || current_file.get().map(|filename| {
                    let maker = atla_doc.with(|d| d.header.manufacturer.clone());
                    let (title, rest) = current_file_parts(&filename, &ldc.get(), maker);
                    view! {
                        <div class="file-info" title=filename.clone()>
                            {locale.get().ui.dropzone.current_file.clone()}" "<strong>{title}</strong>
                            {rest.into_iter().map(|r| view! {
                                <span class="file-info-sep">" · "</span><span class="file-info-part">{r}</span>
                            }).collect_view()}
                        </div>
                    }
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

                // View mode switch: Dashboard overview or Editor
                {move || match view_mode.get() {
                    ViewMode::Dashboard => view! {
                        <Dashboard
                            ldc=ldc
                            on_select=Callback::new(move |(new_ldc, name): (Eulumdat, String)| {
                                // The dashboard hands us the lossless
                                // `Eulumdat` it parsed from an embedded
                                // template. Register it as the pristine
                                // source *before* nudging the signals, so
                                // the save-Effect that fires when the LDC
                                // → ATLA sync runs keeps the lossless
                                // bytes instead of an atla-round-tripped
                                // (potentially lossy) re-serialization.
                                let atla = LuminaireOpticalData::from_eulumdat(&new_ldc);
                                record_pristine_ldc(&new_ldc, &atla.to_eulumdat());
                                if !name.contains(".gldf/") {
                                    body_mesh.set(None);
                                }
                                set_ldc.set(new_ldc);
                                set_current_file.set(Some(name));
                            })
                            on_compare=Callback::new(move |(cmp_ldt, name): (Eulumdat, String)| {
                                set_compare_ldc_b.set(Some(cmp_ldt));
                                set_compare_label_b.set(Some(name));
                            })
                            on_edit=Callback::new(move |_| set_view_mode.set(ViewMode::Editor))
                            on_compare_view=Callback::new(move |_| {
                                set_view_mode.set(ViewMode::Editor);
                                set_active_tab.set(Tab::CompareTab);
                            })
                            on_designer=Callback::new(move |_| {
                                set_view_mode.set(ViewMode::Editor);
                                set_active_tab.set(Tab::AreaDesignerTab);
                            })
                            on_light_scene=Callback::new(move |_| {
                                set_view_mode.set(ViewMode::Editor);
                                set_active_tab.set(Tab::SceneRtTab);
                            })
                            on_feature=Callback::new(move |f: Feature| {
                                let tab = match f {
                                    Feature::Editor => Tab::Diagram2D,
                                    Feature::Validation => Tab::ValidationTab,
                                    Feature::Bug => Tab::BugRating,
                                    Feature::Compare => Tab::CompareTab,
                                    Feature::Bim => Tab::BimTab,
                                    Feature::LightScene | Feature::Gldf => Tab::SceneRtTab,
                                    Feature::Scene3D => Tab::Scene3DTab,
                                    Feature::Area => Tab::AreaDesignerTab,
                                    Feature::Interior => Tab::ZonalDesignerTab,
                                    Feature::Maps => {
                                        super::site_map::request_site_map();
                                        Tab::AreaDesignerTab
                                    }
                                    Feature::Street => Tab::StreetDesignTab,
                                    Feature::Spectral | Feature::Digitizer => Tab::Spectral,
                                    // The card needs a spectrum: without one, open the demo LED.
                                    Feature::ColourQuality => {
                                        let has_spectrum = atla_doc
                                            .get_untracked()
                                            .emitters
                                            .iter()
                                            .any(|e| e.spectral_distribution.is_some());
                                        if !has_spectrum {
                                            if let Some(demo) = super::templates::ALL_TEMPLATES
                                                .iter()
                                                .find(|t| t.id == "demo-led-3000k")
                                            {
                                                load_template(
                                                    demo,
                                                    set_atla_doc,
                                                    set_current_file,
                                                    set_selected_lamp_set,
                                                    set_templates_loading,
                                                    rotate_c_planes,
                                                );
                                            }
                                        }
                                        spectral_view.0.set(Some("colour-quality".into()));
                                        Tab::Spectral
                                    }
                                    Feature::SpectrumLab => Tab::SpectrumDemoTab,
                                    Feature::Daylight => Tab::DaylightSkyTab,
                                    Feature::Sensor => Tab::SensorTab,
                                    Feature::Gonio => Tab::GonioSimTab,
                                    // Own pages (greenhouse, skyglow, TM-25, quiz) and the
                                    // PDF report are handled on the dashboard itself.
                                    _ => return,
                                };
                                set_view_mode.set(ViewMode::Editor);
                                set_active_tab.set(tab);
                            })
                            on_export_pdf=Callback::new(move |export_ldt: Eulumdat| {
                                let default_name = current_file
                                    .get()
                                    .map(|_| format!("{}.pdf", export_base()))
                                    .unwrap_or_else(|| "photometric_report.pdf".to_string());
                                // same rules as the File menu: sensor chapter / sensor report;
                                // the spectrum only belongs to the luminaire in the editor
                                let current = ldc.get_untracked();
                                let doc = atla_doc.get_untracked();
                                let spectrum = (export_ldt == current)
                                    .then(|| report_spectrum(&doc))
                                    .flatten();
                                let (typst_source, filename) = report_typst(
                                    &sensor_ui,
                                    &export_ldt,
                                    &current,
                                    &locale.get_untracked().ui.sensor.loaded_ldt,
                                    default_name,
                                    spectrum.as_ref().map(|(spd, n)| (spd, n.as_str())),
                                );
                                set_pdf_exporting.set(true);
                                wasm_bindgen_futures::spawn_local(async move {
                                    match compile_typst_to_pdf(&typst_source).await {
                                        Ok(pdf_bytes) => {
                                            super::file_handler::download_bytes(
                                                &filename,
                                                &pdf_bytes,
                                                "application/pdf",
                                            );
                                        }
                                        Err(e) => {
                                            web_sys::console::error_1(
                                                &format!("PDF export failed: {}", e).into(),
                                            );
                                            let typ_filename = filename.replace(".pdf", ".typ");
                                            super::file_handler::download_file(
                                                &typ_filename,
                                                &typst_source,
                                                "text/plain",
                                            );
                                        }
                                    }
                                    set_pdf_exporting.set(false);
                                });
                            })
                            library=Signal::from(project_library)
                            on_library_remove=Callback::new(move |id: String| projects.remove(&id))
                            on_library_clear=Callback::new(move |_| projects.remove_luminaires())
                            show_templates=Signal::derive(move || projects.is_demo())
                            on_compare_sensors=Callback::new(move |(a, b): (String, String)| {
                                if !compare_requests.sensors_mode.get_untracked() {
                                    compare_requests.sensors_mode.set(true);
                                }
                                compare_requests.pair.set(Some((a, b)));
                                set_view_mode.set(ViewMode::Editor);
                                set_active_tab.set(Tab::CompareTab);
                            })
                            on_open_sensor=Callback::new(move |i: usize| {
                                sensor_ui.use_loaded.set(false);
                                sensor_ui.selected.set(i);
                                set_view_mode.set(ViewMode::Editor);
                                set_active_tab.set(Tab::SensorTab);
                            })
                        />
                    }.into_any(),
                    ViewMode::Editor => view! {
                        <main id="main-content" class="main-content" role="main" aria-label="Editor content">
                            <div class="editor-back-bar">
                                <button
                                    class="btn btn-secondary btn-sm"
                                    on:click=move |_| set_view_mode.set(ViewMode::Dashboard)
                                >
                                    {move || format!("\u{2190} {}", locale.get().dashboard.back)}
                                </button>
                            </div>
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
                                            title=move || locale.get().ui.bim.tooltip.clone()
                                        >
                                            {move || locale.get().ui.tabs.bim.clone()}
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
                            <button
                                class=move || format!("tab{}", if active_main_tab.get() == MainTab::AreaDesigner { " active" } else { "" })
                                on:click=move |_| set_active_tab.set(Tab::default_for_main(MainTab::AreaDesigner))
                            >
                                {move || format!("📐 {}", locale.get().ui.tabs.area_designer)}
                            </button>
                            <button
                                class=move || format!("tab{}", if active_main_tab.get() == MainTab::ZonalDesigner { " active" } else { "" })
                                on:click=move |_| set_active_tab.set(Tab::default_for_main(MainTab::ZonalDesigner))
                            >
                                {move || format!("🏢 {}", locale.get().ui.tabs.zonal_designer)}
                            </button>
                            <button
                                class=move || format!("tab{}", if active_main_tab.get() == MainTab::GonioSim { " active" } else { "" })
                                on:click=move |_| set_active_tab.set(Tab::default_for_main(MainTab::GonioSim))
                            >
                                {move || format!("🔬 {}", locale.get().goniosim.title)}
                            </button>
                            <button
                                class=move || format!("tab{}", if active_main_tab.get() == MainTab::SpectrumDemo { " active" } else { "" })
                                on:click=move |_| set_active_tab.set(Tab::default_for_main(MainTab::SpectrumDemo))
                            >
                                {move || format!("🌈 {}", locale.get().spectrum_lab.title)}
                            </button>
                            <button
                                class=move || format!("tab{}", if active_main_tab.get() == MainTab::DaylightSky { " active" } else { "" })
                                on:click=move |_| set_active_tab.set(Tab::default_for_main(MainTab::DaylightSky))
                            >
                                {move || format!("🌇 {}", locale.get().spectrum_lab.sky_tab_title)}
                            </button>
                            <button
                                class=move || format!("tab{}", if active_main_tab.get() == MainTab::StreetDesign { " active" } else { "" })
                                on:click=move |_| set_active_tab.set(Tab::default_for_main(MainTab::StreetDesign))
                            >
                                {move || locale.get().ui.tabs.street_design.clone()}
                            </button>
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
                                                Tab::FloodlightIsoView => locale.get().ui.tabs.iso_view.clone(),
                                                Tab::FloodlightIsocandela => locale.get().ui.tabs.floodlight_isocandela.clone(),
                                                Tab::ValidationTab => locale.get().ui.tabs.validation.clone(),
                                                Tab::CompareTab => locale.get().ui.tabs.compare.clone(),
                                                Tab::BimTab => locale.get().ui.tabs.bim.clone(),
                                                Tab::Scene3DTab => locale.get().ui.tabs.scene_3d.clone(),
                                                Tab::SceneRtTab => locale.get().ui.tabs.scene_rt.clone(),
                                                Tab::AreaDesignerTab => locale.get().ui.tabs.area_designer.clone(),
                                                Tab::ZonalDesignerTab => locale.get().ui.tabs.zonal_designer.clone(),
                                                Tab::GonioSimTab => locale.get().goniosim.title.clone(),
                                                Tab::SpectrumDemoTab => locale.get().spectrum_lab.title.clone(),
                                                Tab::DaylightSkyTab => locale.get().spectrum_lab.sky_tab_title.clone(),
                                                Tab::StreetDesignTab => locale.get().ui.tabs.street_design.clone(),
                                                Tab::SensorTab => locale.get().ui.sensor.tab.clone(),
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
                            // The light scene keeps its canvas mounted: Bevy cannot re-attach to a
                            // new element. Hidden tabs pause it through the `active` flag.
                            <div style=move || if active_tab.get() == Tab::SceneRtTab {
                                "height: 640px; position: relative; display: flex;".to_string()
                            } else {
                                "position: absolute; left: 0; right: 0; height: 640px; visibility: hidden; pointer-events: none; display: flex;".to_string()
                            }>
                                <RtSceneViewer active=Signal::derive(move || active_tab.get() == Tab::SceneRtTab) ldc=ldc />
                            </div>
                            {move || match active_tab.get() {
                                Tab::General => view! {
                                    <GeneralTab ldc=ldc set_ldc=set_ldc />
                                }.into_any(),
                                Tab::Dimensions => view! {
                                    <DimensionsTab ldc=ldc set_ldc=set_ldc />
                                }.into_any(),
                                Tab::LampSets => view! {
                                    <LampSetsTab ldc=ldc set_ldc=set_ldc selected=selected_lamp_set set_selected=set_selected_lamp_set />
                                }.into_any(),
                                Tab::DirectRatios => view! {
                                    <DirectRatiosTab ldc=ldc set_ldc=set_ldc />
                                }.into_any(),
                                Tab::Intensity => view! {
                                    <div class="intensity-tab">
                                        <div class="intensity-table-section">
                                            <div class="table-header">
                                                <span>{move || locale.get().ui.intensity.title.clone()}</span>
                                                <span class="table-info">
                                                    {move || {
                                                        let l = ldc.get();
                                                        let template = locale.get().ui.intensity.table_info.clone();
                                                        template.replace("{c_planes}", &l.c_angles.len().to_string())
                                                            .replace("{g_angles}", &l.g_angles.len().to_string())
                                                    }}
                                                </span>
                                            </div>
                                            <DataTable ldc=ldc set_ldc=set_ldc />
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
                                                        title=move || locale.get().ui.diagram.beam_angle_tooltip.clone()
                                                    >
                                                        {move || locale.get().ui.diagram.beam_angle.clone()}
                                                    </button>
                                                </div>
                                                <span class="zoom-hint">{move || locale.get().ui.diagram.zoom_hint.clone()}</span>
                                            </div>
                                        </div>
                                        <DiagramZoom>
                                            <div class="diagram-fullwidth">
                                                {move || match diagram_type.get() {
                                                    DiagramType::Polar => view! { <PolarDiagram ldc=ldc /> }.into_any(),
                                                    DiagramType::Cartesian => view! { <CartesianDiagram ldc=ldc /> }.into_any(),
                                                    DiagramType::BeamAngle => view! { <BeamAngleDiagram ldc=ldc /> }.into_any(),
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
                                                <Butterfly3D ldc=ldc />
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
                                                <IntensityHeatmap ldc=ldc />
                                            </div>
                                        </DiagramZoom>
                                    </div>
                                }.into_any(),
                                Tab::Cone => {
                                    // Get expanded C-plane angles for the slider
                                    let c_angles = Memo::new(move |_| {
                                        let l = ldc.get();
                                        eulumdat::SymmetryHandler::expand_c_angles(&l)
                                    });
                                    let has_variation = Memo::new(move |_| {
                                        eulumdat::diagram::ConeDiagram::has_c_plane_variation(&ldc.get())
                                    });

                                    view! {
                                        <div class="cone-tab">
                                            <div class="diagram-header">
                                                <span class="diagram-title">{move || locale.get().diagram.title.cone.clone()}</span>
                                                <div class="mounting-height-control">
                                                    <label>{move || locale.get().diagram.cone.mounting_height.clone()}</label>
                                                    <input
                                                        type="range"
                                                        prop:min=move || match unit_system.get() { UnitSystem::Metric => "1", _ => "3" }
                                                        prop:max=move || match unit_system.get() { UnitSystem::Metric => "15", _ => "50" }
                                                        prop:step=move || match unit_system.get() { UnitSystem::Metric => "0.5", _ => "1" }
                                                        prop:value=move || unit_system.get().convert_meters(mounting_height.get()).to_string()
                                                        on:input=move |ev| {
                                                            if let Ok(value) = event_target_value(&ev).parse::<f64>() {
                                                                set_mounting_height.set(unit_system.get().to_meters(value));
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
                                                    <ConeDiagramView ldc=ldc mounting_height=mounting_height c_plane=selected_c_plane />
                                                </div>
                                            </DiagramZoom>
                                            <ConeIlluminanceTableView ldc=ldc mounting_height=mounting_height c_plane=selected_c_plane />
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
                                                        prop:min=move || match unit_system.get() { UnitSystem::Metric => "0.5", _ => "2" }
                                                        prop:max=move || match unit_system.get() { UnitSystem::Metric => "6", _ => "20" }
                                                        prop:step=move || match unit_system.get() { UnitSystem::Metric => "0.5", _ => "1" }
                                                        prop:value=move || unit_system.get().convert_meters(greenhouse_height.get()).to_string()
                                                        on:input=move |ev| {
                                                            if let Ok(value) = event_target_value(&ev).parse::<f64>() {
                                                                set_greenhouse_height.set(unit_system.get().to_meters(value));
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
                                                <BugRating ldc=ldc />
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
                                                <LcsClassification ldc=ldc />
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
                                                <FloodlightCartesian ldc=ldc />
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
                                        <IsoluxFootprint ldc=ldc />
                                    </div>
                                }.into_any(),
                                Tab::FloodlightIsoView => view! {
                                    <div class="iso-view-tab">
                                        <div class="diagram-header">
                                            <span class="diagram-title">{move || locale.get().ui.diagram.title_iso_view.clone()}</span>
                                            <span class="text-muted">{move || locale.get().ui.diagram.desc_iso_view.clone()}</span>
                                        </div>
                                        <IsoluxIsometric ldc=ldc />
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
                                                <IsocandelaDiagramView ldc=ldc />
                                            </div>
                                        </DiagramZoom>
                                    </div>
                                }.into_any(),
                                Tab::ValidationTab => view! {
                                    <div class="validation-tab">
                                        <h3>{move || locale.get().ui.validation.title.clone()}</h3>
                                        <ValidationPanel ldc=ldc />
                                    </div>
                                }.into_any(),
                                Tab::CompareTab => view! {
                                    <div class="compare-tab">
                                        <ComparePanel
                                            ldc=ldc
                                            current_file=current_file
                                            ldc_b=compare_ldc_b
                                            set_ldc_b=set_compare_ldc_b
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
                                Tab::SceneRtTab => view! { <div></div> }.into_any(),
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
                                Tab::AreaDesignerTab => view! {
                                    <div class="area-designer-tab">
                                        <div class="diagram-header">
                                            <span class="diagram-title">{move || locale.get().ui.tabs.area_designer.clone()}</span>
                                        </div>
                                        <AreaDesigner ldc=ldc />
                                    </div>
                                }.into_any(),
                                Tab::ZonalDesignerTab => view! {
                                    <div class="zonal-designer-tab">
                                        <div class="diagram-header">
                                            <span class="diagram-title">{move || locale.get().ui.tabs.zonal_designer.clone()}</span>
                                        </div>
                                        <ZonalDesigner ldc=ldc />
                                    </div>
                                }.into_any(),
                                Tab::GonioSimTab => view! {
                                    <GonioSimDemo ldc=ldc />
                                }.into_any(),
                                Tab::SpectrumDemoTab => view! {
                                    <SpectrumDemo ldc=ldc />
                                }.into_any(),
                                Tab::DaylightSkyTab => view! {
                                    <DaylightSky />
                                }.into_any(),
                                Tab::SensorTab => view! {
                                    <SensorView ldc=ldc />
                                }.into_any(),
                                Tab::StreetDesignTab => view! {
                                    <div class="street-design-tab">
                                        <div class="diagram-header">
                                            <span class="diagram-title">
                                                {move || locale.get().ui.tabs.street_design.clone()}
                                            </span>
                                            <span class="text-muted">
                                                "Multi-luminaire RP-8 / EN 13201 / CJJ 45 / MLO compliance"
                                            </span>
                                        </div>
                                        <StreetLauncher />
                                    </div>
                                }.into_any(),
                            }}
                        </div>
                    </div>
                        </main>
                    }.into_any()
                }}
            </div>

            // Chart image → SPD digitizer dialog
            <super::spd_digitize::DigitizeDialog
                pending=pending_chart
                on_spd=Callback::new(move |(name, spd): (String, String)| {
                    load_file_content(name, spd, crate::library::LibrarySource::LocalFile);
                })
                on_attach=Callback::new(move |spd_text: String| {
                    match eulumdat::atla::spd_loader::parse(&spd_text) {
                        Ok(loaded) => {
                            let target = selected_lamp_set.get_untracked();
                            let mut n = 0;
                            set_atla_doc.update(|doc| {
                                n = eulumdat::atla::spd_loader::attach_spd(doc, &loaded.spd, Some(target));
                            });
                            web_sys::console::info_1(
                                &format!("Attached digitized spectrum ({} samples) to {} emitter(s)", loaded.spd.wavelengths.len(), n).into(),
                            );
                        }
                        Err(e) => web_sys::console::error_1(&format!("Attach failed: {e}").into()),
                    }
                })
                luminaire=Signal::derive(move || {
                    let doc = atla_doc.get();
                    let has_photometry = doc.emitters.iter().any(|e| e.intensity_distribution.is_some());
                    has_photometry.then(|| {
                        current_file.get()
                            .or_else(|| doc.header.description.clone())
                            .unwrap_or_else(|| "loaded luminaire".to_string())
                    })
                })
            />

            // About Modal
            {move || {
                if show_about.get() {
                    view! {
                        <div class="modal-overlay" on:click=move |_| set_show_about.set(false)>
                            <div class="modal-content about-modal" on:click=|e| e.stop_propagation()>
                                <h2>{move || locale.get().ui.about.title.clone()}</h2>
                                <p class="about-subtitle">{move || locale.get().ui.about.subtitle.clone()}</p>
                                <div class="about-description">
                                    <p>{move || locale.get().ui.about.desc_parsing.clone()}</p>
                                    <p>{move || locale.get().ui.about.desc_diagrams.clone()}</p>
                                    <p class="about-highlight">{move || locale.get().ui.about.desc_platforms.clone()}</p>
                                </div>
                                <div class="about-links">
                                    <a href="https://github.com/holg/eulumdat-rs" target="_blank" rel="noopener noreferrer">
                                        {move || locale.get().ui.header.github.clone()}
                                    </a>
                                    <span class="about-version">"v0.4.0"</span>
                                </div>
                                <button class="btn btn-primary" on:click=move |_| set_show_about.set(false)>
                                    {move || locale.get().ui.about.close.clone()}
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
    }.into_any()
}

/// GLDF container (zip with product.xml, photometry and optional L3D body).
fn is_gldf_file(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".gldf")
}

/// Open a GLDF container: the brightest photometry becomes the editor's
/// luminaire (named `container.gldf/ldc/x.ldt`, so it dispatches as LDT /
/// IES) and the first L3D model becomes the Light scene body.
fn open_gldf(
    file: web_sys::File,
    body_mesh: RwSignal<Option<std::sync::Arc<eulumdat_scene::mesh::LuminaireMesh>>>,
    projects: super::projects_ui::ProjectsCtx,
    load_content: impl Fn(String, String, crate::library::LibrarySource) + 'static,
    show_sensors: impl Fn() + 'static,
    picked_sensor: impl Fn(String) + 'static,
) {
    let name = file.name();
    wasm_bindgen_futures::spawn_local(async move {
        let Ok(bytes) = gloo_file::futures::read_as_bytes(&file.into()).await else {
            web_sys::console::error_1(&"GLDF: could not read the file".into());
            return;
        };
        let Some(x) = eulumdat_scene::l3d::gldf_extract(&bytes) else {
            web_sys::console::error_1(
                &"GLDF: no LDT / IES photometry and no sensor files in the container".into(),
            );
            return;
        };
        let mesh = x
            .l3d
            .as_deref()
            .and_then(eulumdat_scene::l3d::mesh_source_from_l3d)
            .and_then(|src| eulumdat_scene::mesh::LuminaireMesh::from_source(&src))
            .map(std::sync::Arc::new);
        web_sys::console::info_1(
            &format!(
                "GLDF {}: {} photometries, body {}",
                name,
                x.photometries.len(),
                mesh.as_ref().map_or("none".to_string(), |m| format!(
                    "{} triangles",
                    m.tris.len()
                ))
            )
            .into(),
        );
        // The container's sensors join the current project.
        let has_sensors = !x.sensors.is_empty();
        if has_sensors {
            web_sys::console::info_1(
                &format!("GLDF {}: {} sensor file(s)", name, x.sensors.len()).into(),
            );
        }
        let mut first_sensor = None;
        for sf in &x.sensors {
            if SensorFile::from_bytes(
                &format!("{}/{}", name, sf.name),
                &sf.content_type,
                &sf.bytes,
            )
            .is_none()
            {
                continue;
            }
            let text = match std::str::from_utf8(&sf.bytes) {
                Ok(t) => t.to_string(),
                // EULUMDAT files are often Latin-1.
                Err(_) => sf.bytes.iter().map(|&c| c as char).collect(),
            };
            let item_name = format!("{}/{}", name, sf.name);
            projects.add(
                &item_name,
                text,
                crate::projects::ItemKind::Sensor,
                crate::library::LibrarySource::LocalFile,
            );
            first_sensor.get_or_insert(item_name);
        }
        if let Some(s) = first_sensor {
            picked_sensor(s);
        }
        // Emergency emitters of the variants (GLDF emergencyBehaviour):
        // each joins the project as a luminaire at its emergency flux, for
        // the designers' emergency lighting (EN 1838).
        if let Some(product) = x
            .product_xml
            .as_deref()
            .and_then(|xml| eulumdat::gldf_product::GldfProduct::parse(xml).ok())
        {
            let product_name = product
                .product_name
                .clone()
                .unwrap_or_else(|| name.trim_end_matches(".gldf").to_string());
            for cfg in product.configs(None) {
                for lamp in cfg.emergency_lamps() {
                    let (Some(file), Some(lm)) =
                        (lamp.photometry_file.as_deref(), lamp.emergency_lm)
                    else {
                        continue;
                    };
                    let Some(ph) = x
                        .photometries
                        .iter()
                        .find(|p| p.name == file || p.name.ends_with(&format!("/{file}")))
                    else {
                        continue;
                    };
                    let Ok(l) = Eulumdat::parse(&ph.text) else {
                        continue;
                    };
                    let mut em = eulumdat::emergency::at_flux(&l, lm);
                    em.luminaire_name = format!("{product_name} · emergency mode {lm:.0} lm");
                    projects.add(
                        &format!("{name}/emergency {lm:.0} lm/{file}"),
                        em.to_ldt(),
                        crate::projects::ItemKind::Luminaire,
                        crate::library::LibrarySource::LocalFile,
                    );
                }
            }
        }
        let Some(phot) = x.best_photometry() else {
            // A detector product without luminaire photometry.
            if has_sensors {
                show_sensors();
            }
            return;
        };
        body_mesh.set(mesh);
        load_content(
            format!("{}/{}", name, phot.name),
            phot.text.clone(),
            crate::library::LibrarySource::LocalFile,
        );
    });
}

/// `key=value` pairs of the URL fragment, URL-decoded.
fn parse_hash_params() -> Vec<(String, String)> {
    let Some(window) = web_sys::window() else {
        return Vec::new();
    };
    let Ok(hash) = window.location().hash() else {
        return Vec::new();
    };
    hash.trim_start_matches('#')
        .split('&')
        .filter_map(|part| {
            let (k, v) = part.split_once('=')?;
            let dec = |s: &str| {
                js_sys::decode_uri_component(s)
                    .ok()
                    .and_then(|j| j.as_string())
                    .unwrap_or_else(|| s.to_string())
            };
            Some((dec(k), dec(v)))
        })
        .collect()
}

/// Open files the way the app's file picker does (context): sensors and
/// luminaires are recognised and join the current project.
#[derive(Clone, Copy)]
pub struct OpenFiles(pub StoredValue<OpenFilesFn, LocalStorage>);

/// What [`OpenFiles`] holds.
pub type OpenFilesFn = Box<dyn Fn(Vec<web_sys::File>)>;

impl OpenFiles {
    pub fn open(&self, files: Vec<web_sys::File>) {
        self.0.with_value(|f| f(files));
    }
}

/// Deep-link pairs for the Light scene tab (context).
#[derive(Clone, Copy)]
pub struct RtLinkParams(pub StoredValue<Vec<(String, String)>>);

/// The plan the Light scene shows as its "Planned" scene (context).
#[derive(Clone, Copy)]
pub struct LightScenePlan(pub RwSignal<Option<std::sync::Arc<eulumdat_scene::scene::ScenePlan>>>);

/// A saved Light scene to show; the Light scene takes it and clears it
/// (context).
#[derive(Clone, Copy)]
pub struct LightSceneDoc(pub RwSignal<Option<std::sync::Arc<eulumdat_scene::doc::SceneDoc>>>);

/// Open a designer's plan in the Light scene tab (context).
#[derive(Clone, Copy)]
pub struct OpenLightScene(pub Callback<eulumdat_scene::scene::ScenePlan>);

/// Go back from the Light scene to the Interior (`true`) or Outdoor
/// designer (context).
#[derive(Clone, Copy)]
pub struct OpenDesigner(pub Callback<Designer>);

/// The designer a Light scene plan came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Designer {
    Interior,
    Outdoor,
    Street,
}

/// `window.candelabraOpenStreetScene(json)`: the Street designer bundle
/// opens its layout (`{"layout": StreetLayout, "mf": 0.8}`) in the Light
/// scene as a planned street, with the editor's luminaire at the same
/// maintenance factor (so the values compare with the designer's).
fn register_street_scene_hook(
    open: Callback<eulumdat_scene::scene::ScenePlan>,
    ldc: ReadSignal<Eulumdat>,
) {
    use wasm_bindgen::closure::Closure;
    let hook = Closure::<dyn Fn(String)>::new(move |json: String| {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&json) else {
            web_sys::console::warn_1(&"[Street] unreadable layout".into());
            return;
        };
        let Ok(layout) =
            serde_json::from_value::<eulumdat::street::StreetLayout>(v["layout"].clone())
        else {
            web_sys::console::warn_1(&"[Street] unreadable layout".into());
            return;
        };
        let mf = v["mf"].as_f64().unwrap_or(1.0).clamp(0.1, 1.0);
        let arrangement = match layout.arrangement {
            eulumdat::street::Arrangement::SingleSide => "single-sided",
            eulumdat::street::Arrangement::Opposite => "opposite",
            eulumdat::street::Arrangement::Staggered => "staggered",
        };
        let name = format!(
            "{} × {:.1} m lanes · {:.0} m spacing · {arrangement}",
            layout.num_lanes, layout.lane_width_m, layout.pole_spacing_m
        );
        let mut plan = eulumdat_scene::scene::plan_from_street(&layout, name);
        // the designer's maintained values: the luminaire at MF × its flux
        let ldt = ldc.get_untracked();
        let flux = eulumdat::emergency::luminaire_flux_lm(&ldt);
        if mf < 0.999 && flux > 0.0 {
            plan.photometries
                .push(eulumdat_scene::scene::PlanPhotometry {
                    name: format!("{} × MF {mf:.2}", ldt.luminaire_name.trim()),
                    ldt: eulumdat::emergency::at_flux(&ldt, mf * flux).to_ldt(),
                });
            for l in &mut plan.luminaires {
                l.photometry = 1;
            }
            plan.note = Some(format!(
                "Maintained values (MF {mf:.2}), like the Street designer."
            ));
        }
        open.run(plan);
    });
    if let Some(w) = web_sys::window() {
        let _ = js_sys::Reflect::set(
            &w,
            &wasm_bindgen::JsValue::from_str("candelabraOpenStreetScene"),
            hook.as_ref(),
        );
    }
    hook.forget();
}

/// lux → PPFD factor of the current spectrum (context for the Light scene).
#[derive(Clone, Copy)]
pub struct PpfdFactor(pub RwSignal<Option<f32>>);

/// Melanopic daylight efficacy ratio of the editor's luminaire spectrum.
#[derive(Clone, Copy)]
pub struct MelanopicDer(pub RwSignal<Option<f32>>);

/// Parse a bundled template into the lossless luminaire model.
fn template_to_eulumdat(template: &super::templates::Template) -> Option<Eulumdat> {
    use super::templates::TemplateFormat;
    let content = eulumdat_wasm_templates::get_template_content(template.id)?;
    match template.format {
        TemplateFormat::Ldt => Eulumdat::parse(&content).ok(),
        TemplateFormat::IesLm63 => eulumdat::IesParser::parse(&content).ok(),
        TemplateFormat::AtlaXml | TemplateFormat::AtlaJson => eulumdat::atla::parse(&content)
            .ok()
            .map(|d| d.to_eulumdat()),
    }
}

/// Context: spectrum + name of the luminaire in the editor (see `report_spectrum`).
#[derive(Clone, Copy)]
pub struct ReportSpectrum(pub Signal<Option<(eulumdat::atla::SpectralDistribution, String)>>);

/// Measured spectrum of the luminaire in the editor (first emitter with a
/// spectral distribution) and a name for the report's colour chapter.
fn report_spectrum(
    doc: &LuminaireOpticalData,
) -> Option<(eulumdat::atla::SpectralDistribution, String)> {
    let spd = doc
        .emitters
        .iter()
        .find_map(|e| e.spectral_distribution.clone())?;
    let name = doc
        .header
        .description
        .clone()
        .or_else(|| doc.header.catalog_number.clone())
        .unwrap_or_default();
    Some((spd, name))
}
