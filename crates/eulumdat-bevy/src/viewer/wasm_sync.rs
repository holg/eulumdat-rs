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
#[cfg(all(target_arch = "wasm32", feature = "wasm-sync"))]
const DESIGNER_EXTERIOR_KEY: &str = "eulumdat_designer_exterior";
#[cfg(all(target_arch = "wasm32", feature = "wasm-sync"))]
const DESIGNER_INTERIOR_KEY: &str = "eulumdat_designer_interior";
#[cfg(all(target_arch = "wasm32", feature = "wasm-sync"))]
const DESIGNER_TIMESTAMP_KEY: &str = "eulumdat_designer_timestamp";

/// Resource to track localStorage timestamp for hot-reload.
#[derive(Resource, Default)]
pub struct LdtTimestamp(pub String);

/// Resource to track ViewerSettings timestamp for sync.
#[derive(Resource, Default)]
pub struct ViewerSettingsTimestamp(pub String);

/// Resource to track designer data timestamp for sync.
#[derive(Resource, Default)]
pub struct DesignerTimestamp(pub String);

/// Load LDT from localStorage (WASM only).
#[cfg(all(target_arch = "wasm32", feature = "wasm-sync"))]
pub fn load_from_local_storage() -> Option<Eulumdat> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    let ldt_string = storage.get_item(LDT_STORAGE_KEY).ok()??;

    web_sys::console::log_1(
        &format!(
            "[Bevy] Loading LDT from localStorage, {} bytes",
            ldt_string.len()
        )
        .into(),
    );

    match Eulumdat::parse(&ldt_string) {
        Ok(ldt) => {
            web_sys::console::log_1(
                &format!(
                    "[Bevy] Parsed LDT: {} lumens, {} cd/klm max",
                    ldt.total_luminous_flux(),
                    ldt.max_intensity()
                )
                .into(),
            );
            Some(ldt)
        }
        Err(e) => {
            web_sys::console::error_1(&format!("[Bevy] Failed to parse LDT: {:?}", e).into());
            None
        }
    }
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
                web_sys::console::log_1(
                    &format!(
                        "[Bevy] LDT timestamp changed: {} -> {}",
                        last_timestamp.0, new_timestamp
                    )
                    .into(),
                );
                if let Some(ldt) = load_from_local_storage() {
                    web_sys::console::log_1(
                        &format!("[Bevy] Updating ViewerSettings with new LDT").into(),
                    );
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
        4 => SceneType::DesignerExterior,
        5 => SceneType::DesignerInterior,
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
        luminaire_tilt: get_f32("luminaire_tilt").unwrap_or(current.luminaire_tilt),
        lane_width: get_f32("lane_width").unwrap_or(current.lane_width),
        num_lanes: get_u8("num_lanes").unwrap_or(current.num_lanes as u8) as u32,
        sidewalk_width: get_f32("sidewalk_width").unwrap_or(current.sidewalk_width),
        pole_spacing: get_f32("pole_spacing").unwrap_or(current.pole_spacing),
        // Preserve designer data - synced separately via designer keys
        area_result: current.area_result.clone(),
        area_placements: current.area_placements.clone(),
        designer_room: current.designer_room.clone(),
        designer_layout: current.designer_layout.clone(),
        designer_reflectances: current.designer_reflectances.clone(),
        designer_cavity: current.designer_cavity.clone(),
        designer_ppb: current.designer_ppb.clone(),
        show_light_cones: get_bool("show_light_cones").unwrap_or(current.show_light_cones),
        show_cavities: get_bool("show_cavities").unwrap_or(current.show_cavities),
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

/// Get designer data timestamp from localStorage.
#[cfg(all(target_arch = "wasm32", feature = "wasm-sync"))]
pub fn get_designer_timestamp() -> Option<String> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    storage.get_item(DESIGNER_TIMESTAMP_KEY).ok()?
}

#[cfg(not(all(target_arch = "wasm32", feature = "wasm-sync")))]
pub fn get_designer_timestamp() -> Option<String> {
    None
}

/// Poll localStorage for designer data changes (exterior + interior).
#[cfg(feature = "wasm-sync")]
#[allow(unused_mut, unused_variables)]
pub fn poll_designer_changes(
    mut settings: ResMut<ViewerSettings>,
    mut last_timestamp: ResMut<DesignerTimestamp>,
) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(new_timestamp) = get_designer_timestamp() {
            if new_timestamp != last_timestamp.0 {
                let window = web_sys::window();
                let storage = window
                    .as_ref()
                    .and_then(|w| w.local_storage().ok().flatten());

                if let Some(storage) = storage {
                    // Try exterior designer data
                    if let Ok(Some(json)) = storage.get_item(DESIGNER_EXTERIOR_KEY) {
                        if let Ok(data) = serde_json::from_str::<DesignerExteriorData>(&json) {
                            settings.area_result = Some(data.area_result);
                            settings.area_placements = data.placements;
                            web_sys::console::log_1(
                                &format!(
                                    "[Bevy] Loaded exterior designer: {} placements",
                                    settings.area_placements.len()
                                )
                                .into(),
                            );
                        }
                    }

                    // Try interior designer data
                    if let Ok(Some(json)) = storage.get_item(DESIGNER_INTERIOR_KEY) {
                        if let Ok(data) = serde_json::from_str::<DesignerInteriorData>(&json) {
                            settings.designer_room = Some(data.room);
                            settings.designer_layout = Some(data.layout);
                            settings.designer_reflectances = Some(data.reflectances);
                            settings.designer_cavity = Some(data.cavity);
                            settings.designer_ppb = data.ppb;
                            web_sys::console::log_1(&"[Bevy] Loaded interior designer data".into());
                        }
                    }
                }

                last_timestamp.0 = new_timestamp;
            }
        }
    }
}

// Stub for when wasm-sync is disabled
#[cfg(not(feature = "wasm-sync"))]
#[allow(unused_mut, unused_variables, dead_code)]
pub fn poll_designer_changes(
    mut settings: ResMut<ViewerSettings>,
    mut last_timestamp: ResMut<DesignerTimestamp>,
) {
    // No-op when wasm-sync feature is disabled
}

/// Serializable container for exterior designer data.
#[cfg(all(target_arch = "wasm32", feature = "wasm-sync"))]
#[derive(serde::Deserialize)]
struct DesignerExteriorData {
    area_result: eulumdat::area::AreaResult,
    placements: Vec<eulumdat::area::LuminairePlace>,
}

/// Serializable container for interior designer data.
#[cfg(all(target_arch = "wasm32", feature = "wasm-sync"))]
#[derive(serde::Deserialize)]
struct DesignerInteriorData {
    room: eulumdat::zonal::Room,
    layout: eulumdat::zonal::LuminaireLayout,
    reflectances: eulumdat::zonal::Reflectances,
    cavity: eulumdat::zonal::CavityResults,
    ppb: Option<eulumdat::zonal::PpbResult>,
}
