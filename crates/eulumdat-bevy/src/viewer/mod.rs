//! Viewer module - demo application with scenes, camera, and controls.
//!
//! This module provides a complete 3D viewer application built on top
//! of the generic [`photometric`](crate::photometric) module.
//!
//! # Features
//!
//! - Pre-built demo scenes (Room, Road, Parking, Outdoor)
//! - First-person camera controller
//! - Keyboard controls for toggling visualizations
//! - Optional localStorage sync for WASM hot-reload
//!
//! # Example
//!
//! ```ignore
//! use bevy::prelude::*;
//! use eulumdat_bevy::viewer::*;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(EulumdatViewerPlugin::default())
//!         .run();
//! }
//! ```

pub mod camera;
pub mod controls;
pub mod plugin;
pub mod scenes;
pub mod wasm_sync;

pub use camera::{CameraPlugin, FirstPersonCamera};
pub use controls::calculate_light_position;
pub use plugin::EulumdatViewerPlugin;
pub use scenes::{SceneGeometry, ScenePlugin, SceneType};
pub use wasm_sync::{
    load_default_ldt, load_from_local_storage, poll_viewer_settings_changes, LdtTimestamp,
    ViewerSettingsTimestamp,
};

use bevy::prelude::*;
use eulumdat::Eulumdat;

/// Global viewer settings resource.
///
/// This resource controls the viewer's behavior and appearance.
/// Changes to this resource trigger reactive updates to the scene.
#[derive(Resource, Clone)]
pub struct ViewerSettings {
    /// Current scene type
    pub scene_type: SceneType,
    /// Room/scene width in meters (X axis)
    pub room_width: f32,
    /// Room/scene length in meters (Z axis)
    pub room_length: f32,
    /// Room height in meters (Y axis, only for Room scene)
    pub room_height: f32,
    /// Luminaire mounting height in meters (for outdoor poles)
    /// For indoor scenes, this is ignored - use pendulum_length instead
    pub mounting_height: f32,
    /// Pendulum/suspension length in meters (for indoor ceiling-mounted luminaires)
    /// 0.0 = flush mounted to ceiling
    /// >0.0 = hangs down from ceiling by this amount
    pub pendulum_length: f32,
    /// Light intensity (not used directly, available for UI)
    pub light_intensity: f32,
    /// Whether to show the luminaire model
    pub show_luminaire: bool,
    /// Whether to show the photometric solid
    pub show_photometric_solid: bool,
    /// Whether to enable shadows
    pub show_shadows: bool,
    /// The LDT data to display
    pub ldt_data: Option<Eulumdat>,
}

impl Default for ViewerSettings {
    fn default() -> Self {
        Self {
            scene_type: SceneType::Room,
            room_width: 4.0,
            room_length: 5.0,
            room_height: 2.8,
            mounting_height: 8.0, // For outdoor poles
            pendulum_length: 0.3, // 30cm pendulum for indoor
            light_intensity: 1000.0,
            show_luminaire: true,
            show_photometric_solid: false,
            show_shadows: false,
            ldt_data: None,
        }
    }
}

impl ViewerSettings {
    /// Calculate the effective luminaire center height for the current scene.
    ///
    /// For Room scenes:
    /// - Luminaire hangs from ceiling by pendulum_length
    /// - Center Y = room_height - pendulum_length - half_luminaire_height
    ///
    /// For outdoor scenes (Road, Parking, Outdoor):
    /// - Luminaire is fixed to pole arm at mounting_height
    /// - Center Y = mounting_height - arm_offset - half_luminaire_height
    pub fn luminaire_height(&self, ldt: &Eulumdat) -> f32 {
        let lum_height = (ldt.height / 1000.0).max(0.05) as f32;

        match self.scene_type {
            SceneType::Room => {
                // Ceiling mounted with pendulum
                self.room_height - self.pendulum_length - lum_height / 2.0
            }
            SceneType::Road | SceneType::Parking | SceneType::Outdoor => {
                // Pole mounted - luminaire fixed to arm
                // Arm is at mounting_height - 0.25, luminaire hangs 0.05m below arm
                let arm_bottom = self.mounting_height - 0.25;
                arm_bottom - 0.05 - lum_height / 2.0
            }
        }
    }

    /// Get the attachment point height (where pendulum/cable starts).
    /// Only meaningful for Room scene.
    pub fn attachment_height(&self) -> f32 {
        match self.scene_type {
            SceneType::Room => self.room_height,
            _ => self.mounting_height,
        }
    }
}
