//! Bevy 3D Scene Viewer Component
//!
//! This component lazy-loads the Bevy 3D viewer when the user opens the 3D Scene tab.
//! The Bevy WASM (~22MB) is loaded on demand, keeping initial app load fast (~2MB).

use leptos::prelude::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = loadBevyViewer)]
    fn load_bevy_viewer() -> js_sys::Promise;

    #[wasm_bindgen(js_name = isBevyLoaded)]
    fn is_bevy_loaded() -> bool;

    #[wasm_bindgen(js_name = isBevyLoading)]
    fn is_bevy_loading() -> bool;
}

/// Loading state for the Bevy viewer
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BevyLoadState {
    NotLoaded,
    Loading,
    Loaded,
    Error,
}

/// Bevy 3D Scene Viewer component
///
/// Renders a canvas and lazy-loads the Bevy WASM when mounted.
/// The Bevy app syncs with the editor via localStorage.
#[component]
pub fn BevySceneViewer() -> impl IntoView {
    let (load_state, set_load_state) = signal(BevyLoadState::NotLoaded);
    let (error_msg, set_error_msg) = signal::<Option<String>>(None);

    // Check if already loaded on mount
    Effect::new(move |_| {
        if is_bevy_loaded() {
            set_load_state.set(BevyLoadState::Loaded);
        } else if is_bevy_loading() {
            set_load_state.set(BevyLoadState::Loading);
        }
    });

    // Trigger loading
    let start_loading = move |_| {
        if load_state.get() != BevyLoadState::NotLoaded {
            return;
        }

        set_load_state.set(BevyLoadState::Loading);

        wasm_bindgen_futures::spawn_local(async move {
            let promise = load_bevy_viewer();
            let result = wasm_bindgen_futures::JsFuture::from(promise).await;

            match result {
                Ok(_) => {
                    set_load_state.set(BevyLoadState::Loaded);
                }
                Err(e) => {
                    let msg = format!("{:?}", e);
                    web_sys::console::error_1(&format!("Bevy load error: {}", msg).into());
                    set_error_msg.set(Some(msg));
                    set_load_state.set(BevyLoadState::Error);
                }
            }
        });
    };

    view! {
        <div class="bevy-scene-container" style="width: 100%; height: 100%; position: relative;">
            // Canvas for Bevy to render into
            <canvas
                id="bevy-canvas"
                style="width: 100%; height: 100%; display: block; touch-action: none;"
            />

            // Loading overlay
            {move || match load_state.get() {
                BevyLoadState::NotLoaded => view! {
                    <div class="bevy-overlay" style="
                        position: absolute; top: 0; left: 0; right: 0; bottom: 0;
                        display: flex; flex-direction: column; align-items: center; justify-content: center;
                        background: rgba(0,0,0,0.8); color: white; cursor: pointer;
                    " on:click=start_loading>
                        <div style="font-size: 48px; margin-bottom: 20px;">"🎮"</div>
                        <div style="font-size: 18px; margin-bottom: 10px;">"3D Scene Viewer"</div>
                        <div style="font-size: 14px; color: #aaa; margin-bottom: 20px;">"Click to load (~22MB)"</div>
                        <button style="
                            padding: 12px 24px; font-size: 16px;
                            background: #4a9eff; color: white; border: none;
                            border-radius: 8px; cursor: pointer;
                        ">"Load 3D Viewer"</button>
                    </div>
                }.into_any(),

                BevyLoadState::Loading => view! {
                    <div class="bevy-overlay" style="
                        position: absolute; top: 0; left: 0; right: 0; bottom: 0;
                        display: flex; flex-direction: column; align-items: center; justify-content: center;
                        background: rgba(0,0,0,0.8); color: white;
                    ">
                        <div class="spinner" style="
                            width: 50px; height: 50px; border: 4px solid #333;
                            border-top-color: #4a9eff; border-radius: 50%;
                            animation: spin 1s linear infinite;
                        "></div>
                        <div style="margin-top: 20px; font-size: 16px;">"Loading 3D Viewer..."</div>
                        <div style="margin-top: 10px; font-size: 12px; color: #888;">"Downloading ~22MB"</div>
                    </div>
                    <style>"@keyframes spin { to { transform: rotate(360deg); } }"</style>
                }.into_any(),

                BevyLoadState::Loaded => view! {
                    // Bevy is rendering, canvas is visible
                    <div></div>
                }.into_any(),

                BevyLoadState::Error => view! {
                    <div class="bevy-overlay" style="
                        position: absolute; top: 0; left: 0; right: 0; bottom: 0;
                        display: flex; flex-direction: column; align-items: center; justify-content: center;
                        background: rgba(0,0,0,0.8); color: white;
                    ">
                        <div style="font-size: 48px; margin-bottom: 20px;">"❌"</div>
                        <div style="font-size: 18px; color: #ff6b6b;">"Failed to load 3D Viewer"</div>
                        <div style="margin-top: 10px; font-size: 12px; color: #888; max-width: 400px; text-align: center;">
                            {move || error_msg.get().unwrap_or_default()}
                        </div>
                        <button
                            style="
                                margin-top: 20px; padding: 12px 24px; font-size: 16px;
                                background: #4a9eff; color: white; border: none;
                                border-radius: 8px; cursor: pointer;
                            "
                            on:click=move |_| {
                                set_load_state.set(BevyLoadState::NotLoaded);
                                set_error_msg.set(None);
                            }
                        >"Try Again"</button>
                    </div>
                }.into_any(),
            }}
        </div>
    }
}

/// Standalone Bevy scene for when iframe is still preferred
#[component]
pub fn BevySceneIframe(
    #[prop(default = 0)] reload_key: u32,
) -> impl IntoView {
    view! {
        <div class="scene-container">
            <iframe
                src=move || format!("bevy/index.html?r={}", reload_key)
                style="width: 100%; height: 600px; border: none; border-radius: 8px;"
                title="3D Scene Viewer"
            />
        </div>
    }
}
