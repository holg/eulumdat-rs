use crossterm::event::KeyCode;
use eulumdat_i18n::Language;
use eulumdat_quiz::i18n::QuizLocale;
use eulumdat_quiz::{Category, Difficulty, QuizConfig, QuizSession};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::config_screen::{ConfigFocus, ConfigState};
use crate::diagram::TemplateLuminaires;
use crate::quiz_screen::QuizState;
use crate::results_screen::ResultsState;

pub struct App {
    pub screen: Screen,
    pub language: Language,
    pub locale: QuizLocale,
    pub should_quit: bool,
    pub lang_picker: Option<LangPickerState>,
    pub templates: TemplateLuminaires,
}

pub enum Screen {
    Config(ConfigState),
    Quiz(QuizState),
    Results(ResultsState),
}

pub struct LangPickerState {
    pub cursor: usize,
    pub languages: Vec<Language>,
}

impl App {
    pub fn new(language: Language) -> Self {
        let locale = QuizLocale::for_code(language.code());
        let config = ConfigState::new(&locale);
        let templates = TemplateLuminaires::load();
        Self {
            screen: Screen::Config(config),
            language,
            locale,
            should_quit: false,
            lang_picker: None,
            templates,
        }
    }

    pub fn set_language(&mut self, lang: Language) {
        self.language = lang;
        self.locale = QuizLocale::for_code(lang.code());
    }

    fn toggle_lang_picker(&mut self) {
        if self.lang_picker.is_some() {
            self.lang_picker = None;
        } else {
            let languages: Vec<Language> = Language::all().to_vec();
            let cursor = languages
                .iter()
                .position(|l| *l == self.language)
                .unwrap_or(0);
            self.lang_picker = Some(LangPickerState { cursor, languages });
        }
    }

    pub fn handle_lang_picker_key(&mut self, key: KeyCode) {
        let picker = match &mut self.lang_picker {
            Some(p) => p,
            None => return,
        };
        match key {
            KeyCode::Up => {
                if picker.cursor > 0 {
                    picker.cursor -= 1;
                }
            }
            KeyCode::Down => {
                if picker.cursor + 1 < picker.languages.len() {
                    picker.cursor += 1;
                }
            }
            KeyCode::Enter => {
                let lang = picker.languages[picker.cursor];
                self.lang_picker = None;
                self.set_language(lang);
            }
            KeyCode::Esc | KeyCode::F(2) => {
                self.lang_picker = None;
            }
            _ => {}
        }
    }

    pub fn handle_config_key(&mut self, key: KeyCode) {
        let state = match &mut self.screen {
            Screen::Config(s) => s,
            _ => return,
        };
        match key {
            KeyCode::F(2) => self.toggle_lang_picker(),
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Tab => state.next_focus(),
            KeyCode::BackTab => state.prev_focus(),
            KeyCode::Up => state.cursor_up(),
            KeyCode::Down => state.cursor_down(),
            KeyCode::Left => state.cursor_left(),
            KeyCode::Right => state.cursor_right(),
            KeyCode::Char(' ') => {
                if matches!(state.focus, ConfigFocus::Categories) {
                    state.toggle_category();
                }
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                if matches!(state.focus, ConfigFocus::Categories) {
                    state.select_all();
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                if matches!(state.focus, ConfigFocus::Categories) {
                    state.select_none();
                }
            }
            KeyCode::Enter => {
                self.start_quiz();
            }
            _ => {}
        }
    }

    pub fn handle_quiz_key(&mut self, key: KeyCode) {
        let state = match &mut self.screen {
            Screen::Quiz(s) => s,
            _ => return,
        };

        if state.feedback.is_some() {
            // After answering: Enter/N advances, Q quits
            match key {
                KeyCode::F(2) => self.toggle_lang_picker(),
                KeyCode::Enter | KeyCode::Char('n') | KeyCode::Char('N') => {
                    state.feedback = None;
                    if state.session.is_finished() {
                        let score = state.session.score();
                        self.screen = Screen::Results(ResultsState::new(score));
                    }
                }
                KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                _ => {}
            }
            return;
        }

        match key {
            KeyCode::F(2) => self.toggle_lang_picker(),
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Up => {
                if state.option_cursor > 0 {
                    state.option_cursor -= 1;
                }
            }
            KeyCode::Down => {
                if state.option_cursor < 3 {
                    state.option_cursor += 1;
                }
            }
            KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Char('1') => {
                state.answer_current(0, &self.locale);
            }
            KeyCode::Char('b') | KeyCode::Char('B') | KeyCode::Char('2') => {
                state.answer_current(1, &self.locale);
            }
            KeyCode::Char('c') | KeyCode::Char('C') | KeyCode::Char('3') => {
                state.answer_current(2, &self.locale);
            }
            KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Char('4') => {
                state.answer_current(3, &self.locale);
            }
            KeyCode::Enter => {
                state.answer_current(state.option_cursor as u8, &self.locale);
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                state.skip_current(&self.locale);
            }
            _ => {}
        }
    }

    pub fn handle_results_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::F(2) => self.toggle_lang_picker(),
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('r') | KeyCode::Char('R') | KeyCode::Enter => {
                let config = ConfigState::new(&self.locale);
                self.screen = Screen::Config(config);
            }
            KeyCode::Up => {
                if let Screen::Results(s) = &mut self.screen {
                    s.scroll_up();
                }
            }
            KeyCode::Down => {
                if let Screen::Results(s) = &mut self.screen {
                    s.scroll_down();
                }
            }
            _ => {}
        }
    }

    fn start_quiz(&mut self) {
        let state = match &self.screen {
            Screen::Config(s) => s,
            _ => return,
        };

        let selected_cats: Vec<Category> = state
            .categories
            .iter()
            .filter(|(_, sel, _)| *sel)
            .map(|(cat, _, _)| *cat)
            .collect();

        if selected_cats.is_empty() {
            return;
        }

        let difficulty = match state.difficulty_cursor {
            1 => Some(Difficulty::Beginner),
            2 => Some(Difficulty::Intermediate),
            3 => Some(Difficulty::Expert),
            _ => None,
        };

        let num_questions = state.selected_count_value();

        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(42);

        let config = QuizConfig {
            categories: selected_cats,
            difficulty,
            num_questions,
            shuffle: true,
            seed: Some(seed),
        };

        let session = QuizSession::new(config);
        if session.questions.is_empty() {
            return;
        }

        self.screen = Screen::Quiz(QuizState::new(session));
    }

    pub fn draw(&self, frame: &mut Frame) {
        let area = frame.area();

        // Main layout: title bar + content + status bar
        let layout = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(area);

        self.draw_title_bar(frame, layout[0]);

        match &self.screen {
            Screen::Config(state) => {
                crate::config_screen::draw(frame, layout[1], state, &self.locale)
            }
            Screen::Quiz(state) => {
                crate::quiz_screen::draw(frame, layout[1], state, &self.locale, &self.templates)
            }
            Screen::Results(state) => {
                crate::results_screen::draw(frame, layout[1], state, &self.locale)
            }
        }

        self.draw_status_bar(frame, layout[2]);

        // Language picker overlay
        if let Some(picker) = &self.lang_picker {
            self.draw_lang_picker(frame, area, picker);
        }
    }

    fn draw_title_bar(&self, frame: &mut Frame, area: Rect) {
        let lang_label = format!("[F2: {}]", self.language.native_name());
        let title = format!(" {} ", self.locale.ui.title);
        let right_pad = lang_label.len() as u16 + 1;

        let bar =
            Layout::horizontal([Constraint::Min(0), Constraint::Length(right_pad)]).split(area);

        frame.render_widget(
            Paragraph::new(title).style(Style::new().bold().fg(Color::White).bg(Color::Cyan)),
            bar[0],
        );
        frame.render_widget(
            Paragraph::new(lang_label)
                .alignment(Alignment::Right)
                .style(Style::new().fg(Color::Yellow).bg(Color::Cyan)),
            bar[1],
        );
    }

    fn draw_status_bar(&self, frame: &mut Frame, area: Rect) {
        let hint = match &self.screen {
            Screen::Config(_) => {
                "Tab: sections  Space: toggle  \u{2190}\u{2192}: select  Enter: start  Q: quit"
            }
            Screen::Quiz(s) => {
                if s.feedback.is_some() {
                    "Enter/N: next  Q: quit"
                } else {
                    "A-D/1-4: answer  \u{2191}\u{2193}+Enter: select  S: skip  Q: quit"
                }
            }
            Screen::Results(_) => "R/Enter: try again  \u{2191}\u{2193}: scroll  Q: quit",
        };

        let block = Block::default()
            .borders(Borders::TOP)
            .border_style(Style::new().fg(Color::DarkGray));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(
            Paragraph::new(hint).style(Style::new().fg(Color::DarkGray)),
            inner,
        );
    }

    fn draw_lang_picker(&self, frame: &mut Frame, area: Rect, picker: &LangPickerState) {
        let width = 32u16;
        let height = (picker.languages.len() as u16) + 4;
        let x = area.width.saturating_sub(width) / 2;
        let y = area.height.saturating_sub(height) / 2;
        let popup = Rect::new(x, y, width.min(area.width), height.min(area.height));

        frame.render_widget(Clear, popup);

        let block = Block::default()
            .title(" Select Language ")
            .title_alignment(Alignment::Left)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(Color::Cyan));

        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        let items: Vec<Line> = picker
            .languages
            .iter()
            .enumerate()
            .map(|(i, lang)| {
                let prefix = if i == picker.cursor { "> " } else { "  " };
                let style = if i == picker.cursor {
                    Style::new().fg(Color::Cyan).bold()
                } else if *lang == self.language {
                    Style::new().fg(Color::Green)
                } else {
                    Style::default()
                };
                Line::from(Span::styled(
                    format!("{}{}", prefix, lang.native_name()),
                    style,
                ))
            })
            .collect();

        let hint_line = Line::from(Span::styled(
            "\u{2191}\u{2193}: select  Enter: apply  Esc: cancel",
            Style::new().fg(Color::DarkGray),
        ));

        let mut lines = items;
        lines.push(Line::default());
        lines.push(hint_line);

        frame.render_widget(Paragraph::new(lines), inner);
    }
}
