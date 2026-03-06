pub mod butterfly;
pub mod cartesian;
pub mod cone;
pub mod heatmap;
pub mod info;
pub mod polar;
pub mod status;

use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Sidebar,
    Diagram,
}

impl Focus {
    pub fn cycle(self) -> Self {
        match self {
            Focus::Sidebar => Focus::Diagram,
            Focus::Diagram => Focus::Sidebar,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Polar,
    Cartesian,
    Heatmap,
    Cone,
    Butterfly,
}

impl ViewMode {
    pub fn cycle(self) -> Self {
        match self {
            ViewMode::Polar => ViewMode::Cartesian,
            ViewMode::Cartesian => ViewMode::Heatmap,
            ViewMode::Heatmap => ViewMode::Cone,
            ViewMode::Cone => ViewMode::Butterfly,
            ViewMode::Butterfly => ViewMode::Polar,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ViewMode::Polar => "Polar",
            ViewMode::Cartesian => "Cartesian",
            ViewMode::Heatmap => "Heatmap",
            ViewMode::Cone => "Cone",
            ViewMode::Butterfly => "Butterfly",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LayoutAreas {
    pub sidebar: Rect,
    pub diagram: Rect,
    pub status: Rect,
}

pub fn calculate_layout(area: Rect) -> LayoutAreas {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(area);

    let sidebar_width = 35u16.min(area.width / 3);
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(sidebar_width), Constraint::Min(10)])
        .split(vertical[0]);

    LayoutAreas {
        sidebar: horizontal[0],
        diagram: horizontal[1],
        status: vertical[1],
    }
}
