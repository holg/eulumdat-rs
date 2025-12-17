//! Spectral Power Distribution (SPD) diagram component
//!
//! Displays spectral data from ATLA emitters using the atla::spectral module.
//! Auto-detects horticultural lighting and shows PAR zones.
//! Can synthesize spectrum from CCT/CRI when spectral data is not available.
//! Calculates and displays TM-30 metrics when real spectral data is available.
//! Shows IR/UV content and thermal/hazard warnings.

use atla::spectral::{synthesize_spectrum, SpectralDiagram, SpectralMetrics, SpectralTheme};
use atla::tm30::{calculate_tm30, Tm30Theme};
use atla::{LuminaireOpticalData, SpectralDistribution, SpectralUnits};
use leptos::prelude::*;

/// Check if this appears to be a horticultural/grow light based on metadata
fn is_horticultural(doc: &LuminaireOpticalData) -> bool {
    let desc = doc.header.description.as_deref().unwrap_or("").to_lowercase();
    let lum_type = doc.header.luminaire_type.as_deref().unwrap_or("").to_lowercase();
    let comments = doc.header.comments.as_deref().unwrap_or("").to_lowercase();

    let keywords = ["grow", "horticultural", "par", "plant", "greenhouse", "cultivation"];

    keywords.iter().any(|kw| {
        desc.contains(kw) || lum_type.contains(kw) || comments.contains(kw)
    })
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

/// Detect what spectral source we have
fn detect_spectral_source(doc: &LuminaireOpticalData) -> SpectralSource {
    if doc.emitters.iter().any(|e| e.spectral_distribution.is_some()) {
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
    let (active_sub_tab, set_active_sub_tab) = signal(SpectralSubTab::Spd);

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
                if let Some(spd) = doc.emitters.iter().filter_map(|e| e.spectral_distribution.as_ref()).next() {
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

    // TM-30 result (only for direct spectral data)
    let tm30_result = move || {
        let doc = atla_doc.get();
        if let Some(spd) = doc.emitters.iter().filter_map(|e| e.spectral_distribution.as_ref()).next() {
            calculate_tm30(spd)
        } else {
            None
        }
    };

    // TM-30 Color Vector Graphic SVG
    let tm30_cvg_svg = move || {
        let theme = if dark.get() { Tm30Theme::dark() } else { Tm30Theme::light() };
        tm30_result().map(|tm30| tm30.to_svg(500.0, 500.0, &theme))
    };

    // TM-30 Rf Hue Bar Chart SVG
    let tm30_hue_svg = move || {
        let theme = if dark.get() { Tm30Theme::dark() } else { Tm30Theme::light() };
        tm30_result().map(|tm30| tm30.rf_hue_svg(700.0, 300.0, &theme))
    };

    // Spectral metrics (IR/UV/PAR distribution)
    let spectral_metrics = move || -> Option<SpectralMetrics> {
        let doc = atla_doc.get();
        match detect_spectral_source(&doc) {
            SpectralSource::Direct => {
                doc.emitters.iter()
                    .filter_map(|e| e.spectral_distribution.as_ref())
                    .next()
                    .map(SpectralMetrics::from_spd)
            }
            SpectralSource::Synthesized => {
                get_cct_cri(&doc).map(|(cct, cri)| {
                    let spd = synthesize_spectrum(cct, cri);
                    SpectralMetrics::from_spd(&spd)
                })
            }
            SpectralSource::Sample => {
                let sample = create_sample_led_spectrum();
                Some(SpectralMetrics::from_spd(&sample))
            }
        }
    };

    let is_hort_display = move || is_horticultural(&atla_doc.get());
    let has_ir = move || spectral_metrics().map(|m| m.has_ir).unwrap_or(false);
    let has_uv = move || spectral_metrics().map(|m| m.has_uv).unwrap_or(false);
    let thermal_warning = move || spectral_metrics().map(|m| m.thermal_warning).unwrap_or(false);
    let uv_warning = move || spectral_metrics().map(|m| m.uv_warning).unwrap_or(false);

    view! {
        <div class="spectral-tab-container">
            // Sub-tabs navigation
            <div class="spectral-sub-tabs">
                <button
                    class=move || if active_sub_tab.get() == SpectralSubTab::Spd { "sub-tab active" } else { "sub-tab" }
                    on:click=move |_| set_active_sub_tab.set(SpectralSubTab::Spd)
                >
                    "SPD"
                </button>
                <button
                    class=move || if active_sub_tab.get() == SpectralSubTab::Metrics { "sub-tab active" } else { "sub-tab" }
                    on:click=move |_| set_active_sub_tab.set(SpectralSubTab::Metrics)
                >
                    "Metrics"
                </button>
                {move || if has_tm30() {
                    view! {
                        <button
                            class=move || if active_sub_tab.get() == SpectralSubTab::Tm30Cvg { "sub-tab active" } else { "sub-tab" }
                            on:click=move |_| set_active_sub_tab.set(SpectralSubTab::Tm30Cvg)
                        >
                            "TM-30 CVG"
                        </button>
                        <button
                            class=move || if active_sub_tab.get() == SpectralSubTab::Tm30Hue { "sub-tab active" } else { "sub-tab" }
                            on:click=move |_| set_active_sub_tab.set(SpectralSubTab::Tm30Hue)
                        >
                            "TM-30 Hue"
                        </button>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }}

                // Badges on the right
                <div class="sub-tab-badges">
                    // IR warning badge
                    {move || if thermal_warning() {
                        view! {
                            <span class="badge badge-danger" title="High infrared content - thermal hazard">"IR"</span>
                        }.into_any()
                    } else if has_ir() {
                        view! {
                            <span class="badge badge-ir" title="Contains infrared data">"IR"</span>
                        }.into_any()
                    } else {
                        view! { <span></span> }.into_any()
                    }}
                    // UV warning badge
                    {move || if uv_warning() {
                        view! {
                            <span class="badge badge-danger" title="High UV content - exposure risk">"UV"</span>
                        }.into_any()
                    } else if has_uv() {
                        view! {
                            <span class="badge badge-uv" title="Contains UV data">"UV"</span>
                        }.into_any()
                    } else {
                        view! { <span></span> }.into_any()
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
                            <span class="badge badge-info">"Direct SPD"</span>
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
                            <span class="badge badge-secondary">"Sample"</span>
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
                            {move || tm30_result().map(|tm30| view! {
                                <div class="tm30-summary">
                                    <div class="tm30-metric-inline">
                                        <span class="label">"Rf"</span>
                                        <span class="value" style=format!("color: {}", rf_color(tm30.rf))>
                                            {format!("{:.0}", tm30.rf)}
                                        </span>
                                    </div>
                                    <div class="tm30-metric-inline">
                                        <span class="label">"Rg"</span>
                                        <span class="value" style=format!("color: {}", rg_color(tm30.rg))>
                                            {format!("{:.0}", tm30.rg)}
                                        </span>
                                    </div>
                                    <div class="tm30-metric-inline">
                                        <span class="label">"CCT"</span>
                                        <span class="value">{format!("{:.0}K", tm30.cct)}</span>
                                    </div>
                                    <div class="tm30-metric-inline">
                                        <span class="label">"Duv"</span>
                                        <span class="value">{format!("{:.4}", tm30.duv)}</span>
                                    </div>
                                </div>
                            })}

                            // Hint for sample spectra
                            {move || if matches!(spectral_source(), SpectralSource::Sample) {
                                view! {
                                    <p class="text-muted hint-text">
                                        "Load an ATLA template with spectral data for TM-30 analysis"
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
                                        <h4>"Wavelength Range"</h4>
                                        <div class="metric-value">{format!("{:.0} - {:.0} nm", m.wavelength_min, m.wavelength_max)}</div>
                                        <div class="metric-detail">{format!("Peak: {:.0} nm", m.peak_wavelength)}</div>
                                    </div>

                                    // Energy distribution
                                    <div class="metric-card">
                                        <h4>"Energy Distribution"</h4>
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
                                                <span class="energy-label">"Visible"</span>
                                                <div class="energy-bar visible-bar" style=format!("width: {}%", m.visible_percent.min(100.0))></div>
                                                <span class="energy-value">{format!("{:.1}%", m.visible_percent)}</span>
                                            </div>
                                            {if m.has_ir {
                                                view! {
                                                    <div class="energy-row">
                                                        <span class="energy-label">"Near-IR"</span>
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
                                        <h4>"PAR Distribution (400-700nm)"</h4>
                                        <div class="par-bars">
                                            <div class="par-row">
                                                <span class="par-label blue">"Blue"</span>
                                                <div class="par-bar blue-bar" style=format!("width: {}%", m.blue_par_percent.min(100.0))></div>
                                                <span class="par-value">{format!("{:.1}%", m.blue_par_percent)}</span>
                                            </div>
                                            <div class="par-row">
                                                <span class="par-label green">"Green"</span>
                                                <div class="par-bar green-bar" style=format!("width: {}%", m.green_par_percent.min(100.0))></div>
                                                <span class="par-value">{format!("{:.1}%", m.green_par_percent)}</span>
                                            </div>
                                            <div class="par-row">
                                                <span class="par-label red">"Red"</span>
                                                <div class="par-bar red-bar" style=format!("width: {}%", m.red_par_percent.min(100.0))></div>
                                                <span class="par-value">{format!("{:.1}%", m.red_par_percent)}</span>
                                            </div>
                                        </div>
                                        <div class="metric-detail">{format!("PAR total: {:.1}%", m.par_percent)}</div>
                                    </div>

                                    // Horticultural metrics
                                    <div class="metric-card">
                                        <h4>"Horticultural Metrics"</h4>
                                        <div class="hort-metrics">
                                            <div class="hort-row">
                                                <span class="hort-label">"Far-Red (700-780nm)"</span>
                                                <span class="hort-value">{format!("{:.1}%", m.far_red_percent)}</span>
                                            </div>
                                            {m.r_fr_ratio.map(|ratio| view! {
                                                <div class="hort-row">
                                                    <span class="hort-label">"R:FR Ratio"</span>
                                                    <span class="hort-value" style=r_fr_style(ratio)>{format!("{:.2}", ratio)}</span>
                                                </div>
                                            })}
                                        </div>
                                        <p class="metric-hint">"R:FR ratio affects plant morphology (>1.0 = compact growth)"</p>
                                    </div>

                                    // Warnings
                                    {if m.thermal_warning || m.uv_warning {
                                        view! {
                                            <div class="metric-card warning-card">
                                                <h4>"Warnings"</h4>
                                                {if m.thermal_warning {
                                                    view! {
                                                        <div class="warning-item thermal">
                                                            <span class="warning-icon">"🔥"</span>
                                                            <span>"High infrared content ({:.1}%) - significant thermal output"</span>
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    view! { <span></span> }.into_any()
                                                }}
                                                {if m.uv_warning {
                                                    view! {
                                                        <div class="warning-item uv">
                                                            <span class="warning-icon">"⚠️"</span>
                                                            <span>"Elevated UV-A content ({:.1}%) - potential material degradation"</span>
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
                        <div class="tm30-cvg-container">
                            {move || tm30_cvg_svg().map(|svg| view! {
                                <div class="tm30-cvg-diagram">
                                    <div class="diagram-svg" inner_html=svg />
                                </div>
                            })}
                            <div class="tm30-cvg-legend">
                                <p class="text-muted">"Color Vector Graphic shows hue and chroma shifts for 16 hue bins."</p>
                                <p class="text-muted">"Dashed circle = reference illuminant. Solid shape = test source."</p>
                            </div>
                        </div>
                    }.into_any(),

                    SpectralSubTab::Tm30Hue => view! {
                        <div class="tm30-hue-container">
                            {move || tm30_hue_svg().map(|svg| view! {
                                <div class="tm30-hue-diagram">
                                    <div class="diagram-svg" inner_html=svg />
                                </div>
                            })}
                            {move || tm30_result().map(|tm30| view! {
                                <div class="tm30-hue-details">
                                    <table class="tm30-hue-table">
                                        <thead>
                                            <tr>
                                                <th>"Hue"</th>
                                                <th>"Rf"</th>
                                                <th>"Rcs"</th>
                                                <th>"Rhs"</th>
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
                                                        <td>{format!("{:+.1}°", rhs)}</td>
                                                    </tr>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </tbody>
                                    </table>
                                    <div class="tm30-legend">
                                        <p><strong>"Rf"</strong>": Fidelity (0-100, higher = better match)"</p>
                                        <p><strong>"Rcs"</strong>": Chroma shift (+ve = more saturated)"</p>
                                        <p><strong>"Rhs"</strong>": Hue shift (degrees)"</p>
                                    </div>
                                </div>
                            })}
                        </div>
                    }.into_any(),
                }}
            </div>
        </div>
    }
}

/// Get color for Rf value (green = good, red = poor)
fn rf_color(rf: f64) -> &'static str {
    if rf >= 90.0 { "#22c55e" }
    else if rf >= 80.0 { "#84cc16" }
    else if rf >= 70.0 { "#eab308" }
    else if rf >= 60.0 { "#f97316" }
    else { "#ef4444" }
}

/// Get color for Rg value (100 = neutral, deviation shows color)
fn rg_color(rg: f64) -> &'static str {
    if (rg - 100.0).abs() <= 5.0 { "#22c55e" }
    else if (rg - 100.0).abs() <= 10.0 { "#84cc16" }
    else if (rg - 100.0).abs() <= 15.0 { "#eab308" }
    else { "#f97316" }
}

/// Get style for R:FR ratio value
/// Sunlight ~1.1-1.2, shade ~0.1-0.7, HPS ~1.5-2.0
fn r_fr_style(ratio: f64) -> String {
    let color = if ratio >= 0.8 && ratio <= 1.5 {
        "#22c55e" // Good for most plants
    } else if ratio >= 0.5 && ratio <= 2.0 {
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
