//! Validation panel widget

use egui::{Color32, RichText, Ui};
use eulumdat::{validate, Eulumdat};

/// Validation panel showing warnings
pub struct ValidationPanel;

impl ValidationPanel {
    /// Show the validation panel
    pub fn show(ui: &mut Ui, ldt: &Eulumdat) {
        let warnings = validate(ldt);

        ui.heading("Validation Results");
        ui.separator();

        if warnings.is_empty() {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("✓")
                        .color(Color32::from_rgb(80, 200, 80))
                        .size(20.0),
                );
                ui.label("No validation issues found.");
            });
        } else {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("⚠ {} issue(s) found", warnings.len()))
                        .color(Color32::from_rgb(220, 180, 50))
                        .size(14.0),
                );
            });

            ui.add_space(10.0);

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for warning in &warnings {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("⚠")
                                    .color(Color32::from_rgb(220, 180, 50))
                                    .monospace(),
                            );

                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(warning.code)
                                        .monospace()
                                        .color(Color32::GRAY),
                                );
                                ui.label(&warning.message);
                            });
                        });

                        ui.separator();
                    }
                });
        }

        ui.add_space(20.0);

        // Summary
        ui.collapsing("Validation Summary", |ui| {
            egui::Grid::new("validation_summary")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Total warnings:");
                    ui.label(format!("{}", warnings.len()));
                    ui.end_row();

                    ui.label("Data points:");
                    ui.label(format!("{}", ldt.c_angles.len() * ldt.g_angles.len()));
                    ui.end_row();

                    ui.label("C-planes:");
                    ui.label(format!("{}", ldt.c_angles.len()));
                    ui.end_row();

                    ui.label("Gamma angles:");
                    ui.label(format!("{}", ldt.g_angles.len()));
                    ui.end_row();
                });
        });
    }
}
