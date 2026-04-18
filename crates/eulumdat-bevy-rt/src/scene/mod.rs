//! Photometric scene plugin — manages scene data, BVH, and bind group 0.

pub mod binder;
pub mod bvh;
pub mod extract;
pub mod types;

use bevy_app::{App, Plugin};
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_render::{ExtractSchedule, Render, RenderApp, RenderStartup, RenderSystems};
use bevy_shader::load_shader_library;
use binder::{init_scene_bindings, prepare_scene_bindings};
use extract::{extract_photometric_scene, ExtractedPhotometricScene};

pub struct PhotometricScenePlugin;

impl Plugin for PhotometricScenePlugin {
    fn build(&self, app: &mut App) {
        // Register composable shader libraries
        load_shader_library!(app, "common.wgsl");
        load_shader_library!(app, "intersect.wgsl");
        load_shader_library!(app, "bvh.wgsl");
        load_shader_library!(app, "material.wgsl");
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);

        render_app
            .init_resource::<ExtractedPhotometricScene>()
            .add_systems(RenderStartup, init_scene_bindings)
            .add_systems(ExtractSchedule, extract_photometric_scene)
            .add_systems(
                Render,
                prepare_scene_bindings.in_set(RenderSystems::PrepareBindGroups),
            );
    }
}
