use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event};
use ratatui::{backend::CrosstermBackend, Terminal};

use eulumdat::{
    diagram::{ButterflyDiagram, CartesianDiagram, ConeDiagram, HeatmapDiagram, PolarDiagram},
    validate, BeamFieldAnalysis, Eulumdat, IesParser, PhotometricCalculations, PhotometricSummary,
    ValidationWarning,
};

use crate::input::{self, Action, MouseActionKind};
use crate::ui::{self, Focus, LayoutAreas, ViewMode};

pub struct App {
    ldt: Eulumdat,
    file_name: String,
    summary: PhotometricSummary,
    warnings: Vec<ValidationWarning>,
    beam_field: BeamFieldAnalysis,

    // Precomputed diagram data
    polar: PolarDiagram,
    cartesian: CartesianDiagram,
    heatmap: HeatmapDiagram,
    cone: ConeDiagram,
    butterfly: ButterflyDiagram,

    // C-plane selection for polar diagram
    c_plane_angles: Vec<f64>,
    c_plane_index: usize, // 0 = default (C0-C180 / C90-C270)

    // View state
    view_mode: ViewMode,
    focus: Focus,
    zoom: f64,
    pan: (f64, f64),
    sidebar_scroll: u16,
    last_layout: Option<LayoutAreas>,
    last_drag: Option<(u16, u16)>,
    should_quit: bool,
}

impl App {
    pub fn new(file_path: Option<&str>) -> Result<Self> {
        let (ldt, file_name) = match file_path {
            Some(path) => {
                let content = std::fs::read_to_string(path)?;
                let lower = path.to_lowercase();
                let ldt = if lower.ends_with(".ies") {
                    IesParser::parse(&content)?
                } else {
                    Eulumdat::parse(&content)?
                };
                let name = std::path::Path::new(path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string());
                (ldt, name)
            }
            None => {
                anyhow::bail!("No file specified. Usage: eulumdat-tui <FILE.ldt|FILE.ies>");
            }
        };

        let summary = PhotometricSummary::from_eulumdat(&ldt);
        let warnings = validate(&ldt);
        let beam_field = PhotometricCalculations::beam_field_analysis(&ldt);

        let polar = PolarDiagram::from_eulumdat(&ldt);
        let cartesian = CartesianDiagram::from_eulumdat(&ldt, 800.0, 600.0, 8);
        let heatmap = HeatmapDiagram::from_eulumdat(&ldt, 800.0, 600.0);
        let cone = ConeDiagram::from_eulumdat(&ldt, 3.0);
        let butterfly = ButterflyDiagram::from_eulumdat(&ldt, 500.0, 400.0, 60.0);

        // Build list of available C-plane angles for cycling
        // Index 0 = "default" (standard C0-C180 + C90-C270 view)
        let c_plane_angles = ldt.c_angles.clone();

        Ok(App {
            ldt,
            file_name,
            summary,
            warnings,
            beam_field,
            polar,
            cartesian,
            heatmap,
            cone,
            butterfly,
            c_plane_angles,
            c_plane_index: 0,
            view_mode: ViewMode::Polar,
            focus: Focus::Diagram,
            zoom: 1.0,
            pan: (0.0, 0.0),
            sidebar_scroll: 0,
            last_layout: None,
            last_drag: None,
            should_quit: false,
        })
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| self.draw(frame))?;

            if event::poll(Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(key) => {
                        if let Some(action) = input::map_key_to_action(key, self.focus) {
                            self.handle_action(action);
                        }
                    }
                    Event::Mouse(mouse) => {
                        if let Some(mouse_action) =
                            input::map_mouse_event(mouse, &mut self.last_drag)
                        {
                            self.handle_mouse(mouse_action);
                        }
                    }
                    Event::Resize(_, _) => {} // ratatui handles this
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut ratatui::Frame) {
        let layout = ui::calculate_layout(frame.area());
        self.last_layout = Some(layout);

        // Info sidebar
        ui::info::render_info(
            layout.sidebar,
            frame.buffer_mut(),
            &self.ldt,
            &self.summary,
            &self.warnings,
            self.sidebar_scroll,
            self.focus == Focus::Sidebar,
        );

        // Diagram
        let focused = self.focus == Focus::Diagram;
        match self.view_mode {
            ViewMode::Polar => {
                ui::polar::render_polar(
                    layout.diagram,
                    frame.buffer_mut(),
                    &self.polar,
                    &self.beam_field,
                    self.zoom,
                    self.pan,
                    focused,
                );
            }
            ViewMode::Cartesian => {
                ui::cartesian::render_cartesian(
                    layout.diagram,
                    frame.buffer_mut(),
                    &self.cartesian,
                    self.zoom,
                    self.pan,
                    focused,
                );
            }
            ViewMode::Heatmap => {
                ui::heatmap::render_heatmap(
                    layout.diagram,
                    frame.buffer_mut(),
                    &self.heatmap,
                    focused,
                );
            }
            ViewMode::Cone => {
                ui::cone::render_cone(
                    layout.diagram,
                    frame.buffer_mut(),
                    &self.cone,
                    self.zoom,
                    self.pan,
                    focused,
                );
            }
            ViewMode::Butterfly => {
                ui::butterfly::render_butterfly(
                    layout.diagram,
                    frame.buffer_mut(),
                    &self.butterfly,
                    self.zoom,
                    self.pan,
                    focused,
                );
            }
        }

        // Status bar
        let c_plane_label = self.c_plane_status_label();
        ui::status::render_status(
            layout.status,
            frame.buffer_mut(),
            self.view_mode,
            &self.file_name,
            c_plane_label.as_deref(),
        );
    }

    fn c_plane_status_label(&self) -> Option<String> {
        if self.view_mode != ViewMode::Polar {
            return None;
        }
        if self.c_plane_index == 0 {
            Some("C0/C180+C90/C270".to_string())
        } else {
            let angle = self.c_plane_angles[self.c_plane_index - 1];
            let opposite = (angle + 180.0) % 360.0;
            Some(format!("C{:.0}\u{00b0}/C{:.0}\u{00b0}", angle, opposite))
        }
    }

    fn handle_action(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::CycleView => {
                self.view_mode = self.view_mode.cycle();
                self.zoom = 1.0;
                self.pan = (0.0, 0.0);
            }
            Action::ZoomIn => self.zoom *= 1.2,
            Action::ZoomOut => self.zoom = (self.zoom / 1.2).max(0.1),
            Action::PanUp => self.pan.1 += self.pan_step(),
            Action::PanDown => self.pan.1 -= self.pan_step(),
            Action::PanLeft => self.pan.0 -= self.pan_step(),
            Action::PanRight => self.pan.0 += self.pan_step(),
            Action::ResetView => {
                self.zoom = 1.0;
                self.pan = (0.0, 0.0);
                self.sidebar_scroll = 0;
                if self.view_mode == ViewMode::Polar {
                    self.c_plane_index = 0;
                    self.polar = PolarDiagram::from_eulumdat(&self.ldt);
                }
            }
            Action::CycleFocus => self.focus = self.focus.cycle(),
            Action::ScrollUp => self.sidebar_scroll = self.sidebar_scroll.saturating_sub(1),
            Action::ScrollDown => self.sidebar_scroll += 1,
            Action::NextCPlane => self.cycle_c_plane(1),
            Action::PrevCPlane => self.cycle_c_plane(-1),
        }
    }

    fn cycle_c_plane(&mut self, delta: i32) {
        if self.view_mode != ViewMode::Polar || self.c_plane_angles.is_empty() {
            return;
        }
        // Total positions: 1 (default) + number of C-plane angles
        let total = 1 + self.c_plane_angles.len();
        let current = self.c_plane_index as i32;
        let next = ((current + delta).rem_euclid(total as i32)) as usize;

        if next != self.c_plane_index {
            self.c_plane_index = next;
            if next == 0 {
                self.polar = PolarDiagram::from_eulumdat(&self.ldt);
            } else {
                let angle = self.c_plane_angles[next - 1];
                self.polar = PolarDiagram::from_eulumdat_for_plane(&self.ldt, angle);
            }
        }
    }

    fn handle_mouse(&mut self, action: input::MouseAction) {
        let layout = match self.last_layout {
            Some(l) => l,
            None => return,
        };

        let pos = ratatui::layout::Position::new(action.column, action.row);
        let in_sidebar = layout.sidebar.contains(pos);
        let in_diagram = layout.diagram.contains(pos);

        match action.kind {
            MouseActionKind::ScrollUp => {
                if in_sidebar {
                    self.sidebar_scroll = self.sidebar_scroll.saturating_sub(3);
                } else if in_diagram {
                    self.zoom *= 1.15;
                }
            }
            MouseActionKind::ScrollDown => {
                if in_sidebar {
                    self.sidebar_scroll += 3;
                } else if in_diagram {
                    self.zoom = (self.zoom / 1.15).max(0.1);
                }
            }
            MouseActionKind::DragStart => {
                if in_sidebar {
                    self.focus = Focus::Sidebar;
                } else if in_diagram {
                    self.focus = Focus::Diagram;
                }
            }
            MouseActionKind::Drag { dx, dy } => {
                if in_diagram || self.focus == Focus::Diagram {
                    let scale = self.pan_step() * 0.5;
                    self.pan.0 -= dx as f64 * scale;
                    self.pan.1 += dy as f64 * scale;
                }
            }
            MouseActionKind::DragEnd => {}
        }
    }

    fn pan_step(&self) -> f64 {
        let base = match self.view_mode {
            ViewMode::Polar => self.polar.scale.scale_max * 0.1,
            ViewMode::Cartesian => self.cartesian.scale.scale_max * 0.05,
            ViewMode::Cone => self.cone.mounting_height * 0.1,
            ViewMode::Butterfly => self.butterfly.scale.scale_max * 0.1,
            ViewMode::Heatmap => 10.0,
        };
        base / self.zoom
    }
}
