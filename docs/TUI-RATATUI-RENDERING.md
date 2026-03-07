# TUI Rendering with ratatui — Reference from bimifc-viewer-tui

Reference implementation: `../bimifc/crates/bimifc-viewer-tui/`

This document captures the patterns and architecture used in the bimifc TUI viewer
so we can replicate and adapt them for an `eulumdat-tui` crate.

---

## Architecture Overview

```
main.rs          — Terminal setup (crossterm), clap args, app lifecycle
app.rs           — App state, event loop (poll/draw), input dispatch
input.rs         — Key/mouse → Action enum mapping
lib.rs           — pub exports (App, camera, renderer, scene, ui modules)

ui/
  viewport.rs    — Viewport widget (Canvas + Braille markers, multiple view modes)
  hierarchy.rs   — Tree panel (StatefulWidget with ListState)
  properties.rs  — Properties panel (key/value display)
  status.rs      — Status bar

renderer/
  framebuffer.rs — Legacy block-char buffer (char + color per cell)
  floorplan.rs   — Floor plan slice renderer
  ...
```

## Dependencies (Cargo.toml)

```toml
crossterm = "0.28"          # Terminal raw mode, mouse capture, events
ratatui = "0.29"            # TUI framework (widgets, layout, buffer)
glam = "0.29"               # Math (Vec2/Vec3 for geometry)
clap = { version = "4", features = ["derive"] }
anyhow = "1.0"
```

## Terminal Lifecycle (main.rs)

```rust
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

fn main() -> Result<()> {
    // Setup
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run app
    let result = run_app(&mut terminal);

    // Restore (always runs, even on panic — consider using a guard struct)
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    result
}
```

## Event Loop (app.rs)

```rust
use crossterm::event::{self, Event, KeyCode, KeyEvent, MouseEvent, MouseEventKind, MouseButton};
use std::time::Duration;

pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    loop {
        // Draw
        terminal.draw(|frame| self.draw(frame))?;

        // Poll with timeout (16ms ≈ 60fps, or longer if idle)
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if self.handle_key(key) { break; } // quit
                }
                Event::Mouse(mouse) => self.handle_mouse(mouse),
                Event::Resize(_, _) => {} // ratatui handles resize
                _ => {}
            }
        }
    }
    Ok(())
}
```

## Layout (app.rs draw)

```rust
use ratatui::layout::{Constraint, Direction, Layout, Rect};

fn draw(&mut self, frame: &mut Frame) {
    let area = frame.area();

    // Horizontal split: sidebar | viewport
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(30), Constraint::Min(40)])
        .split(area);

    // Sidebar: vertical split
    let sidebar = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_chunks[0]);

    // Render widgets
    frame.render_widget(some_widget, main_chunks[1]);
    frame.render_stateful_widget(list_widget, sidebar[0], &mut self.list_state);
}
```

## Braille Canvas — Key Pattern for Diagrams

ratatui's `Canvas` widget with `Marker::Braille` gives 2×4 subpixel resolution per terminal cell.
Each cell becomes a braille character (U+2800–U+28FF), giving 8× more detail than block chars.

```rust
use ratatui::widgets::canvas::{Canvas, Line as CanvasLine};
use ratatui::symbols::Marker;

let canvas = Canvas::default()
    .block(Block::default().borders(Borders::ALL).title(" Polar Diagram "))
    .marker(Marker::Braille)
    .x_bounds([-bound, bound])       // world coordinate range
    .y_bounds([-bound, bound])
    .paint(|ctx| {
        // Draw a line
        ctx.draw(&CanvasLine {
            x1: 0.0, y1: 0.0,
            x2: 100.0, y2: 50.0,
            color: Color::Rgb(80, 140, 255),
        });

        // Draw text label
        ctx.print(
            x, y,
            ratatui::text::Line::styled("label", Style::default().fg(Color::DarkGray)),
        );

        // Draw a circle (as line segments)
        let steps = 72;
        for i in 0..steps {
            let a0 = i as f64 * std::f64::consts::TAU / steps as f64;
            let a1 = (i + 1) as f64 * std::f64::consts::TAU / steps as f64;
            ctx.draw(&CanvasLine {
                x1: radius * a0.cos(), y1: radius * a0.sin(),
                x2: radius * a1.cos(), y2: radius * a1.sin(),
                color: Color::Rgb(40, 40, 55),
            });
        }
    });

canvas.render(area, buf);
```

### Polar Diagram (already in bimifc, port directly)

The bimifc TUI renders `eulumdat::diagram::PolarDiagram` via braille canvas.
Key steps:
1. Parse LDT/IES → `Eulumdat`
2. `PolarDiagram::from_eulumdat(&ldt)` → get curves + scale
3. Draw grid circles at `polar.scale.grid_values`
4. Draw C0-C180 curve (blue) and C90-C270 curve (red)
5. Draw axes and labels

This can be extracted almost verbatim into `eulumdat-tui`.

## StatefulWidget vs Widget

**Widget** — immutable render, no state feedback:
```rust
impl Widget for MyWidget {
    fn render(self, area: Rect, buf: &mut Buffer) { ... }
}
frame.render_widget(widget, area);
```

**StatefulWidget** — mutable state, ratatui updates scroll offsets:
```rust
impl StatefulWidget for MyPanel {
    type State = MyState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut MyState) { ... }
}
frame.render_stateful_widget(widget, area, &mut state);
```

**IMPORTANT**: Use StatefulWidget for any scrollable list (List + ListState).
If you clone ListState for rendering, the real state diverges from ratatui's
auto-scroll offset → click coordinates break. We hit this bug in bimifc.

## Mouse Support

Enable in terminal setup: `EnableMouseCapture` / `DisableMouseCapture`.

```rust
Event::Mouse(MouseEvent { kind, column, row, .. }) => {
    match kind {
        MouseEventKind::Down(MouseButton::Left) => { /* click */ }
        MouseEventKind::ScrollUp => { /* zoom in */ }
        MouseEventKind::ScrollDown => { /* zoom out */ }
        MouseEventKind::Drag(MouseButton::Left) => { /* pan/rotate */ }
        MouseEventKind::Moved => { /* hover highlight */ }
        _ => {}
    }
}
```

Use stored layout `Rect` from last draw to determine which panel was clicked:
```rust
if self.last_viewport_area.contains(Position::new(col, row)) {
    // click was in viewport
}
```

## Color Scheme

```rust
// Borders
let focused = Style::default().fg(Color::Cyan);
let unfocused = Style::default().fg(Color::DarkGray);

// Selection highlight
Style::default().bg(Color::Rgb(40, 40, 60)).add_modifier(Modifier::BOLD)

// Grid lines (subtle)
Color::Rgb(40, 40, 55)

// Axes
Color::Rgb(60, 60, 80)

// Data curves
Color::Rgb(80, 140, 255)   // primary (blue)
Color::Rgb(255, 100, 100)  // secondary (red)
Color::Rgb(100, 255, 100)  // tertiary (green)
```

## Suggested eulumdat-tui Structure

```
crates/eulumdat-tui/
├── Cargo.toml
└── src/
    ├── main.rs          # Terminal setup, clap args
    ├── app.rs           # App state + event loop
    ├── input.rs         # Key bindings → Action enum
    └── ui/
        ├── mod.rs
        ├── polar.rs     # Polar diagram (Canvas + Braille)
        ├── cartesian.rs # Cartesian diagram (Canvas + Braille)
        ├── info.rs      # File info panel (luminaire name, flux, etc.)
        └── status.rs    # Status bar (file name, key hints)
```

### Possible features:
- **File browser**: Open LDT/IES from directory listing
- **Multi-file compare**: Side-by-side polar diagrams
- **View modes**: Polar, Cartesian, Cone (use ViewMode enum pattern)
- **Interactive**: Mouse rotate/zoom diagram, click to select C-plane
- **Export**: Screenshot to terminal (copy-paste braille art)

## Key Gotchas

1. **Canvas coordinates are world-space**, not pixel-space. Set `x_bounds`/`y_bounds` to match your data range. Canvas handles the mapping.

2. **Canvas Y axis points UP** (math convention). If your data has Y-down, negate: `y1: -point.y`.

3. **Braille resolution**: each terminal cell = 2×4 dots. A 100-column terminal gives 200 horizontal dots. Plan canvas bounds accordingly.

4. **Always restore terminal** on exit/panic. Use `std::panic::set_hook` + a cleanup guard if needed.

5. **ListState offset bug**: Never clone ListState for rendering. Use StatefulWidget and pass `&mut state`.

6. **crossterm version**: Must match ratatui's expected crossterm version. Check ratatui's Cargo.toml.
