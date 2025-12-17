use atla::{Emitter, IntensityDistribution, LuminaireOpticalData};
use eulumdat::{Eulumdat, IesParser};
use leptos::ev;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, HtmlSelectElement};

use super::bevy_scene::BevySceneViewer;
use super::bug_rating::BugRating;
use super::butterfly_3d::Butterfly3D;
use super::cartesian_diagram::CartesianDiagram;
use super::data_table::DataTable;
use super::diagram_zoom::DiagramZoom;
use super::greenhouse_diagram::GreenhouseDiagramView;
use super::intensity_heatmap::IntensityHeatmap;
use super::lcs_classification::LcsClassification;
use super::polar_diagram::PolarDiagram;
use super::spectral_diagram::SpectralDiagramView;
use super::tabs::{DimensionsTab, DirectRatiosTab, GeneralTab, LampSetsTab};
use super::templates::{TemplateFormat, ALL_TEMPLATES};
use super::theme::{ThemeMode, ThemeProvider};
use super::validation_panel::ValidationPanel;

const ATLA_STORAGE_KEY: &str = "eulumdat_current_atla";
const ATLA_TIMESTAMP_KEY: &str = "eulumdat_atla_timestamp";

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

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    General,
    Dimensions,
    LampSets,
    DirectRatios,
    Intensity,
    Diagram2D,
    Diagram3D,
    Heatmap,
    Spectral,
    Greenhouse,
    BugRating,
    Lcs,
    Validation,
    Scene3D,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum DiagramType {
    #[default]
    Polar,
    Cartesian,
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
            let _ = storage.set_item(ATLA_STORAGE_KEY, &ldt_string);
            let timestamp = js_sys::Date::now().to_string();
            let _ = storage.set_item(ATLA_TIMESTAMP_KEY, &timestamp);
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
    let (drag_active, set_drag_active) = signal(false);
    let (diagram_type, set_diagram_type) = signal(DiagramType::default());
    let (theme_mode, set_theme_mode) = signal(detect_system_theme());

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

        // Parse to ATLA format (source of truth)
        if is_ies {
            // IES → Eulumdat → ATLA
            match IesParser::parse(&content) {
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
        let content = eulumdat::IesExporter::export(&atla_doc.get().to_eulumdat());
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

    let on_template_select = move |ev: ev::Event| {
        let select: HtmlSelectElement = ev.target().unwrap().unchecked_into();
        let idx = select.selected_index();
        select.set_selected_index(0);
        if idx > 0 {
            if let Some(template) = ALL_TEMPLATES.get((idx - 1) as usize) {
                let ext = match template.format {
                    TemplateFormat::Ldt => "ldt",
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

                // Parse template based on format (with raw value logging for LDT)
                match template.format {
                    TemplateFormat::Ldt => match Eulumdat::parse(template.content) {
                        Ok(ldt) => {
                            let doc = LuminaireOpticalData::from_eulumdat(&ldt);
                            log_color_data_from_ldt(&filename, &ldt, &doc);
                            set_atla_doc.set(doc);
                            set_current_file.set(Some(filename));
                            set_selected_lamp_set.set(0);
                        }
                        Err(e) => {
                            web_sys::console::error_1(
                                &format!("Failed to parse template: {}", e).into(),
                            );
                        }
                    },
                    TemplateFormat::AtlaXml => match atla::xml::parse(template.content) {
                        Ok(doc) => {
                            log_color_data(&filename, &doc);
                            set_atla_doc.set(doc);
                            set_current_file.set(Some(filename));
                            set_selected_lamp_set.set(0);
                        }
                        Err(e) => {
                            web_sys::console::error_1(
                                &format!("Failed to parse template: {}", e).into(),
                            );
                        }
                    },
                    TemplateFormat::AtlaJson => match atla::json::parse(template.content) {
                        Ok(doc) => {
                            log_color_data(&filename, &doc);
                            set_atla_doc.set(doc);
                            set_current_file.set(Some(filename));
                            set_selected_lamp_set.set(0);
                        }
                        Err(e) => {
                            web_sys::console::error_1(
                                &format!("Failed to parse template: {}", e).into(),
                            );
                        }
                    },
                }
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

    let on_tab_click = move |tab: Tab| {
        move |_| {
            set_active_tab.set(tab);
        }
    };

    // Note: Child components use the `ldt` Memo which derives from atla_doc.
    // When they call set_ldt, it converts back to ATLA internally.

    view! {
        <ThemeProvider mode=theme_mode>
            <div class=move || format!("app {}", theme_mode.get().class_name())>
                // Header
                <header class="header">
                    <h1>"Eulumdat Editor"</h1>
                    <div class="header-actions">
                        <button class="btn btn-secondary" on:click=on_new_file>
                            "New"
                        </button>
                        <select class="btn btn-secondary" on:change=on_template_select>
                            <option value="">"Templates..."</option>
                            {ALL_TEMPLATES.iter().enumerate().map(|(i, t)| {
                                view! {
                                    <option value=i.to_string() title=t.description>
                                        {t.name}
                                    </option>
                                }
                            }).collect_view()}
                        </select>
                        <label class="btn btn-secondary">
                            "Open"
                            <input
                                type="file"
                                accept=".ldt,.LDT,.ies,.IES,.xml,.XML,.json,.JSON"
                                style="display: none;"
                                on:change=on_file_input
                            />
                        </label>
                        <button class="btn btn-primary" on:click=on_save_ldt>
                            "Save LDT"
                        </button>
                        <button class="btn btn-success" on:click=on_export_ies>
                            "Export IES"
                        </button>
                        <button class="btn btn-info" on:click=on_export_atla_xml title="Export as ATLA/TM-33 XML">
                            "ATLA XML"
                        </button>
                        <button class="btn btn-info" on:click=on_export_atla_json title="Export as ATLA/TM-33 JSON">
                            "ATLA JSON"
                        </button>
                        <button
                            class="btn btn-secondary theme-toggle"
                            on:click=on_toggle_theme
                            title=move || match theme_mode.get() {
                                ThemeMode::Light => "Switch to dark mode",
                                ThemeMode::Dark => "Switch to light mode",
                            }
                        >
                            {move || match theme_mode.get() {
                                ThemeMode::Light => "🌙",
                                ThemeMode::Dark => "☀️",
                            }}
                        </button>
                    </div>
                </header>

                // File info
                {move || current_file.get().map(|filename| view! {
                    <div class="file-info">
                        "Current file: "<strong>{filename}</strong>
                    </div>
                })}

                // Drop zone
                <div
                    class=move || format!("drop-zone{}", if drag_active.get() { " active" } else { "" })
                    on:dragover=on_dragover
                    on:dragleave=on_dragleave
                    on:drop=on_drop
                >
                    <p>"Drag and drop an LDT, IES, or ATLA (XML/JSON) file here, or use the Open button above"</p>
                </div>

                // Main content
                <div class="main-content">
                    <div class="panel">
                        // Tabs
                        <div class="tabs">
                            <TabButton tab=Tab::General active_tab=active_tab on_click=on_tab_click(Tab::General) label="General" />
                            <TabButton tab=Tab::Dimensions active_tab=active_tab on_click=on_tab_click(Tab::Dimensions) label="Dimensions" />
                            <TabButton tab=Tab::LampSets active_tab=active_tab on_click=on_tab_click(Tab::LampSets) label="Lamp Sets" />
                            <TabButton tab=Tab::DirectRatios active_tab=active_tab on_click=on_tab_click(Tab::DirectRatios) label="Direct Ratios" />
                            <TabButton tab=Tab::Intensity active_tab=active_tab on_click=on_tab_click(Tab::Intensity) label="Intensity" />
                            <TabButton tab=Tab::Diagram2D active_tab=active_tab on_click=on_tab_click(Tab::Diagram2D) label="2D Diagram" />
                            <TabButton tab=Tab::Diagram3D active_tab=active_tab on_click=on_tab_click(Tab::Diagram3D) label="3D Diagram" />
                            <TabButton tab=Tab::Heatmap active_tab=active_tab on_click=on_tab_click(Tab::Heatmap) label="Heatmap" />
                            {move || {
                                let doc = atla_doc.get();
                                let has_spectral_or_cct = doc.emitters.iter().any(|e| {
                                    e.spectral_distribution.is_some() || e.cct.is_some()
                                });
                                if has_spectral_or_cct {
                                    Some(view! {
                                        <TabButton tab=Tab::Spectral active_tab=active_tab on_click=on_tab_click(Tab::Spectral) label="Spectral" />
                                    })
                                } else {
                                    None
                                }
                            }}
                            <TabButton tab=Tab::Greenhouse active_tab=active_tab on_click=on_tab_click(Tab::Greenhouse) label="Greenhouse" />
                            <TabButton tab=Tab::BugRating active_tab=active_tab on_click=on_tab_click(Tab::BugRating) label="BUG Rating" />
                            <TabButton tab=Tab::Lcs active_tab=active_tab on_click=on_tab_click(Tab::Lcs) label="LCS" />
                            <TabButton tab=Tab::Validation active_tab=active_tab on_click=on_tab_click(Tab::Validation) label="Validation" />
                            <TabButton tab=Tab::Scene3D active_tab=active_tab on_click=on_tab_click(Tab::Scene3D) label="3D Scene" />
                        </div>

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
                                                <span>"Luminous Intensity (cd/klm)"</span>
                                                <span class="table-info">
                                                    {move || {
                                                        let l = ldt.get();
                                                        format!("{} C-planes × {} γ-angles", l.c_angles.len(), l.g_angles.len())
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
                                            <span class="diagram-title">"2D Intensity Distribution"</span>
                                            <div class="diagram-controls">
                                                <div class="diagram-toggle">
                                                    <button
                                                        class=move || format!("btn-toggle{}", if diagram_type.get() == DiagramType::Polar { " active" } else { "" })
                                                        on:click=move |_| set_diagram_type.set(DiagramType::Polar)
                                                    >
                                                        "Polar"
                                                    </button>
                                                    <button
                                                        class=move || format!("btn-toggle{}", if diagram_type.get() == DiagramType::Cartesian { " active" } else { "" })
                                                        on:click=move |_| set_diagram_type.set(DiagramType::Cartesian)
                                                    >
                                                        "Cartesian"
                                                    </button>
                                                </div>
                                                <span class="zoom-hint">"Scroll to zoom | Drag to pan"</span>
                                            </div>
                                        </div>
                                        <DiagramZoom>
                                            <div class="diagram-fullwidth">
                                                {move || match diagram_type.get() {
                                                    DiagramType::Polar => view! { <PolarDiagram ldt=ldt /> }.into_any(),
                                                    DiagramType::Cartesian => view! { <CartesianDiagram ldt=ldt /> }.into_any(),
                                                }}
                                            </div>
                                        </DiagramZoom>
                                    </div>
                                }.into_any(),
                                Tab::Diagram3D => view! {
                                    <div class="diagram-3d-tab">
                                        <div class="diagram-header">
                                            <span class="diagram-title">"3D Butterfly Diagram"</span>
                                            <span class="text-muted">"Drag to rotate | Scroll to zoom | Auto-rotates on load"</span>
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
                                            <span class="diagram-title">"Intensity Heatmap"</span>
                                            <span class="text-muted">"Scroll to zoom | Drag to pan"</span>
                                        </div>
                                        <DiagramZoom>
                                            <div class="diagram-fullwidth">
                                                <IntensityHeatmap ldt=ldt />
                                            </div>
                                        </DiagramZoom>
                                    </div>
                                }.into_any(),
                                Tab::Spectral => {
                                    let is_dark = Memo::new(move |_| theme_mode.get() == ThemeMode::Dark);
                                    view! {
                                        <div class="spectral-tab">
                                            <div class="diagram-header">
                                                <span class="diagram-title">"Spectral Power Distribution"</span>
                                                <span class="text-muted">"ATLA S001 spectral data | CCT/CRI synthesis"</span>
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
                                                <span class="diagram-title">"Greenhouse PPFD"</span>
                                                <span class="text-muted">"µmol/m²/s at mounting distances"</span>
                                            </div>
                                            <DiagramZoom>
                                                <div class="diagram-fullwidth">
                                                    <GreenhouseDiagramView atla_doc=atla_doc dark=is_dark />
                                                </div>
                                            </DiagramZoom>
                                        </div>
                                    }.into_any()
                                },
                                Tab::BugRating => view! {
                                    <div class="bug-rating-tab">
                                        <div class="diagram-header">
                                            <span class="diagram-title">"BUG Rating Analysis"</span>
                                            <span class="text-muted">"IES TM-15-11 | Scroll to zoom | Drag to pan"</span>
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
                                            <span class="diagram-title">"Luminaire Classification System"</span>
                                            <span class="text-muted">"IES TM-15-07 | Scroll to zoom | Drag to pan"</span>
                                        </div>
                                        <DiagramZoom>
                                            <div class="diagram-fullwidth">
                                                <LcsClassification ldt=ldt />
                                            </div>
                                        </DiagramZoom>
                                    </div>
                                }.into_any(),
                                Tab::Validation => view! {
                                    <div class="validation-tab">
                                        <h3>"Validation Results"</h3>
                                        <ValidationPanel ldt=ldt />
                                    </div>
                                }.into_any(),
                                Tab::Scene3D => view! {
                                    <div class="scene-3d-tab">
                                        <div class="diagram-header">
                                            <span class="diagram-title">"3D Scene Viewer"</span>
                                        </div>
                                        <div class="scene-container" style="height: 600px; position: relative;">
                                            <BevySceneViewer />
                                        </div>
                                        <div class="scene-controls">
                                            <p class="text-muted">
                                                "Controls: WASD/Arrows to move • Q/E up/down • Right-click+drag to look • R to reset view • 1-4 for scene types • P for photometric solid • L for luminaire"
                                            </p>
                                        </div>
                                    </div>
                                }.into_any(),
                            }}
                        </div>
                    </div>
                </div>
            </div>
        </ThemeProvider>
    }
}

#[component]
fn TabButton(
    tab: Tab,
    active_tab: ReadSignal<Tab>,
    on_click: impl Fn(ev::MouseEvent) + 'static,
    label: &'static str,
) -> impl IntoView {
    view! {
        <button
            class=move || format!("tab{}", if active_tab.get() == tab { " active" } else { "" })
            on:click=on_click
        >
            {label}
        </button>
    }
}
