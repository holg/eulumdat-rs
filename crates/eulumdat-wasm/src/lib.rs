#![allow(dead_code, clippy::enum_variant_names)]

pub mod analytics;
#[cfg(feature = "benchmark")]
pub mod benchmark;
mod components;
pub mod i18n;
pub mod library;

#[cfg(feature = "benchmark")]
pub use benchmark::{
    compare_benchmark, run_benchmark, run_benchmark_challenging, run_benchmark_full,
};
pub use components::App;
pub use i18n::{use_language, use_locale, I18nProvider, LanguageSelector, LanguageSelectorCompact};
