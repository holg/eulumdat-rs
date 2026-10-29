//! The viewer state machine: settings, the loaded luminaire, the built
//! scene, the camera, exposure and the accumulation tokens, turned into one
//! [`FrameInput`] per frame for whichever GPU shell renders it.

use crate::camera::OrbitCamera;
use crate::gpu::{
    GpuLamp, GpuPrim, Params, MAX_SAMPLES, PRIM_CAPSULE, PRIM_CYLINDER, PRIM_OBB, PRIM_SPHERE,
    PROBE_MAX_SAMPLES,
};
use crate::measure::{
    format_lux, heat_ramp, keep_at_level, lod_level, lux_ramp_t, measurement_layout,
    probe_buffer_data, MeasureLayout,
};
use crate::photometry::{
    camera_key_lux, estimate_illuminance, exposure_for, kelvin_to_linear_rgb, luminaire_shape,
    nice_ceil, IesTable, LuminaireShape,
};
use crate::scene::{
    auto_rotation, build_scene, hidden_from, segment_blocked, BuiltScene, SceneKind, SceneParams,
    Shape, StreetLayout, WallFinish, ROAD_HALF_WIDTH,
};
use crate::sync::{scene_from_id, GridStat, Status, SyncSettings};
use eulumdat::{Eulumdat, PhotometricCalculations, ThrowDirection};
use glam::{UVec2, UVec4, Vec3, Vec4};
use std::sync::Arc;

use crate::mesh::LuminaireMesh;

/// Night sky radiance of a lit city, cd/m².
const NIGHT_SKY_RADIANCE: f32 = 0.01;
/// Used when a file does not declare its own colour temperature.
pub const DEFAULT_CCT: f32 = 3000.0;
const MAX_PIXEL_RATIO: f32 = 1.25;
const MAX_PIXELS: f32 = 1.7e6;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    Render,
    Isolux,
    Points,
}

impl ViewMode {
    pub fn next(self) -> Self {
        match self {
            ViewMode::Render => ViewMode::Isolux,
            ViewMode::Isolux => ViewMode::Points,
            ViewMode::Points => ViewMode::Render,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            ViewMode::Render => "Photo",
            ViewMode::Isolux => "Illuminance",
            ViewMode::Points => "Grid",
        }
    }
    pub fn id(self) -> &'static str {
        match self {
            ViewMode::Render => "render",
            ViewMode::Isolux => "isolux",
            ViewMode::Points => "points",
        }
    }
    pub fn from_id(id: &str) -> Self {
        match id {
            "isolux" => ViewMode::Isolux,
            "points" => ViewMode::Points,
            _ => ViewMode::Render,
        }
    }
    fn gpu_id(self) -> u32 {
        match self {
            ViewMode::Render => 0,
            ViewMode::Isolux => 1,
            ViewMode::Points => 2,
        }
    }
}

/// User-facing settings. Changing them re-renders; the camera is separate.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub mode: ViewMode,
    pub isolines: bool,
    /// 0..1 haze density slider.
    pub haze: f32,
    pub glare: bool,
    pub exposure_ev: f32,
    pub cct: f32,
    /// Multiplier on candela output.
    pub dimming: f32,
    pub bounces: u32,
    pub params: SceneParams,
    pub auto_rotate: bool,
    pub show_help: bool,
    /// Draw status, help, legend and statistics on the canvas (native shells).
    pub overlay: bool,
    /// Keep tracing. A host pauses the viewer while it is hidden.
    pub active: bool,
    /// Samples per pixel before the picture is considered done.
    pub max_samples: u32,
    /// Samples per measurement point before the points are considered done.
    pub probe_max_samples: u32,
    /// Trace passes per frame: faster convergence on a strong GPU.
    pub passes_per_frame: u32,
    /// µmol per (lx·s) of the loaded spectrum, from the host; `None` assumes
    /// a white LED in PPFD scenes.
    pub ppfd_factor: Option<f32>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: ViewMode::Render,
            isolines: false,
            haze: 0.05,
            glare: true,
            exposure_ev: 0.0,
            cct: DEFAULT_CCT,
            dimming: 1.0,
            bounces: 3,
            params: SceneParams::defaults(SceneKind::Street),
            auto_rotate: true,
            show_help: true,
            overlay: true,
            active: true,
            max_samples: MAX_SAMPLES,
            probe_max_samples: PROBE_MAX_SAMPLES,
            passes_per_frame: 1,
            ppfd_factor: None,
        }
    }
}

/// A sample file shipped with the viewer.
pub struct Sample {
    pub name: &'static str,
    pub content: &'static str,
    pub scene: SceneKind,
}

macro_rules! template {
    ($f:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../eulumdat-wasm/templates/",
            $f
        ))
    };
}

pub const SAMPLES: &[Sample] = &[
    Sample {
        name: "Road luminaire",
        content: template!("road_luminaire.ldt"),
        scene: SceneKind::Street,
    },
    Sample {
        name: "Flood",
        content: template!("wiki-flood.ldt"),
        scene: SceneKind::Facade,
    },
    Sample {
        name: "Spotlight",
        content: template!("wiki-spotlight.ldt"),
        scene: SceneKind::Parking,
    },
    Sample {
        name: "Batwing",
        content: template!("wiki-batwing.ldt"),
        scene: SceneKind::Office,
    },
    Sample {
        name: "Fluorescent",
        content: template!("fluorescent_luminaire.ldt"),
        scene: SceneKind::Office,
    },
    Sample {
        name: "Projector",
        content: template!("projector.ldt"),
        scene: SceneKind::Parking,
    },
];

/// Parse an LDT or IES file by its name's extension.
pub fn parse_any(name: &str, content: &str) -> Option<Eulumdat> {
    if name.to_lowercase().ends_with(".ies") {
        eulumdat::IesParser::parse(content).ok()
    } else {
        Eulumdat::parse(content).ok()
    }
}

/// The luminaire currently loaded, with everything derived from it.
#[derive(Clone, Debug)]
pub struct Photometry {
    pub ldt: Eulumdat,
    pub name: String,
    pub table: IesTable,
    pub throw: ThrowDirection,
    pub cct_from_file: Option<f32>,
    /// Bumped on every load; a shell re-uploads the table when it changes.
    pub version: u64,
}

impl Photometry {
    pub fn new(name: &str, ldt: Eulumdat, version: u64) -> Self {
        let file_name = name.rsplit(['/', '\\']).next().unwrap_or(name);
        let name = match ldt.luminaire_name.trim() {
            "" => file_name,
            n => n,
        };
        let table = IesTable::from_eulumdat(&ldt);
        let throw = PhotometricCalculations::throw_direction(&ldt);
        let cct_from_file = ldt
            .lamp_sets
            .first()
            .and_then(|l| parse_color_temperature(&l.color_appearance));
        Self {
            name: name.to_string(),
            ldt,
            table,
            throw,
            cct_from_file,
            version,
        }
    }
}

/// Colour temperature from a lamp's colour appearance ("3000K", "LED 4000 Kelvin").
pub fn parse_color_temperature(appearance: &str) -> Option<f32> {
    let digits: String = appearance.chars().filter(|c| c.is_ascii_digit()).collect();
    let mut best: Option<f32> = None;
    for w in appearance.split(|c: char| !c.is_ascii_digit()) {
        if w.len() == 4 {
            if let Ok(k) = w.parse::<f32>() {
                if (1500.0..=20000.0).contains(&k) {
                    best = Some(k);
                    break;
                }
            }
        }
    }
    if best.is_none() && digits.len() == 4 {
        best = digits
            .parse::<f32>()
            .ok()
            .filter(|k| (1500.0..=20000.0).contains(k));
    }
    best
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraMode {
    Keep,
    Animate,
    Jump,
}

/// What a shell uploads and dispatches for one frame.
#[derive(Clone, Debug)]
pub struct FrameInput {
    pub params: Params,
    pub prims: Vec<GpuPrim>,
    pub lamps: Vec<GpuLamp>,
    /// Position / normal pairs; `probes_version` bumps when they change.
    pub probes: Arc<Vec<[f32; 8]>>,
    pub probes_version: u64,
    /// Bumps when the IES table must be re-uploaded.
    pub ies_version: u64,
    /// Luminaire body mesh (per-lamp instance); `mesh_version` bumps when it changes.
    pub mesh: Option<Arc<LuminaireMesh>>,
    pub mesh_version: u64,
    pub size: UVec2,
    /// Bumped whenever the picture must start over.
    pub reset_token: u64,
    /// Bumped whenever the point values must start over.
    pub probe_reset_token: u64,
    pub max_samples: u32,
    pub probe_max_samples: u32,
    pub passes_per_frame: u32,
    pub trace: bool,
    pub probe: bool,
}

/// A measurement point drawn by the overlay: `labelled` ones carry the lux
/// value in a pill of the ramp colour, the rest are small dots.
#[derive(Clone, Debug)]
pub struct OverlayPoint {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub text: Option<String>,
    /// sRGB 0..1.
    pub color: [f32; 3],
    /// Light text on the dark end of the ramp.
    pub light_text: bool,
}

/// Keyboard-style commands for native shells.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    Scene(SceneKind),
    NextMode,
    ToggleIsolines,
    ToggleGlare,
    Haze(f32),
    Exposure(f32),
    Height(f32),
    Rotation(f32),
    Tilt(f32),
    CycleCount,
    CycleLayout,
    ToggleFinish,
    ToggleFigure,
    Spacing(f32),
    Ceiling(f32),
    Cct(f32),
    CycleBounces,
    ToggleHelp,
    ResetView,
    NextSample,
}

/// White LED, 4000 K: µmol of PAR photons per lux-second. Used when the
/// luminaire carries no spectrum.
pub const DEFAULT_PPFD_PER_LUX: f32 = 0.0152;

pub struct Viewer {
    pub settings: Settings,
    pub photometry: Option<Photometry>,
    pub scene: BuiltScene,
    pub camera: OrbitCamera,
    pub shape: LuminaireShape,
    pub layout: MeasureLayout,
    scene_dirty: bool,
    camera_mode: CameraMode,
    pub exposure: f32,
    pub lux_max: f32,
    pub iso_step: f32,
    peak_lux: f32,
    lamp_glare: Vec<f32>,
    glare_dirty: bool,
    pub reset_token: u64,
    pub probe_reset_token: u64,
    pub probes_version: u64,
    pub probes: Arc<Vec<[f32; 8]>>,
    /// Body of the luminaire from an L3D model, else the housing box is drawn.
    pub mesh: Option<Arc<LuminaireMesh>>,
    pub mesh_version: u64,
    pub size: UVec2,
    /// Upper bound on traced pixels; a CPU host lowers it to preview size.
    pub max_pixels: f32,
    render_scale: f32,
    slow_frames: u32,
    last_reset_view: u64,
    /// Latest probe values from the GPU, in `layout.points` order.
    pub probe_values: Vec<f32>,
    /// Mirrors of the shell's accumulation counters, for status and progress.
    pub samples: u32,
    pub probe_samples: u32,
    last_photometry_version: u64,
    next_photometry_version: u64,
    sample_index: usize,
}

impl Default for Viewer {
    fn default() -> Self {
        Self::new(Settings::default())
    }
}

impl Viewer {
    pub fn new(settings: Settings) -> Self {
        let scene = build_scene(&settings.params);
        let layout = measurement_layout(&scene);
        let probes = Arc::new(probe_buffer_data(&layout));
        let mut camera = OrbitCamera::default();
        camera.limits = scene.limits;
        camera.set_view(scene.view, false);
        Self {
            settings,
            photometry: None,
            scene,
            camera,
            shape: LuminaireShape {
                half_x: 0.1,
                half_z: 0.1,
                round: true,
                housing_half: Vec3::new(0.1, 0.04, 0.1),
            },
            layout,
            scene_dirty: true,
            camera_mode: CameraMode::Jump,
            exposure: 1.0,
            lux_max: 100.0,
            iso_step: 10.0,
            peak_lux: 100.0,
            lamp_glare: Vec::new(),
            glare_dirty: true,
            reset_token: 1,
            probe_reset_token: 1,
            probes_version: 1,
            probes,
            mesh: None,
            mesh_version: 1,
            size: UVec2::new(8, 8),
            max_pixels: MAX_PIXELS,
            render_scale: 1.0,
            slow_frames: 0,
            last_reset_view: 0,
            probe_values: Vec::new(),
            samples: 0,
            probe_samples: 0,
            last_photometry_version: 0,
            next_photometry_version: 1,
            sample_index: 0,
        }
    }

    /// Load a luminaire. `scene` switches to that scene when it differs.
    pub fn load(&mut self, name: &str, ldt: Eulumdat, scene: Option<SceneKind>) {
        let version = self.next_photometry_version;
        self.next_photometry_version += 1;
        let photometry = Photometry::new(name, ldt, version);
        self.settings.cct = photometry.cct_from_file.unwrap_or(DEFAULT_CCT);
        self.settings.exposure_ev = 0.0;
        self.settings.auto_rotate = true;
        if let Some(kind) = scene {
            if kind != self.settings.params.kind {
                self.settings.params = SceneParams::defaults(kind);
                self.camera_mode = CameraMode::Animate;
            }
        }
        self.photometry = Some(photometry);
        self.scene_dirty = true;
    }

    /// Load `SAMPLES[index]`.
    pub fn load_sample(&mut self, index: usize) {
        let s = &SAMPLES[index % SAMPLES.len()];
        self.sample_index = index % SAMPLES.len();
        if let Some(ldt) = parse_any(s.name, s.content) {
            self.load(s.name, ldt, Some(s.scene));
        }
    }

    /// Change settings; the viewer works out what must be rebuilt or restarted.
    pub fn edit(&mut self, f: impl FnOnce(&mut Settings)) {
        let old = self.settings.clone();
        f(&mut self.settings);
        self.after_edit(&old);
    }

    fn after_edit(&mut self, old: &Settings) {
        let s = &self.settings;
        if s.params != old.params || s.dimming != old.dimming {
            if s.params.kind != old.params.kind {
                self.camera_mode = CameraMode::Animate;
            }
            self.scene_dirty = true;
        }
        if s.haze != old.haze
            || s.glare != old.glare
            || s.cct != old.cct
            || s.bounces != old.bounces
            || s.exposure_ev != old.exposure_ev && false
        {
            self.reset_token += 1;
        }
        if s.bounces != old.bounces {
            self.probe_reset_token += 1;
        }
        if s.glare != old.glare {
            self.glare_dirty = true;
        }
    }

    pub fn switch_scene(&mut self, kind: SceneKind) {
        if kind == self.settings.params.kind {
            return;
        }
        let housing = self.settings.params.housing_half;
        self.settings.params = SceneParams::defaults(kind);
        self.settings.params.housing_half = housing;
        self.camera_mode = CameraMode::Animate;
        self.scene_dirty = true;
    }

    pub fn command(&mut self, cmd: Command) {
        let old = self.settings.clone();
        let p = &mut self.settings.params;
        match cmd {
            Command::Scene(kind) => {
                self.switch_scene(kind);
                return;
            }
            Command::NextMode => {
                self.settings.mode = self.settings.mode.next();
                self.settings.isolines = self.settings.mode == ViewMode::Isolux;
            }
            Command::ToggleIsolines => self.settings.isolines = !self.settings.isolines,
            Command::ToggleGlare => self.settings.glare = !self.settings.glare,
            Command::Haze(d) => self.settings.haze = (self.settings.haze + d).clamp(0.0, 1.0),
            Command::Exposure(d) => {
                self.settings.exposure_ev = (self.settings.exposure_ev + d).clamp(-4.0, 4.0)
            }
            Command::Height(d) => {
                let (lo, hi) = p.height_range();
                p.height = (p.height + d).clamp(lo, hi);
            }
            Command::Rotation(d) => {
                p.rotation = (p.rotation + d + 180.0).rem_euclid(360.0) - 180.0;
                self.settings.auto_rotate = false;
            }
            Command::Tilt(d) => p.tilt = (p.tilt + d).clamp(-90.0, 90.0),
            Command::CycleCount => {
                let opts = p.count_options();
                let i = opts.iter().position(|&o| o == p.count).unwrap_or(0);
                p.count = opts[(i + 1) % opts.len()];
            }
            Command::CycleLayout => {
                if p.kind == SceneKind::Street {
                    p.street_layout = p.street_layout.next();
                }
            }
            Command::ToggleFinish => {
                if matches!(p.kind, SceneKind::Street | SceneKind::Facade) {
                    p.wall_finish = match p.wall_finish {
                        WallFinish::Render => WallFinish::Brick,
                        WallFinish::Brick => WallFinish::Render,
                    };
                }
            }
            Command::ToggleFigure => p.show_figure = !p.show_figure,
            Command::Spacing(dir) => {
                let (lo, hi, step) = match p.kind {
                    SceneKind::Street => (12.0, 60.0, 2.0),
                    SceneKind::Parking => (8.0, 40.0, 2.0),
                    _ => (0.5, 12.0, 0.5),
                };
                p.spacing = (p.spacing + dir * step).clamp(lo, hi);
            }
            Command::Ceiling(d) => match p.kind {
                SceneKind::Office => {
                    p.ceiling_height = (p.ceiling_height + d).clamp(2.4, 4.5);
                    let (_, hi) = p.height_range();
                    p.height = p.height.min(hi);
                }
                SceneKind::Facade => p.wall_distance = (p.wall_distance + d).clamp(0.2, 6.0),
                _ => {}
            },
            Command::Cct(d) => self.settings.cct = (self.settings.cct + d).clamp(2000.0, 6500.0),
            Command::CycleBounces => self.settings.bounces = self.settings.bounces % 6 + 1,
            Command::ToggleHelp => self.settings.show_help = !self.settings.show_help,
            Command::ResetView => {
                self.reset_view();
                return;
            }
            Command::NextSample => {
                let next = (self.sample_index + 1) % SAMPLES.len();
                self.load_sample(next);
                return;
            }
        }
        self.after_edit(&old);
    }

    pub fn reset_view(&mut self) {
        self.camera.limits = self.scene.limits;
        self.camera.set_view(self.scene.view, true);
    }

    // ----- camera input -----

    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.camera.orbit(dx, dy);
        self.camera_changed();
    }

    pub fn pan(&mut self, dx: f32, dy: f32, css_height: f32) {
        self.camera.pan(dx, dy, css_height);
        self.camera_changed();
    }

    pub fn zoom(&mut self, factor: f32) {
        self.camera.zoom(factor);
        self.camera_changed();
    }

    fn camera_changed(&mut self) {
        self.glare_dirty = true;
        self.reset_token += 1;
    }

    /// Render size for a canvas of `css_w × css_h` CSS pixels at `dpr`,
    /// capped like the web viewer; returns true when it changed.
    pub fn resize(&mut self, css_w: f32, css_h: f32, dpr: f32) -> bool {
        let css_w = css_w.max(1.0);
        let css_h = css_h.max(1.0);
        let mut pr = dpr.clamp(0.5, MAX_PIXEL_RATIO) * self.render_scale;
        if css_w * css_h * pr * pr > self.max_pixels {
            pr = (self.max_pixels / (css_w * css_h)).sqrt();
        }
        let size = UVec2::new(
            (css_w * pr).round().max(1.0) as u32,
            (css_h * pr).round().max(1.0) as u32,
        );
        if size != self.size {
            self.size = size;
            self.reset_token += 1;
            true
        } else {
            false
        }
    }

    /// Lower the internal resolution when frames get slow.
    pub fn note_frame_time(&mut self, dt_ms: f32) {
        if dt_ms > 48.0 {
            self.slow_frames += 1;
        } else {
            self.slow_frames = self.slow_frames.saturating_sub(1);
        }
        if self.slow_frames > 10 && self.render_scale > 0.55 {
            self.slow_frames = 0;
            self.render_scale *= 0.85;
        }
    }

    /// Advance the camera, rebuild the scene if needed, refresh exposure and glare.
    /// Returns true when the scene was rebuilt (grids and probes changed).
    pub fn update(&mut self, dt_secs: f32) -> bool {
        if self.camera.update(dt_secs) {
            self.camera_changed();
        }
        let rebuilt = if self.scene_dirty {
            self.rebuild_scene();
            true
        } else {
            false
        };
        let key = camera_key_lux(&self.scene);
        self.exposure = exposure_for(key, self.settings.exposure_ev);
        if self.glare_dirty {
            self.glare_dirty = false;
            self.lamp_glare = self.compute_glare();
        }
        rebuilt
    }

    /// Factor from lux to the displayed unit: µmol/(m²·s) per lx in the
    /// greenhouse, 0 elsewhere (values stay lux).
    pub fn display_factor(&self) -> f32 {
        if self.settings.params.kind == SceneKind::Greenhouse {
            self.settings.ppfd_factor.unwrap_or(DEFAULT_PPFD_PER_LUX)
        } else {
            0.0
        }
    }

    /// Replace the luminaire body (`None` goes back to the housing box).
    /// The scene is rebuilt because arms and the emitting face follow it.
    pub fn set_mesh(&mut self, mesh: Option<Arc<LuminaireMesh>>) {
        // A body reaching more than a metre below the emitting face is a
        // free-standing luminaire: it stands on the ground at its own height.
        let standing = mesh.as_ref().is_some_and(|m| m.bmin.y < -1.0);
        self.settings.params.freestanding = standing;
        if let Some(m) = mesh.as_ref().filter(|_| standing) {
            self.settings.params.height = -m.bmin.y;
        }
        self.mesh = mesh;
        self.mesh_version += 1;
        self.scene_dirty = true;
        self.reset_token += 1;
        self.probe_reset_token += 1;
    }

    fn rebuild_scene(&mut self) {
        self.scene_dirty = false;
        let Some(photometry) = self.photometry.as_ref() else {
            return;
        };
        let fresh_luminaire = photometry.version != self.last_photometry_version;
        self.last_photometry_version = photometry.version;

        self.shape = luminaire_shape(&photometry.ldt, self.settings.params.kind);
        if let Some(m) = self.mesh.as_ref() {
            if let Some((hx, hz)) = m.face_half {
                self.shape.half_x = hx;
                self.shape.half_z = hz;
                self.shape.round = m.face_round;
            }
            let ext = m.bmax - m.bmin;
            self.shape.housing_half = Vec3::new(
                (0.5 * ext.x).max(0.02),
                (0.5 * m.bmax.y.max(0.02)).max(0.01),
                (0.5 * ext.z).max(0.02),
            );
        }
        self.settings.params.housing_half = self.shape.housing_half;
        if self.settings.auto_rotate {
            self.settings.params.rotation = auto_rotation(
                self.settings.params.kind,
                photometry.throw.c as f32,
                photometry.throw.asymmetry as f32,
            );
        }
        let (_, hi) = self.settings.params.height_range();
        self.settings.params.height = self.settings.params.height.min(hi);

        let scene = build_scene(&self.settings.params);
        let layout = measurement_layout(&scene);
        let probes = Arc::new(probe_buffer_data(&layout));
        if probes.len() != self.probes.len() || fresh_luminaire {
            self.probe_values.clear();
        }
        self.layout = layout;
        self.probes = probes;
        self.probes_version += 1;
        self.probe_reset_token += 1;

        self.camera.limits = scene.limits;
        match self.camera_mode {
            CameraMode::Keep => {}
            CameraMode::Animate => self.camera.set_view(scene.view, true),
            CameraMode::Jump => self.camera.set_view(scene.view, false),
        }
        self.camera_mode = CameraMode::Keep;
        self.scene = scene;

        let (max, _) = estimate_illuminance(&self.scene, &photometry.table, self.settings.dimming);
        let key = camera_key_lux(&self.scene);
        self.exposure = exposure_for(key, self.settings.exposure_ev);
        let mount = self
            .scene
            .lamps
            .iter()
            .fold(1.0_f32, |h, l| h.max(l.position.y));
        let anchor_lux = photometry.table.max_cd * self.settings.dimming / (mount * mount).max(1.0);
        self.peak_lux = max.max(anchor_lux).max(1e-3);
        let f = self.display_factor();
        if f > 0.0 {
            self.lux_max = nice_ceil(max.max(1e-3) * 1.02 * f) / f;
            self.iso_step = nice_ceil(self.lux_max * f / 10.0) / f;
        } else {
            self.lux_max = nice_ceil(max.max(1e-3) * 1.02);
            self.iso_step = nice_ceil(self.lux_max / 10.0);
        }
        self.glare_dirty = true;
        self.reset_token += 1;
    }

    fn compute_glare(&self) -> Vec<f32> {
        let Some(photometry) = self.photometry.as_ref() else {
            return Vec::new();
        };
        let eye = self.camera.eye();
        let has_cutaway = self.scene.prims.iter().any(|b| b.cutaway.is_some());
        self.scene
            .lamps
            .iter()
            .map(|lamp| {
                if !self.settings.glare {
                    return 0.0;
                }
                let d = eye - lamp.position;
                let d2 = d.length_squared();
                let dist = d2.sqrt();
                if dist < 0.2 {
                    return 0.0;
                }
                let i =
                    photometry.table.sample_dir(&lamp.rotation, d / dist) * self.settings.dimming;
                if i <= 0.0 {
                    return 0.0;
                }
                let hidden = !has_cutaway && segment_blocked(&self.scene, eye, lamp.position);
                if hidden {
                    0.0
                } else {
                    i / d2
                }
            })
            .collect()
    }

    /// Everything a shell needs for this frame.
    pub fn frame_input(&self) -> FrameInput {
        let eye = self.camera.eye();
        let aspect = self.size.x as f32 / self.size.y.max(1) as f32;
        let (_, inv_vp) = self.camera.matrices(aspect);
        let s = &self.settings;
        let tint = kelvin_to_linear_rgb(s.cct);
        let tint_luma = (0.2126 * tint.x + 0.7152 * tint.y + 0.0722 * tint.z).max(1e-4);
        let shape = self.shape;
        let (c_size, g_size, ies_version) = self
            .photometry
            .as_ref()
            .map(|p| (p.table.c_size, p.table.g_size, p.version))
            .unwrap_or((1, 1, 0));

        let prims: Vec<GpuPrim> = self
            .scene
            .prims
            .iter()
            .map(|b| {
                let mut flags = if b.casts_shadow { 1 } else { 0 };
                if hidden_from(b, eye) {
                    flags |= 2;
                }
                let mut g = GpuPrim {
                    material: b.material,
                    flags,
                    ..Default::default()
                };
                let cols = |m: &glam::Mat3| {
                    (
                        m.x_axis.extend(0.0).to_array(),
                        m.y_axis.extend(0.0).to_array(),
                        m.z_axis.extend(0.0).to_array(),
                    )
                };
                match &b.shape {
                    Shape::Box { min, max } => {
                        g.kind = PRIM_OBB;
                        g.p0 = ((*min + *max) * 0.5).extend(0.0).to_array();
                        g.p1 = ((*max - *min) * 0.5).extend(0.0).to_array();
                        (g.r0, g.r1, g.r2) = cols(&glam::Mat3::IDENTITY);
                    }
                    Shape::Obb { center, half, rot } => {
                        g.kind = PRIM_OBB;
                        g.p0 = center.extend(0.0).to_array();
                        g.p1 = half.extend(0.0).to_array();
                        (g.r0, g.r1, g.r2) = cols(rot);
                    }
                    Shape::Sphere { center, radius } => {
                        g.kind = PRIM_SPHERE;
                        g.p0 = center.extend(*radius).to_array();
                    }
                    Shape::Capsule { a, b, radius } => {
                        g.kind = PRIM_CAPSULE;
                        g.p0 = a.extend(*radius).to_array();
                        g.p1 = b.extend(0.0).to_array();
                    }
                    Shape::Cylinder { a, b, radius } => {
                        g.kind = PRIM_CYLINDER;
                        g.p0 = a.extend(*radius).to_array();
                        g.p1 = b.extend(0.0).to_array();
                    }
                }
                g
            })
            .collect();
        let lamps: Vec<GpuLamp> = self
            .scene
            .lamps
            .iter()
            .enumerate()
            .map(|(i, l)| {
                let r = l.rotation;
                let glare = self.lamp_glare.get(i).copied().unwrap_or(0.0);
                GpuLamp {
                    pos: [l.position.x, l.position.y, l.position.z, glare],
                    r0: r.x_axis.extend(0.0).to_array(),
                    r1: r.y_axis.extend(0.0).to_array(),
                    r2: r.z_axis.extend(0.0).to_array(),
                }
            })
            .collect();

        let params = Params {
            inv_view_proj: inv_vp,
            cam_pos: eye.extend(1.0),
            tint: tint.extend(tint_luma),
            housing_half: shape
                .housing_half
                .extend(if shape.round { 1.0 } else { 0.0 }),
            lum: Vec4::new(shape.half_x, shape.half_z, s.dimming, 0.0),
            counts: UVec4::new(c_size, g_size, lamps.len() as u32, prims.len() as u32),
            flags: UVec4::new(self.scene.ground, 0, s.bounces.clamp(1, 6), s.mode.gpu_id()),
            frame: UVec4::new(0, 0, 0, s.isolines as u32),
            f0: Vec4::new(
                1.0,
                1.0,
                ROAD_HALF_WIDTH,
                if self.scene.outdoor { 1.0 } else { 0.0 },
            ),
            f1: Vec4::new(
                0.25,
                if self.scene.outdoor {
                    NIGHT_SKY_RADIANCE
                } else {
                    0.0
                },
                s.haze * s.haze * 0.12,
                if s.glare { 0.6 } else { 0.0 },
            ),
            f2: Vec4::new(
                2.0 * self.peak_lux / std::f32::consts::PI,
                self.exposure,
                self.lux_max,
                self.iso_step,
            ),
            size: UVec4::new(
                self.size.x,
                self.size.y,
                self.mesh.as_ref().map_or(0, |m| m.nodes.len() as u32),
                self.mesh.as_ref().map_or(0, |m| m.tris.len() as u32),
            ),
        };

        FrameInput {
            params,
            prims,
            lamps,
            mesh: self.mesh.clone(),
            mesh_version: self.mesh_version,
            probes: self.probes.clone(),
            probes_version: self.probes_version,
            ies_version,
            size: self.size,
            reset_token: self.reset_token,
            probe_reset_token: self.probe_reset_token,
            max_samples: s.max_samples.max(1),
            probe_max_samples: s.probe_max_samples.max(16),
            passes_per_frame: s.passes_per_frame.clamp(1, 8),
            trace: s.active && self.photometry.is_some(),
            probe: s.active && s.mode == ViewMode::Points && self.photometry.is_some(),
        }
    }

    /// Em, Emin and Uo = Emin / Em per calculation grid from the probe values.
    pub fn grid_statistics(&self) -> Vec<GridStat> {
        if self.probe_values.len() < self.layout.points.len() || self.probe_values.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::new();
        for (gi, g) in self.scene.grids.iter().enumerate() {
            let vals: Vec<f32> = self
                .layout
                .points
                .iter()
                .enumerate()
                .filter(|(_, p)| p.grid == gi)
                .filter_map(|(k, _)| self.probe_values.get(k).copied())
                .collect();
            if vals.is_empty() {
                continue;
            }
            let em = vals.iter().sum::<f32>() / vals.len() as f32;
            let emin = vals.iter().cloned().fold(f32::MAX, f32::min);
            out.push(GridStat {
                name: g.name.to_string(),
                em,
                emin,
                uo: if em > 0.0 { emin / em } else { 0.0 },
            });
        }
        out
    }

    pub fn status(&self) -> Status {
        let (name, lumens, peak_cd, throw_c, asymmetry, cct_file) = self
            .photometry
            .as_ref()
            .map(|p| {
                (
                    p.name.clone(),
                    p.ldt.luminous_flux(),
                    p.table.max_cd,
                    p.throw.c,
                    p.throw.asymmetry,
                    p.cct_from_file,
                )
            })
            .unwrap_or_default();
        Status {
            name,
            lumens,
            peak_cd,
            throw_c,
            asymmetry,
            cct_file,
            cct: self.settings.cct,
            rotation: self.settings.params.rotation,
            samples: self.samples,
            max_samples: self.settings.max_samples,
            probe_samples: self.probe_samples,
            probe_max_samples: self.settings.probe_max_samples,
            point_count: self.probes.len() as u32,
            body_tris: self.mesh.as_ref().map_or(0, |m| m.tris.len() as u32),
            ppfd_factor: self.display_factor(),
            ppfd_from_spectrum: self.settings.ppfd_factor.is_some(),
            photoperiod: self.settings.params.photoperiod,
            lux_max: self.lux_max,
            iso_step: self.iso_step,
            key_lux: camera_key_lux(&self.scene),
            grids: self.grid_statistics(),
        }
    }

    /// Apply settings sent by a host.
    pub fn apply_sync(&mut self, sync: &SyncSettings) {
        let old = self.settings.clone();
        let kind = scene_from_id(&sync.scene);
        if kind != self.settings.params.kind {
            let housing = self.settings.params.housing_half;
            self.settings.params = SceneParams::defaults(kind);
            self.settings.params.housing_half = housing;
            self.camera_mode = CameraMode::Animate;
        }
        let p = &mut self.settings.params;
        p.height = sync.height;
        p.tilt = sync.tilt.clamp(-90.0, 90.0);
        p.count = sync.count.max(1);
        p.spacing = sync.spacing.max(0.5);
        p.wall_distance = sync.wall_distance.max(0.2);
        p.ceiling_height = sync.ceiling_height.clamp(2.4, 4.5);
        p.canopy_height = sync.canopy_height.clamp(0.5, 2.2);
        p.photoperiod = sync.photoperiod.clamp(1.0, 24.0);
        self.settings.ppfd_factor = sync.ppfd_factor.filter(|f| *f > 0.0);
        p.wall_finish = if sync.wall_finish == "brick" {
            WallFinish::Brick
        } else {
            WallFinish::Render
        };
        p.street_layout = match sync.street_layout.as_str() {
            "opposite" => StreetLayout::Opposite,
            "staggered" => StreetLayout::Staggered,
            _ => StreetLayout::Single,
        };
        p.show_figure = sync.show_figure;
        match sync.rotation {
            Some(r) => {
                p.rotation = r;
                self.settings.auto_rotate = false;
            }
            None => self.settings.auto_rotate = true,
        }
        let (lo, hi) = self.settings.params.height_range();
        self.settings.params.height = self.settings.params.height.clamp(lo, hi);
        self.settings.mode = ViewMode::from_id(&sync.mode);
        self.settings.isolines = sync.isolines || self.settings.mode == ViewMode::Isolux;
        self.settings.haze = sync.haze.clamp(0.0, 1.0);
        self.settings.glare = sync.glare;
        self.settings.exposure_ev = sync.exposure_ev.clamp(-4.0, 4.0);
        self.settings.bounces = sync.bounces.clamp(1, 6);
        let file_cct = self
            .photometry
            .as_ref()
            .and_then(|p| p.cct_from_file)
            .unwrap_or(DEFAULT_CCT);
        self.settings.cct = sync.cct.unwrap_or(file_cct).clamp(1800.0, 8000.0);
        self.settings.overlay = sync.overlay;
        self.settings.active = sync.active;
        self.settings.max_samples = sync.samples.clamp(16, 16384);
        self.settings.probe_max_samples = sync.point_samples.clamp(16, 65536);
        self.settings.passes_per_frame = sync.passes.clamp(1, 8);
        if sync.reset_view != self.last_reset_view {
            self.last_reset_view = sync.reset_view;
            self.camera_mode = CameraMode::Animate;
            self.scene_dirty = true;
        }
        self.after_edit(&old);
    }

    /// Measurement points for the overlay, in CSS pixels of a `css_w × css_h`
    /// canvas. Where neighbours crowd on screen only every 2nd, 4th, … point
    /// along each grid axis is kept; labels are dropped where they overlap.
    pub fn overlay(&self, css_w: f32, css_h: f32) -> Vec<OverlayPoint> {
        let mut out = Vec::new();
        let layout = &self.layout;
        let n = layout.points.len();
        if self.settings.mode != ViewMode::Points || n == 0 || self.probe_values.len() < n {
            return out;
        }
        let css_w = css_w.max(1.0);
        let css_h = css_h.max(1.0);
        let (vp, _) = self.camera.matrices(css_w / css_h);
        let eye = self.camera.eye();
        let focal = css_h / (2.0 * (self.camera.fov_y / 2.0).tan());
        let grid_radius: Vec<f32> = self
            .scene
            .grids
            .iter()
            .map(|g| {
                let cell =
                    (g.receiver.u.length() / g.nu as f32).min(g.receiver.v.length() / g.nv as f32);
                (0.1 * cell).clamp(0.05, 0.22)
            })
            .collect();

        let mut sx = vec![f32::NAN; n];
        let mut sy = vec![f32::NAN; n];
        let mut radius = vec![0.0f32; n];
        let mut on_screen = vec![false; n];
        for (k, p) in layout.points.iter().enumerate() {
            let Some((x, y, w)) = OrbitCamera::project(&vp, p.position) else {
                continue;
            };
            sx[k] = (x * 0.5 + 0.5) * css_w;
            sy[k] = (1.0 - (y * 0.5 + 0.5)) * css_h;
            radius[k] = (grid_radius[p.grid] * focal / w).max(2.0);
            let facing = (eye - p.position).dot(p.normal) > 0.0;
            let margin = 40.0 + radius[k];
            on_screen[k] = facing
                && sx[k] > -margin
                && sx[k] < css_w + margin
                && sy[k] > -margin
                && sy[k] < css_h + margin;
        }

        let mut discs: Vec<(usize, f32)> = Vec::new();
        for k in 0..n {
            if !on_screen[k] {
                continue;
            }
            let r = radius[k];
            let level = lod_level(layout, k, &sx, &sy, 2.0 * r, 2.0 * r);
            if !keep_at_level(&layout.points[k], level) {
                continue;
            }
            if segment_blocked(&self.scene, eye, layout.points[k].position) {
                continue;
            }
            discs.push((k, (layout.points[k].position - eye).length()));
            if discs.len() >= 600 {
                break;
            }
        }

        let label_h = 11.0 * 0.72;
        let gap = 5.0;
        let mut labels: Vec<(usize, String, f32, f32)> = Vec::new();
        for &(k, dist) in &discs {
            let f = self.display_factor();
            let text = format_lux(self.probe_values[k] * if f > 0.0 { f } else { 1.0 });
            let width = text.len() as f32 * 6.4;
            let level = lod_level(layout, k, &sx, &sy, width + gap, label_h + gap);
            if !keep_at_level(&layout.points[k], level) {
                continue;
            }
            labels.push((k, text, width, dist));
        }
        labels.sort_by(|a, b| a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal));
        let mut placed: Vec<[f32; 4]> = Vec::new();
        let mut labelled: Vec<(usize, String, f32)> = Vec::new();
        'outer: for (k, text, width, _) in labels {
            let x0 = sx[k] - width / 2.0 - gap / 2.0;
            let x1 = sx[k] + width / 2.0 + gap / 2.0;
            let y0 = sy[k] - label_h / 2.0 - gap / 2.0;
            let y1 = sy[k] + label_h / 2.0 + gap / 2.0;
            for r in &placed {
                if x0 < r[2] && x1 > r[0] && y0 < r[3] && y1 > r[1] {
                    continue 'outer;
                }
            }
            for &(d, _) in &discs {
                if d == k {
                    continue;
                }
                let r = radius[d];
                if x0 < sx[d] + r && x1 > sx[d] - r && y0 < sy[d] + r && y1 > sy[d] - r {
                    continue 'outer;
                }
            }
            placed.push([x0, y0, x1, y1]);
            labelled.push((k, text, width));
            if labelled.len() >= 400 {
                break;
            }
        }
        let has_label: std::collections::HashSet<usize> = labelled.iter().map(|l| l.0).collect();
        for &(k, _) in &discs {
            if has_label.contains(&k) {
                continue;
            }
            let c = heat_ramp(lux_ramp_t(self.probe_values[k], self.lux_max));
            out.push(OverlayPoint {
                x: sx[k],
                y: sy[k],
                radius: (radius[k] * 0.45).clamp(2.0, 5.0),
                text: None,
                color: c.to_array(),
                light_text: false,
            });
        }
        for (k, text, width) in labelled {
            let c = heat_ramp(lux_ramp_t(self.probe_values[k], self.lux_max));
            let luma = 0.2126 * c.x + 0.7152 * c.y + 0.0722 * c.z;
            out.push(OverlayPoint {
                x: sx[k],
                y: sy[k],
                radius: width / 2.0 + 4.0,
                text: Some(text),
                color: c.to_array(),
                light_text: luma < 0.5,
            });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_loads_and_produces_a_frame() {
        let mut v = Viewer::default();
        v.load_sample(0);
        v.resize(800.0, 500.0, 1.0);
        assert!(v.update(0.016));
        let f = v.frame_input();
        assert!(f.trace);
        assert!(f.lamps.len() == 1);
        assert!(f.params.counts.x == 360);
        assert!(v.lux_max > 1.0);
    }

    #[test]
    fn sync_round_trip_changes_scene() {
        let mut v = Viewer::default();
        v.load_sample(0);
        let mut s = SyncSettings::for_scene(SceneKind::Office);
        s.mode = "points".into();
        v.apply_sync(&s);
        v.update(0.016);
        assert_eq!(v.settings.params.kind, SceneKind::Office);
        assert_eq!(v.settings.mode, ViewMode::Points);
        assert_eq!(v.scene.grids[0].name, "workplane 0.75 m");
    }

    #[test]
    fn cct_from_appearance() {
        assert_eq!(parse_color_temperature("3000K"), Some(3000.0));
        assert_eq!(parse_color_temperature("LED 4000 Kelvin"), Some(4000.0));
        assert_eq!(parse_color_temperature("unknown"), None);
    }
}
