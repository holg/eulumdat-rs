# Plan: `eulumdat-quiz` — Shared Photometric Knowledge Quiz Library

## Goal

A pure Rust quiz engine library with zero UI dependencies, exposable through every frontend:
TUI (ratatui), Web (Leptos/WASM), Desktop (egui), iOS/macOS (SwiftUI via uniffi),
Android (Kotlin via uniffi), Python (PyO3).

---

## Architecture

```
crates/eulumdat-quiz/src/lib.rs       ← Pure Rust: QuizBank, Question, Session, Scoring
    │
    ├── eulumdat-tui                   (ratatui quiz mode)
    ├── eulumdat-wasm                  (Leptos component)
    ├── eulumdat-desktop               (egui panel)
    ├── eulumdat-ffi                   (uniffi → Swift/Kotlin)
    │   ├── EulumdatApp (iOS/macOS)
    │   └── EulumdatAndroid
    └── eulumdat-py                    (PyO3 bindings)
```

### Design Constraints (uniffi/FFI-safe)

- All public types: plain structs, enums, Vecs, Strings, Option — no generics, traits, closures
- No `&str` in public API — use `String`
- Enums: simple C-style or with named struct variants (uniffi-compatible)
- No lifetimes in public types
- `#[derive(Clone, Debug)]` on everything; `serde::Serialize/Deserialize` behind feature flag

---

## Core Types

### `crates/eulumdat-quiz/src/lib.rs`

```rust
/// Knowledge domain categories
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Category {
    EulumdatFormat,      // LDT file structure, line numbers, field meanings
    IesFormat,           // LM-63 versions, keywords, photometric types A/B/C
    Symmetry,            // 5 symmetry types, data expansion, compression ratios
    CoordinateSystems,   // C-plane angles, gamma angles, nadir/zenith, Type B↔C
    PhotometricCalc,     // LOR, DFF, beam/field angles, CIE flux codes, efficacy
    BugRating,           // TM-15-11 zones, thresholds, B/U/G 0-5 scale
    UgrGlare,            // UGR formula, standard rooms, CIE 117
    ColorScience,        // CCT, CRI groups, TM-30 Rf/Rg, Duv, SPD
    Horticultural,       // PAR, PPF, PPFD, DLI, R:FR ratio, spectral zones
    BimIntegration,      // TM-32-24 parameters, NEMA GUIDs, housing shapes
    ModernFormats,       // TM-33-23/ATLA S001, XML vs JSON, spectral support
    Validation,          // Warning codes W001-W046, error codes E001-E006
    Units,               // lux/fc, m/ft, mm/in, cd/klm, lm/W
    DiagramTypes,        // Polar, cartesian, heatmap, cone, butterfly, isolux
    Standards,           // CIE, IES, NEMA, EN 13201, IDA, LEED, Title 24
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Difficulty {
    Beginner,       // Format basics, unit definitions, simple facts
    Intermediate,   // Calculations, thresholds, standard comparisons
    Expert,         // Cross-standard nuances, edge cases, formulas
}

#[derive(Clone, Debug)]
pub struct Question {
    pub id: u32,
    pub category: Category,
    pub difficulty: Difficulty,
    pub text: String,
    pub options: Vec<String>,         // 4 choices (A-D)
    pub correct_index: u8,            // 0-3
    pub explanation: String,          // Shown after answering
    pub reference: Option<String>,    // e.g., "CIE S 017:2020", "LDT Line 3"
}

#[derive(Clone, Debug)]
pub struct QuizConfig {
    pub categories: Vec<Category>,    // Empty = all categories
    pub difficulty: Option<Difficulty>,// None = mixed
    pub num_questions: u32,           // 0 = all matching
    pub shuffle: bool,
    pub seed: Option<u64>,            // Reproducible shuffle
}

#[derive(Clone, Debug)]
pub struct QuizSession {
    pub questions: Vec<Question>,
    pub answers: Vec<Option<u8>>,     // None = unanswered
    pub current_index: usize,
    pub score: QuizScore,
}

#[derive(Clone, Debug, Default)]
pub struct QuizScore {
    pub correct: u32,
    pub wrong: u32,
    pub skipped: u32,
    pub total: u32,
    pub by_category: Vec<CategoryScore>,
    pub by_difficulty: Vec<DifficultyScore>,
}

#[derive(Clone, Debug)]
pub struct CategoryScore {
    pub category: Category,
    pub correct: u32,
    pub total: u32,
}

#[derive(Clone, Debug)]
pub struct DifficultyScore {
    pub difficulty: Difficulty,
    pub correct: u32,
    pub total: u32,
}

#[derive(Clone, Debug)]
pub struct AnswerResult {
    pub is_correct: bool,
    pub correct_index: u8,
    pub explanation: String,
    pub reference: Option<String>,
}
```

### Public API

```rust
impl QuizBank {
    /// All questions in the bank
    pub fn all_questions() -> Vec<Question>;

    /// Available categories with question counts
    pub fn categories() -> Vec<(Category, u32)>;

    /// Total question count
    pub fn total_count() -> u32;
}

impl QuizSession {
    /// Create a new session from config
    pub fn new(config: QuizConfig) -> Self;

    /// Get the current question (None if finished)
    pub fn current_question(&self) -> Option<Question>;

    /// Submit answer for current question, advance to next
    pub fn answer(&mut self, choice: u8) -> AnswerResult;

    /// Skip current question, advance to next
    pub fn skip(&mut self) -> bool;

    /// Is the quiz finished?
    pub fn is_finished(&self) -> bool;

    /// Current progress (0-based index, total)
    pub fn progress(&self) -> (usize, usize);

    /// Final score (available at any time, updates live)
    pub fn score(&self) -> QuizScore;
}
```

---

## Question Bank: ~200+ Questions across 15 Categories

### 1. EULUMDAT Format (~25 questions)

| # | Difficulty | Question | Answer |
|---|-----------|----------|--------|
| 1 | Beginner | What file extension does EULUMDAT use? | `.ldt` |
| 2 | Beginner | What unit are intensity values stored in? | cd/klm (candelas per kilolumen) |
| 3 | Beginner | What unit are luminaire dimensions stored in? | Millimeters (mm) |
| 4 | Intermediate | What is Line 3 of an EULUMDAT file? | Symmetry indicator (Isym, 0-4) |
| 5 | Intermediate | How many lamp sets can a file contain (max)? | 20 |
| 6 | Intermediate | What is the maximum number of C-planes (Mc)? | 721 |
| 7 | Intermediate | Who proposed the EULUMDAT format and when? | Axel Stockmar, 1990 |
| 8 | Expert | How are European decimal commas handled? | Commas converted to dots before parsing |
| 9 | Expert | What encoding fallback is used for legacy files? | ISO-8859-1 (Latin-1) |
| 10 | Expert | What does a negative lamp count indicate? | Absolute photometry mode |

### 2. IES Format (~20 questions)

| # | Difficulty | Question | Answer |
|---|-----------|----------|--------|
| 1 | Beginner | What organization publishes the IES file format? | IESNA (Illuminating Engineering Society of North America) |
| 2 | Beginner | What standard defines the IES format? | ANSI/IESNA LM-63 |
| 3 | Intermediate | What are the three IES photometric types? | Type A (automotive), Type B (floodlight), Type C (architectural) |
| 4 | Intermediate | Which photometric type is most common? | Type C |
| 5 | Intermediate | How is absolute photometry indicated in IES? | lumens_per_lamp = -1 |
| 6 | Expert | What does file generation code 1.10100 mean? | Tested at accredited lab, lumen scaled |
| 7 | Expert | How many IES format versions exist? | 4 (LM-63-1991, 1995, 2002, 2019) |

### 3. Symmetry (~15 questions)

| # | Difficulty | Question | Answer |
|---|-----------|----------|--------|
| 1 | Beginner | How many symmetry types exist in EULUMDAT? | 5 (Isym 0-4) |
| 2 | Intermediate | How many C-planes are stored for VerticalAxis (Isym=1)? | 1 (single plane) |
| 3 | Intermediate | What does BothPlanes symmetry (Isym=4) store? | Quarter data (0-90°), Nc/4 + 1 planes |
| 4 | Expert | Can you query intensity at C=270° with BothPlanes symmetry? | Yes — automatic mirroring expands to full 360° |
| 5 | Expert | What is the compression ratio for VerticalAxis vs full 360°? | Up to 360:1 |

### 4. Coordinate Systems (~15 questions)

| # | Difficulty | Question | Answer |
|---|-----------|----------|--------|
| 1 | Beginner | What does gamma = 0° represent? | Nadir (straight down) |
| 2 | Beginner | What does gamma = 90° represent? | Horizontal |
| 3 | Beginner | What does C = 0° represent? | Front of luminaire |
| 4 | Intermediate | What is the gamma angle at zenith? | 180° |
| 5 | Expert | What transformation converts Type B (H,V) to Type C (C,γ)? | γ = arccos(cos(V)·cos(H)), C = atan2(sin(H), sin(V)·cos(H)) |

### 5. Photometric Calculations (~30 questions)

| # | Difficulty | Question | Answer |
|---|-----------|----------|--------|
| 1 | Beginner | What does LOR stand for? | Light Output Ratio |
| 2 | Beginner | What unit is luminous efficacy measured in? | lm/W (lumens per watt) |
| 3 | Intermediate | What is the IES beam angle definition? | Full angle where intensity ≥ 50% of maximum |
| 4 | Intermediate | What is the CIE beam angle definition? | Full angle where intensity ≥ 50% of center-beam intensity |
| 5 | Intermediate | When do IES and CIE beam angles differ significantly? | Batwing distributions (center < max) |
| 6 | Intermediate | What is field angle based on? | 10% of maximum (IES) or center-beam (CIE) intensity |
| 7 | Intermediate | Are beam/field angles half-angles or full angles? | Full angles (per CIE S 017:2020) |
| 8 | Expert | What are CIE flux codes N1-N5? | N1=DLOR, N2=0-60°, N3=0-40°, N4=ULOR, N5=90-120° |
| 9 | Expert | What formula calculates UGR? | UGR = 8 × log₁₀((0.25/Lb) × Σ(L²×ω/p²)) |
| 10 | Expert | What are the 10 standard room indices for CU tables? | 0.60, 0.80, 1.00, 1.25, 1.50, 2.00, 2.50, 3.00, 4.00, 5.00 |

### 6. BUG Rating (~20 questions)

| # | Difficulty | Question | Answer |
|---|-----------|----------|--------|
| 1 | Beginner | What does BUG stand for? | Backlight, Uplight, Glare |
| 2 | Beginner | What standard defines BUG ratings? | IESNA TM-15-11 |
| 3 | Beginner | Is a lower or higher BUG rating better? | Lower (B0-U0-G0 is best) |
| 4 | Intermediate | What angle range defines uplight? | Above horizontal (90-180° from nadir) |
| 5 | Intermediate | What compliance programs use BUG ratings? | IDA/IES MLO, LEED v4, California Title 24 |
| 6 | Expert | What is the lumen threshold for U0 (zero uplight)? | 0 lumens in both UL and UH zones |
| 7 | Expert | What angle range is "Backlight Very High" (BVH)? | 80-90° from nadir |

### 7. Color Science (~20 questions)

| # | Difficulty | Question | Answer |
|---|-----------|----------|--------|
| 1 | Beginner | What unit is color temperature measured in? | Kelvin (K) |
| 2 | Beginner | Is 2700K warm or cool? | Warm (incandescent-like) |
| 3 | Intermediate | What CRI group has Ra ≥ 90? | 1A |
| 4 | Intermediate | What does TM-30 Rf measure? | Fidelity (color accuracy, 0-100) |
| 5 | Intermediate | What does TM-30 Rg measure? | Gamut (color saturation, 60-140, 100=reference) |
| 6 | Expert | What does a positive Duv value mean? | Greenish tint (above Planckian locus) |
| 7 | Expert | Why is R9 reported separately from Ra? | Deep red rendering is critical but averaged out in Ra |

### 8. Horticultural Lighting (~15 questions)

| # | Difficulty | Question | Answer |
|---|-----------|----------|--------|
| 1 | Beginner | What wavelength range is PAR? | 400-700nm |
| 2 | Beginner | What unit is PPFD measured in? | µmol/(m²·s) |
| 3 | Intermediate | What spectral zone promotes flowering in plants? | Red (600-700nm) |
| 4 | Intermediate | What does R:FR ratio control in plants? | Morphology — high R:FR = compact, low = elongated |
| 5 | Expert | What is DLI and what are typical values? | Daily Light Integral, mol/(m²·day); seedlings 3-6, flowering 14-25 |

### 9. BIM Integration (~10 questions)

| # | Difficulty | Question | Answer |
|---|-----------|----------|--------|
| 1 | Intermediate | What standard defines BIM parameters for lighting? | ANSI/IES TM-32-24 |
| 2 | Intermediate | What two housing shapes does TM-32 define? | Cuboid and Cylinder |
| 3 | Expert | What is melanopic factor used for? | Circadian rhythm impact assessment (melatonin suppression) |

### 10. Modern Formats (~10 questions)

| # | Difficulty | Question | Answer |
|---|-----------|----------|--------|
| 1 | Intermediate | What formats are equivalent to TM-33? | ATLA S001, UNI 11733 |
| 2 | Intermediate | What data type does TM-33 support that LDT/IES cannot? | Full spectral power distribution (SPD) |
| 3 | Expert | How much smaller are TM-33 JSON files vs XML? | ~90% smaller |

### 11-15. Remaining Categories

Similar depth for Validation (W001-W046 codes), Units (conversion factors),
Diagram Types (polar/cartesian/etc.), Standards (CIE/IES/NEMA/EN), totaling 200+ questions.

---

## File Structure

```
crates/eulumdat-quiz/
├── Cargo.toml
├── src/
│   ├── lib.rs          # Public types, QuizBank, QuizSession
│   ├── questions/
│   │   ├── mod.rs      # QuestionBuilder, bank assembly
│   │   ├── eulumdat_format.rs
│   │   ├── ies_format.rs
│   │   ├── symmetry.rs
│   │   ├── coordinates.rs
│   │   ├── calculations.rs
│   │   ├── bug_rating.rs
│   │   ├── color_science.rs
│   │   ├── horticultural.rs
│   │   ├── bim.rs
│   │   ├── modern_formats.rs
│   │   ├── validation.rs
│   │   ├── units.rs
│   │   ├── diagrams.rs
│   │   └── standards.rs
│   └── session.rs      # QuizSession state machine, scoring
```

## Cargo.toml

```toml
[package]
name = "eulumdat-quiz"
version.workspace = true
edition.workspace = true
description = "Photometric knowledge quiz engine for lighting professionals"

[features]
default = []
serde = ["dep:serde"]

[dependencies]
serde = { version = "1.0", features = ["derive"], optional = true }

[dev-dependencies]
```

No external dependencies in the default build — just pure Rust with `std`.
Optional `serde` feature for JSON serialization (useful for WASM/API).

---

## FFI Integration Points

### uniffi (Swift/Kotlin)

Add to `crates/eulumdat-ffi/src/lib.rs`:

```rust
// Quiz types exposed via uniffi
pub use eulumdat_quiz::{
    AnswerResult, Category, Difficulty, Question,
    QuizConfig, QuizScore, QuizSession,
};
```

All types are uniffi-safe (no lifetimes, no generics, no closures).

### PyO3 (Python)

Add to `crates/eulumdat-py/`:

```python
import eulumdat

session = eulumdat.QuizSession(categories=["BugRating"], difficulty="Intermediate", num_questions=10)
while not session.is_finished():
    q = session.current_question()
    print(q.text)
    for i, opt in enumerate(q.options):
        print(f"  {chr(65+i)}) {opt}")
    answer = int(input("Choice (0-3): "))
    result = session.answer(answer)
    print("Correct!" if result.is_correct else f"Wrong. {result.explanation}")
```

### WASM (Leptos)

Direct Rust usage — no FFI needed. Add `eulumdat-quiz` as dependency.

---

## Verification

1. `cargo test -p eulumdat-quiz` — all questions have valid correct_index, no duplicate IDs
2. `cargo run -p eulumdat-tui -- --quiz` — TUI quiz mode works
3. Each category has ≥ 5 questions
4. Each difficulty level has ≥ 20 questions
5. All explanations reference the authoritative standard
6. `cargo clippy -p eulumdat-quiz -- -D warnings` — clean
