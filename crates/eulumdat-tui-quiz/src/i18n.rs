use eulumdat_i18n::Language;

/// Detect terminal language from environment variables.
/// Checks LC_ALL, LC_MESSAGES, LANG in priority order.
pub fn detect_terminal_language() -> Language {
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(val) = std::env::var(var) {
            if let Some(lang) = parse_locale_string(&val) {
                return lang;
            }
        }
    }
    Language::English
}

fn parse_locale_string(s: &str) -> Option<Language> {
    let s = s.trim();
    if s.is_empty() || s == "C" || s == "POSIX" {
        return Some(Language::English);
    }
    // e.g. "de_DE.UTF-8" → "de", "pt_BR.UTF-8" → "pt-BR", "zh_CN" → "zh"
    let code = s.split('.').next().unwrap_or(s);
    let code = code.split('@').next().unwrap_or(code);

    // Convert underscore to hyphen for matching
    let normalized = code.replace('_', "-").to_lowercase();

    let lang = if normalized.starts_with("pt") {
        Language::PortugueseBrazil
    } else if normalized.starts_with("zh") {
        Language::Chinese
    } else if normalized.starts_with("de") {
        Language::German
    } else if normalized.starts_with("fr") {
        Language::French
    } else if normalized.starts_with("es") {
        Language::Spanish
    } else if normalized.starts_with("it") {
        Language::Italian
    } else if normalized.starts_with("ru") {
        Language::Russian
    } else if normalized.starts_with("en") {
        Language::English
    } else {
        return None;
    };
    Some(lang)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_locale_string() {
        assert_eq!(parse_locale_string("de_DE.UTF-8"), Some(Language::German));
        assert_eq!(
            parse_locale_string("pt_BR.UTF-8"),
            Some(Language::PortugueseBrazil)
        );
        assert_eq!(parse_locale_string("zh_CN"), Some(Language::Chinese));
        assert_eq!(parse_locale_string("en_US.UTF-8"), Some(Language::English));
        assert_eq!(parse_locale_string("C"), Some(Language::English));
        assert_eq!(parse_locale_string("POSIX"), Some(Language::English));
        assert_eq!(parse_locale_string("fr_FR"), Some(Language::French));
        assert_eq!(parse_locale_string("it_IT.UTF-8"), Some(Language::Italian));
        assert_eq!(parse_locale_string("ru_RU.UTF-8"), Some(Language::Russian));
        assert_eq!(parse_locale_string("es_ES"), Some(Language::Spanish));
    }
}
