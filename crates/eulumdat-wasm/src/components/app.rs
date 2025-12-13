use eulumdat::{Eulumdat, IesParser, LampSet, Symmetry, TypeIndicator};
use leptos::prelude::*;
use leptos::ev;
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, HtmlSelectElement};

use super::bevy_scene::BevySceneViewer;
use super::bug_rating::BugRating;
use super::butterfly_3d::Butterfly3D;
use super::cartesian_diagram::CartesianDiagram;
use super::data_table::DataTable;
use super::diagram_zoom::DiagramZoom;
use super::intensity_heatmap::IntensityHeatmap;
use super::polar_diagram::PolarDiagram;
use super::tabs::{DimensionsTab, DirectRatiosTab, GeneralTab, LampSetsTab};
use super::templates::ALL_TEMPLATES;
use super::theme::{ThemeMode, ThemeProvider};
use super::validation_panel::ValidationPanel;

const LDT_STORAGE_KEY: &str = "eulumdat_current_ldt";
const LDT_TIMESTAMP_KEY: &str = "eulumdat_ldt_timestamp";

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
    BugRating,
    Validation,
    Scene3D,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum DiagramType {
    #[default]
    Polar,
    Cartesian,
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

    ldt.lamp_sets.push(LampSet {
        num_lamps: 1,
        lamp_type: "LED".to_string(),
        total_luminous_flux: 1000.0,
        color_appearance: "3000K".to_string(),
        color_rendering_group: "80".to_string(),
        wattage_with_ballast: 10.0,
    });

    ldt.c_angles = vec![0.0];
    ldt.g_angles = (0..19).map(|i| i as f64 * 5.0).collect();
    ldt.intensities = vec![vec![
        100.0, 99.0, 96.0, 91.0, 84.0, 75.0, 64.0, 51.0, 36.0, 25.0, 16.0, 9.0, 4.0, 2.0, 1.0,
        0.5, 0.2, 0.1, 0.0,
    ]];

    ldt
}

fn detect_system_theme() -> ThemeMode {
    super::theme::detect_system_theme()
}

fn save_ldt_to_storage(ldt: &Eulumdat) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let ldt_string = ldt.to_ldt();
            let _ = storage.set_item(LDT_STORAGE_KEY, &ldt_string);
            let timestamp = js_sys::Date::now().to_string();
            let _ = storage.set_item(LDT_TIMESTAMP_KEY, &timestamp);
        }
    }
}

#[component]
pub fn App() -> impl IntoView {
    // State signals
    let (ldt, set_ldt) = signal(create_default_ldt());
    let (current_file, set_current_file) = signal::<Option<String>>(None);
    let (active_tab, set_active_tab) = signal(Tab::default());
    let (selected_lamp_set, set_selected_lamp_set) = signal(0_usize);
    let (drag_active, set_drag_active) = signal(false);
    let (diagram_type, set_diagram_type) = signal(DiagramType::default());
    let (theme_mode, set_theme_mode) = signal(detect_system_theme());

    // Save to localStorage whenever LDT changes
    Effect::new(move |_| {
        save_ldt_to_storage(&ldt.get());
    });

    // File loading helper
    let load_file_content = move |name: String, content: String| {
        let is_ies = name.to_lowercase().ends_with(".ies");
        let parse_result = if is_ies {
            IesParser::parse(&content)
        } else {
            Eulumdat::parse(&content)
        };

        match parse_result {
            Ok(parsed_ldt) => {
                set_ldt.set(parsed_ldt);
                let display_name = if is_ies {
                    name.replace(".ies", ".ldt").replace(".IES", ".ldt")
                } else {
                    name
                };
                set_current_file.set(Some(display_name));
                set_selected_lamp_set.set(0);
            }
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to parse file: {}", e).into());
            }
        }
    };

    // Handlers
    let on_new_file = move |_| {
        set_ldt.set(create_default_ldt());
        set_current_file.set(None);
        set_selected_lamp_set.set(0);
    };

    let on_save_ldt = move |_| {
        let content = ldt.get().to_ldt();
        let filename = current_file.get().unwrap_or_else(|| "luminaire.ldt".to_string());
        super::file_handler::download_file(&filename, &content, "text/plain");
    };

    let on_export_ies = move |_| {
        let content = eulumdat::IesExporter::export(&ldt.get());
        let filename = current_file
            .get()
            .map(|f| f.replace(".ldt", ".ies"))
            .unwrap_or_else(|| "luminaire.ies".to_string());
        super::file_handler::download_ies(&filename, &content);
    };

    let on_toggle_theme = move |_| {
        set_theme_mode.update(|m| *m = m.toggle());
    };

    let on_file_input = move |ev: ev::Event| {
        let input: HtmlInputElement = ev.target().unwrap().unchecked_into();
        if let Some(files) = input.files() {
            if let Some(file) = files.get(0) {
                let name = file.name();
                let load_content = load_file_content.clone();
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
                match Eulumdat::parse(template.content) {
                    Ok(parsed_ldt) => {
                        set_ldt.set(parsed_ldt);
                        set_current_file.set(Some(format!(
                            "{}.ldt",
                            template.name.to_lowercase().replace(' ', "_")
                        )));
                        set_selected_lamp_set.set(0);
                    }
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to parse template: {}", e).into());
                    }
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
                    let load_content = load_file_content.clone();
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

    // LDT update callback for child components
    let update_ldt = move |f: Box<dyn FnOnce(&mut Eulumdat)>| {
        set_ldt.update(|ldt| f(ldt));
    };

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
                                accept=".ldt,.LDT,.ies,.IES"
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
                    <p>"Drag and drop an LDT or IES file here, or use the Open button above"</p>
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
                            <TabButton tab=Tab::BugRating active_tab=active_tab on_click=on_tab_click(Tab::BugRating) label="BUG Rating" />
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
                                            <span class="text-muted">"Drag to rotate | Auto-rotates on load"</span>
                                        </div>
                                        <div class="diagram-fullwidth">
                                            <Butterfly3D ldt=ldt />
                                        </div>
                                    </div>
                                }.into_any(),
                                Tab::Heatmap => view! {
                                    <div class="heatmap-tab">
                                        <div class="diagram-header">
                                            <span class="diagram-title">"Intensity Heatmap"</span>
                                            <span class="text-muted">"Candela distribution across C-planes and gamma angles"</span>
                                        </div>
                                        <div class="diagram-fullwidth">
                                            <IntensityHeatmap ldt=ldt />
                                        </div>
                                    </div>
                                }.into_any(),
                                Tab::BugRating => view! {
                                    <div class="bug-rating-tab">
                                        <div class="diagram-header">
                                            <span class="diagram-title">"BUG Rating Analysis"</span>
                                            <span class="text-muted">"IES TM-15-11 Backlight, Uplight, Glare"</span>
                                        </div>
                                        <div class="diagram-fullwidth">
                                            <BugRating ldt=ldt />
                                        </div>
                                    </div>
                                }.into_any(),
                                Tab::Validation => view! {
                                    <div class="validation-tab">
                                        <div class="validation-section">
                                            <h3>"Validation Results"</h3>
                                            <ValidationPanel ldt=ldt />
                                        </div>
                                        <div class="summary-section">
                                            <h3>"Summary"</h3>
                                            <div class="info-grid-wide">
                                                <div class="info-item">
                                                    <div class="info-label">"Total Flux"</div>
                                                    <div class="info-value">{move || format!("{:.0} lm", ldt.get().total_luminous_flux())}</div>
                                                </div>
                                                <div class="info-item">
                                                    <div class="info-label">"Total Wattage"</div>
                                                    <div class="info-value">{move || format!("{:.1} W", ldt.get().total_wattage())}</div>
                                                </div>
                                                <div class="info-item">
                                                    <div class="info-label">"Efficacy"</div>
                                                    <div class="info-value">{move || format!("{:.1} lm/W", ldt.get().luminous_efficacy())}</div>
                                                </div>
                                                <div class="info-item">
                                                    <div class="info-label">"Max Intensity"</div>
                                                    <div class="info-value">{move || format!("{:.1} cd/klm", ldt.get().max_intensity())}</div>
                                                </div>
                                                <div class="info-item">
                                                    <div class="info-label">"Symmetry"</div>
                                                    <div class="info-value">{move || ldt.get().symmetry.description()}</div>
                                                </div>
                                                <div class="info-item">
                                                    <div class="info-label">"C-Planes"</div>
                                                    <div class="info-value">{move || {
                                                        let l = ldt.get();
                                                        format!("{} (Mc={})", l.num_c_planes, l.actual_c_planes())
                                                    }}</div>
                                                </div>
                                            </div>
                                        </div>
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
