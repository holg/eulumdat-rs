//! Bevy shell for the light scene (`eulumdat-scene`): the same tracer the
//! editor runs on plain wgpu, here as Bevy compute shaders with a bevy_ui
//! overlay, for the native window and the optional `?wasm=rt_viewer`
//! bundle. All scene, camera and photometry logic lives in
//! [`eulumdat_scene::viewer::Viewer`]; this module only maps Bevy input and
//! assets onto it and dispatches the passes (`render`).
//!
//! Keys: 1–4 scene · V view · I isolines · G glare · H/Shift+H haze ·
//! [ ] exposure · ↑/↓ height · ←/→ rotation · T/Shift+T tilt · N count ·
//! L street layout · W facade finish · P person · ,/. spacing · K ceiling /
//! facade distance · C/Shift+C colour temperature · B bounces · Tab next
//! sample · R reset view · F1 help. Mouse: drag orbit, right-drag pan,
//! wheel zoom.

pub mod render;
pub mod ui;

use bevy::asset::RenderAssetUsages;
use bevy::input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::render::gpu_readback::{Readback, ReadbackComplete};
use bevy::render::render_resource::{
    BufferUsages, Extent3d, TextureDimension, TextureFormat, TextureUsages,
};
use bevy::render::storage::ShaderBuffer;
use bevy::window::PrimaryWindow;
use eulumdat_scene::scene::SceneKind;
use eulumdat_scene::viewer::{parse_any, Command, Settings, ViewMode, Viewer};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

pub use eulumdat_scene::gpu::MAX_SAMPLES;
use render::{RtFrameInput, RtRenderPlugin};

/// The viewer state (see `eulumdat_scene::viewer`).
#[derive(Resource)]
pub struct RtViewer(pub Viewer);

/// GPU-side assets the render world binds, plus the accumulation counters
/// it reports back.
#[derive(Resource)]
pub struct RtAssets {
    pub ies: Handle<Image>,
    pub ies_version: u64,
    pub output: Handle<Image>,
    pub probe_buffer: Handle<ShaderBuffer>,
    pub probe_len: usize,
    pub readback: Option<Entity>,
    pub samples: Arc<AtomicU32>,
    pub probe_samples: Arc<AtomicU32>,
}

/// Latest measurement point values from the GPU.
#[derive(Resource, Default)]
pub struct RtProbeValues(pub Vec<f32>);

/// A file path (native) to open at start.
#[derive(Resource, Default, Clone)]
pub struct RtStartFile(pub Option<String>);

/// Settings to start with (the binary's flags).
#[derive(Resource, Clone)]
pub struct RtStartSettings(pub Settings);

/// When set, keyboard and mouse are ignored: unattended screenshot runs
/// must not pick up input meant for other windows.
#[derive(Resource, Default, Clone)]
pub struct RtInputLocked(pub bool);

pub struct RtViewerPlugin;

impl Plugin for RtViewerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RtRenderPlugin)
            .init_resource::<RtProbeValues>()
            .init_resource::<RtStartFile>()
            .init_resource::<RtInputLocked>()
            .insert_resource(ClearColor(Color::BLACK))
            .add_systems(Startup, (setup, ui::setup_ui).chain())
            .add_systems(
                Update,
                (
                    poll_ldt,
                    poll_sync_settings,
                    handle_keys,
                    handle_mouse,
                    tick,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (ui::update_status, ui::update_points, publish_status).after(tick),
            );
    }
}

fn ies_image(table: &eulumdat_scene::photometry::IesTable) -> Image {
    Image::new(
        Extent3d {
            width: table.c_size,
            height: table.g_size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        table.to_bytes(),
        TextureFormat::R32Float,
        RenderAssetUsages::RENDER_WORLD,
    )
}

fn output_image(size: UVec2) -> Image {
    let mut image = Image::new_target_texture(
        size.x.max(1),
        size.y.max(1),
        TextureFormat::Rgba16Float,
        None,
    );
    image.asset_usage = RenderAssetUsages::RENDER_WORLD;
    image.texture_descriptor.usage =
        TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;
    image
}

fn probe_shader_buffer(count: usize) -> ShaderBuffer {
    let bytes = vec![0u8; count.max(1) * 16];
    let mut buf = ShaderBuffer::new(&bytes, RenderAssetUsages::RENDER_WORLD);
    buf.buffer_description.usage =
        BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC;
    buf
}

fn spawn_readback(commands: &mut Commands, assets: &mut RtAssets) {
    if let Some(e) = assets.readback.take() {
        commands.entity(e).despawn();
    }
    let e = commands
        .spawn(Readback::buffer(assets.probe_buffer.clone()))
        .observe(
            |ev: On<ReadbackComplete>, mut values: ResMut<RtProbeValues>| {
                let data: Vec<Vec4> = ev.to_shader_type();
                values.0 = data.iter().map(|v| v.x).collect();
            },
        )
        .id();
    assets.readback = Some(e);
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    start: Res<RtStartFile>,
    start_settings: Option<Res<RtStartSettings>>,
) {
    commands.spawn(Camera2d);
    let settings = start_settings.map(|s| s.0.clone()).unwrap_or_default();
    let mut viewer = Viewer::new(settings);

    // Start file (native path), then the browser's current luminaire, then the first sample.
    let mut loaded = false;
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(path) = &start.0 {
        match std::fs::read_to_string(path) {
            Ok(text) => match parse_any(path, &text) {
                Some(ldt) => {
                    viewer.load(path, ldt, None);
                    loaded = true;
                }
                None => error!("rt: could not parse {path}"),
            },
            Err(e) => error!("rt: cannot read {path}: {e}"),
        }
    }
    let _ = &start;
    if !loaded {
        if let Some(ldt) = crate::viewer::load_from_local_storage() {
            viewer.load("editor luminaire", ldt, None);
            loaded = true;
        }
    }
    if !loaded {
        viewer.load_sample(0);
    }
    if let Some(p) = &viewer.photometry {
        info!(
            "rt: loaded '{}': {} lm, peak {:.0} cd, throw C{:.0} asym {:.2}",
            p.name,
            p.ldt.luminous_flux(),
            p.table.max_cd,
            p.throw.c,
            p.throw.asymmetry
        );
    }

    let ies = images.add(
        viewer
            .photometry
            .as_ref()
            .map(|p| ies_image(&p.table))
            .unwrap_or_else(|| {
                Image::new_fill(
                    Extent3d::default(),
                    TextureDimension::D2,
                    &0f32.to_le_bytes(),
                    TextureFormat::R32Float,
                    RenderAssetUsages::RENDER_WORLD,
                )
            }),
    );
    let ies_version = viewer.photometry.as_ref().map(|p| p.version).unwrap_or(0);
    let output = images.add(output_image(UVec2::new(8, 8)));
    let probe_buffer = buffers.add(probe_shader_buffer(viewer.probes.len()));
    let mut assets = RtAssets {
        ies,
        ies_version,
        output,
        probe_buffer,
        probe_len: viewer.probes.len(),
        readback: None,
        samples: Arc::new(AtomicU32::new(0)),
        probe_samples: Arc::new(AtomicU32::new(0)),
    };
    spawn_readback(&mut commands, &mut assets);
    commands.insert_resource(assets);
    commands.insert_resource(RtViewer(viewer));
}

/// Native: files dropped on the window. Web: the editor's luminaire in
/// localStorage, re-read when its timestamp changes.
fn poll_ldt(
    mut viewer: ResMut<RtViewer>,
    #[allow(unused_mut, unused_variables)] mut drops: MessageReader<bevy::window::FileDragAndDrop>,
    mut last_stamp: Local<Option<String>>,
) {
    #[cfg(not(target_arch = "wasm32"))]
    for ev in drops.read() {
        if let bevy::window::FileDragAndDrop::DroppedFile { path_buf, .. } = ev {
            let name = path_buf.to_string_lossy().to_string();
            match std::fs::read_to_string(path_buf) {
                Ok(text) => match parse_any(&name, &text) {
                    Some(ldt) => viewer.0.load(&name, ldt, None),
                    None => error!("rt: could not parse {name}"),
                },
                Err(e) => error!("rt: cannot read {name}: {e}"),
            }
        }
    }
    let stamp = crate::viewer::wasm_sync::get_ldt_timestamp();
    if stamp.is_some() && stamp != *last_stamp {
        let first = last_stamp.is_none();
        *last_stamp = stamp;
        if !first {
            if let Some(ldt) = crate::viewer::load_from_local_storage() {
                viewer.0.load("editor luminaire", ldt, None);
            }
        }
    }
}

/// Web: pick up a host's settings (`eulumdat_rt_settings`) when their timestamp changes.
#[allow(unused_variables, unused_mut)]
fn poll_sync_settings(mut viewer: ResMut<RtViewer>, mut last_stamp: Local<Option<String>>) {
    #[cfg(target_arch = "wasm32")]
    {
        let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) else {
            return;
        };
        let stamp = storage
            .get_item("eulumdat_rt_settings_timestamp")
            .ok()
            .flatten();
        if stamp.is_none() || stamp == *last_stamp {
            return;
        }
        *last_stamp = stamp;
        let Some(json) = storage.get_item("eulumdat_rt_settings").ok().flatten() else {
            return;
        };
        match serde_json::from_str::<eulumdat_scene::sync::SyncSettings>(&json) {
            Ok(sync) => viewer.0.apply_sync(&sync),
            Err(e) => warn!("rt: bad settings JSON: {e}"),
        }
    }
}

/// Web: report progress, scale and grid statistics (`eulumdat_rt_status`).
#[allow(unused_variables, unused_mut)]
fn publish_status(viewer: Res<RtViewer>, mut tick: Local<u32>) {
    #[cfg(target_arch = "wasm32")]
    {
        *tick += 1;
        if *tick % 15 != 0 {
            return;
        }
        let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) else {
            return;
        };
        if let Ok(json) = serde_json::to_string(&viewer.0.status()) {
            let _ = storage.set_item("eulumdat_rt_status", &json);
        }
    }
}

fn handle_keys(
    keys: Res<ButtonInput<KeyCode>>,
    locked: Res<RtInputLocked>,
    mut viewer: ResMut<RtViewer>,
) {
    if locked.0 {
        return;
    }
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let sign = if shift { -1.0 } else { 1.0 };
    let v = &mut viewer.0;
    let scenes = [
        (KeyCode::Digit1, SceneKind::Street),
        (KeyCode::Digit2, SceneKind::Parking),
        (KeyCode::Digit3, SceneKind::Facade),
        (KeyCode::Digit4, SceneKind::Office),
        (KeyCode::Digit5, SceneKind::Greenhouse),
    ];
    for (key, kind) in scenes {
        if keys.just_pressed(key) {
            v.command(Command::Scene(kind));
        }
    }
    let step = if shift { 0.5 } else { 0.1 };
    if keys.just_pressed(KeyCode::ArrowUp) {
        v.command(Command::Height(step));
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        v.command(Command::Height(-step));
    }
    if keys.just_pressed(KeyCode::ArrowRight) {
        v.command(Command::Rotation(5.0));
    }
    if keys.just_pressed(KeyCode::ArrowLeft) {
        v.command(Command::Rotation(-5.0));
    }
    let single = [
        (KeyCode::KeyT, Command::Tilt(5.0 * sign)),
        (KeyCode::KeyN, Command::CycleCount),
        (KeyCode::KeyL, Command::CycleLayout),
        (KeyCode::KeyW, Command::ToggleFinish),
        (KeyCode::KeyP, Command::ToggleFigure),
        (KeyCode::Comma, Command::Spacing(-1.0)),
        (KeyCode::Period, Command::Spacing(1.0)),
        (KeyCode::KeyK, Command::Ceiling(0.1 * sign)),
        (KeyCode::KeyV, Command::NextMode),
        (KeyCode::KeyI, Command::ToggleIsolines),
        (KeyCode::KeyG, Command::ToggleGlare),
        (KeyCode::KeyH, Command::Haze(0.1 * sign)),
        (KeyCode::BracketLeft, Command::Exposure(-0.5)),
        (KeyCode::BracketRight, Command::Exposure(0.5)),
        (KeyCode::KeyC, Command::Cct(250.0 * sign)),
        (KeyCode::KeyB, Command::CycleBounces),
        (KeyCode::F1, Command::ToggleHelp),
        (KeyCode::Slash, Command::ToggleHelp),
        (KeyCode::KeyR, Command::ResetView),
        (KeyCode::Tab, Command::NextSample),
    ];
    for (key, cmd) in single {
        if keys.just_pressed(key) {
            v.command(cmd);
        }
    }
}

fn handle_mouse(
    buttons: Res<ButtonInput<MouseButton>>,
    locked: Res<RtInputLocked>,
    mut motion: MessageReader<MouseMotion>,
    mut wheel: MessageReader<MouseWheel>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut viewer: ResMut<RtViewer>,
) {
    if locked.0 {
        motion.clear();
        wheel.clear();
        return;
    }
    let mut delta = Vec2::ZERO;
    for m in motion.read() {
        delta += m.delta;
    }
    if delta != Vec2::ZERO {
        if buttons.pressed(MouseButton::Left) {
            viewer.0.orbit(delta.x, delta.y);
        } else if buttons.pressed(MouseButton::Right) || buttons.pressed(MouseButton::Middle) {
            let h = window.height().max(1.0);
            viewer.0.pan(delta.x, delta.y, h);
        }
    }
    for w in wheel.read() {
        let dy = match w.unit {
            MouseScrollUnit::Line => w.y * 40.0,
            MouseScrollUnit::Pixel => w.y,
        };
        if dy != 0.0 {
            viewer.0.zoom((-dy * 0.0015).clamp(-1.0, 1.0).exp());
        }
    }
}

/// Advance the viewer, keep the GPU assets in step and hand the frame to
/// the render world.
#[allow(clippy::too_many_arguments)]
fn tick(
    mut commands: Commands,
    time: Res<Time>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    mut viewer: ResMut<RtViewer>,
    mut assets: ResMut<RtAssets>,
    values: Res<RtProbeValues>,
    mut image_nodes: Query<&mut ImageNode, With<ui::RtImageNode>>,
) {
    let v = &mut viewer.0;
    v.note_frame_time(time.delta_secs() * 1000.0);
    if v.resize(window.width(), window.height(), window.scale_factor()) {
        assets.output = images.add(output_image(v.size));
        for mut node in image_nodes.iter_mut() {
            node.image = assets.output.clone();
        }
    }
    v.update(time.delta_secs());

    if let Some(p) = &v.photometry {
        if p.version != assets.ies_version {
            assets.ies = images.add(ies_image(&p.table));
            assets.ies_version = p.version;
        }
    }
    if v.probes.len() != assets.probe_len {
        assets.probe_buffer = buffers.add(probe_shader_buffer(v.probes.len()));
        assets.probe_len = v.probes.len();
        spawn_readback(&mut commands, &mut assets);
    }
    if values.0.len() == v.layout.points.len() {
        v.probe_values = values.0.clone();
    }
    v.samples = assets.samples.load(Ordering::Relaxed);
    v.probe_samples = assets.probe_samples.load(Ordering::Relaxed);

    commands.insert_resource(RtFrameInput {
        frame: v.frame_input(),
        ies: assets.ies.clone(),
        output: assets.output.clone(),
        probe_buffer: assets.probe_buffer.clone(),
        samples_out: assets.samples.clone(),
        probe_samples_out: assets.probe_samples.clone(),
    });
}

/// Samples rendered so far (0..MAX_SAMPLES), for the progress bar.
pub fn samples_done(viewer: &Viewer) -> u32 {
    viewer.samples
}

/// View mode of the viewer, for the overlay.
pub fn mode(viewer: &Viewer) -> ViewMode {
    viewer.settings.mode
}
