//! PhotometricPlugin for Bevy.
//!
//! This plugin provides minimal photometric lighting support without
//! any scene geometry, camera, or controls.

use super::light::PhotometricPluginState;
use super::systems::{
    cleanup_photometric_lights, spawn_photometric_lights, update_photometric_lights,
};
use super::PhotometricData;
use bevy::prelude::*;
use std::marker::PhantomData;

/// Minimal plugin for photometric lighting.
///
/// This plugin handles:
/// - Spawning Bevy lights from `PhotometricLight` components
/// - Updating lights when components change
/// - Managing photometric solid and luminaire model entities
///
/// It does NOT provide:
/// - Scene geometry (bring your own scene)
/// - Camera (bring your own camera)
/// - Keyboard controls (implement your own if needed)
///
/// # Type Parameters
/// * `T` - The photometric data type (must implement [`PhotometricData`])
///
/// # Example
/// ```ignore
/// use bevy::prelude::*;
/// use eulumdat_bevy::photometric::*;
///
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .add_plugins(PhotometricPlugin::<MyLightData>::default())
///         .add_systems(Startup, setup)
///         .run();
/// }
///
/// fn setup(mut commands: Commands) {
///     // Spawn a camera
///     commands.spawn(Camera3dBundle {
///         transform: Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
///         ..default()
///     });
///
///     // Spawn your scene geometry
///     commands.spawn(PbrBundle {
///         mesh: meshes.add(Plane3d::default().mesh().size(10.0, 10.0)),
///         material: materials.add(Color::WHITE),
///         ..default()
///     });
///
///     // Spawn a photometric light
///     let light_data = MyLightData::load("light.ldt");
///     commands.spawn(PhotometricLightBundle::new(light_data)
///         .with_transform(Transform::from_xyz(0.0, 3.0, 0.0)));
/// }
/// ```
#[derive(Default)]
pub struct PhotometricPlugin<T: PhotometricData> {
    _phantom: PhantomData<T>,
}

impl<T: PhotometricData> PhotometricPlugin<T> {
    /// Create a new PhotometricPlugin.
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<T: PhotometricData> Plugin for PhotometricPlugin<T> {
    fn build(&self, app: &mut App) {
        app.init_resource::<PhotometricPluginState<T>>()
            .add_systems(
                Update,
                (
                    spawn_photometric_lights::<T>,
                    update_photometric_lights::<T>,
                    cleanup_photometric_lights::<T>,
                ),
            );
    }
}
