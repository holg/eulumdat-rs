use eulumdat_quiz::i18n::QuizLocale;
use eulumdat_quiz::{AnswerResult, Category, Question, QuizSession};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::diagram::TemplateLuminaires;

pub struct QuizState {
    pub session: QuizSession,
    pub feedback: Option<FeedbackState>,
    pub option_cursor: usize,
}

pub struct FeedbackState {
    pub question: Question,
    pub result: AnswerResult,
    pub chosen: u8,
    pub was_skip: bool,
}

impl QuizState {
    pub fn new(session: QuizSession) -> Self {
        Self {
            session,
            feedback: None,
            option_cursor: 0,
        }
    }

    pub fn answer_current(&mut self, choice: u8, locale: &QuizLocale) {
        if self.feedback.is_some() {
            return;
        }
        let question = match self.session.current_question() {
            Some(q) => apply_locale_to_question(q, locale),
            None => return,
        };
        let result = self.session.answer(choice);
        self.feedback = Some(FeedbackState {
            question,
            result,
            chosen: choice,
            was_skip: false,
        });
        self.option_cursor = 0;
    }

    pub fn skip_current(&mut self, locale: &QuizLocale) {
        if self.feedback.is_some() {
            return;
        }
        let question = match self.session.current_question() {
            Some(q) => apply_locale_to_question(q, locale),
            None => return,
        };
        let correct_index = question.correct_index;
        let explanation = question.explanation.clone();
        let reference = question.reference.clone();
        self.session.skip();
        self.feedback = Some(FeedbackState {
            question,
            result: AnswerResult {
                is_correct: false,
                correct_index,
                explanation,
                reference,
            },
            chosen: 255, // sentinel for skip
            was_skip: true,
        });
        self.option_cursor = 0;
    }
}

fn apply_locale_to_question(mut q: Question, locale: &QuizLocale) -> Question {
    if let Some(tl) = locale.question(q.id) {
        q.text = tl.text.clone();
        q.options = tl.options.clone();
        q.explanation = tl.explanation.clone();
    }
    q
}

pub fn draw(
    frame: &mut Frame,
    area: Rect,
    state: &QuizState,
    locale: &QuizLocale,
    templates: &TemplateLuminaires,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let layout = Layout::vertical([
        Constraint::Length(2), // Progress header
        Constraint::Length(1), // Progress bar
        Constraint::Length(1), // Spacer
        Constraint::Min(0),    // Question + options + feedback
    ])
    .split(inner);

    let (idx, total) = state.session.progress();
    let display_idx = if state.feedback.is_some() {
        idx // already advanced
    } else {
        idx + 1
    };
    let score = state.session.score();

    // Progress header
    let question_label = QuizLocale::format(&locale.ui.question_of, &[&display_idx, &total]);
    let score_label = QuizLocale::format(&locale.ui.correct_count, &[&score.correct, &score.wrong]);

    let progress_header =
        Layout::horizontal([Constraint::Min(0), Constraint::Length(30)]).split(layout[0]);

    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("  {}", question_label),
            Style::new().bold(),
        )),
        progress_header[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(score_label, Style::new().fg(Color::DarkGray)))
            .alignment(Alignment::Right),
        progress_header[1],
    );

    // Progress bar
    let pct = if total > 0 {
        (display_idx as f64 / total as f64 * 100.0) as u16
    } else {
        0
    };
    let gauge = Gauge::default()
        .gauge_style(Style::new().fg(Color::Cyan).bg(Color::DarkGray))
        .percent(pct)
        .label(format!("{}%", pct));
    frame.render_widget(
        gauge,
        Rect::new(
            layout[1].x + 2,
            layout[1].y,
            layout[1].width.saturating_sub(4),
            1,
        ),
    );

    // Question content
    let question = if let Some(fb) = &state.feedback {
        &fb.question
    } else {
        match &state.session.current_question() {
            Some(q) => {
                let q = apply_locale_to_question(q.clone(), locale);
                return draw_question_content(frame, layout[3], state, locale, &q, templates);
            }
            None => return,
        }
    };

    draw_question_content(frame, layout[3], state, locale, question, templates);
}

fn draw_question_content(
    frame: &mut Frame,
    area: Rect,
    state: &QuizState,
    locale: &QuizLocale,
    question: &Question,
    templates: &TemplateLuminaires,
) {
    let cat_label = locale.category_label(&question.category);
    let diff_label = locale.difficulty_label(&question.difficulty);

    // Pick diagram: per-question for DiagramReading, per-category otherwise
    let diagram_spec = if question.category == Category::DiagramReading {
        templates.diagram_for_question(question.id)
    } else {
        templates.diagram_for_category(&question.category)
    };
    let has_diagram = diagram_spec.is_some();

    // If diagram, split area: left = question, right = diagram
    // Only show diagram if terminal is wide enough (>=80 columns)
    let (question_area, diagram_area) = if has_diagram && area.width >= 80 {
        let splits = Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(area);
        (splits[0], Some(splits[1]))
    } else {
        (area, None)
    };

    // Draw diagram if present
    if let (Some(spec), Some(d_area)) = (&diagram_spec, diagram_area) {
        crate::diagram::draw_diagram(frame, d_area, spec, locale);
    }

    // Question box with border
    let q_block = Block::default()
        .title_top(format!(" {} ", cat_label))
        .title_top(Line::from(format!(" {} ", diff_label)).alignment(Alignment::Right))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(Color::Cyan));

    let q_inner = q_block.inner(question_area);
    frame.render_widget(q_block, question_area);

    // Split inner for question text + options + feedback
    let q_layout = Layout::vertical([
        Constraint::Length(3), // Question text
        Constraint::Length(1), // Spacer
        Constraint::Length(4), // Options
        Constraint::Min(0),    // Feedback
    ])
    .split(q_inner);

    // Question text
    frame.render_widget(
        Paragraph::new(format!("  {}", question.text)).wrap(Wrap { trim: false }),
        q_layout[0],
    );

    // Options
    let option_labels = ['A', 'B', 'C', 'D'];
    let lines: Vec<Line> = question
        .options
        .iter()
        .enumerate()
        .map(|(i, opt)| {
            let label = option_labels[i];
            let style = if let Some(fb) = &state.feedback {
                if i as u8 == fb.result.correct_index {
                    Style::new().fg(Color::Green).bold()
                } else if i as u8 == fb.chosen {
                    Style::new().fg(Color::Red).bold()
                } else {
                    Style::new().fg(Color::DarkGray)
                }
            } else if i == state.option_cursor {
                Style::new().fg(Color::Cyan).bold()
            } else {
                Style::default()
            };

            let prefix = if let Some(fb) = &state.feedback {
                if i as u8 == fb.result.correct_index {
                    "\u{2714} "
                } else if i as u8 == fb.chosen && !fb.result.is_correct {
                    "\u{2718} "
                } else {
                    "  "
                }
            } else if i == state.option_cursor {
                "\u{25b6} "
            } else {
                "  "
            };

            Line::from(Span::styled(
                format!("  {}{}) {}", prefix, label, opt),
                style,
            ))
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), q_layout[2]);

    // Feedback
    if let Some(fb) = &state.feedback {
        let mut fb_lines = Vec::new();
        fb_lines.push(Line::default());

        let result_text = if fb.was_skip {
            format!("  {} {}", locale.ui.skip, locale.ui.correct)
        } else if fb.result.is_correct {
            format!("  \u{2714} {}", locale.ui.correct)
        } else {
            format!("  \u{2718} {}", locale.ui.wrong)
        };

        let result_style = if fb.was_skip {
            Style::new().fg(Color::Yellow).bold()
        } else if fb.result.is_correct {
            Style::new().fg(Color::Green).bold()
        } else {
            Style::new().fg(Color::Red).bold()
        };

        fb_lines.push(Line::from(Span::styled(result_text, result_style)));
        fb_lines.push(Line::default());

        fb_lines.push(Line::from(Span::styled(
            format!("  {}", fb.result.explanation),
            Style::new().fg(Color::White),
        )));

        if let Some(ref reference) = fb.result.reference {
            fb_lines.push(Line::from(Span::styled(
                format!("  {}: {}", locale.ui.reference, reference),
                Style::new().fg(Color::DarkGray).italic(),
            )));
        }

        let next_label = if state.session.is_finished() {
            &locale.ui.see_results
        } else {
            &locale.ui.next_question
        };
        fb_lines.push(Line::from(Span::styled(
            format!("  [Enter: {}]", next_label),
            Style::new().fg(Color::Cyan),
        )));

        frame.render_widget(
            Paragraph::new(fb_lines).wrap(Wrap { trim: false }),
            q_layout[3],
        );
    }
}
