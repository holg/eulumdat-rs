mod app;
mod input;
mod ui;

use std::io;
use std::panic;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::App;

#[derive(Parser)]
#[command(name = "eulumdat-tui", about = "Terminal photometric data viewer")]
struct Cli {
    /// Path to an LDT or IES file
    file: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Create App first (validates args, loads file) before entering raw mode
    let mut app = App::new(cli.file.as_deref())?;

    // Panic hook: restore terminal before printing panic info
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        default_hook(info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = app.run(&mut terminal);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}
