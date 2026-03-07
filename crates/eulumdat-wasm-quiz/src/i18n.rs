//! I18n support for the WASM quiz app.
//!
//! Mirrors the main WASM app's i18n pattern: I18nProvider at root,
//! hooks for components, LanguageSelector dropdown.

use eulumdat_i18n::Language;
use eulumdat_quiz::i18n::QuizLocale;
use leptos::prelude::*;

/// Signal pair for the current language.
pub type LanguageSignal = (ReadSignal<Language>, WriteSignal<Language>);

/// Detect browser's preferred language.
fn detect_browser_language() -> Language {
    let window = web_sys::window();
    if let Some(window) = window {
        if let Some(lang) = window.navigator().language() {
            return Language::from_code(&lang);
        }
        let languages = window.navigator().languages();
        if languages.length() > 0 {
            if let Some(lang) = languages.get(0).as_string() {
                return Language::from_code(&lang);
            }
        }
    }
    Language::English
}

/// Get saved language from localStorage (shared key with main app).
fn get_saved_language() -> Option<Language> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    let code = storage.get_item("eulumdat_language").ok()??;
    Some(Language::from_code(&code))
}

/// Save language to localStorage.
fn save_language(lang: Language) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item("eulumdat_language", lang.code());
        }
    }
}

/// Initialize language: localStorage > browser > English.
fn init_language() -> Language {
    get_saved_language().unwrap_or_else(detect_browser_language)
}

/// Provides language context and quiz locale at the app root.
#[component]
pub fn I18nProvider(children: Children) -> impl IntoView {
    let initial_lang = init_language();
    let (language, set_language) = signal(initial_lang);

    Effect::new(move |_| {
        save_language(language.get());
    });

    provide_context((language, set_language));

    children()
}

/// Get a reactive QuizLocale memo from context.
pub fn use_quiz_locale() -> Memo<QuizLocale> {
    let (language, _) = use_context::<LanguageSignal>().expect("I18nProvider not found");
    Memo::new(move |_| QuizLocale::for_code(language.get().code()))
}

/// Get the language signal from context.
pub fn use_language() -> LanguageSignal {
    use_context::<LanguageSignal>().expect("I18nProvider not found")
}

/// Language selector dropdown.
#[component]
pub fn LanguageSelector() -> impl IntoView {
    let (language, set_language) = use_language();

    let on_change = move |ev: web_sys::Event| {
        let target = event_target::<web_sys::HtmlSelectElement>(&ev);
        let code = target.value();
        set_language.set(Language::from_code(&code));
    };

    view! {
        <select
            class="language-selector"
            on:change=on_change
            prop:value=move || language.get().code()
            aria-label="Select language"
        >
            {Language::all()
                .iter()
                .map(|lang| {
                    let code = lang.code();
                    let name = lang.native_name();
                    let is_selected = move || language.get() == *lang;
                    view! {
                        <option value=code selected=is_selected>
                            {name}
                        </option>
                    }
                })
                .collect::<Vec<_>>()}
        </select>
    }
}
