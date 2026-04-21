//! Lazy-load launcher for the Street Designer companion app.
//!
//! Renders a "Street Design →" button. On click, invokes
//! `window.loadStreetDesigner()` (from `street-loader.js`) which fetches the
//! separate `eulumdat-wasm-street` WASM bundle and mounts it into `#street-root`.
//!
//! Mirrors the pattern used by the Bevy 3D viewer launcher.

use leptos::prelude::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = loadStreetDesigner)]
    fn load_street_designer() -> js_sys::Promise;

    #[wasm_bindgen(js_name = isStreetDesignerLoaded)]
    fn is_street_designer_loaded() -> bool;
}

/// Renders the "Street Design" launch button. Clicking it fetches the
/// companion WASM bundle (no-op if already loaded) and reveals the mount
/// point where the designer attaches.
#[component]
pub fn StreetLauncher() -> impl IntoView {
    let (loaded, set_loaded) = signal(false);
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(None::<String>);

    let on_click = move |_| {
        if loaded.get() || loading.get() {
            return;
        }
        set_loading.set(true);
        set_error.set(None);
        let promise = load_street_designer();
        let future = wasm_bindgen_futures::JsFuture::from(promise);
        wasm_bindgen_futures::spawn_local(async move {
            match future.await {
                Ok(_) => {
                    set_loaded.set(true);
                    set_loading.set(false);
                }
                Err(e) => {
                    set_loading.set(false);
                    set_error.set(Some(format!("{e:?}")));
                }
            }
        });
    };

    view! {
        <div class="street-launcher">
            <button
                on:click=on_click
                disabled=move || loading.get() || loaded.get()
            >
                {move || {
                    if loaded.get() {
                        "✓ Street Designer loaded".to_string()
                    } else if loading.get() {
                        "Loading Street Designer…".to_string()
                    } else {
                        "Street Design →".to_string()
                    }
                }}
            </button>
            {move || error.get().map(|e| view! {
                <p style="color: #a83232;">{format!("Failed to load: {e}")}</p>
            })}
            // Mount point the companion app attaches to.
            <div id="street-root"></div>
        </div>
    }
}
