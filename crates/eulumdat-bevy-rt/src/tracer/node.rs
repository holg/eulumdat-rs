//! Compute pipeline init and dispatch for camera-mode photometric tracing.

use super::extract::ExtractedPhotometricCameras;
use crate::scene::binder::PhotometricSceneBindings;
use bevy_asset::{load_embedded_asset, AssetServer};
use bevy_ecs::prelude::*;
use bevy_render::{
    camera::ExtractedCamera,
    render_resource::{
        binding_types::texture_storage_2d,
        BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
        BufferInitDescriptor, BufferUsages, CachedComputePipelineId, ComputePassDescriptor,
        ComputePipelineDescriptor, PipelineCache, ShaderStages, StorageTextureAccess,
        TextureFormat,
    },
    renderer::{RenderContext, RenderDevice},
    view::ViewTarget,
};
use bevy_utils::default;

/// Camera config matching the WGSL CameraConfig struct.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuCameraConfig {
    pub width: u32,
    pub height: u32,
    pub samples_per_pixel: u32,
    pub max_bounces: u32,
    pub cam_pos: [f32; 3],
    pub pad0: f32,
    pub cam_forward: [f32; 3],
    pub pad1: f32,
    pub cam_right: [f32; 3],
    pub pad2: f32,
    pub cam_up: [f32; 3],
    pub fov_tan: f32,
    pub num_primitives: u32,
    pub seed_offset: u32,
    pub source_intensity: f32,
    pub source_radius: f32,
    pub source_pos: [f32; 3],
    pub pad3: f32,
    pub lvk_c_steps: u32,
    pub lvk_g_steps: u32,
    pub lvk_g_max: f32,
    pub lvk_max_intensity: f32,
}

#[derive(Resource)]
pub struct TracerPipelines {
    pub bind_group_layout: BindGroupLayoutDescriptor,
    pub pipeline: CachedComputePipelineId,
}

/// Frame counter for seed variation.
#[derive(Resource, Default)]
pub struct TracerFrameCount(pub u32);

pub fn init_tracer_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    scene_bindings: Res<PhotometricSceneBindings>,
    asset_server: Res<AssetServer>,
) {
    use bevy_render::render_resource::{BindingType, BufferBindingType};

    let uniform = BindingType::Buffer {
        ty: BufferBindingType::Uniform,
        has_dynamic_offset: false,
        min_binding_size: None,
    };

    let bind_group_layout = BindGroupLayoutDescriptor::new(
        "photometric_tracer_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                // @binding(0) output texture
                texture_storage_2d(TextureFormat::Rgba16Float, StorageTextureAccess::WriteOnly),
                // @binding(1) camera uniforms
                uniform,
            ),
        ),
    );

    let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("photometric_tracer_pipeline".into()),
        layout: vec![
            scene_bindings.bind_group_layout.clone(),
            bind_group_layout.clone(),
        ],
        shader: load_embedded_asset!(asset_server.as_ref(), "trace_camera.wgsl"),
        ..default()
    });

    commands.insert_resource(TracerPipelines {
        bind_group_layout,
        pipeline,
    });
    commands.init_resource::<TracerFrameCount>();
}

/// Dispatch the photometric tracer compute pass.
pub fn photometric_tracer(
    extracted_cameras: Res<ExtractedPhotometricCameras>,
    tracer_pipelines: Option<Res<TracerPipelines>>,
    scene_bindings: Option<Res<PhotometricSceneBindings>>,
    pipeline_cache: Res<PipelineCache>,
    render_device: Res<RenderDevice>,
    mut ctx: RenderContext,
    cameras: Query<(&ExtractedCamera, &ViewTarget)>,
    mut frame_count: Option<ResMut<TracerFrameCount>>,
    extracted_scene: Res<crate::scene::extract::ExtractedPhotometricScene>,
) {
    if extracted_cameras.cameras.is_empty() {
        return;
    }
    let Some(tracer_pipelines) = tracer_pipelines else { return; };
    let Some(scene_bindings) = scene_bindings else { return; };
    let Some(scene_bind_group) = &scene_bindings.bind_group else { return; };
    let Some(pipeline) = pipeline_cache.get_compute_pipeline(tracer_pipelines.pipeline) else {
        return;
    };

    let Some((_camera, view_target)) = cameras.iter().next() else { return; };

    let output_view = view_target.get_unsampled_color_attachment().view;
    let size = view_target.main_texture().size();
    let width = size.width;
    let height = size.height;

    let fc = if let Some(ref mut fc) = frame_count {
        fc.0 += 1;
        fc.0
    } else {
        0
    };

    // Camera from extracted data
    let cam = &extracted_cameras.cameras[0];
    let fov_tan = (45.0_f32.to_radians() / 2.0).tan();

    // Get LVK data from first luminaire if available
    let profile = extracted_scene.luminaires.first()
        .and_then(|l| l.profile.as_ref());

    if fc == 1 {
        tracing::info!(
            "Tracer dispatch: {} prims, {} luminaires, profile={}",
            extracted_scene.primitives.len(),
            extracted_scene.luminaires.len(),
            profile.is_some(),
        );
        if let Some(p) = profile {
            tracing::info!(
                "LVK: {}x{} steps, g_max={}, max_intensity={}, data_len={}",
                p.cdf_c_steps, p.cdf_g_steps, p.cdf_g_max, p.lvk_max_intensity, p.lvk_data.len(),
            );
        }
    }

    let (lvk_c_steps, lvk_g_steps, lvk_g_max, lvk_max_intensity) = profile
        .map(|p| (p.cdf_c_steps, p.cdf_g_steps, p.cdf_g_max, p.lvk_max_intensity))
        .unwrap_or((0, 0, 0.0, 1.0));

    let camera_config = GpuCameraConfig {
        width,
        height,
        samples_per_pixel: cam.samples_per_pixel,
        max_bounces: cam.max_bounces,
        cam_pos: cam.cam_pos.to_array(),
        pad0: 0.0,
        cam_forward: cam.cam_forward.to_array(),
        pad1: 0.0,
        cam_right: cam.cam_right.to_array(),
        pad2: 0.0,
        cam_up: cam.cam_up.to_array(),
        fov_tan,
        num_primitives: extracted_scene.primitives.len() as u32,
        seed_offset: fc,
        source_intensity: 100.0,
        source_radius: 0.05,
        source_pos: [0.0, 6.0, 0.0], // bottom of luminaire housing
        pad3: 0.0,
        lvk_c_steps,
        lvk_g_steps,
        lvk_g_max,
        lvk_max_intensity,
    };

    let camera_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("photometric_camera_config"),
        contents: bytemuck::bytes_of(&camera_config),
        usage: BufferUsages::UNIFORM,
    });

    let bind_group = render_device.create_bind_group(
        "photometric_tracer_bind_group",
        &pipeline_cache.get_bind_group_layout(&tracer_pipelines.bind_group_layout),
        &BindGroupEntries::sequential((
            output_view,
            camera_buffer.as_entire_binding(),
        )),
    );

    let command_encoder = ctx.command_encoder();
    let mut pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
        label: Some("photometric_tracer"),
        timestamp_writes: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, scene_bind_group, &[]);
    pass.set_bind_group(1, &bind_group, &[]);
    pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
}
