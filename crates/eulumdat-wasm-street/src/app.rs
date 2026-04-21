//! Street designer Leptos app.
//!
//! v0: single luminaire, a small form for street geometry, and a compliance
//! table that runs all four standards simultaneously. No heatmap yet.

use leptos::prelude::*;

use eulumdat::standards::{
    cjj45::{Cjj45Class, Cjj45Standard},
    en13201::{En13201Class, En13201Standard},
    mlo::MloStandard,
    rp8::{PedestrianConflict, RoadClass, Rp8Selection, Rp8Standard},
    ComplianceResult, LightingStandard,
};
use eulumdat::street::{Arrangement, StreetLayout};
use eulumdat::{bug_rating::LightingZone, Eulumdat};

#[component]
pub fn StreetApp() -> impl IntoView {
    let (ldt, set_ldt) = signal(None::<Eulumdat>);
    let (layout, set_layout) = signal(StreetLayout::default());

    // Selections for each region's standard.
    let (mlo_zone, _) = signal(LightingZone::LZ2);
    let (rp8_sel, _) = signal(Rp8Selection {
        road_class: RoadClass::Collector,
        pedestrian_conflict: PedestrianConflict::Medium,
    });
    let (en_class, _) = signal(En13201Class::C3);
    let (cjj_class, _) = signal(Cjj45Class::ClassII);

    // Recompute on every relevant change.
    let design_and_results = Memo::new(move |_| {
        let ldt_val = ldt.get()?;
        let layout_val = layout.get();
        let area = layout_val.compute(&ldt_val, 0.8);
        let design = layout_val.design_result(&area);

        let mut results: Vec<ComplianceResult> = Vec::new();
        if let Some(r) = MloStandard.check_file(&mlo_zone.get(), &ldt_val) {
            results.push(r);
        }
        if let Some(r) = Rp8Standard.check_design(&rp8_sel.get(), &design) {
            results.push(r);
        }
        if let Some(r) = En13201Standard.check_design(&en_class.get(), &design) {
            results.push(r);
        }
        if let Some(r) = Cjj45Standard.check_design(&cjj_class.get(), &design) {
            results.push(r);
        }
        Some((design, results, area.avg_lux, area.min_lux, area.max_lux))
    });

    view! {
        <div class="street-app">
            <FilePicker set_ldt=set_ldt />
            <LayoutForm layout=layout set_layout=set_layout />

            {move || match design_and_results.get() {
                None => view! {
                    <p style="color: #888; font-style: italic;">"Load a luminaire file to see compliance results."</p>
                }.into_any(),
                Some((_design, results, avg, min, max)) => view! {
                    <h2>"Computed illuminance"</h2>
                    <p>
                        "Average: " <strong>{format!("{avg:.1} lux")}</strong>
                        " · Min: " <strong>{format!("{min:.1} lux")}</strong>
                        " · Max: " <strong>{format!("{max:.1} lux")}</strong>
                    </p>
                    <h2>"Compliance"</h2>
                    <ResultsTable results=results />
                }.into_any(),
            }}
        </div>
    }
}

#[component]
fn FilePicker(set_ldt: WriteSignal<Option<Eulumdat>>) -> impl IntoView {
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
            // Try LDT first, then IES.
            let parsed = Eulumdat::parse(&text).or_else(|_| eulumdat::IesParser::parse(&text));
            match parsed {
                Ok(ldt) => set_ldt.set(Some(ldt)),
                Err(e) => web_sys::console::warn_1(&format!("parse failed: {e}").into()),
            }
        }) as Box<dyn FnMut(_)>);
        reader.set_onload(Some(onload.as_ref().unchecked_ref()));
        onload.forget();
        reader.read_as_text(&file).unwrap();
    };

    view! {
        <div style="margin: 1rem 0;">
            <label>
                "Luminaire file (.ldt or .ies): "
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
