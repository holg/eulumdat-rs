//! localStorage bridge to the main eulumdat editor.
//!
//! The editor writes the current light distribution (LDC) to `localStorage`
//! under the key `eulumdat_current_ldt` (the key name kept for compatibility
//! with the 3D viewer; the *value* is currently EULUMDAT-format text but the
//! in-memory model is a generic light distribution, hence `ldc` for
//! variables here). A matching `..._timestamp` key bumps on every change.
//!
//! Since Web Storage's `storage` event only fires on **other** tabs, we use
//! a short polling loop on the timestamp key instead — that's what Bevy
//! does. Poll cost is effectively zero (one string comparison per second).

use leptos::prelude::*;
use wasm_bindgen::prelude::*;

// Storage key names — the *values* stored here are LDT-formatted text,
// hence the `LDT` suffix. The in-memory Rust side uses `ldc` names because
// the same signal/variable could later hold non-LDT distributions.
const LDT_KEY: &str = "eulumdat_current_ldt";
const LDT_TIMESTAMP_KEY: &str = "eulumdat_ldt_timestamp";

/// Fetch the current LDT-encoded text from localStorage, if any.
fn read_ldt_text_from_storage() -> Option<String> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    storage.get_item(LDT_KEY).ok()?
}

/// Fetch the current timestamp from localStorage, if any.
fn read_timestamp() -> Option<String> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    storage.get_item(LDT_TIMESTAMP_KEY).ok()?
}

/// Parse an LDT or IES string into the in-memory light distribution
/// (currently `eulumdat::Eulumdat`; this is the generic LDC representation
/// consumers work with).
fn parse_editor_text(s: &str) -> Option<eulumdat::Eulumdat> {
    eulumdat::Eulumdat::parse(s)
        .or_else(|_| eulumdat::IesParser::parse(s))
        .ok()
}

/// Synchronous snapshot of whatever LDC the editor has saved.
///
/// Use this to seed the initial signal value so the first render already
/// reflects reality — the reactive polling loop then keeps it up to date.
pub fn read_ldc_now() -> Option<eulumdat::Eulumdat> {
    let s = read_ldt_text_from_storage()?;
    let parsed = parse_editor_text(&s);
    match &parsed {
        Some(ldc) => web_sys::console::log_1(
            &format!("[Street] seeded from storage: '{}'", ldc.luminaire_name).into(),
        ),
        None => web_sys::console::warn_1(
            &format!(
                "[Street] seed failed — storage had {} chars but parse rejected them",
                s.len()
            )
            .into(),
        ),
    }
    parsed
}

/// Wire up localStorage synchronization:
///
/// - Seeds the `ldc` signal from current storage (always).
/// - Registers a 1 Hz polling loop that pushes future storage updates into
///   the same signal. Registration is idempotent — repeated calls
///   (e.g. every re-mount of the Leptos app as the user flips the tab)
///   update the signal-setter without accumulating extra timers.
///
/// Call once from the app root on each mount.
pub fn wire_storage_sync(set_ldc: WriteSignal<Option<eulumdat::Eulumdat>>) {
    // Always do an initial read so a remount picks up the latest LDC.
    match read_ldt_text_from_storage() {
        Some(s) => {
            web_sys::console::log_1(&format!("[Street] storage hit: {} chars", s.len()).into());
            match parse_editor_text(&s) {
                Some(ldc) => {
                    web_sys::console::log_1(
                        &format!("[Street] parsed OK: '{}'", ldc.luminaire_name).into(),
                    );
                    set_ldc.set(Some(ldc));
                }
                None => {
                    web_sys::console::warn_1(
                        &"[Street] storage value failed to parse as LDT or IES".into(),
                    );
                }
            }
        }
        None => {
            web_sys::console::log_1(
                &format!("[Street] storage empty (key '{LDT_KEY}' missing)").into(),
            );
        }
    }

    // Swap the current signal-setter into the polling loop. On first call
    // this also spawns the setInterval; subsequent calls just overwrite the
    // setter so the running timer pushes updates to the fresh signal.
    SETTER.with(|cell| {
        *cell.borrow_mut() = Some(set_ldc);
    });
    ensure_poll_registered();
}

thread_local! {
    static SETTER: std::cell::RefCell<Option<WriteSignal<Option<eulumdat::Eulumdat>>>> =
        const { std::cell::RefCell::new(None) };
    static POLL_REGISTERED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static LAST_TS: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

fn ensure_poll_registered() {
    if POLL_REGISTERED.with(std::cell::Cell::get) {
        return;
    }
    let Some(window) = web_sys::window() else {
        return;
    };

    LAST_TS.with(|ts| {
        *ts.borrow_mut() = read_timestamp().unwrap_or_default();
    });

    let callback = Closure::<dyn FnMut()>::new(|| {
        let Some(ts) = read_timestamp() else {
            return;
        };
        let changed = LAST_TS.with(|last| {
            let mut last = last.borrow_mut();
            if *last == ts {
                false
            } else {
                *last = ts;
                true
            }
        });
        if !changed {
            return;
        }
        if let Some(s) = read_ldt_text_from_storage() {
            if let Some(ldc) = parse_editor_text(&s) {
                web_sys::console::log_1(
                    &format!("[Street] poll update: '{}'", ldc.luminaire_name).into(),
                );
                SETTER.with(|cell| {
                    if let Some(set_ldc) = *cell.borrow() {
                        set_ldc.set(Some(ldc));
                    }
                });
            }
        }
    });
    let _ = window.set_interval_with_callback_and_timeout_and_arguments_0(
        callback.as_ref().unchecked_ref(),
        1000,
    );
    // Callback outlives the Rust reference: the setInterval holds onto it
    // in the JS runtime, matching how eulumdat-bevy keeps its sync loop alive.
    callback.forget();
    POLL_REGISTERED.with(|cell| cell.set(true));
}
