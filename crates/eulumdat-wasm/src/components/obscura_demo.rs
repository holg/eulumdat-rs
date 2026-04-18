//! Obscura Demo Component
//!
//! Fullscreen lazy-loaded Bevy WASM app for the Darkness Preservation Simulator.
//! Activated via `?wasm=obscura_demo` query parameter.

use leptos::prelude::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = loadObscuraDemo)]
    fn load_obscura_demo() -> js_sys::Promise;

    #[wasm_bindgen(js_name = isObscuraLoaded)]
    fn is_obscura_loaded() -> bool;

    #[wasm_bindgen(js_name = isObscuraLoading)]
    fn is_obscura_loading() -> bool;
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LoadState {
    NotLoaded,
    Loading,
    Loaded,
    Error,
}

#[component]
pub fn ObscuraDemo() -> impl IntoView {
    let (load_state, set_load_state) = signal(LoadState::NotLoaded);
    let (error_msg, set_error_msg) = signal::<Option<String>>(None);

    // Check if already loaded on mount
    Effect::new(move |_| {
        if is_obscura_loaded() {
            set_load_state.set(LoadState::Loaded);
        } else if is_obscura_loading() {
            set_load_state.set(LoadState::Loading);
        }
    });

    let start_loading = move |_| {
        if load_state.get() != LoadState::NotLoaded {
            return;
        }

        set_load_state.set(LoadState::Loading);

        wasm_bindgen_futures::spawn_local(async move {
            let promise = load_obscura_demo();
            let result = wasm_bindgen_futures::JsFuture::from(promise).await;

            match result {
                Ok(_) => {
                    set_load_state.set(LoadState::Loaded);
                }
                Err(e) => {
                    let msg = format!("{:?}", e);
                    web_sys::console::error_1(&format!("Obscura load error: {}", msg).into());
                    set_error_msg.set(Some(msg));
                    set_load_state.set(LoadState::Error);
                }
            }
        });
    };

    view! {
        <div style="width: 100vw; height: 100vh; position: fixed; top: 0; left: 0; background: #0a0a1a; overflow: hidden;">
            <canvas
                id="obscura-canvas"
                style="width: 100%; height: 100%; display: block; touch-action: none;"
            />

            {move || match load_state.get() {
                LoadState::NotLoaded => view! {
                    <div style="
                        position: absolute; top: 0; left: 0; right: 0; bottom: 0;
                        display: flex; flex-direction: column; align-items: center; justify-content: center;
                        background: rgba(5,5,20,0.95); color: white; cursor: pointer;
                    " on:click=start_loading>
                        <div style="font-size: 14px; text-transform: uppercase; letter-spacing: 4px; color: #6ab0ff; margin-bottom: 12px;">
                            "Obscura"
                        </div>
                        <div style="font-size: 28px; font-weight: 300; margin-bottom: 8px;">
                            "Darkness Preservation Simulator"
                        </div>
                        <div style="font-size: 14px; color: #777; margin-bottom: 32px;">
                            "Interactive light pollution analysis with photometric data"
                        </div>
                        <button style="
                            padding: 14px 36px; font-size: 16px;
                            background: transparent; color: #6ab0ff;
                            border: 1px solid #6ab0ff; border-radius: 4px;
                            cursor: pointer; letter-spacing: 1px;
                            transition: all 0.2s;
                        ">"Launch Demo"</button>
                        <div style="margin-top: 24px; font-size: 12px; color: #555;">
                            "Bistro Exterior scene \u{00b7} ~25 MB download"
                        </div>
                    </div>
                }.into_any(),

                LoadState::Loading => view! {
                    <div style="
                        position: absolute; top: 0; left: 0; right: 0; bottom: 0;
                        display: flex; flex-direction: column; align-items: center; justify-content: center;
                        background: rgba(5,5,20,0.95); color: white;
                    ">
                        <div class="spinner" style="
                            width: 40px; height: 40px; border: 2px solid #222;
                            border-top-color: #6ab0ff; border-radius: 50%;
                            animation: spin 1s linear infinite;
                        "></div>
                        <div style="margin-top: 20px; font-size: 16px; color: #aaa;">
                            "Loading Obscura Demo..."
                        </div>
                        <div style="margin-top: 8px; font-size: 12px; color: #555;">
                            "Downloading scene assets and Bevy runtime"
                        </div>
                    </div>
                    <style>"@keyframes spin { to { transform: rotate(360deg); } }"</style>
                }.into_any(),

                LoadState::Loaded => view! {
                    <div style="
                        position: absolute; bottom: 12px; left: 50%; transform: translateX(-50%);
                        background: rgba(0,0,0,0.5); color: #888; padding: 6px 14px;
                        border-radius: 4px; font-size: 11px; pointer-events: none;
                    ">
                        "WASD: move \u{00b7} Right-click: look \u{00b7} Space: toggle mode \u{00b7} R: reset"
                    </div>
                }.into_any(),

                LoadState::Error => view! {
                    <div style="
                        position: absolute; top: 0; left: 0; right: 0; bottom: 0;
                        display: flex; flex-direction: column; align-items: center; justify-content: center;
                        background: rgba(5,5,20,0.95); color: white;
                    ">
                        <div style="font-size: 18px; color: #ff6b6b; margin-bottom: 12px;">
                            "Failed to load Obscura Demo"
                        </div>
                        <div style="font-size: 12px; color: #666; max-width: 400px; text-align: center;">
                            {move || error_msg.get().unwrap_or_default()}
                        </div>
                        <button
                            style="
                                margin-top: 20px; padding: 10px 24px; font-size: 14px;
                                background: transparent; color: #6ab0ff;
                                border: 1px solid #6ab0ff; border-radius: 4px; cursor: pointer;
                            "
                            on:click=move |_| {
                                set_load_state.set(LoadState::NotLoaded);
                                set_error_msg.set(None);
                            }
                        >"Try Again"</button>
                    </div>
                }.into_any(),
            }}
        </div>
    }
}
