//! Street designer Leptos app.
//!
//! Main flow:
//! 1. The luminaire (LDC) comes from the main editor via localStorage.
//! 2. A **standard selector** at the top of the UI chooses the
//!    authoritative region — RP-8 (US) / EN 13201 (EU) / CJJ 45 (CN).
//! 3. The matching **decision tree** asks the questions the chosen
//!    standard's selection matrix actually needs, and derives a
//!    recommended class. Users can override the class directly.
//! 4. Geometry + class feeds the plan-view heatmap and compliance panel.
//! 5. An **optimizer panel** sweeps (pole_spacing × mounting_height ×
//!    arrangement) to find passing layouts minimizing a user-chosen cost.
//!
//! Language + unit system track the main editor via shared localStorage
//! keys (see [`crate::i18n`]).

use leptos::prelude::*;

use crate::download::{save_csv, save_json, save_pdf, save_svg};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

// Bridge to the typst-loader.js global. The host page loads
// `typst-loader-<hash>.js` ahead of the street WASM, which exposes
// `window.compileTypstToPdf(source) -> Promise<Uint8Array>`.
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = compileTypstToPdf, catch)]
    fn compile_typst_to_pdf(source: &str) -> Result<js_sys::Promise, JsValue>;
}
use crate::i18n::{install_locale, install_units};
use crate::storage_sync::{read_ldc_now, wire_storage_sync};
use eulumdat::area::AreaResult;
use eulumdat::diagram::{principal_planes_svg, SvgTheme};
use eulumdat::standards::{
    cjj45::{Cjj45Class, Cjj45Standard},
    en13201::{En13201Class, En13201Standard},
    en13201_selector::{
        AmbientLuminance, CParamsEn13201, DesignSpeed, FacialRecognition, JunctionDensity,
        NavigationalTask, PParamsEn13201, ParkedVehicles, PedestrianSpeed, Separation,
        TrafficComposition, TrafficVolume, UserDensity,
    },
    mlo::MloStandard,
    rp8::{PedestrianConflict, RoadClass, Rp8Selection, Rp8Standard},
    ComplianceResult, LightingStandard,
};
use eulumdat::street::{
    advisor::{advise, AdvisorTip},
    layout_tradeoff_chart, optimize_layout_all, pareto_front_tradeoff, plan_view_heatmap,
    report::{generate_street_report, StreetReportInput},
    Arrangement, FailureOverlay, LayoutTradeoffOptions, OptimizationCandidate, OptimizerBounds,
    OptimizerObjective, PlanViewOptions, StreetLayout, StreetTheme,
};
use eulumdat::{bug_rating::LightingZone, Eulumdat, UnitSystem};
use eulumdat_i18n::Locale;

// ─────────────────────────────────────────────────────────────────────────
// Top-level state enums
// ─────────────────────────────────────────────────────────────────────────

/// Which region's standard is authoritative for the current design.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AuthoritativeStandard {
    Rp8,
    En13201,
    Cjj45,
}

/// EN 13201 comes in two families per §A.2/§A.3 — the user picks which
/// one their road falls into before filling out the decision tree.
#[derive(Clone, Copy, PartialEq, Eq)]
enum EnFamily {
    /// C-family: conflict areas (junctions, complex urban intersections).
    C,
    /// P-family: pedestrian / low-speed areas.
    P,
}

/// Mapping from CJJ 45-2015 colloquial road grades to lighting classes.
///
/// Per CJJ 45-2015 Table 3.3.2, the four lighting classes correspond to:
///   Class I  → 快速路 (expressway) AND 主干路 (major arterial)
///   Class II → 次干路 (secondary / minor arterial)
///   Class III → 支路 (branch / local road)
///   Class IV → 居住区道路 (residential street)
///
/// Note that *both* expressways and major arterials share Class I.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CjjRoadGrade {
    /// 快速路 → Class I (expressway).
    Expressway,
    /// 主干路 → Class I (major arterial).
    MajorArterial,
    /// 次干路 → Class II (minor / secondary arterial).
    MinorArterial,
    /// 支路 → Class III (branch road).
    Branch,
    /// 居住区道路 → Class IV (residential street).
    Residential,
}

impl CjjRoadGrade {
    fn to_class(self) -> Cjj45Class {
        match self {
            Self::Expressway | Self::MajorArterial => Cjj45Class::ClassI,
            Self::MinorArterial => Cjj45Class::ClassII,
            Self::Branch => Cjj45Class::ClassIII,
            Self::Residential => Cjj45Class::ClassIV,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Root component
// ─────────────────────────────────────────────────────────────────────────

#[component]
pub fn StreetApp() -> impl IntoView {
    let locale = install_locale();
    let units = install_units();

    let (ldc, set_ldc) = signal(read_ldc_now());
    let (layout, set_layout) = signal(StreetLayout::default());
    wire_storage_sync(set_ldc);

    // Environmental zone for MLO checks — independent of the chosen
    // authoritative standard (MLO applies regardless of region).
    let (mlo_zone, _) = signal(LightingZone::LZ2);

    // Authoritative standard. No auto-detect — user picks.
    let (standard, set_standard) = signal(AuthoritativeStandard::Rp8);

    // RP-8 decision tree inputs.
    let (rp8_road, set_rp8_road) = signal(RoadClass::Collector);
    let (rp8_ped, set_rp8_ped) = signal(PedestrianConflict::Medium);
    let rp8_sel = Memo::new(move |_| Rp8Selection {
        road_class: rp8_road.get(),
        pedestrian_conflict: rp8_ped.get(),
    });

    // EN 13201 decision tree — family switch + per-family parameter bags.
    let (en_family, set_en_family) = signal(EnFamily::C);
    let (en_c_params, set_en_c_params) = signal(CParamsEn13201::default());
    let (en_p_params, set_en_p_params) = signal(PParamsEn13201::default());
    let en_class = Memo::new(move |_| match en_family.get() {
        EnFamily::C => en_c_params.get().recommended_class(),
        EnFamily::P => en_p_params.get().recommended_class(),
    });

    // CJJ 45 — single dropdown → class.
    let (cjj_grade, set_cjj_grade) = signal(CjjRoadGrade::MinorArterial);
    let cjj_class = Memo::new(move |_| cjj_grade.get().to_class());

    // Failure-overlay threshold source.
    let (threshold_source, set_threshold_source) = signal(ThresholdSource::Off);
    let (custom_ratio, set_custom_ratio) = signal(0.4_f64);

    // Optimizer state.
    let (optimizer_objective, set_optimizer_objective) = signal(OptimizerObjective::PoleCountPerKm);
    let (optimizer_results, set_optimizer_results) = signal(Vec::<OptimizationCandidate>::new());
    let (optimizer_running, set_optimizer_running) = signal(false);
    // Full passing set + Pareto-front indices for the layout trade-off chart.
    // Populated alongside `optimizer_results` so a single optimizer run
    // serves both the top-3 table and the trade-off plot.
    let (tradeoff_all, set_tradeoff_all) = signal(Vec::<OptimizationCandidate>::new());
    let (tradeoff_frontier, set_tradeoff_frontier) = signal(Vec::<usize>::new());
    let (tradeoff_visible, set_tradeoff_visible) = signal(false);

    // Show/hide for the principal-planes (PV / PC) intensity diagram —
    // the road-lighting industry's standard "what does this luminaire's
    // distribution look like along vs across the road?" plot.
    let (principal_planes_visible, set_principal_planes_visible) = signal(false);

    // Plan-view overlay toggles. Show calc-grid-points overlays the
    // EN 13201 / RP-8 measurement grid on the heatmap so the user can
    // verify the right cells are being scored.
    let (show_grid_points, set_show_grid_points) = signal(false);

    // Two grids as before — roadway-only for compliance, widened for viz.
    let area_result = Memo::new(move |_| -> Option<AreaResult> {
        let ldc_val = ldc.get()?;
        Some(layout.get().compute(&ldc_val, 0.8))
    });
    let area_result_with_walk = Memo::new(move |_| -> Option<AreaResult> {
        let ldc_val = ldc.get()?;
        Some(layout.get().compute_with_sidewalks(&ldc_val, 0.8))
    });

    // Compliance: always MLO (env zone check), plus the *selected*
    // standard's row. Other standards are hidden — the decision tree
    // already guides the user into the right region.
    let compliance = Memo::new(move |_| -> Vec<ComplianceResult> {
        let Some(ldc_val) = ldc.get() else {
            return Vec::new();
        };
        let Some(area) = area_result.get() else {
            return Vec::new();
        };
        let design = layout.get().design_result(&area);
        let mut out: Vec<ComplianceResult> = Vec::new();
        if let Some(r) = MloStandard.check_file(&mlo_zone.get(), &ldc_val) {
            out.push(r);
        }
        match standard.get() {
            AuthoritativeStandard::Rp8 => {
                if let Some(r) = Rp8Standard.check_design(&rp8_sel.get(), &design) {
                    out.push(r);
                }
            }
            AuthoritativeStandard::En13201 => {
                if let Some(r) = En13201Standard.check_design(&en_class.get(), &design) {
                    out.push(r);
                }
            }
            AuthoritativeStandard::Cjj45 => {
                if let Some(r) = Cjj45Standard.check_design(&cjj_class.get(), &design) {
                    out.push(r);
                }
            }
        }
        out
    });

    view! {
        <div class="street-app">
            <LdcSourceBadge ldc=ldc locale=locale />

            <StandardSelector standard=standard set_standard=set_standard locale=locale />

            {move || match standard.get() {
                AuthoritativeStandard::Rp8 => view! {
                    <Rp8Tree
                        road=rp8_road set_road=set_rp8_road
                        ped=rp8_ped set_ped=set_rp8_ped
                        locale=locale />
                }.into_any(),
                AuthoritativeStandard::En13201 => view! {
                    <En13201Tree
                        family=en_family set_family=set_en_family
                        c_params=en_c_params set_c_params=set_en_c_params
                        p_params=en_p_params set_p_params=set_en_p_params
                        recommended=en_class
                        locale=locale />
                }.into_any(),
                AuthoritativeStandard::Cjj45 => view! {
                    <Cjj45Tree grade=cjj_grade set_grade=set_cjj_grade locale=locale />
                }.into_any(),
            }}

            <LayoutForm layout=layout set_layout=set_layout locale=locale units=units />

            {move || area_result.get().map(|road_area| {
                let layout_val = layout.get();
                let overlay = resolve_overlay(
                    threshold_source.get(),
                    custom_ratio.get(),
                    rp8_sel.get(),
                    en_class.get(),
                    cjj_class.get(),
                );
                let viz_area = area_result_with_walk.get().unwrap_or_else(|| road_area.clone());
                let svg = render_plan_svg(&layout_val, &viz_area, overlay, show_grid_points.get());
                let u = units.get();
                let avg = u.convert_lux(road_area.avg_lux);
                let min = u.convert_lux(road_area.min_lux);
                let max = u.convert_lux(road_area.max_lux);
                let l = locale.get();
                let threshold_label = describe_overlay(overlay, &l, u);
                let stats_avg = l.street.stats.avg.clone();
                let stats_min = l.street.stats.min.clone();
                let stats_max = l.street.stats.max.clone();
                let stats_unit = u.illuminance_label().to_string();
                let grid_label = l.street.plan_overlays.grid_points.clone();
                let dl_title = l.street.exports.plan_svg.clone();
                let plan_svg_for_dl = svg.clone();
                let on_inline_dl = move |_| {
                    save_svg(&plan_svg_for_dl, "street-plan-view.svg");
                };
                view! {
                    <section class="street-viz">
                        <div class="street-viz-toolbar">
                            <ThresholdControls
                                source=threshold_source
                                set_source=set_threshold_source
                                custom_ratio=custom_ratio
                                set_custom_ratio=set_custom_ratio
                                threshold_label=threshold_label
                                locale=locale />
                            <label class="street-viz-overlay-toggle">
                                <input
                                    type="checkbox"
                                    prop:checked=move || show_grid_points.get()
                                    on:change=move |ev| {
                                        use wasm_bindgen::JsCast;
                                        let checked = ev
                                            .target()
                                            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                                            .map(|i| i.checked())
                                            .unwrap_or(false);
                                        set_show_grid_points.set(checked);
                                    } />
                                {grid_label}
                            </label>
                            <button
                                type="button"
                                class="street-inline-dl"
                                title=dl_title
                                on:click=on_inline_dl>
                                "⬇ SVG"
                            </button>
                            <div class="street-viz-stats">
                                {stats_avg} " " <strong>{format!("{avg:.1}")}</strong> " · "
                                {stats_min} " " <strong>{format!("{min:.1}")}</strong> " · "
                                {stats_max} " " <strong>{format!("{max:.1}")}</strong> " " {stats_unit}
                            </div>
                        </div>
                        <div class="street-viz-canvas" inner_html=svg></div>
                    </section>
                }.into_any()
            }).unwrap_or_else(|| {
                let placeholder = locale.get().street.placeholder.clone();
                view! { <p class="street-placeholder">{placeholder}</p> }.into_any()
            })}

            // Principal-planes (PV/PC) intensity diagram — appears
            // whenever an LDC is loaded. Hidden behind a toggle to keep
            // the panel compact for users who only care about the plan
            // view.
            {move || {
                ldc.get().map(|ldc_val| {
                    let l = locale.get();
                    let toggle_label = if principal_planes_visible.get() {
                        l.street.principal_planes.hide.clone()
                    } else {
                        l.street.principal_planes.show.clone()
                    };
                    let dl_title = l.street.exports.principal_planes_svg.clone();
                    let chart = if principal_planes_visible.get() {
                        Some(principal_planes_svg(&ldc_val, 720.0, 380.0, &SvgTheme::default()))
                    } else {
                        None
                    };
                    let chart_for_dl = chart.clone();
                    let on_inline_dl = move |_| {
                        // Always re-render at download time so a fresh SVG goes
                        // out even if the toggle is currently hiding it. This
                        // matches the intent: the SVG is *available*, the
                        // toggle just gates the inline preview.
                        if let Some(ldc_now) = ldc.get_untracked() {
                            let svg = chart_for_dl.clone().unwrap_or_else(|| {
                                principal_planes_svg(
                                    &ldc_now,
                                    720.0,
                                    380.0,
                                    &SvgTheme::default(),
                                )
                            });
                            save_svg(&svg, "street-principal-planes.svg");
                        }
                    };
                    view! {
                        <div class="street-pv-pc">
                            <button
                                class="street-pv-pc-toggle"
                                on:click=move |_| set_principal_planes_visible.update(|v| *v = !*v)>
                                {toggle_label}
                            </button>
                            <button
                                type="button"
                                class="street-inline-dl"
                                title=dl_title
                                on:click=on_inline_dl>
                                "⬇ SVG"
                            </button>
                            {chart.map(|svg| view! {
                                <div class="street-pv-pc-canvas" inner_html=svg />
                            })}
                        </div>
                    }
                })
            }}

            {move || {
                let results = compliance.get();
                let heading = locale.get().street.compliance.heading.clone();
                (!results.is_empty()).then(|| view! {
                    <h2>{heading}</h2>
                    <ResultsTable results=results locale=locale />
                })
            }}

            {move || {
                let results = compliance.get();
                let tips = advise(&results);
                (!tips.is_empty()).then(|| {
                    let heading = locale.get().street.advisor.heading.clone();
                    view! { <AdvisorPanel heading=heading tips=tips /> }
                })
            }}

            <OptimizerPanel
                ldc=ldc
                layout=layout
                set_layout=set_layout
                standard=standard
                rp8_sel=rp8_sel
                en_class=en_class
                cjj_class=cjj_class
                objective=optimizer_objective
                set_objective=set_optimizer_objective
                results=optimizer_results
                set_results=set_optimizer_results
                running=optimizer_running
                set_running=set_optimizer_running
                tradeoff_all=tradeoff_all
                set_tradeoff_all=set_tradeoff_all
                tradeoff_frontier=tradeoff_frontier
                set_tradeoff_frontier=set_tradeoff_frontier
                tradeoff_visible=tradeoff_visible
                set_tradeoff_visible=set_tradeoff_visible
                units=units
                locale=locale />

            <ExportsPanel
                ldc=ldc
                layout=layout
                area_result=area_result
                area_result_with_walk=area_result_with_walk
                threshold_source=threshold_source
                custom_ratio=custom_ratio
                rp8_sel=rp8_sel
                en_class=en_class
                cjj_class=cjj_class
                standard=standard
                compliance=compliance
                optimizer_top=optimizer_results
                tradeoff_all=tradeoff_all
                tradeoff_frontier=tradeoff_frontier
                show_grid_points=show_grid_points
                principal_planes_visible=principal_planes_visible
                units=units
                locale=locale />
        </div>
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Plan-view SVG + threshold helpers (unchanged from pre-decision-tree)
// ─────────────────────────────────────────────────────────────────────────

fn render_plan_svg(
    layout: &StreetLayout,
    area: &AreaResult,
    overlay: Option<FailureOverlay>,
    show_grid_points: bool,
) -> String {
    let opts = PlanViewOptions {
        theme: StreetTheme::Dark,
        failure_overlay: overlay,
        show_grid_points,
    };
    plan_view_heatmap(layout, area, 800.0, 280.0, opts)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ThresholdSource {
    Off,
    Rp8,
    En13201,
    Cjj45,
    Custom,
}

impl ThresholdSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Rp8 => "rp8",
            Self::En13201 => "en13201",
            Self::Cjj45 => "cjj45",
            Self::Custom => "custom",
        }
    }
    fn from_str(s: &str) -> Self {
        match s {
            "rp8" => Self::Rp8,
            "en13201" => Self::En13201,
            "cjj45" => Self::Cjj45,
            "custom" => Self::Custom,
            _ => Self::Off,
        }
    }
    fn label(self, loc: &Locale) -> String {
        match self {
            Self::Off => loc.street.threshold.off.clone(),
            Self::Rp8 => loc.street.threshold.rp8.clone(),
            Self::En13201 => loc.street.threshold.en13201.clone(),
            Self::Cjj45 => loc.street.threshold.cjj45.clone(),
            Self::Custom => loc.street.threshold.custom.clone(),
        }
    }
}

fn resolve_overlay(
    src: ThresholdSource,
    custom_ratio: f64,
    rp8: Rp8Selection,
    en: En13201Class,
    cjj: Cjj45Class,
) -> Option<FailureOverlay> {
    match src {
        ThresholdSource::Off => None,
        ThresholdSource::Rp8 => Some(rp8.failure_overlay()),
        ThresholdSource::En13201 => Some(en.failure_overlay()),
        ThresholdSource::Cjj45 => Some(cjj.failure_overlay()),
        ThresholdSource::Custom => Some(FailureOverlay::ratio(custom_ratio.clamp(0.0, 1.0))),
    }
}

fn describe_overlay(overlay: Option<FailureOverlay>, loc: &Locale, units: UnitSystem) -> String {
    match overlay {
        None => String::new(),
        Some(FailureOverlay::RatioFloor { min_over_avg }) => loc
            .street
            .threshold
            .below_ratio
            .replace("{pct}", &format!("{:.0}", min_over_avg * 100.0)),
        Some(FailureOverlay::AbsoluteLux { min_lux }) => {
            let display = units.convert_lux(min_lux);
            let unit_label = units.illuminance_label();
            loc.street
                .threshold
                .below_lux
                .replace("{lux}", &format!("{display:.1} {unit_label}"))
        }
    }
}

#[component]
fn ThresholdControls(
    source: ReadSignal<ThresholdSource>,
    set_source: WriteSignal<ThresholdSource>,
    custom_ratio: ReadSignal<f64>,
    set_custom_ratio: WriteSignal<f64>,
    threshold_label: String,
    locale: Memo<Locale>,
) -> impl IntoView {
    use wasm_bindgen::JsCast;

    view! {
        <div class="street-viz-controls">
            <label class="street-viz-dropdown">
                {move || format!("{} ", locale.get().street.threshold.label)}
                <select
                    on:change=move |ev| {
                        let v = ev.target()
                            .and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok())
                            .map(|s| s.value())
                            .unwrap_or_default();
                        set_source.set(ThresholdSource::from_str(&v));
                    }
                >
                    {move || {
                        let loc = locale.get();
                        [
                            ThresholdSource::Off,
                            ThresholdSource::Rp8,
                            ThresholdSource::En13201,
                            ThresholdSource::Cjj45,
                            ThresholdSource::Custom,
                        ].iter().map(|ts| {
                            let val = ts.as_str();
                            let label = ts.label(&loc);
                            let ts_copy = *ts;
                            let selected = Signal::derive(move || source.get() == ts_copy);
                            view! {
                                <option value=val prop:selected=move || selected.get()>{label}</option>
                            }
                        }).collect_view()
                    }}
                </select>
            </label>

            {move || (source.get() == ThresholdSource::Custom).then(|| {
                let ratio_label = locale.get().street.threshold.ratio.clone();
                view! {
                    <label class="street-viz-ratio">
                        {format!("{} ", ratio_label)}
                        <input
                            type="range"
                            min="0.1"
                            max="0.7"
                            step="0.05"
                            prop:value=move || format!("{:.2}", custom_ratio.get())
                            on:input=move |ev| {
                                if let Some(v) = ev.target()
                                    .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                                    .and_then(|i| i.value().parse::<f64>().ok())
                                {
                                    set_custom_ratio.set(v);
                                }
                            }
                        />
                        <span>{move || format!("{:.2}", custom_ratio.get())}</span>
                    </label>
                }
            })}

            {(!threshold_label.is_empty()).then(|| view! {
                <span class="street-viz-threshold">{threshold_label}</span>
            })}
        </div>
    }
}

#[component]
fn LdcSourceBadge(ldc: ReadSignal<Option<Eulumdat>>, locale: Memo<Locale>) -> impl IntoView {
    view! {
        <div class="street-source-panel">
            {move || {
                let loc = locale.get();
                match ldc.get() {
                    Some(l) => {
                        let name = if l.luminaire_name.is_empty() {
                            loc.street.source.unnamed.clone()
                        } else {
                            l.luminaire_name.clone()
                        };
                        let using = loc.street.source.using.clone();
                        let live = loc.street.source.live_updates.clone();
                        view! {
                            <div class="street-source-badge street-source-badge--editor">
                                <span class="dot"></span>
                                {using}
                                <strong>{name}</strong>
                                <span class="text-muted">{live}</span>
                            </div>
                        }.into_any()
                    },
                    None => {
                        let hint = loc.street.source.hint_none.clone();
                        view! {
                            <div class="street-source-hint text-muted">{hint}</div>
                        }.into_any()
                    },
                }
            }}
        </div>
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Standard selector
// ─────────────────────────────────────────────────────────────────────────

#[component]
fn StandardSelector(
    standard: ReadSignal<AuthoritativeStandard>,
    set_standard: WriteSignal<AuthoritativeStandard>,
    locale: Memo<Locale>,
) -> impl IntoView {
    use wasm_bindgen::JsCast;
    view! {
        <fieldset class="street-standard-selector">
            <legend>{move || locale.get().street.standard_selector.legend.clone()}</legend>
            <label>
                <select on:change=move |ev| {
                    let v = ev.target()
                        .and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok())
                        .map(|s| s.value())
                        .unwrap_or_default();
                    let s = match v.as_str() {
                        "rp8" => AuthoritativeStandard::Rp8,
                        "en13201" => AuthoritativeStandard::En13201,
                        "cjj45" => AuthoritativeStandard::Cjj45,
                        _ => AuthoritativeStandard::Rp8,
                    };
                    set_standard.set(s);
                }>
                    {move || {
                        let l = locale.get();
                        let items = [
                            ("rp8", AuthoritativeStandard::Rp8, l.street.standard_selector.rp8.clone()),
                            ("en13201", AuthoritativeStandard::En13201, l.street.standard_selector.en13201.clone()),
                            ("cjj45", AuthoritativeStandard::Cjj45, l.street.standard_selector.cjj45.clone()),
                        ];
                        items.into_iter().map(|(code, v, label)| {
                            let sel = Signal::derive(move || standard.get() == v);
                            view! {
                                <option value=code prop:selected=move || sel.get()>{label}</option>
                            }
                        }).collect_view()
                    }}
                </select>
            </label>
        </fieldset>
    }
}

// ─────────────────────────────────────────────────────────────────────────
// RP-8 decision tree (RoadClass × PedestrianConflict)
// ─────────────────────────────────────────────────────────────────────────

#[component]
fn Rp8Tree(
    road: ReadSignal<RoadClass>,
    set_road: WriteSignal<RoadClass>,
    ped: ReadSignal<PedestrianConflict>,
    set_ped: WriteSignal<PedestrianConflict>,
    locale: Memo<Locale>,
) -> impl IntoView {
    use wasm_bindgen::JsCast;

    let on_road_change = move |ev: web_sys::Event| {
        let v = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok())
            .map(|s| s.value())
            .unwrap_or_default();
        let r = match v.as_str() {
            "Major" => RoadClass::Major,
            "Collector" => RoadClass::Collector,
            "Local" => RoadClass::Local,
            _ => RoadClass::Collector,
        };
        set_road.set(r);
    };
    let on_ped_change = move |ev: web_sys::Event| {
        let v = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok())
            .map(|s| s.value())
            .unwrap_or_default();
        let p = match v.as_str() {
            "High" => PedestrianConflict::High,
            "Medium" => PedestrianConflict::Medium,
            "Low" => PedestrianConflict::Low,
            _ => PedestrianConflict::Medium,
        };
        set_ped.set(p);
    };

    view! {
        <fieldset class="street-decision-tree">
            <legend>{move || locale.get().street.rp8_tree.legend.clone()}</legend>
            <div class="street-tree-grid">
                <label>
                    {move || format!("{}: ", locale.get().street.rp8_tree.road_class)}
                    <select on:change=on_road_change prop:value=move || format!("{:?}", road.get())>
                        {move || {
                            let l = locale.get();
                            [
                                ("Major", l.street.rp8_tree.major.clone()),
                                ("Collector", l.street.rp8_tree.collector.clone()),
                                ("Local", l.street.rp8_tree.local.clone()),
                            ].into_iter().map(|(v, label)| view! {
                                <option value=v>{label}</option>
                            }).collect_view()
                        }}
                    </select>
                </label>
                <label>
                    {move || format!("{}: ", locale.get().street.rp8_tree.pedestrian_conflict)}
                    <select on:change=on_ped_change prop:value=move || format!("{:?}", ped.get())>
                        {move || {
                            let l = locale.get();
                            [
                                ("High", l.street.rp8_tree.ped_high.clone()),
                                ("Medium", l.street.rp8_tree.ped_medium.clone()),
                                ("Low", l.street.rp8_tree.ped_low.clone()),
                            ].into_iter().map(|(v, label)| view! {
                                <option value=v>{label}</option>
                            }).collect_view()
                        }}
                    </select>
                </label>
            </div>
        </fieldset>
    }
}

// ─────────────────────────────────────────────────────────────────────────
// EN 13201 decision tree (C and P families, full matrix)
// ─────────────────────────────────────────────────────────────────────────

#[component]
fn En13201Tree(
    family: ReadSignal<EnFamily>,
    set_family: WriteSignal<EnFamily>,
    c_params: ReadSignal<CParamsEn13201>,
    set_c_params: WriteSignal<CParamsEn13201>,
    p_params: ReadSignal<PParamsEn13201>,
    set_p_params: WriteSignal<PParamsEn13201>,
    recommended: Memo<En13201Class>,
    locale: Memo<Locale>,
) -> impl IntoView {
    use wasm_bindgen::JsCast;

    let on_family_change = move |ev: web_sys::Event| {
        let v = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok())
            .map(|s| s.value())
            .unwrap_or_default();
        set_family.set(if v == "P" { EnFamily::P } else { EnFamily::C });
    };

    let vws_display = Memo::new(move |_| match family.get() {
        EnFamily::C => c_params.get().vws(),
        EnFamily::P => p_params.get().vws(),
    });

    view! {
        <fieldset class="street-decision-tree">
            <legend>{move || locale.get().street.en13201_tree.legend.clone()}</legend>
            <div class="street-tree-grid">
                <label>
                    {move || format!("{}: ", locale.get().street.en13201_tree.family)}
                    <select on:change=on_family_change prop:value=move || match family.get() { EnFamily::C => "C", EnFamily::P => "P" }>
                        {move || {
                            let l = locale.get();
                            [
                                ("C", l.street.en13201_tree.family_c.clone()),
                                ("P", l.street.en13201_tree.family_p.clone()),
                            ].into_iter().map(|(v, label)| view! {
                                <option value=v>{label}</option>
                            }).collect_view()
                        }}
                    </select>
                </label>

                // ── C-family parameters ─────────────────────────────────
                {move || (family.get() == EnFamily::C).then(|| view! {
                    <EnCParamsForm params=c_params set_params=set_c_params locale=locale />
                })}

                // ── P-family parameters ─────────────────────────────────
                {move || (family.get() == EnFamily::P).then(|| view! {
                    <EnPParamsForm params=p_params set_params=set_p_params locale=locale />
                })}
            </div>

            <div class="street-tree-summary">
                {move || format!("Vws = {}", vws_display.get())}
                " · "
                <strong>{move || format!("{:?}", recommended.get())}</strong>
            </div>
        </fieldset>
    }
}

#[component]
fn EnCParamsForm(
    params: ReadSignal<CParamsEn13201>,
    set_params: WriteSignal<CParamsEn13201>,
    locale: Memo<Locale>,
) -> impl IntoView {
    // One helper macro would be cleaner but expanded inline keeps the
    // component readable — each dropdown follows the same pattern.
    use wasm_bindgen::JsCast;

    let pick = |ev: &web_sys::Event| -> String {
        ev.target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok())
            .map(|s| s.value())
            .unwrap_or_default()
    };

    view! {
        <>
        // Design speed
        <label>
            {move || format!("{}: ", locale.get().street.en13201_tree.design_speed)}
            <select
                prop:value=move || format!("{:?}", params.get().design_speed)
                on:change=move |ev| {
                    let v = pick(&ev);
                    let x = match v.as_str() {
                        "VeryHigh" => DesignSpeed::VeryHigh,
                        "High" => DesignSpeed::High,
                        "Moderate" => DesignSpeed::Moderate,
                        "Low" => DesignSpeed::Low,
                        _ => DesignSpeed::High,
                    };
                    set_params.update(|p| p.design_speed = x);
                }
            >
                {move || {
                    let l = locale.get();
                    [
                        ("VeryHigh", l.street.en13201_tree.speed_very_high.clone()),
                        ("High",     l.street.en13201_tree.speed_high.clone()),
                        ("Moderate", l.street.en13201_tree.speed_moderate.clone()),
                        ("Low",      l.street.en13201_tree.speed_low.clone()),
                    ].into_iter().map(|(v, label)| view! {
                        <option value=v>{label}</option>
                    }).collect_view()
                }}
            </select>
        </label>

        // Traffic volume
        <label>
            {move || format!("{}: ", locale.get().street.en13201_tree.traffic_volume)}
            <select
                prop:value=move || format!("{:?}", params.get().traffic_volume)
                on:change=move |ev| {
                    let v = pick(&ev);
                    let x = match v.as_str() {
                        "VeryHigh" => TrafficVolume::VeryHigh,
                        "High" => TrafficVolume::High,
                        "Moderate" => TrafficVolume::Moderate,
                        "Low" => TrafficVolume::Low,
                        "VeryLow" => TrafficVolume::VeryLow,
                        _ => TrafficVolume::Moderate,
                    };
                    set_params.update(|p| p.traffic_volume = x);
                }
            >
                {move || {
                    let l = locale.get();
                    [
                        ("VeryHigh", l.street.en13201_tree.vol_very_high.clone()),
                        ("High",     l.street.en13201_tree.vol_high.clone()),
                        ("Moderate", l.street.en13201_tree.vol_moderate.clone()),
                        ("Low",      l.street.en13201_tree.vol_low.clone()),
                        ("VeryLow",  l.street.en13201_tree.vol_very_low.clone()),
                    ].into_iter().map(|(v, label)| view! {
                        <option value=v>{label}</option>
                    }).collect_view()
                }}
            </select>
        </label>

        // Traffic composition
        <label>
            {move || format!("{}: ", locale.get().street.en13201_tree.traffic_composition)}
            <select
                prop:value=move || format!("{:?}", params.get().traffic_composition)
                on:change=move |ev| {
                    let v = pick(&ev);
                    let x = match v.as_str() {
                        "Mixed" => TrafficComposition::Mixed,
                        "MixedMotorDominant" => TrafficComposition::MixedMotorDominant,
                        "MotorOnly" => TrafficComposition::MotorOnly,
                        _ => TrafficComposition::MixedMotorDominant,
                    };
                    set_params.update(|p| p.traffic_composition = x);
                }
            >
                {move || {
                    let l = locale.get();
                    [
                        ("Mixed",              l.street.en13201_tree.comp_mixed.clone()),
                        ("MixedMotorDominant", l.street.en13201_tree.comp_mixed_motor.clone()),
                        ("MotorOnly",          l.street.en13201_tree.comp_motor_only.clone()),
                    ].into_iter().map(|(v, label)| view! {
                        <option value=v>{label}</option>
                    }).collect_view()
                }}
            </select>
        </label>

        // Separation
        <label>
            {move || format!("{}: ", locale.get().street.en13201_tree.separation)}
            <select
                prop:value=move || format!("{:?}", params.get().separation)
                on:change=move |ev| {
                    let v = pick(&ev);
                    let x = match v.as_str() {
                        "Separated" => Separation::Separated,
                        "NotSeparated" => Separation::NotSeparated,
                        _ => Separation::NotSeparated,
                    };
                    set_params.update(|p| p.separation = x);
                }
            >
                {move || {
                    let l = locale.get();
                    [
                        ("Separated",    l.street.en13201_tree.sep_yes.clone()),
                        ("NotSeparated", l.street.en13201_tree.sep_no.clone()),
                    ].into_iter().map(|(v, label)| view! {
                        <option value=v>{label}</option>
                    }).collect_view()
                }}
            </select>
        </label>

        // Junction density
        <label>
            {move || format!("{}: ", locale.get().street.en13201_tree.junction_density)}
            <select
                prop:value=move || format!("{:?}", params.get().junction_density)
                on:change=move |ev| {
                    let v = pick(&ev);
                    let x = match v.as_str() {
                        "High" => JunctionDensity::High,
                        "Moderate" => JunctionDensity::Moderate,
                        _ => JunctionDensity::Moderate,
                    };
                    set_params.update(|p| p.junction_density = x);
                }
            >
                {move || {
                    let l = locale.get();
                    [
                        ("High",     l.street.en13201_tree.jd_high.clone()),
                        ("Moderate", l.street.en13201_tree.jd_moderate.clone()),
                    ].into_iter().map(|(v, label)| view! {
                        <option value=v>{label}</option>
                    }).collect_view()
                }}
            </select>
        </label>

        // Parked vehicles
        <label>
            {move || format!("{}: ", locale.get().street.en13201_tree.parked_vehicles)}
            <select
                prop:value=move || format!("{:?}", params.get().parked_vehicles)
                on:change=move |ev| {
                    let v = pick(&ev);
                    let x = match v.as_str() {
                        "Present" => ParkedVehicles::Present,
                        "NotPresent" => ParkedVehicles::NotPresent,
                        _ => ParkedVehicles::NotPresent,
                    };
                    set_params.update(|p| p.parked_vehicles = x);
                }
            >
                {move || {
                    let l = locale.get();
                    [
                        ("Present",    l.street.en13201_tree.parked_yes.clone()),
                        ("NotPresent", l.street.en13201_tree.parked_no.clone()),
                    ].into_iter().map(|(v, label)| view! {
                        <option value=v>{label}</option>
                    }).collect_view()
                }}
            </select>
        </label>

        // Ambient luminance
        <label>
            {move || format!("{}: ", locale.get().street.en13201_tree.ambient_luminance)}
            <select
                prop:value=move || format!("{:?}", params.get().ambient_luminance)
                on:change=move |ev| {
                    let v = pick(&ev);
                    let x = match v.as_str() {
                        "High" => AmbientLuminance::High,
                        "Moderate" => AmbientLuminance::Moderate,
                        "Low" => AmbientLuminance::Low,
                        _ => AmbientLuminance::Moderate,
                    };
                    set_params.update(|p| p.ambient_luminance = x);
                }
            >
                {move || {
                    let l = locale.get();
                    [
                        ("High",     l.street.en13201_tree.amb_high.clone()),
                        ("Moderate", l.street.en13201_tree.amb_moderate.clone()),
                        ("Low",      l.street.en13201_tree.amb_low.clone()),
                    ].into_iter().map(|(v, label)| view! {
                        <option value=v>{label}</option>
                    }).collect_view()
                }}
            </select>
        </label>

        // Navigational task
        <label>
            {move || format!("{}: ", locale.get().street.en13201_tree.navigational_task)}
            <select
                prop:value=move || format!("{:?}", params.get().navigational_task)
                on:change=move |ev| {
                    let v = pick(&ev);
                    let x = match v.as_str() {
                        "VeryDifficult" => NavigationalTask::VeryDifficult,
                        "Difficult" => NavigationalTask::Difficult,
                        "Easy" => NavigationalTask::Easy,
                        _ => NavigationalTask::Easy,
                    };
                    set_params.update(|p| p.navigational_task = x);
                }
            >
                {move || {
                    let l = locale.get();
                    [
                        ("VeryDifficult", l.street.en13201_tree.nav_very_difficult.clone()),
                        ("Difficult",     l.street.en13201_tree.nav_difficult.clone()),
                        ("Easy",          l.street.en13201_tree.nav_easy.clone()),
                    ].into_iter().map(|(v, label)| view! {
                        <option value=v>{label}</option>
                    }).collect_view()
                }}
            </select>
        </label>
        </>
    }
}

#[component]
fn EnPParamsForm(
    params: ReadSignal<PParamsEn13201>,
    set_params: WriteSignal<PParamsEn13201>,
    locale: Memo<Locale>,
) -> impl IntoView {
    use wasm_bindgen::JsCast;
    let pick = |ev: &web_sys::Event| -> String {
        ev.target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok())
            .map(|s| s.value())
            .unwrap_or_default()
    };

    view! {
        <>
        <label>
            {move || format!("{}: ", locale.get().street.en13201_tree.pedestrian_speed)}
            <select
                prop:value=move || format!("{:?}", params.get().pedestrian_speed)
                on:change=move |ev| {
                    let v = pick(&ev);
                    let x = match v.as_str() {
                        "WalkingOnly" => PedestrianSpeed::WalkingOnly,
                        "SlowMixed" => PedestrianSpeed::SlowMixed,
                        "LowSpeedMotor" => PedestrianSpeed::LowSpeedMotor,
                        _ => PedestrianSpeed::SlowMixed,
                    };
                    set_params.update(|p| p.pedestrian_speed = x);
                }
            >
                {move || {
                    let l = locale.get();
                    [
                        ("WalkingOnly",   l.street.en13201_tree.pspeed_walking.clone()),
                        ("SlowMixed",     l.street.en13201_tree.pspeed_slow_mixed.clone()),
                        ("LowSpeedMotor", l.street.en13201_tree.pspeed_low_motor.clone()),
                    ].into_iter().map(|(v, label)| view! {
                        <option value=v>{label}</option>
                    }).collect_view()
                }}
            </select>
        </label>

        <label>
            {move || format!("{}: ", locale.get().street.en13201_tree.user_density)}
            <select
                prop:value=move || format!("{:?}", params.get().user_density)
                on:change=move |ev| {
                    let v = pick(&ev);
                    let x = match v.as_str() {
                        "High" => UserDensity::High,
                        "Moderate" => UserDensity::Moderate,
                        "Low" => UserDensity::Low,
                        _ => UserDensity::Moderate,
                    };
                    set_params.update(|p| p.user_density = x);
                }
            >
                {move || {
                    let l = locale.get();
                    [
                        ("High",     l.street.en13201_tree.ud_high.clone()),
                        ("Moderate", l.street.en13201_tree.ud_moderate.clone()),
                        ("Low",      l.street.en13201_tree.ud_low.clone()),
                    ].into_iter().map(|(v, label)| view! {
                        <option value=v>{label}</option>
                    }).collect_view()
                }}
            </select>
        </label>

        <label>
            {move || format!("{}: ", locale.get().street.en13201_tree.ambient_luminance)}
            <select
                prop:value=move || format!("{:?}", params.get().ambient_luminance)
                on:change=move |ev| {
                    let v = pick(&ev);
                    let x = match v.as_str() {
                        "High" => AmbientLuminance::High,
                        "Moderate" => AmbientLuminance::Moderate,
                        "Low" => AmbientLuminance::Low,
                        _ => AmbientLuminance::Moderate,
                    };
                    set_params.update(|p| p.ambient_luminance = x);
                }
            >
                {move || {
                    let l = locale.get();
                    [
                        ("High",     l.street.en13201_tree.amb_high.clone()),
                        ("Moderate", l.street.en13201_tree.amb_moderate.clone()),
                        ("Low",      l.street.en13201_tree.amb_low.clone()),
                    ].into_iter().map(|(v, label)| view! {
                        <option value=v>{label}</option>
                    }).collect_view()
                }}
            </select>
        </label>

        <label>
            {move || format!("{}: ", locale.get().street.en13201_tree.facial_recognition)}
            <select
                prop:value=move || format!("{:?}", params.get().facial_recognition)
                on:change=move |ev| {
                    let v = pick(&ev);
                    let x = match v.as_str() {
                        "Necessary" => FacialRecognition::Necessary,
                        "Useful" => FacialRecognition::Useful,
                        "NotNeeded" => FacialRecognition::NotNeeded,
                        _ => FacialRecognition::Useful,
                    };
                    set_params.update(|p| p.facial_recognition = x);
                }
            >
                {move || {
                    let l = locale.get();
                    [
                        ("Necessary", l.street.en13201_tree.fr_necessary.clone()),
                        ("Useful",    l.street.en13201_tree.fr_useful.clone()),
                        ("NotNeeded", l.street.en13201_tree.fr_not_needed.clone()),
                    ].into_iter().map(|(v, label)| view! {
                        <option value=v>{label}</option>
                    }).collect_view()
                }}
            </select>
        </label>
        </>
    }
}

// ─────────────────────────────────────────────────────────────────────────
// CJJ 45 decision tree (single Road Grade dropdown)
// ─────────────────────────────────────────────────────────────────────────

#[component]
fn Cjj45Tree(
    grade: ReadSignal<CjjRoadGrade>,
    set_grade: WriteSignal<CjjRoadGrade>,
    locale: Memo<Locale>,
) -> impl IntoView {
    use wasm_bindgen::JsCast;

    let on_change = move |ev: web_sys::Event| {
        let v = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok())
            .map(|s| s.value())
            .unwrap_or_default();
        let g = match v.as_str() {
            "Expressway" => CjjRoadGrade::Expressway,
            "MajorArterial" => CjjRoadGrade::MajorArterial,
            "MinorArterial" => CjjRoadGrade::MinorArterial,
            "Branch" => CjjRoadGrade::Branch,
            "Residential" => CjjRoadGrade::Residential,
            _ => CjjRoadGrade::MinorArterial,
        };
        set_grade.set(g);
    };

    view! {
        <fieldset class="street-decision-tree">
            <legend>{move || locale.get().street.cjj45_tree.legend.clone()}</legend>
            <label>
                {move || format!("{}: ", locale.get().street.cjj45_tree.road_grade)}
                <select on:change=on_change prop:value=move || format!("{:?}", grade.get())>
                    {move || {
                        let l = locale.get();
                        [
                            ("Expressway",    l.street.cjj45_tree.expressway.clone()),
                            ("MajorArterial", l.street.cjj45_tree.major_arterial.clone()),
                            ("MinorArterial", l.street.cjj45_tree.minor_arterial.clone()),
                            ("Branch",        l.street.cjj45_tree.branch.clone()),
                            ("Residential",   l.street.cjj45_tree.residential.clone()),
                        ].into_iter().map(|(v, label)| view! {
                            <option value=v>{label}</option>
                        }).collect_view()
                    }}
                </select>
            </label>
            <div class="street-tree-summary">
                <strong>{move || format!("{:?}", grade.get().to_class())}</strong>
            </div>
        </fieldset>
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Layout form (distance + unit conversion; unchanged)
// ─────────────────────────────────────────────────────────────────────────

#[component]
fn LayoutForm(
    layout: ReadSignal<StreetLayout>,
    set_layout: WriteSignal<StreetLayout>,
    locale: Memo<Locale>,
    units: ReadSignal<UnitSystem>,
) -> impl IntoView {
    let dist_label =
        move |raw: String| -> String { format!("{} ({})", raw, units.get().distance_label()) };

    view! {
        <fieldset style="margin: 1rem 0; padding: 1rem; border: 1px solid #ddd;">
            <legend>{move || locale.get().street.layout.legend.clone()}</legend>
            <div style="display: grid; grid-template-columns: repeat(2, 1fr); gap: 0.5rem 1rem;">
                <DistanceField
                    label=Signal::derive(move || dist_label(locale.get().street.layout.length.clone()))
                    value_m=Signal::derive(move || layout.get().length_m)
                    units=units
                    on_change_m=move |v| set_layout.update(|l| l.length_m = v) />
                <DistanceField
                    label=Signal::derive(move || dist_label(locale.get().street.layout.lane_width.clone()))
                    value_m=Signal::derive(move || layout.get().lane_width_m)
                    units=units
                    on_change_m=move |v| set_layout.update(|l| l.lane_width_m = v) />
                <NumberField
                    label=Signal::derive(move || locale.get().street.layout.num_lanes.clone())
                    value=Signal::derive(move || layout.get().num_lanes as f64)
                    on_change=move |v| set_layout.update(|l| l.num_lanes = v.max(1.0).round() as usize) />
                <DistanceField
                    label=Signal::derive(move || dist_label(locale.get().street.layout.pole_spacing.clone()))
                    value_m=Signal::derive(move || layout.get().pole_spacing_m)
                    units=units
                    on_change_m=move |v| set_layout.update(|l| l.pole_spacing_m = v) />
                <DistanceField
                    label=Signal::derive(move || dist_label(locale.get().street.layout.mounting_height.clone()))
                    value_m=Signal::derive(move || layout.get().mounting_height_m)
                    units=units
                    on_change_m=move |v| set_layout.update(|l| l.mounting_height_m = v) />
                <DistanceField
                    label=Signal::derive(move || dist_label(locale.get().street.layout.overhang.clone()))
                    value_m=Signal::derive(move || layout.get().overhang_m)
                    units=units
                    on_change_m=move |v| set_layout.update(|l| l.overhang_m = v) />
                <NumberField
                    label=Signal::derive(move || format!("{} (°)", locale.get().street.layout.tilt))
                    value=Signal::derive(move || layout.get().tilt_deg)
                    on_change=move |v| set_layout.update(|l| l.tilt_deg = v) />
                <DistanceField
                    label=Signal::derive(move || dist_label(locale.get().street.layout.pole_offset.clone()))
                    value_m=Signal::derive(move || layout.get().pole_offset_m)
                    units=units
                    on_change_m=move |v| set_layout.update(|l| l.pole_offset_m = v) />
                <DistanceField
                    label=Signal::derive(move || dist_label(locale.get().street.layout.sidewalk_width.clone()))
                    value_m=Signal::derive(move || layout.get().sidewalk_width_m)
                    units=units
                    on_change_m=move |v| set_layout.update(|l| l.sidewalk_width_m = v.max(0.0)) />
                <label style="grid-column: 1 / -1;">
                    {move || format!("{} ", locale.get().street.layout.arrangement_label)}
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
                        {move || {
                            let loc = locale.get();
                            [
                                (Arrangement::SingleSide, loc.street.arrangement.single_side.clone()),
                                (Arrangement::Opposite,   loc.street.arrangement.opposite.clone()),
                                (Arrangement::Staggered,  loc.street.arrangement.staggered.clone()),
                            ].into_iter().map(|(a, label)| view! {
                                <option value={format!("{:?}", a)}>{label}</option>
                            }).collect_view()
                        }}
                    </select>
                </label>
            </div>
        </fieldset>
    }
}

#[component]
fn DistanceField(
    label: Signal<String>,
    value_m: Signal<f64>,
    units: ReadSignal<UnitSystem>,
    on_change_m: impl Fn(f64) + 'static + Copy,
) -> impl IntoView {
    view! {
        <label>
            {move || format!("{}: ", label.get())}
            <input
                type="number"
                step="0.1"
                prop:value=move || {
                    let u = units.get();
                    format!("{:.2}", u.convert_meters(value_m.get()))
                }
                on:input=move |ev| {
                    use wasm_bindgen::JsCast;
                    let u = units.get();
                    let v = ev.target()
                        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                        .and_then(|i| i.value().parse::<f64>().ok());
                    if let Some(v) = v {
                        on_change_m(u.to_meters(v));
                    }
                }
            />
        </label>
    }
}

#[component]
fn NumberField(
    label: Signal<String>,
    value: Signal<f64>,
    on_change: impl Fn(f64) + 'static + Copy,
) -> impl IntoView {
    view! {
        <label>
            {move || format!("{}: ", label.get())}
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

// ─────────────────────────────────────────────────────────────────────────
// Optimizer panel
// ─────────────────────────────────────────────────────────────────────────

#[component]
#[allow(clippy::too_many_arguments)]
fn OptimizerPanel(
    ldc: ReadSignal<Option<Eulumdat>>,
    layout: ReadSignal<StreetLayout>,
    set_layout: WriteSignal<StreetLayout>,
    standard: ReadSignal<AuthoritativeStandard>,
    rp8_sel: Memo<Rp8Selection>,
    en_class: Memo<En13201Class>,
    cjj_class: Memo<Cjj45Class>,
    objective: ReadSignal<OptimizerObjective>,
    set_objective: WriteSignal<OptimizerObjective>,
    results: ReadSignal<Vec<OptimizationCandidate>>,
    set_results: WriteSignal<Vec<OptimizationCandidate>>,
    running: ReadSignal<bool>,
    set_running: WriteSignal<bool>,
    tradeoff_all: ReadSignal<Vec<OptimizationCandidate>>,
    set_tradeoff_all: WriteSignal<Vec<OptimizationCandidate>>,
    tradeoff_frontier: ReadSignal<Vec<usize>>,
    set_tradeoff_frontier: WriteSignal<Vec<usize>>,
    tradeoff_visible: ReadSignal<bool>,
    set_tradeoff_visible: WriteSignal<bool>,
    units: ReadSignal<UnitSystem>,
    locale: Memo<Locale>,
) -> impl IntoView {
    use wasm_bindgen::JsCast;

    let on_obj_change = move |ev: web_sys::Event| {
        let v = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok())
            .map(|s| s.value())
            .unwrap_or_default();
        let o = match v.as_str() {
            "TotalFluxPerKm" => OptimizerObjective::TotalFluxPerKm,
            "SafetyMargin" => OptimizerObjective::SafetyMargin,
            _ => OptimizerObjective::PoleCountPerKm,
        };
        set_objective.set(o);
    };

    let on_run = move |_| {
        let Some(ldc_val) = ldc.get() else { return };
        set_running.set(true);
        let layout_val = layout.get();
        let std_sel = standard.get();
        let obj = objective.get();
        let rp8 = rp8_sel.get();
        let en = en_class.get();
        let cjj = cjj_class.get();

        // The fit closure only depends on the chosen standard — clone
        // its small inputs and capture by move twice (once per call
        // below) since the optimizer takes `impl Fn` not `Fn` shared
        // across runs.
        let make_fit = move || {
            move |design: &eulumdat::standards::DesignResult| -> bool {
                match std_sel {
                    AuthoritativeStandard::Rp8 => Rp8Standard
                        .check_design(&rp8, design)
                        .map(|r| r.passed())
                        .unwrap_or(false),
                    AuthoritativeStandard::En13201 => En13201Standard
                        .check_design(&en, design)
                        .map(|r| r.passed())
                        .unwrap_or(false),
                    AuthoritativeStandard::Cjj45 => Cjj45Standard
                        .check_design(&cjj, design)
                        .map(|r| r.passed())
                        .unwrap_or(false),
                }
            }
        };

        let bounds = OptimizerBounds::default();

        // Full passing set — feeds both the top-3 table (after sort +
        // truncate below) and the layout trade-off chart.
        let mut all = optimize_layout_all(&ldc_val, &layout_val, &bounds, obj, make_fit());
        let frontier = pareto_front_tradeoff(&all);

        // Top-3 by current objective (mirrors the previous behavior of
        // `optimize_layout` so the table doesn't lose its existing UX).
        all.sort_by(|a, b| {
            a.cost
                .partial_cmp(&b.cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let top3: Vec<_> = all.iter().take(3).cloned().collect();

        set_results.set(top3);
        set_tradeoff_all.set(all);
        set_tradeoff_frontier.set(frontier);
        set_running.set(false);
    };

    let toggle_tradeoff = move |_| {
        set_tradeoff_visible.update(|v| *v = !*v);
    };

    let apply = move |cand: OptimizationCandidate| {
        set_layout.update(|l| {
            l.pole_spacing_m = cand.pole_spacing_m;
            l.mounting_height_m = cand.mounting_height_m;
            l.arrangement = cand.arrangement;
        });
    };

    view! {
        <fieldset class="street-optimizer">
            <legend>{move || locale.get().street.optimizer.legend.clone()}</legend>

            <div class="street-optimizer-controls">
                <label>
                    {move || format!("{}: ", locale.get().street.optimizer.objective_label)}
                    <select on:change=on_obj_change prop:value=move || format!("{:?}", objective.get())>
                        {move || {
                            let l = locale.get();
                            [
                                ("PoleCountPerKm",   l.street.optimizer.obj_pole_count.clone()),
                                ("TotalFluxPerKm",   l.street.optimizer.obj_flux.clone()),
                                ("SafetyMargin",     l.street.optimizer.obj_safety.clone()),
                            ].into_iter().map(|(v, label)| view! {
                                <option value=v>{label}</option>
                            }).collect_view()
                        }}
                    </select>
                </label>
                <button on:click=on_run disabled=move || running.get() || ldc.get().is_none()>
                    {move || {
                        let l = locale.get();
                        if running.get() { l.street.optimizer.running.clone() } else { l.street.optimizer.run.clone() }
                    }}
                </button>
            </div>

            {move || {
                let r = results.get();
                if r.is_empty() {
                    let msg = locale.get().street.optimizer.no_results.clone();
                    view! { <p class="text-muted">{msg}</p> }.into_any()
                } else {
                    let u = units.get();
                    let l = locale.get();
                    let header_spacing = l.street.optimizer.col_spacing.clone();
                    let header_height = l.street.optimizer.col_height.clone();
                    let header_arr = l.street.optimizer.col_arrangement.clone();
                    let header_poles = l.street.optimizer.col_poles.clone();
                    let header_flux = l.street.optimizer.col_flux.clone();
                    let header_uniformity = l.street.optimizer.col_uniformity.clone();
                    let header_apply = l.street.optimizer.col_apply.clone();
                    let apply_label = l.street.optimizer.apply.clone();
                    let arr_names = (
                        l.street.arrangement.single_side.clone(),
                        l.street.arrangement.opposite.clone(),
                        l.street.arrangement.staggered.clone(),
                    );
                    view! {
                        <table class="street-optimizer-results">
                            <thead>
                                <tr>
                                    <th>{header_spacing}</th>
                                    <th>{header_height}</th>
                                    <th>{header_arr}</th>
                                    <th>{header_poles}</th>
                                    <th>{header_flux}</th>
                                    <th>{header_uniformity}</th>
                                    <th>{header_apply}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {r.into_iter().map(|cand| {
                                    let cand_clone = cand.clone();
                                    let arr_label = match cand.arrangement {
                                        Arrangement::SingleSide => arr_names.0.clone(),
                                        Arrangement::Opposite   => arr_names.1.clone(),
                                        Arrangement::Staggered  => arr_names.2.clone(),
                                    };
                                    let spacing_disp = format!("{:.1} {}", u.convert_meters(cand.pole_spacing_m), u.distance_label());
                                    let height_disp = format!("{:.1} {}", u.convert_meters(cand.mounting_height_m), u.distance_label());
                                    let apply_btn_label = apply_label.clone();
                                    view! {
                                        <tr>
                                            <td>{spacing_disp}</td>
                                            <td>{height_disp}</td>
                                            <td>{arr_label}</td>
                                            <td>{format!("{:.1}", cand.poles_per_km)}</td>
                                            <td>{format!("{:.0} lm", cand.flux_per_km)}</td>
                                            <td>{format!("{:.2}", cand.design.uniformity_overall)}</td>
                                            <td>
                                                <button on:click=move |_| apply(cand_clone.clone())>
                                                    {apply_btn_label}
                                                </button>
                                            </td>
                                        </tr>
                                    }
                                }).collect_view()}
                            </tbody>
                        </table>
                    }.into_any()
                }
            }}

            // Layout trade-off chart — visible only after a run, behind a
            // toggle so the panel stays compact for the common case.
            {move || {
                let all = tradeoff_all.get();
                (!all.is_empty()).then(|| {
                    let l = locale.get();
                    let toggle_label = if tradeoff_visible.get() {
                        l.street.optimizer.tradeoff_hide.clone()
                    } else {
                        l.street.optimizer.tradeoff_show.clone()
                    };
                    let chart = if tradeoff_visible.get() {
                        let frontier = tradeoff_frontier.get();
                        let svg = layout_tradeoff_chart(
                            &all,
                            &frontier,
                            560.0,
                            340.0,
                            LayoutTradeoffOptions {
                                theme: StreetTheme::Dark,
                                highlight_idx: None,
                            },
                        );
                        Some(svg)
                    } else {
                        None
                    };

                    // Click-to-apply: event delegation on the canvas div.
                    // Each marker is emitted as a <g class="tradeoff-marker"
                    // data-idx="N">; we walk up from the click target until
                    // we find one, then apply candidate N.
                    let on_canvas_click = move |ev: web_sys::MouseEvent| {
                        use wasm_bindgen::JsCast;
                        let Some(target) = ev.target() else { return };
                        let Ok(node) = target.dyn_into::<web_sys::Element>() else {
                            return;
                        };
                        let Ok(Some(marker)) = node.closest(".tradeoff-marker") else {
                            return;
                        };
                        let Some(idx_str) = marker.get_attribute("data-idx") else {
                            return;
                        };
                        let Ok(idx) = idx_str.parse::<usize>() else {
                            return;
                        };
                        let snapshot = tradeoff_all.get_untracked();
                        if let Some(cand) = snapshot.get(idx) {
                            apply(cand.clone());
                        }
                    };

                    let dl_title = l.street.exports.tradeoff_svg.clone();
                    let on_inline_dl = move |_| {
                        let snapshot = tradeoff_all.get_untracked();
                        if snapshot.is_empty() {
                            return;
                        }
                        let frontier = tradeoff_frontier.get_untracked();
                        let svg = layout_tradeoff_chart(
                            &snapshot,
                            &frontier,
                            560.0,
                            340.0,
                            LayoutTradeoffOptions {
                                theme: StreetTheme::Dark,
                                highlight_idx: None,
                            },
                        );
                        save_svg(&svg, "street-tradeoff.svg");
                    };
                    view! {
                        <div class="street-tradeoff">
                            <button
                                class="street-tradeoff-toggle"
                                on:click=toggle_tradeoff>
                                {toggle_label}
                            </button>
                            <button
                                type="button"
                                class="street-inline-dl"
                                title=dl_title
                                on:click=on_inline_dl>
                                "⬇ SVG"
                            </button>
                            {chart.map(|svg| view! {
                                <div
                                    class="street-tradeoff-canvas"
                                    on:click=on_canvas_click
                                    inner_html=svg />
                            })}
                        </div>
                    }
                })
            }}
        </fieldset>
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Exports panel — SVG / CSV / JSON downloads + (later) Typst PDF report
// ─────────────────────────────────────────────────────────────────────────

#[component]
#[allow(clippy::too_many_arguments)]
fn ExportsPanel(
    ldc: ReadSignal<Option<Eulumdat>>,
    layout: ReadSignal<StreetLayout>,
    area_result: Memo<Option<AreaResult>>,
    area_result_with_walk: Memo<Option<AreaResult>>,
    threshold_source: ReadSignal<ThresholdSource>,
    custom_ratio: ReadSignal<f64>,
    rp8_sel: Memo<Rp8Selection>,
    en_class: Memo<En13201Class>,
    cjj_class: Memo<Cjj45Class>,
    standard: ReadSignal<AuthoritativeStandard>,
    compliance: Memo<Vec<ComplianceResult>>,
    optimizer_top: ReadSignal<Vec<OptimizationCandidate>>,
    tradeoff_all: ReadSignal<Vec<OptimizationCandidate>>,
    tradeoff_frontier: ReadSignal<Vec<usize>>,
    show_grid_points: ReadSignal<bool>,
    principal_planes_visible: ReadSignal<bool>,
    units: ReadSignal<UnitSystem>,
    locale: Memo<Locale>,
) -> impl IntoView {
    let _ = units; // reserved for unit-aware filenames

    let on_plan_svg = move |_| {
        let Some(road_area) = area_result.get() else {
            return;
        };
        let layout_val = layout.get();
        let overlay = resolve_overlay(
            threshold_source.get(),
            custom_ratio.get(),
            rp8_sel.get(),
            en_class.get(),
            cjj_class.get(),
        );
        let viz_area = area_result_with_walk
            .get()
            .unwrap_or_else(|| road_area.clone());
        let svg = render_plan_svg(&layout_val, &viz_area, overlay, show_grid_points.get());
        save_svg(&svg, "street-plan-view.svg");
    };

    let on_pv_pc_svg = move |_| {
        let Some(ldc_val) = ldc.get() else {
            return;
        };
        let svg = principal_planes_svg(&ldc_val, 720.0, 380.0, &SvgTheme::default());
        save_svg(&svg, "street-principal-planes.svg");
    };

    let on_tradeoff_svg = move |_| {
        let all = tradeoff_all.get();
        if all.is_empty() {
            return;
        }
        let frontier = tradeoff_frontier.get();
        let svg = layout_tradeoff_chart(
            &all,
            &frontier,
            560.0,
            340.0,
            LayoutTradeoffOptions {
                theme: StreetTheme::Dark,
                highlight_idx: None,
            },
        );
        save_svg(&svg, "street-tradeoff.svg");
    };

    let on_optimizer_csv = move |_| {
        let all = tradeoff_all.get();
        if all.is_empty() {
            return;
        }
        save_csv(&optimizer_csv(&all), "street-optimizer.csv");
    };

    let on_optimizer_json = move |_| {
        let all = tradeoff_all.get();
        if all.is_empty() {
            return;
        }
        let frontier = tradeoff_frontier.get();
        save_json(&optimizer_json(&all, &frontier), "street-optimizer.json");
    };

    let on_report = move |_| {
        let Some(ldc_val) = ldc.get() else {
            return;
        };
        let Some(road_area) = area_result.get() else {
            return;
        };
        let layout_val = layout.get();
        let overlay = resolve_overlay(
            threshold_source.get(),
            custom_ratio.get(),
            rp8_sel.get(),
            en_class.get(),
            cjj_class.get(),
        );
        let viz_area = area_result_with_walk
            .get()
            .unwrap_or_else(|| road_area.clone());
        let plan_svg = render_plan_svg(&layout_val, &viz_area, overlay, show_grid_points.get());
        // PV/PC follows the on-screen toggle: if the user has it hidden,
        // the PDF skips the section too. This keeps the report congruent
        // with what's visible in the app.
        let include_pv_pc = principal_planes_visible.get();
        let pv_pc = if include_pv_pc {
            Some(principal_planes_svg(
                &ldc_val,
                720.0,
                380.0,
                &SvgTheme::default(),
            ))
        } else {
            None
        };
        let tradeoff_set = tradeoff_all.get();
        let tradeoff_svg = if tradeoff_set.is_empty() {
            None
        } else {
            let frontier = tradeoff_frontier.get();
            Some(layout_tradeoff_chart(
                &tradeoff_set,
                &frontier,
                560.0,
                340.0,
                LayoutTradeoffOptions {
                    theme: StreetTheme::Dark,
                    highlight_idx: None,
                },
            ))
        };
        let standard_name = match standard.get() {
            AuthoritativeStandard::Rp8 => "ANSI/IES RP-8",
            AuthoritativeStandard::En13201 => "DIN EN 13201",
            AuthoritativeStandard::Cjj45 => "CJJ 45",
        };
        let compliance_val = compliance.get();
        let optimizer_top_val = optimizer_top.get();
        let luminaire = if ldc_val.luminaire_name.is_empty() {
            "(unnamed luminaire)".to_string()
        } else {
            ldc_val.luminaire_name.clone()
        };
        let typ = {
            let input = StreetReportInput {
                luminaire_name: &luminaire,
                standard_name,
                layout: &layout_val,
                area: &road_area,
                compliance: &compliance_val,
                plan_svg: Some(&plan_svg),
                principal_planes_svg: pv_pc.as_deref(),
                tradeoff_svg: tradeoff_svg.as_deref(),
                optimizer_top: &optimizer_top_val,
            };
            generate_street_report(&input)
        };

        // Hand the source off to typst-loader.js, which lazy-loads the
        // typst WASM module on first call (~10 MB compressed) and returns
        // PDF bytes. We download the result as `street-report.pdf`.
        wasm_bindgen_futures::spawn_local(async move {
            let promise = match compile_typst_to_pdf(&typ) {
                Ok(p) => p,
                Err(e) => {
                    web_sys::console::error_1(&e);
                    return;
                }
            };
            match JsFuture::from(promise).await {
                Ok(value) => {
                    let bytes: Vec<u8> = match value.dyn_into::<js_sys::Uint8Array>() {
                        Ok(arr) => arr.to_vec(),
                        Err(other) => {
                            web_sys::console::error_2(
                                &JsValue::from_str("compileTypstToPdf returned non-Uint8Array"),
                                &other,
                            );
                            return;
                        }
                    };
                    save_pdf(&bytes, "street-report.pdf");
                }
                Err(e) => {
                    web_sys::console::error_2(&JsValue::from_str("Typst compilation failed"), &e);
                }
            }
        });
    };

    view! {
        <fieldset class="street-exports">
            {move || {
                let l = locale.get();
                let heading = l.street.exports.heading.clone();
                let plan_label = l.street.exports.plan_svg.clone();
                let pv_pc_label = l.street.exports.principal_planes_svg.clone();
                let tradeoff_label = l.street.exports.tradeoff_svg.clone();
                let csv_label = l.street.exports.optimizer_csv.clone();
                let json_label = l.street.exports.optimizer_json.clone();
                let report_label = l.street.exports.report_pdf.clone();
                let report_hint = l.street.exports.report_pdf_hint.clone();
                view! {
                    <legend>{heading}</legend>
                    <div class="street-exports-row">
                        <button
                            type="button"
                            disabled=move || area_result.get().is_none()
                            on:click=on_plan_svg>
                            {plan_label}
                        </button>
                        <button
                            type="button"
                            disabled=move || ldc.get().is_none()
                            on:click=on_pv_pc_svg>
                            {pv_pc_label}
                        </button>
                        <button
                            type="button"
                            disabled=move || tradeoff_all.get().is_empty()
                            on:click=on_tradeoff_svg>
                            {tradeoff_label}
                        </button>
                        <button
                            type="button"
                            disabled=move || tradeoff_all.get().is_empty()
                            on:click=on_optimizer_csv>
                            {csv_label}
                        </button>
                        <button
                            type="button"
                            disabled=move || tradeoff_all.get().is_empty()
                            on:click=on_optimizer_json>
                            {json_label}
                        </button>
                        <button
                            type="button"
                            disabled=move || ldc.get().is_none() || area_result.get().is_none()
                            title=report_hint
                            on:click=on_report>
                            {report_label}
                        </button>
                    </div>
                }
            }}
        </fieldset>
    }
}

fn arrangement_name(a: Arrangement) -> &'static str {
    match a {
        Arrangement::SingleSide => "single_side",
        Arrangement::Opposite => "opposite",
        Arrangement::Staggered => "staggered",
    }
}

/// CSV serialization for the optimizer passing set.
fn optimizer_csv(candidates: &[OptimizationCandidate]) -> String {
    let mut out = String::new();
    out.push_str("spacing_m,mounting_height_m,arrangement,poles_per_km,avg_lux,min_lux,uniformity,flux_per_km\n");
    for c in candidates {
        out.push_str(&format!(
            "{spacing:.1},{height:.1},{arr},{poles:.2},{avg:.2},{min:.2},{u:.3},{flux:.0}\n",
            spacing = c.pole_spacing_m,
            height = c.mounting_height_m,
            arr = arrangement_name(c.arrangement),
            poles = c.poles_per_km,
            avg = c.design.avg_illuminance_lux,
            min = c.design.min_illuminance_lux,
            u = c.design.uniformity_overall,
            flux = c.flux_per_km,
        ));
    }
    out
}

/// JSON serialization. Hand-rolled to avoid pulling serde_json as a
/// runtime dep — the structure is simple and stable.
fn optimizer_json(candidates: &[OptimizationCandidate], frontier: &[usize]) -> String {
    let mut out = String::from("{\n  \"candidates\": [\n");
    for (i, c) in candidates.iter().enumerate() {
        let on_frontier = frontier.contains(&i);
        out.push_str(&format!(
            "    {{\"spacing_m\": {spacing:.1}, \"mounting_height_m\": {height:.1}, \"arrangement\": \"{arr}\", \"poles_per_km\": {poles:.2}, \"avg_lux\": {avg:.2}, \"min_lux\": {min:.2}, \"uniformity\": {u:.3}, \"flux_per_km\": {flux:.0}, \"on_frontier\": {front}}}",
            spacing = c.pole_spacing_m,
            height = c.mounting_height_m,
            arr = arrangement_name(c.arrangement),
            poles = c.poles_per_km,
            avg = c.design.avg_illuminance_lux,
            min = c.design.min_illuminance_lux,
            u = c.design.uniformity_overall,
            flux = c.flux_per_km,
            front = on_frontier,
        ));
        if i + 1 < candidates.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ]\n}\n");
    out
}

// ─────────────────────────────────────────────────────────────────────────
// Advisor panel — one-line recommendations on each failed criterion
// ─────────────────────────────────────────────────────────────────────────

#[component]
fn AdvisorPanel(heading: String, tips: Vec<AdvisorTip>) -> impl IntoView {
    view! {
        <section class="street-advisor" style="margin: 0.75rem 0; padding: 0.75rem 1rem; border-left: 3px solid #f59e0b; background: rgba(245, 158, 11, 0.08);">
            <h3 style="margin: 0 0 0.5rem; font-size: 1.05rem;">{heading}</h3>
            <ul style="margin: 0; padding-left: 1.25rem;">
                {tips.into_iter().map(|t| view! {
                    <li style="margin: 0.25rem 0; line-height: 1.4;">
                        <strong>{t.parameter}</strong>": "{t.message}
                    </li>
                }).collect_view()}
            </ul>
        </section>
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Compliance results table (unchanged)
// ─────────────────────────────────────────────────────────────────────────

#[component]
fn ResultsTable(results: Vec<ComplianceResult>, locale: Memo<Locale>) -> impl IntoView {
    view! {
        <div>
            {results.into_iter().map(|r| {
                let passed = r.passed();
                let header_color = if passed { "#2d7a2d" } else { "#a83232" };
                let loc = locale.get();
                let badge = if passed { loc.street.compliance.pass.clone() } else { loc.street.compliance.fail.clone() };
                let col_criterion = loc.street.compliance.criterion.clone();
                let col_required = loc.street.compliance.required.clone();
                let col_achieved = loc.street.compliance.achieved.clone();
                view! {
                    <div style="margin: 1rem 0; border: 1px solid #ddd; padding: 0.75rem;">
                        <h3 style=format!("color: {header_color}; margin: 0 0 0.5rem;")>
                            {format!("{} · {} · {}", r.region, r.standard, badge)}
                        </h3>
                        <table style="width: 100%; border-collapse: collapse;">
                            <thead>
                                <tr style="background: #f5f5f5;">
                                    <th style="text-align: left; padding: 0.25rem 0.5rem;">{col_criterion}</th>
                                    <th style="text-align: left; padding: 0.25rem 0.5rem;">{col_required}</th>
                                    <th style="text-align: left; padding: 0.25rem 0.5rem;">{col_achieved}</th>
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
