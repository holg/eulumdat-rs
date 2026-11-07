//! Spectral Power Distribution (SPD) diagram component
//!
//! Displays spectral data from ATLA emitters using the eulumdat::atla::spectral module.
//! Auto-detects horticultural lighting and shows PAR zones.
//! Can synthesize spectrum from CCT/CRI when spectral data is not available.
//! Calculates and displays TM-30 metrics when real spectral data is available.
//! Shows IR/UV content and thermal/hazard warnings.

use crate::i18n::use_locale;
use eulumdat::atla::spectral::{
    synthesize_spectrum, SpectralDiagram, SpectralMetrics, SpectralTheme,
};
use eulumdat::atla::tm30::Tm30Theme;
use eulumdat::atla::{ColourQuality, ColourQualityLabels};
use eulumdat::atla::{LuminaireOpticalData, SpectralDistribution, SpectralUnits};
use leptos::prelude::*;

/// Check if this appears to be a horticultural/grow light based on metadata
fn is_horticultural(doc: &LuminaireOpticalData) -> bool {
    let desc = doc
        .header
        .description
        .as_deref()
        .unwrap_or("")
        .to_lowercase();
    let lum_type = doc
        .header
        .luminaire_type
        .as_deref()
        .unwrap_or("")
        .to_lowercase();
    let comments = doc.header.comments.as_deref().unwrap_or("").to_lowercase();

    let keywords = [
        "grow",
        "horticultural",
        "par",
        "plant",
        "greenhouse",
        "cultivation",
    ];

    keywords
        .iter()
        .any(|kw| desc.contains(kw) || lum_type.contains(kw) || comments.contains(kw))
}

/// Spectral data source detected
#[derive(Clone, Copy, PartialEq)]
enum SpectralSource {
    /// Direct spectral data from emitter
    Direct,
    /// Synthesized from CCT/CRI
    Synthesized,
    /// No data available - show sample
    Sample,
}

/// Sub-tabs within the Spectral tab
#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum SpectralSubTab {
    #[default]
    Spd,
    Metrics,
    Tm30Cvg,
    Tm30Hue,
}

/// Request for the spectral view to show a sub-tab (`spd`, `metrics`,
/// `colour-quality`, `tm30-hue`), e.g. from a dashboard tile. App-wide
/// context; the view takes the request and clears it.
#[derive(Clone, Copy)]
pub struct SpectralViewRequest(pub RwSignal<Option<String>>);

fn parse_sub_tab(view: &str) -> Option<SpectralSubTab> {
    match view {
        "spd" => Some(SpectralSubTab::Spd),
        "metrics" => Some(SpectralSubTab::Metrics),
        "colour-quality" | "color-quality" | "tm30" | "cvg" => Some(SpectralSubTab::Tm30Cvg),
        "tm30-hue" | "hue" => Some(SpectralSubTab::Tm30Hue),
        _ => None,
    }
}

/// `spectralview=spd|metrics|colour-quality|tm30-hue` from the start-up
/// link, applied once per page load (later visits open the SPD view).
fn link_sub_tab() -> Option<SpectralSubTab> {
    use std::sync::atomic::{AtomicBool, Ordering};
    static APPLIED: AtomicBool = AtomicBool::new(false);
    if APPLIED.swap(true, Ordering::Relaxed) {
        return None;
    }
    let pairs = use_context::<super::app::RtLinkParams>()?.0.get_value();
    let view = pairs.iter().find(|(k, _)| k == "spectralview")?.1.clone();
    parse_sub_tab(&view)
}

/// Detect what spectral source we have
fn detect_spectral_source(doc: &LuminaireOpticalData) -> SpectralSource {
    if doc
        .emitters
        .iter()
        .any(|e| e.spectral_distribution.is_some())
    {
        return SpectralSource::Direct;
    }

    for emitter in &doc.emitters {
        if emitter.cct.is_some() {
            return SpectralSource::Synthesized;
        }
    }

    SpectralSource::Sample
}

/// Get CCT and CRI from document
fn get_cct_cri(doc: &LuminaireOpticalData) -> Option<(f64, Option<f64>)> {
    for emitter in &doc.emitters {
        if let Some(cct) = emitter.cct {
            let cri = emitter.color_rendering.as_ref().and_then(|cr| cr.ra);
            return Some((cct, cri));
        }
    }
    None
}

/// Spectral diagram component that displays SPD from ATLA document
#[component]
pub fn SpectralDiagramView(
    atla_doc: ReadSignal<LuminaireOpticalData>,
    dark: Memo<bool>,
) -> impl IntoView {
    let locale = use_locale();
    let (active_sub_tab, set_active_sub_tab) = signal(link_sub_tab().unwrap_or_default());
    // Sub-tab requested from elsewhere (dashboard tile).
    if let Some(req) = use_context::<SpectralViewRequest>() {
        Effect::new(move |_| {
            if let Some(view) = req.0.get() {
                if let Some(tab) = parse_sub_tab(&view) {
                    set_active_sub_tab.set(tab);
                }
                req.0.set(None);
            }
        });
    }

    let spectral_source = move || detect_spectral_source(&atla_doc.get());
    let has_tm30 = move || spectral_source() == SpectralSource::Direct;

    // SPD diagram SVG
    let spd_svg = move || {
        let doc = atla_doc.get();
        let is_hort = is_horticultural(&doc);

        let theme = match (dark.get(), is_hort) {
            (true, true) => SpectralTheme::dark_par(),
            (true, false) => SpectralTheme::dark(),
            (false, true) => SpectralTheme::light_par(),
            (false, false) => SpectralTheme::light(),
        };

        match detect_spectral_source(&doc) {
            SpectralSource::Direct => {
                if let Some(spd) = doc
                    .emitters
                    .iter()
                    .filter_map(|e| e.spectral_distribution.as_ref())
                    .next()
                {
                    let diagram = SpectralDiagram::from_spectral(spd);
                    diagram.to_svg(700.0, 400.0, &theme)
                } else {
                    String::new()
                }
            }
            SpectralSource::Synthesized => {
                if let Some((cct, cri)) = get_cct_cri(&doc) {
                    let spd = synthesize_spectrum(cct, cri);
                    let diagram = SpectralDiagram::from_spectral(&spd);
                    diagram.to_svg(700.0, 400.0, &theme)
                } else {
                    String::new()
                }
            }
            SpectralSource::Sample => {
                let sample_spd = create_sample_led_spectrum();
                let diagram = SpectralDiagram::from_spectral(&sample_spd);
                diagram.to_svg(700.0, 400.0, &theme)
            }
        }
    };

    // TM-30 + CRI (only for direct spectral data)
    let colour_quality = move || {
        let doc = atla_doc.get();
        doc.emitters
            .iter()
            .filter_map(|e| e.spectral_distribution.as_ref())
            .next()
            .and_then(ColourQuality::from_spd)
    };
    let tm30_result = move || colour_quality().map(|q| q.tm30);

    // Colour-quality card (vector graphic, Duv bar, key values)
    let cq_card_svg = move || {
        let theme = if dark.get() {
            Tm30Theme::dark()
        } else {
            Tm30Theme::light()
        };
        let doc = atla_doc.get();
        let subtitle = doc
            .header
            .description
            .clone()
            .or_else(|| doc.header.catalog_number.clone())
            .unwrap_or_default();
        let labels = cq_labels(&locale.get().ui.colour_quality, subtitle);
        colour_quality().map(|q| q.card_svg(&labels, &theme))
    };

    // TM-30 Rf Hue Bar Chart SVG
    let tm30_hue_svg = move || {
        let theme = if dark.get() {
            Tm30Theme::dark()
        } else {
            Tm30Theme::light()
        };
        tm30_result().map(|tm30| tm30.rf_hue_svg(700.0, 300.0, &theme))
    };

    // Spectral metrics (IR/UV/PAR distribution)
    let spectral_metrics = move || -> Option<SpectralMetrics> {
        let doc = atla_doc.get();
        match detect_spectral_source(&doc) {
            SpectralSource::Direct => doc
                .emitters
                .iter()
                .filter_map(|e| e.spectral_distribution.as_ref())
                .next()
                .map(SpectralMetrics::from_spd),
            SpectralSource::Synthesized => get_cct_cri(&doc).map(|(cct, cri)| {
                let spd = synthesize_spectrum(cct, cri);
                SpectralMetrics::from_spd(&spd)
            }),
            SpectralSource::Sample => {
                let sample = create_sample_led_spectrum();
                Some(SpectralMetrics::from_spd(&sample))
            }
        }
    };

    let is_hort_display = move || is_horticultural(&atla_doc.get());
    let has_ir = move || spectral_metrics().map(|m| m.has_ir).unwrap_or(false);
    let has_uv = move || spectral_metrics().map(|m| m.has_uv).unwrap_or(false);
    let thermal_warning = move || {
        spectral_metrics()
            .map(|m| m.thermal_warning)
            .unwrap_or(false)
    };
    let uv_warning = move || spectral_metrics().map(|m| m.uv_warning).unwrap_or(false);

    view! {
        <div class="spectral-tab-container">
            // Sub-tabs navigation
            <div class="spectral-sub-tabs">
                <button
                    class=move || if active_sub_tab.get() == SpectralSubTab::Spd { "sub-tab active" } else { "sub-tab" }
                    on:click=move |_| set_active_sub_tab.set(SpectralSubTab::Spd)
                >
                    {move || locale.get().ui.subtabs.spd.clone()}
                </button>
                <button
                    class=move || if active_sub_tab.get() == SpectralSubTab::Metrics { "sub-tab active" } else { "sub-tab" }
                    on:click=move |_| set_active_sub_tab.set(SpectralSubTab::Metrics)
                >
                    {move || locale.get().ui.subtabs.metrics.clone()}
                </button>
                {move || if has_tm30() {
                    view! {
                        <button
                            class=move || if active_sub_tab.get() == SpectralSubTab::Tm30Cvg { "sub-tab active" } else { "sub-tab" }
                            on:click=move |_| set_active_sub_tab.set(SpectralSubTab::Tm30Cvg)
                        >
                            {move || locale.get().ui.colour_quality.subtab.clone()}
                        </button>
                        <button
                            class=move || if active_sub_tab.get() == SpectralSubTab::Tm30Hue { "sub-tab active" } else { "sub-tab" }
                            on:click=move |_| set_active_sub_tab.set(SpectralSubTab::Tm30Hue)
                        >
                            {move || locale.get().ui.subtabs.tm30_hue.clone()}
                        </button>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }}

                // Badges on the right
                <div class="sub-tab-badges">
                    // IR warning badge
                    {move || {
                        let l = locale.get();
                        if thermal_warning() {
                            view! {
                                <span class="badge badge-danger" title=l.ui.spectral_badges.ir_high_title.clone()>{l.ui.spectral_badges.ir.clone()}</span>
                            }.into_any()
                        } else if has_ir() {
                            view! {
                                <span class="badge badge-ir" title=l.ui.spectral_badges.ir_title.clone()>{l.ui.spectral_badges.ir.clone()}</span>
                            }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }
                    }}
                    // UV warning badge
                    {move || {
                        let l = locale.get();
                        if uv_warning() {
                            view! {
                                <span class="badge badge-danger" title=l.ui.spectral_badges.uv_high_title.clone()>{l.ui.spectral_badges.uv.clone()}</span>
                            }.into_any()
                        } else if has_uv() {
                            view! {
                                <span class="badge badge-uv" title=l.ui.spectral_badges.uv_title.clone()>{l.ui.spectral_badges.uv.clone()}</span>
                            }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }
                    }}
                    // PAR badge
                    {move || if is_hort_display() {
                        view! {
                            <span class="badge badge-success">"PAR"</span>
                        }.into_any()
                    } else {
                        view! { <span></span> }.into_any()
                    }}
                    {move || match spectral_source() {
                        SpectralSource::Direct => view! {
                            <span class="badge badge-info">{move || locale.get().ui.spectral.direct_spd.clone()}</span>
                        }.into_any(),
                        SpectralSource::Synthesized => {
                            if let Some((cct, cri)) = get_cct_cri(&atla_doc.get()) {
                                view! {
                                    <span class="badge badge-warning">
                                        {format!("{}K", cct as i32)}
                                        {cri.map(|c| format!(" CRI{}", c as i32)).unwrap_or_default()}
                                    </span>
                                }.into_any()
                            } else {
                                view! { <span></span> }.into_any()
                            }
                        },
                        SpectralSource::Sample => view! {
                            <span class="badge badge-secondary">{move || locale.get().ui.spectral.sample.clone()}</span>
                        }.into_any(),
                    }}
                </div>
            </div>

            // Sub-tab content
            <div class="spectral-sub-content">
                {move || match active_sub_tab.get() {
                    SpectralSubTab::Spd => view! {
                        <div class="spd-diagram-container">
                            <div class="diagram-svg" inner_html=spd_svg />

                            // TM-30 metrics summary (if available)
                            {move || tm30_result().map(|tm30| {
                                let l = locale.get();
                                view! {
                                    <div class="tm30-summary">
                                        <div class="tm30-metric-inline">
                                            <span class="label">{l.ui.spectral.hue_table.rf.clone()}</span>
                                            <span class="value" style=format!("color: {}", rf_color(tm30.rf))>
                                                {format!("{:.0}", tm30.rf)}
                                            </span>
                                        </div>
                                        <div class="tm30-metric-inline">
                                            <span class="label">{l.ui.spectral_badges.rg.clone()}</span>
                                            <span class="value" style=format!("color: {}", rg_color(tm30.rg))>
                                                {format!("{:.0}", tm30.rg)}
                                            </span>
                                        </div>
                                        <div class="tm30-metric-inline">
                                            <span class="label">{l.luminaire.photometric.cct.clone()}</span>
                                            <span class="value">{format!("{:.0}K", tm30.cct)}</span>
                                        </div>
                                        <div class="tm30-metric-inline">
                                            <span class="label">{l.ui.spectral_badges.duv.clone()}</span>
                                            <span class="value">{format!("{:+.4}", tm30.duv)}</span>
                                        </div>
                                        {colour_quality().and_then(|q| q.cri).map(|c| view! {
                                            <div class="tm30-metric-inline">
                                                <span class="label">{l.ui.colour_quality.ra.clone()}</span>
                                                <span class="value">{format!("{:.0}", c.ra)}</span>
                                            </div>
                                            <div class="tm30-metric-inline">
                                                <span class="label">{l.ui.colour_quality.r9.clone()}</span>
                                                <span class="value">{format!("{:.0}", c.r9())}</span>
                                            </div>
                                        })}
                                    </div>
                                }
                            })}

                            // Hint for sample spectra
                            {move || if matches!(spectral_source(), SpectralSource::Sample) {
                                view! {
                                    <p class="text-muted hint-text">
                                        {move || locale.get().ui.spectral.load_hint.clone()}
                                    </p>
                                }.into_any()
                            } else {
                                view! { <span></span> }.into_any()
                            }}
                        </div>
                    }.into_any(),

                    SpectralSubTab::Metrics => view! {
                        <div class="spectral-metrics-container">
                            {move || spectral_metrics().map(|m| view! {
                                <div class="metrics-grid">
                                    // Wavelength range
                                    <div class="metric-card">
                                        <h4>{move || locale.get().ui.spectral.wavelength_range.clone()}</h4>
                                        <div class="metric-value">{format!("{:.0} - {:.0} nm", m.wavelength_min, m.wavelength_max)}</div>
                                        <div class="metric-detail">{move || format!("{}: {:.0} nm", locale.get().ui.spectral.peak.clone(), m.peak_wavelength)}</div>
                                    </div>

                                    // Energy distribution
                                    <div class="metric-card">
                                        <h4>{move || locale.get().ui.spectral.energy_distribution.clone()}</h4>
                                        <div class="energy-bars">
                                            {if m.has_uv {
                                                view! {
                                                    <div class="energy-row">
                                                        <span class="energy-label">"UV-A"</span>
                                                        <div class="energy-bar uv-bar" style=format!("width: {}%", m.uv_a_percent.min(100.0))></div>
                                                        <span class="energy-value">{format!("{:.1}%", m.uv_a_percent)}</span>
                                                    </div>
                                                }.into_any()
                                            } else {
                                                view! { <span></span> }.into_any()
                                            }}
                                            <div class="energy-row">
                                                <span class="energy-label">{move || locale.get().spectral.region.visible.clone()}</span>
                                                <div class="energy-bar visible-bar" style=format!("width: {}%", m.visible_percent.min(100.0))></div>
                                                <span class="energy-value">{format!("{:.1}%", m.visible_percent)}</span>
                                            </div>
                                            {if m.has_ir {
                                                view! {
                                                    <div class="energy-row">
                                                        <span class="energy-label">{move || locale.get().spectral.region.near_ir.clone()}</span>
                                                        <div class="energy-bar ir-bar" style=format!("width: {}%", m.nir_percent.min(100.0))></div>
                                                        <span class="energy-value">{format!("{:.1}%", m.nir_percent)}</span>
                                                    </div>
                                                }.into_any()
                                            } else {
                                                view! { <span></span> }.into_any()
                                            }}
                                        </div>
                                    </div>

                                    // PAR distribution (for all light sources)
                                    <div class="metric-card">
                                        <h4>{move || locale.get().ui.spectral.par_distribution.clone()}</h4>
                                        <div class="par-bars">
                                            <div class="par-row">
                                                <span class="par-label blue">{move || locale.get().spectral.region.blue.clone()}</span>
                                                <div class="par-bar blue-bar" style=format!("width: {}%", m.blue_par_percent.min(100.0))></div>
                                                <span class="par-value">{format!("{:.1}%", m.blue_par_percent)}</span>
                                            </div>
                                            <div class="par-row">
                                                <span class="par-label green">{move || locale.get().spectral.region.green.clone()}</span>
                                                <div class="par-bar green-bar" style=format!("width: {}%", m.green_par_percent.min(100.0))></div>
                                                <span class="par-value">{format!("{:.1}%", m.green_par_percent)}</span>
                                            </div>
                                            <div class="par-row">
                                                <span class="par-label red">{move || locale.get().spectral.region.red.clone()}</span>
                                                <div class="par-bar red-bar" style=format!("width: {}%", m.red_par_percent.min(100.0))></div>
                                                <span class="par-value">{format!("{:.1}%", m.red_par_percent)}</span>
                                            </div>
                                        </div>
                                        <div class="metric-detail">{move || format!("{}: {:.1}%", locale.get().ui.spectral.par_total.clone(), m.par_percent)}</div>
                                    </div>

                                    // Horticultural metrics
                                    <div class="metric-card">
                                        <h4>{move || locale.get().ui.spectral.hort_metrics.clone()}</h4>
                                        <div class="hort-metrics">
                                            <div class="hort-row">
                                                <span class="hort-label">{move || locale.get().ui.spectral.far_red.clone()}</span>
                                                <span class="hort-value">{format!("{:.1}%", m.far_red_percent)}</span>
                                            </div>
                                            {m.r_fr_ratio.map(|ratio| view! {
                                                <div class="hort-row">
                                                    <span class="hort-label">"R:FR"</span>
                                                    <span class="hort-value" style=r_fr_style(ratio)>{format!("{:.2}", ratio)}</span>
                                                </div>
                                            })}
                                        </div>
                                        <p class="metric-hint">{move || locale.get().ui.spectral.r_fr_hint.clone()}</p>
                                    </div>

                                    // Warnings
                                    {if m.thermal_warning || m.uv_warning {
                                        view! {
                                            <div class="metric-card warning-card">
                                                <h4>{move || locale.get().ui.spectral.warnings.clone()}</h4>
                                                {if m.thermal_warning {
                                                    view! {
                                                        <div class="warning-item thermal">
                                                            <span class="warning-icon">"🔥"</span>
                                                            <span>{move || locale.get().ui.spectral.thermal_warning.clone()}</span>
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    view! { <span></span> }.into_any()
                                                }}
                                                {if m.uv_warning {
                                                    view! {
                                                        <div class="warning-item uv">
                                                            <span class="warning-icon">"⚠️"</span>
                                                            <span>{move || locale.get().ui.spectral.uv_warning.clone()}</span>
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    view! { <span></span> }.into_any()
                                                }}
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! { <span></span> }.into_any()
                                    }}
                                </div>
                            })}
                        </div>
                    }.into_any(),

                    SpectralSubTab::Tm30Cvg => view! {
                        <div class="tm30-cvg-container colour-quality">
                            {move || cq_card_svg().map(|svg| view! {
                                <div class="diagram-svg cq-card" inner_html=svg />
                            })}
                            {move || colour_quality().and_then(|q| q.cri).map(|c| {
                                let l = locale.get();
                                view! {
                                    <div class="cq-ri">
                                        <h4>{l.ui.colour_quality.ri_title.clone()}</h4>
                                        <table class="tm30-hue-table">
                                            <thead><tr>
                                                {(1..=15).map(|i| view! { <th>{format!("R{i}")}</th> }).collect_view()}
                                            </tr></thead>
                                            <tbody><tr>
                                                {c.ri.iter().map(|r| view! { <td>{format!("{r:.0}")}</td> }).collect_view()}
                                            </tr></tbody>
                                        </table>
                                    </div>
                                }
                            })}
                            <button class="btn btn-secondary cq-download" on:click=move |_| {
                                if let Some(svg) = cq_card_svg() {
                                    super::file_handler::download_svg("colour_quality.svg", &svg);
                                }
                            }>
                                {move || locale.get().ui.colour_quality.download.clone()}
                            </button>
                        </div>
                    }.into_any(),

                    SpectralSubTab::Tm30Hue => {
                        let locale_for_table = locale;
                        let locale_for_legend = locale;
                        view! {
                            <div class="tm30-hue-container">
                                {move || tm30_hue_svg().map(|svg| view! {
                                    <div class="tm30-hue-diagram">
                                        <div class="diagram-svg" inner_html=svg />
                                    </div>
                                })}
                                {move || tm30_result().map(|tm30| {
                                    let l = locale_for_table.get();
                                    view! {
                                        <div class="tm30-hue-details">
                                            <table class="tm30-hue-table">
                                                <thead>
                                                    <tr>
                                                        <th>{l.ui.spectral.hue_table.hue.clone()}</th>
                                                        <th>{l.ui.spectral.hue_table.rf.clone()}</th>
                                                        <th>{l.ui.spectral.hue_table.rcs.clone()}</th>
                                                        <th>{l.ui.spectral.hue_table.rhs.clone()}</th>
                                                    </tr>
                                                </thead>
                                                <tbody>
                                                    {(0..16).map(|i| {
                                                        let rf = tm30.rf_hue[i];
                                                        let rcs = tm30.rcs_hue[i];
                                                        let rhs = tm30.rhs_hue[i];
                                                        view! {
                                                            <tr>
                                                                <td>{format!("h{}", i + 1)}</td>
                                                                <td style=format!("color: {}", rf_color(rf))>{format!("{:.0}", rf)}</td>
                                                                <td>{format!("{:+.1}%", rcs)}</td>
                                                                <td>{format!("{:+.3}", rhs)}</td>
                                                            </tr>
                                                        }
                                                    }).collect::<Vec<_>>()}
                                                </tbody>
                                            </table>
                                        </div>
                                    }
                                })}
                                <div class="tm30-legend">
                                    <p><strong>{move || locale_for_legend.get().ui.spectral.hue_table.rf.clone()}</strong>": "{move || locale_for_legend.get().ui.spectral.hue_legend.rf.clone()}</p>
                                    <p><strong>{move || locale_for_legend.get().ui.spectral.hue_table.rcs.clone()}</strong>": "{move || locale_for_legend.get().ui.spectral.hue_legend.rcs.clone()}</p>
                                    <p><strong>{move || locale_for_legend.get().ui.spectral.hue_table.rhs.clone()}</strong>": "{move || locale_for_legend.get().ui.spectral.hue_legend.rhs.clone()}</p>
                                </div>
                            </div>
                        }.into_any()
                    },
                }}
            </div>
        </div>
    }
}

/// Card texts from the locale (`ui.colour_quality`).
fn cq_labels(l: &eulumdat_i18n::UiColourQuality, subtitle: String) -> ColourQualityLabels {
    let mut labels = ColourQualityLabels {
        title: l.title.clone(),
        subtitle,
        cvg_title: l.cvg_title.clone(),
        cvg_text: l.cvg_text.clone(),
        cvg_note: l.cvg_note.clone(),
        duv_title: l.duv_title.clone(),
        magenta: l.magenta.clone(),
        green: l.green.clone(),
        black_body_line: l.black_body_line.clone(),
        measured: l.measured.clone(),
        cct: l.cct.clone(),
        duv: l.duv.clone(),
        rf_rg: l.rf_rg.clone(),
        ra: l.ra.clone(),
        r9: l.r9.clone(),
        re: l.re.clone(),
        cri_limit: l.cri_limit.clone(),
        method: l.method.clone(),
        ..Default::default()
    };
    if l.hues.len() == labels.hues.len() {
        for (dst, src) in labels.hues.iter_mut().zip(&l.hues) {
            dst.clone_from(src);
        }
    }
    labels
}

/// Get color for Rf value (green = good, red = poor)
fn rf_color(rf: f64) -> &'static str {
    if rf >= 90.0 {
        "#22c55e"
    } else if rf >= 80.0 {
        "#84cc16"
    } else if rf >= 70.0 {
        "#eab308"
    } else if rf >= 60.0 {
        "#f97316"
    } else {
        "#ef4444"
    }
}

/// Get color for Rg value (100 = neutral, deviation shows color)
fn rg_color(rg: f64) -> &'static str {
    if (rg - 100.0).abs() <= 5.0 {
        "#22c55e"
    } else if (rg - 100.0).abs() <= 10.0 {
        "#84cc16"
    } else if (rg - 100.0).abs() <= 15.0 {
        "#eab308"
    } else {
        "#f97316"
    }
}

/// Get style for R:FR ratio value
/// Sunlight ~1.1-1.2, shade ~0.1-0.7, HPS ~1.5-2.0
fn r_fr_style(ratio: f64) -> String {
    let color = if (0.8..=1.5).contains(&ratio) {
        "#22c55e" // Good for most plants
    } else if (0.5..=2.0).contains(&ratio) {
        "#eab308" // Acceptable
    } else {
        "#f97316" // Extreme
    };
    format!("color: {}", color)
}

/// Create a sample warm white LED spectrum for demonstration
fn create_sample_led_spectrum() -> SpectralDistribution {
    let wavelengths: Vec<f64> = (380..=780).step_by(5).map(|w| w as f64).collect();
    let values: Vec<f64> = wavelengths
        .iter()
        .map(|&wl| {
            let blue_peak = (-((wl - 450.0) / 20.0).powi(2)).exp() * 0.7;
            let phosphor = if wl > 480.0 {
                (-((wl - 580.0) / 80.0).powi(2)).exp() * 1.0
            } else {
                0.0
            };
            blue_peak + phosphor
        })
        .collect();

    SpectralDistribution {
        wavelengths,
        values,
        units: SpectralUnits::Relative,
        start_wavelength: None,
        wavelength_interval: None,
    }
}
