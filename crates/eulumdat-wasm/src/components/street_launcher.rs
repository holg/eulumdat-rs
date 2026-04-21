//! Lazy-load launcher for the Street Designer companion app.
//!
//! Renders a launch card that becomes a "load now" button on first click,
//! then transitions to a live mount point for the `eulumdat-wasm-street`
//! app. Mirrors the pattern used by the Bevy 3D viewer launcher.

use leptos::prelude::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = loadStreetDesigner)]
    fn load_street_designer() -> js_sys::Promise;

    #[wasm_bindgen(js_name = isStreetDesignerLoaded)]
    fn is_street_designer_loaded() -> bool;
}

/// The Street Design tab body. Shows an intro card while the companion
/// bundle is unloaded; after a successful lazy-load, the `#street-root`
/// mount point receives the full multi-standard compliance UI.
#[component]
pub fn StreetLauncher() -> impl IntoView {
    // Initialize from window state so hot-reloads / re-renders don't force
    // the user to click again if the bundle is already in memory.
    let initial = is_street_designer_loaded();
    let (loaded, set_loaded) = signal(initial);
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
                    set_loading.set(false);
                    set_loaded.set(true);
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
            // Pre-load intro card — hidden after the bundle mounts.
            {move || (!loaded.get()).then(|| view! {
                <div class="street-launcher-intro">
                    <h3>"Multi-luminaire street lighting compliance"</h3>
                    <ul class="street-launcher-regions">
                        <li><strong>"🇺🇸 US"</strong> " — ANSI/IES RP-8 (illuminance method)"</li>
                        <li><strong>"🇪🇺 EU"</strong> " — DIN EN 13201-2 (C- and P-classes)"</li>
                        <li><strong>"🇨🇳 CN"</strong> " — CJJ 45-2015 (Classes I–IV)"</li>
                        <li><strong>"🌌 MLO"</strong> " — IES/IDA Model Lighting Ordinance (dark-sky)"</li>
                    </ul>
                    <p class="text-muted">
                        "The designer runs in a separate on-demand WASM bundle so the \
                         main editor stays lean. First load fetches the module once; \
                         subsequent opens are instant."
                    </p>
                    <button
                        class="btn btn-primary street-launcher-button"
                        on:click=on_click
                        disabled=move || loading.get()
                    >
                        {move || if loading.get() {
                            "Loading Street Designer…".to_string()
                        } else {
                            "Launch Street Designer →".to_string()
                        }}
                    </button>
                    {move || error.get().map(|e| view! {
                        <p class="street-launcher-error">{format!("Failed to load: {e}")}</p>
                    })}
                </div>
            })}

            // Mount point the companion app attaches to. Always present in the
            // DOM so the dynamic import can find it; styled hidden until loaded.
            <div
                id="street-root"
                class=move || if loaded.get() { "street-root visible" } else { "street-root" }
            ></div>
        </div>
    }
}
