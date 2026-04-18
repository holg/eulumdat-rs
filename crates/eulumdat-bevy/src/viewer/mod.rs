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
pub mod designer_scenes;
#[cfg(feature = "egui-ui")]
pub mod egui_panel;
pub mod plugin;
pub mod scenes;
pub mod wasm_sync;

pub use camera::{CameraPlugin, FirstPersonCamera};
pub use controls::{
    calculate_all_luminaire_transforms, calculate_light_position, LuminaireTransform,
};
pub use plugin::EulumdatViewerPlugin;
pub use scenes::{SceneGeometry, ScenePlugin, SceneType};
pub use wasm_sync::{
    load_default_ldt, load_from_local_storage, poll_viewer_settings_changes, DesignerTimestamp,
    LdtTimestamp, ViewerSettingsTimestamp,
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
    /// Luminaire tilt angle in degrees (for road/outdoor scenes).
    /// 0 = pointing straight down, 90 = pointing horizontally across the road.
    /// Default is 15 degrees for road luminaires.
    pub luminaire_tilt: f32,
    /// Lane width in meters (for road scenes). Default 3.5m per EN 13201.
    pub lane_width: f32,
    /// Number of lanes (for road scenes). Default 2 (one per direction).
    pub num_lanes: u32,
    /// Sidewalk width in meters. Default 2.0m.
    pub sidewalk_width: f32,
    /// Pole spacing in meters. Calculated based on mounting height if 0.
    /// Typical: 3-4x mounting height for good uniformity.
    pub pole_spacing: f32,
    // --- Designer scene fields ---
    /// Exterior designer: area computation result (heatmap, stats)
    pub area_result: Option<eulumdat::area::AreaResult>,
    /// Exterior designer: luminaire placements
    pub area_placements: Vec<eulumdat::area::LuminairePlace>,
    /// Interior designer: room parameters
    pub designer_room: Option<eulumdat::zonal::Room>,
    /// Interior designer: luminaire layout grid
    pub designer_layout: Option<eulumdat::zonal::LuminaireLayout>,
    /// Interior designer: surface reflectances
    pub designer_reflectances: Option<eulumdat::zonal::Reflectances>,
    /// Interior designer: cavity ratio results
    pub designer_cavity: Option<eulumdat::zonal::CavityResults>,
    /// Interior designer: point-by-point illuminance result
    pub designer_ppb: Option<eulumdat::zonal::PpbResult>,
    /// Toggle light cone visualization
    pub show_light_cones: bool,
    /// Toggle cavity zone overlays (interior scene)
    pub show_cavities: bool,
}

impl Default for ViewerSettings {
    fn default() -> Self {
        Self {
            scene_type: SceneType::Room,
            room_width: 4.0,
            room_length: 5.0,
            room_height: 2.8,
            mounting_height: 8.0, // For outdoor poles (EN 13201: 6-12m typical)
            pendulum_length: 0.3, // 30cm pendulum for indoor
            light_intensity: 1000.0,
            show_luminaire: true,
            show_photometric_solid: false,
            show_shadows: false,
            ldt_data: None,
            luminaire_tilt: 15.0, // 15 degrees tilt for road luminaires (typical)
            lane_width: 3.5,      // EN 13201 standard lane width
            num_lanes: 2,         // Two lanes (one per direction)
            sidewalk_width: 2.0,  // Standard sidewalk
            pole_spacing: 0.0,    // 0 = auto-calculate (3.5x mounting height)
            area_result: None,
            area_placements: Vec::new(),
            designer_room: None,
            designer_layout: None,
            designer_reflectances: None,
            designer_cavity: None,
            designer_ppb: None,
            show_light_cones: true,
            show_cavities: false,
        }
    }
}

impl ViewerSettings {
    /// Calculate effective pole spacing.
    /// If pole_spacing is 0, use 3.5x mounting height (good uniformity).
    pub fn effective_pole_spacing(&self) -> f32 {
        if self.pole_spacing > 0.0 {
            self.pole_spacing
        } else {
            // EN 13201 recommends spacing of 3-4x mounting height
            self.mounting_height * 3.5
        }
    }

    /// Calculate total road width including sidewalks.
    pub fn total_road_width(&self) -> f32 {
        self.num_lanes as f32 * self.lane_width + 2.0 * self.sidewalk_width
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
            SceneType::Room | SceneType::DesignerInterior => {
                // Ceiling mounted with pendulum
                self.room_height - self.pendulum_length - lum_height / 2.0
            }
            SceneType::Road | SceneType::Parking | SceneType::Outdoor
            | SceneType::DesignerExterior => {
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
            SceneType::Room | SceneType::DesignerInterior => self.room_height,
            _ => self.mounting_height,
        }
    }
}
