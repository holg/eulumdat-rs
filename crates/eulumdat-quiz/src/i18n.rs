//! Self-contained i18n for quiz questions and UI strings.
//!
//! Quiz translations live separately from the main `eulumdat-i18n` locale files
//! to avoid bloating them. Each language's JSON is embedded at compile time.

use serde::Deserialize;
use std::collections::HashMap;

/// All translatable UI strings for the quiz.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct QuizUiStrings {
    pub title: String,
    pub configure: String,
    pub categories: String,
    pub select_all: String,
    pub select_none: String,
    pub difficulty: String,
    pub all_levels: String,
    pub beginner: String,
    pub intermediate: String,
    pub expert: String,
    pub num_questions: String,
    pub questions_available: String,
    pub questions_selected: String,
    pub start_quiz: String,
    pub question_of: String,
    pub correct_count: String,
    pub correct: String,
    pub wrong: String,
    pub reference: String,
    pub next_question: String,
    pub see_results: String,
    pub skip: String,
    pub excellent: String,
    pub good_job: String,
    pub keep_learning: String,
    pub try_again: String,
    pub score_detail: String,
    pub by_category: String,
    pub by_difficulty: String,
    pub try_again_btn: String,
    pub back_to_editor: String,
    pub powered_by: String,
    pub questions_across: String,
    pub polar_diagram: String,
    pub cartesian_diagram: String,
    pub symmetric: String,
    pub asymmetric: String,
    pub projector_narrow: String,
    #[serde(default)]
    pub heatmap: Option<String>,
}

/// A single translated question.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct QuestionLocale {
    pub text: String,
    pub options: Vec<String>,
    pub explanation: String,
}

/// Complete quiz locale data for one language.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct QuizLocale {
    pub ui: QuizUiStrings,
    pub categories: HashMap<String, String>,
    pub questions: HashMap<String, QuestionLocale>,
}

const EN_JSON: &str = include_str!("../locales/en.json");
const ZH_JSON: &str = include_str!("../locales/zh.json");
const DE_JSON: &str = include_str!("../locales/de.json");
const FR_JSON: &str = include_str!("../locales/fr.json");
const ES_JSON: &str = include_str!("../locales/es.json");
const IT_JSON: &str = include_str!("../locales/it.json");
const RU_JSON: &str = include_str!("../locales/ru.json");
const PT_BR_JSON: &str = include_str!("../locales/pt-BR.json");

impl QuizLocale {
    /// Load locale for a language code (e.g. "en", "zh", "de").
    /// Falls back to English for unknown codes.
    pub fn for_code(code: &str) -> Self {
        let json = match code.to_lowercase().as_str() {
            "en" => EN_JSON,
            "zh" | "zh-cn" | "zh-hans" => ZH_JSON,
            "de" => DE_JSON,
            "fr" => FR_JSON,
            "es" => ES_JSON,
            "it" => IT_JSON,
            "ru" => RU_JSON,
            "pt" | "pt-br" => PT_BR_JSON,
            _ => EN_JSON,
        };
        serde_json::from_str(json).expect("invalid quiz locale JSON")
    }

    /// Get translated question by numeric ID, falling back to None if missing.
    pub fn question(&self, id: u32) -> Option<&QuestionLocale> {
        self.questions.get(&id.to_string())
    }

    /// Get translated category label, falling back to the built-in English label.
    pub fn category_label(&self, cat: &crate::Category) -> &str {
        self.categories
            .get(cat.key())
            .map(|s| s.as_str())
            .unwrap_or(cat.label())
    }

    /// Get translated difficulty label.
    pub fn difficulty_label(&self, diff: &crate::Difficulty) -> &str {
        match diff {
            crate::Difficulty::Beginner => &self.ui.beginner,
            crate::Difficulty::Intermediate => &self.ui.intermediate,
            crate::Difficulty::Expert => &self.ui.expert,
        }
    }

    /// Simple template formatter: replaces {0}, {1}, ... with args.
    pub fn format(template: &str, args: &[&dyn std::fmt::Display]) -> String {
        let mut result = template.to_string();
        for (i, arg) in args.iter().enumerate() {
            result = result.replace(&format!("{{{}}}", i), &arg.to_string());
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_english_locale_loads() {
        let locale = QuizLocale::for_code("en");
        assert_eq!(locale.ui.title, "Photometric Knowledge Quiz");
        assert!(!locale.questions.is_empty());
    }

    #[test]
    fn test_all_locales_load() {
        for code in &["en", "zh", "de", "fr", "es", "it", "ru", "pt-BR"] {
            let locale = QuizLocale::for_code(code);
            assert!(
                !locale.ui.title.is_empty(),
                "Locale {} has empty title",
                code
            );
            assert!(
                !locale.categories.is_empty(),
                "Locale {} has empty categories",
                code
            );
        }
    }

    #[test]
    fn test_english_has_all_questions() {
        let locale = QuizLocale::for_code("en");
        let all = crate::QuizBank::all_questions();
        for q in &all {
            assert!(
                locale.question(q.id).is_some(),
                "English locale missing question {}",
                q.id
            );
        }
    }

    #[test]
    fn test_format() {
        let s = QuizLocale::format("{0} of {1} questions", &[&5, &10]);
        assert_eq!(s, "5 of 10 questions");
    }

    #[test]
    fn test_category_label_lookup() {
        let locale = QuizLocale::for_code("en");
        let label = locale.category_label(&crate::Category::EulumdatFormat);
        assert_eq!(label, "EULUMDAT Format");
    }
}
