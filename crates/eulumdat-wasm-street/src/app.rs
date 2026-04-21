//! Street designer Leptos app.
//!
//! v0: single luminaire, a small form for street geometry, and a compliance
//! table that runs all four standards simultaneously. No heatmap yet.
//!
//! The luminaire comes from the main editor via localStorage (see
//! [`crate::storage_sync`]); the fallback file picker is only used when the
//! editor hasn't loaded anything yet.

use leptos::prelude::*;

use crate::storage_sync::{wire_storage_sync, LdtSource};
use eulumdat::area::AreaResult;
use eulumdat::standards::{
    cjj45::{Cjj45Class, Cjj45Standard},
    en13201::{En13201Class, En13201Standard},
    mlo::MloStandard,
    rp8::{PedestrianConflict, RoadClass, Rp8Selection, Rp8Standard},
    ComplianceResult, LightingStandard,
};
use eulumdat::street::{
    plan_view_heatmap, Arrangement, FailureOverlay, PlanViewOptions, StreetLayout, StreetTheme,
};
use eulumdat::{bug_rating::LightingZone, Eulumdat};

#[component]
pub fn StreetApp() -> impl IntoView {
    let (ldt, set_ldt) = signal(None::<Eulumdat>);
    let (source, set_source) = signal(LdtSource::None);
    let (layout, set_layout) = signal(StreetLayout::default());

    // Hook into the editor's localStorage writes so the same luminaire the
    // user is editing appears here automatically, with live updates.
    wire_storage_sync(set_ldt, set_source);

    // Selections for each region's standard.
    let (mlo_zone, _) = signal(LightingZone::LZ2);
    let (rp8_sel, _) = signal(Rp8Selection {
        road_class: RoadClass::Collector,
        pedestrian_conflict: PedestrianConflict::Medium,
    });
    let (en_class, _) = signal(En13201Class::C3);
    let (cjj_class, _) = signal(Cjj45Class::ClassII);

    // Viz toggle — off by default so the first impression is clean.
    let (highlight_failures, set_highlight_failures) = signal(false);

    // Compute the illuminance grid (drives both the heatmap and the
    // compliance results). Kept as a separate memo so changing only a
    // standard selection doesn't force a full layout recompute.
    let area_result = Memo::new(move |_| -> Option<AreaResult> {
        let ldt_val = ldt.get()?;
        let layout_val = layout.get();
        Some(layout_val.compute(&ldt_val, 0.8))
    });

    let compliance = Memo::new(move |_| -> Vec<ComplianceResult> {
        let Some(ldt_val) = ldt.get() else {
            return Vec::new();
        };
        let Some(area) = area_result.get() else {
            return Vec::new();
        };
        let layout_val = layout.get();
        let design = layout_val.design_result(&area);

        let mut out: Vec<ComplianceResult> = Vec::new();
        if let Some(r) = MloStandard.check_file(&mlo_zone.get(), &ldt_val) {
            out.push(r);
        }
        if let Some(r) = Rp8Standard.check_design(&rp8_sel.get(), &design) {
            out.push(r);
        }
        if let Some(r) = En13201Standard.check_design(&en_class.get(), &design) {
            out.push(r);
        }
        if let Some(r) = Cjj45Standard.check_design(&cjj_class.get(), &design) {
            out.push(r);
        }
        out
    });

    view! {
        <div class="street-app">
            <LdtSourcePanel ldt=ldt source=source set_ldt=set_ldt set_source=set_source />
            <LayoutForm layout=layout set_layout=set_layout />

            {move || area_result.get().map(|area| {
                let layout_val = layout.get();
                let svg = render_plan_svg(&layout_val, &area, highlight_failures.get());
                let avg = area.avg_lux;
                let min = area.min_lux;
                let max = area.max_lux;
                view! {
                    <section class="street-viz">
                        <div class="street-viz-toolbar">
                            <label class="street-viz-toggle">
                                <input
                                    type="checkbox"
                                    prop:checked=move || highlight_failures.get()
                                    on:change=move |ev| {
                                        use wasm_bindgen::JsCast;
                                        let checked = ev.target()
                                            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                                            .map(|i| i.checked())
                                            .unwrap_or(false);
                                        set_highlight_failures.set(checked);
                                    }
                                />
                                " Highlight failures (cells below avg × uniformity target)"
                            </label>
                            <div class="street-viz-stats">
                                "avg " <strong>{format!("{avg:.1}")}</strong> " · "
                                "min " <strong>{format!("{min:.1}")}</strong> " · "
                                "max " <strong>{format!("{max:.1}")}</strong> " lux"
                            </div>
                        </div>
                        <div class="street-viz-canvas" inner_html=svg></div>
                    </section>
                }.into_any()
            }).unwrap_or_else(|| view! {
                <p class="street-placeholder">
                    "Load a luminaire in the main editor (or upload one here) to see the plan view and compliance."
                </p>
            }.into_any())}

            {move || {
                let results = compliance.get();
                (!results.is_empty()).then(|| view! {
                    <h2>"Compliance"</h2>
                    <ResultsTable results=results />
                })
            }}
        </div>
    }
}

/// Render the plan-view SVG with the current failure-overlay toggle.
///
/// Threshold picks a representative roadway uniformity floor (0.4 ≈
/// EN 13201 U₀, and a reasonable proxy for RP-8's "avg/min ≤ 3" on
/// collector/local roads). Per-standard thresholds would be nicer but
/// require piping the active selection through here.
fn render_plan_svg(layout: &StreetLayout, area: &AreaResult, highlight: bool) -> String {
    let opts = PlanViewOptions {
        theme: StreetTheme::Dark,
        failure_overlay: highlight.then_some(FailureOverlay { ratio_floor: 0.4 }),
    };
    plan_view_heatmap(layout, area, 800.0, 280.0, opts)
}

/// Shows which luminaire is currently loaded (badge + name) plus a
/// collapsible fallback upload. When `ldt` is `None` the upload field is
/// front-and-centre; when an LDT is present it's collapsed under a
/// "Use a different file" disclosure.
#[component]
fn LdtSourcePanel(
    ldt: ReadSignal<Option<Eulumdat>>,
    source: ReadSignal<LdtSource>,
    set_ldt: WriteSignal<Option<Eulumdat>>,
    set_source: WriteSignal<LdtSource>,
) -> impl IntoView {
    view! {
        <div class="street-source-panel">
            {move || match (source.get(), ldt.get()) {
                (LdtSource::Editor, Some(l)) => view! {
                    <div class="street-source-badge street-source-badge--editor">
                        <span class="dot"></span>
                        "Using luminaire from editor: "
                        <strong>
                            {if l.luminaire_name.is_empty() { "(unnamed)".to_string() } else { l.luminaire_name.clone() }}
                        </strong>
                        <span class="text-muted"> " — live updates as you edit"</span>
                    </div>
                }.into_any(),
                (LdtSource::Upload, Some(l)) => view! {
                    <div class="street-source-badge street-source-badge--upload">
                        <span class="dot"></span>
                        "Uploaded: "
                        <strong>
                            {if l.luminaire_name.is_empty() { "(unnamed)".to_string() } else { l.luminaire_name.clone() }}
                        </strong>
                    </div>
                }.into_any(),
                _ => view! {
                    <div class="street-source-hint text-muted">
                        "No luminaire loaded yet. Open one in the main editor, "
                        "or upload a .ldt or .ies file below."
                    </div>
                }.into_any(),
            }}

            <details class="street-source-upload" open=move || ldt.get().is_none()>
                <summary>
                    {move || if ldt.get().is_some() { "Use a different file…" } else { "Upload a luminaire file" }}
                </summary>
                <FallbackFilePicker set_ldt=set_ldt set_source=set_source />
            </details>
        </div>
    }
}

#[component]
fn FallbackFilePicker(
    set_ldt: WriteSignal<Option<Eulumdat>>,
    set_source: WriteSignal<LdtSource>,
) -> impl IntoView {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;
    use web_sys::{FileReader, HtmlInputElement};

    let on_change = move |ev: leptos::ev::Event| {
        let target = ev
            .target()
            .and_then(|t| t.dyn_into::<HtmlInputElement>().ok());
        let Some(input) = target else { return };
        let Some(files) = input.files() else { return };
        let Some(file) = files.get(0) else { return };

        let reader = FileReader::new().unwrap();
        let reader_clone = reader.clone();

        let onload = Closure::wrap(Box::new(move |_: web_sys::ProgressEvent| {
            let result = reader_clone.result().unwrap();
            let text = result.as_string().unwrap_or_default();
            let parsed = Eulumdat::parse(&text).or_else(|_| eulumdat::IesParser::parse(&text));
            match parsed {
                Ok(ldt) => {
                    set_ldt.set(Some(ldt));
                    set_source.set(LdtSource::Upload);
                }
                Err(e) => web_sys::console::warn_1(&format!("parse failed: {e}").into()),
            }
        }) as Box<dyn FnMut(_)>);
        reader.set_onload(Some(onload.as_ref().unchecked_ref()));
        onload.forget();
        reader.read_as_text(&file).unwrap();
    };

    view! {
        <div class="street-upload-row">
            <label>
                "File: "
                <input type="file" accept=".ldt,.ies" on:change=on_change />
            </label>
        </div>
    }
}

#[component]
fn LayoutForm(
    layout: ReadSignal<StreetLayout>,
    set_layout: WriteSignal<StreetLayout>,
) -> impl IntoView {
    let arrangements = [
        (Arrangement::SingleSide, "Single-side"),
        (Arrangement::Opposite, "Opposite"),
        (Arrangement::Staggered, "Staggered"),
    ];

    view! {
        <fieldset style="margin: 1rem 0; padding: 1rem; border: 1px solid #ddd;">
            <legend>"Street geometry"</legend>
            <div style="display: grid; grid-template-columns: repeat(2, 1fr); gap: 0.5rem 1rem;">
                <NumberField label="Length (m)" value=Signal::derive(move || layout.get().length_m)
                    on_change=move |v| set_layout.update(|l| l.length_m = v) />
                <NumberField label="Lane width (m)" value=Signal::derive(move || layout.get().lane_width_m)
                    on_change=move |v| set_layout.update(|l| l.lane_width_m = v) />
                <NumberField label="# Lanes" value=Signal::derive(move || layout.get().num_lanes as f64)
                    on_change=move |v| set_layout.update(|l| l.num_lanes = v.max(1.0).round() as usize) />
                <NumberField label="Pole spacing (m)" value=Signal::derive(move || layout.get().pole_spacing_m)
                    on_change=move |v| set_layout.update(|l| l.pole_spacing_m = v) />
                <NumberField label="Mounting height (m)" value=Signal::derive(move || layout.get().mounting_height_m)
                    on_change=move |v| set_layout.update(|l| l.mounting_height_m = v) />
                <NumberField label="Overhang (m)" value=Signal::derive(move || layout.get().overhang_m)
                    on_change=move |v| set_layout.update(|l| l.overhang_m = v) />
                <NumberField label="Tilt (°)" value=Signal::derive(move || layout.get().tilt_deg)
                    on_change=move |v| set_layout.update(|l| l.tilt_deg = v) />
                <NumberField label="Pole offset (m)" value=Signal::derive(move || layout.get().pole_offset_m)
                    on_change=move |v| set_layout.update(|l| l.pole_offset_m = v) />
                <NumberField label="Sidewalk (m)" value=Signal::derive(move || layout.get().sidewalk_width_m)
                    on_change=move |v| set_layout.update(|l| l.sidewalk_width_m = v.max(0.0)) />
                <label style="grid-column: 1 / -1;">
                    "Arrangement: "
                    <select
                        prop:value=move || format!("{:?}", layout.get().arrangement)
                        on:change=move |ev| {
                            use wasm_bindgen::JsCast;
                            let v = ev.target()
                                .and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok())
                                .map(|s| s.value())
                                .unwrap_or_default();
                            let arr = match v.as_str() {
                                "SingleSide" => Arrangement::SingleSide,
                                "Opposite" => Arrangement::Opposite,
                                _ => Arrangement::Staggered,
                            };
                            set_layout.update(|l| l.arrangement = arr);
                        }
                    >
                        {arrangements.iter().map(|(a, label)| view! {
                            <option value={format!("{:?}", a)}>{label.to_string()}</option>
                        }).collect_view()}
                    </select>
                </label>
            </div>
        </fieldset>
    }
}

#[component]
fn NumberField(
    label: &'static str,
    value: Signal<f64>,
    on_change: impl Fn(f64) + 'static + Copy,
) -> impl IntoView {
    view! {
        <label>
            {label} ": "
            <input
                type="number"
                step="0.1"
                prop:value=move || format!("{:.2}", value.get())
                on:input=move |ev| {
                    use wasm_bindgen::JsCast;
                    let v = ev.target()
                        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                        .and_then(|i| i.value().parse::<f64>().ok());
                    if let Some(v) = v {
                        on_change(v);
                    }
                }
            />
        </label>
    }
}

#[component]
fn ResultsTable(results: Vec<ComplianceResult>) -> impl IntoView {
    view! {
        <div>
            {results.into_iter().map(|r| {
                let passed = r.passed();
                let header_color = if passed { "#2d7a2d" } else { "#a83232" };
                let badge = if passed { "✓ PASS" } else { "✗ FAIL" };
                view! {
                    <div style="margin: 1rem 0; border: 1px solid #ddd; padding: 0.75rem;">
                        <h3 style=format!("color: {header_color}; margin: 0 0 0.5rem;")>
                            {format!("{} · {} · {}", r.region, r.standard, badge)}
                        </h3>
                        <table style="width: 100%; border-collapse: collapse;">
                            <thead>
                                <tr style="background: #f5f5f5;">
                                    <th style="text-align: left; padding: 0.25rem 0.5rem;">"Criterion"</th>
                                    <th style="text-align: left; padding: 0.25rem 0.5rem;">"Required"</th>
                                    <th style="text-align: left; padding: 0.25rem 0.5rem;">"Achieved"</th>
                                    <th style="padding: 0.25rem 0.5rem;">""</th>
                                </tr>
                            </thead>
                            <tbody>
                                {r.items.into_iter().map(|item| {
                                    let status = if item.passed { "✓" } else { "✗" };
                                    let color = if item.passed { "#2d7a2d" } else { "#a83232" };
                                    view! {
                                        <tr>
                                            <td style="padding: 0.25rem 0.5rem;">{item.parameter}</td>
                                            <td style="padding: 0.25rem 0.5rem;">{item.required}</td>
                                            <td style="padding: 0.25rem 0.5rem;">{item.achieved}</td>
                                            <td style=format!("padding: 0.25rem 0.5rem; color: {color}; text-align: center;")>{status}</td>
                                        </tr>
                                    }
                                }).collect_view()}
                            </tbody>
                        </table>
                    </div>
                }
            }).collect_view()}
        </div>
    }
}
