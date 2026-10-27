//! Host page for the lazily loaded TM-25 ray file viewer (`?wasm=tm25`).
//!
//! The Bevy bundle renders into `#tm25-canvas`; this shell owns the DOM
//! controls. Settings go to Bevy as JSON in `localStorage["tm25_settings"]`,
//! status (header, far field, spectrum) comes back in
//! `localStorage["tm25_status"]`, and the chosen file is handed over through
//! the loader's IndexedDB stash (`tm25StashFile` / `tm25TakeFile`).

use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = loadTm25Viewer)]
    fn load_tm25_viewer() -> js_sys::Promise;
    #[wasm_bindgen(js_name = isTm25Loaded)]
    fn is_tm25_loaded() -> bool;
    #[wasm_bindgen(js_name = isTm25Loading)]
    fn is_tm25_loading() -> bool;
    #[wasm_bindgen(js_name = tm25StashFile)]
    pub fn tm25_stash_file(file: &web_sys::File) -> js_sys::Promise;
}

/// True for file names the ray viewer opens: `.tm25ray`, and `.ray`
/// (LightTools binary; other `.ray` dialects are rejected by magic on load).
pub fn is_tm25_file(name: &str) -> bool {
    let n = name.to_lowercase();
    n.ends_with(".tm25ray") || n.ends_with(".ray")
}

/// Stash `file` for the viewer and navigate to `?wasm=tm25`.
pub fn open_in_viewer(file: web_sys::File) {
    wasm_bindgen_futures::spawn_local(async move {
        if let Err(e) = wasm_bindgen_futures::JsFuture::from(tm25_stash_file(&file)).await {
            web_sys::console::error_1(&format!("tm25StashFile failed: {e:?}").into());
            return;
        }
        if let Some(loc) = web_sys::window().map(|w| w.location()) {
            let _ = loc.set_search("?wasm=tm25");
        }
    });
}

/// Fetch `url` and wrap the bytes in a `File` named after its last path
/// segment, so a linked sample loads exactly like a dropped one.
///
/// Used by the `&url=` parameter: `?wasm=tm25&url=https://…/sample.TM25RAY`.
/// Cross-origin URLs need CORS on the other end; GitHub Pages sends
/// `access-control-allow-origin: *`, so the samples there work.
async fn fetch_as_file(url: &str) -> Result<web_sys::File, String> {
    let window = web_sys::window().ok_or("no window")?;
    let resp_val = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(url))
        .await
        .map_err(|e| format!("fetch failed: {e:?}"))?;
    let resp: web_sys::Response = resp_val.dyn_into().map_err(|_| "not a Response")?;
    if !resp.ok() {
        return Err(format!("{} {}", resp.status(), resp.status_text()));
    }
    let blob_val =
        wasm_bindgen_futures::JsFuture::from(resp.blob().map_err(|e| format!("no body: {e:?}"))?)
            .await
            .map_err(|e| format!("read failed: {e:?}"))?;
    let blob: web_sys::Blob = blob_val.dyn_into().map_err(|_| "body is not a Blob")?;
    let name = url
        .rsplit('/')
        .next()
        .and_then(|n| n.split(['?', '#']).next())
        .filter(|n| !n.is_empty())
        .unwrap_or("sample.TM25RAY");
    let parts = js_sys::Array::of1(&blob);
    web_sys::File::new_with_blob_sequence(&parts.into(), name)
        .map_err(|e| format!("cannot wrap as File: {e:?}"))
}

/// Put `bytes` back into the loader's IndexedDB stash under `key`, so a
/// download button that took them can be used again.
fn restash_bytes(key: &str, bytes: &[u8]) {
    if let Some(window) = web_sys::window() {
        if let Ok(f) = js_sys::Reflect::get(&window, &"tm25StashFile".into())
            .and_then(|f| f.dyn_into::<js_sys::Function>())
        {
            let arr = js_sys::Uint8Array::from(bytes);
            let _ = f.call2(&window, &arr, &key.into());
        }
    }
}

/// Download a LightTools `.ray` the viewer stashed under `key` (see
/// `eulumdat_bevy::tm25::LIGHTTOOLS_STASH_KEY` and
/// `LIGHTTOOLS_SAMPLED_STASH_KEY`), then put it back.
fn download_lighttools(key: &'static str, filename: String) {
    wasm_bindgen_futures::spawn_local(async move {
        match super::goniosim::take_tm25_stash(key).await {
            Some(bytes) => {
                super::file_handler::download_bytes(&filename, &bytes, "application/octet-stream");
                restash_bytes(key, &bytes);
            }
            None => web_sys::console::warn_1(&format!("no {key} export stashed").into()),
        }
    });
}

/// Build the report, compile it to PDF in the browser, download it; on a
/// compiler failure download the Typst source instead.
fn download_report(
    st: serde_json::Value,
    ex: Option<serde_json::Value>,
    lum: Option<serde_json::Value>,
    file_name: String,
    set_busy: WriteSignal<bool>,
) {
    let data = eulumdat_typst::tm25_report::from_json(&st, ex.as_ref(), lum.as_ref(), &file_name);
    let source = eulumdat_typst::generate_tm25_report(&data);
    let stem = file_name
        .rsplit_once('.')
        .map(|(a, _)| a.to_string())
        .unwrap_or(file_name.clone());
    set_busy.set(true);
    wasm_bindgen_futures::spawn_local(async move {
        match super::app::compile_typst_to_pdf(&source).await {
            Ok(pdf) => super::file_handler::download_bytes(
                &format!("{stem}_report.pdf"),
                &pdf,
                "application/pdf",
            ),
            Err(e) => {
                web_sys::console::error_1(
                    &format!("PDF report failed: {e}; downloading the Typst source").into(),
                );
                super::file_handler::download_file(
                    &format!("{stem}_report.typ"),
                    &source,
                    "text/plain",
                );
            }
        }
        set_busy.set(false);
    });
}

/// A query parameter of the page, if present and non-empty.
fn query_param(key: &str) -> Option<String> {
    let search = web_sys::window()?.location().search().ok()?;
    let params = web_sys::UrlSearchParams::new_with_str(&search).ok()?;
    params.get(key).filter(|u| !u.is_empty())
}

/// The `url` query parameter, if the page was opened with one.
fn url_param() -> Option<String> {
    query_param("url")
}

fn backend_unsupported() -> bool {
    web_sys::window()
        .and_then(|w| js_sys::Reflect::get(&w, &"tm25Backend".into()).ok())
        .and_then(|v| v.as_string())
        .map(|s| s == "unsupported")
        .unwrap_or(false)
}

/// Mirror of `eulumdat_bevy::tm25::Tm25Settings` (same JSON).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    display_count: usize,
    ray_length_mm: f32,
    show_rays: bool,
    show_points: bool,
    show_solid: bool,
    solid_scale_mm: f32,
    point_size_mm: f32,
    color_mode: String,
    seed: u64,
    reset_camera: u32,
    show_plane: bool,
    plane_z_mm: f32,
    plane_size_mm: f32,
    plane_cells: usize,
    show_camera_view: bool,
    camera_cone_deg: f32,
    luminance_pixel_mm: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            display_count: 50_000,
            ray_length_mm: 3.0,
            show_rays: true,
            show_points: true,
            show_solid: false,
            solid_scale_mm: 8.0,
            point_size_mm: 0.01,
            color_mode: "gamma".into(),
            seed: 1,
            reset_camera: 0,
            show_plane: false,
            plane_z_mm: 0.1,
            plane_size_mm: 0.0,
            plane_cells: 96,
            show_camera_view: false,
            camera_cone_deg: 8.0,
            luminance_pixel_mm: 0.05,
        }
    }
}

/// Paint a luminance map (row-major values, `w × h`) into `canvas` with the
/// irradiance colour ramp, black for unlit pixels; row 0 is the bottom.
fn paint_luminance(
    canvas: &web_sys::HtmlCanvasElement,
    w: usize,
    h: usize,
    values: &[f64],
    peak: f64,
) {
    canvas.set_width(w as u32);
    canvas.set_height(h as u32);
    let Some(ctx) = canvas
        .get_context("2d")
        .ok()
        .flatten()
        .and_then(|c| c.dyn_into::<web_sys::CanvasRenderingContext2d>().ok())
    else {
        return;
    };
    let mut px = vec![0u8; w * h * 4];
    let stops: [[f64; 3]; 5] = [
        [0.0, 0.0, 255.0],
        [0.0, 255.0, 255.0],
        [0.0, 255.0, 0.0],
        [255.0, 255.0, 0.0],
        [255.0, 0.0, 0.0],
    ];
    for y in 0..h {
        for x in 0..w {
            let v = values.get(y * w + x).copied().unwrap_or(0.0);
            let i = ((h - 1 - y) * w + x) * 4;
            if v <= 0.0 || peak <= 0.0 {
                px[i + 3] = 255;
                continue;
            }
            let t = (v / peak).clamp(0.0, 1.0) * 4.0;
            let k = (t.floor() as usize).min(3);
            let f = t - k as f64;
            for c in 0..3 {
                px[i + c] = (stops[k][c] + (stops[k + 1][c] - stops[k][c]) * f) as u8;
            }
            px[i + 3] = 255;
        }
    }
    if let Ok(img) = web_sys::ImageData::new_with_u8_clamped_array_and_sh(
        wasm_bindgen::Clamped(&px),
        w as u32,
        h as u32,
    ) {
        let _ = ctx.put_image_data(&img, 0.0, 0.0);
    }
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

fn write_settings(s: &Settings) {
    if let (Some(st), Ok(json)) = (storage(), serde_json::to_string(s)) {
        let _ = st.set_item("tm25_settings", &json);
    }
}

#[derive(Clone, Copy, PartialEq)]
enum LoadState {
    NotLoaded,
    Loading,
    Loaded,
    Unsupported,
    Error,
}

#[component]
pub fn Tm25Viewer() -> impl IntoView {
    let (load_state, set_load_state) = signal(LoadState::NotLoaded);
    let (error_msg, set_error_msg) = signal::<Option<String>>(None);
    let (settings, set_settings) = signal(Settings::default());
    let (status, set_status) = signal::<Option<serde_json::Value>>(None);
    let (progress, set_progress) = signal(String::new());
    let (file_name, set_file_name) = signal::<Option<String>>(None);
    let (cam_view, set_cam_view) = signal(String::new());
    let (luminance, set_luminance) = signal::<Option<serde_json::Value>>(None);
    let (luminance_raw, set_luminance_raw) = signal(String::new());
    let lum_canvas = NodeRef::<leptos::html::Canvas>::new();
    let (exposure_s, set_exposure_s) = signal(1.0f64);
    // Export bundle from Bevy (`tm25_exports`): LDT / IES / ATLA / SPD texts.
    let (exports, set_exports) = signal::<Option<serde_json::Value>>(None);
    let (exports_raw, set_exports_raw) = signal(String::new());
    let (report_busy, set_report_busy) = signal(false);

    // Settings → localStorage on every change.
    Effect::new(move |_| write_settings(&settings.get()));

    // Status ← localStorage, polled.
    Effect::new(move |_| {
        let cb = Closure::<dyn FnMut()>::new(move || {
            if let Some(st) = storage() {
                if let Ok(Some(p)) = st.get_item("tm25_progress") {
                    set_progress.set(p);
                }
                if let Ok(Some(c)) = st.get_item("tm25_camera_view") {
                    if cam_view.get_untracked() != c {
                        set_cam_view.set(c);
                    }
                }
                match st.get_item("tm25_luminance") {
                    Ok(Some(json)) => {
                        if luminance_raw.get_untracked() != json {
                            set_luminance
                                .set(serde_json::from_str::<serde_json::Value>(&json).ok());
                            set_luminance_raw.set(json);
                        }
                    }
                    _ => {
                        if !luminance_raw.get_untracked().is_empty() {
                            set_luminance.set(None);
                            set_luminance_raw.set(String::new());
                        }
                    }
                }
                if let Ok(Some(json)) = st.get_item("tm25_exports") {
                    if exports_raw.get_untracked() != json {
                        set_exports.set(serde_json::from_str::<serde_json::Value>(&json).ok());
                        set_exports_raw.set(json);
                    }
                }
                if let Ok(Some(json)) = st.get_item("tm25_status") {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json) {
                        if status.get_untracked().as_ref() != Some(&v) {
                            set_status.set(Some(v));
                        }
                    }
                }
            }
        });
        if let Some(w) = web_sys::window() {
            let _ = w.set_interval_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                500,
            );
        }
        cb.forget();
    });

    // Paint the luminance map whenever it changes.
    Effect::new(move |_| {
        let Some(m) = luminance.get() else { return };
        let Some(canvas) = lum_canvas.get() else {
            return;
        };
        let w = m.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let h = m.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let peak = m.get("peak").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let values: Vec<f64> = m
            .get("values")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().map(|x| x.as_f64().unwrap_or(0.0)).collect())
            .unwrap_or_default();
        if w > 0 && h > 0 && values.len() == w * h {
            paint_luminance(&canvas, w, h, &values, peak);
        }
    });

    Effect::new(move |_| {
        if is_tm25_loaded() {
            set_load_state.set(LoadState::Loaded);
        } else if is_tm25_loading() {
            set_load_state.set(LoadState::Loading);
        }
    });

    let start_loading = move || {
        if !matches!(
            load_state.get_untracked(),
            LoadState::NotLoaded | LoadState::Error
        ) {
            return;
        }
        set_load_state.set(LoadState::Loading);
        set_error_msg.set(None);
        // Fresh status for this session.
        if let Some(st) = storage() {
            let _ = st.remove_item("tm25_status");
            let _ = st.remove_item("tm25_progress");
        }
        wasm_bindgen_futures::spawn_local(async move {
            let result = wasm_bindgen_futures::JsFuture::from(load_tm25_viewer()).await;
            match result {
                Ok(_) => set_load_state.set(LoadState::Loaded),
                Err(e) => {
                    if backend_unsupported() {
                        set_load_state.set(LoadState::Unsupported);
                    } else {
                        set_error_msg.set(Some(format!("{e:?}")));
                        set_load_state.set(LoadState::Error);
                    }
                }
            }
        });
    };

    // A file chosen here: stash it, then make sure the bundle is running.
    let take_file = move |file: web_sys::File| {
        set_file_name.set(Some(file.name()));
        set_status.set(None);
        set_progress.set(String::new());
        set_exports.set(None);
        set_exports_raw.set(String::new());
        if let Some(st) = storage() {
            let _ = st.remove_item("tm25_exports");
        }
        wasm_bindgen_futures::spawn_local(async move {
            let _ = wasm_bindgen_futures::JsFuture::from(tm25_stash_file(&file)).await;
        });
        start_loading();
    };

    // `?wasm=tm25&autoload=1&name=…`: another page on this origin (the
    // tm25.eu viewer) stashed its file for us; start the bundle at once, it
    // takes the file from the stash.
    Effect::new(move |_| {
        if query_param("autoload").is_none() || file_name.get_untracked().is_some() {
            return;
        }
        set_file_name.set(Some(
            query_param("name").unwrap_or_else(|| "ray file".into()),
        ));
        start_loading();
    });

    // `?wasm=tm25&url=…`: fetch that file and load it as if it had been
    // dropped, so a link can open a sample in one click.
    Effect::new(move |_| {
        let Some(url) = url_param() else { return };
        if file_name.get_untracked().is_some() {
            return; // the user already picked something
        }
        set_file_name.set(Some(url.rsplit('/').next().unwrap_or("sample").to_string()));
        set_progress.set("downloading…".into());
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_as_file(&url).await {
                Ok(file) => take_file(file),
                Err(e) => {
                    set_progress.set(String::new());
                    set_error_msg.set(Some(format!("could not load {url}: {e}")));
                    set_load_state.set(LoadState::Error);
                }
            }
        });
    });

    let on_file_input = move |ev: leptos::ev::Event| {
        let input: web_sys::HtmlInputElement = ev.target().unwrap().unchecked_into();
        if let Some(file) = input.files().and_then(|f| f.get(0)) {
            take_file(file);
        }
    };
    let on_drop = move |ev: leptos::ev::DragEvent| {
        ev.prevent_default();
        if let Some(file) = ev
            .data_transfer()
            .and_then(|d| d.files())
            .and_then(|f| f.get(0))
        {
            take_file(file);
        }
    };

    let upd = move |f: &dyn Fn(&mut Settings)| set_settings.update(|s| f(s));

    view! {
        <div
            style="width: 100vw; height: 100vh; position: fixed; top: 0; left: 0; background: #0d0f14; overflow: hidden; color: #e8ecf1; font-family: system-ui, sans-serif;"
            on:dragover=move |ev: leptos::ev::DragEvent| ev.prevent_default()
            on:drop=on_drop
        >
            <canvas id="tm25-canvas" style="width: 100%; height: 100%; display: block; touch-action: none;" />

            // Control panel
            <div style="position: absolute; top: 12px; left: 12px; width: 280px; max-height: calc(100vh - 24px); overflow-y: auto; background: rgba(12,14,20,0.88); border: 1px solid #2a2f3a; border-radius: 10px; padding: 12px 14px; font-size: 13px; backdrop-filter: blur(6px);">
                <div style="font-size: 11px; letter-spacing: 3px; text-transform: uppercase; color: #8fb4ff; margin-bottom: 6px;">"IES TM-25"</div>
                <div style="font-size: 18px; font-weight: 300; margin-bottom: 10px;">"Ray File Viewer"</div>

                <label style="display: block; margin-bottom: 10px;">
                    <span style="display: inline-block; padding: 6px 10px; background: #1f6feb; border-radius: 6px; cursor: pointer;">"Open .TM25RAY or LightTools .ray"</span>
                    <input type="file" accept=".tm25ray,.TM25RAY,.ray,.RAY" style="display: none;" on:change=on_file_input />
                </label>
                <div style="color: #9aa4b2; font-size: 12px; margin-bottom: 10px;">"or drop a ray file anywhere on the page"</div>

                {move || file_name.get().map(|n| view! { <div style="margin-bottom: 6px; word-break: break-all;">{n}</div> })}
                <super::tm25_mix::Tm25Mixer on_mixed=Callback::new(move |file: web_sys::File| take_file(file)) />
                {move || (!progress.get().is_empty() && status.get().is_none()).then(|| view! { <div style="color: #9aa4b2;">{format!("streaming {}", progress.get())}</div> })}

                <hr style="border: 0; border-top: 1px solid #2a2f3a; margin: 10px 0;" />

                <label style="display: flex; justify-content: space-between; gap: 8px; margin: 6px 0;">
                    <span>"Rays drawn"</span>
                    <select on:change=move |ev| { let v: usize = event_target_value(&ev).parse().unwrap_or(50_000); upd(&|s| s.display_count = v); }>
                        <option value="5000">"5k"</option>
                        <option value="20000">"20k"</option>
                        <option value="50000" selected=true>"50k"</option>
                        <option value="200000">"200k"</option>
                    </select>
                </label>
                <label style="display: block; margin: 6px 0;">
                    <span>{move || format!("Ray length {:.1} mm", settings.get().ray_length_mm)}</span>
                    <input type="range" min="-0.3" max="1.7" step="0.05" style="width: 100%;"
                        prop:value=move || settings.get().ray_length_mm.log10().to_string()
                        on:input=move |ev| { let v: f32 = event_target_value(&ev).parse().unwrap_or(0.48); upd(&|s| s.ray_length_mm = 10f32.powf(v)); } />
                </label>
                <label style="display: block; margin: 6px 0;">
                    <span>{move || format!("Solid size {:.0} mm", settings.get().solid_scale_mm)}</span>
                    <input type="range" min="1" max="40" step="1" style="width: 100%;"
                        prop:value=move || settings.get().solid_scale_mm.to_string()
                        on:input=move |ev| { let v: f32 = event_target_value(&ev).parse().unwrap_or(8.0); upd(&|s| s.solid_scale_mm = v); } />
                </label>
                <label style="display: block; margin: 6px 0;">
                    <span>{move || format!("Point size {:.3} mm", settings.get().point_size_mm)}</span>
                    <input type="range" min="0.002" max="0.05" step="0.001" style="width: 100%;"
                        prop:value=move || settings.get().point_size_mm.to_string()
                        on:input=move |ev| { let v: f32 = event_target_value(&ev).parse().unwrap_or(0.01); upd(&|s| s.point_size_mm = v); } />
                </label>
                <div style="display: flex; gap: 10px; flex-wrap: wrap; margin: 8px 0;">
                    <label><input type="checkbox" prop:checked=move || settings.get().show_rays on:change=move |ev| { let c = event_target_checked(&ev); upd(&|s| s.show_rays = c); } /> " Rays"</label>
                    <label><input type="checkbox" prop:checked=move || settings.get().show_points on:change=move |ev| { let c = event_target_checked(&ev); upd(&|s| s.show_points = c); } /> " Points"</label>
                    <label><input type="checkbox" prop:checked=move || settings.get().show_solid on:change=move |ev| { let c = event_target_checked(&ev); upd(&|s| s.show_solid = c); } /> " Far-field solid"</label>
                </div>
                <label style="display: flex; justify-content: space-between; gap: 8px; margin: 6px 0;">
                    <span>"Colour"</span>
                    <select on:change=move |ev| { let v = event_target_value(&ev); upd(&|s| s.color_mode = v.clone()); }>
                        <option value="gamma">"Polar angle γ"</option>
                        <option value="height">"Start height z"</option>
                        <option value="flux">"Flux per ray"</option>
                        <option value="spectrum">"Spectrum colour"</option>
                    </select>
                </label>
                <hr style="border: 0; border-top: 1px solid #2a2f3a; margin: 10px 0;" />
                <label><input type="checkbox" prop:checked=move || settings.get().show_camera_view on:change=move |ev| { let c = event_target_checked(&ev); upd(&|s| s.show_camera_view = c); } /> " Camera view (near-field image)"</label>
                <label style="display: block; margin: 6px 0;">
                    <span>{move || format!("Acceptance cone ±{:.0}°", settings.get().camera_cone_deg)}</span>
                    <input type="range" min="1" max="30" step="1" style="width: 100%;"
                        prop:value=move || settings.get().camera_cone_deg.to_string()
                        on:input=move |ev| { let v: f32 = event_target_value(&ev).parse().unwrap_or(8.0); upd(&|s| s.camera_cone_deg = v); } />
                </label>
                <label style="display: block; margin: 6px 0;">
                    <span>{move || format!("Luminance pixel {:.3} mm", settings.get().luminance_pixel_mm)}</span>
                    <input type="range" min="-2.3" max="0" step="0.05" style="width: 100%;"
                        prop:value=move || settings.get().luminance_pixel_mm.log10().to_string()
                        on:input=move |ev| { let v: f32 = event_target_value(&ev).parse().unwrap_or(-1.3); upd(&|s| s.luminance_pixel_mm = 10f32.powf(v)); } />
                </label>
                {move || luminance.get().filter(|_| settings.get().show_camera_view).map(|m| {
                    let f = |k: &str| m.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let unit = m.get("unit").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let (w, h) = (f("width"), f("height"));
                    let px = f("pixel_mm");
                    let fmt = |v: f64| if v >= 1e5 { format!("{:.3e}", v) } else { format!("{:.0}", v) };
                    view! {
                        <div style="margin-top: 8px; font-size: 11px; letter-spacing: 2px; text-transform: uppercase; color: #8fb4ff;">{format!("Luminance map · {unit}")}</div>
                        <canvas node_ref=lum_canvas style="width: 100%; height: auto; display: block; margin-top: 4px; border-radius: 6px; background: #000; image-rendering: pixelated;" />
                        <div style="display: flex; justify-content: space-between; color: #9aa4b2; font-size: 11px;"><span>"0"</span><span>{format!("peak {} {unit}", fmt(f("peak")))}</span></div>
                        <div style="color: #9aa4b2; font-size: 12px;">{format!(
                            "{} × {} px of {px:.3} mm ({:.2} × {:.2} mm) · mean over lit pixels {} {unit} · {:.0} rays per pixel, ≈ {:.0} % noise",
                            w, h, w * px, h * px, fmt(f("mean_lit")), f("rays_per_lit_pixel"), 100.0 / f("rays_per_lit_pixel").max(1e-9).sqrt()
                        )}</div>
                    }
                })}
                {move || {
                    let c = cam_view.get();
                    (settings.get().show_camera_view && !c.is_empty()).then(|| {
                        let v: serde_json::Value = serde_json::from_str(&c).unwrap_or_default();
                        let sel = v.get("selected").and_then(|x| x.as_u64()).unwrap_or(0);
                        let tot = v.get("total").and_then(|x| x.as_u64()).unwrap_or(0).max(1);
                        view! { <div style="color: #9aa4b2; font-size: 12px;">{format!("{sel} of {tot} rays head towards the camera ({:.2}%) — highlighted in yellow; orbit to scan", sel as f64 / tot as f64 * 100.0)}</div> }
                    })
                }}
                <hr style="border: 0; border-top: 1px solid #2a2f3a; margin: 10px 0;" />
                <label><input type="checkbox" prop:checked=move || settings.get().show_plane on:change=move |ev| { let c = event_target_checked(&ev); upd(&|s| s.show_plane = c); } /> " Irradiance plane (near field)"</label>
                <label style="display: block; margin: 6px 0;">
                    <span>{move || { let z = settings.get().plane_z_mm; if z < 1.0 { format!("Plane at z = {z:.2} mm") } else { format!("Plane at z = {z:.1} mm") } }}</span>
                    <input type="range" min="-1.3" max="2.7" step="0.02" style="width: 100%;"
                        prop:value=move || settings.get().plane_z_mm.log10().to_string()
                        on:input=move |ev| { let v: f32 = event_target_value(&ev).parse().unwrap_or(-1.0); upd(&|s| s.plane_z_mm = 10f32.powf(v)); } />
                </label>
                <label style="display: block; margin: 6px 0;">
                    <span>{move || { let h = settings.get().plane_size_mm; if h > 0.0 { format!("Plane half width {h:.1} mm") } else { "Plane half width: auto".to_string() } }}</span>
                    <input type="range" min="0" max="2.6" step="0.02" style="width: 100%;"
                        prop:value=move || { let h = settings.get().plane_size_mm; if h > 0.0 { h.log10().max(0.0).to_string() } else { "0".into() } }
                        on:input=move |ev| { let v: f32 = event_target_value(&ev).parse().unwrap_or(0.0); upd(&|s| s.plane_size_mm = if v <= 0.0 { 0.0 } else { 10f32.powf(v) }); } />
                </label>
                <label style="display: flex; justify-content: space-between; gap: 8px; margin: 6px 0;">
                    <span>"Grid"</span>
                    <select on:change=move |ev| { let v: usize = event_target_value(&ev).parse().unwrap_or(96); upd(&|s| s.plane_cells = v); }>
                        <option value="48">"48 × 48"</option>
                        <option value="96" selected=true>"96 × 96"</option>
                        <option value="192">"192 × 192"</option>
                    </select>
                </label>
                <button style="margin-top: 6px; padding: 5px 10px; background: #2a2f3a; color: #e8ecf1; border: 0; border-radius: 6px; cursor: pointer;"
                    on:click=move |_| upd(&|s| s.reset_camera = s.reset_camera.wrapping_add(1))>"Reset camera"</button>
                <div style="color: #9aa4b2; font-size: 11px; margin-top: 8px;">"Left drag orbit · right drag pan · wheel zoom · keys 1/2/3 toggle views, C colour, [ ] ray length, R reset"</div>
                {move || exports.get().map(|e| {
                    let name = e.get("name").and_then(|v| v.as_str()).unwrap_or("tm25").to_string();
                    let text = |k: &str| e.get(k).and_then(|v| v.as_str()).map(|s| s.to_string());
                    let ldt = text("ldt");
                    let ies = text("ies");
                    let xml = text("atla_xml");
                    let spd = text("spd");
                    let lumens = e.get("lamp_lumens").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let kept = e.get("rays_kept").and_then(|v| v.as_u64()).unwrap_or(0);
                    let has_lt = e.get("lighttools").and_then(|v| v.as_bool()).unwrap_or(false);
                    let has_lt_sampled = e.get("lighttools_sampled").and_then(|v| v.as_bool()).unwrap_or(false);
                    let sre = text("sre");
                    let quality_txt = text("quality_txt");
                    let (n_lt, n_lts) = (name.clone(), name.clone());
                    let btn = "padding: 5px 9px; background: #2a2f3a; color: #e8ecf1; border: 0; border-radius: 6px; cursor: pointer; font-size: 12px;";
                    let n_dl = name.clone();
                    let dl = move |content: Option<String>, ext: &'static str| {
                        let name = n_dl.clone();
                        move |_| if let Some(c) = &content { super::goniosim::download_string(c, &format!("{name}.{ext}")); }
                    };
                    let (n_editor, ldt_editor) = (name.clone(), ldt.clone());
                    let open_in_editor = move |_| {
                        let Some(c) = &ldt_editor else { return };
                        if let Some(st) = storage() {
                            let v = serde_json::json!({ "name": format!("{n_editor}.ldt"), "content": c });
                            let _ = st.set_item(super::app::PENDING_FILE_KEY, &v.to_string());
                        }
                        if let Some(loc) = web_sys::window().map(|w| w.location()) {
                            let _ = loc.set_search("");
                        }
                    };
                    let open_gonio = move |_| {
                        if let Some(loc) = web_sys::window().map(|w| w.location()) {
                            let _ = loc.set_search("?wasm=goniosim&source=tm25");
                        }
                    };
                    view! {
                        <hr style="border: 0; border-top: 1px solid #2a2f3a; margin: 10px 0;" />
                        <div style="font-size: 11px; letter-spacing: 2px; text-transform: uppercase; color: #8fb4ff; margin-bottom: 6px;">"Export far field"</div>
                        <div style="display: flex; gap: 6px; flex-wrap: wrap;">
                            <button style=btn on:click=dl(ldt.clone(), "ldt")>"EULUMDAT .ldt"</button>
                            <button style=btn on:click=dl(ies, "ies")>"IES .ies"</button>
                            <button style=btn on:click=dl(xml, "xml")>"ATLA .xml"</button>
                            {spd.clone().map(|_| view! { <button style=btn on:click=dl(spd.clone(), "spd")>"Spectrum .spd"</button> })}
                            {has_lt.then(|| view! { <button style=btn title="LightTools binary ray file of the kept rays, flux rescaled to the file total; no colour information, assign the spectrum in LightTools" on:click=move |_| download_lighttools("lighttools", format!("{}.ray", n_lt))>"LightTools .ray"</button> })}
                            {sre.clone().map(|_| view! { <button style=btn title="The spectrum as a LightTools .sre spectral distribution file, to assign to the ray data source" on:click=dl(sre.clone(), "sre")>"LightTools .sre"</button> })}
                            {has_lt_sampled.then(|| view! { <button style=btn title="Second option: a wavelength per ray drawn from the spectral table. The spectrum is then hard-wired into the rays in LightTools and cannot be reassigned; for sources whose spectrum varies over position and angle" on:click=move |_| download_lighttools("lighttools_sampled", format!("{}_sampled.ray", n_lts))>"LightTools .ray + λ"</button> })}
                            {quality_txt.clone().map(|_| view! { <button style=btn title="The quality report as text" on:click=dl(quality_txt.clone(), "quality.txt")>"Quality report .txt"</button> })}
                        </div>
                        <div style="display: flex; gap: 6px; flex-wrap: wrap; margin-top: 6px;">
                            <button style="padding: 6px 10px; background: #6f42c1; color: #fff; border: 0; border-radius: 6px; cursor: pointer; font-size: 12px;"
                                title="A PDF with the header, quality findings, far-field diagrams, spectrum with CCT, irradiance plane and the luminance map (switch on the camera view to include it)"
                                disabled=move || report_busy.get()
                                on:click=move |_| {
                                    let Some(st) = status.get_untracked() else { return };
                                    let lum = luminance.get_untracked().filter(|_| settings.get_untracked().show_camera_view);
                                    download_report(st, exports.get_untracked(), lum, file_name.get_untracked().unwrap_or_else(|| "rayfile".into()), set_report_busy);
                                }>{move || if report_busy.get() { "Rendering PDF…" } else { "PDF report" }}</button>
                        </div>
                        <div style="color: #9aa4b2; font-size: 11px; margin-top: 6px;">{
                            if lumens > 0.0 { format!("lamp flux {lumens:.2} lm · ATLA carries the spectrum") }
                            else { "radiometric file without lumens: intensities per kW, see report field".to_string() }
                        }</div>
                        <div style="display: flex; gap: 6px; flex-wrap: wrap; margin-top: 8px;">
                            <button style="padding: 6px 10px; background: #1f6feb; color: #fff; border: 0; border-radius: 6px; cursor: pointer; font-size: 12px;" on:click=open_in_editor>"Open in editor"</button>
                            <button style="padding: 6px 10px; background: #238636; color: #fff; border: 0; border-radius: 6px; cursor: pointer; font-size: 12px;" on:click=open_gonio>"Virtual goniometer"</button>
                        </div>
                        <div style="color: #9aa4b2; font-size: 11px; margin-top: 6px;">{format!("The goniometer replays {kept} flux-weighted rays as the near-field source, so covers and reflectors see the real emitting area.")}</div>
                    }
                })}
            </div>

            // Info panel
            {move || status.get().map(|st| {
                if let Some(err) = st.get("error").and_then(|e| e.as_str()) {
                    return view! { <div style="position: absolute; top: 12px; right: 12px; width: 320px; background: rgba(60,10,10,0.9); border: 1px solid #7a2a2a; border-radius: 10px; padding: 12px 14px; font-size: 13px;">{format!("Load failed: {err}")}</div> }.into_any();
                }
                let g = |k: &str| st.get(k).cloned().unwrap_or(serde_json::Value::Null);
                let text = g("text");
                let t = |i: usize| text.get(i).and_then(|v| v.as_str()).unwrap_or("").to_string();
                let ff = g("far_field");
                let rel: Vec<f64> = ff.get("relative").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_f64()).collect()).unwrap_or_default();
                let gam: Vec<f64> = ff.get("gamma").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_f64()).collect()).unwrap_or_default();
                let poly: String = rel.iter().zip(&gam).map(|(r, g)| format!("{:.1},{:.1}", g / 180.0 * 260.0, 70.0 - 66.0 * r)).collect::<Vec<_>>().join(" ");
                let spec = g("spectrum");
                let sw: Vec<f64> = spec.get("wavelengths").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_f64()).collect()).unwrap_or_default();
                let sv: Vec<f64> = spec.get("values").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_f64()).collect()).unwrap_or_default();
                let smax = sv.iter().cloned().fold(0.0, f64::max).max(1e-9);
                let (w0, w1) = (sw.first().copied().unwrap_or(380.0), sw.last().copied().unwrap_or(780.0));
                let spoly: String = sw.iter().zip(&sv).map(|(w, v)| format!("{:.1},{:.1}", (w - w0) / (w1 - w0).max(1.0) * 260.0, 70.0 - 66.0 * v / smax)).collect::<Vec<_>>().join(" ");
                // Cross sections like a vendor datasheet: absolute intensity, ±90°.
                let cuts = |key: &str| -> Vec<(f64, f64)> {
                    ff.get(key).and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|p| Some((p.get(0)?.as_f64()?, p.get(1)?.as_f64()?))).collect()).unwrap_or_default()
                };
                let (cx, cy) = (cuts("cut_x"), cuts("cut_y"));
                let cmax = cx.iter().chain(&cy).map(|p| p.1).fold(0.0, f64::max).max(1e-12);
                let cut_poly = |c: &Vec<(f64, f64)>| c.iter().map(|(a, v)| format!("{:.1},{:.1}", (a + 90.0) / 180.0 * 260.0, 70.0 - 66.0 * v / cmax)).collect::<Vec<_>>().join(" ");
                let (poly_x, poly_y) = (cut_poly(&cx), cut_poly(&cy));
                let unit = ff.get("unit").and_then(|v| v.as_str()).map(|u| if u == "Candela" { "cd" } else { "W/sr" }).unwrap_or("");
                let kind = g("flux_kind").as_str().unwrap_or("").to_string();
                let irr = st.get("irradiance").filter(|v| !v.is_null()).cloned();
                let uv = st.get("uv").filter(|v| !v.is_null()).cloned();
                let hort = st.get("horticulture").filter(|v| !v.is_null()).cloned();
                let flux_line = if kind == "Photometric" {
                    format!("{:.3} lm", g("luminous_flux_lm").as_f64().unwrap_or(0.0))
                } else {
                    format!("{:.4} W", g("radiant_flux_w").as_f64().unwrap_or(0.0))
                };
                view! {
                    <div style="position: absolute; top: 12px; right: 12px; width: 320px; max-height: calc(100vh - 24px); overflow-y: auto; background: rgba(12,14,20,0.88); border: 1px solid #2a2f3a; border-radius: 10px; padding: 12px 14px; font-size: 13px; backdrop-filter: blur(6px);">
                        <div style="font-size: 16px; margin-bottom: 4px;">{t(0)}</div>
                        <div style="color: #9aa4b2;">{t(1)} {" · "} {t(6)}</div>
                        <div style="margin-top: 8px;">{format!("{} rays · {} · {}", g("rays"), flux_line, g("date").as_str().unwrap_or(""))}</div>
                        <div style="color: #9aa4b2; font-size: 12px;">{format!("kept {} for display · columns {}", g("kept"), g("columns").as_array().map(|a| a.len()).unwrap_or(0))}</div>
                        {st.get("quality").cloned().map(|q| {
                            let errors = q.get("errors").and_then(|v| v.as_u64()).unwrap_or(0);
                            let warnings = q.get("warnings").and_then(|v| v.as_u64()).unwrap_or(0);
                            let verdict = q.get("verdict").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let noise = q.get("noise").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            let color = if errors > 0 { "#ff6b6b" } else if warnings > 0 { "#ffd166" } else { "#8ee07a" };
                            let findings: Vec<(String, String, String)> = q.get("findings").and_then(|v| v.as_array()).map(|a| a.iter().map(|f| (
                                f.get("severity").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                f.get("code").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                f.get("message").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            )).collect()).unwrap_or_default();
                            view! {
                                <div style="margin-top: 10px; font-size: 11px; letter-spacing: 2px; text-transform: uppercase; color: #8fb4ff;">"Quality"</div>
                                <div><span style=format!("color: {color}; font-weight: 600;")>{verdict}</span><span style="color: #9aa4b2; font-size: 12px;">{format!(" · far-field noise {:.0} % per 10°×5° bin at γ = 30°", noise * 100.0)}</span></div>
                                {findings.into_iter().map(|(sev, code, msg)| {
                                    let c = match sev.as_str() { "error" => "#ff6b6b", "warning" => "#ffd166", _ => "#9aa4b2" };
                                    view! { <div style="font-size: 12px; margin-top: 3px;"><span style=format!("color: {c}; font-family: ui-monospace, monospace;")>{code}</span>{format!(": {msg}")}</div> }
                                }).collect::<Vec<_>>()}
                            }
                        })}
                        <div style="margin-top: 10px; font-size: 11px; letter-spacing: 2px; text-transform: uppercase; color: #8fb4ff;">"Far field"</div>
                        <svg viewBox="0 0 260 80" style="width: 100%; height: auto; background: #0b0d12; border-radius: 6px; margin-top: 4px;">
                            <polyline points=poly fill="none" stroke="#8fb4ff" stroke-width="1.5" />
                            <line x1="0" y1="70" x2="260" y2="70" stroke="#2a2f3a" />
                            <text x="2" y="78" fill="#9aa4b2" font-size="7">"0°"</text>
                            <text x="125" y="78" fill="#9aa4b2" font-size="7">"90°"</text>
                            <text x="246" y="78" fill="#9aa4b2" font-size="7">"180°"</text>
                        </svg>
                        <div style="color: #9aa4b2; font-size: 12px;">{format!("peak {:.4} {} · half intensity at {}°", ff.get("peak").and_then(|v| v.as_f64()).unwrap_or(0.0), unit, ff.get("half_intensity_deg").and_then(|v| v.as_f64()).map(|v| format!("{v:.0}")).unwrap_or_else(|| "–".into()))}</div>
                        {(!cx.is_empty()).then(|| view! {
                            <div style="margin-top: 8px; font-size: 11px; letter-spacing: 2px; text-transform: uppercase; color: #8fb4ff;">{format!("Cross sections · {unit}")}</div>
                            <svg viewBox="0 0 260 80" style="width: 100%; height: auto; background: #0b0d12; border-radius: 6px; margin-top: 4px;">
                                <line x1="130" y1="4" x2="130" y2="70" stroke="#2a2f3a" />
                                <line x1="0" y1="70" x2="260" y2="70" stroke="#2a2f3a" />
                                <polyline points=poly_x fill="none" stroke="#ff8c5a" stroke-width="1.5" />
                                <polyline points=poly_y fill="none" stroke="#5ad2ff" stroke-width="1.5" />
                                <text x="2" y="78" fill="#9aa4b2" font-size="7">"−90°"</text>
                                <text x="124" y="78" fill="#9aa4b2" font-size="7">"0°"</text>
                                <text x="242" y="78" fill="#9aa4b2" font-size="7">"+90°"</text>
                                <text x="2" y="10" fill="#9aa4b2" font-size="7">{format!("{cmax:.4}")}</text>
                            </svg>
                            <div style="color: #9aa4b2; font-size: 12px;"><span style="color:#ff8c5a;">"— x cut"</span> "  " <span style="color:#5ad2ff;">"— y cut"</span> {format!("  · axis {:.4}, plateau {:.4} {unit}", cx.iter().find(|p| p.0 == 0.0).map(|p| p.1).unwrap_or(0.0), cmax)}</div>
                        })}
                        {irr.as_ref().map(|ir| {
                            let unit = ir.get("unit").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let peak = ir.get("peak").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            let radial: Vec<(f64, f64)> = ir.get("radial").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|p| Some((p.get(0)?.as_f64()?, p.get(1)?.as_f64()?))).collect()).unwrap_or_default();
                            let rmax = radial.last().map(|p| p.0).unwrap_or(1.0).max(1e-6);
                            let pmax = radial.iter().map(|p| p.1).fold(0.0, f64::max).max(1e-12);
                            let rpoly: String = radial.iter().map(|(r, e)| format!("{:.1},{:.1}", r / rmax * 260.0, 70.0 - 66.0 * e / pmax)).collect::<Vec<_>>().join(" ");
                            let z = ir.get("z_mm").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            let half = ir.get("half_mm").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            let captured = ir.get("captured").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            let fmt = |v: f64| if v >= 1000.0 { format!("{:.3e}", v) } else { format!("{:.3}", v) };
                            view! {
                                <div style="margin-top: 10px; font-size: 11px; letter-spacing: 2px; text-transform: uppercase; color: #8fb4ff;">{format!("Irradiance at z = {z} mm")}</div>
                                <div style="height: 8px; margin-top: 4px; border-radius: 4px; background: linear-gradient(90deg, #0000ff, #00ffff, #00ff00, #ffff00, #ff0000);"></div>
                                <div style="display: flex; justify-content: space-between; color: #9aa4b2; font-size: 11px;"><span>"0"</span><span>{format!("{} {unit}", fmt(peak))}</span></div>
                                <svg viewBox="0 0 260 80" style="width: 100%; height: auto; background: #0b0d12; border-radius: 6px; margin-top: 4px;">
                                    <polyline points=rpoly fill="none" stroke="#ffd166" stroke-width="1.5" />
                                    <line x1="0" y1="70" x2="260" y2="70" stroke="#2a2f3a" />
                                    <text x="2" y="78" fill="#9aa4b2" font-size="7">"r = 0"</text>
                                    <text x="226" y="78" fill="#9aa4b2" font-size="7">{format!("{half:.1} mm")}</text>
                                </svg>
                                <div style="color: #9aa4b2; font-size: 12px;">{format!("radial profile · peak {} {unit} · {:.0}% of flux inside ±{half:.1} mm", fmt(peak), captured * 100.0)}</div>
                            }
                        })}
                        {hort.as_ref().map(|hv| {
                            let f = |k: &str| hv.get(k).and_then(|v| v.as_f64());
                            let ppf = f("ppf_umol_s").unwrap_or(0.0);
                            let per_w = f("ppfd_per_watt").unwrap_or(0.0);
                            let ppfd = f("ppfd_peak_umol_m2_s");
                            view! {
                                <div style="margin-top: 10px; font-size: 11px; letter-spacing: 2px; text-transform: uppercase; color: #8ee07a;">"Horticulture"</div>
                                <div style="font-size: 12px;">{format!("PPF {ppf:.2} µmol/s from the file flux · {per_w:.2} µmol/J of this spectrum")}</div>
                                {ppfd.map(|p| view! { <div style="font-size: 12px;">{format!("PPFD peak on the plane {p:.1} µmol m⁻² s⁻¹")}</div> })}
                                {ppfd.is_none().then(|| view! { <div style="color: #9aa4b2; font-size: 12px;">"Enable the irradiance plane for PPFD."</div> })}
                            }
                        })}
                        {uv.as_ref().map(|u| {
                            let k = u.get("effective_fraction").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            let name = u.get("action_spectrum").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let limit = u.get("limit_j_per_m2").and_then(|v| v.as_f64()).unwrap_or(30.0);
                            let peak_eff = u.get("plane_peak_effective_w_per_m2").and_then(|v| v.as_f64());
                            view! {
                                <div style="margin-top: 10px; font-size: 11px; letter-spacing: 2px; text-transform: uppercase; color: #c48cff;">"UV exposure"</div>
                                <div style="color: #9aa4b2; font-size: 12px;">{format!("{name}: effective fraction {k:.3} of the radiometric value")}</div>
                                {peak_eff.map(|e| view! {
                                    <label style="display: block; margin: 6px 0;">
                                        <span>{move || format!("Exposure time {:.0} s", exposure_s.get())}</span>
                                        <input type="range" min="0" max="4" step="0.05" style="width: 100%;"
                                            prop:value=move || exposure_s.get().log10().to_string()
                                            on:input=move |ev| { let v: f64 = event_target_value(&ev).parse().unwrap_or(0.0); set_exposure_s.set(10f64.powf(v)); } />
                                    </label>
                                    <div style="font-size: 12px;">{move || {
                                        let t = exposure_s.get();
                                        let dose = e * t; // J/m²
                                        let over = dose / limit;
                                        let color = if over >= 1.0 { "#ff6b6b" } else { "#8ee07a" };
                                        view! { <span style=format!("color: {color};")>{format!("effective peak {:.3e} W/m² · dose {:.3} mJ/cm² ({:.0}% of the {limit:.0} J/m² 8 h limit)", e, dose / 10.0, over * 100.0)}</span> }
                                    }}</div>
                                    <div style="color: #9aa4b2; font-size: 12px;">{u.get("seconds_to_limit_at_peak").and_then(|v| v.as_f64()).map(|s| if s < 60.0 { format!("limit reached after {s:.1} s at the plane peak") } else { format!("limit reached after {:.1} min at the plane peak", s / 60.0) }).unwrap_or_default()}</div>
                                })}
                                {peak_eff.is_none().then(|| view! { <div style="color: #9aa4b2; font-size: 12px;">"Enable the irradiance plane for a dose estimate."</div> })}
                            }
                        })}
                        {(!sw.is_empty()).then(|| view! {
                            <div style="margin-top: 10px; font-size: 11px; letter-spacing: 2px; text-transform: uppercase; color: #8fb4ff;">"Spectrum"</div>
                            <svg viewBox="0 0 260 80" style="width: 100%; height: auto; background: #0b0d12; border-radius: 6px; margin-top: 4px;">
                                <polyline points=spoly fill="none" stroke="#c48cff" stroke-width="1.5" />
                                <text x="2" y="78" fill="#9aa4b2" font-size="7">{format!("{w0:.0} nm")}</text>
                                <text x="226" y="78" fill="#9aa4b2" font-size="7">{format!("{w1:.0} nm")}</text>
                            </svg>
                            <div style="color: #9aa4b2; font-size: 12px;">{format!("peak {} nm{}", spec.get("peak").and_then(|p| p.get(0)).and_then(|v| v.as_f64()).map(|v| format!("{v:.0}")).unwrap_or_else(|| "–".into()), if spec.get("uv").and_then(|v| v.as_bool()).unwrap_or(false) { " · ultraviolet (false colour)" } else { "" })}</div>
                        })}
                    </div>
                }.into_any()
            })}

            // Load overlay
            {move || match load_state.get() {
                LoadState::NotLoaded => view! {
                    <div style="position: absolute; bottom: 24px; left: 50%; transform: translateX(-50%); background: rgba(12,14,20,0.9); border: 1px solid #2a2f3a; border-radius: 10px; padding: 14px 20px; cursor: pointer; text-align: center;" on:click=move |_| start_loading()>
                        <div style="font-size: 15px;">"Click to load the 3D viewer (WebGPU, ~25 MB)"</div>
                        <div style="color: #9aa4b2; font-size: 12px; margin-top: 4px;">"Opening a file starts it automatically"</div>
                    </div>
                }.into_any(),
                LoadState::Loading => view! {
                    <div style="position: absolute; bottom: 24px; left: 50%; transform: translateX(-50%); background: rgba(12,14,20,0.9); border: 1px solid #2a2f3a; border-radius: 10px; padding: 14px 20px;">"Loading viewer…"</div>
                }.into_any(),
                LoadState::Loaded => view! { <span /> }.into_any(),
                LoadState::Unsupported => view! {
                    <div style="position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; background: rgba(6,8,12,0.95); text-align: center; padding: 24px;">
                        <div>
                            <div style="font-size: 22px; margin-bottom: 8px;">"WebGPU is not available"</div>
                            <div style="color: #9aa4b2;">"The TM-25 viewer needs WebGPU: Chrome/Edge 113+, Safari 26+, or Firefox with WebGPU enabled."</div>
                        </div>
                    </div>
                }.into_any(),
                LoadState::Error => view! {
                    <div style="position: absolute; bottom: 24px; left: 50%; transform: translateX(-50%); background: rgba(60,10,10,0.9); border: 1px solid #7a2a2a; border-radius: 10px; padding: 14px 20px; cursor: pointer;" on:click=move |_| start_loading()>
                        {format!("Viewer failed to load: {} (click to retry)", error_msg.get().unwrap_or_default())}
                    </div>
                }.into_any(),
            }}
        </div>
    }
}
