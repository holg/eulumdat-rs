use eulumdat_quiz::i18n::QuizLocale;
use eulumdat_quiz::{Category, QuizBank};
use ratatui::prelude::*;
use ratatui::widgets::*;

pub struct ConfigState {
    pub categories: Vec<(Category, bool, u32)>,
    pub difficulty_cursor: usize, // 0=All, 1=Beg, 2=Int, 3=Exp
    pub count_cursor: usize,      // 0=5, 1=10, 2=20, 3=50, 4=All
    pub focus: ConfigFocus,
    pub category_cursor: usize,
}

const COUNT_OPTIONS: [u32; 5] = [5, 10, 20, 50, 0];

#[derive(PartialEq)]
pub enum ConfigFocus {
    Categories,
    Difficulty,
    QuestionCount,
    StartButton,
}

impl ConfigState {
    pub fn new(_locale: &QuizLocale) -> Self {
        let bank_cats = QuizBank::categories();
        let categories: Vec<(Category, bool, u32)> =
            bank_cats.into_iter().map(|(c, n)| (c, true, n)).collect();
        Self {
            categories,
            difficulty_cursor: 0,
            count_cursor: 1, // default 10
            focus: ConfigFocus::Categories,
            category_cursor: 0,
        }
    }

    pub fn next_focus(&mut self) {
        self.focus = match self.focus {
            ConfigFocus::Categories => ConfigFocus::Difficulty,
            ConfigFocus::Difficulty => ConfigFocus::QuestionCount,
            ConfigFocus::QuestionCount => ConfigFocus::StartButton,
            ConfigFocus::StartButton => ConfigFocus::Categories,
        };
    }

    pub fn prev_focus(&mut self) {
        self.focus = match self.focus {
            ConfigFocus::Categories => ConfigFocus::StartButton,
            ConfigFocus::Difficulty => ConfigFocus::Categories,
            ConfigFocus::QuestionCount => ConfigFocus::Difficulty,
            ConfigFocus::StartButton => ConfigFocus::QuestionCount,
        };
    }

    pub fn cursor_up(&mut self) {
        if self.focus == ConfigFocus::Categories && self.category_cursor > 0 {
            self.category_cursor -= 1;
        }
    }

    pub fn cursor_down(&mut self) {
        if self.focus == ConfigFocus::Categories && self.category_cursor + 1 < self.categories.len()
        {
            self.category_cursor += 1;
        }
    }

    pub fn cursor_left(&mut self) {
        match self.focus {
            ConfigFocus::Difficulty => {
                if self.difficulty_cursor > 0 {
                    self.difficulty_cursor -= 1;
                }
            }
            ConfigFocus::QuestionCount => {
                if self.count_cursor > 0 {
                    self.count_cursor -= 1;
                }
            }
            _ => {}
        }
    }

    pub fn cursor_right(&mut self) {
        match self.focus {
            ConfigFocus::Difficulty => {
                if self.difficulty_cursor < 3 {
                    self.difficulty_cursor += 1;
                }
            }
            ConfigFocus::QuestionCount => {
                if self.count_cursor < 4 {
                    self.count_cursor += 1;
                }
            }
            _ => {}
        }
    }

    pub fn toggle_category(&mut self) {
        if let Some(item) = self.categories.get_mut(self.category_cursor) {
            item.1 = !item.1;
        }
    }

    pub fn select_all(&mut self) {
        for item in &mut self.categories {
            item.1 = true;
        }
    }

    pub fn select_none(&mut self) {
        for item in &mut self.categories {
            item.1 = false;
        }
    }

    pub fn selected_question_count(&self) -> u32 {
        self.categories
            .iter()
            .filter(|(_, sel, _)| *sel)
            .map(|(_, _, n)| *n)
            .sum()
    }

    pub fn total_question_count(&self) -> u32 {
        self.categories.iter().map(|(_, _, n)| *n).sum()
    }

    pub fn selected_count_value(&self) -> u32 {
        COUNT_OPTIONS[self.count_cursor]
    }
}

pub fn draw(frame: &mut Frame, area: Rect, state: &ConfigState, locale: &QuizLocale) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Main content layout
    let layout = Layout::vertical([
        Constraint::Length(3), // Header
        Constraint::Min(0),    // Categories
        Constraint::Length(3), // Difficulty
        Constraint::Length(3), // Count
        Constraint::Length(2), // Summary + Start
    ])
    .split(inner);

    // Header
    let header_text = format!(
        "  {}",
        QuizLocale::format(
            &locale.ui.questions_across,
            &[&state.total_question_count(), &state.categories.len()],
        ),
    );
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                format!("  {}", locale.ui.configure),
                Style::new().bold(),
            )),
            Line::from(Span::styled(header_text, Style::new().fg(Color::DarkGray))),
        ]),
        layout[0],
    );

    // Categories section
    draw_categories(frame, layout[1], state, locale);

    // Difficulty section
    draw_difficulty(frame, layout[2], state, locale);

    // Question count section
    draw_count(frame, layout[3], state, locale);

    // Summary + Start button
    draw_start(frame, layout[4], state, locale);
}

fn draw_categories(frame: &mut Frame, area: Rect, state: &ConfigState, locale: &QuizLocale) {
    let is_focused = matches!(state.focus, ConfigFocus::Categories);
    let title_style = if is_focused {
        Style::new().fg(Color::Cyan).bold()
    } else {
        Style::new().fg(Color::White).bold()
    };

    let header = Layout::horizontal([Constraint::Min(0), Constraint::Length(20)])
        .split(Rect::new(area.x, area.y, area.width, 1));

    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("  {}", locale.ui.categories),
            title_style,
        )),
        header[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("[A]{} [N]{}", locale.ui.select_all, locale.ui.select_none),
            Style::new().fg(Color::DarkGray),
        ))
        .alignment(Alignment::Right),
        header[1],
    );

    let list_area = Rect::new(
        area.x,
        area.y + 1,
        area.width,
        area.height.saturating_sub(1),
    );

    let items: Vec<Line> = state
        .categories
        .iter()
        .enumerate()
        .map(|(i, (cat, selected, count))| {
            let check = if *selected { "x" } else { " " };
            let cursor = if is_focused && i == state.category_cursor {
                ">"
            } else {
                " "
            };
            let label = locale.category_label(cat);
            let text = format!(" {} [{}] {:<30} ({:>2})", cursor, check, label, count);
            let style = if is_focused && i == state.category_cursor {
                Style::new().fg(Color::Cyan)
            } else if *selected {
                Style::default()
            } else {
                Style::new().fg(Color::DarkGray)
            };
            Line::from(Span::styled(text, style))
        })
        .collect();

    frame.render_widget(Paragraph::new(items), list_area);
}

fn draw_difficulty(frame: &mut Frame, area: Rect, state: &ConfigState, locale: &QuizLocale) {
    let is_focused = matches!(state.focus, ConfigFocus::Difficulty);
    let title_style = if is_focused {
        Style::new().fg(Color::Cyan).bold()
    } else {
        Style::new().fg(Color::White).bold()
    };

    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("  {}", locale.ui.difficulty),
            title_style,
        )),
        Rect::new(area.x, area.y, area.width, 1),
    );

    let labels = [
        &locale.ui.all_levels,
        &locale.ui.beginner,
        &locale.ui.intermediate,
        &locale.ui.expert,
    ];

    let options: Vec<Span> = labels
        .iter()
        .enumerate()
        .flat_map(|(i, label)| {
            let selected = i == state.difficulty_cursor;
            let dot = if selected { "\u{25cf}" } else { "\u{25cb}" };
            let style = if is_focused && selected {
                Style::new().fg(Color::Cyan).bold()
            } else if selected {
                Style::new().bold()
            } else {
                Style::new().fg(Color::DarkGray)
            };
            vec![Span::styled(format!("  {} {}", dot, label), style)]
        })
        .collect();

    frame.render_widget(
        Paragraph::new(Line::from(options)),
        Rect::new(area.x, area.y + 1, area.width, 1),
    );
}

fn draw_count(frame: &mut Frame, area: Rect, state: &ConfigState, locale: &QuizLocale) {
    let is_focused = matches!(state.focus, ConfigFocus::QuestionCount);
    let title_style = if is_focused {
        Style::new().fg(Color::Cyan).bold()
    } else {
        Style::new().fg(Color::White).bold()
    };

    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("  {}", locale.ui.num_questions),
            title_style,
        )),
        Rect::new(area.x, area.y, area.width, 1),
    );

    let labels = ["5", "10", "20", "50", "All"];
    let options: Vec<Span> = labels
        .iter()
        .enumerate()
        .flat_map(|(i, label)| {
            let selected = i == state.count_cursor;
            let style = if is_focused && selected {
                Style::new().fg(Color::Black).bg(Color::Cyan).bold()
            } else if selected {
                Style::new().fg(Color::Black).bg(Color::White).bold()
            } else {
                Style::new().fg(Color::DarkGray)
            };
            vec![Span::styled(format!("  [ {} ]", label), style)]
        })
        .collect();

    frame.render_widget(
        Paragraph::new(Line::from(options)),
        Rect::new(area.x, area.y + 1, area.width, 1),
    );
}

fn draw_start(frame: &mut Frame, area: Rect, state: &ConfigState, locale: &QuizLocale) {
    let is_focused = matches!(state.focus, ConfigFocus::StartButton);
    let selected = state.selected_question_count();
    let total = state.total_question_count();

    let summary = QuizLocale::format(&locale.ui.questions_selected, &[&selected, &total]);

    let button_style = if is_focused {
        Style::new().fg(Color::Black).bg(Color::Cyan).bold()
    } else {
        Style::new().fg(Color::Cyan).bold()
    };

    let line = Line::from(vec![
        Span::styled(format!("  {}", summary), Style::new().fg(Color::DarkGray)),
        Span::raw("    "),
        Span::styled(
            format!("[ {} \u{2192} ]", locale.ui.start_quiz),
            button_style,
        ),
    ]);

    frame.render_widget(Paragraph::new(line), area);
}
