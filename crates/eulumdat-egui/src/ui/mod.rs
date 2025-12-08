//! UI components for the Eulumdat application

pub mod diagram_panel;
mod info_panel;
pub mod tabs;

pub use diagram_panel::{render_diagram_panel, DiagramType};
pub use info_panel::render_info_panel;
pub use tabs::{render_tab_bar, AppTab};
