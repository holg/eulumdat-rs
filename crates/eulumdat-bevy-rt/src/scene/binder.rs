//! Scene bind group management — prepares GPU buffers and creates bind group 0.

use super::{bvh::build_bvh, extract::ExtractedPhotometricScene, types::BvhNode};
use bevy_ecs::prelude::*;
use bevy_render::{
    render_resource::{
        BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries,
        BindGroupLayoutDescriptor, BindingType, BufferBindingType, BufferInitDescriptor,
        BufferUsages, PipelineCache, ShaderStages,
    },
    renderer::RenderDevice,
};
use eulumdat_rt::GpuTracerConfig;

/// Resource holding the scene bind group and layout for bind group 0.
#[derive(Resource)]
pub struct PhotometricSceneBindings {
    pub bind_group_layout: BindGroupLayoutDescriptor,
    pub bind_group: Option<BindGroup>,
}

/// Initialize scene bindings layout at render startup.
pub fn init_scene_bindings(mut commands: Commands) {
    let storage_ro = BindingType::Buffer {
        ty: BufferBindingType::Storage { read_only: true },
        has_dynamic_offset: false,
        min_binding_size: None,
    };
    let uniform = BindingType::Buffer {
        ty: BufferBindingType::Uniform,
        has_dynamic_offset: false,
        min_binding_size: None,
    };

    let bind_group_layout = BindGroupLayoutDescriptor::new(
        "photometric_scene_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                storage_ro, // @binding(0) primitives
                storage_ro, // @binding(1) materials
                storage_ro, // @binding(2) bvh_nodes
                uniform,    // @binding(3) config
                storage_ro, // @binding(4) cdf_data
            ),
        ),
    );

    commands.insert_resource(PhotometricSceneBindings {
        bind_group_layout,
        bind_group: None,
    });
}

/// Prepare scene buffers and create bind group 0.
pub fn prepare_scene_bindings(
    extracted: Res<ExtractedPhotometricScene>,
    scene_bindings: Option<ResMut<PhotometricSceneBindings>>,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
) {
    let Some(mut scene_bindings) = scene_bindings else {
        return;
    };
    if !extracted.changed {
        return;
    }

    // Build BVH from primitives
    let (bvh_nodes, sorted_prims) = build_bvh(&extracted.primitives);

    // Ensure non-empty buffers (wgpu requires non-zero size)
    let prims_data: &[u8] = if sorted_prims.is_empty() {
        &[0u8; 64] // sizeof GpuPrimitive
    } else {
        bytemuck::cast_slice(&sorted_prims)
    };
    let mats_data: &[u8] = if extracted.materials.is_empty() {
        &[0u8; 48] // sizeof GpuMaterial
    } else {
        bytemuck::cast_slice(&extracted.materials)
    };
    let default_bvh = BvhNode::default();
    let bvh_data: &[u8] = if bvh_nodes.is_empty() {
        bytemuck::bytes_of(&default_bvh)
    } else {
        bytemuck::cast_slice(&bvh_nodes)
    };

    // Default config
    let config: GpuTracerConfig = bytemuck::Zeroable::zeroed();
    let config_bytes = bytemuck::bytes_of(&config);

    let prims_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("photometric_primitives"),
        contents: prims_data,
        usage: BufferUsages::STORAGE,
    });
    let mats_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("photometric_materials"),
        contents: mats_data,
        usage: BufferUsages::STORAGE,
    });
    let bvh_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("photometric_bvh"),
        contents: bvh_data,
        usage: BufferUsages::STORAGE,
    });
    let config_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("photometric_config"),
        contents: config_bytes,
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });
    // Upload LVK data from first luminaire profile (for camera rendering)
    let lvk_data: Vec<f32> = extracted.luminaires.first()
        .and_then(|l| l.profile.as_ref())
        .map(|p| p.lvk_data.clone())
        .unwrap_or_else(|| vec![1.0]);

    let cdf_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("photometric_lvk_data"),
        contents: bytemuck::cast_slice(&lvk_data),
        usage: BufferUsages::STORAGE,
    });

    let layout = pipeline_cache.get_bind_group_layout(&scene_bindings.bind_group_layout);
    let bind_group = render_device.create_bind_group(
        "photometric_scene_bind_group",
        &layout,
        &BindGroupEntries::sequential((
            prims_buffer.as_entire_binding(),
            mats_buffer.as_entire_binding(),
            bvh_buffer.as_entire_binding(),
            config_buffer.as_entire_binding(),
            cdf_buffer.as_entire_binding(),
        )),
    );

    scene_bindings.bind_group = Some(bind_group);
}
