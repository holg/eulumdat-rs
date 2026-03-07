# eulumdat-quiz

Python bindings for the [eulumdat-rs](https://github.com/holg/eulumdat-rs) photometric knowledge quiz engine.

195 multiple-choice questions across 16 categories covering EULUMDAT, IES, photometric calculations, BUG ratings, color science, and more. Fully translated into 8 languages.

## Installation

```bash
pip install eulumdat-quiz
```

## Command-Line Quiz

After installation, you can run the quiz directly from the command line:

```bash
# Text-based interactive quiz
eulumdat-quiz

# Or via Python module
python -m eulumdat_quiz

# Launch the TUI version (requires: cargo install eulumdat-tui-quiz)
python -m eulumdat_quiz --tui
```

## Quick Start

```python
import eulumdat_quiz as quiz

# Browse the question bank
print(f"Total questions: {quiz.QuizBank.total_count()}")
for cat, count in quiz.QuizBank.categories():
    print(f"  {cat.label()}: {count} questions")

# Create a quiz session
config = quiz.QuizConfig(
    categories=[quiz.Category.Symmetry, quiz.Category.BugRating],
    difficulty=quiz.Difficulty.Beginner,
    num_questions=5,
)
session = quiz.QuizSession(config)

# Run through questions
while not session.is_finished():
    q = session.current_question()
    print(f"\n{q.text}")
    for i, opt in enumerate(q.options):
        print(f"  {chr(65+i)}) {opt}")

    choice = int(input("Answer (0-3): "))
    result = session.answer(choice)
    if result.is_correct:
        print("Correct!")
    else:
        print(f"Wrong! Correct answer: {chr(65 + result.correct_index)}")
    print(f"Explanation: {result.explanation}")

# Show final score
score = session.score()
print(f"\nScore: {score.correct}/{score.total} ({score.percentage():.0f}%)")
```

## i18n Support

All questions and UI strings are available in 8 languages.

```python
import eulumdat_quiz as quiz

# Load German translations
locale = quiz.QuizLocale.for_code("de")
print(locale.ui_title())  # "Photometrisches Wissensquiz"

# Get translated question
q = session.current_question()
translated = locale.question(q.id)
if translated:
    print(translated.text)
    for opt in translated.options:
        print(f"  - {opt}")
```

**Supported languages:** English, Deutsch, 简体中文, Français, Español, Italiano, Русский, Português (Brasil)

## Categories

| Category | Questions | Description |
|----------|-----------|-------------|
| EULUMDAT Format | 15 | LDT file structure, fields |
| IES Format | 12 | LM-63 versions, keywords |
| Symmetry | 12 | 5 symmetry types, data expansion |
| Coordinate Systems | 12 | C/gamma angles, nadir/zenith |
| Photometric Calculations | 15 | LOR, DFF, beam angles, efficacy |
| BUG Rating | 12 | TM-15-11 zones, thresholds |
| UGR & Glare | 10 | UGR formula, CIE 117 |
| Color Science | 15 | CCT, CRI, TM-30, SPD |
| Horticultural | 12 | PAR, PPF, PPFD, DLI |
| BIM Integration | 10 | TM-32-24, NEMA GUIDs |
| Modern Formats | 10 | TM-33-23/ATLA, XML/JSON |
| Validation | 10 | Warning/error codes |
| Units & Conversions | 12 | lux/fc, m/ft, cd/klm |
| Diagram Types | 10 | Polar, cartesian, heatmap |
| Diagram Reading | 20 | Interpreting diagrams |
| Standards | 8 | CIE, IES, NEMA, EN 13201 |

## License

AGPL-3.0-or-later
