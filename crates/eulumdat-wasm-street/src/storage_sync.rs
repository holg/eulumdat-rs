//! localStorage bridge to the main eulumdat editor.
//!
//! The editor writes the current luminaire to `localStorage` under the key
//! `eulumdat_current_ldt` (+ a matching `..._timestamp` key that bumps on
//! every change). The 3D viewer reads the same keys; this module lets the
//! street designer do the same.
//!
//! Since Web Storage's `storage` event only fires on **other** tabs, we use
//! a short polling loop on the timestamp key instead — that's what Bevy
//! does. Poll cost is effectively zero (one string comparison per second).

use leptos::prelude::*;
use wasm_bindgen::prelude::*;

const LDT_KEY: &str = "eulumdat_current_ldt";
const LDT_TIMESTAMP_KEY: &str = "eulumdat_ldt_timestamp";

/// Identifies where the currently-loaded luminaire came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LdtSource {
    /// No luminaire loaded.
    None,
    /// Received live from the main editor via localStorage.
    Editor,
    /// User uploaded a file directly to the street designer.
    Upload,
}

/// Fetch the current LDT string from localStorage, if any.
fn read_ldt_from_storage() -> Option<String> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    storage.get_item(LDT_KEY).ok()?
}

/// Fetch the current LDT timestamp from localStorage, if any.
fn read_timestamp() -> Option<String> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    storage.get_item(LDT_TIMESTAMP_KEY).ok()?
}

/// Parse an LDT or IES string into an `Eulumdat`.
fn parse_editor_string(s: &str) -> Option<eulumdat::Eulumdat> {
    eulumdat::Eulumdat::parse(s)
        .or_else(|_| eulumdat::IesParser::parse(s))
        .ok()
}

/// Wire up localStorage synchronization:
///
/// - Seeds the `ldt` signal from current storage (always).
/// - Registers a 1 Hz polling loop that pushes future storage updates into
///   the same signal. Registration is idempotent — repeated calls
///   (e.g. every re-mount of the Leptos app as the user flips the tab)
///   update the signal-setters without accumulating extra timers.
///
/// Call once from the app root on each mount.
pub fn wire_storage_sync(
    set_ldt: WriteSignal<Option<eulumdat::Eulumdat>>,
    set_source: WriteSignal<LdtSource>,
) {
    // Always do an initial read so a remount picks up the latest LDT.
    if let Some(s) = read_ldt_from_storage() {
        if let Some(ldt) = parse_editor_string(&s) {
            set_ldt.set(Some(ldt));
            set_source.set(LdtSource::Editor);
        }
    }

    // Swap the current signal-setters into the polling loop. On first call
    // this also spawns the setInterval; subsequent calls just overwrite the
    // setters so the running timer pushes updates to the fresh signals.
    SETTERS.with(|cell| {
        *cell.borrow_mut() = Some((set_ldt, set_source));
    });
    ensure_poll_registered();
}

type SignalSetters = (
    WriteSignal<Option<eulumdat::Eulumdat>>,
    WriteSignal<LdtSource>,
);

thread_local! {
    static SETTERS: std::cell::RefCell<Option<SignalSetters>> =
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
        if let Some(s) = read_ldt_from_storage() {
            if let Some(ldt) = parse_editor_string(&s) {
                SETTERS.with(|cell| {
                    if let Some((set_ldt, set_source)) = *cell.borrow() {
                        set_ldt.set(Some(ldt));
                        set_source.set(LdtSource::Editor);
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
