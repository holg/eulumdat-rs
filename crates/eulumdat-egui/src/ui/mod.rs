//! UI components for the Eulumdat application

pub mod diagram_panel;
mod info_panel;
pub mod tabs;

pub use diagram_panel::DiagramType;
pub use info_panel::render_info_panel;
pub use tabs::{render_main_tab_bar, render_sub_tab_bar, MainTab, SubTab};
