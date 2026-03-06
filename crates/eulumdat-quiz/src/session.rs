use crate::{
    AnswerResult, Category, CategoryScore, Difficulty, DifficultyScore, Question, QuizConfig,
    QuizScore,
};

/// A quiz session that tracks progress and scoring.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct QuizSession {
    pub questions: Vec<Question>,
    pub answers: Vec<Option<u8>>,
    pub current_index: usize,
}

impl QuizSession {
    /// Create a new session from config.
    pub fn new(config: QuizConfig) -> Self {
        let all = crate::questions::all_questions();

        let mut filtered: Vec<Question> = all
            .into_iter()
            .filter(|q| {
                if !config.categories.is_empty() && !config.categories.contains(&q.category) {
                    return false;
                }
                if let Some(ref diff) = config.difficulty {
                    if q.difficulty != *diff {
                        return false;
                    }
                }
                true
            })
            .collect();

        if config.shuffle {
            // Simple seeded shuffle (xorshift64)
            let mut rng = config.seed.unwrap_or(0x517cc1b727220a95);
            for i in (1..filtered.len()).rev() {
                rng ^= rng << 13;
                rng ^= rng >> 7;
                rng ^= rng << 17;
                let j = (rng as usize) % (i + 1);
                filtered.swap(i, j);
            }
        }

        if config.num_questions > 0 {
            filtered.truncate(config.num_questions as usize);
        }

        let len = filtered.len();
        Self {
            questions: filtered,
            answers: vec![None; len],
            current_index: 0,
        }
    }

    /// Get the current question (None if finished).
    pub fn current_question(&self) -> Option<Question> {
        self.questions.get(self.current_index).cloned()
    }

    /// Submit answer for current question, advance to next.
    pub fn answer(&mut self, choice: u8) -> AnswerResult {
        let q = &self.questions[self.current_index];
        let is_correct = choice == q.correct_index;
        let result = AnswerResult {
            is_correct,
            correct_index: q.correct_index,
            explanation: q.explanation.clone(),
            reference: q.reference.clone(),
        };
        self.answers[self.current_index] = Some(choice);
        self.current_index += 1;
        result
    }

    /// Skip current question, advance to next. Returns false if already finished.
    pub fn skip(&mut self) -> bool {
        if self.is_finished() {
            return false;
        }
        // answers[current_index] stays None = skipped
        self.current_index += 1;
        true
    }

    /// Is the quiz finished?
    pub fn is_finished(&self) -> bool {
        self.current_index >= self.questions.len()
    }

    /// Current progress (0-based index, total).
    pub fn progress(&self) -> (usize, usize) {
        (self.current_index, self.questions.len())
    }

    /// Current score (updates live).
    pub fn score(&self) -> QuizScore {
        let mut correct = 0u32;
        let mut wrong = 0u32;
        let mut skipped = 0u32;

        // Count by category
        let mut cat_correct: std::collections::HashMap<Category, u32> =
            std::collections::HashMap::new();
        let mut cat_total: std::collections::HashMap<Category, u32> =
            std::collections::HashMap::new();

        // Count by difficulty
        let mut diff_correct: std::collections::HashMap<Difficulty, u32> =
            std::collections::HashMap::new();
        let mut diff_total: std::collections::HashMap<Difficulty, u32> =
            std::collections::HashMap::new();

        for (i, q) in self.questions.iter().enumerate() {
            if i >= self.current_index {
                break;
            }
            *cat_total.entry(q.category.clone()).or_default() += 1;
            *diff_total.entry(q.difficulty.clone()).or_default() += 1;

            match self.answers[i] {
                Some(choice) => {
                    if choice == q.correct_index {
                        correct += 1;
                        *cat_correct.entry(q.category.clone()).or_default() += 1;
                        *diff_correct.entry(q.difficulty.clone()).or_default() += 1;
                    } else {
                        wrong += 1;
                    }
                }
                None => {
                    skipped += 1;
                }
            }
        }

        let by_category: Vec<CategoryScore> = cat_total
            .into_iter()
            .map(|(cat, total)| CategoryScore {
                correct: *cat_correct.get(&cat).unwrap_or(&0),
                category: cat,
                total,
            })
            .collect();

        let by_difficulty: Vec<DifficultyScore> = diff_total
            .into_iter()
            .map(|(diff, total)| DifficultyScore {
                correct: *diff_correct.get(&diff).unwrap_or(&0),
                difficulty: diff,
                total,
            })
            .collect();

        QuizScore {
            correct,
            wrong,
            skipped,
            total: self.questions.len() as u32,
            by_category,
            by_difficulty,
        }
    }
}
