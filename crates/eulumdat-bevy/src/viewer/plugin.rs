//! EulumdatViewerPlugin - Full demo application plugin.
//!
//! This plugin provides a complete 3D viewer with:
//! - Demo scenes (Room, Road, Parking, Outdoor)
//! - First-person camera
//! - Keyboard controls
//! - Optional localStorage sync for WASM

use super::camera::CameraPlugin;
use super::controls::{
    calculate_all_luminaire_transforms, sync_viewer_to_lights, viewer_controls_system,
};
use super::scenes::ScenePlugin;
use super::wasm_sync::{load_default_ldt, DesignerTimestamp, LdtTimestamp, ViewerSettingsTimestamp};
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
        app.insert_resource(DesignerTimestamp::default());

        // Add startup system to spawn the light
        app.add_systems(Startup, setup_viewer_light);

        // Add keyboard controls if enabled
        if self.enable_keyboard_controls {
            app.add_systems(Update, viewer_controls_system);
        }

        // Add localStorage polling if feature is enabled
        // sync_ldt_to_light runs first and its commands are applied before
        // sync_viewer_to_lights, preventing stale entity references.
        #[cfg(feature = "wasm-sync")]
        if self.enable_local_storage_sync {
            app.add_systems(
                Update,
                (
                    super::wasm_sync::poll_ldt_changes,
                    super::wasm_sync::poll_viewer_settings_changes,
                    super::wasm_sync::poll_designer_changes,
                    sync_ldt_to_light,
                    ApplyDeferred,
                    sync_viewer_to_lights,
                )
                    .chain(),
            );
        }

        // Add sync system for non-wasm builds (no ordering conflict)
        #[cfg(not(feature = "wasm-sync"))]
        app.add_systems(Update, sync_viewer_to_lights);

        // Add egui settings panel for native builds only (not WASM - causes font init panic)
        #[cfg(all(feature = "egui-ui", not(target_arch = "wasm32")))]
        {
            app.add_plugins(super::egui_panel::EguiSettingsPlugin);
        }
    }
}

/// Startup system to spawn the initial photometric lights.
fn setup_viewer_light(mut commands: Commands, settings: Res<ViewerSettings>) {
    // Try to get LDT data from settings or load default
    let ldt = settings.ldt_data.clone().or_else(load_default_ldt);

    if let Some(ldt_data) = ldt {
        // Calculate all luminaire positions and rotations
        let transforms = calculate_all_luminaire_transforms(&settings, &ldt_data);

        for transform in transforms {
            commands.spawn(
                EulumdatLightBundle::new(ldt_data.clone())
                    .with_transform(
                        Transform::from_translation(transform.position)
                            .with_rotation(transform.rotation),
                    )
                    .with_solid(settings.show_photometric_solid)
                    .with_model(settings.show_luminaire)
                    .with_shadows(settings.show_shadows),
            );
        }
    }
}

/// System to sync LDT data changes to the light entities.
/// Despawns existing lights and respawns with new configuration.
#[cfg(feature = "wasm-sync")]
fn sync_ldt_to_light(
    mut commands: Commands,
    settings: Res<ViewerSettings>,
    lights: Query<Entity, With<crate::photometric::PhotometricLight<Eulumdat>>>,
) {
    if !settings.is_changed() {
        return;
    }

    if let Some(ref new_ldt) = settings.ldt_data {
        // Despawn all existing lights
        for entity in lights.iter() {
            commands.entity(entity).despawn();
        }

        // Spawn new lights with updated configuration
        let transforms = calculate_all_luminaire_transforms(&settings, new_ldt);

        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&format!("[Bevy] Spawning {} luminaires", transforms.len()).into());

        for transform in transforms {
            commands.spawn(
                EulumdatLightBundle::new(new_ldt.clone())
                    .with_transform(
                        Transform::from_translation(transform.position)
                            .with_rotation(transform.rotation),
                    )
                    .with_solid(settings.show_photometric_solid)
                    .with_model(settings.show_luminaire)
                    .with_shadows(settings.show_shadows),
            );
        }
    }
}
