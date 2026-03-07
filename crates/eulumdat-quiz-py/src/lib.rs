//! Python bindings for the eulumdat photometric knowledge quiz engine.

use pyo3::prelude::*;

use ::eulumdat_quiz as quiz_core;

// ---------------------------------------------------------------------------
// Category enum
// ---------------------------------------------------------------------------

#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Category {
    EulumdatFormat = 0,
    IesFormat = 1,
    Symmetry = 2,
    CoordinateSystems = 3,
    PhotometricCalc = 4,
    BugRating = 5,
    UgrGlare = 6,
    ColorScience = 7,
    Horticultural = 8,
    BimIntegration = 9,
    ModernFormats = 10,
    Validation = 11,
    Units = 12,
    DiagramTypes = 13,
    DiagramReading = 14,
    Standards = 15,
}

#[pymethods]
impl Category {
    /// Stable string key for i18n lookup.
    fn key(&self) -> &'static str {
        self.to_core().key()
    }

    /// Human-readable English label.
    fn label(&self) -> &'static str {
        self.to_core().label()
    }

    fn __repr__(&self) -> String {
        format!("Category.{}", self.label())
    }
}

impl Category {
    fn to_core(self) -> quiz_core::Category {
        match self {
            Self::EulumdatFormat => quiz_core::Category::EulumdatFormat,
            Self::IesFormat => quiz_core::Category::IesFormat,
            Self::Symmetry => quiz_core::Category::Symmetry,
            Self::CoordinateSystems => quiz_core::Category::CoordinateSystems,
            Self::PhotometricCalc => quiz_core::Category::PhotometricCalc,
            Self::BugRating => quiz_core::Category::BugRating,
            Self::UgrGlare => quiz_core::Category::UgrGlare,
            Self::ColorScience => quiz_core::Category::ColorScience,
            Self::Horticultural => quiz_core::Category::Horticultural,
            Self::BimIntegration => quiz_core::Category::BimIntegration,
            Self::ModernFormats => quiz_core::Category::ModernFormats,
            Self::Validation => quiz_core::Category::Validation,
            Self::Units => quiz_core::Category::Units,
            Self::DiagramTypes => quiz_core::Category::DiagramTypes,
            Self::DiagramReading => quiz_core::Category::DiagramReading,
            Self::Standards => quiz_core::Category::Standards,
        }
    }

    fn from_core(c: quiz_core::Category) -> Self {
        match c {
            quiz_core::Category::EulumdatFormat => Self::EulumdatFormat,
            quiz_core::Category::IesFormat => Self::IesFormat,
            quiz_core::Category::Symmetry => Self::Symmetry,
            quiz_core::Category::CoordinateSystems => Self::CoordinateSystems,
            quiz_core::Category::PhotometricCalc => Self::PhotometricCalc,
            quiz_core::Category::BugRating => Self::BugRating,
            quiz_core::Category::UgrGlare => Self::UgrGlare,
            quiz_core::Category::ColorScience => Self::ColorScience,
            quiz_core::Category::Horticultural => Self::Horticultural,
            quiz_core::Category::BimIntegration => Self::BimIntegration,
            quiz_core::Category::ModernFormats => Self::ModernFormats,
            quiz_core::Category::Validation => Self::Validation,
            quiz_core::Category::Units => Self::Units,
            quiz_core::Category::DiagramTypes => Self::DiagramTypes,
            quiz_core::Category::DiagramReading => Self::DiagramReading,
            quiz_core::Category::Standards => Self::Standards,
        }
    }
}

// ---------------------------------------------------------------------------
// Difficulty enum
// ---------------------------------------------------------------------------

#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Difficulty {
    Beginner = 0,
    Intermediate = 1,
    Expert = 2,
}

#[pymethods]
impl Difficulty {
    fn key(&self) -> &'static str {
        self.to_core().key()
    }

    fn label(&self) -> &'static str {
        self.to_core().label()
    }

    fn __repr__(&self) -> String {
        format!("Difficulty.{}", self.label())
    }
}

impl Difficulty {
    fn to_core(self) -> quiz_core::Difficulty {
        match self {
            Self::Beginner => quiz_core::Difficulty::Beginner,
            Self::Intermediate => quiz_core::Difficulty::Intermediate,
            Self::Expert => quiz_core::Difficulty::Expert,
        }
    }

    fn from_core(d: quiz_core::Difficulty) -> Self {
        match d {
            quiz_core::Difficulty::Beginner => Self::Beginner,
            quiz_core::Difficulty::Intermediate => Self::Intermediate,
            quiz_core::Difficulty::Expert => Self::Expert,
        }
    }
}

// ---------------------------------------------------------------------------
// Question (read-only view)
// ---------------------------------------------------------------------------

#[pyclass]
#[derive(Clone)]
pub struct Question {
    inner: quiz_core::Question,
}

#[pymethods]
impl Question {
    #[getter]
    fn id(&self) -> u32 {
        self.inner.id
    }

    #[getter]
    fn category(&self) -> Category {
        Category::from_core(self.inner.category)
    }

    #[getter]
    fn difficulty(&self) -> Difficulty {
        Difficulty::from_core(self.inner.difficulty)
    }

    #[getter]
    fn text(&self) -> &str {
        &self.inner.text
    }

    #[getter]
    fn options(&self) -> Vec<String> {
        self.inner.options.clone()
    }

    #[getter]
    fn correct_index(&self) -> u8 {
        self.inner.correct_index
    }

    #[getter]
    fn explanation(&self) -> &str {
        &self.inner.explanation
    }

    #[getter]
    fn reference(&self) -> Option<String> {
        self.inner.reference.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "Question(id={}, category={}, difficulty={})",
            self.inner.id,
            self.inner.category.label(),
            self.inner.difficulty.label()
        )
    }
}

// ---------------------------------------------------------------------------
// AnswerResult
// ---------------------------------------------------------------------------

#[pyclass]
#[derive(Clone)]
pub struct AnswerResult {
    inner: quiz_core::AnswerResult,
}

#[pymethods]
impl AnswerResult {
    #[getter]
    fn is_correct(&self) -> bool {
        self.inner.is_correct
    }

    #[getter]
    fn correct_index(&self) -> u8 {
        self.inner.correct_index
    }

    #[getter]
    fn explanation(&self) -> &str {
        &self.inner.explanation
    }

    #[getter]
    fn reference(&self) -> Option<String> {
        self.inner.reference.clone()
    }

    fn __repr__(&self) -> String {
        if self.inner.is_correct {
            "AnswerResult(correct=True)".to_string()
        } else {
            format!(
                "AnswerResult(correct=False, correct_index={})",
                self.inner.correct_index
            )
        }
    }
}

// ---------------------------------------------------------------------------
// CategoryScore / DifficultyScore
// ---------------------------------------------------------------------------

#[pyclass]
#[derive(Clone)]
pub struct CategoryScore {
    inner: quiz_core::CategoryScore,
}

#[pymethods]
impl CategoryScore {
    #[getter]
    fn category(&self) -> Category {
        Category::from_core(self.inner.category)
    }

    #[getter]
    fn correct(&self) -> u32 {
        self.inner.correct
    }

    #[getter]
    fn total(&self) -> u32 {
        self.inner.total
    }

    fn percentage(&self) -> f64 {
        if self.inner.total == 0 {
            0.0
        } else {
            self.inner.correct as f64 / self.inner.total as f64 * 100.0
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "CategoryScore({}: {}/{})",
            self.inner.category.label(),
            self.inner.correct,
            self.inner.total
        )
    }
}

#[pyclass]
#[derive(Clone)]
pub struct DifficultyScore {
    inner: quiz_core::DifficultyScore,
}

#[pymethods]
impl DifficultyScore {
    #[getter]
    fn difficulty(&self) -> Difficulty {
        Difficulty::from_core(self.inner.difficulty)
    }

    #[getter]
    fn correct(&self) -> u32 {
        self.inner.correct
    }

    #[getter]
    fn total(&self) -> u32 {
        self.inner.total
    }

    fn percentage(&self) -> f64 {
        if self.inner.total == 0 {
            0.0
        } else {
            self.inner.correct as f64 / self.inner.total as f64 * 100.0
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "DifficultyScore({}: {}/{})",
            self.inner.difficulty.label(),
            self.inner.correct,
            self.inner.total
        )
    }
}

// ---------------------------------------------------------------------------
// QuizScore
// ---------------------------------------------------------------------------

#[pyclass]
#[derive(Clone)]
pub struct QuizScore {
    inner: quiz_core::QuizScore,
}

#[pymethods]
impl QuizScore {
    #[getter]
    fn correct(&self) -> u32 {
        self.inner.correct
    }

    #[getter]
    fn wrong(&self) -> u32 {
        self.inner.wrong
    }

    #[getter]
    fn skipped(&self) -> u32 {
        self.inner.skipped
    }

    #[getter]
    fn total(&self) -> u32 {
        self.inner.total
    }

    fn percentage(&self) -> f64 {
        self.inner.percentage()
    }

    #[getter]
    fn by_category(&self) -> Vec<CategoryScore> {
        self.inner
            .by_category
            .iter()
            .map(|s| CategoryScore { inner: s.clone() })
            .collect()
    }

    #[getter]
    fn by_difficulty(&self) -> Vec<DifficultyScore> {
        self.inner
            .by_difficulty
            .iter()
            .map(|s| DifficultyScore { inner: s.clone() })
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "QuizScore({}/{} correct, {:.0}%)",
            self.inner.correct,
            self.inner.total,
            self.inner.percentage()
        )
    }
}

// ---------------------------------------------------------------------------
// QuizConfig
// ---------------------------------------------------------------------------

#[pyclass]
#[derive(Clone)]
pub struct QuizConfig {
    inner: quiz_core::QuizConfig,
}

#[pymethods]
impl QuizConfig {
    #[new]
    #[pyo3(signature = (categories=vec![], difficulty=None, num_questions=10, shuffle=true, seed=None))]
    fn new(
        categories: Vec<Category>,
        difficulty: Option<Difficulty>,
        num_questions: u32,
        shuffle: bool,
        seed: Option<u64>,
    ) -> Self {
        Self {
            inner: quiz_core::QuizConfig {
                categories: categories.into_iter().map(|c| c.to_core()).collect(),
                difficulty: difficulty.map(|d| d.to_core()),
                num_questions,
                shuffle,
                seed,
            },
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "QuizConfig(categories={}, difficulty={:?}, num_questions={})",
            self.inner.categories.len(),
            self.inner
                .difficulty
                .as_ref()
                .map(|d| d.label())
                .unwrap_or("All"),
            self.inner.num_questions
        )
    }
}

// ---------------------------------------------------------------------------
// QuizSession
// ---------------------------------------------------------------------------

#[pyclass]
pub struct QuizSession {
    inner: quiz_core::QuizSession,
}

#[pymethods]
impl QuizSession {
    #[new]
    #[pyo3(signature = (config=None))]
    fn new(config: Option<QuizConfig>) -> Self {
        let config = config.map(|c| c.inner).unwrap_or_default();
        Self {
            inner: quiz_core::QuizSession::new(config),
        }
    }

    /// Get the current question, or None if finished.
    fn current_question(&self) -> Option<Question> {
        self.inner.current_question().map(|q| Question { inner: q })
    }

    /// Submit an answer (0-3) for the current question.
    fn answer(&mut self, choice: u8) -> AnswerResult {
        AnswerResult {
            inner: self.inner.answer(choice),
        }
    }

    /// Skip the current question.
    fn skip(&mut self) -> bool {
        self.inner.skip()
    }

    /// Is the quiz finished?
    fn is_finished(&self) -> bool {
        self.inner.is_finished()
    }

    /// Current progress as (current_index, total).
    fn progress(&self) -> (usize, usize) {
        self.inner.progress()
    }

    /// Get the current score.
    fn score(&self) -> QuizScore {
        QuizScore {
            inner: self.inner.score(),
        }
    }

    fn __repr__(&self) -> String {
        let (idx, total) = self.inner.progress();
        format!("QuizSession(question {}/{})", idx + 1, total)
    }
}

// ---------------------------------------------------------------------------
// QuizBank (static methods)
// ---------------------------------------------------------------------------

#[pyclass]
pub struct QuizBank;

#[pymethods]
impl QuizBank {
    /// All questions in the bank.
    #[staticmethod]
    fn all_questions() -> Vec<Question> {
        quiz_core::QuizBank::all_questions()
            .into_iter()
            .map(|q| Question { inner: q })
            .collect()
    }

    /// Categories with question counts as list of (Category, count) tuples.
    #[staticmethod]
    fn categories() -> Vec<(Category, u32)> {
        quiz_core::QuizBank::categories()
            .into_iter()
            .map(|(c, n)| (Category::from_core(c), n))
            .collect()
    }

    /// Total number of questions.
    #[staticmethod]
    fn total_count() -> u32 {
        quiz_core::QuizBank::total_count()
    }
}

// ---------------------------------------------------------------------------
// QuizLocale (i18n)
// ---------------------------------------------------------------------------

#[pyclass]
#[derive(Clone)]
pub struct QuestionLocale {
    inner: quiz_core::i18n::QuestionLocale,
}

#[pymethods]
impl QuestionLocale {
    #[getter]
    fn text(&self) -> &str {
        &self.inner.text
    }

    #[getter]
    fn options(&self) -> Vec<String> {
        self.inner.options.clone()
    }

    #[getter]
    fn explanation(&self) -> &str {
        &self.inner.explanation
    }

    fn __repr__(&self) -> String {
        let preview = if self.inner.text.len() > 50 {
            format!("{}...", &self.inner.text[..50])
        } else {
            self.inner.text.clone()
        };
        format!("QuestionLocale(\"{}\")", preview)
    }
}

#[pyclass]
pub struct QuizLocale {
    inner: quiz_core::i18n::QuizLocale,
}

#[pymethods]
impl QuizLocale {
    /// Load locale for a language code (e.g. "en", "zh", "de").
    /// Falls back to English for unknown codes.
    #[staticmethod]
    fn for_code(code: &str) -> Self {
        Self {
            inner: quiz_core::i18n::QuizLocale::for_code(code),
        }
    }

    /// Get translated question by numeric ID.
    fn question(&self, id: u32) -> Option<QuestionLocale> {
        self.inner
            .question(id)
            .map(|q| QuestionLocale { inner: q.clone() })
    }

    /// Get translated category label.
    fn category_label(&self, category: Category) -> String {
        self.inner.category_label(&category.to_core()).to_string()
    }

    /// Get translated difficulty label.
    fn difficulty_label(&self, difficulty: Difficulty) -> String {
        self.inner
            .difficulty_label(&difficulty.to_core())
            .to_string()
    }

    // UI string accessors
    fn ui_title(&self) -> &str {
        &self.inner.ui.title
    }

    fn ui_correct(&self) -> &str {
        &self.inner.ui.correct
    }

    fn ui_wrong(&self) -> &str {
        &self.inner.ui.wrong
    }

    fn ui_start_quiz(&self) -> &str {
        &self.inner.ui.start_quiz
    }

    fn ui_next_question(&self) -> &str {
        &self.inner.ui.next_question
    }

    fn ui_see_results(&self) -> &str {
        &self.inner.ui.see_results
    }

    fn ui_skip(&self) -> &str {
        &self.inner.ui.skip
    }

    fn ui_try_again(&self) -> &str {
        &self.inner.ui.try_again_btn
    }

    fn __repr__(&self) -> String {
        format!("QuizLocale(\"{}\")", self.inner.ui.title)
    }
}

// ---------------------------------------------------------------------------
// Module
// ---------------------------------------------------------------------------

#[pymodule]
fn eulumdat_quiz(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Category>()?;
    m.add_class::<Difficulty>()?;
    m.add_class::<Question>()?;
    m.add_class::<AnswerResult>()?;
    m.add_class::<QuizConfig>()?;
    m.add_class::<QuizSession>()?;
    m.add_class::<QuizBank>()?;
    m.add_class::<QuizScore>()?;
    m.add_class::<CategoryScore>()?;
    m.add_class::<DifficultyScore>()?;
    m.add_class::<QuizLocale>()?;
    m.add_class::<QuestionLocale>()?;
    Ok(())
}
