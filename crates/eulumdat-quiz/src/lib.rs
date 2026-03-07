//! Photometric knowledge quiz engine for lighting professionals.
//!
//! Pure Rust library with no UI dependencies. Designed to be FFI-safe
//! (uniffi/PyO3) for use across TUI, Web, Desktop, iOS, Android, and Python.

pub mod i18n;
mod questions;
mod session;

pub use session::QuizSession;

/// Knowledge domain categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Category {
    /// LDT file structure, line numbers, field meanings
    EulumdatFormat,
    /// LM-63 versions, keywords, photometric types A/B/C
    IesFormat,
    /// 5 symmetry types, data expansion, compression ratios
    Symmetry,
    /// C-plane angles, gamma angles, nadir/zenith, Type B↔C
    CoordinateSystems,
    /// LOR, DFF, beam/field angles, CIE flux codes, efficacy
    PhotometricCalc,
    /// TM-15-11 zones, thresholds, B/U/G 0-5 scale
    BugRating,
    /// UGR formula, standard rooms, CIE 117
    UgrGlare,
    /// CCT, CRI groups, TM-30 Rf/Rg, Duv, SPD
    ColorScience,
    /// PAR, PPF, PPFD, DLI, R:FR ratio, spectral zones
    Horticultural,
    /// TM-32-24 parameters, NEMA GUIDs, housing shapes
    BimIntegration,
    /// TM-33-23/ATLA S001, XML vs JSON, spectral support
    ModernFormats,
    /// Warning codes W001-W046, error codes E001-E006
    Validation,
    /// lux/fc, m/ft, mm/in, cd/klm, lm/W
    Units,
    /// Polar, cartesian, heatmap, cone, butterfly, isolux
    DiagramTypes,
    /// Reading and interpreting polar light distribution diagrams
    DiagramReading,
    /// CIE, IES, NEMA, EN 13201, IDA, LEED, Title 24
    Standards,
}

impl Category {
    /// Stable string key for i18n lookup (matches JSON locale keys).
    pub fn key(&self) -> &'static str {
        match self {
            Self::EulumdatFormat => "eulumdat_format",
            Self::IesFormat => "ies_format",
            Self::Symmetry => "symmetry",
            Self::CoordinateSystems => "coordinate_systems",
            Self::PhotometricCalc => "photometric_calc",
            Self::BugRating => "bug_rating",
            Self::UgrGlare => "ugr_glare",
            Self::ColorScience => "color_science",
            Self::Horticultural => "horticultural",
            Self::BimIntegration => "bim_integration",
            Self::ModernFormats => "modern_formats",
            Self::Validation => "validation",
            Self::Units => "units",
            Self::DiagramTypes => "diagram_types",
            Self::DiagramReading => "diagram_reading",
            Self::Standards => "standards",
        }
    }

    /// Human-readable label for display.
    pub fn label(&self) -> &'static str {
        match self {
            Self::EulumdatFormat => "EULUMDAT Format",
            Self::IesFormat => "IES Format",
            Self::Symmetry => "Symmetry",
            Self::CoordinateSystems => "Coordinate Systems",
            Self::PhotometricCalc => "Photometric Calculations",
            Self::BugRating => "BUG Rating",
            Self::UgrGlare => "UGR & Glare",
            Self::ColorScience => "Color Science",
            Self::Horticultural => "Horticultural Lighting",
            Self::BimIntegration => "BIM Integration",
            Self::ModernFormats => "Modern Formats",
            Self::Validation => "Validation",
            Self::Units => "Units & Conversions",
            Self::DiagramTypes => "Diagram Types",
            Self::DiagramReading => "Diagram Reading",
            Self::Standards => "Standards & Compliance",
        }
    }

    /// All category variants.
    pub fn all() -> Vec<Category> {
        vec![
            Self::EulumdatFormat,
            Self::IesFormat,
            Self::Symmetry,
            Self::CoordinateSystems,
            Self::PhotometricCalc,
            Self::BugRating,
            Self::UgrGlare,
            Self::ColorScience,
            Self::Horticultural,
            Self::BimIntegration,
            Self::ModernFormats,
            Self::Validation,
            Self::Units,
            Self::DiagramTypes,
            Self::DiagramReading,
            Self::Standards,
        ]
    }
}

/// Difficulty level for questions.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum Difficulty {
    /// Format basics, unit definitions, simple facts
    Beginner,
    /// Calculations, thresholds, standard comparisons
    Intermediate,
    /// Cross-standard nuances, edge cases, formulas
    Expert,
}

impl Difficulty {
    /// Stable string key for i18n lookup.
    pub fn key(&self) -> &'static str {
        match self {
            Self::Beginner => "beginner",
            Self::Intermediate => "intermediate",
            Self::Expert => "expert",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Beginner => "Beginner",
            Self::Intermediate => "Intermediate",
            Self::Expert => "Expert",
        }
    }
}

/// A single quiz question with 4 multiple-choice options.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Question {
    pub id: u32,
    pub category: Category,
    pub difficulty: Difficulty,
    pub text: String,
    /// 4 choices (A-D)
    pub options: Vec<String>,
    /// Index of the correct option (0-3)
    pub correct_index: u8,
    /// Explanation shown after answering
    pub explanation: String,
    /// Reference standard or specification
    pub reference: Option<String>,
}

/// Configuration for creating a quiz session.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct QuizConfig {
    /// Filter by categories (empty = all)
    pub categories: Vec<Category>,
    /// Filter by difficulty (None = mixed)
    pub difficulty: Option<Difficulty>,
    /// Number of questions (0 = all matching)
    pub num_questions: u32,
    /// Shuffle question order
    pub shuffle: bool,
    /// Seed for reproducible shuffle
    pub seed: Option<u64>,
}

impl Default for QuizConfig {
    fn default() -> Self {
        Self {
            categories: vec![],
            difficulty: None,
            num_questions: 10,
            shuffle: true,
            seed: None,
        }
    }
}

/// Score for a specific category.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CategoryScore {
    pub category: Category,
    pub correct: u32,
    pub total: u32,
}

/// Score for a specific difficulty level.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DifficultyScore {
    pub difficulty: Difficulty,
    pub correct: u32,
    pub total: u32,
}

/// Overall quiz score with breakdowns.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct QuizScore {
    pub correct: u32,
    pub wrong: u32,
    pub skipped: u32,
    pub total: u32,
    pub by_category: Vec<CategoryScore>,
    pub by_difficulty: Vec<DifficultyScore>,
}

impl QuizScore {
    /// Percentage score (0.0-100.0).
    pub fn percentage(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.correct as f64 / self.total as f64 * 100.0
        }
    }
}

/// Result of answering a question.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AnswerResult {
    pub is_correct: bool,
    pub correct_index: u8,
    pub explanation: String,
    pub reference: Option<String>,
}

/// Static quiz bank with all available questions.
pub struct QuizBank;

impl QuizBank {
    /// All questions in the bank.
    pub fn all_questions() -> Vec<Question> {
        questions::all_questions()
    }

    /// Available categories with question counts.
    pub fn categories() -> Vec<(Category, u32)> {
        let questions = Self::all_questions();
        Category::all()
            .into_iter()
            .map(|cat| {
                let count = questions.iter().filter(|q| q.category == cat).count() as u32;
                (cat, count)
            })
            .filter(|(_, count)| *count > 0)
            .collect()
    }

    /// Total number of questions.
    pub fn total_count() -> u32 {
        Self::all_questions().len() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_questions_valid() {
        let questions = QuizBank::all_questions();
        assert!(
            questions.len() >= 100,
            "Expected at least 100 questions, got {}",
            questions.len()
        );

        for q in &questions {
            assert_eq!(q.options.len(), 4, "Question {} must have 4 options", q.id);
            assert!(
                q.correct_index < 4,
                "Question {} has invalid correct_index {}",
                q.id,
                q.correct_index
            );
            assert!(!q.text.is_empty(), "Question {} has empty text", q.id);
            assert!(
                !q.explanation.is_empty(),
                "Question {} has empty explanation",
                q.id
            );
        }
    }

    #[test]
    fn test_no_duplicate_ids() {
        let questions = QuizBank::all_questions();
        let mut ids: Vec<u32> = questions.iter().map(|q| q.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), questions.len(), "Duplicate question IDs found");
    }

    #[test]
    fn test_all_categories_have_questions() {
        let questions = QuizBank::all_questions();
        for cat in Category::all() {
            let count = questions.iter().filter(|q| q.category == cat).count();
            assert!(
                count >= 5,
                "Category {:?} has only {} questions (need >= 5)",
                cat,
                count
            );
        }
    }

    #[test]
    fn test_all_difficulties_have_questions() {
        let questions = QuizBank::all_questions();
        for diff in [
            Difficulty::Beginner,
            Difficulty::Intermediate,
            Difficulty::Expert,
        ] {
            let count = questions.iter().filter(|q| q.difficulty == diff).count();
            assert!(
                count >= 20,
                "Difficulty {:?} has only {} questions (need >= 20)",
                diff,
                count
            );
        }
    }

    #[test]
    fn test_quiz_session_basic() {
        let config = QuizConfig {
            num_questions: 5,
            shuffle: false,
            ..Default::default()
        };
        let mut session = QuizSession::new(config);
        assert!(!session.is_finished());

        let (idx, total) = session.progress();
        assert_eq!(idx, 0);
        assert_eq!(total, 5);

        let q = session.current_question().unwrap();
        let result = session.answer(q.correct_index);
        assert!(result.is_correct);

        let score = session.score();
        assert_eq!(score.correct, 1);
        assert_eq!(score.wrong, 0);
    }

    #[test]
    fn test_quiz_session_skip() {
        let config = QuizConfig {
            num_questions: 3,
            shuffle: false,
            ..Default::default()
        };
        let mut session = QuizSession::new(config);
        session.skip();
        let score = session.score();
        assert_eq!(score.skipped, 1);
    }

    #[test]
    fn test_quiz_config_filter_category() {
        let config = QuizConfig {
            categories: vec![Category::BugRating],
            num_questions: 0,
            shuffle: false,
            ..Default::default()
        };
        let session = QuizSession::new(config);
        let (_, total) = session.progress();
        assert!(total > 0);
        // All questions should be BugRating
        for i in 0..total {
            let q = &session.questions[i];
            assert_eq!(q.category, Category::BugRating);
        }
    }

    #[test]
    fn test_quiz_config_filter_difficulty() {
        let config = QuizConfig {
            difficulty: Some(Difficulty::Expert),
            num_questions: 0,
            shuffle: false,
            ..Default::default()
        };
        let session = QuizSession::new(config);
        for q in &session.questions {
            assert_eq!(q.difficulty, Difficulty::Expert);
        }
    }

    #[test]
    fn test_score_percentage() {
        let score = QuizScore {
            correct: 7,
            wrong: 3,
            skipped: 0,
            total: 10,
            ..Default::default()
        };
        assert!((score.percentage() - 70.0).abs() < f64::EPSILON);
    }
}
