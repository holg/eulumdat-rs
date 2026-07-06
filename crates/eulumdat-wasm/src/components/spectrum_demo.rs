//! Spectrum Lab — luminaire spectral analysis.
//!
//! Pick a real measured vendor SPD (Yuji, Luxeon, Signify — embedded from
//! `docs/SPDs/`) or trace the editor's LDT, and read the full spectral metric
//! set: correlated colour temperature, S/P ratio (night vision), melanopic DER
//! (circadian), mesopic road luminance, and a dark-sky compliance report.
//!
//! The daylight/sun/moon/sky side lives in the sibling **Daylight & Sky** tab
//! ([`crate::components::daylight_sky`]); both share
//! [`crate::components::spectral_ui`].

use crate::components::spectral_ui::*;
use crate::i18n::use_locale;
use eulumdat::Eulumdat;
use leptos::prelude::*;

#[component]
pub fn SpectrumDemo(
    /// When provided, trace this LDT from the editor. When None, a bare
    /// isotropic emitter carries the chosen vendor spectrum.
    #[prop(optional)]
    ldc: Option<ReadSignal<Eulumdat>>,
) -> impl IntoView {
    let locale = use_locale();

    let (preset, set_preset) = signal(SpdPreset(2)); // default: Luxeon 3000K
    let (photons, set_photons) = signal(200_000u64);
    let (road_luminance, set_road_luminance) = signal(1.0f64); // cd/m² photopic
    let (result, set_result) = signal::<Option<SpectralResult>>(None);
    let (running, set_running) = signal(false);

    let source_ldc = Memo::new(move |_| ldc.map(|l| l.get()));

    let trace = move |_: web_sys::MouseEvent| {
        set_running.set(true);
        let p = preset.get_untracked();
        let n = photons.get_untracked();
        let road_l = road_luminance.get_untracked();
        let src = source_ldc.get_untracked();

        wasm_bindgen_futures::spawn_local(async move {
            yield_to_browser().await;
            let out = run_luminaire_trace(&p, &src, n, road_l);
            set_result.set(Some(out));
            set_running.set(false);
        });
    };

    view! {
        <div style="width: 100%; padding: 20px; box-sizing: border-box; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;">
            <div style="max-width: 1000px; margin: 0 auto;">
                <div style="margin-bottom: 12px;">
                    <h2 style="margin: 0 0 4px; font-size: 1.3rem;">{move || format!("🌈 {}", locale.get().spectrum_lab.title)}</h2>
                    <p style="margin: 0; color: var(--text-muted, #8b949e); font-size: 0.9rem;">
                        {move || locale.get().spectrum_lab.intro.clone()}
                    </p>
                </div>

                // Controls row
                <div style="display: flex; flex-wrap: wrap; gap: 16px; align-items: flex-end; padding: 16px; background: var(--surface-elevated, #161b22); border: 1px solid var(--border, #30363d); border-radius: 8px; margin-bottom: 16px;">
                    <div>
                        <label style="display: block; font-size: 0.75rem; color: var(--text-muted, #8b949e); text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 4px;">{move || locale.get().spectrum_lab.source_spectrum.clone()}</label>
                        <select
                            style="padding: 6px 10px; background: var(--background, #0d1117); color: inherit; border: 1px solid var(--border, #30363d); border-radius: 6px;"
                            prop:value=move || preset.get().index().to_string()
                            on:change=move |ev| {
                                if let Ok(i) = event_target_value(&ev).parse::<usize>() {
                                    set_preset.set(SpdPreset::from_index(i));
                                }
                            }
                        >
                            {(0..PRESETS.len()).map(|i| {
                                let p = SpdPreset(i);
                                view! { <option value=i.to_string()>{p.label()}</option> }
                            }).collect::<Vec<_>>()}
                        </select>
                    </div>

                    <div>
                        <label style="display: block; font-size: 0.75rem; color: var(--text-muted, #8b949e); text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 4px;">{move || locale.get().spectrum_lab.photons.clone()}</label>
                        <div style="display: flex; gap: 4px;">
                            {[(50_000u64, "50K"), (200_000, "200K"), (500_000, "500K"), (1_000_000, "1M")].iter().map(|&(count, label)| {
                                view! {
                                    <button
                                        style=move || format!(
                                            "padding: 5px 10px; font-size: 0.8rem; border-radius: 6px; cursor: pointer; border: 1px solid {}; background: {}; color: inherit;",
                                            if photons.get() == count { "#58a6ff" } else { "var(--border, #30363d)" },
                                            if photons.get() == count { "rgba(88,166,255,0.15)" } else { "transparent" },
                                        )
                                        on:click=move |_| set_photons.set(count)
                                    >{label}</button>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    </div>

                    <div>
                        <label style="display: block; font-size: 0.75rem; color: var(--text-muted, #8b949e); text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 4px;">
                            {move || locale.get().spectrum_lab.road_luminance.clone()}
                        </label>
                        <input type="range" min="0.1" max="3.0" step="0.1"
                            style="width: 140px; accent-color: #58a6ff; vertical-align: middle;"
                            prop:value=move || road_luminance.get().to_string()
                            on:input=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                    set_road_luminance.set(v);
                                }
                            }
                        />
                        <span style="margin-left: 8px; font-size: 0.85rem;">{move || format!("{:.1}", road_luminance.get())}</span>
                    </div>

                    <button
                        style="padding: 8px 20px; background: #238636; color: white; border: none; border-radius: 6px; cursor: pointer; font-size: 0.9rem; font-weight: 600; margin-left: auto;"
                        disabled=move || running.get()
                        on:click=trace
                    >
                        {move || {
                            let sl = locale.get().spectrum_lab;
                            if running.get() { sl.tracing } else { sl.trace }
                        }}
                    </button>
                </div>

                // Results
                {move || {
                    let sl = locale.get().spectrum_lab;
                    match result.get() {
                        None => view! {
                            <div style="padding: 40px; text-align: center; color: var(--text-muted, #8b949e); border: 1px dashed var(--border, #30363d); border-radius: 8px;">
                                {sl.empty_hint.clone()}
                            </div>
                        }.into_any(),
                        Some(r) => render_result(&r, &sl, road_luminance.get_untracked()).into_any(),
                    }
                }}
            </div>
        </div>
    }
}
