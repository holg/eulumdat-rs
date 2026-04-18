//! Photometric tracer plugin — camera rendering via compute shaders.

mod extract;
mod node;

use bevy_app::{App, Plugin};
use bevy_asset::embedded_asset;
use bevy_core_pipeline::schedule::{Core3d, Core3dSystems};
use bevy_core_pipeline::tonemapping::tonemapping;
use bevy_ecs::{component::Component, schedule::IntoScheduleConfigs};
use bevy_render::{ExtractSchedule, RenderApp, RenderStartup};
use crate::scene::binder::init_scene_bindings;
use extract::{extract_photometric_camera, ExtractedPhotometricCameras};
use node::{init_tracer_pipelines, photometric_tracer};

pub struct PhotometricTracerPlugin;

impl Plugin for PhotometricTracerPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "trace_camera.wgsl");
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);

        render_app
            .init_resource::<ExtractedPhotometricCameras>()
            .add_systems(RenderStartup, init_tracer_pipelines.after(init_scene_bindings))
            .add_systems(ExtractSchedule, extract_photometric_camera)
            .add_systems(
                Core3d,
                photometric_tracer
                    .after(Core3dSystems::MainPass)
                    .before(tonemapping),
            );
    }
}

/// Attach to a camera to enable photometric raytracing.
#[derive(Component, Default, Clone)]
pub struct PhotometricCamera {
    pub samples_per_pixel: u32,
    pub max_bounces: u32,
    pub reset: bool,
}
