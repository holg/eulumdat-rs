//! Street designer — lazy-loaded companion to the main eulumdat WASM editor.
//!
//! Exposes a single `mount()` entry point (called by `street-loader.js`) that
//! attaches a Leptos app to `#street-root` in the host page.
//!
//! The luminaire comes exclusively from the main editor via localStorage
//! — there is no separate picker here. The UI is a
//! [`StreetLayout`](eulumdat::street::StreetLayout) form, a plan-view SVG
//! heatmap with optional failure-overlay, and a compliance panel that
//! cycles through MLO / RP-8 / EN 13201 / CJJ 45.

mod app;
mod download;
mod i18n;
mod storage_sync;

pub use app::StreetApp;

use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

thread_local! {
    /// The app's own container: created and mounted once, then moved into
    /// whichever `#street-root` the Street Design tab shows, so the design
    /// survives leaving the tab (e.g. for the Light scene) and coming back.
    static HOST: RefCell<Option<web_sys::HtmlElement>> = const { RefCell::new(None) };
}

/// JS entry point — `street-loader.js` calls this after fetching the module,
/// and again each time the Street Design tab is shown.
#[wasm_bindgen]
pub fn mount() {
    console_error_panic_hook::set_once();
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    HOST.with(|h| {
        let mut h = h.borrow_mut();
        if h.is_none() {
            let Some(host) = doc
                .create_element("div")
                .ok()
                .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
            else {
                return;
            };
            host.set_class_name("street-app-host");
            leptos::mount::mount_to(host.clone(), app::StreetApp).forget();
            *h = Some(host);
        }
        if let Some(host) = h.as_ref() {
            attach(host.clone(), 0);
        }
    });
}

/// Move the app into the tab's `#street-root` (it used to go to the end of
/// `<body>`, where the tab never showed it). The tab may still be building
/// its view when the loader calls `mount()`: then try again shortly.
fn attach(host: web_sys::HtmlElement, attempt: u32) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let root = window
        .document()
        .and_then(|d| d.get_element_by_id("street-root"));
    match root {
        Some(root) => {
            if host.parent_element().as_ref() != Some(&root) {
                let _ = root.append_child(&host);
            }
        }
        None if attempt < 60 => {
            let retry =
                wasm_bindgen::closure::Closure::once_into_js(move || attach(host, attempt + 1));
            let _ = window
                .set_timeout_with_callback_and_timeout_and_arguments_0(retry.unchecked_ref(), 50);
        }
        None => {}
    }
}
