//! Information panel widget for displaying luminaire details

use egui::Ui;
use eulumdat::Eulumdat;

/// Information panel showing luminaire details
pub struct InfoPanel;

impl InfoPanel {
    /// Show the info panel (read-only view)
    pub fn show(ui: &mut Ui, ldt: &Eulumdat) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Luminaire Information");
            ui.separator();

            egui::Grid::new("info_grid")
                .num_columns(2)
                .spacing([40.0, 6.0])
                .striped(true)
                .show(ui, |ui| {
                    Self::row(ui, "Name", &ldt.luminaire_name);
                    Self::row(ui, "Manufacturer", &ldt.identification);
                    Self::row(ui, "Number", &ldt.luminaire_number);
                    Self::row(ui, "File Name", &ldt.file_name);
                    Self::row(ui, "Date/User", &ldt.date_user);
                    Self::row(ui, "Type", &format!("{:?}", ldt.type_indicator));
                    Self::row(ui, "Symmetry", &format!("{:?}", ldt.symmetry));
                });

            ui.add_space(20.0);
            ui.heading("Dimensions");
            ui.separator();

            egui::Grid::new("dimensions_grid")
                .num_columns(2)
                .spacing([40.0, 6.0])
                .striped(true)
                .show(ui, |ui| {
                    Self::row(ui, "Length", &format!("{:.1} mm", ldt.length));
                    Self::row(ui, "Width", &format!("{:.1} mm", ldt.width));
                    Self::row(ui, "Height", &format!("{:.1} mm", ldt.height));
                    Self::row(
                        ui,
                        "Luminous Area",
                        &format!(
                            "{:.1} × {:.1} mm",
                            ldt.luminous_area_length, ldt.luminous_area_width
                        ),
                    );
                });

            ui.add_space(20.0);
            ui.heading("Photometric Data");
            ui.separator();

            egui::Grid::new("photometric_grid")
                .num_columns(2)
                .spacing([40.0, 6.0])
                .striped(true)
                .show(ui, |ui| {
                    Self::row(ui, "C-planes", &format!("{}", ldt.c_angles.len()));
                    Self::row(
                        ui,
                        "C-plane Range",
                        &format!(
                            "{:.0}° - {:.0}°",
                            ldt.c_angles.first().unwrap_or(&0.0),
                            ldt.c_angles.last().unwrap_or(&0.0)
                        ),
                    );
                    Self::row(ui, "Gamma Angles", &format!("{}", ldt.g_angles.len()));
                    Self::row(
                        ui,
                        "Gamma Range",
                        &format!(
                            "{:.0}° - {:.0}°",
                            ldt.g_angles.first().unwrap_or(&0.0),
                            ldt.g_angles.last().unwrap_or(&0.0)
                        ),
                    );
                    Self::row(
                        ui,
                        "Max Intensity",
                        &format!("{:.1} cd/klm", ldt.max_intensity()),
                    );
                    Self::row(
                        ui,
                        "Total Flux",
                        &format!("{:.0} lm", ldt.total_luminous_flux()),
                    );
                });

            // Lamp sets
            if !ldt.lamp_sets.is_empty() {
                ui.add_space(20.0);
                ui.heading("Lamp Sets");
                ui.separator();

                for (i, lamp) in ldt.lamp_sets.iter().enumerate() {
                    ui.collapsing(format!("Lamp Set {}", i + 1), |ui| {
                        egui::Grid::new(format!("lamp_grid_{}", i))
                            .num_columns(2)
                            .spacing([40.0, 4.0])
                            .show(ui, |ui| {
                                Self::row(ui, "Number of Lamps", &format!("{}", lamp.num_lamps));
                                Self::row(ui, "Type", &lamp.lamp_type);
                                Self::row(
                                    ui,
                                    "Luminous Flux",
                                    &format!("{:.0} lm", lamp.total_luminous_flux),
                                );
                                Self::row(ui, "Color", &lamp.color_appearance);
                                Self::row(ui, "CRI/Group", &lamp.color_rendering_group);
                                Self::row(ui, "Wattage", &format!("{:.1} W", lamp.wattage_with_ballast));
                            });
                    });
                }
            }

            // Direct ratios
            if ldt.direct_ratios.iter().any(|&r| r > 0.0) {
                ui.add_space(20.0);
                ui.heading("Direct Ratios (DFF)");
                ui.separator();

                ui.horizontal_wrapped(|ui| {
                    for (i, &ratio) in ldt.direct_ratios.iter().enumerate() {
                        if ratio > 0.0 {
                            ui.label(format!("DFF{}: {:.1}%", i + 1, ratio * 100.0));
                        }
                    }
                });
            }
        });
    }

    fn row(ui: &mut Ui, label: &str, value: &str) {
        ui.label(label);
        ui.label(value);
        ui.end_row();
    }
}
