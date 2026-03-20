//! Keyboard controls for the viewer.

use super::scenes::SceneType;
use super::ViewerSettings;
use bevy::prelude::*;
use eulumdat::Eulumdat;

/// Dimension adjustment step size in meters.
const DIMENSION_STEP: f32 = 0.5;
/// Fine adjustment step size in meters (for pendulum, mounting height).
const FINE_STEP: f32 = 0.1;
/// Minimum room dimension in meters.
const MIN_DIMENSION: f32 = 1.0;
/// Maximum room dimension in meters.
const MAX_DIMENSION: f32 = 50.0;
/// Minimum room height in meters.
const MIN_HEIGHT: f32 = 2.0;
/// Maximum room height in meters.
const MAX_HEIGHT: f32 = 20.0;
/// Maximum pendulum length in meters (high-bay, industrial, street lamps).
const MAX_PENDULUM: f32 = 20.0;

/// Keyboard control system for the 3D viewer.
///
/// # Key bindings
///
/// ## Visualization toggles
/// - `P`: Toggle photometric solid
/// - `L`: Toggle luminaire model
/// - `H`: Toggle shadows
///
/// ## Scene selection
/// - `1-4`: Switch scene type (Room, Road, Parking, Outdoor)
/// - `5`: Designer Exterior (Area Lighting)
/// - `6`: Designer Interior (Zonal Cavity)
///
/// ## Designer toggles
/// - `C`: Toggle cavity zone overlays (interior)
/// - `V`: Toggle light cone visualization
///
/// ## Room dimensions
/// - `[` / `]`: Decrease/increase room width (±0.5m)
/// - `-` / `=`: Decrease/increase room length (±0.5m)
/// - `9` / `0`: Decrease/increase room height (±0.5m)
///
/// ## Luminaire positioning
/// - `;` / `'`: Decrease/increase pendulum/suspension length (±0.1m)
/// - `,` / `.`: Decrease/increase mounting height (±0.1m, pole height for outdoor)
/// - `T` / `Y`: Decrease/increase luminaire tilt angle (±5°, for road scenes)
pub fn viewer_controls_system(
    mut settings: ResMut<ViewerSettings>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    // Toggle photometric solid with P key
    if keyboard.just_pressed(KeyCode::KeyP) {
        settings.show_photometric_solid = !settings.show_photometric_solid;
    }

    // Toggle luminaire with L key
    if keyboard.just_pressed(KeyCode::KeyL) {
        settings.show_luminaire = !settings.show_luminaire;
    }

    // Toggle shadows with H key (H = Hide/show shadows)
    if keyboard.just_pressed(KeyCode::KeyH) {
        settings.show_shadows = !settings.show_shadows;
    }

    // Cycle scene types with 1-4 keys
    if keyboard.just_pressed(KeyCode::Digit1) {
        settings.scene_type = SceneType::Room;
    }
    if keyboard.just_pressed(KeyCode::Digit2) {
        settings.scene_type = SceneType::Road;
    }
    if keyboard.just_pressed(KeyCode::Digit3) {
        settings.scene_type = SceneType::Parking;
    }
    if keyboard.just_pressed(KeyCode::Digit4) {
        settings.scene_type = SceneType::Outdoor;
    }
    if keyboard.just_pressed(KeyCode::Digit5) {
        settings.scene_type = SceneType::DesignerExterior;
        // Auto-populate exterior designer data from LDT if not already set
        if settings.area_result.is_none() {
            if let Some(ldt) = settings.ldt_data.clone() {
                populate_exterior_defaults(&mut settings, &ldt);
            }
        }
    }
    if keyboard.just_pressed(KeyCode::Digit6) {
        settings.scene_type = SceneType::DesignerInterior;
        // Auto-populate interior designer data from LDT if not already set
        if settings.designer_room.is_none() {
            if let Some(ldt) = settings.ldt_data.clone() {
                populate_interior_defaults(&mut settings, &ldt);
            }
        }
    }

    // Designer toggles
    if keyboard.just_pressed(KeyCode::KeyC) {
        settings.show_cavities = !settings.show_cavities;
    }
    if keyboard.just_pressed(KeyCode::KeyV) {
        settings.show_light_cones = !settings.show_light_cones;
    }

    // Room dimension controls (only for Room scene, but allow adjustment for all)
    // Width: [ and ]
    if keyboard.just_pressed(KeyCode::BracketLeft) {
        settings.room_width = (settings.room_width - DIMENSION_STEP).max(MIN_DIMENSION);
    }
    if keyboard.just_pressed(KeyCode::BracketRight) {
        settings.room_width = (settings.room_width + DIMENSION_STEP).min(MAX_DIMENSION);
    }

    // Length: - and =
    if keyboard.just_pressed(KeyCode::Minus) {
        settings.room_length = (settings.room_length - DIMENSION_STEP).max(MIN_DIMENSION);
    }
    if keyboard.just_pressed(KeyCode::Equal) {
        settings.room_length = (settings.room_length + DIMENSION_STEP).min(MAX_DIMENSION);
    }

    // Height: 9 and 0
    if keyboard.just_pressed(KeyCode::Digit9) {
        settings.room_height = (settings.room_height - DIMENSION_STEP).max(MIN_HEIGHT);
        // Also adjust mounting height if it exceeds room height
        if settings.mounting_height > settings.room_height - 0.1 {
            settings.mounting_height = settings.room_height - 0.1;
        }
    }
    if keyboard.just_pressed(KeyCode::Digit0) {
        settings.room_height = (settings.room_height + DIMENSION_STEP).min(MAX_HEIGHT);
    }

    // Pendulum/suspension length: ; and '
    if keyboard.just_pressed(KeyCode::Semicolon) {
        settings.pendulum_length = (settings.pendulum_length - FINE_STEP).max(0.0);
    }
    if keyboard.just_pressed(KeyCode::Quote) {
        // Max pendulum = attachment_height - 1.0 (leave 1m clearance from floor)
        let max_pendulum = (settings.attachment_height() - 1.0).clamp(0.0, MAX_PENDULUM);
        settings.pendulum_length = (settings.pendulum_length + FINE_STEP).min(max_pendulum);
    }

    // Mounting height (for outdoor scenes): , and .
    if keyboard.just_pressed(KeyCode::Comma) {
        settings.mounting_height = (settings.mounting_height - FINE_STEP).max(2.0);
    }
    if keyboard.just_pressed(KeyCode::Period) {
        settings.mounting_height = (settings.mounting_height + FINE_STEP).min(MAX_HEIGHT);
    }

    // Luminaire tilt angle (for road scenes): T and Y
    const TILT_STEP: f32 = 5.0;
    if keyboard.just_pressed(KeyCode::KeyT) {
        settings.luminaire_tilt = (settings.luminaire_tilt - TILT_STEP).max(0.0);
    }
    if keyboard.just_pressed(KeyCode::KeyY) {
        settings.luminaire_tilt = (settings.luminaire_tilt + TILT_STEP).min(90.0);
    }
}

/// System to sync ViewerSettings with PhotometricLight components.
///
/// When settings change, this system:
/// - Updates visualization flags (solid, model, shadows)
/// - Respawns all luminaires if the count changes (scene type change, etc.)
/// - Updates light positions and rotations
pub fn sync_viewer_to_lights(
    mut commands: Commands,
    settings: Res<ViewerSettings>,
    lights: Query<(
        Entity,
        &crate::photometric::PhotometricLight<Eulumdat>,
        &Transform,
    )>,
) {
    if !settings.is_changed() {
        return;
    }

    // Get LDT data from first light
    let ldt_data = lights.iter().next().map(|(_, l, _)| l.data.clone());
    let Some(ldt) = ldt_data else {
        return;
    };

    // Calculate required transforms for current scene
    let transforms = calculate_all_luminaire_transforms(&settings, &ldt);
    let current_count = lights.iter().count();
    let required_count = transforms.len();

    // If count changed, despawn all and respawn
    if current_count != required_count {
        // Despawn all existing lights
        for (entity, _, _) in lights.iter() {
            commands.entity(entity).despawn();
        }

        // Spawn new lights
        for transform in transforms {
            commands.spawn(
                crate::eulumdat_impl::EulumdatLightBundle::new(ldt.clone())
                    .with_transform(
                        Transform::from_translation(transform.position)
                            .with_rotation(transform.rotation),
                    )
                    .with_solid(settings.show_photometric_solid)
                    .with_model(settings.show_luminaire)
                    .with_shadows(settings.show_shadows),
            );
        }
    } else {
        // Just update existing lights in place
        for (idx, (entity, light, _)) in lights.iter().enumerate() {
            if let Some(lt) = transforms.get(idx) {
                let mut updated_light =
                    crate::photometric::PhotometricLight::new(light.data.clone());
                updated_light.show_solid = settings.show_photometric_solid;
                updated_light.show_model = settings.show_luminaire;
                updated_light.shadow_maps_enabled = settings.show_shadows;
                updated_light.intensity_scale = light.intensity_scale;

                commands.entity(entity).insert((
                    Transform::from_translation(lt.position).with_rotation(lt.rotation),
                    updated_light,
                ));
            }
        }
    }
}

/// Luminaire position with rotation for multi-luminaire scenes.
#[derive(Clone, Copy)]
pub struct LuminaireTransform {
    pub position: Vec3,
    pub rotation: Quat,
}

/// Calculate all luminaire positions for the current scene.
/// Returns a list of positions and rotations for each luminaire.
pub fn calculate_all_luminaire_transforms(
    settings: &ViewerSettings,
    ldt: &Eulumdat,
) -> Vec<LuminaireTransform> {
    let y = settings.luminaire_height(ldt);

    match settings.scene_type {
        SceneType::Room => {
            // Single luminaire centered in room
            vec![LuminaireTransform {
                position: Vec3::new(settings.room_width / 2.0, y, settings.room_length / 2.0),
                rotation: Quat::IDENTITY,
            }]
        }
        SceneType::Road => calculate_road_luminaires(settings, y),
        SceneType::Parking | SceneType::Outdoor => {
            // Single luminaire for now
            vec![LuminaireTransform {
                position: Vec3::new(
                    settings.room_width / 2.0 - 0.2,
                    y,
                    settings.room_length / 2.0,
                ),
                rotation: Quat::IDENTITY,
            }]
        }
        SceneType::DesignerExterior => {
            super::designer_scenes::calculate_exterior_transforms(&settings.area_placements)
        }
        SceneType::DesignerInterior => {
            match (&settings.designer_room, &settings.designer_layout) {
                (Some(room), Some(layout)) => {
                    super::designer_scenes::calculate_interior_transforms(room, layout)
                }
                _ => vec![],
            }
        }
    }
}

/// Calculate luminaire positions for road scene based on EN 13201 guidelines.
/// Luminaires are placed on outer sides (sidewalks) to illuminate both road and pedestrian areas.
/// The wider part of the LDC faces the road, softer part faces the sidewalk.
/// Middle poles are added every 50m for better center illumination on wide roads.
fn calculate_road_luminaires(settings: &ViewerSettings, y: f32) -> Vec<LuminaireTransform> {
    let lane_w = settings.lane_width;
    let num_lanes = settings.num_lanes;
    let sidewalk_w = settings.sidewalk_width;
    let road_width = num_lanes as f32 * lane_w;
    let total_width = road_width + 2.0 * sidewalk_w;
    let road_length = settings.room_length;
    let pole_spacing = settings.effective_pole_spacing();

    // Calculate number of poles and actual spacing
    let num_poles = ((road_length / pole_spacing).floor() as i32).max(1);
    let actual_spacing = road_length / (num_poles as f32 + 1.0);

    // Determine arrangement based on road/height ratio
    let ratio = road_width / settings.mounting_height;
    let tilt = settings.luminaire_tilt.to_radians();

    // Arm extends from pole toward road center
    let arm_length = 1.5;

    let mut transforms = Vec::new();

    // Middle pole spacing (every 50m for center illumination on wide roads)
    let middle_pole_spacing = 50.0;
    let center_x = sidewalk_w + road_width / 2.0;

    if ratio < 1.0 {
        // Single side arrangement - poles on right sidewalk
        // Luminaire faces LEFT toward road (positive Z rotation tilts light toward -X)
        let rotation = Quat::from_rotation_z(tilt);
        for i in 1..=num_poles {
            let z = i as f32 * actual_spacing;
            transforms.push(LuminaireTransform {
                position: Vec3::new(total_width - sidewalk_w / 2.0 - arm_length, y, z),
                rotation,
            });
        }
    } else if ratio < 1.5 {
        // Staggered arrangement - alternating sides on sidewalks
        for i in 1..=num_poles {
            let z = i as f32 * actual_spacing;
            if i % 2 == 0 {
                // Left sidewalk - luminaire faces RIGHT toward road (negative Z rotation)
                transforms.push(LuminaireTransform {
                    position: Vec3::new(sidewalk_w / 2.0 + arm_length, y, z),
                    rotation: Quat::from_rotation_z(-tilt),
                });
            } else {
                // Right sidewalk - luminaire faces LEFT toward road (positive Z rotation)
                transforms.push(LuminaireTransform {
                    position: Vec3::new(total_width - sidewalk_w / 2.0 - arm_length, y, z),
                    rotation: Quat::from_rotation_z(tilt),
                });
            }
        }
    } else {
        // Opposite arrangement - poles on both sidewalks, aligned
        // Each side illuminates its sidewalk + half the road
        for i in 1..=num_poles {
            let z = i as f32 * actual_spacing;
            // Left sidewalk - luminaire faces RIGHT toward road (negative Z rotation)
            transforms.push(LuminaireTransform {
                position: Vec3::new(sidewalk_w / 2.0 + arm_length, y, z),
                rotation: Quat::from_rotation_z(-tilt),
            });
            // Right sidewalk - luminaire faces LEFT toward road (positive Z rotation)
            transforms.push(LuminaireTransform {
                position: Vec3::new(total_width - sidewalk_w / 2.0 - arm_length, y, z),
                rotation: Quat::from_rotation_z(tilt),
            });
        }

        // Add middle poles every 50m for better center illumination
        if road_width > 6.0 {
            let num_middle_poles = ((road_length / middle_pole_spacing).floor() as i32).max(0);
            for i in 1..=num_middle_poles {
                let z = i as f32 * middle_pole_spacing;
                // Middle pole with two luminaires pointing outward (no tilt, straight down)
                // Left-facing luminaire
                transforms.push(LuminaireTransform {
                    position: Vec3::new(center_x - 1.0, y, z),
                    rotation: Quat::from_rotation_z(-tilt * 0.5), // Less tilt for center
                });
                // Right-facing luminaire
                transforms.push(LuminaireTransform {
                    position: Vec3::new(center_x + 1.0, y, z),
                    rotation: Quat::from_rotation_z(tilt * 0.5),
                });
            }
        }
    }

    transforms
}

/// Calculate light position based on scene type and settings.
/// Returns position for the first/primary luminaire only.
/// For multi-luminaire scenes, use `calculate_all_luminaire_transforms`.
pub fn calculate_light_position(settings: &ViewerSettings, ldt: &Eulumdat) -> Vec3 {
    let transforms = calculate_all_luminaire_transforms(settings, ldt);
    transforms.first().map(|t| t.position).unwrap_or(Vec3::ZERO)
}

/// Calculate light rotation based on scene type.
///
/// For road luminaires, the luminaire should be tilted to point across the road.
/// The pole is on the right side of the road, so the luminaire tilts left (toward road center).
/// The tilt angle is controlled by `settings.luminaire_tilt` (0° = down, 90° = horizontal).
pub fn calculate_light_rotation(settings: &ViewerSettings) -> Quat {
    match settings.scene_type {
        SceneType::Room => Quat::IDENTITY, // No rotation for indoor
        SceneType::Road => {
            // Road luminaire needs to be tilted to point across the road
            // Pole is on right side (high X), luminaire points toward road center (low X)
            //
            // Rotate around Z axis with NEGATIVE angle to tilt DOWN toward road (negative X)
            // luminaire_tilt: 0 = pointing straight down, 90 = pointing horizontally toward road
            let tilt_angle = -settings.luminaire_tilt.to_radians();
            Quat::from_rotation_z(tilt_angle)
        }
        SceneType::Parking => Quat::IDENTITY, // Parking lots typically want omnidirectional
        SceneType::Outdoor => Quat::IDENTITY, // Garden lights typically want omnidirectional
        SceneType::DesignerExterior | SceneType::DesignerInterior => Quat::IDENTITY,
    }
}

/// Auto-populate interior designer data from the loaded LDT.
///
/// Uses the zonal cavity method to compute a realistic room, layout,
/// reflectances, cavity ratios, and point-by-point illuminance grid
/// so the 3D viewer has something to show in native mode.
fn populate_interior_defaults(settings: &mut ViewerSettings, ldt: &Eulumdat) {
    use eulumdat::zonal;
    use eulumdat::CuTable;

    let room = zonal::Room::new(
        settings.room_length as f64,
        settings.room_width as f64,
        settings.room_height as f64,
        0.80,
        settings.pendulum_length as f64,
    );
    let reflectances = zonal::Reflectances::new(0.80, 0.50, 0.20);
    let llf = zonal::LightLossFactor::new(0.90, 0.95, 1.0, 0.98);
    let cu_table = CuTable::calculate(ldt);

    let zr = zonal::compute_zonal(
        ldt,
        &room,
        &reflectances,
        &llf,
        500.0, // 500 lux target
        &cu_table,
        zonal::SolveMode::TargetToCount,
        None,
        None,
    );

    // Compute PPB overlay for workplane heatmap
    let ppb = zonal::compute_ppb_overlay(
        ldt, &zr.layout, &room, 20, zr.llf_total, zr.cu, zr.achieved_illuminance,
    );

    settings.designer_room = Some(room);
    settings.designer_layout = Some(zr.layout);
    settings.designer_reflectances = Some(reflectances);
    settings.designer_cavity = Some(zr.cavity);
    settings.designer_ppb = Some(ppb);
}

/// Auto-populate exterior designer data from the loaded LDT.
///
/// Creates a simple 2×2 grid of luminaire placements and computes
/// the area illuminance heatmap.
fn populate_exterior_defaults(settings: &mut ViewerSettings, ldt: &Eulumdat) {
    use eulumdat::area;

    let area_w = 20.0;
    let area_d = 20.0;
    let mh = settings.mounting_height as f64;

    // Create a 2×2 grid of poles
    let placements = vec![
        area::LuminairePlace::simple(0, 5.0, 5.0, mh),
        area::LuminairePlace::simple(1, 15.0, 5.0, mh),
        area::LuminairePlace::simple(2, 5.0, 15.0, mh),
        area::LuminairePlace::simple(3, 15.0, 15.0, mh),
    ];

    let result = area::compute_area_illuminance(ldt, &placements, area_w, area_d, 20, 1.0);
    settings.area_result = Some(result);
    settings.area_placements = placements;
}
