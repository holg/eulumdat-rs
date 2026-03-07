use eulumdat_quiz::i18n::QuizLocale;
use eulumdat_quiz::QuizScore;
use ratatui::prelude::*;
use ratatui::widgets::*;

pub struct ResultsState {
    pub score: QuizScore,
    pub scroll_offset: u16,
}

impl ResultsState {
    pub fn new(score: QuizScore) -> Self {
        Self {
            score,
            scroll_offset: 0,
        }
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset += 1;
    }
}

pub fn draw(frame: &mut Frame, area: Rect, state: &ResultsState, locale: &QuizLocale) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let pct = state.score.percentage();
    let grade = grade_label(pct, locale);
    let g_color = grade_color(pct);

    let mut lines = Vec::new();

    // Grade header
    lines.push(Line::default());
    lines.push(
        Line::from(Span::styled(
            format!("\u{2550}\u{2550} {} \u{2550}\u{2550}", grade),
            Style::new().fg(g_color).bold(),
        ))
        .alignment(Alignment::Center),
    );

    lines.push(
        Line::from(Span::styled(
            format!("{:.0}%", pct),
            Style::new().fg(g_color).bold(),
        ))
        .alignment(Alignment::Center),
    );

    let detail = QuizLocale::format(
        &locale.ui.score_detail,
        &[
            &state.score.correct,
            &state.score.wrong,
            &state.score.skipped,
            &state.score.total,
        ],
    );
    lines.push(
        Line::from(Span::styled(detail, Style::new().fg(Color::White)))
            .alignment(Alignment::Center),
    );

    lines.push(Line::default());

    // By Category
    lines.push(Line::from(Span::styled(
        format!("  {}", locale.ui.by_category),
        Style::new().bold(),
    )));

    let mut cat_scores = state.score.by_category.clone();
    cat_scores.sort_by(|a, b| {
        let pct_a = if a.total > 0 {
            a.correct as f64 / a.total as f64
        } else {
            0.0
        };
        let pct_b = if b.total > 0 {
            b.correct as f64 / b.total as f64
        } else {
            0.0
        };
        pct_b.partial_cmp(&pct_a).unwrap()
    });

    for cs in &cat_scores {
        let cat_pct = if cs.total > 0 {
            cs.correct as f64 / cs.total as f64 * 100.0
        } else {
            0.0
        };
        let label = locale.category_label(&cs.category);
        let bar = make_bar(cat_pct, 20);
        let color = grade_color(cat_pct);
        lines.push(Line::from(vec![
            Span::styled(format!("  {:<25}", label), Style::default()),
            Span::styled(bar, Style::new().fg(color)),
            Span::styled(
                format!("  {}/{} {:>3.0}%", cs.correct, cs.total, cat_pct),
                Style::new().fg(Color::DarkGray),
            ),
        ]));
    }

    lines.push(Line::default());

    // By Difficulty
    lines.push(Line::from(Span::styled(
        format!("  {}", locale.ui.by_difficulty),
        Style::new().bold(),
    )));

    let diff_order = [
        eulumdat_quiz::Difficulty::Beginner,
        eulumdat_quiz::Difficulty::Intermediate,
        eulumdat_quiz::Difficulty::Expert,
    ];

    for diff in &diff_order {
        if let Some(ds) = state
            .score
            .by_difficulty
            .iter()
            .find(|d| d.difficulty == *diff)
        {
            let diff_pct = if ds.total > 0 {
                ds.correct as f64 / ds.total as f64 * 100.0
            } else {
                0.0
            };
            let label = locale.difficulty_label(&ds.difficulty);
            let bar = make_bar(diff_pct, 20);
            let color = grade_color(diff_pct);
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<25}", label), Style::default()),
                Span::styled(bar, Style::new().fg(color)),
                Span::styled(
                    format!("  {}/{} {:>3.0}%", ds.correct, ds.total, diff_pct),
                    Style::new().fg(Color::DarkGray),
                ),
            ]));
        }
    }

    lines.push(Line::default());
    lines.push(Line::default());

    // Buttons
    lines.push(
        Line::from(vec![Span::styled(
            format!("[ {} ]", locale.ui.try_again_btn),
            Style::new().fg(Color::Cyan).bold(),
        )])
        .alignment(Alignment::Center),
    );

    frame.render_widget(
        Paragraph::new(lines).scroll((state.scroll_offset, 0)),
        inner,
    );
}

fn grade_label(pct: f64, locale: &QuizLocale) -> &str {
    if pct >= 90.0 {
        &locale.ui.excellent
    } else if pct >= 70.0 {
        &locale.ui.good_job
    } else if pct >= 50.0 {
        &locale.ui.keep_learning
    } else {
        &locale.ui.try_again
    }
}

fn grade_color(pct: f64) -> Color {
    if pct >= 90.0 {
        Color::Green
    } else if pct >= 70.0 {
        Color::Blue
    } else if pct >= 50.0 {
        Color::Yellow
    } else {
        Color::Red
    }
}

fn make_bar(pct: f64, width: usize) -> String {
    let filled = (pct / 100.0 * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("{}{}", "\u{2593}".repeat(filled), "\u{2591}".repeat(empty),)
}
