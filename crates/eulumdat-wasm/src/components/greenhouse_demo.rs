//! Greenhouse Demo Component — "Vom Schweinestall zum Gewächshaus"
//!
//! Fullscreen lazy-loaded Bevy WASM app: an old pig barn rebuilt as a
//! glasshouse, used to showcase the spectral stack in 3D — real solar
//! position + sky models, spectral glazing, TM-33 grow-light SPDs, canopy
//! PPFD, and the spectral Monte Carlo tracer. Activated via
//! `?wasm=greenhouse_demo` (alias `?wasm=greenhouse`).
//!
//! WebGPU only: Bevy's physically based atmosphere and bloom need compute
//! shaders + storage textures, so unlike the Skyglow demo there is no
//! WebGL2 fallback bundle. `greenhouse-loader.js` probes
//! `navigator.gpu.requestAdapter()` and sets `window.greenhouseBackend` to
//! `'webgpu' | 'unsupported'`; on `unsupported` this component shows the
//! shared missing-WebGPU screen from the Skyglow demo.

use leptos::prelude::*;
use wasm_bindgen::prelude::*;

use super::skyglow_demo::{detect_browser, render_webgpu_missing};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = loadGreenhouseDemo)]
    fn load_greenhouse_demo() -> js_sys::Promise;

    #[wasm_bindgen(js_name = isGreenhouseLoaded)]
    fn is_greenhouse_loaded() -> bool;

    #[wasm_bindgen(js_name = isGreenhouseLoading)]
    fn is_greenhouse_loading() -> bool;
}

/// Read `window.greenhouseBackend` set by greenhouse-loader.js.
fn backend_unsupported() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    js_sys::Reflect::get(&window, &JsValue::from_str("greenhouseBackend"))
        .ok()
        .and_then(|v| v.as_string())
        .as_deref()
        == Some("unsupported")
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
pub fn GreenhouseDemo() -> impl IntoView {
    let (load_state, set_load_state) = signal(LoadState::NotLoaded);
    let (error_msg, set_error_msg) = signal::<Option<String>>(None);
    let browser = StoredValue::new(detect_browser());

    Effect::new(move |_| {
        if is_greenhouse_loaded() {
            set_load_state.set(LoadState::Loaded);
        } else if is_greenhouse_loading() {
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
            let promise = load_greenhouse_demo();
            let result = wasm_bindgen_futures::JsFuture::from(promise).await;
            match result {
                Ok(_) => set_load_state.set(LoadState::Loaded),
                Err(e) => {
                    if backend_unsupported() {
                        set_load_state.set(LoadState::Unsupported);
                    } else {
                        let msg = format!("{:?}", e);
                        web_sys::console::error_1(
                            &format!("Greenhouse load error: {}", msg).into(),
                        );
                        set_error_msg.set(Some(msg));
                        set_load_state.set(LoadState::Error);
                    }
                }
            }
        });
    };

    view! {
        <div style="width: 100vw; height: 100vh; position: fixed; top: 0; left: 0; background: #0b1410; overflow: hidden;">
            <canvas
                id="greenhouse-canvas"
                style="width: 100%; height: 100%; display: block; touch-action: none;"
            />

            {move || match load_state.get() {
                LoadState::NotLoaded => view! {
                    <div style="
                        position: absolute; top: 0; left: 0; right: 0; bottom: 0;
                        display: flex; flex-direction: column; align-items: center; justify-content: center;
                        background: rgba(6,14,10,0.95); color: white; cursor: pointer; text-align: center; padding: 24px;
                    " on:click=start_loading>
                        <div style="font-size: 14px; text-transform: uppercase; letter-spacing: 4px; color: #8ee07a; margin-bottom: 12px;">
                            "Schweinestall \u{2192} Gew\u{e4}chshaus"
                        </div>
                        <div style="font-size: 28px; font-weight: 300; margin-bottom: 8px;">
                            "Greenhouse Spectral Lab"
                        </div>
                        <div style="font-size: 14px; color: #8a9a88; margin-bottom: 8px; max-width: 640px; line-height: 1.5;">
                            "An old M\u{fc}nsterland pig barn rebuilt as a glasshouse: real sun position and sky, "
                            "spectral glazing, TM-33 grow-light spectra, canopy PPFD \u{2014} and a spectral "
                            "Monte Carlo trace of the whole thing, in your browser."
                        </div>
                        <div style="font-size: 12px; color: #6a7a68; margin-bottom: 32px;">
                            "Site: L\u{fc}dinghausen \u{b7} Daylight: eulumdat-daylight \u{b7} Spectra: eulumdat-spectrum \u{b7} Tracer: eulumdat-goniosim"
                        </div>
                        <button style="
                            padding: 14px 36px; font-size: 16px;
                            background: transparent; color: #8ee07a;
                            border: 1px solid #8ee07a; border-radius: 4px;
                            cursor: pointer; letter-spacing: 1px;
                            transition: all 0.2s;
                        ">"Launch Demo"</button>
                        <div style="margin-top: 24px; font-size: 12px; color: #555;">
                            "~25 MB download \u{b7} WebGPU required (Chrome / Edge / Safari 26+)"
                        </div>
                    </div>
                }.into_any(),

                LoadState::Loading => view! {
                    <div style="
                        position: absolute; top: 0; left: 0; right: 0; bottom: 0;
                        display: flex; flex-direction: column; align-items: center; justify-content: center;
                        background: rgba(6,14,10,0.95); color: white;
                    ">
                        <div class="spinner" style="
                            width: 40px; height: 40px; border: 2px solid #223;
                            border-top-color: #8ee07a; border-radius: 50%;
                            animation: spin 1s linear infinite;
                        "></div>
                        <div style="margin-top: 20px; font-size: 16px; color: #aaa;">
                            "Loading Greenhouse Demo..."
                        </div>
                        <div style="margin-top: 8px; font-size: 12px; color: #555;">
                            "Probing WebGPU and downloading the Bevy bundle"
                        </div>
                    </div>
                    <style>"@keyframes spin { to { transform: rotate(360deg); } }"</style>
                }.into_any(),

                LoadState::Loaded => view! {
                    <div style="
                        position: absolute; bottom: 12px; left: 50%; transform: translateX(-50%);
                        background: rgba(0,0,0,0.5); color: #888; padding: 6px 14px;
                        border-radius: 4px; font-size: 11px; pointer-events: none; white-space: nowrap;
                    ">
                        "WASD: move \u{b7} Right-drag: look \u{b7} [ ]: hour \u{b7} K: sky \u{b7} G: glazing \u{b7} 1-4/0: lamps \u{b7} T: heat-map \u{b7} M: Monte Carlo \u{b7} R: reset"
                    </div>
                }.into_any(),

                LoadState::Error => view! {
                    <div style="
                        position: absolute; top: 0; left: 0; right: 0; bottom: 0;
                        display: flex; flex-direction: column; align-items: center; justify-content: center;
                        background: rgba(6,14,10,0.95); color: white;
                    ">
                        <div style="font-size: 18px; color: #ff6b6b; margin-bottom: 12px;">
                            "Failed to load Greenhouse Demo"
                        </div>
                        <div style="font-size: 12px; color: #666; max-width: 400px; text-align: center;">
                            {move || error_msg.get().unwrap_or_default()}
                        </div>
                        <button
                            style="
                                margin-top: 20px; padding: 10px 24px; font-size: 14px;
                                background: transparent; color: #8ee07a;
                                border: 1px solid #8ee07a; border-radius: 4px; cursor: pointer;
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
