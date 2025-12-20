//! EulumdatViewerPlugin - Full demo application plugin.
//!
//! This plugin provides a complete 3D viewer with:
//! - Demo scenes (Room, Road, Parking, Outdoor)
//! - First-person camera
//! - Keyboard controls
//! - Optional localStorage sync for WASM

use super::camera::CameraPlugin;
use super::controls::{calculate_light_position, sync_viewer_to_lights, viewer_controls_system};
use super::scenes::ScenePlugin;
use super::wasm_sync::{load_default_ldt, LdtTimestamp, ViewerSettingsTimestamp};
use super::ViewerSettings;
use crate::eulumdat_impl::EulumdatLightBundle;
use crate::photometric::PhotometricPlugin;
use bevy::prelude::*;
use eulumdat::Eulumdat;

/// Full demo application plugin for the Eulumdat 3D viewer.
///
/// This plugin includes:
/// - [`PhotometricPlugin`] for photometric lighting
/// - [`CameraPlugin`] for first-person camera
/// - [`ScenePlugin`] for demo scene geometry
/// - Keyboard controls (P/L/H/1-4)
/// - Optional localStorage sync for WASM hot-reload
///
/// # Example
///
/// ```ignore
/// use bevy::prelude::*;
/// use eulumdat_bevy::viewer::*;
///
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .add_plugins(EulumdatViewerPlugin::default())
///         .run();
/// }
/// ```
pub struct EulumdatViewerPlugin {
    /// Initial LDT data to display (optional)
    pub initial_ldt: Option<Eulumdat>,
    /// Enable keyboard controls (P, L, H, 1-4 keys). Default: true
    pub enable_keyboard_controls: bool,
    /// Enable localStorage polling for hot-reload (WASM only, requires `wasm-sync` feature).
    /// Default: true when `wasm-sync` feature is enabled, false otherwise.
    pub enable_local_storage_sync: bool,
}

impl Default for EulumdatViewerPlugin {
    fn default() -> Self {
        Self {
            initial_ldt: None,
            enable_keyboard_controls: true,
            enable_local_storage_sync: cfg!(feature = "wasm-sync"),
        }
    }
}

impl EulumdatViewerPlugin {
    /// Create a new plugin with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a plugin with initial LDT data.
    pub fn with_ldt(ldt: Eulumdat) -> Self {
        Self {
            initial_ldt: Some(ldt),
            enable_keyboard_controls: true,
            enable_local_storage_sync: cfg!(feature = "wasm-sync"),
        }
    }
}

impl Plugin for EulumdatViewerPlugin {
    fn build(&self, app: &mut App) {
        // Add the generic photometric plugin for Eulumdat
        app.add_plugins(PhotometricPlugin::<Eulumdat>::new());

        // Add viewer-specific plugins
        app.add_plugins((CameraPlugin, ScenePlugin));

        // Insert viewer settings
        let settings = ViewerSettings {
            ldt_data: self.initial_ldt.clone(),
            ..default()
        };
        app.insert_resource(settings);
        app.insert_resource(LdtTimestamp::default());
        app.insert_resource(ViewerSettingsTimestamp::default());

        // Add startup system to spawn the light
        app.add_systems(Startup, setup_viewer_light);

        // Add keyboard controls if enabled
        if self.enable_keyboard_controls {
            app.add_systems(Update, viewer_controls_system);
        }

        // Add sync system to propagate settings to lights
        app.add_systems(Update, sync_viewer_to_lights);

        // Add localStorage polling if feature is enabled
        #[cfg(feature = "wasm-sync")]
        if self.enable_local_storage_sync {
            app.add_systems(Update, super::wasm_sync::poll_ldt_changes);
            app.add_systems(Update, super::wasm_sync::poll_viewer_settings_changes);
            app.add_systems(Update, sync_ldt_to_light);
        }
    }
}

/// Startup system to spawn the initial photometric light.
fn setup_viewer_light(mut commands: Commands, settings: Res<ViewerSettings>) {
    // Try to get LDT data from settings or load default
    let ldt = settings.ldt_data.clone().or_else(load_default_ldt);

    if let Some(ldt_data) = ldt {
        // Calculate light position based on scene type
        let position = calculate_light_position(&settings, &ldt_data);

        commands.spawn(
            EulumdatLightBundle::new(ldt_data)
                .with_transform(Transform::from_translation(position))
                .with_solid(settings.show_photometric_solid)
                .with_model(settings.show_luminaire)
                .with_shadows(settings.show_shadows),
        );
    }
}

/// System to sync LDT data changes to the light entity.
#[cfg(feature = "wasm-sync")]
fn sync_ldt_to_light(
    settings: Res<ViewerSettings>,
    mut lights: Query<(
        &mut crate::photometric::PhotometricLight<Eulumdat>,
        &mut Transform,
    )>,
) {
    if !settings.is_changed() {
        return;
    }

    if let Some(ref new_ldt) = settings.ldt_data {
        for (mut light, mut transform) in lights.iter_mut() {
            light.data = new_ldt.clone();
            // Update position based on new LDT dimensions
            transform.translation = calculate_light_position(&settings, new_ldt);
        }
    }
}
