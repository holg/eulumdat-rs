//! Street designer — lazy-loaded companion to the main eulumdat WASM editor.
//!
//! Exposes a single `mount()` entry point (called by `street-loader.js`) that
//! attaches a Leptos app to `#street-root` in the host page.
//!
//! The UI is intentionally minimal in this first commit: a file picker, a
//! [`StreetLayout`](eulumdat::street::StreetLayout) form, and a compliance
//! panel that cycles through MLO / RP-8 / EN 13201 / CJJ 45 against the
//! computed design result. Heatmap, multi-LDT catalog support, and polished
//! layout visualization come in follow-up commits.

mod app;

pub use app::StreetApp;

use wasm_bindgen::prelude::*;

/// JS entry point — `street-loader.js` calls this after fetching the module.
#[wasm_bindgen]
pub fn mount() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::StreetApp);
}
