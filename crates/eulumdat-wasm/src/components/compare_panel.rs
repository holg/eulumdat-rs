use eulumdat::diagram::{
    ButterflyDiagram, CartesianDiagram, ConeDiagram, FloodlightCartesianDiagram, HeatmapDiagram,
    IsocandelaDiagram, IsoluxDiagram, IsoluxParams, PolarDiagram, SvgTheme, YScale,
};
use eulumdat::{BugDiagram, Eulumdat, IesParser, PhotometricComparison, Significance};
use leptos::ev;
use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

use super::templates::{TemplateFormat, ALL_TEMPLATES};

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

    fn label(self) -> &'static str {
        match self {
            Self::Polar => "Polar",
            Self::Cartesian => "Cartesian",
            Self::Heatmap => "Heatmap",
            Self::Butterfly => "Butterfly",
            Self::Cone => "Cone",
            Self::Isocandela => "Isocandela",
            Self::Isolux => "Isolux",
            Self::Floodlight => "Floodlight",
            Self::Bug => "BUG",
            Self::Lcs => "LCS",
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
fn render_diagrams(
    a: &Eulumdat,
    b: &Eulumdat,
    mode: CompareDiagramMode,
    theme: &SvgTheme,
    la: &str,
    lb: &str,
) -> Vec<String> {
    match mode {
        CompareDiagramMode::Polar => {
            let pa = PolarDiagram::from_eulumdat(a);
            let pb = PolarDiagram::from_eulumdat(b);
            vec![PolarDiagram::to_overlay_svg(
                &pa, &pb, 500.0, 500.0, theme, la, lb,
            )]
        }
        CompareDiagramMode::Cartesian => {
            let ca = CartesianDiagram::from_eulumdat(a, 600.0, 400.0, 4);
            let cb = CartesianDiagram::from_eulumdat(b, 600.0, 400.0, 4);
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
            let ca = ConeDiagram::from_eulumdat(a, 3.0);
            let cb = ConeDiagram::from_eulumdat(b, 3.0);
            vec![
                ca.to_svg(400.0, 350.0, theme),
                cb.to_svg(400.0, 350.0, theme),
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
            let params = IsoluxParams::default();
            let ia = IsoluxDiagram::from_eulumdat(a, 400.0, 350.0, params);
            let ib = IsoluxDiagram::from_eulumdat(b, 400.0, 350.0, params);
            vec![
                ia.to_svg(400.0, 350.0, theme),
                ib.to_svg(400.0, 350.0, theme),
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
    let (diagram_mode, set_diagram_mode) = signal(CompareDiagramMode::default());
    let (drag_active, set_drag_active) = signal(false);

    // File A label derived from current_file
    let label_a = Memo::new(move |_| current_file.get().unwrap_or_else(|| "File A".to_string()));

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

    // Handle template selection for File B
    let on_template_select = move |ev: ev::Event| {
        let select: web_sys::HtmlSelectElement = ev.target().unwrap().unchecked_into();
        let idx: usize = select.value().parse().unwrap_or(usize::MAX);
        if idx < ALL_TEMPLATES.len() {
            let tmpl = ALL_TEMPLATES[idx];
            let parsed = match tmpl.format {
                TemplateFormat::Ldt => Eulumdat::parse(tmpl.content).ok(),
                TemplateFormat::AtlaXml => atla::xml::parse(tmpl.content)
                    .ok()
                    .map(|doc| doc.to_eulumdat()),
                TemplateFormat::AtlaJson => atla::json::parse(tmpl.content)
                    .ok()
                    .map(|doc| doc.to_eulumdat()),
            };
            if let Some(ldt) = parsed {
                set_ldt_b.set(Some(ldt));
                set_label_b.set(Some(tmpl.name.to_string()));
            }
        }
    };

    // Compute comparison as a derived memo
    let comparison = Memo::new(move |_| {
        let a = ldt.get();
        ldt_b.get().map(|b| {
            let la = label_a.get();
            let lb = label_b.get().unwrap_or_else(|| "File B".to_string());
            PhotometricComparison::from_eulumdat(&a, &b, &la, &lb)
        })
    });

    // Generate diagram SVG(s)
    let diagram_svgs = Memo::new(move |_| {
        let a = ldt.get();
        ldt_b.get().map(|b| {
            let la = label_a.get();
            let lb = label_b.get().unwrap_or_else(|| "File B".to_string());
            let theme = SvgTheme::css_variables();
            render_diagrams(&a, &b, diagram_mode.get(), &theme, &la, &lb)
        })
    });

    // Export: Typst source download
    let on_export_compare_typ = move |_: ev::MouseEvent| {
        if let Some(b_ldt) = ldt_b.get() {
            let a_ldt = ldt.get();
            let la = label_a.get();
            let lb = label_b.get().unwrap_or_else(|| "File B".to_string());
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
            let lb = label_b.get().unwrap_or_else(|| "File B".to_string());
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
                <h4>"Compare with:"</h4>

                // Drag-and-drop zone for File B
                <div
                    class=move || format!("compare-drop-zone{}", if drag_active.get() { " active" } else { "" })
                    on:dragover=on_dragover
                    on:dragleave=on_dragleave
                    on:drop=on_drop
                >
                    <p>"Drop a .ldt, .ies, or .xml file here"</p>
                    <div class="compare-inputs">
                        <label class="compare-file-btn">
                            "Browse file"
                            <input
                                type="file"
                                accept=".ldt,.ies,.xml,.json"
                                style="display:none"
                                on:change=on_file_b_input
                            />
                        </label>
                        <span class="compare-or">"or"</span>
                        <select class="compare-template-select" on:change=on_template_select>
                            <option value="" disabled selected>"Select template..."</option>
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
                        <strong>"File B: "</strong>{name.clone()}
                        <button class="compare-clear-btn" on:click=on_clear_b title="Clear File B">"✕"</button>
                    </div>
                })}
            </div>

            // Comparison results (only shown when File B is loaded)
            {move || {
                if let Some(cmp) = comparison.get() {
                    let score = cmp.similarity_score;
                    let score_class = if score > 0.9 { "similarity-high" }
                        else if score > 0.7 { "similarity-medium" }
                        else { "similarity-low" };

                    let metrics = cmp.metrics.clone();
                    let svgs = diagram_svgs.get().unwrap_or_default();
                    let mode = diagram_mode.get();
                    let sbs = mode.is_side_by_side();
                    let la_label = label_a.get();
                    let lb_label = label_b.get().unwrap_or_else(|| "File B".to_string());

                    view! {
                        <div class="compare-results">
                            // Similarity badge
                            <div class={format!("similarity-badge {}", score_class)}>
                                <span class="similarity-label">"Similarity"</span>
                                <span class="similarity-value">{format!("{:.1}%", score * 100.0)}</span>
                            </div>

                            // Export buttons
                            <div class="compare-export-buttons">
                                <button
                                    class="compare-export-btn"
                                    on:click=on_export_compare_pdf
                                    disabled=move || pdf_exporting.get()
                                >
                                    {move || if pdf_exporting.get() { "Exporting..." } else { "Export Comparison (.pdf)" }}
                                </button>
                                <button
                                    class="compare-export-btn secondary"
                                    on:click=on_export_compare_typ
                                >
                                    "Export Comparison (.typ)"
                                </button>
                            </div>

                            // Diagram toggle
                            <div class="compare-diagram-toggle">
                                {CompareDiagramMode::ALL.iter().map(|&m| {
                                    let label = m.label();
                                    view! {
                                        <button
                                            class=move || if diagram_mode.get() == m { "active" } else { "" }
                                            on:click=move |_| set_diagram_mode.set(m)
                                        >{label}</button>
                                    }
                                }).collect::<Vec<_>>()}
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
                                            <th>"Metric"</th>
                                            <th>"File A"</th>
                                            <th>"File B"</th>
                                            <th>"Delta"</th>
                                            <th>"%"</th>
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
                    view! {
                        <div class="compare-empty">
                            <p>"Load a second photometric file to compare side-by-side."</p>
                            <p class="text-muted">"Compares flux, efficacy, beam angles, intensity distribution, and more."</p>
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}
