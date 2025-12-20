//! Internationalization support for the WASM app
//!
//! Provides a Leptos context for language switching and localized strings.

use eulumdat_i18n::{Language, Locale};
use leptos::prelude::*;

/// Signal for the current language
pub type LanguageSignal = (ReadSignal<Language>, WriteSignal<Language>);

/// Get the browser's preferred language
pub fn detect_browser_language() -> Language {
    let window = web_sys::window();
    if let Some(window) = window {
        if let Some(navigator) = window.navigator().language() {
            return Language::from_code(&navigator);
        }
        // Try languages array
        let languages = window.navigator().languages();
        if languages.length() > 0 {
            if let Some(lang) = languages.get(0).as_string() {
                return Language::from_code(&lang);
            }
        }
    }
    Language::English
}

/// Get saved language from localStorage
pub fn get_saved_language() -> Option<Language> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    let code = storage.get_item("eulumdat_language").ok()??;
    Some(Language::from_code(&code))
}

/// Save language to localStorage
pub fn save_language(lang: Language) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item("eulumdat_language", lang.code());
        }
    }
}

/// Initialize language - checks localStorage first, then browser, falls back to English
pub fn init_language() -> Language {
    get_saved_language().unwrap_or_else(detect_browser_language)
}

/// Provide the language context at the app root
#[component]
pub fn I18nProvider(children: Children) -> impl IntoView {
    let initial_lang = init_language();
    let (language, set_language) = signal(initial_lang);

    // Save to localStorage when language changes
    Effect::new(move |_| {
        save_language(language.get());
    });

    provide_context((language, set_language));

    children()
}

/// Get the current locale from context
pub fn use_locale() -> Memo<Locale> {
    let (language, _) = use_context::<LanguageSignal>().expect("I18nProvider not found");
    Memo::new(move |_| Locale::for_language(language.get()))
}

/// Get the language signal from context
pub fn use_language() -> LanguageSignal {
    use_context::<LanguageSignal>().expect("I18nProvider not found")
}

/// Language selector component
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

/// Compact language selector (dropdown to save space)
#[component]
pub fn LanguageSelectorCompact() -> impl IntoView {
    let (language, set_language) = use_language();

    let on_change = move |ev: web_sys::Event| {
        let target = event_target::<web_sys::HtmlSelectElement>(&ev);
        let code = target.value();
        set_language.set(Language::from_code(&code));
    };

    view! {
        <select
            class="language-selector-compact"
            on:change=on_change
            prop:value=move || language.get().code()
            title="Select language"
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
