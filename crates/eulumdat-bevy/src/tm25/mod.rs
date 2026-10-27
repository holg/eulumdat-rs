//! TM-25 ray file viewer: a lazily loaded bundle (`?wasm=tm25`) that shows
//! where light leaves an LED (ray segments, emission point cloud) and the
//! far-field solid derived from the rays, overlaid in the same convention as
//! the photometric solid (γ = 0 → Bevy −Y).
//!
//! Phase 1 (this module): CPU ingest with reservoir subsampling, static
//! `LineList` meshes for rays and point crosses, the reused photometric
//! solid, an orbit camera at millimetre scale, settings/status exchanged with
//! the Leptos shell through `localStorage`, files handed over through the
//! loader's IndexedDB stash (`window.tm25TakeFile`). Camera-view filtering
//! and the target-plane irradiance are phase 2 (see `docs/tm25-bevy.md`).

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;

use bevy::asset::RenderAssetUsages;
use bevy::input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::mesh::PrimitiveTopology;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use serde::{Deserialize, Serialize};

use crate::photometric::{
    photometric_solid_material, photometric_solid_mesh, PhotometricData, PhotometricMeshResolution,
};
use eulumdat_tm25::{
    to_bevy, FarField, FarFieldBuilder, FluxKind, Header, ParseOptions, Ray, RayLayout, Reservoir,
    SpectralTable, Tm25Error,
};

/// localStorage keys shared with the Leptos shell.
pub const SETTINGS_KEY: &str = "tm25_settings";
pub const STATUS_KEY: &str = "tm25_status";
pub const STATUS_TS_KEY: &str = "tm25_status_ts";

/// Rays kept for analysis and display.
const DEFAULT_KEEP: usize = 200_000;

// ---------------------------------------------------------------------------
// Settings (JSON in localStorage, written by the shell)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ColorMode {
    /// Blue on the axis, red at 90°.
    #[default]
    Gamma,
    /// Start height z: separates chip from reflector walls.
    Height,
    /// Per-ray flux.
    Flux,
    /// Colour of the header spectrum (false colour for UV / IR).
    Spectrum,
}

#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Tm25Settings {
    /// Rays drawn as segments.
    pub display_count: usize,
    pub ray_length_mm: f32,
    pub show_rays: bool,
    pub show_points: bool,
    pub show_solid: bool,
    /// Radius of the far-field solid at peak intensity, mm.
    pub solid_scale_mm: f32,
    /// Half size of a point cross, mm.
    pub point_size_mm: f32,
    pub color_mode: ColorMode,
    pub seed: u64,
    /// Bumped by the shell to request a camera reset.
    pub reset_camera: u32,
    /// Target-plane irradiance map (near field).
    pub show_plane: bool,
    /// Plane distance above the coordinate origin along the emission axis, mm.
    pub plane_z_mm: f32,
    /// Half width of the plane, mm; 0 = automatic from distance and source size.
    pub plane_size_mm: f32,
    /// Cells per side of the irradiance grid.
    pub plane_cells: usize,
    /// Highlight the start points of rays heading towards the camera.
    pub show_camera_view: bool,
    /// Half angle of the acceptance cone, degrees.
    pub camera_cone_deg: f32,
    /// Pixel side of the luminance map drawn for the camera view, mm.
    pub luminance_pixel_mm: f32,
}

impl Default for Tm25Settings {
    fn default() -> Self {
        Self {
            display_count: 50_000,
            ray_length_mm: 3.0,
            show_rays: true,
            show_points: true,
            show_solid: false,
            solid_scale_mm: 8.0,
            point_size_mm: 0.01,
            color_mode: ColorMode::Gamma,
            seed: 1,
            reset_camera: 0,
            show_plane: false,
            plane_z_mm: 0.1,
            plane_size_mm: 0.0,
            plane_cells: 96,
            show_camera_view: false,
            camera_cone_deg: 8.0,
            luminance_pixel_mm: 0.05,
        }
    }
}

// ---------------------------------------------------------------------------
// Target-plane irradiance
// ---------------------------------------------------------------------------

/// Flux of the kept (flux-rescaled) rays splatted onto the plane z = `z_mm`.
#[derive(Clone, Debug)]
pub struct IrradianceGrid {
    pub n: usize,
    pub half_mm: f32,
    pub z_mm: f32,
    /// Row-major `n × n`, W/m² (radiometric) or lm/m² = lx (photometric).
    pub cells: Vec<f64>,
    pub peak: f64,
    /// Mean over cells that received any flux.
    pub mean_lit: f64,
    /// Fraction of the file flux that reached the plane inside the grid.
    pub captured: f64,
    /// Radial profile `(r mm, E)` averaged over rings from the centre.
    pub radial: Vec<(f64, f64)>,
    pub rays_hit: u64,
}

impl IrradianceGrid {
    pub fn compute(l: &LoadedRays, z_mm: f32, half_mm: f32, n: usize) -> Self {
        let n = n.clamp(8, 512);
        let kind = l.flux_kind();
        let cell = 2.0 * half_mm / n as f32;
        let cell_area_m2 = (cell as f64 * 1e-3).powi(2);
        let mut cells = vec![0.0f64; n * n];
        let mut hit = 0u64;
        let mut total = 0.0f64;
        let mut captured = 0.0f64;
        for r in &l.rays {
            let f = r.flux(kind) as f64;
            total += f;
            if r.kz <= 1e-6 {
                continue;
            }
            let t = (z_mm - r.z) / r.kz;
            if t < 0.0 {
                continue;
            }
            let x = r.x + r.kx * t;
            let y = r.y + r.ky * t;
            if x.abs() >= half_mm || y.abs() >= half_mm {
                continue;
            }
            let ix = ((x + half_mm) / cell) as usize;
            let iy = ((y + half_mm) / cell) as usize;
            cells[iy.min(n - 1) * n + ix.min(n - 1)] += f;
            captured += f;
            hit += 1;
        }
        for c in cells.iter_mut() {
            *c /= cell_area_m2;
        }
        let peak = cells.iter().cloned().fold(0.0, f64::max);
        let lit: Vec<f64> = cells.iter().cloned().filter(|v| *v > 0.0).collect();
        let mean_lit = if lit.is_empty() {
            0.0
        } else {
            lit.iter().sum::<f64>() / lit.len() as f64
        };
        // Radial profile: 24 rings out to the half width.
        let rings = 24usize;
        let mut sum = vec![0.0f64; rings];
        let mut cnt = vec![0u32; rings];
        for iy in 0..n {
            for ix in 0..n {
                let x = (ix as f32 + 0.5) * cell - half_mm;
                let y = (iy as f32 + 0.5) * cell - half_mm;
                let r = (x * x + y * y).sqrt();
                let ri = ((r / half_mm) * rings as f32) as usize;
                if ri < rings {
                    sum[ri] += cells[iy * n + ix];
                    cnt[ri] += 1;
                }
            }
        }
        let radial = (0..rings)
            .map(|i| {
                let r = (i as f64 + 0.5) / rings as f64 * half_mm as f64;
                (
                    r,
                    if cnt[i] > 0 {
                        sum[i] / cnt[i] as f64
                    } else {
                        0.0
                    },
                )
            })
            .collect();
        Self {
            n,
            half_mm,
            z_mm,
            cells,
            peak,
            mean_lit,
            captured: if total > 0.0 { captured / total } else { 0.0 },
            radial,
            rays_hit: hit,
        }
    }

    /// Automatic half width: the source footprint plus the spread of a
    /// ±72° cone from the lowest start point to the plane.
    pub fn auto_half_mm(l: &LoadedRays, z_mm: f32) -> f32 {
        let footprint = l.extent_mm() * 0.6;
        let rise = (z_mm - l.min[2]).max(0.0);
        (footprint + rise * 3.0).clamp(1.0, 400.0)
    }

    /// RGBA8 texture, jet-like map, transparent where nothing arrived.
    pub fn to_rgba(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.n * self.n * 4);
        for v in &self.cells {
            let t = if self.peak > 0.0 {
                (v / self.peak) as f32
            } else {
                0.0
            };
            let [r, g, b] = jet(t);
            let a = if *v > 0.0 { 225 } else { 90 };
            out.extend_from_slice(&[(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, a]);
        }
        out
    }
}

/// Blue → cyan → green → yellow → red, like the vendor plots.
pub fn jet(t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    let r = (1.5 - (4.0 * t - 3.0).abs()).clamp(0.0, 1.0);
    let g = (1.5 - (4.0 * t - 2.0).abs()).clamp(0.0, 1.0);
    let b = (1.5 - (4.0 * t - 1.0).abs()).clamp(0.0, 1.0);
    [r, g, b]
}

// ---------------------------------------------------------------------------
// Loaded data
// ---------------------------------------------------------------------------

/// Everything the viewer keeps from a file.
#[derive(Clone, Debug)]
pub struct LoadedRays {
    pub name: String,
    pub header: Header,
    /// Flux-rescaled analysis subset (≤ `DEFAULT_KEEP` rays).
    pub rays: Vec<Ray>,
    /// Far field over every ray of the file.
    pub far_field: FarField,
    /// Start-point bounds in TM-25 coordinates (mm).
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub ray_count: u64,
    pub max_ray_flux: f32,
    /// Consistency checks over every ray (`tm25ray::quality`).
    pub quality: eulumdat_tm25::QualityReport,
}

impl LoadedRays {
    pub fn flux_kind(&self) -> FluxKind {
        self.header.flux_kind()
    }

    pub fn centre_bevy(&self) -> Vec3 {
        let c = [
            (self.min[0] + self.max[0]) * 0.5,
            (self.min[1] + self.max[1]) * 0.5,
            (self.min[2] + self.max[2]) * 0.5,
        ];
        Vec3::from(to_bevy(c))
    }

    pub fn extent_mm(&self) -> f32 {
        (self.max[0] - self.min[0])
            .max(self.max[1] - self.min[1])
            .max(self.max[2] - self.min[2])
            .max(0.5)
    }
}

/// Streaming ingest shared by the native thread and the browser task.
pub struct Ingest {
    pub header: Header,
    layout: RayLayout,
    ff: FarFieldBuilder,
    res: Reservoir,
    count: u64,
    min: [f32; 3],
    max: [f32; 3],
    max_flux: f32,
    carry: Vec<u8>,
    quality: eulumdat_tm25::QualityAccumulator,
}

impl Ingest {
    pub fn new(header: Header, keep: usize, seed: u64) -> Self {
        let kind = header.flux_kind();
        Self {
            layout: RayLayout::from_header(&header),
            ff: FarFieldBuilder::new(10.0, 5.0, kind),
            res: Reservoir::new(keep, seed),
            count: 0,
            min: [f32::MAX; 3],
            max: [f32::MIN; 3],
            max_flux: 0.0,
            carry: Vec::new(),
            quality: eulumdat_tm25::QualityAccumulator::new(&header),
            header,
        }
    }

    /// Feed raw ray-block bytes (any chunking; partial records are carried over).
    pub fn feed(&mut self, bytes: &[u8]) {
        let rs = self.layout.record_size;
        let kind = self.header.flux_kind();
        let data: std::borrow::Cow<[u8]> = if self.carry.is_empty() {
            std::borrow::Cow::Borrowed(bytes)
        } else {
            let mut v = std::mem::take(&mut self.carry);
            v.extend_from_slice(bytes);
            std::borrow::Cow::Owned(v)
        };
        let whole = data.len() / rs * rs;
        for rec in data[..whole].chunks_exact(rs) {
            let r = self.layout.decode(rec);
            self.ff.add(&r);
            let p = r.position();
            for ((lo, hi), v) in self.min.iter_mut().zip(self.max.iter_mut()).zip(p) {
                *lo = lo.min(v);
                *hi = hi.max(v);
            }
            self.max_flux = self.max_flux.max(r.flux(kind));
            self.quality.add(&r);
            self.res.push(r);
            self.count += 1;
        }
        self.carry = data[whole..].to_vec();
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    pub fn finish(self, name: String) -> LoadedRays {
        LoadedRays {
            name,
            header: self.header,
            rays: self.res.finish(),
            far_field: self.ff.finish(),
            min: self.min,
            max: self.max,
            ray_count: self.count,
            max_ray_flux: self.max_flux,
            quality: self.quality.finish(),
        }
    }
}

/// Grow-and-retry header parse over a prefix of the file.
/// Parse the header at the start of a ray file: TM-25, or a LightTools
/// binary ray file (`LTRF`), which comes back as a synthesised TM-25 header
/// whose column layout, `ray_start` and `n_rays` (from `file_size`) let the
/// same record decoder stream it. `name` fills the synthesised name field.
pub fn parse_header_prefix(
    prefix: &[u8],
    name: &str,
    file_size: Option<u64>,
) -> Result<Header, Tm25Error> {
    if eulumdat_tm25::is_lighttools(prefix) {
        let lt = eulumdat_tm25::LightToolsHeader::parse(prefix)?;
        return Ok(lt.to_tm25_header(name, file_size));
    }
    Header::parse(prefix, &ParseOptions::default())
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

#[allow(dead_code)] // `Idle` is only produced by the browser poll
enum LoadMsg {
    Idle,
    Progress(u64, u64),
    Done(Box<LoadedRays>),
    Error(String),
}

#[derive(Resource)]
#[allow(dead_code)] // some fields are only read on wasm32
struct LoadChannel {
    tx: Sender<LoadMsg>,
    rx: Mutex<Receiver<LoadMsg>>,
    busy: bool,
    progress: Option<(u64, u64)>,
    error: Option<String>,
    poll_timer: f32,
}

#[derive(Resource, Default)]
pub struct Tm25Data {
    pub loaded: Option<LoadedRays>,
    dirty: bool,
    /// Last irradiance grid, republished with the status.
    pub irradiance: Option<IrradianceGrid>,
}

#[derive(Resource, Default)]
#[allow(dead_code)] // `last_json` is only used on wasm32
struct SettingsSync {
    last_json: String,
    last_reset: u32,
}

#[derive(Resource)]
struct Orbit {
    target: Vec3,
    yaw: f32,
    pitch: f32,
    distance: f32,
}

impl Default for Orbit {
    fn default() -> Self {
        // Emission goes to −Y, so start below the horizontal plane looking up
        // at the emitter: the ray fan spreads down and across the view like
        // a downlight, instead of pointing at the camera.
        Self {
            target: Vec3::ZERO,
            yaw: 0.7,
            pitch: -0.45,
            distance: 12.0,
        }
    }
}

#[derive(Component)]
struct Tm25Entity;

/// Entities of the camera-view highlight, rebuilt when the view direction moves.
#[derive(Component)]
struct CamViewEntity;

#[derive(Resource, Default)]
struct CamViewState {
    /// View direction (TM-25 frame, unit) used for the current highlight.
    last_dir: Option<[f32; 3]>,
    last_cone: f32,
    last_count: usize,
    last_display: usize,
    last_pixel: f32,
}

#[derive(Component)]
struct Tm25Camera;

/// Optional native start-up file (path from the command line).
#[derive(Resource, Default, Clone)]
pub struct Tm25StartFile(pub Option<String>);

pub struct Tm25ViewerPlugin;

impl Plugin for Tm25ViewerPlugin {
    fn build(&self, app: &mut App) {
        let (tx, rx) = channel();
        app.insert_resource(Tm25Settings::default())
            .insert_resource(Tm25Data::default())
            .insert_resource(SettingsSync::default())
            .insert_resource(Orbit::default())
            .insert_resource(CamViewState::default())
            .insert_resource(LoadChannel {
                tx,
                rx: Mutex::new(rx),
                busy: false,
                progress: None,
                error: None,
                poll_timer: 0.0,
            })
            .init_resource::<Tm25StartFile>()
            .insert_resource(ClearColor(Color::srgb(0.05, 0.06, 0.08)))
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    start_native_load,
                    poll_browser_file,
                    receive_loads,
                    sync_settings,
                    keyboard,
                    rebuild_scene,
                    orbit_camera,
                    camera_view,
                    draw_gizmos,
                )
                    .chain(),
            );
    }
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            near: 0.01,
            far: 5000.0,
            ..default()
        }),
        Transform::from_xyz(8.0, 6.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
        Tm25Camera,
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 3000.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 400.0,
        ..default()
    });
}

// ---------------------------------------------------------------------------
// Loading: native
// ---------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
fn load_native_blocking(path: &str, tx: &Sender<LoadMsg>) -> Result<LoadedRays, String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).map_err(|e| format!("{path}: {e}"))?;
    let total = file.metadata().map(|m| m.len()).unwrap_or(0);
    // Header: read a growing prefix until it parses.
    let mut prefix = Vec::with_capacity(64 * 1024);
    let mut need = 64 * 1024usize;
    let header = loop {
        let old = prefix.len();
        prefix.resize(need, 0);
        let mut filled = old;
        while filled < need {
            let n = file
                .read(&mut prefix[filled..need])
                .map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            filled += n;
        }
        prefix.truncate(filled);
        match parse_header_prefix(&prefix, &file_name_of(path), Some(total)) {
            Ok(h) => break h,
            Err(Tm25Error::Truncated { needed, .. }) if needed > prefix.len() && filled == need => {
                need = needed
            }
            Err(e) => return Err(e.to_string()),
        }
    };
    if total > 0 {
        eulumdat_tm25::check_file_size(&header, total)?;
    }
    let mut ingest = Ingest::new(header.clone(), DEFAULT_KEEP, 1);
    if prefix.len() > header.ray_start {
        ingest.feed(&prefix[header.ray_start..]);
    }
    let mut buf = vec![0u8; 4 << 20];
    let mut last_report = 0u64;
    // Stop at the end of the ray block: a LightTools file has a trailer.
    let mut left = header
        .ray_block_size()
        .saturating_sub(ingest.count() * header.record_size() as u64);
    while left > 0 {
        let want = (left.min(buf.len() as u64)) as usize;
        let n = file.read(&mut buf[..want]).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        left -= n as u64;
        ingest.feed(&buf[..n]);
        if ingest.count() - last_report > 1_000_000 {
            last_report = ingest.count();
            let _ = tx.send(LoadMsg::Progress(ingest.count(), header.n_rays));
        }
    }
    if ingest.count() != header.n_rays {
        return Err(format!(
            "ray count mismatch: header {} vs file {} ({} bytes)",
            header.n_rays,
            ingest.count(),
            total
        ));
    }
    Ok(ingest.finish(file_name_of(path)))
}

#[cfg(not(target_arch = "wasm32"))]
fn file_name_of(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_native_load(path: String, chan: &mut LoadChannel) {
    let tx = chan.tx.clone();
    chan.busy = true;
    chan.error = None;
    chan.progress = Some((0, 0));
    std::thread::spawn(move || {
        let msg = match load_native_blocking(&path, &tx) {
            Ok(l) => LoadMsg::Done(Box::new(l)),
            Err(e) => LoadMsg::Error(e),
        };
        let _ = tx.send(msg);
    });
}

/// Native: command-line file at start-up, then drag-and-drop.
fn start_native_load(
    #[allow(unused_mut)] mut chan: ResMut<LoadChannel>,
    #[allow(unused_mut)] mut start: ResMut<Tm25StartFile>,
    #[allow(unused_variables, unused_mut)] mut drops: MessageReader<bevy::window::FileDragAndDrop>,
) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(path) = start.0.take() {
            spawn_native_load(path, &mut chan);
        }
        for ev in drops.read() {
            if let bevy::window::FileDragAndDrop::DroppedFile { path_buf, .. } = ev {
                if !chan.busy {
                    spawn_native_load(path_buf.to_string_lossy().to_string(), &mut chan);
                }
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (&mut chan, &mut start);
    }
}

// ---------------------------------------------------------------------------
// Loading: browser
// ---------------------------------------------------------------------------

/// Browser: ask the loader for a stashed `File` (`window.tm25TakeFile()`),
/// stream it through `Blob.slice` in 4 MiB pieces.
fn poll_browser_file(mut chan: ResMut<LoadChannel>, time: Res<Time>) {
    #[cfg(target_arch = "wasm32")]
    {
        if chan.busy {
            return;
        }
        chan.poll_timer += time.delta_secs();
        if chan.poll_timer < 0.5 {
            return;
        }
        chan.poll_timer = 0.0;
        chan.busy = true;
        let tx = chan.tx.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let msg = match load_browser_file().await {
                Ok(Some(l)) => LoadMsg::Done(Box::new(l)),
                Ok(None) => LoadMsg::Idle,
                Err(e) => LoadMsg::Error(e),
            };
            let _ = tx.send(msg);
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (&mut chan, &time);
    }
}

#[cfg(target_arch = "wasm32")]
async fn load_browser_file() -> Result<Option<LoadedRays>, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let window = web_sys::window().ok_or("no window")?;
    let take = js_sys::Reflect::get(&window, &"tm25TakeFile".into())
        .map_err(|_| "tm25TakeFile missing")?;
    let Ok(take) = take.dyn_into::<js_sys::Function>() else {
        return Ok(None); // loader not present (dev shell)
    };
    let promise: js_sys::Promise = take
        .call0(&window)
        .map_err(|e| format!("{e:?}"))?
        .dyn_into()
        .map_err(|_| "tm25TakeFile did not return a Promise")?;
    let val = JsFuture::from(promise)
        .await
        .map_err(|e| format!("{e:?}"))?;
    if val.is_null() || val.is_undefined() {
        return Ok(None);
    }
    let name = js_sys::Reflect::get(&val, &"name".into())
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| "ray file".to_string());
    let blob: web_sys::Blob = val.dyn_into().map_err(|_| "stashed value is not a Blob")?;
    let size = blob.size();
    let read = |start: f64, end: f64| {
        let blob = blob.clone();
        async move {
            let part = blob
                .slice_with_f64_and_f64(start, end)
                .map_err(|e| format!("{e:?}"))?;
            let buf = JsFuture::from(part.array_buffer())
                .await
                .map_err(|e| format!("{e:?}"))?;
            Ok::<Vec<u8>, String>(js_sys::Uint8Array::new(&buf).to_vec())
        }
    };
    // Header prefix, grown on demand.
    let mut need = (1u64 << 20).min(size as u64) as f64;
    let (header, prefix) = loop {
        let prefix = read(0.0, need).await?;
        match parse_header_prefix(&prefix, &name, Some(size as u64)) {
            Ok(h) => break (h, prefix),
            Err(Tm25Error::Truncated { needed, .. }) if (needed as f64) > need && need < size => {
                need = (needed as f64).min(size);
            }
            Err(e) => return Err(e.to_string()),
        }
    };
    eulumdat_tm25::check_file_size(&header, size as u64)?;
    let mut ingest = Ingest::new(header.clone(), DEFAULT_KEEP, 1);
    let rs = header.record_size() as f64;
    let mut offset = header.ray_start as f64;
    if prefix.len() as f64 > offset {
        // Only whole records from the prefix; the rest is re-read below.
        let whole = ((prefix.len() as f64 - offset) / rs).floor() * rs;
        ingest.feed(&prefix[offset as usize..(offset + whole) as usize]);
        offset += whole;
    }
    let step = ((4u64 << 20) as f64 / rs).floor() * rs;
    // End of the ray block, not of the file: a LightTools file has a trailer.
    let block_end = (header.ray_start as f64 + header.ray_block_size() as f64).min(size);
    while offset < block_end {
        let end = (offset + step).min(block_end);
        let bytes = read(offset, end).await?;
        ingest.feed(&bytes);
        offset = end;
        publish_progress(ingest.count(), header.n_rays);
    }
    Ok(Some(ingest.finish(name)))
}

#[cfg(target_arch = "wasm32")]
fn publish_progress(done: u64, total: u64) {
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = storage.set_item("tm25_progress", &format!("{done}/{total}"));
    }
}

fn receive_loads(
    mut chan: ResMut<LoadChannel>,
    mut data: ResMut<Tm25Data>,
    mut orbit: ResMut<Orbit>,
) {
    let msgs: Vec<LoadMsg> = {
        let rx = chan.rx.lock().unwrap();
        rx.try_iter().collect()
    };
    for m in msgs {
        match m {
            LoadMsg::Idle => chan.busy = false,
            LoadMsg::Progress(a, b) => chan.progress = Some((a, b)),
            LoadMsg::Error(e) => {
                error!("TM-25 load failed: {e}");
                chan.error = Some(e.clone());
                chan.busy = false;
                chan.progress = None;
                publish_status(&format!("{{\"error\":{}}}", json_str(&e)));
            }
            LoadMsg::Done(l) => {
                info!(
                    "TM-25 loaded {}: {} rays, {:?}, kept {}",
                    l.name,
                    l.ray_count,
                    l.flux_kind(),
                    l.rays.len()
                );
                orbit.target = l.centre_bevy();
                orbit.distance = l.extent_mm() * 4.0;
                publish_status(&status_json(&l, None));
                publish_exports(&l);
                data.irradiance = None;
                data.loaded = Some(*l);
                data.dirty = true;
                chan.busy = false;
                chan.progress = None;
                chan.error = None;
            }
        }
    }
}

fn json_str(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into())
}

/// Status for the shell's info panel.
fn status_json(l: &LoadedRays, irr: Option<&IrradianceGrid>) -> String {
    let h = &l.header;
    // Actinic UV weighting of the header spectrum (safety check for UV sources).
    let uv = h.shared_spectrum().filter(|t| t.is_ultraviolet()).map(|t| {
        let spd = eulumdat_spectrum::Spd::new(
            &t.wavelengths_nm.iter().map(|w| *w as f64).collect::<Vec<_>>(),
            &t.values.iter().map(|v| *v as f64).collect::<Vec<_>>(),
        );
        let action = eulumdat_spectrum::ActionSpectrum::actinic_uv_hazard();
        let k = action.effective_fraction(&spd);
        let peak_eff = irr.map(|g| g.peak * k);
        serde_json::json!({
            "action_spectrum": action.name,
            "effective_fraction": k,
            "limit_j_per_m2": eulumdat_spectrum::ACTINIC_LIMIT_J_PER_M2,
            "plane_peak_effective_w_per_m2": peak_eff,
            "seconds_to_limit_at_peak": peak_eff.and_then(|e| eulumdat_spectrum::uv::seconds_to_limit(e, eulumdat_spectrum::ACTINIC_LIMIT_J_PER_M2)),
        })
    });
    // Horticulture: photon quantities whenever the spectrum overlaps PAR.
    let horticulture = h.shared_spectrum().and_then(|t| {
        let spd = eulumdat_spectrum::Spd::new(
            &t.wavelengths_nm
                .iter()
                .map(|w| *w as f64)
                .collect::<Vec<_>>(),
            &t.values.iter().map(|v| *v as f64).collect::<Vec<_>>(),
        );
        let m = eulumdat_spectrum::SpectralMetrics::from_spd(&spd);
        if m.ppfd_per_watt <= 0.0 {
            return None;
        }
        // Radiant watts of the file: direct, or lumens / efficacy of radiation.
        let radiant_w = match l.flux_kind() {
            FluxKind::Photometric => {
                if m.ler_lm_per_w > 0.0 {
                    h.luminous_flux_lm as f64 / m.ler_lm_per_w
                } else {
                    0.0
                }
            }
            _ => h.radiant_flux_w as f64,
        };
        let to_w = |v: f64| match l.flux_kind() {
            FluxKind::Photometric => {
                if m.ler_lm_per_w > 0.0 {
                    v / m.ler_lm_per_w
                } else {
                    0.0
                }
            }
            _ => v,
        };
        Some(serde_json::json!({
            "ppfd_per_watt": m.ppfd_per_watt,
            "ler_lm_per_w": m.ler_lm_per_w,
            "radiant_w": radiant_w,
            "ppf_umol_s": radiant_w * m.ppfd_per_watt,
            "ppfd_peak_umol_m2_s": irr.map(|g| to_w(g.peak) * m.ppfd_per_watt),
            "blue_fraction": m.blue_fraction,
        }))
    });
    let irradiance = irr.map(|g| {
        serde_json::json!({
            "z_mm": g.z_mm,
            "half_mm": g.half_mm,
            "cells": g.n,
            "peak": g.peak,
            "mean_lit": g.mean_lit,
            "captured": g.captured,
            "rays_hit": g.rays_hit,
            "unit": match l.flux_kind() { FluxKind::Photometric => "lx", _ => "W/m²" },
            "radial": g.radial,
        })
    });
    let prof = l.far_field.azimuthal_average();
    let max = prof.iter().cloned().fold(0.0, f64::max);
    let rel: Vec<f64> = prof
        .iter()
        .map(|v| if max > 0.0 { v / max } else { 0.0 })
        .collect();
    // PDF-style cross sections through the emission axis: x cut is C = 0
    // (positive angles) / C = 180 (negative), y cut is C = 90 / 270, from a
    // lightly smoothed grid so single-bin noise does not dominate.
    let sm = l.far_field.smoothed(1, 0, 6.0);
    let cut = |c_pos: f64, c_neg: f64| -> Vec<(f64, f64)> {
        let mut v = Vec::new();
        let mut a = -90.0;
        while a <= 90.0 + 1e-9 {
            let val = if a < 0.0 {
                sm.sample(c_neg, -a)
            } else {
                sm.sample(c_pos, a)
            };
            v.push((a, val));
            a += 2.5;
        }
        v
    };
    let cut_x = cut(0.0, 180.0);
    let cut_y = cut(90.0, 270.0);
    let spectrum = spectrum_of(l).map(|t| {
        serde_json::json!({
            "wavelengths": t.wavelengths_nm,
            "values": t.values,
            "peak": t.peak(),
            "centroid": t.centroid_nm(),
            "uv": t.is_ultraviolet(),
        })
    });
    serde_json::json!({
        "name": l.name,
        "rays": l.ray_count,
        "kept": l.rays.len(),
        "flux_kind": format!("{:?}", l.flux_kind()),
        "radiant_flux_w": h.radiant_flux_w,
        "luminous_flux_lm": h.luminous_flux_lm,
        "date": h.date_time,
        "text": h.text.raw,
        "columns": h.columns.iter().map(|c| format!("{c:?}")).collect::<Vec<_>>(),
        "bounds_mm": {"min": l.min, "max": l.max},
        "far_field": {
            "unit": format!("{:?}", l.far_field.unit),
            "peak": max,
            "gamma": l.far_field.g_angles(),
            "relative": rel,
            "half_intensity_deg": l.far_field.half_intensity_gamma(),
            "cut_x": cut_x,
            "cut_y": cut_y,
        },
        "spectrum": spectrum,
        "quality": {
            "verdict": l.quality.verdict(),
            "errors": l.quality.errors(),
            "warnings": l.quality.warnings(),
            "noise": l.quality.far_field_noise,
            "duplicates": l.quality.stats.duplicates,
            "findings": l.quality.findings.iter().map(|f| serde_json::json!({
                "severity": f.severity.as_str(),
                "code": f.code,
                "message": f.message,
            })).collect::<Vec<_>>(),
        },
        "irradiance": irradiance,
        "uv": uv,
        "horticulture": horticulture,
    })
    .to_string()
}

/// Key of the export bundle for the shell's download buttons.
pub const EXPORTS_KEY: &str = "tm25_exports";
/// IndexedDB key (via the loader's `tm25StashFile`) of the ray subset that
/// the virtual goniometer picks up.
pub const GONIO_STASH_KEY: &str = "gonio";
/// IndexedDB key of the same subset as a LightTools binary ray file, for the
/// shell's download button (binary, so not in the localStorage JSON).
pub const LIGHTTOOLS_STASH_KEY: &str = "lighttools";
/// IndexedDB key of the subset as a LightTools file with a wavelength per
/// ray drawn from the shared spectral table: the explicit second option,
/// since LightTools cannot reassign a spectrum that is hard-wired into rays.
pub const LIGHTTOOLS_SAMPLED_STASH_KEY: &str = "lighttools_sampled";

/// The spectrum to show and to export: the header's shared table, or a
/// flux-weighted 5 nm histogram of the kept rays' wavelengths for a file
/// with a wavelength per ray (spectral identifier 2).
pub fn spectrum_of(l: &LoadedRays) -> Option<eulumdat_tm25::SpectralTable> {
    l.header
        .shared_spectrum()
        .cloned()
        .or_else(|| eulumdat_tm25::spectrum_from_rays(&l.rays, l.flux_kind(), 5.0))
}

/// Files the shell can hand out for the loaded ray set: LDT / IES / ATLA of
/// the far field (the same smoothed grid the solid shows) and the header
/// spectrum as `.spd`. Also stashes the flux-rescaled ray subset as a small
/// TM-25 file so the virtual goniometer can replay the near field.
pub fn exports_json(l: &LoadedRays) -> String {
    let ff = l.far_field.smoothed(1, 1, 12.0);
    let e = eulumdat_tm25::exports(&ff, &l.header, Some((l.min, l.max)));
    let stem = l
        .name
        .rsplit_once('.')
        .map(|(a, _)| a.to_string())
        .unwrap_or_else(|| l.name.clone());
    serde_json::json!({
        "name": stem,
        "lamp_lumens": e.lamp_lumens,
        "ldt": e.ldt,
        "ies": e.ies,
        "atla_xml": e.atla_xml,
        "spd": e.spd,
        "rays_kept": l.rays.len(),
        "lighttools": eulumdat_tm25::LightToolsHeader::from_tm25(&l.header).is_ok(),
        // LightTools .sre beside the .ray: the spectrum stays reassignable there.
        "sre": spectrum_of(l).map(|t| eulumdat_tm25::write_sre(&t, &stem)),
        // The sampled-wavelength .ray exists only for a shared table + radiant flux.
        "lighttools_sampled": l.header.shared_spectrum().is_some() && l.header.flags.radiant_flux,
        "quality_txt": l.quality.to_text(&l.name),
    })
    .to_string()
}

/// The kept rays as a LightTools file with a wavelength per ray drawn from
/// the shared spectral table (see [`LIGHTTOOLS_SAMPLED_STASH_KEY`]). Empty
/// when the file has no table or no radiant flux.
pub fn subset_lighttools_sampled_bytes(l: &LoadedRays) -> Vec<u8> {
    eulumdat_tm25::to_lighttools_bytes_sampled(&l.header, &l.rays, 1).unwrap_or_default()
}

/// The kept rays as a LightTools binary ray file (`.ray`): radiometric or
/// photometric as the file is, with the wavelength per ray when it has one.
/// Empty when the header cannot be mapped (no flux column).
pub fn subset_lighttools_bytes(l: &LoadedRays) -> Vec<u8> {
    match eulumdat_tm25::to_lighttools_bytes(&l.header, &l.rays) {
        Ok(b) => b,
        Err(e) => {
            warn!("LightTools subset write failed: {e}");
            Vec::new()
        }
    }
}

/// The kept rays as a TM-25 file (header copied, flux already rescaled to the
/// file total by the reservoir).
pub fn subset_tm25_bytes(l: &LoadedRays) -> Vec<u8> {
    let mut buf = Vec::with_capacity(40_000 + l.rays.len() * 32);
    if let Err(e) = eulumdat_tm25::write_tm25(&mut buf, &l.header, &l.rays) {
        warn!("tm25 subset write failed: {e}");
        buf.clear();
    }
    buf
}

fn publish_exports(l: &LoadedRays) {
    let json = exports_json(l);
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = storage.set_item(EXPORTS_KEY, &json);
        }
        // Hand the subset to the goniometer (TM-25) and to the LightTools
        // download button (.ray) through the loader's IndexedDB stash.
        if let Some(window) = web_sys::window() {
            use wasm_bindgen::JsCast;
            if let Ok(f) = js_sys::Reflect::get(&window, &"tm25StashFile".into())
                .and_then(|f| f.dyn_into::<js_sys::Function>())
            {
                for (key, bytes) in [
                    (GONIO_STASH_KEY, subset_tm25_bytes(l)),
                    (LIGHTTOOLS_STASH_KEY, subset_lighttools_bytes(l)),
                    (
                        LIGHTTOOLS_SAMPLED_STASH_KEY,
                        subset_lighttools_sampled_bytes(l),
                    ),
                ] {
                    if !bytes.is_empty() {
                        let arr = js_sys::Uint8Array::from(bytes.as_slice());
                        let _ = f.call2(&window, &arr, &key.into());
                    }
                }
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        debug!("tm25 exports: {} bytes", json.len());
    }
}

fn publish_status(json: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = storage.set_item(STATUS_KEY, json);
            let _ = storage.set_item(STATUS_TS_KEY, &js_sys::Date::now().to_string());
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        debug!("tm25 status: {json}");
    }
}

// ---------------------------------------------------------------------------
// Settings from the shell
// ---------------------------------------------------------------------------

fn sync_settings(
    mut settings: ResMut<Tm25Settings>,
    mut sync: ResMut<SettingsSync>,
    mut data: ResMut<Tm25Data>,
) {
    #[cfg(target_arch = "wasm32")]
    {
        let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) else {
            return;
        };
        let Ok(Some(json)) = storage.get_item(SETTINGS_KEY) else {
            return;
        };
        if json == sync.last_json {
            return;
        }
        sync.last_json = json.clone();
        match serde_json::from_str::<Tm25Settings>(&json) {
            Ok(s) => {
                if s != *settings {
                    *settings = s;
                    data.dirty = true;
                }
            }
            Err(e) => warn!("tm25_settings: {e}"),
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (&mut settings, &mut data);
    }
    let _ = &mut sync;
}

fn keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<Tm25Settings>,
    mut data: ResMut<Tm25Data>,
) {
    let mut changed = false;
    if keys.just_pressed(KeyCode::Digit1) {
        settings.show_rays = !settings.show_rays;
        changed = true;
    }
    if keys.just_pressed(KeyCode::Digit2) {
        settings.show_points = !settings.show_points;
        changed = true;
    }
    if keys.just_pressed(KeyCode::Digit3) {
        settings.show_solid = !settings.show_solid;
        changed = true;
    }
    if keys.just_pressed(KeyCode::KeyC) {
        settings.color_mode = match settings.color_mode {
            ColorMode::Gamma => ColorMode::Height,
            ColorMode::Height => ColorMode::Flux,
            ColorMode::Flux => ColorMode::Spectrum,
            ColorMode::Spectrum => ColorMode::Gamma,
        };
        changed = true;
    }
    if keys.just_pressed(KeyCode::BracketRight) {
        settings.ray_length_mm = (settings.ray_length_mm * 1.5).min(50.0);
        changed = true;
    }
    if keys.just_pressed(KeyCode::BracketLeft) {
        settings.ray_length_mm = (settings.ray_length_mm / 1.5).max(0.5);
        changed = true;
    }
    if keys.just_pressed(KeyCode::KeyR) {
        settings.reset_camera = settings.reset_camera.wrapping_add(1);
    }
    if changed {
        data.dirty = true;
    }
}

// ---------------------------------------------------------------------------
// Scene
// ---------------------------------------------------------------------------

/// The ray-derived far field as a `PhotometricData` so the existing solid
/// mesh code draws it (γ = 0 on −Y, like an LDT).
#[derive(Clone, Debug)]
pub struct RaySolid {
    pub far_field: FarField,
    pub half_gamma_rad: f64,
}

impl PhotometricData for RaySolid {
    fn sample(&self, c_angle: f64, g_angle: f64) -> f64 {
        self.far_field.sample(c_angle, g_angle)
    }
    fn max_intensity(&self) -> f64 {
        self.far_field.max_intensity()
    }
    fn total_flux(&self) -> f64 {
        self.far_field.total_flux
    }
    fn light_output_ratio(&self) -> f64 {
        1.0
    }
    fn downward_fraction(&self) -> f64 {
        self.far_field.forward_fraction()
    }
    fn dimensions(&self) -> (f32, f32, f32) {
        (0.003, 0.003, 0.001)
    }
    fn color_temperature(&self) -> Option<f32> {
        None
    }
    fn cri(&self) -> Option<f32> {
        None
    }
    fn beam_angle(&self) -> f64 {
        self.half_gamma_rad
    }
}

fn spectrum_color(t: Option<&SpectralTable>) -> [f32; 3] {
    let Some(t) = t else {
        return [0.9, 0.9, 0.9];
    };
    if t.is_ultraviolet() {
        return [0.65, 0.25, 1.0];
    }
    let centroid = t.centroid_nm().unwrap_or(555.0);
    if centroid > 720.0 {
        return [0.75, 0.12, 0.12];
    }
    let spd = eulumdat_spectrum::Spd::new(
        &t.wavelengths_nm
            .iter()
            .map(|w| *w as f64)
            .collect::<Vec<_>>(),
        &t.values.iter().map(|v| *v as f64).collect::<Vec<_>>(),
    );
    let c = eulumdat_spectrum::colorimetry::analyze(&spd);
    // XYZ (Y = 1) → linear sRGB, then a soft normalisation.
    let (x, y, z) = if c.y_tri > 0.0 {
        (c.x_tri / c.y_tri, 1.0, c.z_tri / c.y_tri)
    } else {
        (0.95, 1.0, 1.09)
    };
    let r = 3.2406 * x - 1.5372 * y - 0.4986 * z;
    let g = -0.9689 * x + 1.8758 * y + 0.0415 * z;
    let b = 0.0557 * x - 0.2040 * y + 1.0570 * z;
    let m = r.max(g).max(b).max(1e-6);
    [
        (r / m).clamp(0.0, 1.0) as f32,
        (g / m).clamp(0.0, 1.0) as f32,
        (b / m).clamp(0.0, 1.0) as f32,
    ]
}

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn ray_color(r: &Ray, l: &LoadedRays, mode: ColorMode, spectrum: [f32; 3]) -> [f32; 3] {
    match mode {
        ColorMode::Gamma => {
            let (_, g) = r.c_gamma();
            lerp3([0.25, 0.45, 1.0], [1.0, 0.3, 0.1], (g / 90.0) as f32)
        }
        ColorMode::Height => {
            let span = (l.max[2] - l.min[2]).max(1e-6);
            lerp3([0.1, 0.8, 0.8], [1.0, 0.9, 0.2], (r.z - l.min[2]) / span)
        }
        ColorMode::Flux => {
            let t = if l.max_ray_flux > 0.0 {
                r.flux(l.flux_kind()) / l.max_ray_flux
            } else {
                1.0
            };
            lerp3([0.3, 0.3, 0.35], [1.0, 1.0, 1.0], t)
        }
        ColorMode::Spectrum => spectrum,
    }
}

fn line_mesh(positions: Vec<[f32; 3]>, colors: Vec<[f32; 4]>) -> Mesh {
    Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
}

fn additive_material() -> StandardMaterial {
    StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        alpha_mode: AlphaMode::Add,
        cull_mode: None,
        ..default()
    }
}

fn rebuild_scene(
    mut commands: Commands,
    mut data: ResMut<Tm25Data>,
    settings: Res<Tm25Settings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    existing: Query<Entity, With<Tm25Entity>>,
) {
    if !data.dirty {
        return;
    }
    data.dirty = false;
    for e in &existing {
        commands.entity(e).despawn();
    }
    // Irradiance plane first: it updates the status the shell shows.
    let plane = if settings.show_plane {
        data.loaded.as_ref().map(|l| {
            let half = if settings.plane_size_mm > 0.0 {
                settings.plane_size_mm
            } else {
                IrradianceGrid::auto_half_mm(l, settings.plane_z_mm)
            };
            IrradianceGrid::compute(l, settings.plane_z_mm, half, settings.plane_cells)
        })
    } else {
        None
    };
    if let Some(l) = data.loaded.as_ref() {
        publish_status(&status_json(l, plane.as_ref()));
    }
    data.irradiance = plane.clone();
    let Some(l) = data.loaded.as_ref() else {
        return;
    };
    if let Some(g) = &plane {
        let mut img = Image::new(
            Extent3d {
                width: g.n as u32,
                height: g.n as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            g.to_rgba(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );
        img.sampler = bevy::image::ImageSampler::nearest();
        let image = images.add(img);
        let material = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(image),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            double_sided: true,
            cull_mode: None,
            ..default()
        });
        // TM-25 plane z = d is Bevy y = −d; TM-25 x → X, y → Z. The mesh
        // UVs run u with +X and v with +Z, matching row = y, column = x.
        let size = 2.0 * g.half_mm;
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(size, size))),
            MeshMaterial3d(material),
            Transform::from_xyz(0.0, -g.z_mm, 0.0),
            Tm25Entity,
        ));
    }
    let spectrum = spectrum_color(l.header.shared_spectrum());
    let n = settings.display_count.min(l.rays.len());
    // Evenly strided subset of the (already random) analysis set.
    let stride = if n > 0 {
        l.rays.len() as f64 / n as f64
    } else {
        1.0
    };
    let subset: Vec<&Ray> = (0..n)
        .map(|i| &l.rays[(i as f64 * stride) as usize])
        .collect();

    // Additive blending sums every overlapping segment, so the alpha must
    // shrink with the count or dense bundles saturate to white: ~0.17 at 5k,
    // ~0.05 at 50k, floor at 0.03.
    let n_f = subset.len().max(1) as f32;
    let ray_alpha = (12.0 / n_f.sqrt()).clamp(0.03, 0.8);
    let point_alpha = (25.0 / n_f.sqrt()).clamp(0.05, 1.0);

    if settings.show_rays && !subset.is_empty() {
        let mut pos = Vec::with_capacity(subset.len() * 2);
        let mut col = Vec::with_capacity(subset.len() * 2);
        for r in &subset {
            let p0 = to_bevy(r.position());
            let d = to_bevy(r.direction());
            let len = settings.ray_length_mm;
            let p1 = [p0[0] + d[0] * len, p0[1] + d[1] * len, p0[2] + d[2] * len];
            let c = ray_color(r, l, settings.color_mode, spectrum);
            pos.push(p0);
            pos.push(p1);
            col.push([c[0], c[1], c[2], ray_alpha]);
            col.push([c[0], c[1], c[2], 0.0]);
        }
        commands.spawn((
            Mesh3d(meshes.add(line_mesh(pos, col))),
            MeshMaterial3d(materials.add(additive_material())),
            Tm25Entity,
        ));
    }

    if settings.show_points && !subset.is_empty() {
        let s = settings.point_size_mm.max(0.001);
        let mut pos = Vec::with_capacity(subset.len() * 6);
        let mut col = Vec::with_capacity(subset.len() * 6);
        for r in &subset {
            let p = to_bevy(r.position());
            let c = ray_color(r, l, settings.color_mode, spectrum);
            let c4 = [c[0], c[1], c[2], point_alpha];
            for axis in 0..3 {
                let mut a = p;
                let mut b = p;
                a[axis] -= s;
                b[axis] += s;
                pos.push(a);
                pos.push(b);
                col.push(c4);
                col.push(c4);
            }
        }
        commands.spawn((
            Mesh3d(meshes.add(line_mesh(pos, col))),
            MeshMaterial3d(materials.add(additive_material())),
            Tm25Entity,
        ));
    }

    if settings.show_solid {
        // Per-bin Monte Carlo noise folds a raw 10°×5° grid over itself
        // (back faces, sawtooth holes), so smooth it first and sample
        // bilinearly; the pole bins get their azimuthal average.
        let solid = RaySolid {
            far_field: l.far_field.smoothed(1, 1, 12.0),
            half_gamma_rad: l
                .far_field
                .half_intensity_gamma()
                .unwrap_or(60.0)
                .to_radians(),
        };
        let mesh = photometric_solid_mesh(
            &solid,
            PhotometricMeshResolution::High,
            settings.solid_scale_mm,
        );
        // Bevy sorts transparent *entities*, not the triangles inside a
        // mesh, and this mesh's index order runs column by column in C, so
        // ordinary alpha blending draws whole C sectors back-over-front and
        // they read as sawtooth holes. Alpha-to-coverage (with the default
        // 4× MSAA) is order independent and writes depth.
        let mut material = photometric_solid_material();
        material.alpha_mode = AlphaMode::AlphaToCoverage;
        material.cull_mode = None;
        material.double_sided = true;
        material.base_color = Color::srgba(1.0, 1.0, 1.0, 0.8);
        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(material)),
            Transform::from_translation(l.centre_bevy()),
            Tm25Entity,
        ));
    }
}

// ---------------------------------------------------------------------------
// Camera
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn orbit_camera(
    mut orbit: ResMut<Orbit>,
    settings: Res<Tm25Settings>,
    mut sync: ResMut<SettingsSync>,
    data: Res<Tm25Data>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    mut wheel: MessageReader<MouseWheel>,
    mut cam: Query<&mut Transform, With<Tm25Camera>>,
) {
    if settings.reset_camera != sync.last_reset {
        sync.last_reset = settings.reset_camera;
        let mut o = Orbit::default();
        if let Some(l) = data.loaded.as_ref() {
            o.target = l.centre_bevy();
            o.distance = l.extent_mm() * 4.0;
        }
        *orbit = o;
    }
    let mut delta = Vec2::ZERO;
    for m in motion.read() {
        delta += m.delta;
    }
    if buttons.pressed(MouseButton::Left) {
        orbit.yaw -= delta.x * 0.005;
        orbit.pitch = (orbit.pitch + delta.y * 0.005).clamp(-1.5, 1.5);
    } else if buttons.pressed(MouseButton::Right) || buttons.pressed(MouseButton::Middle) {
        let right = Vec3::new(orbit.yaw.cos(), 0.0, -orbit.yaw.sin());
        let up = Vec3::Y;
        let k = orbit.distance * 0.0015;
        orbit.target += (-right * delta.x + up * delta.y) * k;
    }
    for w in wheel.read() {
        let amount = match w.unit {
            MouseScrollUnit::Line => w.y * 0.1,
            MouseScrollUnit::Pixel => w.y * 0.005,
        };
        orbit.distance = (orbit.distance * (1.0 - amount).clamp(0.5, 2.0)).clamp(0.2, 5000.0);
    }
    let (sy, cy) = orbit.yaw.sin_cos();
    let (sp, cp) = orbit.pitch.sin_cos();
    let offset = Vec3::new(cp * sy, sp, cp * cy) * orbit.distance;
    for mut t in &mut cam {
        *t = Transform::from_translation(orbit.target + offset).looking_at(orbit.target, Vec3::Y);
    }
}

/// View 4: highlight rays that would reach the camera. Rebuilt whenever
/// the orbit direction moves by more than ~0.5° or the cone changes, over
/// the whole analysis set (200k rays), so it sharpens with more rays.
#[allow(clippy::too_many_arguments)]
fn camera_view(
    mut commands: Commands,
    data: Res<Tm25Data>,
    settings: Res<Tm25Settings>,
    orbit: Res<Orbit>,
    mut state: ResMut<CamViewState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    existing: Query<Entity, With<CamViewEntity>>,
) {
    let clear = |commands: &mut Commands| {
        for e in &existing {
            commands.entity(e).despawn();
        }
    };
    let Some(l) = data.loaded.as_ref() else {
        if state.last_dir.is_some() {
            clear(&mut commands);
            state.last_dir = None;
        }
        return;
    };
    if !settings.show_camera_view {
        if state.last_dir.is_some() {
            clear(&mut commands);
            state.last_dir = None;
            publish_camera_view(0, 0, 0.0);
            publish_luminance(None);
        }
        return;
    }
    // Direction from the source towards the camera, in the TM-25 frame.
    let (sy, cy) = orbit.yaw.sin_cos();
    let (sp, cp) = orbit.pitch.sin_cos();
    let v_bevy = Vec3::new(cp * sy, sp, cp * cy).normalize();
    let v = eulumdat_tm25::from_bevy(v_bevy.to_array());
    let cone = settings.camera_cone_deg.clamp(0.5, 60.0);
    let pixel = settings.luminance_pixel_mm.clamp(0.005, 5.0);
    let moved = match state.last_dir {
        Some(d) => (d[0] * v[0] + d[1] * v[1] + d[2] * v[2]) < (0.5f32).to_radians().cos(),
        None => true,
    };
    if !moved
        && (state.last_cone - cone).abs() < 1e-3
        && state.last_display == settings.display_count
        && (state.last_pixel - pixel).abs() < 1e-6
    {
        return;
    }
    state.last_dir = Some(v);
    state.last_cone = cone;
    state.last_display = settings.display_count;
    state.last_pixel = pixel;
    clear(&mut commands);
    // Luminance map of the kept rays from this direction (docs/luminance.md).
    publish_luminance(
        eulumdat_tm25::luminance_map(&l.rays, l.flux_kind(), v, cone, pixel, 96).as_ref(),
    );

    let cos_cone = cone.to_radians().cos();
    let s = settings.point_size_mm.max(0.001) * 2.0;
    let mut pos = Vec::new();
    let mut col = Vec::new();
    let mut n = 0usize;
    for r in &l.rays {
        if r.kx * v[0] + r.ky * v[1] + r.kz * v[2] >= cos_cone {
            n += 1;
            let p = to_bevy(r.position());
            for axis in 0..3 {
                let mut a = p;
                let mut b = p;
                a[axis] -= s;
                b[axis] += s;
                pos.push(a);
                pos.push(b);
                col.push([1.0, 0.95, 0.6, 0.9]);
                col.push([1.0, 0.95, 0.6, 0.9]);
            }
        }
    }
    state.last_count = n;
    publish_camera_view(n, l.rays.len(), cone);
    if !pos.is_empty() {
        commands.spawn((
            Mesh3d(meshes.add(line_mesh(pos, col))),
            MeshMaterial3d(materials.add(additive_material())),
            CamViewEntity,
        ));
    }
}

/// The luminance map as JSON in `localStorage["tm25_luminance"]` for the
/// shell: parameters, statistics and the row-major pixel values.
fn publish_luminance(m: Option<&eulumdat_tm25::LuminanceMap>) {
    #[cfg(target_arch = "wasm32")]
    {
        let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) else {
            return;
        };
        match m {
            None => {
                let _ = storage.remove_item("tm25_luminance");
            }
            Some(m) => {
                // 4 significant digits keep a 96 × 96 map under 100 KB.
                let values: Vec<f64> = m
                    .values
                    .iter()
                    .map(|v| {
                        if *v == 0.0 {
                            0.0
                        } else {
                            let e = 10f64.powi(3 - v.abs().log10().floor() as i32);
                            (v * e).round() / e
                        }
                    })
                    .collect();
                let json = serde_json::json!({
                    "unit": m.unit(),
                    "pixel_mm": m.pixel_mm,
                    "cone_deg": m.cone_deg,
                    "width": m.width,
                    "height": m.height,
                    "origin_mm": m.origin_mm,
                    "direction": m.direction,
                    "peak": m.peak,
                    "mean_lit": m.mean_lit,
                    "lit_pixels": m.lit_pixels,
                    "rays": m.rays,
                    "rays_per_lit_pixel": m.rays_per_lit_pixel,
                    "values": values,
                });
                let _ = storage.set_item("tm25_luminance", &json.to_string());
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(m) = m {
        debug!(
            "luminance map {}×{} px, peak {:.4} {}",
            m.width,
            m.height,
            m.peak,
            m.unit()
        );
    }
}

fn publish_camera_view(selected: usize, total: usize, cone: f32) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = storage.set_item(
                "tm25_camera_view",
                &format!("{{\"selected\":{selected},\"total\":{total},\"cone_deg\":{cone}}}"),
            );
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        debug!("camera view: {selected}/{total} within {cone}°");
    }
}

fn draw_gizmos(mut gizmos: Gizmos, data: Res<Tm25Data>) {
    // TM-25 axes in Bevy space: x → +X, y → +Z, z (emission) → −Y.
    gizmos.line(Vec3::ZERO, Vec3::X, Color::srgb(0.9, 0.2, 0.2));
    gizmos.line(Vec3::ZERO, Vec3::Z, Color::srgb(0.2, 0.9, 0.2));
    gizmos.line(Vec3::ZERO, -Vec3::Y, Color::srgb(0.3, 0.5, 1.0));
    if let Some(l) = data.loaded.as_ref() {
        // Bounding box of the start points, and a 1 mm scale bar beside it.
        let mn = Vec3::from(to_bevy(l.min));
        let mx = Vec3::from(to_bevy(l.max));
        let lo = mn.min(mx);
        let hi = mn.max(mx);
        let box_color = Color::srgba(1.0, 1.0, 1.0, 0.25);
        let corner = |i: usize| {
            Vec3::new(
                if i & 1 == 0 { lo.x } else { hi.x },
                if i & 2 == 0 { lo.y } else { hi.y },
                if i & 4 == 0 { lo.z } else { hi.z },
            )
        };
        for i in 0..8usize {
            for bit in [1usize, 2, 4] {
                if i & bit == 0 {
                    gizmos.line(corner(i), corner(i | bit), box_color);
                }
            }
        }
        let bar = Vec3::new(lo.x, lo.y - 0.3, hi.z + 0.3);
        gizmos.line(bar, bar + Vec3::X, Color::WHITE);
        gizmos.line(bar, bar + Vec3::Y * 0.1, Color::WHITE);
        gizmos.line(bar + Vec3::X, bar + Vec3::X + Vec3::Y * 0.1, Color::WHITE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::mesh::VertexAttributeValues;

    fn vendor_file() -> Option<Vec<u8>> {
        let home = std::env::var_os("HOME")?;
        let path = std::path::PathBuf::from(home).join(
            "Downloads/rayfile_SU_CULCP1_VC_20260512_IES_TM25/rayfile_SU_CULCP1_VC_100k_20260512_IES_TM25.TM25RAY",
        );
        std::fs::read(&path).ok()
    }

    /// A LightTools .ray written from a file loads through the same
    /// header + ingest path as the TM-25 subset it came from, and gives the
    /// same far field, flux and bounds: the round trip the viewer shows.
    #[test]
    fn lighttools_round_trip_loads_the_same() {
        let Some(bytes) = vendor_file() else {
            return;
        };
        let f = eulumdat_tm25::Tm25File::parse(&bytes).unwrap();
        let mut ingest = Ingest::new(f.header.clone(), 5_000, 1);
        ingest.feed(&bytes[f.header.ray_start..]);
        let l = ingest.finish("rayfile_100k.TM25RAY".into());

        let load = |bytes: &[u8], name: &str| {
            let h = parse_header_prefix(
                &bytes[..bytes.len().min(65_536)],
                name,
                Some(bytes.len() as u64),
            )
            .unwrap();
            eulumdat_tm25::check_file_size(&h, bytes.len() as u64).unwrap();
            let mut ing = Ingest::new(h.clone(), 5_000, 1);
            let end = h.ray_start + h.ray_block_size() as usize;
            for chunk in bytes[h.ray_start..end].chunks(10_007) {
                ing.feed(chunk);
            }
            ing.finish(name.into())
        };
        let a = load(&subset_tm25_bytes(&l), "subset.TM25RAY");
        let b = load(&subset_lighttools_bytes(&l), "subset.ray");
        assert!(b.header.is_lighttools());
        assert!(!a.header.is_lighttools());
        assert_eq!(a.ray_count, b.ray_count);
        assert_eq!(a.rays.len(), b.rays.len());
        assert_eq!(a.rays, b.rays);
        assert_eq!(a.min, b.min);
        assert_eq!(a.max, b.max);
        assert_eq!(a.header.flux_kind(), b.header.flux_kind());
        assert_eq!(a.header.radiant_flux_w, b.header.radiant_flux_w);
        let fa = a.far_field.smoothed(1, 1, 12.0);
        let fb = b.far_field.smoothed(1, 1, 12.0);
        assert_eq!(fa.half_intensity_gamma(), fb.half_intensity_gamma());
        assert_eq!(fa.max_intensity(), fb.max_intensity());
        assert_eq!(fa.azimuthal_average(), fb.azimuthal_average());
        assert_eq!(b.header.text.name(), "subset.ray");
    }

    /// The export bundle carries every format and the stashed subset is a
    /// valid TM-25 file whose flux still sums to the header total.
    #[test]
    fn exports_and_subset_round_trip() {
        let Some(bytes) = vendor_file() else {
            return;
        };
        let f = eulumdat_tm25::Tm25File::parse(&bytes).unwrap();
        let mut ingest = Ingest::new(f.header.clone(), 5_000, 1);
        for chunk in bytes[f.header.ray_start..].chunks(65_536) {
            ingest.feed(chunk);
        }
        let l = ingest.finish("rayfile_100k.TM25RAY".into());
        assert_eq!(l.rays.len(), 5_000);

        let json = exports_json(&l);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["name"], "rayfile_100k");
        assert_eq!(v["rays_kept"], 5_000);
        let ldt = eulumdat::Eulumdat::parse(v["ldt"].as_str().unwrap()).unwrap();
        assert_eq!(ldt.c_angles.len(), 36);
        assert!(v["ies"].as_str().unwrap().contains("LM-63"));
        assert!(v["atla_xml"]
            .as_str()
            .unwrap()
            .contains("SpectralDistribution"));
        assert!(v["spd"].as_str().unwrap().contains("266"));

        let sub = subset_tm25_bytes(&l);
        let back = eulumdat_tm25::Tm25File::parse(&sub).unwrap();
        assert_eq!(back.header.n_rays, 5_000);
        assert_eq!(back.header.text.name(), f.header.text.name());
        let kind = back.header.flux_kind();
        let flux: f64 = back.rays.iter().map(|r| r.flux(kind) as f64).sum();
        let total = f.header.radiant_flux_w as f64;
        assert!((flux - total).abs() / total < 0.01, "{flux} vs {total}");
    }

    #[test]
    fn solid_mesh_has_no_collapsed_sector() {
        let Some(home) = std::env::var_os("HOME") else {
            return;
        };
        let path = std::path::PathBuf::from(home).join(
            "Downloads/rayfile_SU_CULCP1_VC_20260512_IES_TM25/rayfile_SU_CULCP1_VC_100k_20260512_IES_TM25.TM25RAY",
        );
        let Ok(bytes) = std::fs::read(&path) else {
            return;
        };
        let f = eulumdat_tm25::Tm25File::parse(&bytes).unwrap();
        let rays: Vec<Ray> = f.rays.iter().collect();
        let ff = FarField::from_rays(&rays, 10.0, 5.0, FluxKind::Radiometric);
        let solid = RaySolid {
            far_field: ff.smoothed(1, 1, 12.0),
            half_gamma_rad: 1.0,
        };
        let mesh = photometric_solid_mesh(&solid, PhotometricMeshResolution::High, 8.0);
        let Some(VertexAttributeValues::Float32x3(pos)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("no positions");
        };
        let (num_c, num_g) = (72usize, 73usize);
        assert_eq!(pos.len(), num_c * num_g);
        // Radius per column at γ = 45° (gi = 18) must be uniform-ish.
        let radii: Vec<f32> = (0..num_c)
            .map(|ci| {
                let p = pos[ci * num_g + 18];
                (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt()
            })
            .collect();
        let (mn, mx) = radii
            .iter()
            .fold((f32::MAX, f32::MIN), |(a, b), &r| (a.min(r), b.max(r)));
        println!("γ=45°: radius min {mn:.3} max {mx:.3}");
        assert!(mn > 0.5 * mx, "collapsed sector: {radii:?}");
        // Adjacent vertices along C must stay close (no spikes).
        let mut worst = 0.0f32;
        for gi in 1..num_g - 1 {
            for ci in 0..num_c {
                let a = pos[ci * num_g + gi];
                let b = pos[((ci + 1) % num_c) * num_g + gi];
                let d =
                    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
                worst = worst.max(d);
            }
        }
        println!("longest C-neighbour edge {worst:.3} mm");
        assert!(worst < 2.0, "spike edge {worst}");
        let idx = mesh.indices().expect("indices").len();
        println!("indices {idx}");
        let nan: Vec<(usize, usize)> = (0..num_c)
            .flat_map(|ci| (0..num_g).map(move |gi| (ci, gi)))
            .filter(|&(ci, gi)| pos[ci * num_g + gi].iter().any(|v| !v.is_finite()))
            .collect();
        println!("non-finite vertices: {}", nan.len());
        assert!(nan.is_empty(), "{:?}", &nan[..nan.len().min(20)]);
        if let Some(VertexAttributeValues::Float32x3(nrm)) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        {
            let bad = nrm
                .iter()
                .filter(|n| n.iter().any(|v| !v.is_finite()))
                .count();
            println!("non-finite normals: {bad}");
        }
        if let Some(VertexAttributeValues::Float32x4(col)) = mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
            let bad = col
                .iter()
                .filter(|c| c.iter().any(|v| !v.is_finite()))
                .count();
            println!("non-finite colours: {bad}");
        }
    }
}
