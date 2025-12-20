//! WASM localStorage synchronization for hot-reload.
//!
//! This module enables real-time sync between the web editor and the 3D viewer.
//! When the editor saves LDT data to localStorage, the viewer picks it up.
//! ViewerSettings can also be synced via localStorage for UI controls.

#[cfg(all(target_arch = "wasm32", feature = "wasm-sync"))]
use super::SceneType;
use super::ViewerSettings;
use bevy::prelude::*;
use eulumdat::Eulumdat;

#[cfg(all(target_arch = "wasm32", feature = "wasm-sync"))]
const LDT_STORAGE_KEY: &str = "eulumdat_current_ldt";
#[cfg(all(target_arch = "wasm32", feature = "wasm-sync"))]
const LDT_TIMESTAMP_KEY: &str = "eulumdat_ldt_timestamp";
#[cfg(all(target_arch = "wasm32", feature = "wasm-sync"))]
const VIEWER_SETTINGS_KEY: &str = "eulumdat_viewer_settings";
#[cfg(all(target_arch = "wasm32", feature = "wasm-sync"))]
const VIEWER_SETTINGS_TIMESTAMP_KEY: &str = "eulumdat_viewer_settings_timestamp";

/// Resource to track localStorage timestamp for hot-reload.
#[derive(Resource, Default)]
pub struct LdtTimestamp(pub String);

/// Resource to track ViewerSettings timestamp for sync.
#[derive(Resource, Default)]
pub struct ViewerSettingsTimestamp(pub String);

/// Load LDT from localStorage (WASM only).
#[cfg(all(target_arch = "wasm32", feature = "wasm-sync"))]
pub fn load_from_local_storage() -> Option<Eulumdat> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    let ldt_string = storage.get_item(LDT_STORAGE_KEY).ok()??;

    Eulumdat::parse(&ldt_string).ok()
}

#[cfg(not(all(target_arch = "wasm32", feature = "wasm-sync")))]
pub fn load_from_local_storage() -> Option<Eulumdat> {
    None
}

/// Get timestamp from localStorage.
#[cfg(all(target_arch = "wasm32", feature = "wasm-sync"))]
pub fn get_ldt_timestamp() -> Option<String> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    storage.get_item(LDT_TIMESTAMP_KEY).ok()?
}

#[cfg(not(all(target_arch = "wasm32", feature = "wasm-sync")))]
pub fn get_ldt_timestamp() -> Option<String> {
    None
}

/// Load default LDT data.
///
/// For WASM with wasm-sync feature: Loads from localStorage.
/// For native: Tries common file paths.
pub fn load_default_ldt() -> Option<Eulumdat> {
    // For WASM with wasm-sync feature, load from localStorage
    #[cfg(all(target_arch = "wasm32", feature = "wasm-sync"))]
    {
        load_from_local_storage()
    }

    // For WASM without wasm-sync, return None (data must be provided by parent app)
    #[cfg(all(target_arch = "wasm32", not(feature = "wasm-sync")))]
    {
        None
    }

    // For native, try to load from common file paths
    #[cfg(not(target_arch = "wasm32"))]
    {
        let sample_paths = [
            "crates/eulumdat-wasm/templates/road_luminaire.ldt",
            "../eulumdat-wasm/templates/road_luminaire.ldt",
            "crates/eulumdat-wasm/templates/fluorescent_luminaire.ldt",
            "../eulumdat-wasm/templates/fluorescent_luminaire.ldt",
            "templates/road_luminaire.ldt",
            "sample.ldt",
        ];

        for path in sample_paths {
            if let Ok(ldt) = Eulumdat::from_file(path) {
                return Some(ldt);
            }
        }
        None
    }
}

/// Poll localStorage for LDT changes.
#[cfg(feature = "wasm-sync")]
#[allow(unused_mut, unused_variables)]
pub fn poll_ldt_changes(
    mut settings: ResMut<ViewerSettings>,
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

// Stub for when wasm-sync is disabled
#[cfg(not(feature = "wasm-sync"))]
#[allow(unused_mut, unused_variables, dead_code)]
pub fn poll_ldt_changes(
    mut settings: ResMut<ViewerSettings>,
    mut last_timestamp: ResMut<LdtTimestamp>,
) {
    // No-op when wasm-sync feature is disabled
}

/// Get ViewerSettings timestamp from localStorage.
#[cfg(all(target_arch = "wasm32", feature = "wasm-sync"))]
pub fn get_viewer_settings_timestamp() -> Option<String> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    storage.get_item(VIEWER_SETTINGS_TIMESTAMP_KEY).ok()?
}

#[cfg(not(all(target_arch = "wasm32", feature = "wasm-sync")))]
pub fn get_viewer_settings_timestamp() -> Option<String> {
    None
}

/// Load ViewerSettings from localStorage JSON.
#[cfg(all(target_arch = "wasm32", feature = "wasm-sync"))]
pub fn load_viewer_settings_from_local_storage(current: &ViewerSettings) -> Option<ViewerSettings> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    let json_string = storage.get_item(VIEWER_SETTINGS_KEY).ok()??;

    parse_viewer_settings_json(&json_string, current)
}

#[cfg(not(all(target_arch = "wasm32", feature = "wasm-sync")))]
pub fn load_viewer_settings_from_local_storage(
    _current: &ViewerSettings,
) -> Option<ViewerSettings> {
    None
}

/// Parse ViewerSettings from JSON string.
/// Preserves ldt_data from current settings since it's synced separately.
#[cfg(all(target_arch = "wasm32", feature = "wasm-sync"))]
fn parse_viewer_settings_json(json: &str, current: &ViewerSettings) -> Option<ViewerSettings> {
    // Simple JSON parsing without serde dependency
    // Format: {"scene_type":0,"room_width":4.0,"room_length":5.0,...}

    let get_f32 = |key: &str| -> Option<f32> {
        let pattern = format!("\"{}\":", key);
        let start = json.find(&pattern)? + pattern.len();
        let rest = &json[start..];
        let end = rest.find([',', '}'])?;
        rest[..end].trim().parse().ok()
    };

    let get_bool = |key: &str| -> Option<bool> {
        let pattern = format!("\"{}\":", key);
        let start = json.find(&pattern)? + pattern.len();
        let rest = &json[start..];
        let end = rest.find([',', '}'])?;
        let value = rest[..end].trim();
        Some(value == "true")
    };

    let get_u8 = |key: &str| -> Option<u8> {
        let pattern = format!("\"{}\":", key);
        let start = json.find(&pattern)? + pattern.len();
        let rest = &json[start..];
        let end = rest.find([',', '}'])?;
        rest[..end].trim().parse().ok()
    };

    let scene_type = match get_u8("scene_type")? {
        0 => SceneType::Room,
        1 => SceneType::Road,
        2 => SceneType::Parking,
        3 => SceneType::Outdoor,
        _ => SceneType::Room,
    };

    Some(ViewerSettings {
        scene_type,
        room_width: get_f32("room_width").unwrap_or(current.room_width),
        room_length: get_f32("room_length").unwrap_or(current.room_length),
        room_height: get_f32("room_height").unwrap_or(current.room_height),
        mounting_height: get_f32("mounting_height").unwrap_or(current.mounting_height),
        pendulum_length: get_f32("pendulum_length").unwrap_or(current.pendulum_length),
        light_intensity: get_f32("light_intensity").unwrap_or(current.light_intensity),
        show_luminaire: get_bool("show_luminaire").unwrap_or(current.show_luminaire),
        show_photometric_solid: get_bool("show_photometric_solid")
            .unwrap_or(current.show_photometric_solid),
        show_shadows: get_bool("show_shadows").unwrap_or(current.show_shadows),
        // Preserve LDT data - it's synced separately
        ldt_data: current.ldt_data.clone(),
    })
}

/// Poll localStorage for ViewerSettings changes.
#[cfg(feature = "wasm-sync")]
#[allow(unused_mut, unused_variables)]
pub fn poll_viewer_settings_changes(
    mut settings: ResMut<ViewerSettings>,
    mut last_timestamp: ResMut<ViewerSettingsTimestamp>,
) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(new_timestamp) = get_viewer_settings_timestamp() {
            if new_timestamp != last_timestamp.0 {
                // Timestamp changed - reload settings
                if let Some(new_settings) = load_viewer_settings_from_local_storage(&settings) {
                    *settings = new_settings;
                    last_timestamp.0 = new_timestamp;
                }
            }
        }
    }
}

// Stub for when wasm-sync is disabled
#[cfg(not(feature = "wasm-sync"))]
#[allow(unused_mut, unused_variables, dead_code)]
pub fn poll_viewer_settings_changes(
    mut settings: ResMut<ViewerSettings>,
    mut last_timestamp: ResMut<ViewerSettingsTimestamp>,
) {
    // No-op when wasm-sync feature is disabled
}
