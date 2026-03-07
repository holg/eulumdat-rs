use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use crate::ui::Focus;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    Quit,
    CycleView,
    ZoomIn,
    ZoomOut,
    PanUp,
    PanDown,
    PanLeft,
    PanRight,
    ResetView,
    CycleFocus,
    ScrollUp,
    ScrollDown,
    NextCPlane,
    PrevCPlane,
}

pub fn map_key_to_action(key: KeyEvent, focus: Focus) -> Option<Action> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Some(Action::Quit);
    }

    match focus {
        Focus::Sidebar => map_sidebar_key(key),
        Focus::Diagram => map_diagram_key(key),
    }
}

fn map_diagram_key(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),
        KeyCode::Char('v') => Some(Action::CycleView),
        KeyCode::Char('+') | KeyCode::Char('=') => Some(Action::ZoomIn),
        KeyCode::Char('-') => Some(Action::ZoomOut),
        KeyCode::Up => Some(Action::PanUp),
        KeyCode::Down => Some(Action::PanDown),
        KeyCode::Left => Some(Action::PanLeft),
        KeyCode::Right => Some(Action::PanRight),
        KeyCode::Char('r') => Some(Action::ResetView),
        KeyCode::Tab => Some(Action::CycleFocus),
        KeyCode::Char('j') => Some(Action::NextCPlane),
        KeyCode::Char('k') => Some(Action::PrevCPlane),
        _ => None,
    }
}

fn map_sidebar_key(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),
        KeyCode::Char('v') => Some(Action::CycleView),
        KeyCode::Tab => Some(Action::CycleFocus),
        KeyCode::Up | KeyCode::Char('k') => Some(Action::ScrollUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::ScrollDown),
        KeyCode::Char('r') => Some(Action::ResetView),
        KeyCode::Char('+') | KeyCode::Char('=') => Some(Action::ZoomIn),
        KeyCode::Char('-') => Some(Action::ZoomOut),
        _ => None,
    }
}

pub struct MouseAction {
    pub kind: MouseActionKind,
    pub column: u16,
    pub row: u16,
}

pub enum MouseActionKind {
    ScrollUp,
    ScrollDown,
    DragStart,
    Drag { dx: i16, dy: i16 },
    DragEnd,
}

pub fn map_mouse_event(
    event: MouseEvent,
    last_drag: &mut Option<(u16, u16)>,
) -> Option<MouseAction> {
    let col = event.column;
    let row = event.row;

    match event.kind {
        MouseEventKind::ScrollUp => Some(MouseAction {
            kind: MouseActionKind::ScrollUp,
            column: col,
            row,
        }),
        MouseEventKind::ScrollDown => Some(MouseAction {
            kind: MouseActionKind::ScrollDown,
            column: col,
            row,
        }),
        MouseEventKind::Down(MouseButton::Left) => {
            *last_drag = Some((col, row));
            Some(MouseAction {
                kind: MouseActionKind::DragStart,
                column: col,
                row,
            })
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if let Some((lx, ly)) = *last_drag {
                let dx = col as i16 - lx as i16;
                let dy = row as i16 - ly as i16;
                *last_drag = Some((col, row));
                Some(MouseAction {
                    kind: MouseActionKind::Drag { dx, dy },
                    column: col,
                    row,
                })
            } else {
                *last_drag = Some((col, row));
                None
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            *last_drag = None;
            Some(MouseAction {
                kind: MouseActionKind::DragEnd,
                column: col,
                row,
            })
        }
        _ => None,
    }
}
