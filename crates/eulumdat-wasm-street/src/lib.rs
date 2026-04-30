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

use wasm_bindgen::prelude::*;

/// JS entry point — `street-loader.js` calls this after fetching the module.
#[wasm_bindgen]
pub fn mount() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::StreetApp);
}
