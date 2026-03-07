mod app;
mod config_screen;
mod diagram;
mod i18n;
mod quiz_screen;
mod results_screen;

use std::io;

use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use eulumdat_i18n::Language;
use ratatui::prelude::*;

use app::{App, Screen};

#[derive(Parser)]
#[command(
    name = "eulumdat-quiz",
    version,
    about = "Photometric knowledge quiz for lighting professionals",
    long_about = "Photometric knowledge quiz for lighting professionals.\n\n\
        Interactive terminal quiz covering EULUMDAT/IES formats, photometric\n\
        calculations, color science, BUG ratings, horticultural lighting, and more.\n\
        175 questions across 15 categories with full i18n support.\n\n\
        In-app controls:\n\
        \x20 Tab/Shift-Tab   Cycle focus sections\n\
        \x20 Space           Toggle category selection\n\
        \x20 A/N             Select all / none categories\n\
        \x20 Left/Right      Change difficulty or question count\n\
        \x20 Enter           Start quiz / confirm answer / try again\n\
        \x20 A-D or 1-4      Answer question\n\
        \x20 S               Skip question\n\
        \x20 F2              Change language\n\
        \x20 Q / Esc         Quit"
)]
struct Cli {
    /// Language for UI and questions.
    ///
    /// Supported: en, de, zh, fr, es, it, ru, pt-BR.
    /// If omitted, auto-detected from LANG / LC_ALL / LC_MESSAGES
    /// environment variables, falling back to English.
    #[arg(short, long, value_name = "CODE")]
    lang: Option<String>,

    /// List available languages and exit.
    #[arg(long)]
    list_languages: bool,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    if cli.list_languages {
        println!("Available languages:");
        for lang in Language::all() {
            println!("  {:<7} {}", lang.code(), lang.native_name());
        }
        return Ok(());
    }

    let language = match &cli.lang {
        Some(code) => Language::from_code(code),
        None => i18n::detect_terminal_language(),
    };

    // Panic hook: restore terminal before printing panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = io::stdout().execute(LeaveAlternateScreen);
        let _ = disable_raw_mode();
        original_hook(info);
    }));

    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(language);
    let result = run_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|frame| app.draw(frame))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            // Ctrl-C always quits
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return Ok(());
            }

            // Language picker overlay intercepts all keys when active
            if app.lang_picker.is_some() {
                app.handle_lang_picker_key(key.code);
                if app.should_quit {
                    return Ok(());
                }
                continue;
            }

            match &app.screen {
                Screen::Config(_) => app.handle_config_key(key.code),
                Screen::Quiz(_) => app.handle_quiz_key(key.code),
                Screen::Results(_) => app.handle_results_key(key.code),
            }

            if app.should_quit {
                return Ok(());
            }
        }
    }
}
