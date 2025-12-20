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
///
/// ## Room dimensions
/// - `[` / `]`: Decrease/increase room width (±0.5m)
/// - `-` / `=`: Decrease/increase room length (±0.5m)
/// - `9` / `0`: Decrease/increase room height (±0.5m)
///
/// ## Luminaire positioning
/// - `;` / `'`: Decrease/increase pendulum/suspension length (±0.1m)
/// - `,` / `.`: Decrease/increase mounting height (±0.1m, pole height for outdoor)
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
}

/// System to sync ViewerSettings with PhotometricLight components.
///
/// When settings change, this system updates:
/// - Visualization flags (solid, model, shadows)
/// - Light position (based on room dimensions and mounting height)
pub fn sync_viewer_to_lights(
    settings: Res<ViewerSettings>,
    mut lights: Query<(
        &mut crate::photometric::PhotometricLight<Eulumdat>,
        &mut Transform,
    )>,
) {
    if !settings.is_changed() {
        return;
    }

    for (mut light, mut transform) in lights.iter_mut() {
        // Update visualization flags
        light.show_solid = settings.show_photometric_solid;
        light.show_model = settings.show_luminaire;
        light.shadows_enabled = settings.show_shadows;

        // Update light position based on current settings
        let position = calculate_light_position(&settings, &light.data);
        transform.translation = position;
    }
}

/// Calculate light position based on scene type and settings.
///
/// For Room scene:
/// - X, Z: centered in the room
/// - Y: ceiling height - pendulum length - half luminaire height
///
/// For outdoor scenes (Road, Parking, Outdoor):
/// - X, Z: positioned relative to pole
/// - Y: mounting height - arm offset - half luminaire height
pub fn calculate_light_position(settings: &ViewerSettings, ldt: &Eulumdat) -> Vec3 {
    let y = settings.luminaire_height(ldt);

    match settings.scene_type {
        SceneType::Room => Vec3::new(settings.room_width / 2.0, y, settings.room_length / 2.0),
        SceneType::Road => Vec3::new(
            settings.room_width - 0.7 - 0.2, // On right sidewalk, arm extends left
            y,
            settings.room_length / 2.0,
        ),
        SceneType::Parking | SceneType::Outdoor => Vec3::new(
            settings.room_width / 2.0 - 0.2, // Center, arm extends left
            y,
            settings.room_length / 2.0,
        ),
    }
}
