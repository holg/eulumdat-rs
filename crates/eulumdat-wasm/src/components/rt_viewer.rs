//! Host page for the lazily loaded path-traced scene viewer (`?wasm=rt_viewer`).
//!
//! The Bevy bundle renders into `#rt-canvas` and owns its controls (keys
//! and mouse, see `eulumdat-bevy/src/rt`). It reads the editor's current
//! luminaire from `localStorage["eulumdat_current_ldt"]`, so a file opened
//! in the editor is the one placed in the street, the open area, next to
//! the wall or in the room.
//!
//! WebGPU only: the tracer runs as compute shaders. `rt-viewer-loader.js`
//! probes `navigator.gpu.requestAdapter()` and sets
//! `window.rtViewerBackend` to `'webgpu' | 'unsupported'`.

use leptos::prelude::*;
use wasm_bindgen::prelude::*;

use super::skyglow_demo::{detect_browser, render_webgpu_missing};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = loadRtViewer)]
    fn load_rt_viewer() -> js_sys::Promise;

    #[wasm_bindgen(js_name = isRtViewerLoaded)]
    fn is_rt_viewer_loaded() -> bool;

    #[wasm_bindgen(js_name = isRtViewerLoading)]
    fn is_rt_viewer_loading() -> bool;
}

/// Read `window.rtViewerBackend` set by rt-viewer-loader.js.
fn backend_unsupported() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    js_sys::Reflect::get(&window, &JsValue::from_str("rtViewerBackend"))
        .ok()
        .and_then(|v| v.as_string())
        .as_deref()
        == Some("unsupported")
}

/// Name of the luminaire the editor last saved, if any.
fn current_luminaire_name() -> Option<String> {
    let storage = web_sys::window()?.local_storage().ok()??;
    let ldt = storage.get_item("eulumdat_current_ldt").ok()??;
    // Line 2 of an LDT is the luminaire name.
    ldt.lines().nth(1).map(|s| s.trim().to_string())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LoadState {
    NotLoaded,
    Loading,
    Loaded,
    Error,
    Unsupported,
}

#[component]
pub fn RtViewer() -> impl IntoView {
    let (load_state, set_load_state) = signal(LoadState::NotLoaded);
    let (error_msg, set_error_msg) = signal::<Option<String>>(None);
    let browser = StoredValue::new(detect_browser());
    let luminaire = StoredValue::new(current_luminaire_name());

    Effect::new(move |_| {
        if is_rt_viewer_loaded() {
            set_load_state.set(LoadState::Loaded);
        } else if is_rt_viewer_loading() {
            set_load_state.set(LoadState::Loading);
        }
    });

    let start_loading = move |_| {
        if !matches!(load_state.get(), LoadState::NotLoaded | LoadState::Error) {
            return;
        }
        set_load_state.set(LoadState::Loading);
        set_error_msg.set(None);

        wasm_bindgen_futures::spawn_local(async move {
            let result = wasm_bindgen_futures::JsFuture::from(load_rt_viewer()).await;
            match result {
                Ok(_) => set_load_state.set(LoadState::Loaded),
                Err(e) => {
                    if backend_unsupported() {
                        set_load_state.set(LoadState::Unsupported);
                    } else {
                        let msg = format!("{:?}", e);
                        web_sys::console::error_1(
                            &format!("Scene viewer load error: {}", msg).into(),
                        );
                        set_error_msg.set(Some(msg));
                        set_load_state.set(LoadState::Error);
                    }
                }
            }
        });
    };

    view! {
        <div style="width: 100vw; height: 100vh; position: fixed; top: 0; left: 0; background: #07080a; overflow: hidden;">
            <canvas
                id="rt-canvas"
                style="width: 100%; height: 100%; display: block; touch-action: none;"
            />

            {move || match load_state.get() {
                LoadState::NotLoaded => view! {
                    <div style="
                        position: absolute; top: 0; left: 0; right: 0; bottom: 0;
                        display: flex; flex-direction: column; align-items: center; justify-content: center;
                        background: rgba(7,8,10,0.96); color: white; cursor: pointer; text-align: center; padding: 24px;
                    " on:click=start_loading>
                        <div style="font-size: 14px; text-transform: uppercase; letter-spacing: 4px; color: #b3ec49; margin-bottom: 12px;">
                            "Path traced"
                        </div>
                        <div style="font-size: 28px; font-weight: 300; margin-bottom: 8px;">
                            "Scene Viewer"
                        </div>
                        <div style="font-size: 14px; color: #9aa4b2; margin-bottom: 8px; max-width: 640px; line-height: 1.5;">
                            "See how the light actually lands: on a street, in an open area, grazing a wall, in a room. "
                            "The intensity table drives a progressive path tracer; the same picture reads as a render, "
                            "as false-colour isolux, or as measurement points with lux values on EN 12464-1 / EN 13201-3 grids."
                        </div>
                        <div style="font-size: 12px; color: #6a7480; margin-bottom: 32px;">
                            {match luminaire.get_value() {
                                Some(n) => format!("Luminaire from the editor: {n}"),
                                None => "No luminaire in the editor yet: the road luminaire sample is used".to_string(),
                            }}
                        </div>
                        <button style="
                            padding: 14px 36px; font-size: 16px;
                            background: transparent; color: #b3ec49;
                            border: 1px solid #b3ec49; border-radius: 4px;
                            cursor: pointer; letter-spacing: 1px;
                            transition: all 0.2s;
                        ">"Open Viewer"</button>
                        <div style="margin-top: 24px; font-size: 12px; color: #555;">
                            "~15 MB download \u{b7} WebGPU required (Chrome / Edge / Safari 26+)"
                        </div>
                    </div>
                }.into_any(),

                LoadState::Loading => view! {
                    <div style="
                        position: absolute; top: 0; left: 0; right: 0; bottom: 0;
                        display: flex; flex-direction: column; align-items: center; justify-content: center;
                        background: rgba(7,8,10,0.96); color: white;
                    ">
                        <div class="spinner" style="
                            width: 40px; height: 40px; border: 2px solid #223;
                            border-top-color: #b3ec49; border-radius: 50%;
                            animation: spin 1s linear infinite;
                        "></div>
                        <div style="margin-top: 20px; font-size: 16px; color: #aaa;">
                            "Loading Scene Viewer..."
                        </div>
                        <div style="margin-top: 8px; font-size: 12px; color: #555;">
                            "Probing WebGPU and downloading the Bevy bundle"
                        </div>
                    </div>
                    <style>"@keyframes spin { to { transform: rotate(360deg); } }"</style>
                }.into_any(),

                LoadState::Loaded => view! {
                    <a href="./" style="
                        position: absolute; top: 12px; right: 12px;
                        background: rgba(0,0,0,0.55); color: #ccc; padding: 6px 12px;
                        border-radius: 8px; font-size: 12px; text-decoration: none; font-family: system-ui, sans-serif;
                    ">"\u{2190} Editor"</a>
                }.into_any(),

                LoadState::Error => view! {
                    <div style="
                        position: absolute; top: 0; left: 0; right: 0; bottom: 0;
                        display: flex; flex-direction: column; align-items: center; justify-content: center;
                        background: rgba(7,8,10,0.96); color: white;
                    ">
                        <div style="font-size: 18px; color: #ff6b6b; margin-bottom: 12px;">
                            "Failed to load the Scene Viewer"
                        </div>
                        <div style="font-size: 12px; color: #666; max-width: 400px; text-align: center;">
                            {move || error_msg.get().unwrap_or_default()}
                        </div>
                        <button
                            style="
                                margin-top: 20px; padding: 10px 24px; font-size: 14px;
                                background: transparent; color: #b3ec49;
                                border: 1px solid #b3ec49; border-radius: 4px; cursor: pointer;
                            "
                            on:click=move |_| {
                                set_load_state.set(LoadState::NotLoaded);
                                set_error_msg.set(None);
                            }
                        >"Try Again"</button>
                    </div>
                }.into_any(),

                LoadState::Unsupported => render_webgpu_missing(browser.get_value()).into_any(),
            }}
        </div>
    }
}
