use eulumdat::diagram::{
    ButterflyDiagram, CartesianDiagram, ConeDiagram, ConeDiagramLabels, FloodlightCartesianDiagram,
    HeatmapDiagram, IsocandelaDiagram, IsoluxDiagram, IsoluxParams, PolarDiagram, SvgTheme, YScale,
};
use eulumdat::{BugDiagram, Eulumdat, IesParser, PhotometricComparison, Significance, UnitSystem};
use leptos::ev;
use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

use super::app::use_unit_system;
use super::templates::{TemplateFormat, ALL_TEMPLATES};
use crate::i18n::use_locale;

/// Compare diagram mode
#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum CompareDiagramMode {
    #[default]
    Polar,
    Cartesian,
    Heatmap,
    Butterfly,
    Cone,
    Isocandela,
    Isolux,
    Floodlight,
    Bug,
    Lcs,
}

impl CompareDiagramMode {
    /// Whether this mode renders two separate side-by-side SVGs
    /// (vs. a single overlay SVG).
    fn is_side_by_side(self) -> bool {
        !matches!(self, Self::Polar | Self::Cartesian)
    }

    fn label(self, locale: &eulumdat_i18n::Locale) -> String {
        match self {
            Self::Polar => locale.ui.tabs.polar.clone(),
            Self::Cartesian => locale.ui.tabs.cartesian.clone(),
            Self::Heatmap => locale.ui.tabs.heatmap.clone(),
            Self::Butterfly => locale.ui.tabs.diagram_3d.clone(),
            Self::Cone => locale.ui.tabs.cone.clone(),
            Self::Isocandela => locale.ui.tabs.floodlight_isocandela.clone(),
            Self::Isolux => locale.ui.tabs.floodlight_isolux.clone(),
            Self::Floodlight => locale.ui.tabs.floodlight.clone(),
            Self::Bug => locale.ui.tabs.bug_rating.clone(),
            Self::Lcs => locale.ui.tabs.lcs.clone(),
        }
    }

    const ALL: &[Self] = &[
        Self::Polar,
        Self::Cartesian,
        Self::Heatmap,
        Self::Butterfly,
        Self::Cone,
        Self::Isocandela,
        Self::Isolux,
        Self::Floodlight,
        Self::Bug,
        Self::Lcs,
    ];
}

/// Parse photometric file content by extension.
fn parse_photometric(name: &str, content: &str) -> Option<Eulumdat> {
    let lower = name.to_lowercase();
    if lower.ends_with(".ies") {
        IesParser::parse(content).ok()
    } else if lower.ends_with(".xml") {
        atla::xml::parse(content).ok().map(|doc| doc.to_eulumdat())
    } else {
        Eulumdat::parse(content).ok()
    }
}

// JS binding for typst PDF compilation (same as app.rs)
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = compileTypstToPdf, catch)]
    async fn compile_typst_to_pdf_js(source: &str) -> Result<JsValue, JsValue>;
}

async fn compile_typst_to_pdf(typst_source: &str) -> Result<Vec<u8>, String> {
    match compile_typst_to_pdf_js(typst_source).await {
        Ok(js_val) => {
            let array = js_sys::Uint8Array::new(&js_val);
            Ok(array.to_vec())
        }
        Err(e) => {
            // Try to extract a meaningful error message
            let msg = e
                .as_string()
                .or_else(|| {
                    // Try .message property (Error objects)
                    js_sys::Reflect::get(&e, &"message".into())
                        .ok()
                        .and_then(|v| v.as_string())
                })
                .or_else(|| {
                    // Try .toString()
                    e.dyn_ref::<js_sys::Object>()
                        .map(|obj| format!("{}", obj.to_string()))
                })
                .unwrap_or_else(|| format!("{:?}", e));
            Err(msg)
        }
    }
}

/// Generate the SVG(s) for a given diagram mode.
/// Returns 1 SVG for overlay modes, 2 SVGs for side-by-side modes.
#[allow(clippy::too_many_arguments)]
fn render_diagrams(
    a: &Eulumdat,
    b: &Eulumdat,
    mode: CompareDiagramMode,
    theme: &SvgTheme,
    la: &str,
    lb: &str,
    units: UnitSystem,
    cone_height: f64,
    cone_c_plane: Option<f64>,
    isolux_params: IsoluxParams,
    polar_c_plane_a: f64,
    polar_c_plane_b: f64,
) -> Vec<String> {
    match mode {
        CompareDiagramMode::Polar => {
            let pa = PolarDiagram::from_eulumdat_for_plane(a, polar_c_plane_a);
            let pb = PolarDiagram::from_eulumdat_for_plane(b, polar_c_plane_b);
            vec![PolarDiagram::to_overlay_svg(
                &pa, &pb, 500.0, 500.0, theme, la, lb,
            )]
        }
        CompareDiagramMode::Cartesian => {
            let ca = CartesianDiagram::from_eulumdat_for_plane(a, polar_c_plane_a, 600.0, 400.0);
            let cb = CartesianDiagram::from_eulumdat_for_plane(b, polar_c_plane_b, 600.0, 400.0);
            vec![CartesianDiagram::to_overlay_svg(
                &ca, &cb, 600.0, 400.0, theme, la, lb,
            )]
        }
        CompareDiagramMode::Heatmap => {
            let ha = HeatmapDiagram::from_eulumdat(a, 400.0, 300.0);
            let hb = HeatmapDiagram::from_eulumdat(b, 400.0, 300.0);
            vec![
                ha.to_svg(400.0, 300.0, theme),
                hb.to_svg(400.0, 300.0, theme),
            ]
        }
        CompareDiagramMode::Butterfly => {
            let ba = ButterflyDiagram::from_eulumdat(a, 400.0, 350.0, 60.0);
            let bb = ButterflyDiagram::from_eulumdat(b, 400.0, 350.0, 60.0);
            vec![
                ba.to_svg(400.0, 350.0, theme),
                bb.to_svg(400.0, 350.0, theme),
            ]
        }
        CompareDiagramMode::Cone => {
            let ca = match cone_c_plane {
                Some(c) => ConeDiagram::from_eulumdat_for_plane(a, cone_height, c),
                None => ConeDiagram::from_eulumdat(a, cone_height),
            };
            let cb = match cone_c_plane {
                Some(c) => ConeDiagram::from_eulumdat_for_plane(b, cone_height, c),
                None => ConeDiagram::from_eulumdat(b, cone_height),
            };
            let labels = ConeDiagramLabels::default();
            vec![
                ca.to_svg_with_units(400.0, 350.0, theme, &labels, units),
                cb.to_svg_with_units(400.0, 350.0, theme, &labels, units),
            ]
        }
        CompareDiagramMode::Isocandela => {
            let ia = IsocandelaDiagram::from_eulumdat(a, 400.0, 350.0);
            let ib = IsocandelaDiagram::from_eulumdat(b, 400.0, 350.0);
            vec![
                ia.to_svg(400.0, 350.0, theme),
                ib.to_svg(400.0, 350.0, theme),
            ]
        }
        CompareDiagramMode::Isolux => {
            let ia = IsoluxDiagram::from_eulumdat_with_units(a, 400.0, 350.0, isolux_params, units);
            let ib = IsoluxDiagram::from_eulumdat_with_units(b, 400.0, 350.0, isolux_params, units);
            vec![
                ia.to_svg_with_units(400.0, 350.0, theme, units),
                ib.to_svg_with_units(400.0, 350.0, theme, units),
            ]
        }
        CompareDiagramMode::Floodlight => {
            let fa = FloodlightCartesianDiagram::from_eulumdat(a, 400.0, 350.0, YScale::Linear);
            let fb = FloodlightCartesianDiagram::from_eulumdat(b, 400.0, 350.0, YScale::Linear);
            vec![
                fa.to_svg(400.0, 350.0, theme),
                fb.to_svg(400.0, 350.0, theme),
            ]
        }
        CompareDiagramMode::Bug => {
            let ba = BugDiagram::from_eulumdat(a);
            let bb = BugDiagram::from_eulumdat(b);
            vec![
                ba.to_svg(400.0, 350.0, theme),
                bb.to_svg(400.0, 350.0, theme),
            ]
        }
        CompareDiagramMode::Lcs => {
            let ba = BugDiagram::from_eulumdat(a);
            let bb = BugDiagram::from_eulumdat(b);
            vec![
                ba.to_lcs_svg(400.0, 300.0, theme),
                bb.to_lcs_svg(400.0, 300.0, theme),
            ]
        }
    }
}

/// Panel for comparing two photometric files side-by-side.
///
/// File B state (`ldt_b`, `label_b`) is owned by the parent (app.rs) so it
/// persists across tab switches.
#[component]
pub fn ComparePanel(
    ldt: ReadSignal<Eulumdat>,
    current_file: ReadSignal<Option<String>>,
    ldt_b: ReadSignal<Option<Eulumdat>>,
    set_ldt_b: WriteSignal<Option<Eulumdat>>,
    label_b: ReadSignal<Option<String>>,
    set_label_b: WriteSignal<Option<String>>,
) -> impl IntoView {
    let unit_system = use_unit_system();
    let locale = use_locale();
    let (diagram_mode, set_diagram_mode) = signal(CompareDiagramMode::default());
    let (drag_active, set_drag_active) = signal(false);

    // Cone diagram controls
    let (cmp_cone_height, set_cmp_cone_height) = signal(3.0_f64);
    let (cmp_c_plane, set_cmp_c_plane) = signal(None::<f64>);

    // Isolux diagram controls
    let (cmp_isolux_height, set_cmp_isolux_height) = signal(10.0_f64);
    let (cmp_tilt_angle, set_cmp_tilt_angle) = signal(0.0_f64);
    let (cmp_area_size, set_cmp_area_size) = signal(20.0_f64);

    // Per-file C-plane selection for Polar/Cartesian overlay
    let (polar_c_plane_a, set_polar_c_plane_a) = signal(0.0_f64);
    let (polar_c_plane_b, set_polar_c_plane_b) = signal(0.0_f64);
    let (link_sliders, set_link_sliders) = signal(true);

    // Expanded C-plane angles (half-circle 0..180) — memoized per file so DOM isn't recreated on slider move
    let half_angles_a = Memo::new(move |_| {
        let angles = eulumdat::SymmetryHandler::expand_c_angles(&ldt.get());
        angles
            .into_iter()
            .filter(|&a| a < 180.0)
            .collect::<Vec<f64>>()
    });
    let half_angles_b = Memo::new(move |_| {
        ldt_b
            .get()
            .map(|b| {
                eulumdat::SymmetryHandler::expand_c_angles(&b)
                    .into_iter()
                    .filter(|&a| a < 180.0)
                    .collect::<Vec<f64>>()
            })
            .unwrap_or_default()
    });

    // File A label derived from current_file
    let label_a = Memo::new(move |_| {
        current_file
            .get()
            .unwrap_or_else(|| locale.get().ui.compare.file_a.clone())
    });

    // Shared helper: load a file into File B state
    let load_file_b = move |name: String, content: String| {
        if let Some(parsed) = parse_photometric(&name, &content) {
            set_ldt_b.set(Some(parsed));
            set_label_b.set(Some(name));
        }
    };

    // Handle File B upload via file picker
    let on_file_b_input = move |ev: ev::Event| {
        let input: HtmlInputElement = ev.target().unwrap().unchecked_into();
        if let Some(files) = input.files() {
            if let Some(file) = files.get(0) {
                let name = file.name();
                let load = load_file_b;
                wasm_bindgen_futures::spawn_local(async move {
                    let text = gloo_file::futures::read_as_text(&file.into()).await;
                    if let Ok(content) = text {
                        load(name, content);
                    }
                });
            }
        }
    };

    // Handle File B drag-and-drop
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
                    let load = load_file_b;
                    wasm_bindgen_futures::spawn_local(async move {
                        let text = gloo_file::futures::read_as_text(&file.into()).await;
                        if let Ok(content) = text {
                            load(name, content);
                        }
                    });
                }
            }
        }
    };

    // Clear File B
    let on_clear_b = move |_: ev::MouseEvent| {
        set_ldt_b.set(None);
        set_label_b.set(None);
    };

    // Handle template selection for File B (async - loads from templates WASM module)
    let on_template_select = move |ev: ev::Event| {
        let select: web_sys::HtmlSelectElement = ev.target().unwrap().unchecked_into();
        let idx: usize = select.value().parse().unwrap_or(usize::MAX);
        if idx < ALL_TEMPLATES.len() {
            let tmpl = ALL_TEMPLATES[idx];
            let id = tmpl.id.to_string();
            let format = tmpl.format;
            let name = tmpl.name.to_string();

            if let Some(content) = eulumdat_wasm_templates::get_template_content(&id) {
                let parsed = match format {
                    TemplateFormat::Ldt => Eulumdat::parse(&content).ok(),
                    TemplateFormat::IesLm63 => IesParser::parse(&content).ok(),
                    TemplateFormat::AtlaXml => {
                        atla::xml::parse(&content).ok().map(|doc| doc.to_eulumdat())
                    }
                    TemplateFormat::AtlaJson => atla::json::parse(&content)
                        .ok()
                        .map(|doc| doc.to_eulumdat()),
                };
                if let Some(ldt) = parsed {
                    set_ldt_b.set(Some(ldt));
                    set_label_b.set(Some(name));
                }
            }
        }
    };

    // Compute comparison as a derived memo
    let comparison = Memo::new(move |_| {
        let a = ldt.get();
        ldt_b.get().map(|b| {
            let la = label_a.get();
            let lb = label_b
                .get()
                .unwrap_or_else(|| locale.get().ui.compare.file_b.clone());
            let units = unit_system.get();
            let loc = locale.get();
            PhotometricComparison::from_eulumdat_with_units_and_locale(
                &a, &b, &la, &lb, units, &loc,
            )
        })
    });

    // Generate diagram SVG(s)
    let diagram_svgs = Memo::new(move |_| {
        let a = ldt.get();
        ldt_b.get().map(|b| {
            let la = label_a.get();
            let lb = label_b
                .get()
                .unwrap_or_else(|| locale.get().ui.compare.file_b.clone());
            let theme = SvgTheme::css_variables();
            let units = unit_system.get();
            let isolux_params = IsoluxParams {
                mounting_height: cmp_isolux_height.get(),
                tilt_angle: cmp_tilt_angle.get(),
                area_half_width: cmp_area_size.get(),
                area_half_depth: cmp_area_size.get(),
                grid_resolution: 60,
            };
            render_diagrams(
                &a,
                &b,
                diagram_mode.get(),
                &theme,
                &la,
                &lb,
                units,
                cmp_cone_height.get(),
                cmp_c_plane.get(),
                isolux_params,
                polar_c_plane_a.get(),
                polar_c_plane_b.get(),
            )
        })
    });

    // Export: Typst source download
    let on_export_compare_typ = move |_: ev::MouseEvent| {
        if let Some(b_ldt) = ldt_b.get() {
            let a_ldt = ldt.get();
            let la = label_a.get();
            let lb = label_b
                .get()
                .unwrap_or_else(|| locale.get().ui.compare.file_b.clone());
            let typst_source = eulumdat_typst::generate_comparison_report(&a_ldt, &b_ldt, &la, &lb);
            super::file_handler::download_file(
                "comparison_report.typ",
                &typst_source,
                "text/plain",
            );
        }
    };

    // Export: PDF via WASM typst compiler
    let (pdf_exporting, set_pdf_exporting) = signal(false);

    let on_export_compare_pdf = move |_: ev::MouseEvent| {
        if let Some(b_ldt) = ldt_b.get() {
            let a_ldt = ldt.get();
            let la = label_a.get();
            let lb = label_b
                .get()
                .unwrap_or_else(|| locale.get().ui.compare.file_b.clone());
            let typst_source = eulumdat_typst::generate_comparison_report(&a_ldt, &b_ldt, &la, &lb);

            set_pdf_exporting.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                web_sys::console::log_1(
                    &format!(
                        "Compiling comparison report ({} chars)...",
                        typst_source.len()
                    )
                    .into(),
                );
                match compile_typst_to_pdf(&typst_source).await {
                    Ok(pdf_bytes) => {
                        super::file_handler::download_bytes(
                            "comparison_report.pdf",
                            &pdf_bytes,
                            "application/pdf",
                        );
                    }
                    Err(e) => {
                        web_sys::console::error_1(&format!("PDF export failed: {}", e).into());
                        web_sys::window()
                            .unwrap()
                            .alert_with_message(&format!(
                                "PDF export failed: {}. Downloading .typ file instead.",
                                e
                            ))
                            .ok();
                        super::file_handler::download_file(
                            "comparison_report.typ",
                            &typst_source,
                            "text/plain",
                        );
                    }
                }
                set_pdf_exporting.set(false);
            });
        }
    };

    view! {
        <div class="compare-panel">
            // File B selector
            <div class="compare-file-controls">
                <h4>{move || locale.get().ui.compare.title.clone()}</h4>

                // Drag-and-drop zone for File B
                <div
                    class=move || format!("compare-drop-zone{}", if drag_active.get() { " active" } else { "" })
                    on:dragover=on_dragover
                    on:dragleave=on_dragleave
                    on:drop=on_drop
                >
                    <p>{move || locale.get().ui.compare.drop_hint.clone()}</p>
                    <div class="compare-inputs">
                        <label class="compare-file-btn">
                            {move || locale.get().ui.compare.browse.clone()}
                            <input
                                type="file"
                                accept=".ldt,.ies,.xml,.json"
                                style="display:none"
                                on:change=on_file_b_input
                            />
                        </label>
                        <span class="compare-or">{move || locale.get().ui.compare.or.clone()}</span>
                        <select class="compare-template-select" on:change=on_template_select>
                            <option value="" disabled selected>{move || locale.get().ui.compare.select_template.clone()}</option>
                            {ALL_TEMPLATES.iter().enumerate().map(|(i, t)| {
                                view! {
                                    <option value={i.to_string()}>{t.name} " - " {t.description}</option>
                                }
                            }).collect::<Vec<_>>()}
                        </select>
                    </div>
                </div>

                {move || label_b.get().map(|name| view! {
                    <div class="compare-file-b-name">
                        <strong>{move || locale.get().ui.compare.file_b_label.clone()}</strong>{name.clone()}
                        <button class="compare-clear-btn" on:click=on_clear_b title=move || locale.get().ui.compare.clear.clone()>"✕"</button>
                    </div>
                })}
            </div>

            // Comparison results (only shown when File B is loaded)
            {move || {
                if let Some(cmp) = comparison.get() {
                    let loc = locale.get();
                    let score = cmp.similarity_score;
                    let score_class = if score > 0.9 { "similarity-high" }
                        else if score > 0.7 { "similarity-medium" }
                        else { "similarity-low" };

                    let metrics = cmp.metrics.clone();
                    let svgs = diagram_svgs.get().unwrap_or_default();
                    let mode = diagram_mode.get();
                    let sbs = mode.is_side_by_side();
                    let la_label = label_a.get();
                    let lb_label = label_b.get().unwrap_or_else(|| loc.ui.compare.file_b.clone());

                    view! {
                        <div class="compare-results">
                            // Similarity badge
                            <div class={format!("similarity-badge {}", score_class)}>
                                <span class="similarity-label">{loc.ui.compare.similarity.clone()}</span>
                                <span class="similarity-value">{format!("{:.1}%", score * 100.0)}</span>
                            </div>

                            // Export buttons
                            <div class="compare-export-buttons">
                                <button
                                    class="compare-export-btn"
                                    on:click=on_export_compare_pdf
                                    disabled=move || pdf_exporting.get()
                                >
                                    {move || if pdf_exporting.get() { locale.get().ui.compare.exporting.clone() } else { locale.get().ui.compare.export_pdf.clone() }}
                                </button>
                                <button
                                    class="compare-export-btn secondary"
                                    on:click=on_export_compare_typ
                                >
                                    {loc.ui.compare.export_typ.clone()}
                                </button>
                            </div>

                            // Diagram toggle
                            <div class="compare-diagram-toggle">
                                {CompareDiagramMode::ALL.iter().map(|&m| {
                                    let label = m.label(&loc);
                                    view! {
                                        <button
                                            class=move || if diagram_mode.get() == m { "active" } else { "" }
                                            on:click=move |_| set_diagram_mode.set(m)
                                        >{label}</button>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>

                            // C-plane sliders (shown in Polar/Cartesian modes)
                            <div
                                class="compare-controls isolux-controls"
                                style=move || {
                                    let mode = diagram_mode.get();
                                    if mode == CompareDiagramMode::Polar || mode == CompareDiagramMode::Cartesian {
                                        "display:flex"
                                    } else {
                                        "display:none"
                                    }
                                }
                            >
                                <div class="control-group">
                                    <span>{move || locale.get().ui.compare.file_a_c_plane.clone()}</span>
                                    <input
                                        type="range"
                                        min="0"
                                        prop:max=move || {
                                            let ha = half_angles_a.get();
                                            if ha.is_empty() { "0".to_string() } else { (ha.len() - 1).to_string() }
                                        }
                                        step="1"
                                        prop:value=move || {
                                            let ha = half_angles_a.get();
                                            let cp = polar_c_plane_a.get();
                                            ha.iter().position(|&x| (x - cp).abs() < 0.01)
                                                .unwrap_or(0).to_string()
                                        }
                                        on:input=move |ev| {
                                            if let Ok(idx) = event_target_value(&ev).parse::<usize>() {
                                                let ha = half_angles_a.get();
                                                if let Some(&angle) = ha.get(idx) {
                                                    set_polar_c_plane_a.set(angle);
                                                    if link_sliders.get() {
                                                        let hb = half_angles_b.get();
                                                        if let Some(&nearest) = hb.iter()
                                                            .min_by(|&&x, &&y| (x - angle).abs().partial_cmp(&(y - angle).abs()).unwrap())
                                                        {
                                                            set_polar_c_plane_b.set(nearest);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    />
                                    <span class="control-value">{move || format!("C {:.0}°", polar_c_plane_a.get())}</span>
                                </div>
                                <div class="control-group">
                                    <span>{move || locale.get().ui.compare.file_b_c_plane.clone()}</span>
                                    <input
                                        type="range"
                                        min="0"
                                        prop:max=move || {
                                            let hb = half_angles_b.get();
                                            if hb.is_empty() { "0".to_string() } else { (hb.len() - 1).to_string() }
                                        }
                                        step="1"
                                        prop:value=move || {
                                            let hb = half_angles_b.get();
                                            let cp = polar_c_plane_b.get();
                                            hb.iter().position(|&x| (x - cp).abs() < 0.01)
                                                .unwrap_or(0).to_string()
                                        }
                                        on:input=move |ev| {
                                            if let Ok(idx) = event_target_value(&ev).parse::<usize>() {
                                                let hb = half_angles_b.get();
                                                if let Some(&angle) = hb.get(idx) {
                                                    set_polar_c_plane_b.set(angle);
                                                    if link_sliders.get() {
                                                        let ha = half_angles_a.get();
                                                        if let Some(&nearest) = ha.iter()
                                                            .min_by(|&&x, &&y| (x - angle).abs().partial_cmp(&(y - angle).abs()).unwrap())
                                                        {
                                                            set_polar_c_plane_a.set(nearest);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    />
                                    <span class="control-value">{move || format!("C {:.0}°", polar_c_plane_b.get())}</span>
                                </div>
                                <div class="control-group link-checkbox">
                                    <label>
                                        <input
                                            type="checkbox"
                                            prop:checked=move || link_sliders.get()
                                            on:input=move |ev| {
                                                let checked = ev.target().unwrap().unchecked_into::<HtmlInputElement>().checked();
                                                set_link_sliders.set(checked);
                                            }
                                        />
                                        {move || format!(" {}", locale.get().ui.compare.link_sliders)}
                                    </label>
                                </div>
                            </div>

                            // Cone controls
                            <div
                                class="compare-controls isolux-controls"
                                style=move || if diagram_mode.get() == CompareDiagramMode::Cone { "display:flex" } else { "display:none" }
                            >
                                <div class="control-group">
                                    <span>{move || locale.get().diagram.cone.mounting_height.clone()}</span>
                                    <input
                                        type="range"
                                        prop:min=move || match unit_system.get() { UnitSystem::Metric => "1", _ => "3" }
                                        prop:max=move || match unit_system.get() { UnitSystem::Metric => "15", _ => "50" }
                                        prop:step=move || match unit_system.get() { UnitSystem::Metric => "0.5", _ => "1" }
                                        prop:value=move || unit_system.get().convert_meters(cmp_cone_height.get()).to_string()
                                        on:input=move |ev| {
                                            if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                                set_cmp_cone_height.set(unit_system.get().to_meters(v));
                                            }
                                        }
                                    />
                                    <span class="control-value">{move || unit_system.get().format_distance(cmp_cone_height.get())}</span>
                                </div>
                                <div class="control-group">
                                    <span>{move || locale.get().diagram.cone.c_plane.clone()}</span>
                                    <input
                                        type="range"
                                        min="0"
                                        prop:max=move || {
                                            let c_angles = eulumdat::SymmetryHandler::expand_c_angles(&ldt.get());
                                            if c_angles.is_empty() { "0".to_string() } else { (c_angles.len() - 1).to_string() }
                                        }
                                        step="1"
                                        prop:value=move || {
                                            let c_angles = eulumdat::SymmetryHandler::expand_c_angles(&ldt.get());
                                            match cmp_c_plane.get() {
                                                Some(cp) => c_angles.iter().position(|&x| (x - cp).abs() < 0.01)
                                                    .unwrap_or(0).to_string(),
                                                None => "0".to_string(),
                                            }
                                        }
                                        on:input=move |ev| {
                                            if let Ok(idx) = event_target_value(&ev).parse::<usize>() {
                                                let c_angles = eulumdat::SymmetryHandler::expand_c_angles(&ldt.get());
                                                if let Some(&angle) = c_angles.get(idx) {
                                                    set_cmp_c_plane.set(Some(angle));
                                                }
                                            }
                                        }
                                    />
                                    <span class="control-value">
                                        {move || match cmp_c_plane.get() {
                                            Some(cp) => format!("C {cp:.0}°"),
                                            None => locale.get().diagram.cone.all_planes.clone(),
                                        }}
                                    </span>
                                </div>
                            </div>

                            // Isolux controls
                            <div
                                class="compare-controls isolux-controls"
                                style=move || if diagram_mode.get() == CompareDiagramMode::Isolux { "display:flex" } else { "display:none" }
                            >
                                <div class="control-group">
                                    <span>{move || locale.get().ui.floodlight.mounting_height.clone()}</span>
                                    <input
                                        type="range"
                                        prop:min=move || match unit_system.get() { UnitSystem::Metric => "3", _ => "10" }
                                        prop:max=move || match unit_system.get() { UnitSystem::Metric => "30", _ => "100" }
                                        prop:step=move || match unit_system.get() { UnitSystem::Metric => "0.5", _ => "1" }
                                        prop:value=move || unit_system.get().convert_meters(cmp_isolux_height.get()).to_string()
                                        on:input=move |ev| {
                                            if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                                set_cmp_isolux_height.set(unit_system.get().to_meters(v));
                                            }
                                        }
                                    />
                                    <span class="control-value">{move || unit_system.get().format_distance(cmp_isolux_height.get())}</span>
                                </div>
                                <div class="control-group">
                                    <span>{move || locale.get().ui.floodlight.tilt_angle.clone()}</span>
                                    <input
                                        type="range"
                                        min="0" max="80" step="1"
                                        prop:value=move || cmp_tilt_angle.get().to_string()
                                        on:input=move |ev| {
                                            if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                                set_cmp_tilt_angle.set(v);
                                            }
                                        }
                                    />
                                    <span class="control-value">{move || format!("{:.0}°", cmp_tilt_angle.get())}</span>
                                </div>
                                <div class="control-group">
                                    <span>{move || locale.get().ui.floodlight.area_size.clone()}</span>
                                    <input
                                        type="range"
                                        prop:min=move || match unit_system.get() { UnitSystem::Metric => "10", _ => "30" }
                                        prop:max=move || match unit_system.get() { UnitSystem::Metric => "100", _ => "300" }
                                        prop:step=move || match unit_system.get() { UnitSystem::Metric => "5", _ => "10" }
                                        prop:value=move || unit_system.get().convert_meters(cmp_area_size.get()).to_string()
                                        on:input=move |ev| {
                                            if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                                set_cmp_area_size.set(unit_system.get().to_meters(v));
                                            }
                                        }
                                    />
                                    <span class="control-value">{move || unit_system.get().format_distance(cmp_area_size.get())}</span>
                                </div>
                            </div>

                            // Diagram area
                            {if sbs {
                                // Side-by-side: two separate SVGs with labels
                                let svg_a = svgs.first().cloned().unwrap_or_default();
                                let svg_b = svgs.get(1).cloned().unwrap_or_default();
                                view! {
                                    <div class="compare-side-by-side">
                                        <div class="compare-side">
                                            <div class="compare-side-label">{la_label}</div>
                                            <div class="compare-side-svg" inner_html=svg_a></div>
                                        </div>
                                        <div class="compare-side">
                                            <div class="compare-side-label">{lb_label}</div>
                                            <div class="compare-side-svg" inner_html=svg_b></div>
                                        </div>
                                    </div>
                                }.into_any()
                            } else {
                                // Single overlay diagram
                                let svg_html = svgs.first().cloned().unwrap_or_default();
                                view! {
                                    <div class="compare-overlay-diagram" inner_html=svg_html></div>
                                }.into_any()
                            }}

                            // Metrics table
                            <div class="compare-table-wrapper">
                                <table class="compare-table">
                                    <thead>
                                        <tr>
                                            <th>{loc.ui.compare.metric.clone()}</th>
                                            <th>{loc.ui.compare.file_a.clone()}</th>
                                            <th>{loc.ui.compare.file_b.clone()}</th>
                                            <th>{loc.ui.compare.delta.clone()}</th>
                                            <th>{loc.ui.compare.percent.clone()}</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {metrics.iter().map(|m| {
                                            let row_class = match m.significance {
                                                Significance::Negligible => "delta-ok",
                                                Significance::Minor => "delta-minor",
                                                Significance::Moderate => "delta-moderate",
                                                Significance::Major => "delta-major",
                                            };
                                            let unit = if m.unit.is_empty() { String::new() } else { format!(" {}", m.unit) };
                                            let name = m.name.clone();
                                            let val_a = format!("{:.1}{}", m.value_a, unit);
                                            let val_b = format!("{:.1}{}", m.value_b, unit);
                                            let delta = format!("{:+.1}", m.delta);
                                            let pct = format!("{:+.1}%", m.delta_percent);
                                            view! {
                                                <tr class={row_class}>
                                                    <td class="metric-name">{name}</td>
                                                    <td class="metric-value">{val_a}</td>
                                                    <td class="metric-value">{val_b}</td>
                                                    <td class="metric-delta">{delta}</td>
                                                    <td class="metric-percent">{pct}</td>
                                                </tr>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </tbody>
                                </table>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    let loc = locale.get();
                    view! {
                        <div class="compare-empty">
                            <p>{loc.ui.compare.empty_title.clone()}</p>
                            <p class="text-muted">{loc.ui.compare.empty_hint.clone()}</p>
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}
