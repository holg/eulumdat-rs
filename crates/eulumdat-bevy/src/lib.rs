//! Eulumdat 3D Scene Viewer Library
//!
//! This module exposes the Bevy-based 3D viewer as a library that can be
//! embedded in other applications (like the Leptos WASM editor).

pub mod camera;
pub mod lighting;
pub mod scene;

use bevy::prelude::*;
use camera::CameraPlugin;
use lighting::PhotometricLightPlugin;
use scene::{ScenePlugin, SceneType};

/// Resource to track localStorage timestamp for hot-reload
#[derive(Resource, Default)]
pub struct LdtTimestamp(pub String);

/// Global scene settings - exposed for external modification
#[derive(Resource)]
pub struct SceneSettings {
    pub scene_type: SceneType,
    pub room_width: f32,
    pub room_length: f32,
    pub room_height: f32,
    pub mounting_height: f32,
    pub light_intensity: f32,
    pub show_luminaire: bool,
    pub show_photometric_solid: bool,
    pub show_shadows: bool,
    pub ldt_data: Option<eulumdat::Eulumdat>,
}

impl Default for SceneSettings {
    fn default() -> Self {
        Self {
            scene_type: SceneType::Room,
            room_width: 4.0,
            room_length: 5.0,
            room_height: 2.8,
            mounting_height: 2.5,
            light_intensity: 1000.0,
            show_luminaire: true,
            show_photometric_solid: false,
            show_shadows: false,
            ldt_data: None,
        }
    }
}

#[cfg(target_arch = "wasm32")]
const LDT_STORAGE_KEY: &str = "eulumdat_current_ldt";
#[cfg(target_arch = "wasm32")]
const LDT_TIMESTAMP_KEY: &str = "eulumdat_ldt_timestamp";

/// Load LDT from localStorage (WASM only)
#[cfg(target_arch = "wasm32")]
pub fn load_from_local_storage() -> Option<eulumdat::Eulumdat> {
    use wasm_bindgen::JsCast;

    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    let ldt_string = storage.get_item(LDT_STORAGE_KEY).ok()??;

    eulumdat::Eulumdat::parse(&ldt_string).ok()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_from_local_storage() -> Option<eulumdat::Eulumdat> {
    None
}

/// Get timestamp from localStorage
#[cfg(target_arch = "wasm32")]
pub fn get_ldt_timestamp() -> Option<String> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    storage.get_item(LDT_TIMESTAMP_KEY).ok()?
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get_ldt_timestamp() -> Option<String> {
    None
}

/// Load default LDT data
pub fn load_default_ldt() -> Option<eulumdat::Eulumdat> {
    // For WASM, try to load from localStorage first (synced from editor)
    #[cfg(target_arch = "wasm32")]
    {
        // Try localStorage first
        if let Some(ldt) = load_from_local_storage() {
            return Some(ldt);
        }
        // Fallback to embedded sample
        let ldt_content = include_str!("../../eulumdat-wasm/templates/road_luminaire.ldt");
        eulumdat::Eulumdat::parse(ldt_content).ok()
    }

    // For native, try to load from file
    #[cfg(not(target_arch = "wasm32"))]
    {
        let sample_paths = [
            "crates/eulumdat-wasm/templates/road_luminaire.ldt",
            "../eulumdat-wasm/templates/road_luminaire.ldt",
            "crates/eulumdat-wasm/templates/fluorescent_luminaire.ldt",
            "../eulumdat-wasm/templates/fluorescent_luminaire.ldt",
        ];

        for path in sample_paths {
            if let Ok(ldt) = eulumdat::Eulumdat::from_file(path) {
                return Some(ldt);
            }
        }
        None
    }
}

/// Poll localStorage for LDT changes
#[allow(unused_mut, unused_variables)]
pub fn poll_ldt_changes(
    mut settings: ResMut<SceneSettings>,
    mut last_timestamp: ResMut<LdtTimestamp>,
) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(new_timestamp) = get_ldt_timestamp() {
            if new_timestamp != last_timestamp.0 {
                // Timestamp changed - reload LDT
                if let Some(ldt) = load_from_local_storage() {
                    settings.ldt_data = Some(ldt);
                    last_timestamp.0 = new_timestamp;
                }
            }
        }
    }
}

/// Keyboard control system for the 3D viewer
pub fn ui_controls_system(
    mut settings: ResMut<SceneSettings>,
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
}

/// Startup system to initialize scene with default LDT
fn setup_with_default_ldt(mut commands: Commands) {
    let ldt = load_default_ldt();
    commands.insert_resource(SceneSettings {
        ldt_data: ldt,
        ..default()
    });
}

/// Run the 3D viewer on a specific canvas element (WASM)
///
/// # Arguments
/// * `canvas_selector` - CSS selector for the canvas element (e.g., "#bevy-canvas")
#[cfg(target_arch = "wasm32")]
pub fn run_on_canvas(canvas_selector: &str) {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Eulumdat 3D Viewer".to_string(),
                canvas: Some(canvas_selector.to_string()),
                fit_canvas_to_parent: true,
                prevent_default_event_handling: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugins((CameraPlugin, ScenePlugin, PhotometricLightPlugin))
        .insert_resource(SceneSettings::default())
        .insert_resource(LdtTimestamp::default())
        .add_systems(Startup, setup_with_default_ldt)
        .add_systems(Update, ui_controls_system)
        .add_systems(Update, poll_ldt_changes)
        .run();
}

/// Run the 3D viewer in a native window (desktop)
#[cfg(not(target_arch = "wasm32"))]
pub fn run_on_canvas(_canvas_selector: &str) {
    run_native();
}

/// Run the 3D viewer as a native window (desktop only)
#[cfg(not(target_arch = "wasm32"))]
pub fn run_native() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Eulumdat 3D Viewer".to_string(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((CameraPlugin, ScenePlugin, PhotometricLightPlugin))
        .insert_resource(SceneSettings::default())
        .insert_resource(LdtTimestamp::default())
        .add_systems(Startup, setup_with_default_ldt)
        .add_systems(Update, ui_controls_system)
        .add_systems(Update, poll_ldt_changes)
        .run();
}

#[cfg(target_arch = "wasm32")]
pub fn run_native() {
    // On WASM, run_native falls back to a default canvas
    run_on_canvas("#bevy-canvas");
}
