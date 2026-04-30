//! Skyglow Demo Component
//!
//! Fullscreen lazy-loaded Bevy WASM app for the Darkness Preservation Simulator.
//! Activated via `?wasm=skyglow_demo` (or the legacy `?wasm=obscura_demo` alias).
//!
//! # WebGPU + WebGL2 dual-bundle
//!
//! Bevy renders via WebGPU. WebGPU coverage is uneven — Linux Chrome
//! blocklists many GPU/driver combos, Linux Firefox stable usually
//! doesn't ship the runtime, iPad Safari hides it behind a feature
//! flag. To stop those users seeing a blank canvas, the build script
//! ships **two** Bevy bundles:
//!
//! - `dist/skyglow/`        — WebGPU primary (full fidelity: Bloom, IBL,
//!                            photometric IES storage textures)
//! - `dist/skyglow-webgl2/` — WebGL2 fallback (degraded: no Bloom, no
//!                            IBL, simpler clustered lighting)
//!
//! `skyglow-loader.js` probes `navigator.gpu.requestAdapter()` and loads
//! whichever bundle matches the user's browser. It sets
//! `window.skyglowBackend` to `'webgpu' | 'webgl2' | 'unsupported'`,
//! which this component reads to decide whether to show the
//! "reduced fidelity" banner or the "your browser can't render this
//! at all" screen.

use leptos::prelude::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = loadSkyglowDemo)]
    fn load_skyglow_demo() -> js_sys::Promise;

    #[wasm_bindgen(js_name = isSkyglowLoaded)]
    fn is_skyglow_loaded() -> bool;

    #[wasm_bindgen(js_name = isSkyglowLoading)]
    fn is_skyglow_loading() -> bool;
}

/// Read `window.skyglowBackend` set by skyglow-loader.js.
fn read_backend() -> Backend {
    let Some(window) = web_sys::window() else {
        return Backend::Unknown;
    };
    let v = match js_sys::Reflect::get(&window, &JsValue::from_str("skyglowBackend")) {
        Ok(v) => v,
        Err(_) => return Backend::Unknown,
    };
    match v.as_string().as_deref() {
        Some("webgpu") => Backend::WebGpu,
        Some("webgl2") => Backend::WebGl2,
        Some("unsupported") => Backend::Unsupported,
        _ => Backend::Unknown,
    }
}

/// True if `?force=webgl2` (or any non-default forced backend) is in the
/// URL. Drives the banner text — "you're seeing the fallback because
/// WebGPU was unavailable" vs "you're seeing the fallback because you
/// asked for it".
fn force_param_active() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let Ok(search) = window.location().search() else {
        return false;
    };
    // Cheap parse: look for `force=webgl2` or `force=webgpu` substring.
    // URLSearchParams isn't worth pulling for this.
    let s = search.to_lowercase();
    s.contains("force=webgl2") || s.contains("force=webgpu")
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Backend {
    /// Loader hasn't picked yet (still probing).
    Unknown,
    /// Primary WebGPU bundle loaded — no banner needed.
    WebGpu,
    /// WebGL2 fallback bundle loaded — show "reduced fidelity" banner.
    WebGl2,
    /// Neither bundle could be loaded — show the missing-WebGPU screen.
    Unsupported,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LoadState {
    NotLoaded,
    Loading,
    Loaded,
    Error,
    /// Loader's probe found neither WebGPU nor a working WebGL2 bundle.
    /// Show the same screen we used to show on raw probe failure.
    Unsupported,
}

/// Best-effort browser + platform detection from the user-agent string.
#[derive(Clone, Copy, PartialEq, Eq)]
enum BrowserHint {
    SafariIos,
    SafariMac,
    ChromiumLinux,
    Chromium,
    FirefoxLinux,
    Firefox,
    Unknown,
}

fn detect_browser() -> BrowserHint {
    let Some(window) = web_sys::window() else {
        return BrowserHint::Unknown;
    };
    let Ok(ua) = window.navigator().user_agent() else {
        return BrowserHint::Unknown;
    };
    let ua = ua.to_lowercase();
    let max_touch_points = window.navigator().max_touch_points();
    let looks_like_ipad = ua.contains("ipad") || (ua.contains("macintosh") && max_touch_points > 1);
    let is_linux = ua.contains("linux") && !ua.contains("android");

    if ua.contains("crios") || ua.contains("fxios") {
        return BrowserHint::SafariIos;
    }
    if ua.contains("iphone") || looks_like_ipad {
        return BrowserHint::SafariIos;
    }
    if ua.contains("safari") && !ua.contains("chrome") && !ua.contains("chromium") {
        return BrowserHint::SafariMac;
    }
    if ua.contains("firefox") {
        return if is_linux {
            BrowserHint::FirefoxLinux
        } else {
            BrowserHint::Firefox
        };
    }
    if ua.contains("chrome") || ua.contains("chromium") || ua.contains("edg/") {
        return if is_linux {
            BrowserHint::ChromiumLinux
        } else {
            BrowserHint::Chromium
        };
    }
    BrowserHint::Unknown
}

#[component]
pub fn SkyglowDemo() -> impl IntoView {
    let (load_state, set_load_state) = signal(LoadState::NotLoaded);
    let (backend, set_backend) = signal(Backend::Unknown);
    let (error_msg, set_error_msg) = signal::<Option<String>>(None);
    let browser = StoredValue::new(detect_browser());

    Effect::new(move |_| {
        if is_skyglow_loaded() {
            set_load_state.set(LoadState::Loaded);
            set_backend.set(read_backend());
        } else if is_skyglow_loading() {
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
            let promise = load_skyglow_demo();
            let result = wasm_bindgen_futures::JsFuture::from(promise).await;

            // The loader sets window.skyglowBackend before/after
            // import — read it now to know which bundle ran.
            let resolved = read_backend();
            set_backend.set(resolved);

            match result {
                Ok(_) => {
                    if matches!(resolved, Backend::Unsupported) {
                        // Loader gave up before importing anything —
                        // probably missing-WebGPU + no WebGL2 fallback
                        // bundle on disk in dev mode.
                        set_load_state.set(LoadState::Unsupported);
                    } else {
                        set_load_state.set(LoadState::Loaded);
                    }
                }
                Err(e) => {
                    if matches!(resolved, Backend::Unsupported) {
                        set_load_state.set(LoadState::Unsupported);
                    } else {
                        let msg = format!("{:?}", e);
                        web_sys::console::error_1(&format!("Skyglow load error: {}", msg).into());
                        set_error_msg.set(Some(msg));
                        set_load_state.set(LoadState::Error);
                    }
                }
            }
        });
    };

    view! {
        <div style="width: 100vw; height: 100vh; position: fixed; top: 0; left: 0; background: #0a0a1a; overflow: hidden;">
            <canvas
                id="skyglow-canvas"
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
                            "Skyglow"
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
                            "Bistro Exterior scene \u{00b7} ~25 MB download \u{00b7} WebGPU recommended (WebGL2 fallback available)"
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
                            "Loading Skyglow Demo..."
                        </div>
                        <div style="margin-top: 8px; font-size: 12px; color: #555;">
                            "Probing WebGPU and downloading scene assets"
                        </div>
                    </div>
                    <style>"@keyframes spin { to { transform: rotate(360deg); } }"</style>
                }.into_any(),

                LoadState::Loaded => view! {
                    {move || {
                        let backend_now = backend.get();
                        let is_fallback = matches!(backend_now, Backend::WebGl2);
                        let forced = force_param_active();
                        view! {
                            // Reduced-fidelity banner — top of viewport, only
                            // shown when the loader picked the WebGL2 bundle.
                            // Banner text adapts depending on whether we
                            // landed here automatically (WebGPU genuinely
                            // missing) or via ?force=webgl2 (preview mode).
                            {is_fallback.then(|| {
                                let (lead, suffix) = if forced {
                                    (
                                        "WebGL2 fallback (forced via URL)",
                                        " · Reduced fidelity: no Bloom, no IBL. ",
                                    )
                                } else {
                                    (
                                        "Reduced-quality fallback (WebGL2)",
                                        " · No Bloom, no IBL, simpler lighting. ",
                                    )
                                };
                                view! {
                                    <div style="
                                        position: absolute; top: 8px; left: 8px; right: 8px;
                                        padding: 8px 12px;
                                        background: rgba(120, 80, 0, 0.85); color: #ffe0a0;
                                        border: 1px solid rgba(255, 200, 80, 0.6);
                                        border-radius: 4px;
                                        font-size: 12px; line-height: 1.4;
                                        pointer-events: auto; z-index: 5;
                                    ">
                                        <strong>{lead}</strong>
                                        {suffix}
                                        <a
                                            href="?wasm=skyglow_demo"
                                            style="color: #ffe0a0; text-decoration: underline;"
                                        >"Switch to WebGPU"</a>
                                        " (auto-detect) · "
                                        <a
                                            href="https://caniuse.com/webgpu"
                                            target="_blank"
                                            rel="noopener noreferrer"
                                            style="color: #ffe0a0; text-decoration: underline;"
                                        >"WebGPU support info"</a>
                                    </div>
                                }
                            })}
                            // Bottom controls hint.
                            <div style="
                                position: absolute; bottom: 12px; left: 50%; transform: translateX(-50%);
                                background: rgba(0,0,0,0.5); color: #888; padding: 6px 14px;
                                border-radius: 4px; font-size: 11px; pointer-events: none;
                            ">
                                "WASD: move \u{00b7} Right-click: look \u{00b7} Space: toggle mode \u{00b7} R: reset"
                            </div>
                        }
                    }}
                }.into_any(),

                LoadState::Error => view! {
                    <div style="
                        position: absolute; top: 0; left: 0; right: 0; bottom: 0;
                        display: flex; flex-direction: column; align-items: center; justify-content: center;
                        background: rgba(5,5,20,0.95); color: white;
                    ">
                        <div style="font-size: 18px; color: #ff6b6b; margin-bottom: 12px;">
                            "Failed to load Skyglow Demo"
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

                LoadState::Unsupported => render_webgpu_missing(browser.get_value()).into_any(),
            }}
        </div>
    }
}

/// Browser-aware "WebGPU not available AND WebGL2 fallback also failed"
/// screen. With the dual-bundle setup this is now a rare last resort —
/// most users land on the WebGL2 bundle instead.
fn render_webgpu_missing(hint: BrowserHint) -> impl IntoView {
    let (title, steps, more_url) = match hint {
        BrowserHint::SafariIos => (
            "WebGPU isn't enabled on this device",
            vec![
                "Open the Settings app".to_string(),
                "Tap Safari → Advanced → Feature Flags".to_string(),
                "Turn on WebGPU".to_string(),
                "Reload this page".to_string(),
            ],
            "https://developer.apple.com/safari/technology-preview/release-notes/",
        ),
        BrowserHint::SafariMac => (
            "WebGPU isn't enabled in this Safari",
            vec![
                "Update macOS to Tahoe (26) or later — WebGPU ships on by default there".to_string(),
                "Or: Safari → Settings → Advanced → Show features for web developers".to_string(),
                "Then: Develop → Feature Flags → enable WebGPU".to_string(),
                "Reload this page".to_string(),
            ],
            "https://webkit.org/blog/16401/webgpu-now-available-for-testing-in-safari-technology-preview/",
        ),
        BrowserHint::ChromiumLinux => (
            "Neither WebGPU nor WebGL2 worked on this Linux setup",
            vec![
                "Try chrome://gpu — look for 'WebGL2: Hardware accelerated' and 'WebGPU: Hardware accelerated'".to_string(),
                "Update your Mesa / NVIDIA / AMD drivers — old drivers block both APIs".to_string(),
                "Wayland sessions sometimes block GPU access — try an X11 session as a test".to_string(),
                "Or: chrome://flags → enable Unsafe WebGPU → restart".to_string(),
            ],
            "https://chromium.googlesource.com/chromium/src/+/main/docs/gpu/webgpu_status.md",
        ),
        BrowserHint::Chromium => (
            "WebGPU and WebGL2 both unavailable in this browser",
            vec![
                "Update Chrome / Edge to the latest stable".to_string(),
                "Check chrome://gpu for 'WebGL2: Hardware accelerated'".to_string(),
                "Or: chrome://flags → enable Unsafe WebGPU → restart".to_string(),
                "Reload this page".to_string(),
            ],
            "https://caniuse.com/webgpu",
        ),
        BrowserHint::FirefoxLinux => (
            "Neither WebGPU nor WebGL2 worked in this Firefox",
            vec![
                "Most stable Firefox builds on Linux don't ship the WebGPU runtime — try Firefox Nightly".to_string(),
                "Or: about:config → set webgl.force-enabled to true (forces WebGL2)".to_string(),
                "Or use Chrome / Edge / Brave on the same Linux machine".to_string(),
            ],
            "https://hacks.mozilla.org/2025/02/firefox-on-webgpu/",
        ),
        BrowserHint::Firefox => (
            "WebGPU isn't enabled in this Firefox",
            vec![
                "Visit about:config in a new tab".to_string(),
                "Search for dom.webgpu.enabled".to_string(),
                "Set it to true".to_string(),
                "Reload this page".to_string(),
            ],
            "https://hacks.mozilla.org/2025/02/firefox-on-webgpu/",
        ),
        BrowserHint::Unknown => (
            "WebGPU is unavailable in this browser",
            vec![
                "Try a recent Chrome, Edge, or Brave on Windows or macOS".to_string(),
                "On macOS Tahoe (26+) Safari also works".to_string(),
                "Linux: needs a Vulkan-capable GPU + recent Mesa drivers".to_string(),
            ],
            "https://caniuse.com/webgpu",
        ),
    };

    let alt_url = "/?wasm=goniosim";

    view! {
        <div style="
            position: absolute; top: 0; left: 0; right: 0; bottom: 0;
            display: flex; flex-direction: column; align-items: center; justify-content: center;
            background: rgba(5,5,20,0.97); color: white; padding: 24px;
            text-align: center;
        ">
            <div style="font-size: 14px; text-transform: uppercase; letter-spacing: 4px; color: #6ab0ff; margin-bottom: 12px;">
                "Skyglow"
            </div>
            <div style="font-size: 22px; font-weight: 300; margin-bottom: 24px; max-width: 600px;">
                {title}
            </div>
            <div style="font-size: 13px; color: #888; max-width: 500px; margin-bottom: 20px; line-height: 1.6;">
                "This 3D demo needs " <strong style="color: #aaa">"WebGPU"</strong>
                " (preferred) or " <strong style="color: #aaa">"WebGL2"</strong>
                ". Neither was available in your browser/GPU/driver combination."
            </div>
            <ol style="
                text-align: left; max-width: 500px; margin: 0 0 24px;
                padding-left: 20px; color: #ccc; font-size: 14px; line-height: 1.8;
            ">
                {steps.into_iter().map(|s| view! { <li>{s}</li> }).collect_view()}
            </ol>
            <div style="display: flex; gap: 12px; flex-wrap: wrap; justify-content: center;">
                <a
                    href=more_url
                    target="_blank"
                    rel="noopener noreferrer"
                    style="
                        padding: 10px 22px; font-size: 13px;
                        background: transparent; color: #6ab0ff;
                        border: 1px solid #6ab0ff; border-radius: 4px;
                        text-decoration: none; letter-spacing: 0.5px;
                    "
                >"Learn more"</a>
                <a
                    href=alt_url
                    style="
                        padding: 10px 22px; font-size: 13px;
                        background: transparent; color: #888;
                        border: 1px solid #555; border-radius: 4px;
                        text-decoration: none; letter-spacing: 0.5px;
                    "
                >"Try the 2D goniosim demo instead"</a>
            </div>
            <div style="margin-top: 28px; font-size: 11px; color: #555;">
                "The rest of the eulumdat editor (file open, diagrams, validation, street designer) works fine on every browser."
            </div>
        </div>
    }
}
