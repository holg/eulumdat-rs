//! egui widgets for Eulumdat visualization

mod cartesian;
mod heatmap;
mod info;
mod polar;
mod validation;

pub use cartesian::CartesianWidget;
pub use heatmap::HeatmapWidget;
pub use info::InfoPanel;
pub use polar::PolarWidget;
pub use validation::ValidationPanel;

/// Available diagram tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub enum DiagramTab {
    Info,
    #[default]
    Polar,
    Cartesian,
    Heatmap,
    #[cfg(feature = "3d")]
    Viewer3D,
    Validation,
}

/// Editor panel combining info editing and validation
pub struct EditorPanel;

impl EditorPanel {
    pub fn show(ui: &mut egui::Ui, ldt: &mut eulumdat::Eulumdat) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Luminaire Information");
            ui.separator();

            egui::Grid::new("info_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut ldt.luminaire_name);
                    ui.end_row();

                    ui.label("Manufacturer:");
                    ui.text_edit_singleline(&mut ldt.identification);
                    ui.end_row();

                    ui.label("Description:");
                    ui.text_edit_singleline(&mut ldt.luminaire_number);
                    ui.end_row();
                });

            ui.separator();
            ui.heading("Dimensions (mm)");

            egui::Grid::new("dimensions_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Length:");
                    ui.add(egui::DragValue::new(&mut ldt.length).speed(1.0));
                    ui.end_row();

                    ui.label("Width:");
                    ui.add(egui::DragValue::new(&mut ldt.width).speed(1.0));
                    ui.end_row();

                    ui.label("Height:");
                    ui.add(egui::DragValue::new(&mut ldt.height).speed(1.0));
                    ui.end_row();
                });
        });
    }
}
