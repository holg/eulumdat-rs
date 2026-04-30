//! Street designer i18n bridge.
//!
//! The main editor is the single source of truth for the chosen language:
//! it writes the language code into `localStorage["eulumdat_language"]`
//! and the street designer polls the same key. The editor also exposes a
//! `language` signal to React-style contexts, but since the street app
//! mounts as its own Leptos app via the lazy loader, it can't reach that
//! context — localStorage is the bridge.

use eulumdat::UnitSystem;
use eulumdat_i18n::{Language, Locale};
use leptos::prelude::*;
use wasm_bindgen::prelude::*;

const LANG_KEY: &str = "eulumdat_language";
const UNITS_KEY: &str = "eulumdat_unit_system";

/// Read the saved language code, defaulting to English.
fn read_language() -> Language {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(code)) = storage.get_item(LANG_KEY) {
                return Language::from_code(&code);
            }
        }
    }
    Language::English
}

/// Read the saved unit-system preference, defaulting to Metric. Mirrors
/// the editor's storage format (`"metric"` / `"imperial"`).
fn read_units() -> UnitSystem {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(val)) = storage.get_item(UNITS_KEY) {
                if val == "imperial" {
                    return UnitSystem::Imperial;
                }
            }
        }
    }
    UnitSystem::Metric
}

/// Synchronously read both language and unit system from storage. Use this
/// to seed signals before the polling loop starts.
pub fn install_units() -> ReadSignal<UnitSystem> {
    let (units, set_units) = signal(read_units());
    UNIT_SETTER.with(|c| *c.borrow_mut() = Some(set_units));
    // The same poll loop registered by `install_locale` picks up both keys.
    // If the caller only wants units (or only locale), they still get the
    // poll started via either call.
    units
}

/// Provide the language signal at the street-app root and poll for
/// changes coming from the main editor's language selector.
///
/// Returns a memo that resolves the current `Locale` reactively — callers
/// use `locale.get().street.xxx.clone()` in views.
pub fn install_locale() -> Memo<Locale> {
    let (language, set_language) = signal(read_language());

    // Register a 1 Hz poll mirroring the LDT storage-sync pattern. Keeps
    // the street UI in step when the user flips the main editor's language
    // picker. Idempotent via a thread-local latch.
    ensure_poll_registered(set_language);

    Memo::new(move |_| Locale::for_language(language.get()))
}

thread_local! {
    static POLL_SETTER: std::cell::RefCell<Option<WriteSignal<Language>>> =
        const { std::cell::RefCell::new(None) };
    static UNIT_SETTER: std::cell::RefCell<Option<WriteSignal<UnitSystem>>> =
        const { std::cell::RefCell::new(None) };
    static POLL_REGISTERED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static LAST_CODE: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
    static LAST_UNITS: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

fn ensure_poll_registered(set_language: WriteSignal<Language>) {
    // Always update the setter — a remount hands us a fresh signal.
    POLL_SETTER.with(|c| *c.borrow_mut() = Some(set_language));

    if POLL_REGISTERED.with(std::cell::Cell::get) {
        return;
    }
    let Some(window) = web_sys::window() else {
        return;
    };

    LAST_CODE.with(|c| {
        *c.borrow_mut() = window
            .local_storage()
            .ok()
            .flatten()
            .and_then(|s| s.get_item(LANG_KEY).ok().flatten())
            .unwrap_or_default();
    });
    LAST_UNITS.with(|c| {
        *c.borrow_mut() = window
            .local_storage()
            .ok()
            .flatten()
            .and_then(|s| s.get_item(UNITS_KEY).ok().flatten())
            .unwrap_or_default();
    });

    let cb = Closure::<dyn FnMut()>::new(|| {
        let Some(window) = web_sys::window() else {
            return;
        };
        let Ok(Some(storage)) = window.local_storage() else {
            return;
        };

        // Language
        let code = storage
            .get_item(LANG_KEY)
            .ok()
            .flatten()
            .unwrap_or_default();
        let lang_changed = LAST_CODE.with(|c| {
            let mut last = c.borrow_mut();
            if *last == code {
                false
            } else {
                *last = code.clone();
                true
            }
        });
        if lang_changed {
            POLL_SETTER.with(|cell| {
                if let Some(setter) = *cell.borrow() {
                    setter.set(Language::from_code(&code));
                }
            });
        }

        // Unit system
        let units = storage
            .get_item(UNITS_KEY)
            .ok()
            .flatten()
            .unwrap_or_default();
        let units_changed = LAST_UNITS.with(|c| {
            let mut last = c.borrow_mut();
            if *last == units {
                false
            } else {
                *last = units.clone();
                true
            }
        });
        if units_changed {
            let u = if units == "imperial" {
                UnitSystem::Imperial
            } else {
                UnitSystem::Metric
            };
            UNIT_SETTER.with(|cell| {
                if let Some(setter) = *cell.borrow() {
                    setter.set(u);
                }
            });
        }
    });
    let _ = window
        .set_interval_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), 1000);
    cb.forget();
    POLL_REGISTERED.with(|c| c.set(true));
}
