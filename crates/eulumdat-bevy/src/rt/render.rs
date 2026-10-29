//! Render-world side: the `eulumdat-scene` tracer as Bevy compute
//! pipelines, dispatched before the UI camera draws the result.

use bevy::core_pipeline::schedule::camera_driver;
use bevy::prelude::*;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::binding_types::{
    storage_buffer_read_only_sized, storage_buffer_sized, texture_2d, texture_storage_2d,
    uniform_buffer_sized,
};
use bevy::render::render_resource::*;
use bevy::render::renderer::{RenderContext, RenderDevice, RenderGraph, RenderQueue};
use bevy::render::storage::{GpuShaderBuffer, ShaderBuffer};
use bevy::render::texture::GpuImage;
use bevy::render::{Render, RenderApp, RenderStartup, RenderSystems};
use eulumdat_scene::gpu::{Params, PROBE_PASS_SAMPLES};
use eulumdat_scene::scene::{MAX_LAMPS, MAX_PRIMS};
use eulumdat_scene::viewer::FrameInput;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// The viewer's frame plus the Bevy assets to bind. Written by the main
/// world each frame, cloned into the render world by extraction.
#[derive(Resource, Clone, ExtractResource)]
pub struct RtFrameInput {
    pub frame: FrameInput,
    pub ies: Handle<Image>,
    pub output: Handle<Image>,
    pub probe_buffer: Handle<ShaderBuffer>,
    pub samples_out: Arc<AtomicU32>,
    pub probe_samples_out: Arc<AtomicU32>,
}

#[derive(Resource, Clone)]
struct RtShader(Handle<Shader>);

pub struct RtRenderPlugin;

impl Plugin for RtRenderPlugin {
    fn build(&self, app: &mut App) {
        let shader = app
            .world_mut()
            .resource_mut::<Assets<Shader>>()
            .add(Shader::from_wgsl(
                eulumdat_scene::TRACE_WGSL,
                "eulumdat_scene/trace.wgsl",
            ));
        app.add_plugins(ExtractResourcePlugin::<RtFrameInput>::default());
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .insert_resource(RtShader(shader))
            .init_resource::<RtGpuState>()
            .add_systems(RenderStartup, init_pipelines)
            .add_systems(Render, prepare_rt.in_set(RenderSystems::PrepareBindGroups))
            .add_systems(RenderGraph, dispatch_rt.before(camera_driver));
    }
}

#[derive(Resource)]
struct RtPipelines {
    layout: BindGroupLayoutDescriptor,
    trace: CachedComputePipelineId,
    probe: CachedComputePipelineId,
    present: CachedComputePipelineId,
}

/// What the bind group was built for; a change rebuilds it.
type BindKey = (
    AssetId<Image>,
    AssetId<Image>,
    AssetId<ShaderBuffer>,
    UVec2,
    u64,
);

#[derive(Resource, Default)]
struct RtGpuState {
    params: Option<Buffer>,
    boxes: Option<Buffer>,
    lamps: Option<Buffer>,
    color: Option<Buffer>,
    lux: Option<Buffer>,
    probe_pos: Option<Buffer>,
    mesh: Option<Buffer>,
    mesh_version: u64,
    size: UVec2,
    probes_version: u64,
    bind_group: Option<BindGroup>,
    bind_key: Option<BindKey>,
    samples: u32,
    probe_samples: u32,
    frame: u32,
    reset_token: u64,
    probe_reset_token: u64,
    ready: bool,
    logged_error: bool,
}

fn init_pipelines(
    mut commands: Commands,
    shader: Res<RtShader>,
    pipeline_cache: Res<PipelineCache>,
) {
    let layout = BindGroupLayoutDescriptor::new(
        "rt_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                uniform_buffer_sized(false, None),
                storage_buffer_read_only_sized(false, None),
                storage_buffer_read_only_sized(false, None),
                texture_2d(TextureSampleType::Float { filterable: false }),
                storage_buffer_sized(false, None),
                storage_buffer_sized(false, None),
                texture_storage_2d(TextureFormat::Rgba16Float, StorageTextureAccess::WriteOnly),
                storage_buffer_read_only_sized(false, None),
                storage_buffer_sized(false, None),
                storage_buffer_read_only_sized(false, None),
            ),
        ),
    );
    let make = |entry: &'static str| {
        pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some(format!("rt_{entry}").into()),
            layout: vec![layout.clone()],
            shader: shader.0.clone(),
            entry_point: Some(entry.into()),
            ..default()
        })
    };
    let trace = make("trace");
    let probe = make("probe");
    let present = make("present");
    commands.insert_resource(RtPipelines {
        layout,
        trace,
        probe,
        present,
    });
}

fn make_buffer(device: &RenderDevice, label: &str, size: u64, usage: BufferUsages) -> Buffer {
    device.create_buffer(&BufferDescriptor {
        label: Some(label),
        size: size.max(16),
        usage,
        mapped_at_creation: false,
    })
}

#[allow(clippy::too_many_arguments)]
fn prepare_rt(
    input: Option<Res<RtFrameInput>>,
    mut state: ResMut<RtGpuState>,
    pipelines: Option<Res<RtPipelines>>,
    pipeline_cache: Res<PipelineCache>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    images: Res<RenderAssets<GpuImage>>,
    buffers: Res<RenderAssets<GpuShaderBuffer>>,
) {
    state.ready = false;
    let (Some(input), Some(pipelines)) = (input, pipelines) else {
        return;
    };
    let frame = &input.frame;
    let rw = BufferUsages::STORAGE | BufferUsages::COPY_DST;

    if state.boxes.is_none() {
        state.params = Some(make_buffer(
            &device,
            "rt_params",
            std::mem::size_of::<Params>() as u64,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        ));
        state.boxes = Some(make_buffer(
            &device,
            "rt_prims",
            (MAX_PRIMS * 96) as u64,
            rw,
        ));
        state.lamps = Some(make_buffer(
            &device,
            "rt_lamps",
            (MAX_LAMPS * 64) as u64,
            rw,
        ));
    }

    let mut rebuild = false;
    if state.size != frame.size || state.color.is_none() {
        let px = (frame.size.x.max(1) * frame.size.y.max(1)) as u64;
        state.color = Some(make_buffer(&device, "rt_acc_color", px * 16, rw));
        state.lux = Some(make_buffer(&device, "rt_acc_lux", px * 16, rw));
        state.size = frame.size;
        state.samples = 0;
        rebuild = true;
    }
    if state.probe_pos.is_none() || state.probes_version != frame.probes_version {
        let bytes: &[u8] = bytemuck::cast_slice(frame.probes.as_slice());
        let buf = make_buffer(&device, "rt_probe_pos", bytes.len() as u64, rw);
        if !bytes.is_empty() {
            queue.write_buffer(&buf, 0, bytes);
        }
        state.probe_pos = Some(buf);
        state.probes_version = frame.probes_version;
        state.probe_samples = 0;
        rebuild = true;
    }
    if state.mesh.is_none() || state.mesh_version != frame.mesh_version {
        let rows: Vec<[f32; 4]> = frame.mesh.as_ref().map(|m| m.packed()).unwrap_or_default();
        let bytes: &[u8] = bytemuck::cast_slice(rows.as_slice());
        let buf = make_buffer(&device, "rt_mesh", (bytes.len() as u64).max(32), rw);
        if !bytes.is_empty() {
            queue.write_buffer(&buf, 0, bytes);
        }
        state.mesh = Some(buf);
        state.mesh_version = frame.mesh_version;
        rebuild = true;
    }
    if state.reset_token != frame.reset_token {
        state.reset_token = frame.reset_token;
        state.samples = 0;
    }
    if state.probe_reset_token != frame.probe_reset_token {
        state.probe_reset_token = frame.probe_reset_token;
        state.probe_samples = 0;
    }

    let mut boxes = frame.prims.clone();
    boxes.truncate(MAX_PRIMS);
    let mut lamps = frame.lamps.clone();
    lamps.truncate(MAX_LAMPS);
    if !boxes.is_empty() {
        queue.write_buffer(
            state.boxes.as_ref().unwrap(),
            0,
            bytemuck::cast_slice(&boxes),
        );
    }
    if !lamps.is_empty() {
        queue.write_buffer(
            state.lamps.as_ref().unwrap(),
            0,
            bytemuck::cast_slice(&lamps),
        );
    }

    let mut params = frame.params;
    params.counts.z = lamps.len() as u32;
    params.counts.w = boxes.len() as u32;
    params.frame = UVec4::new(
        state.frame,
        frame.probes.len() as u32,
        PROBE_PASS_SAMPLES,
        params.frame.w,
    );
    params.f0.x = 1.0 / (state.samples as f32 + 1.0);
    params.f0.y =
        PROBE_PASS_SAMPLES as f32 / (state.probe_samples as f32 + PROBE_PASS_SAMPLES as f32);
    params.size = UVec4::new(state.size.x, state.size.y, params.size.z, params.size.w);
    queue.write_buffer(
        state.params.as_ref().unwrap(),
        0,
        bytemuck::bytes_of(&params),
    );

    let (Some(ies), Some(out), Some(probe_buf)) = (
        images.get(&input.ies),
        images.get(&input.output),
        buffers.get(&input.probe_buffer),
    ) else {
        return;
    };
    let key = (
        input.ies.id(),
        input.output.id(),
        input.probe_buffer.id(),
        state.size,
        state.probes_version,
    );
    if rebuild || state.bind_key != Some(key) || state.bind_group.is_none() {
        let layout = pipeline_cache.get_bind_group_layout(&pipelines.layout);
        let bind_group = device.create_bind_group(
            "rt_bind_group",
            &layout,
            &BindGroupEntries::sequential((
                state.params.as_ref().unwrap().as_entire_binding(),
                state.boxes.as_ref().unwrap().as_entire_binding(),
                state.lamps.as_ref().unwrap().as_entire_binding(),
                &ies.texture_view,
                state.color.as_ref().unwrap().as_entire_binding(),
                state.lux.as_ref().unwrap().as_entire_binding(),
                &out.texture_view,
                state.probe_pos.as_ref().unwrap().as_entire_binding(),
                probe_buf.buffer.as_entire_binding(),
                state.mesh.as_ref().unwrap().as_entire_binding(),
            )),
        );
        state.bind_group = Some(bind_group);
        state.bind_key = Some(key);
    }
    state.ready = true;
}

fn dispatch_rt(
    mut ctx: RenderContext,
    input: Option<Res<RtFrameInput>>,
    mut state: ResMut<RtGpuState>,
    pipelines: Option<Res<RtPipelines>>,
    pipeline_cache: Res<PipelineCache>,
) {
    let (Some(input), Some(pipelines)) = (input, pipelines) else {
        return;
    };
    if !state.ready {
        return;
    }
    let Some(bind_group) = state.bind_group.clone() else {
        return;
    };
    let get = |id: CachedComputePipelineId| pipeline_cache.get_compute_pipeline(id);
    let (Some(trace), Some(probe), Some(present)) = (
        get(pipelines.trace),
        get(pipelines.probe),
        get(pipelines.present),
    ) else {
        if !state.logged_error {
            for (name, id) in [
                ("trace", pipelines.trace),
                ("probe", pipelines.probe),
                ("present", pipelines.present),
            ] {
                if let CachedPipelineState::Err(e) = pipeline_cache.get_compute_pipeline_state(id) {
                    error!("rt pipeline '{name}' failed: {e}");
                    state.logged_error = true;
                }
            }
        }
        return;
    };

    let frame = &input.frame;
    let w = state.size.x.max(1);
    let h = state.size.y.max(1);
    let mut pass = ctx
        .command_encoder()
        .begin_compute_pass(&ComputePassDescriptor {
            label: Some("rt_pass"),
            ..default()
        });
    pass.set_bind_group(0, &bind_group, &[]);
    if frame.trace && state.samples < frame.max_samples {
        pass.set_pipeline(trace);
        pass.dispatch_workgroups(w.div_ceil(8), h.div_ceil(8), 1);
        state.samples += 1;
    }
    let n = frame.probes.len() as u32;
    if frame.probe && n > 0 && state.probe_samples < frame.probe_max_samples {
        pass.set_pipeline(probe);
        pass.dispatch_workgroups(n.div_ceil(64), 1, 1);
        state.probe_samples += PROBE_PASS_SAMPLES;
    }
    pass.set_pipeline(present);
    pass.dispatch_workgroups(w.div_ceil(8), h.div_ceil(8), 1);
    drop(pass);

    state.frame = state.frame.wrapping_add(1);
    input.samples_out.store(state.samples, Ordering::Relaxed);
    input
        .probe_samples_out
        .store(state.probe_samples, Ordering::Relaxed);
}
