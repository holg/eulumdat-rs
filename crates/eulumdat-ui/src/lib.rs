//! Shared egui UI components for Eulumdat viewer/editor
//!
//! This crate provides reusable egui widgets for visualizing and editing
//! EULUMDAT (LDT) and IES photometric data. It's designed to be used by:
//! - Desktop applications (via eframe)
//! - Web applications (via eframe WASM or embedded in Leptos)
//!
//! # Features
//!
//! - `3d` - Enable 3D viewer using three-d
//! - `persistence` - Enable state persistence via serde
//!
//! # Example
//!
//! ```rust,ignore
//! use eulumdat::Eulumdat;
//! use eulumdat_ui::{EulumdatEditor, Theme};
//!
//! // In your egui app
//! fn update(&mut self, ctx: &egui::Context) {
//!     egui::CentralPanel::default().show(ctx, |ui| {
//!         self.editor.show(ui, &mut self.ldt);
//!     });
//! }
//! ```

mod theme;
mod widgets;

#[cfg(feature = "3d")]
mod viewer_3d;

pub use theme::Theme;
pub use widgets::{
    CartesianWidget, DiagramTab, EditorPanel, HeatmapWidget, InfoPanel, PolarWidget,
    ValidationPanel,
};

#[cfg(feature = "3d")]
pub use viewer_3d::Viewer3D;

/// Main editor component that combines all widgets
pub struct EulumdatEditor {
    /// Current active tab
    pub active_tab: DiagramTab,
    /// Theme settings
    pub theme: Theme,
    /// Show validation panel
    pub show_validation: bool,
}

impl Default for EulumdatEditor {
    fn default() -> Self {
        Self {
            active_tab: DiagramTab::Polar,
            theme: Theme::default(),
            show_validation: true,
        }
    }
}

impl EulumdatEditor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Show the full editor UI
    pub fn show(&mut self, ui: &mut egui::Ui, ldt: &mut Option<eulumdat::Eulumdat>) {
        // Top toolbar
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.active_tab, DiagramTab::Info, "Info");
            ui.selectable_value(&mut self.active_tab, DiagramTab::Polar, "Polar");
            ui.selectable_value(&mut self.active_tab, DiagramTab::Cartesian, "Cartesian");
            ui.selectable_value(&mut self.active_tab, DiagramTab::Heatmap, "Heatmap");
            #[cfg(feature = "3d")]
            ui.selectable_value(&mut self.active_tab, DiagramTab::Viewer3D, "3D");
            ui.selectable_value(&mut self.active_tab, DiagramTab::Validation, "Validation");
        });

        ui.separator();

        // Main content area
        if let Some(ldt) = ldt {
            match self.active_tab {
                DiagramTab::Info => {
                    InfoPanel::show(ui, ldt);
                }
                DiagramTab::Polar => {
                    PolarWidget::show(ui, ldt, &self.theme);
                }
                DiagramTab::Cartesian => {
                    CartesianWidget::show(ui, ldt, &self.theme);
                }
                DiagramTab::Heatmap => {
                    HeatmapWidget::show(ui, ldt, &self.theme);
                }
                #[cfg(feature = "3d")]
                DiagramTab::Viewer3D => {
                    Viewer3D::show(ui, ldt, &self.theme);
                }
                DiagramTab::Validation => {
                    ValidationPanel::show(ui, ldt);
                }
            }
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No file loaded. Drag & drop an LDT or IES file, or use File > Open.");
            });
        }
    }
}
