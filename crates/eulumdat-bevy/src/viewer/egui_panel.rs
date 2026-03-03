//! Egui settings panel for the native viewer.
//!
//! Provides a UI for adjusting viewer settings when running the standalone app.

use super::scenes::SceneType;
use super::ViewerSettings;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};

/// Plugin that adds the egui settings panel.
pub struct EguiSettingsPlugin;

impl Plugin for EguiSettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin::default())
            .add_systems(Update, settings_panel_system);
    }
}

/// System that renders the egui settings panel.
fn settings_panel_system(mut contexts: EguiContexts, mut settings: ResMut<ViewerSettings>) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    egui::SidePanel::left("settings_panel")
        .default_width(240.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("Viewer Settings");
            ui.separator();

            // Scene Type
            ui.label("Scene Type");
            egui::ComboBox::from_id_salt("scene_type")
                .selected_text(match settings.scene_type {
                    SceneType::Room => "Room",
                    SceneType::Road => "Road",
                    SceneType::Parking => "Parking",
                    SceneType::Outdoor => "Outdoor",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut settings.scene_type, SceneType::Room, "Room");
                    ui.selectable_value(&mut settings.scene_type, SceneType::Road, "Road");
                    ui.selectable_value(&mut settings.scene_type, SceneType::Parking, "Parking");
                    ui.selectable_value(&mut settings.scene_type, SceneType::Outdoor, "Outdoor");
                });

            ui.add_space(12.0);

            // Scene-specific settings
            match settings.scene_type {
                SceneType::Room => {
                    ui.label("Room Dimensions");
                    ui.horizontal(|ui| {
                        ui.label("Width (m):");
                        ui.add(
                            egui::DragValue::new(&mut settings.room_width)
                                .range(1.0..=20.0)
                                .speed(0.1),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Length (m):");
                        ui.add(
                            egui::DragValue::new(&mut settings.room_length)
                                .range(1.0..=30.0)
                                .speed(0.1),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Height (m):");
                        ui.add(
                            egui::DragValue::new(&mut settings.room_height)
                                .range(2.0..=10.0)
                                .speed(0.1),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Pendulum (m):");
                        ui.add(
                            egui::DragValue::new(&mut settings.pendulum_length)
                                .range(0.0..=3.0)
                                .speed(0.05),
                        );
                    });
                }
                SceneType::Road => {
                    ui.label("Road Layout");
                    ui.horizontal(|ui| {
                        ui.label("Lanes:");
                        let mut lanes = settings.num_lanes as i32;
                        if ui
                            .add(egui::DragValue::new(&mut lanes).range(1..=6).speed(0.1))
                            .changed()
                        {
                            settings.num_lanes = lanes as u32;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Lane Width (m):");
                        ui.add(
                            egui::DragValue::new(&mut settings.lane_width)
                                .range(2.5..=4.5)
                                .speed(0.1),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Sidewalk (m):");
                        ui.add(
                            egui::DragValue::new(&mut settings.sidewalk_width)
                                .range(1.0..=4.0)
                                .speed(0.1),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Road Length (m):");
                        ui.add(
                            egui::DragValue::new(&mut settings.room_length)
                                .range(30.0..=200.0)
                                .speed(1.0),
                        );
                    });

                    ui.add_space(8.0);
                    ui.label("Luminaire Settings");
                    ui.horizontal(|ui| {
                        ui.label("Mount Height (m):");
                        ui.add(
                            egui::DragValue::new(&mut settings.mounting_height)
                                .range(4.0..=15.0)
                                .speed(0.1),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Tilt Angle (°):");
                        ui.add(
                            egui::DragValue::new(&mut settings.luminaire_tilt)
                                .range(0.0..=45.0)
                                .speed(1.0),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Pole Spacing (m):");
                        ui.add(
                            egui::DragValue::new(&mut settings.pole_spacing)
                                .range(0.0..=60.0)
                                .speed(1.0),
                        );
                    });
                    if settings.pole_spacing == 0.0 {
                        ui.small("(0 = auto: 3.5× mount height)");
                    }

                    // Show calculated info
                    ui.add_space(8.0);
                    let total_width = settings.total_road_width();
                    let spacing = settings.effective_pole_spacing();
                    let num_poles = (settings.room_length / spacing).floor() as i32;
                    ui.label(format!(
                        "Total: {:.1}m wide | {} poles | {:.1}m spacing",
                        total_width,
                        num_poles.max(1),
                        spacing
                    ));
                }
                SceneType::Parking | SceneType::Outdoor => {
                    ui.label("Area Dimensions");
                    ui.horizontal(|ui| {
                        ui.label("Width (m):");
                        ui.add(
                            egui::DragValue::new(&mut settings.room_width)
                                .range(5.0..=50.0)
                                .speed(0.5),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Length (m):");
                        ui.add(
                            egui::DragValue::new(&mut settings.room_length)
                                .range(10.0..=100.0)
                                .speed(1.0),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Mount Height (m):");
                        ui.add(
                            egui::DragValue::new(&mut settings.mounting_height)
                                .range(3.0..=15.0)
                                .speed(0.1),
                        );
                    });
                }
            }

            ui.add_space(16.0);
            ui.separator();
            ui.label("Display Options");

            ui.checkbox(&mut settings.show_luminaire, "Show Luminaire");
            ui.checkbox(
                &mut settings.show_photometric_solid,
                "Show Photometric Solid",
            );
            ui.checkbox(&mut settings.show_shadows, "Enable Shadows");

            ui.add_space(16.0);
            ui.separator();
            ui.label("Camera Controls");
            ui.small("WASD / Arrows: Move");
            ui.small("Q / E: Up / Down");
            ui.small("Right-click + drag: Look");
            ui.small("R: Reset view");
        });
}
