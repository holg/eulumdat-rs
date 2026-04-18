//! Extract camera settings from the main world.

use super::PhotometricCamera;
use bevy_ecs::prelude::*;
use bevy_render::Extract;
use bevy_transform::components::GlobalTransform;

/// Extracted camera state for the render world.
#[derive(Component, Clone)]
pub struct ExtractedPhotometricCamera {
    pub samples_per_pixel: u32,
    pub max_bounces: u32,
    pub reset: bool,
    pub cam_pos: bevy_math::Vec3,
    pub cam_forward: bevy_math::Vec3,
    pub cam_right: bevy_math::Vec3,
    pub cam_up: bevy_math::Vec3,
}

/// Resource tracking extracted cameras.
#[derive(Resource, Default)]
pub struct ExtractedPhotometricCameras {
    pub cameras: Vec<ExtractedPhotometricCamera>,
}

pub fn extract_photometric_camera(
    mut extracted: ResMut<ExtractedPhotometricCameras>,
    cameras: Extract<Query<(&PhotometricCamera, &GlobalTransform)>>,
) {
    extracted.cameras.clear();
    for (cam, transform) in cameras.iter() {
        let (_, rotation, translation) = transform.to_scale_rotation_translation();
        let forward = rotation * bevy_math::Vec3::NEG_Z;
        let right = rotation * bevy_math::Vec3::X;
        let up = rotation * bevy_math::Vec3::Y;
        extracted.cameras.push(ExtractedPhotometricCamera {
            samples_per_pixel: cam.samples_per_pixel.max(1),
            max_bounces: cam.max_bounces.max(1),
            reset: cam.reset,
            cam_pos: translation,
            cam_forward: forward,
            cam_right: right,
            cam_up: up,
        });
    }
}
