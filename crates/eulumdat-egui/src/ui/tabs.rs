//! Tab components and rendering

use eframe::egui::{self, Color32, DragValue, RichText, ScrollArea, Sense, Ui};
use eulumdat::{validate, validate_strict, Eulumdat, LampSet, Symmetry, TypeIndicator};

/// Application tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppTab {
    Diagram,
    General,
    Dimensions,
    Lamps,
    Optical,
    Intensity,
    Validation,
}

impl AppTab {
    pub fn label(&self) -> &'static str {
        match self {
            AppTab::Diagram => "Diagram",
            AppTab::General => "General",
            AppTab::Dimensions => "Dimensions",
            AppTab::Lamps => "Lamps",
            AppTab::Optical => "Optical",
            AppTab::Intensity => "Intensity",
            AppTab::Validation => "Validation",
        }
    }

    pub fn all() -> &'static [AppTab] {
        &[
            AppTab::Diagram,
            AppTab::General,
            AppTab::Dimensions,
            AppTab::Lamps,
            AppTab::Optical,
            AppTab::Intensity,
            AppTab::Validation,
        ]
    }
}

/// Render the tab bar
pub fn render_tab_bar(ui: &mut Ui, current_tab: &mut AppTab, has_data: bool) {
    ui.horizontal(|ui| {
        for tab in AppTab::all() {
            let selected = *current_tab == *tab;
            let enabled = has_data || *tab == AppTab::Diagram;

            let button = egui::Button::new(tab.label())
                .selected(selected)
                .sense(if enabled {
                    Sense::click()
                } else {
                    Sense::hover()
                });

            if ui.add_enabled(enabled, button).clicked() {
                *current_tab = *tab;
            }
        }
    });
}

/// Render the General tab
pub fn render_general_tab(ui: &mut Ui, ldt: &mut Eulumdat) {
    ScrollArea::vertical().show(ui, |ui| {
        ui.heading("Identification");
        ui.separator();

        egui::Grid::new("general_grid")
            .num_columns(2)
            .spacing([20.0, 8.0])
            .show(ui, |ui| {
                ui.label("Manufacturer/ID:");
                ui.text_edit_singleline(&mut ldt.identification);
                ui.end_row();

                ui.label("Luminaire Name:");
                ui.text_edit_singleline(&mut ldt.luminaire_name);
                ui.end_row();

                ui.label("Luminaire Number:");
                ui.text_edit_singleline(&mut ldt.luminaire_number);
                ui.end_row();

                ui.label("File Name:");
                ui.text_edit_singleline(&mut ldt.file_name);
                ui.end_row();

                ui.label("Date/User:");
                ui.text_edit_singleline(&mut ldt.date_user);
                ui.end_row();

                ui.label("Report Number:");
                ui.text_edit_singleline(&mut ldt.measurement_report_number);
                ui.end_row();
            });

        ui.add_space(20.0);
        ui.heading("Type");
        ui.separator();

        egui::Grid::new("type_grid")
            .num_columns(2)
            .spacing([20.0, 8.0])
            .show(ui, |ui| {
                ui.label("Type Indicator:");
                egui::ComboBox::from_id_salt("type_indicator")
                    .selected_text(format!("{:?}", ldt.type_indicator))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut ldt.type_indicator,
                            TypeIndicator::PointSourceSymmetric,
                            "Point Source (Symmetric)",
                        );
                        ui.selectable_value(
                            &mut ldt.type_indicator,
                            TypeIndicator::Linear,
                            "Linear",
                        );
                        ui.selectable_value(
                            &mut ldt.type_indicator,
                            TypeIndicator::PointSourceOther,
                            "Point Source (Other)",
                        );
                    });
                ui.end_row();

                ui.label("Symmetry:");
                egui::ComboBox::from_id_salt("symmetry")
                    .selected_text(format!("{:?}", ldt.symmetry))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut ldt.symmetry, Symmetry::None, "None (Full Data)");
                        ui.selectable_value(
                            &mut ldt.symmetry,
                            Symmetry::VerticalAxis,
                            "Vertical Axis",
                        );
                        ui.selectable_value(
                            &mut ldt.symmetry,
                            Symmetry::PlaneC0C180,
                            "Plane C0-C180",
                        );
                        ui.selectable_value(
                            &mut ldt.symmetry,
                            Symmetry::PlaneC90C270,
                            "Plane C90-C270",
                        );
                        ui.selectable_value(&mut ldt.symmetry, Symmetry::BothPlanes, "Both Planes");
                    });
                ui.end_row();
            });
    });
}

/// Render the Dimensions tab
pub fn render_dimensions_tab(ui: &mut Ui, ldt: &mut Eulumdat) {
    ScrollArea::vertical().show(ui, |ui| {
        ui.heading("Luminaire Dimensions (mm)");
        ui.separator();

        egui::Grid::new("dimensions_grid")
            .num_columns(2)
            .spacing([20.0, 8.0])
            .show(ui, |ui| {
                ui.label("Length:");
                ui.add(DragValue::new(&mut ldt.length).speed(1.0));
                ui.end_row();

                ui.label("Width:");
                ui.add(DragValue::new(&mut ldt.width).speed(1.0));
                ui.end_row();

                ui.label("Height:");
                ui.add(DragValue::new(&mut ldt.height).speed(1.0));
                ui.end_row();
            });

        ui.add_space(20.0);
        ui.heading("Luminous Area (mm)");
        ui.separator();

        egui::Grid::new("luminous_area_grid")
            .num_columns(2)
            .spacing([20.0, 8.0])
            .show(ui, |ui| {
                ui.label("Length:");
                ui.add(DragValue::new(&mut ldt.luminous_area_length).speed(1.0));
                ui.end_row();

                ui.label("Width:");
                ui.add(DragValue::new(&mut ldt.luminous_area_width).speed(1.0));
                ui.end_row();
            });

        ui.add_space(20.0);
        ui.heading("Height to Luminous Area (mm)");
        ui.separator();

        egui::Grid::new("height_grid")
            .num_columns(2)
            .spacing([20.0, 8.0])
            .show(ui, |ui| {
                ui.label("C0:");
                ui.add(DragValue::new(&mut ldt.height_c0).speed(0.1));
                ui.end_row();

                ui.label("C90:");
                ui.add(DragValue::new(&mut ldt.height_c90).speed(0.1));
                ui.end_row();

                ui.label("C180:");
                ui.add(DragValue::new(&mut ldt.height_c180).speed(0.1));
                ui.end_row();

                ui.label("C270:");
                ui.add(DragValue::new(&mut ldt.height_c270).speed(0.1));
                ui.end_row();
            });
    });
}

/// Render the Lamps tab
pub fn render_lamps_tab(ui: &mut Ui, ldt: &mut Eulumdat) {
    ScrollArea::vertical().show(ui, |ui| {
        ui.heading("Lamp Sets");
        ui.separator();

        let mut remove_index = None;

        for (i, lamp) in ldt.lamp_sets.iter_mut().enumerate() {
            ui.push_id(i, |ui| {
                egui::CollapsingHeader::new(format!("Lamp Set {} - {}", i + 1, lamp.lamp_type))
                    .default_open(true)
                    .show(ui, |ui| {
                        egui::Grid::new(format!("lamp_grid_{}", i))
                            .num_columns(2)
                            .spacing([20.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Number of Lamps:");
                                ui.add(
                                    DragValue::new(&mut lamp.num_lamps)
                                        .speed(1.0)
                                        .range(1..=100),
                                );
                                ui.end_row();

                                ui.label("Type:");
                                ui.text_edit_singleline(&mut lamp.lamp_type);
                                ui.end_row();

                                ui.label("Luminous Flux (lm):");
                                ui.add(DragValue::new(&mut lamp.total_luminous_flux).speed(10.0));
                                ui.end_row();

                                ui.label("Color Temp:");
                                ui.text_edit_singleline(&mut lamp.color_appearance);
                                ui.end_row();

                                ui.label("CRI Group:");
                                ui.text_edit_singleline(&mut lamp.color_rendering_group);
                                ui.end_row();

                                ui.label("Wattage (W):");
                                ui.add(DragValue::new(&mut lamp.wattage_with_ballast).speed(0.5));
                                ui.end_row();
                            });

                        if ui.button("Remove").clicked() {
                            remove_index = Some(i);
                        }
                    });
            });
            ui.add_space(5.0);
        }

        if let Some(idx) = remove_index {
            ldt.lamp_sets.remove(idx);
        }

        ui.add_space(10.0);
        if ui.button("+ Add Lamp Set").clicked() {
            ldt.lamp_sets.push(LampSet {
                num_lamps: 1,
                lamp_type: "LED".to_string(),
                total_luminous_flux: 1000.0,
                color_appearance: "3000".to_string(),
                color_rendering_group: "1A".to_string(),
                wattage_with_ballast: 10.0,
            });
        }
    });
}

/// Render the Optical tab
pub fn render_optical_tab(ui: &mut Ui, ldt: &mut Eulumdat) {
    ScrollArea::vertical().show(ui, |ui| {
        ui.heading("Light Output");
        ui.separator();

        egui::Grid::new("optical_grid")
            .num_columns(2)
            .spacing([20.0, 8.0])
            .show(ui, |ui| {
                ui.label("Light Output Ratio (%):");
                ui.add(
                    DragValue::new(&mut ldt.light_output_ratio)
                        .speed(0.5)
                        .range(0.0..=100.0),
                );
                ui.end_row();

                ui.label("Downward Flux (%):");
                ui.add(
                    DragValue::new(&mut ldt.downward_flux_fraction)
                        .speed(0.5)
                        .range(0.0..=100.0),
                );
                ui.end_row();

                ui.label("Tilt Angle:");
                ui.add(
                    DragValue::new(&mut ldt.tilt_angle)
                        .speed(1.0)
                        .range(-90.0..=90.0),
                );
                ui.end_row();

                ui.label("Conversion Factor:");
                ui.add(DragValue::new(&mut ldt.conversion_factor).speed(0.01));
                ui.end_row();
            });

        ui.add_space(20.0);
        ui.heading("Computed Values");
        ui.separator();

        egui::Grid::new("computed_grid")
            .num_columns(2)
            .spacing([20.0, 8.0])
            .show(ui, |ui| {
                ui.label("Max Intensity:");
                ui.label(format!("{:.1} cd/klm", ldt.max_intensity()));
                ui.end_row();

                ui.label("Total Flux:");
                ui.label(format!("{:.0} lm", ldt.total_luminous_flux()));
                ui.end_row();
            });

        ui.add_space(20.0);
        ui.heading("Direct Ratios (Room Index k)");
        ui.separator();

        let indices = [
            "0.60", "0.80", "1.00", "1.25", "1.50", "2.00", "2.50", "3.00", "4.00", "5.00",
        ];
        egui::Grid::new("direct_ratios_grid")
            .num_columns(2)
            .spacing([20.0, 4.0])
            .show(ui, |ui| {
                for (i, idx) in indices.iter().enumerate() {
                    if i < ldt.direct_ratios.len() {
                        ui.label(format!("k = {}:", idx));
                        ui.add(DragValue::new(&mut ldt.direct_ratios[i]).speed(0.01));
                        ui.end_row();
                    }
                }
            });
    });
}

/// State for the intensity tab
pub struct IntensityTabState {
    pub show_colors: bool,
}

/// Generate CSV from intensity data
fn generate_intensity_csv(ldt: &Eulumdat) -> String {
    let mut csv = String::new();

    // Header row: gamma \ C, C0, C15, C30, ...
    csv.push_str("gamma");
    for c_angle in &ldt.c_angles {
        csv.push('\t');
        csv.push_str(&format!("C{}", *c_angle as i32));
    }
    csv.push('\n');

    // Data rows
    for (g_idx, g_angle) in ldt.g_angles.iter().enumerate() {
        csv.push_str(&format!("{}", *g_angle as i32));
        for c_idx in 0..ldt.c_angles.len() {
            let intensity = if c_idx < ldt.intensities.len() && g_idx < ldt.intensities[c_idx].len()
            {
                ldt.intensities[c_idx][g_idx]
            } else {
                0.0
            };
            csv.push('\t');
            csv.push_str(&format!("{:.1}", intensity));
        }
        csv.push('\n');
    }

    csv
}

/// Render the Intensity tab
pub fn render_intensity_tab(ui: &mut Ui, ldt: &Eulumdat, state: &mut IntensityTabState) {
    // Toolbar
    ui.horizontal(|ui| {
        ui.heading("Intensities (cd/klm)");

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Copy CSV button
            if ui.button("Copy as CSV").clicked() {
                let csv = generate_intensity_csv(ldt);
                ui.output_mut(|o| o.copied_text = csv);
            }

            ui.separator();

            // Color toggle
            ui.checkbox(&mut state.show_colors, "Colors");

            ui.separator();

            // Stats
            ui.label(RichText::new(format!("Max: {:.1}", ldt.max_intensity())).small());
        });
    });
    ui.separator();

    // Main content
    if ldt.intensities.is_empty() || ldt.c_angles.is_empty() || ldt.g_angles.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label("No intensity data available");
        });
        return;
    }

    let max_intensity = ldt.max_intensity().max(1.0);
    let cell_width = 52.0;
    let header_width = 40.0;

    ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // Use a table-like layout with fixed column widths
            ui.vertical(|ui| {
                // Header row
                ui.horizontal(|ui| {
                    // Corner cell
                    ui.add_sized(
                        [header_width, 20.0],
                        egui::Label::new(RichText::new("γ \\ C").monospace().small().strong()),
                    );

                    // C-angle headers
                    for c_angle in &ldt.c_angles {
                        ui.add_sized(
                            [cell_width, 20.0],
                            egui::Label::new(
                                RichText::new(format!("{}", *c_angle as i32))
                                    .monospace()
                                    .small()
                                    .strong(),
                            ),
                        );
                    }
                });

                ui.separator();

                // Data rows
                for (g_idx, g_angle) in ldt.g_angles.iter().enumerate() {
                    ui.horizontal(|ui| {
                        // Row header (gamma angle)
                        ui.add_sized(
                            [header_width, 18.0],
                            egui::Label::new(
                                RichText::new(format!("{}", *g_angle as i32))
                                    .monospace()
                                    .small()
                                    .strong(),
                            ),
                        );

                        // Intensity values
                        for c_idx in 0..ldt.c_angles.len() {
                            let intensity = if c_idx < ldt.intensities.len()
                                && g_idx < ldt.intensities[c_idx].len()
                            {
                                ldt.intensities[c_idx][g_idx]
                            } else {
                                0.0
                            };

                            let text = format!("{:.1}", intensity);

                            if state.show_colors {
                                let normalized = intensity / max_intensity;
                                let bg_color = heatmap_color(normalized);
                                let text_color = if normalized > 0.5 {
                                    Color32::WHITE
                                } else {
                                    Color32::BLACK
                                };

                                egui::Frame::none()
                                    .fill(bg_color)
                                    .inner_margin(egui::Margin::symmetric(2.0, 1.0))
                                    .show(ui, |ui| {
                                        ui.add_sized(
                                            [cell_width - 4.0, 16.0],
                                            egui::Label::new(
                                                RichText::new(&text)
                                                    .monospace()
                                                    .small()
                                                    .color(text_color),
                                            ),
                                        );
                                    });
                            } else {
                                ui.add_sized(
                                    [cell_width, 18.0],
                                    egui::Label::new(RichText::new(&text).monospace().small()),
                                );
                            }
                        }
                    });
                }
            });
        });

    // Footer
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!(
                "{} C-planes × {} γ-angles = {} values",
                ldt.c_angles.len(),
                ldt.g_angles.len(),
                ldt.c_angles.len() * ldt.g_angles.len()
            ))
            .small()
            .color(Color32::GRAY),
        );

        if state.show_colors {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new("max").small());
                for i in (0..=9).rev() {
                    let color = heatmap_color(i as f64 / 9.0);
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), Sense::hover());
                    ui.painter().rect_filled(rect, 2.0, color);
                }
                ui.label(RichText::new("0").small());
            });
        }
    });
}

/// Render the Validation tab
pub fn render_validation_tab(ui: &mut Ui, ldt: &Eulumdat) {
    let warnings = validate(ldt);
    let errors = match validate_strict(ldt) {
        Ok(()) => vec![],
        Err(e) => e,
    };

    ui.heading("Validation Results");
    ui.separator();

    // Summary header
    let has_errors = !errors.is_empty();
    let has_warnings = !warnings.is_empty();

    let (status_text, status_color) = if has_errors {
        ("Validation Failed", Color32::from_rgb(220, 53, 69))
    } else if has_warnings {
        ("Passed with Warnings", Color32::from_rgb(255, 193, 7))
    } else {
        ("Validation Passed", Color32::from_rgb(40, 167, 69))
    };

    ui.horizontal(|ui| {
        ui.label(RichText::new("●").color(status_color).size(20.0));
        ui.label(RichText::new(status_text).strong().size(16.0));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if has_errors || has_warnings {
                ui.label(
                    RichText::new(format!(
                        "{} error{}, {} warning{}",
                        errors.len(),
                        if errors.len() == 1 { "" } else { "s" },
                        warnings.len(),
                        if warnings.len() == 1 { "" } else { "s" }
                    ))
                    .small()
                    .color(Color32::GRAY),
                );
            }
        });
    });

    ui.separator();

    if !has_errors && !has_warnings {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(
                RichText::new("✓")
                    .size(48.0)
                    .color(Color32::from_rgb(40, 167, 69)),
            );
            ui.label(RichText::new("No validation issues found").size(16.0));
            ui.add_space(40.0);
        });
    } else {
        ScrollArea::vertical().show(ui, |ui| {
            // Errors first (red)
            if !errors.is_empty() {
                ui.label(
                    RichText::new(format!("Errors ({})", errors.len()))
                        .strong()
                        .color(Color32::from_rgb(220, 53, 69)),
                );
                ui.add_space(5.0);

                for error in &errors {
                    egui::Frame::none()
                        .fill(Color32::from_rgb(248, 215, 218))
                        .inner_margin(8.0)
                        .rounding(4.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("✕").color(Color32::from_rgb(220, 53, 69)));
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(error.code).strong());
                                    ui.label(&error.message);
                                });
                            });
                        });
                    ui.add_space(5.0);
                }

                ui.add_space(10.0);
            }

            // Warnings (orange/yellow)
            if !warnings.is_empty() {
                ui.label(
                    RichText::new(format!("Warnings ({})", warnings.len()))
                        .strong()
                        .color(Color32::from_rgb(255, 193, 7)),
                );
                ui.add_space(5.0);

                for warning in &warnings {
                    egui::Frame::none()
                        .fill(Color32::from_rgb(255, 243, 205))
                        .inner_margin(8.0)
                        .rounding(4.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("!").color(Color32::from_rgb(255, 193, 7)));
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(warning.code.to_string()).strong());
                                    ui.label(&warning.message);
                                });
                            });
                        });
                    ui.add_space(5.0);
                }
            }
        });
    }
}

/// Heatmap color function (blue -> cyan -> green -> yellow -> red)
fn heatmap_color(normalized: f64) -> Color32 {
    let value = normalized.clamp(0.0, 1.0);

    let (r, g, b) = if value < 0.25 {
        let t = value / 0.25;
        (0.0, t, 1.0)
    } else if value < 0.5 {
        let t = (value - 0.25) / 0.25;
        (0.0, 1.0, 1.0 - t)
    } else if value < 0.75 {
        let t = (value - 0.5) / 0.25;
        (t, 1.0, 0.0)
    } else {
        let t = (value - 0.75) / 0.25;
        (1.0, 1.0 - t, 0.0)
    };

    Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}
