#![allow(dead_code, clippy::enum_variant_names)]

mod components;
pub mod i18n;

pub use components::App;
pub use i18n::{use_language, use_locale, I18nProvider, LanguageSelector, LanguageSelectorCompact};
