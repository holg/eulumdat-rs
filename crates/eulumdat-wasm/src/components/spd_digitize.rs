//! Chart image → SPD digitizing for the web editor.
//!
//! A dropped PNG/JPEG is decoded by the browser (`createImageBitmap` +
//! `<canvas>`), so the WASM side only sees raw RGBA pixels and
//! `eulumdat-digitize` stays dependency-free. The user supplies the x-axis
//! range in a small dialog, the curve is traced, the resulting `.spd` text is
//! offered for download and loaded into the editor through the normal SPD
//! path.

use eulumdat_digitize::{digitize, CurveMode, Digitized, ImageRef, Options};
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

/// Longer image side is downscaled to this many pixels before tracing:
/// plenty for 1 nm resolution and keeps the pass under ~100 ms.
const MAX_SIDE: u32 = 2400;

/// True for file names the browser can decode as raster images.
pub fn is_chart_image(name: &str) -> bool {
    let lower = name.to_lowercase();
    [".png", ".jpg", ".jpeg", ".webp", ".gif", ".bmp"]
        .iter()
        .any(|ext| lower.ends_with(ext))
}

/// A decoded chart waiting for the user's axis range.
#[derive(Clone, Debug, PartialEq)]
pub struct PendingChart {
    pub name: String,
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

fn js_err(e: wasm_bindgen::JsValue) -> String {
    e.as_string()
        .or_else(|| js_sys::Error::from(e.clone()).message().as_string())
        .unwrap_or_else(|| format!("{e:?}"))
}

/// Decode `file` through the browser into RGBA pixels.
async fn decode(file: web_sys::File) -> Result<PendingChart, String> {
    let name = file.name();
    let window = web_sys::window().ok_or("no window")?;
    let promise = window
        .create_image_bitmap_with_blob(&file)
        .map_err(js_err)?;
    let bitmap: web_sys::ImageBitmap = JsFuture::from(promise)
        .await
        .map_err(js_err)?
        .dyn_into()
        .map_err(|_| "createImageBitmap did not return an ImageBitmap".to_string())?;
    let (w, h) = (bitmap.width(), bitmap.height());
    if w == 0 || h == 0 {
        return Err("image has no pixels".into());
    }
    let scale = (MAX_SIDE as f64 / w.max(h) as f64).min(1.0);
    let cw = ((w as f64 * scale).round() as u32).max(1);
    let ch = ((h as f64 * scale).round() as u32).max(1);

    let document = window.document().ok_or("no document")?;
    let canvas: web_sys::HtmlCanvasElement = document
        .create_element("canvas")
        .map_err(js_err)?
        .dyn_into()
        .map_err(|_| "canvas element".to_string())?;
    canvas.set_width(cw);
    canvas.set_height(ch);
    let ctx: web_sys::CanvasRenderingContext2d = canvas
        .get_context("2d")
        .map_err(js_err)?
        .ok_or("no 2d context")?
        .dyn_into()
        .map_err(|_| "2d context".to_string())?;
    ctx.draw_image_with_image_bitmap_and_dw_and_dh(&bitmap, 0.0, 0.0, cw as f64, ch as f64)
        .map_err(js_err)?;
    bitmap.close();
    let data = ctx
        .get_image_data(0.0, 0.0, cw as f64, ch as f64)
        .map_err(js_err)?
        .data();
    Ok(PendingChart {
        name,
        width: cw as usize,
        height: ch as usize,
        rgba: data.0,
    })
}

/// Kick off decoding; the dialog opens once pixels are available.
pub fn start(file: web_sys::File, pending: RwSignal<Option<PendingChart>>) {
    wasm_bindgen_futures::spawn_local(async move {
        match decode(file).await {
            Ok(chart) => pending.set(Some(chart)),
            Err(e) => web_sys::console::error_1(&format!("Chart decode failed: {e}").into()),
        }
    });
}

/// Outcome of one trace, kept for display.
#[derive(Clone, Debug, PartialEq)]
struct Traced {
    spd_name: String,
    spd_text: String,
    summary: String,
    peaks: String,
    warnings: Vec<String>,
    /// Polyline points in a 300×100 box for the preview.
    preview: String,
}

fn spd_file_name(image_name: &str) -> String {
    let stem = image_name
        .rsplit_once('.')
        .map(|(s, _)| s)
        .unwrap_or(image_name);
    format!("{stem}.spd")
}

fn preview_points(d: &Digitized) -> String {
    let n = d.values.len();
    if n < 2 {
        return String::new();
    }
    d.values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            format!(
                "{:.1},{:.1}",
                300.0 * i as f64 / (n - 1) as f64,
                100.0 - 98.0 * v
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn trace(
    chart: &PendingChart,
    label: &str,
    first: f64,
    last: f64,
    mode: CurveMode,
) -> Result<Traced, String> {
    let img =
        ImageRef::new(chart.width, chart.height, 4, &chart.rgba).map_err(|e| e.to_string())?;
    let opts = Options {
        x_range: Some((first, last)),
        mode,
        ..Options::default()
    };
    let d = digitize(img, &opts).map_err(|e| e.to_string())?;
    let c = &d.calibration;
    let summary = format!(
        "{} calibration points from {} (max residual {:.2} nm), {:?} curve, {:.2} nm/px, {} samples {}–{} nm",
        c.x_points.len(),
        if c.x_from_labels { "label positions" } else { "tick marks" },
        c.x_residual,
        c.mode,
        c.nm_per_px,
        d.wavelengths.len(),
        d.wavelengths.first().copied().unwrap_or(0.0),
        d.wavelengths.last().copied().unwrap_or(0.0),
    );
    Ok(Traced {
        spd_name: spd_file_name(&chart.name),
        spd_text: d.to_spd(label, &chart.name, None),
        summary,
        peaks: d.peak_summary(),
        warnings: d.warnings.clone(),
        preview: preview_points(&d),
    })
}

/// Modal shown while a chart image is pending: asks for the x-axis range,
/// traces the curve, previews the result and hands the `.spd` text on.
#[component]
pub fn DigitizeDialog(
    pending: RwSignal<Option<PendingChart>>,
    /// Replace the editor content with a spectral-only document `(name, spd text)`.
    on_spd: Callback<(String, String)>,
    /// Attach the spectrum to the luminaire currently loaded `(spd text)`.
    on_attach: Callback<String>,
    /// Name of the loaded luminaire that has an intensity distribution, if any.
    luminaire: Signal<Option<String>>,
) -> impl IntoView {
    let (first_nm, set_first_nm) = signal(String::from("380"));
    let (last_nm, set_last_nm) = signal(String::from("780"));
    let (label, set_label) = signal(String::new());
    let (mode, set_mode) = signal(String::from("auto"));
    let (result, set_result) = signal(None::<Result<Traced, String>>);

    let close = move || {
        pending.set(None);
        set_result.set(None);
    };

    let run = move || {
        let Some(chart) = pending.get_untracked() else {
            return;
        };
        let parse = |s: String, what: &str| -> Result<f64, String> {
            s.trim()
                .parse::<f64>()
                .map_err(|_| format!("{what} must be a number, got {s:?}"))
        };
        let outcome = (|| {
            let first = parse(first_nm.get_untracked(), "First tick")?;
            let last = parse(last_nm.get_untracked(), "Last tick")?;
            // NaN-safe: anything but a strictly greater last tick is rejected.
            if !matches!(last.partial_cmp(&first), Some(std::cmp::Ordering::Greater)) {
                return Err("Last tick must be greater than the first".into());
            }
            let mode = match mode.get_untracked().as_str() {
                "fill" => CurveMode::Fill,
                "line" => CurveMode::Line,
                _ => CurveMode::Auto,
            };
            let lbl = label.get_untracked();
            let lbl = if lbl.trim().is_empty() {
                chart
                    .name
                    .rsplit_once('.')
                    .map(|(s, _)| s.to_string())
                    .unwrap_or_else(|| chart.name.clone())
            } else {
                lbl
            };
            trace(&chart, &lbl, first, last, mode)
        })();
        set_result.set(Some(outcome));
    };

    let download = move || {
        if let Some(Ok(t)) = result.get_untracked() {
            super::file_handler::download_file(&t.spd_name, &t.spd_text, "text/plain");
        }
    };

    let use_in_editor = move || {
        if let Some(Ok(t)) = result.get_untracked() {
            on_spd.run((t.spd_name.clone(), t.spd_text.clone()));
            close();
        }
    };

    let attach = move || {
        if let Some(Ok(t)) = result.get_untracked() {
            on_attach.run(t.spd_text.clone());
            close();
        }
    };

    view! {
        {move || pending.get().map(|chart| view! {
            <div class="modal-overlay" on:click=move |_| close()>
                <div class="modal-content digitize-modal" on:click=|e| e.stop_propagation()>
                    <h2>"Digitize spectrum chart"</h2>
                    <p class="about-subtitle">
                        {format!("{} — {}×{} px", chart.name, chart.width, chart.height)}
                    </p>
                    <p class="about-description">
                        "The axes, tick marks and curve are detected automatically. "
                        "Enter the wavelength printed at the first and last x-axis tick."
                    </p>
                    <div class="digitize-form">
                        <div class="form-group">
                            <label for="digitize-first">"First tick (nm)"</label>
                            <input id="digitize-first" type="number" step="1"
                                prop:value=move || first_nm.get()
                                on:input=move |ev| set_first_nm.set(event_target_value(&ev)) />
                        </div>
                        <div class="form-group">
                            <label for="digitize-last">"Last tick (nm)"</label>
                            <input id="digitize-last" type="number" step="1"
                                prop:value=move || last_nm.get()
                                on:input=move |ev| set_last_nm.set(event_target_value(&ev)) />
                        </div>
                        <div class="form-group">
                            <label for="digitize-mode">"Curve"</label>
                            <select id="digitize-mode"
                                prop:value=move || mode.get()
                                on:change=move |ev| set_mode.set(event_target_value(&ev))>
                                <option value="auto">"Auto"</option>
                                <option value="fill">"Filled area"</option>
                                <option value="line">"Line"</option>
                            </select>
                        </div>
                        <div class="form-group">
                            <label for="digitize-label">"Label"</label>
                            <input id="digitize-label" type="text" placeholder="Lamp name"
                                prop:value=move || label.get()
                                on:input=move |ev| set_label.set(event_target_value(&ev)) />
                        </div>
                    </div>
                    <div class="modal-actions">
                        <button class="btn btn-primary" on:click=move |_| run()>"Trace curve"</button>
                        <button class="btn btn-secondary" on:click=move |_| close()>"Cancel"</button>
                    </div>
                    {move || result.get().map(|r| match r {
                        Ok(t) => view! {
                            <div class="digitize-result">
                                <svg viewBox="0 0 300 100" class="digitize-preview" role="img" aria-label="Digitized spectrum preview">
                                    <polyline points=t.preview.clone() fill="none" stroke="currentColor" stroke-width="1.5" />
                                </svg>
                                <p class="digitize-peaks">{if t.peaks.is_empty() { "No distinct peaks".to_string() } else { format!("Peaks: {}", t.peaks) }}</p>
                                <p class="digitize-summary">{t.summary.clone()}</p>
                                {(!t.warnings.is_empty()).then(|| view! {
                                    <ul class="digitize-warnings">
                                        {t.warnings.iter().map(|w| view! { <li>{w.clone()}</li> }).collect_view()}
                                    </ul>
                                })}
                                <div class="modal-actions">
                                    {move || luminaire.get().map(|name| view! {
                                        <button class="btn btn-primary" on:click=move |_| attach()
                                            title="Store the spectrum on the loaded luminaire so PPFD, colorimetry and spectral tracing use it">
                                            {format!("Attach to {name}")}
                                        </button>
                                    })}
                                    <button class="btn btn-secondary" on:click=move |_| use_in_editor()>"Load as spectrum only"</button>
                                    <button class="btn btn-secondary" on:click=move |_| download()>{format!("Download {}", t.spd_name)}</button>
                                </div>
                            </div>
                        }.into_any(),
                        Err(e) => view! {
                            <p class="digitize-error">{e}</p>
                        }.into_any(),
                    })}
                </div>
            </div>
        })}
    }
}
