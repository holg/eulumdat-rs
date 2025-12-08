//! Info panel rendering

use eframe::egui::{self, Color32, RichText, ScrollArea, Ui};
use eulumdat::{validate, Eulumdat};

/// Render the info panel (right side)
pub fn render_info_panel(ui: &mut Ui, ldt: &Eulumdat) {
    ScrollArea::vertical().show(ui, |ui| {
        // Summary section
        ui.heading(&ldt.luminaire_name);
        ui.label(RichText::new(&ldt.identification).color(Color32::GRAY));
        ui.separator();

        // Key metrics
        egui::Grid::new("metrics_grid")
            .num_columns(2)
            .spacing([20.0, 4.0])
            .show(ui, |ui| {
                ui.label("Max Intensity:");
                ui.label(format!("{:.0} cd/klm", ldt.max_intensity()));
                ui.end_row();

                ui.label("Total Flux:");
                ui.label(format!("{:.0} lm", ldt.total_luminous_flux()));
                ui.end_row();

                ui.label("Symmetry:");
                ui.label(format!("{:?}", ldt.symmetry));
                ui.end_row();

                ui.label("C-Planes:");
                ui.label(format!("{}", ldt.c_angles.len()));
                ui.end_row();

                ui.label("g-Angles:");
                ui.label(format!("{}", ldt.g_angles.len()));
                ui.end_row();
            });

        // Validation warnings
        let warnings = validate(ldt);
        if !warnings.is_empty() {
            ui.add_space(10.0);
            ui.separator();
            ui.label(
                RichText::new(format!("! {} warnings", warnings.len())).color(Color32::YELLOW),
            );
        }
    });
}
