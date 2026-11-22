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
    auto_rotation, build_scene, hidden_from, interior_box, outline_area_perimeter, segment_blocked,
    sensor_mount, BuiltScene, CameraView, LampRole, ObjectId, SceneKind, SceneParams, ScenePlan,
    SceneSensor, Shape, StreetLayout, WallFinish, GRID_EMPHASIS, MAX_SENSORS, ROAD_HALF_WIDTH,
};
use crate::sensing::{evaluate, ControlMode, Sensing, SensorControl};
use crate::sync::{scene_from_id, GridStat, Status, SyncSettings};
use eulumdat::sensor::{DetectionArea, Motion, SensorFile};
use eulumdat::{Eulumdat, PhotometricCalculations, ThrowDirection};
use glam::{UVec2, UVec4, Vec2, Vec3, Vec4};
use std::sync::Arc;

use crate::mesh::LuminaireMesh;

/// One sensor's detection volume on screen ([`Viewer::sensor_volume_lines`]).
#[derive(Clone, Debug, Default)]
pub struct SensorVolumeLines {
    /// Polylines in CSS pixels.
    pub lines: Vec<Vec<(f32, f32)>>,
    /// Whether the sensor detects something.
    pub detecting: bool,
}

/// Wireframe of a detection volume for a sensor at height `mount`,
/// relative to its head: horizontal rings at five heights between the
/// floor and the head, and lines from the head down along the boundary in
/// eight C-planes.
/// Length of a polyline (m).
fn path_length(path: &[Vec2]) -> f32 {
    path.windows(2).map(|w| (w[1] - w[0]).length()).sum()
}

/// The point `s` metres along a polyline (clamped to its ends).
fn point_along(path: &[Vec2], s: f32) -> Vec2 {
    let mut left = s.max(0.0);
    for w in path.windows(2) {
        let len = (w[1] - w[0]).length();
        if left <= len && len > 0.0 {
            return w[0] + (w[1] - w[0]) * (left / len);
        }
        left -= len;
    }
    path.last().copied().unwrap_or(Vec2::ZERO)
}

/// Clip-space w below which a point counts as behind the camera.
const NEAR_W: f32 = 0.05;

/// Clip a closed polygon (clip space) to w ≥ `NEAR_W` (Sutherland–Hodgman).
fn clip_near(poly: &[Vec4]) -> Vec<Vec4> {
    let mut out = Vec::with_capacity(poly.len() + 2);
    for (i, &a) in poly.iter().enumerate() {
        let b = poly[(i + 1) % poly.len()];
        let (da, db) = (a.w - NEAR_W, b.w - NEAR_W);
        if da >= 0.0 {
            out.push(a);
        }
        if (da >= 0.0) != (db >= 0.0) {
            out.push(a + (b - a) * (da / (da - db)));
        }
    }
    out
}

/// Clip a segment (clip space) to w ≥ `NEAR_W`.
fn clip_segment(a: Vec4, b: Vec4) -> Option<(Vec4, Vec4)> {
    let (da, db) = (a.w - NEAR_W, b.w - NEAR_W);
    match (da >= 0.0, db >= 0.0) {
        (true, true) => Some((a, b)),
        (false, false) => None,
        (true, false) => Some((a, a + (b - a) * (da / (da - db)))),
        (false, true) => Some((a + (b - a) * (da / (da - db)), b)),
    }
}

fn volume_wireframe(v: &eulumdat::sensor::DetectionVolume, mount: f32) -> Vec<Vec<Vec3>> {
    let mount = mount.max(0.1) as f64;
    let mut out = Vec::new();
    for k in 0..5 {
        let depth = mount * (1.0 - k as f64 / 5.0);
        let fp = v.footprint(depth, 10.0);
        if fp.is_empty() || fp.outline.len() < 3 {
            continue;
        }
        let mut ring: Vec<Vec3> = fp
            .outline
            .iter()
            .map(|&(x, y)| Vec3::new(x as f32, -depth as f32, y as f32))
            .collect();
        ring.push(ring[0]);
        out.push(ring);
    }
    for c in (0..360).step_by(45) {
        let cr = (c as f64).to_radians();
        let mut line = vec![Vec3::ZERO];
        for j in 1..=16 {
            let depth = mount * j as f64 / 16.0;
            let r = v.reach(c as f64, depth);
            if r <= 0.0 {
                break;
            }
            line.push(Vec3::new(
                (r * cr.cos()) as f32,
                -depth as f32,
                (r * cr.sin()) as f32,
            ));
        }
        if line.len() >= 2 {
            out.push(line);
        }
    }
    out
}

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

/// Speed / accuracy trade-off of the tracer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Quality {
    /// Lower resolution, one random lamp per shading point, no haze: a
    /// fast preview for placing things (the panel adds direct light only).
    Quick,
    /// Medium resolution, every lamp at the first hit, one random lamp on
    /// the bounces.
    Balanced,
    /// Full resolution, every lamp everywhere.
    #[default]
    High,
}

impl Quality {
    pub fn id(self) -> &'static str {
        match self {
            Quality::Quick => "quick",
            Quality::Balanced => "balanced",
            Quality::High => "high",
        }
    }

    pub fn from_id(id: &str) -> Self {
        match id {
            "quick" => Quality::Quick,
            "balanced" => Quality::Balanced,
            _ => Quality::High,
        }
    }

    /// Traced pixels at most.
    pub fn pixel_cap(self) -> f32 {
        match self {
            Quality::Quick => 0.45e6,
            Quality::Balanced => 1.0e6,
            Quality::High => f32::MAX,
        }
    }

    /// `Params::lum.w`: 0 every lamp, 1 one lamp on the bounces, 2 one lamp
    /// everywhere.
    pub fn lamp_mode(self) -> f32 {
        match self {
            Quality::Quick => 2.0,
            Quality::Balanced => 1.0,
            Quality::High => 0.0,
        }
    }
}

/// Seconds after the last camera / object move during which `auto_quick`
/// keeps tracing Quick.
const INTERACTION_SECS: f32 = 0.4;

/// Ray–primitive tests one trace dispatch may cost before the frame is
/// split into bands of rows (one submission each): browsers reset a GPU
/// whose command buffer runs too long ("device lost").
const WORK_PER_DISPATCH: f32 = 1.0e9;
const MAX_BANDS: u32 = 32;

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
    /// Melanopic DER of the luminaire's spectrum (melanopic EDI = lux × DER).
    pub mel_der: Option<f32>,
    /// How the sensors switch the lamps.
    pub control: SensorControl,
    pub quality: Quality,
    /// Smooth the reflected-light noise of the picture (display only).
    pub denoise: bool,
    /// Trace Quick (direct light + estimated fill) while the camera or an
    /// object moves, the chosen quality once things stand still.
    pub auto_quick: bool,
    /// The host draws the sensors' detection volumes in 3D
    /// ([`Viewer::sensor_volume_lines`]).
    pub show_sensor_volume: bool,
    /// Emergency mode (EN 1838): normal lighting off, emergency luminaires
    /// on (`LampRole`), exposure and false colours for a few lux.
    pub emergency_mode: bool,
    /// The host draws the plan's emergency markings
    /// ([`Viewer::emergency_marks`]); off = as a person escaping sees it.
    pub show_emergency_marks: bool,
    /// Natural light over outdoor scenes for a date, time and place (the
    /// sun's or moon's beam with shadows and the sky, in the picture and in
    /// the lux values).
    pub sky: Option<crate::moon::SkyLight>,
    /// All luminaires off (the scene in natural light alone).
    pub lamps_off: bool,
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
            mel_der: None,
            control: SensorControl::default(),
            quality: Quality::default(),
            denoise: true,
            auto_quick: false,
            show_sensor_volume: false,
            emergency_mode: false,
            show_emergency_marks: true,
            sky: None,
            lamps_off: false,
        }
    }
}

/// A sample file shipped with the viewer.
pub struct Sample {
    pub name: &'static str,
    pub content: &'static str,
    pub scene: SceneKind,
}

/// A sample LDT from `samples/` (copies of the editor's templates in
/// `eulumdat-wasm/templates/`: a published crate can only embed its own
/// files).
macro_rules! template {
    ($f:literal) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/samples/", $f))
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
    /// Each trace pass runs as this many bands of rows, one submission each.
    pub bands: u32,
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

/// The grab point of a movable object on screen (CSS pixels).
#[derive(Clone, Debug)]
pub struct ObjectHandle {
    pub id: ObjectId,
    pub x: f32,
    pub y: f32,
    /// Sensors: it detects something; person / cars: a sensor detects it;
    /// luminaires: at full output.
    pub active: bool,
    /// Luminaires: output 0–1 (1 for the others).
    pub level: f32,
}

/// A sensor's detection area on the ground, on screen (CSS pixels).
#[derive(Clone, Debug)]
pub struct SensorOutline {
    pub sensor: usize,
    pub points: Vec<(f32, f32)>,
    pub detecting: bool,
}

/// An object being dragged: it slides on the horizontal plane through its
/// grab point; `grab` = anchor − first hit, so it does not jump.
#[derive(Clone, Copy, Debug)]
struct ObjectDrag {
    id: ObjectId,
    plane_y: f32,
    grab: Vec2,
}

/// Grab radius around a handle (CSS pixels).
const PICK_RADIUS: f32 = 26.0;

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

/// Emergency mode: the picture starts with the exposure of normal
/// lighting (the moment the power fails) and opens up like an eye adapting
/// to the dark, by this many stops at most (a fully adapted eye would be
/// 6–7 stops, which made emergency light look as bright as normal light).
pub const EMERGENCY_ADAPT_STOPS: f32 = 4.0;
/// Time constant of that adaptation (s): ~2.5 stops after 3 s, ~3.9 after
/// 10 s.
pub const EMERGENCY_ADAPT_TAU: f32 = 3.0;
/// Top of the false-colour scale in emergency mode (lx).
pub const EMERGENCY_LUX_MAX: f32 = 5.0;
/// Eye height when walking an escape route (m).
pub const WALK_EYE_HEIGHT: f32 = 1.6;
/// Walking pace when the walk plays (m/s).
pub const WALK_SPEED: f32 = 1.0;
/// The walking camera looks at the route this far ahead (m).
const WALK_LOOK_AHEAD: f32 = 2.5;

/// Walking an escape route: which one, where (0–1), on by itself or not.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Walk {
    route: usize,
    t: f32,
    playing: bool,
}

/// The plan's emergency markings on screen (CSS pixels), clipped at the
/// camera: route bands (one closed polygon per segment), centre lines,
/// the open area's border and the points of emphasis.
#[derive(Clone, Debug, Default)]
pub struct EmergencyMarks {
    pub bands: Vec<Vec<(f32, f32)>>,
    pub centre_lines: Vec<Vec<(f32, f32)>>,
    pub open_area: Vec<Vec<(f32, f32)>>,
    pub points: Vec<EmergencyMark>,
}

/// A point of emphasis on screen, with its measured vertical illuminance
/// (emergency mode, once measured).
#[derive(Clone, Debug)]
pub struct EmergencyMark {
    pub x: f32,
    pub y: f32,
    pub symbol: String,
    pub vertical: bool,
    pub lux: Option<f32>,
}

pub struct Viewer {
    pub settings: Settings,
    pub photometry: Option<Photometry>,
    /// Further luminaire types of a planned scene (`SceneLamp::table` 1, 2, …).
    pub extra_photometries: Vec<Photometry>,
    /// The main table and the extra types stacked (`None`: one type).
    ies_stack: Option<IesTable>,
    /// Bumped when the stack changes (part of `FrameInput::ies_version`).
    stack_generation: u64,
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
    /// The plan shown by `SceneKind::Planned` ([`Self::set_plan`]); kept
    /// while another scene is shown, so switching back restores it.
    pub plan: Option<Arc<ScenePlan>>,
    /// Sensor file of all sensors (name, file); see [`Self::set_sensor_file`].
    pub sensor_file: Option<(String, Arc<SensorFile>)>,
    sensor_version: u64,
    area_key: Option<(u64, Motion, i32)>,
    area: Option<DetectionArea>,
    volume_key: Option<(u64, Motion)>,
    volume: Option<Arc<eulumdat::sensor::DetectionVolume>>,
    /// Wireframe of one sensor's volume, relative to its head (rings at
    /// several heights + lines from the head down), for the 3D overlay.
    volume_wire: Vec<Vec<Vec3>>,
    /// What the sensors detect and the lamp levels.
    pub sensing: Sensing,
    drag: Option<ObjectDrag>,
    /// Seconds since the camera or an object last moved.
    idle_secs: f32,
    was_interacting: bool,
    /// Average illuminance of the main surfaces (estimate, lux) and the
    /// even fill that stands in for reflected light in Quick (lux).
    avg_lux: f32,
    pub ambient_lux: f32,
    /// Extra splitting of the trace when frames stay slow (1, 2, 4, …).
    band_boost: u32,
    very_slow_frames: u32,
    fast_frames: u32,
    /// Walking an escape route at eye height (`None`: orbit camera).
    walk: Option<Walk>,
    /// The host's last walk request (route, position, play).
    last_walk: (u32, Option<f32>, bool),
    /// Seconds since the emergency mode came on (eye adaptation).
    adapt_secs: f32,
    last_replay: u32,
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
            extra_photometries: Vec::new(),
            ies_stack: None,
            stack_generation: 0,
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
            plan: None,
            sensor_file: None,
            sensor_version: 0,
            area_key: None,
            area: None,
            volume_key: None,
            volume: None,
            volume_wire: Vec::new(),
            sensing: Sensing::default(),
            drag: None,
            band_boost: 1,
            very_slow_frames: 0,
            fast_frames: 0,
            walk: None,
            last_walk: (0, None, false),
            adapt_secs: 0.0,
            last_replay: 0,
            idle_secs: 10.0,
            was_interacting: false,
            avg_lux: 0.0,
            ambient_lux: 0.0,
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
        self.rebuild_stack();
        self.scene_dirty = true;
    }

    /// Stack the main table with the extra types (or drop the stack).
    fn rebuild_stack(&mut self) {
        self.stack_generation += 1;
        self.ies_stack = match self.photometry.as_ref() {
            Some(main) if !self.extra_photometries.is_empty() => {
                let mut tables = vec![&main.table];
                tables.extend(self.extra_photometries.iter().map(|p| &p.table));
                Some(IesTable::stack(&tables))
            }
            _ => None,
        };
    }

    /// The intensity table(s) to upload: the stack when the scene has
    /// several luminaire types, else the luminaire's table.
    pub fn ies_table(&self) -> Option<&IesTable> {
        self.ies_stack
            .as_ref()
            .or(self.photometry.as_ref().map(|p| &p.table))
    }

    /// Version of [`Self::ies_table`]: changes with the luminaire and the
    /// extra types (a shell re-uploads the table when it changes).
    pub fn ies_version(&self) -> u64 {
        self.photometry.as_ref().map_or(0, |p| {
            p.version
                .wrapping_mul(1_000_003)
                .wrapping_add(self.stack_generation)
        })
    }

    /// The natural light on the scene now: on, outdoors, not in emergency
    /// mode (codes do not count it).
    pub fn sky_light(&self) -> Option<crate::moon::SkyLight> {
        self.settings
            .sky
            .filter(|_| self.scene.outdoor && !self.settings.emergency_mode)
    }

    /// The natural light decides exposure and false colours: by day, at
    /// dusk brighter than the lamps' own range, or with the lamps off.
    fn natural_dominates(&self) -> Option<crate::moon::SkyLight> {
        self.sky_light()
            .filter(|s| self.settings.lamps_off || s.horizontal_lux > 0.5 * self.lux_max)
    }

    /// False-colour range and isoline step: the lamps' own, or the natural
    /// light's when it dominates.
    pub fn shown_lux_range(&self) -> (f32, f32) {
        match self.natural_dominates() {
            Some(s) => {
                let max = crate::photometry::nice_ceil((s.horizontal_lux * 1.3).max(0.01));
                (max, max / 10.0)
            }
            None => (self.lux_max, self.iso_step),
        }
    }

    /// Exposure key (lx): a few lux in emergency mode; daylight and
    /// twilight as they are when they outshine the lamps; with the lamps
    /// off at night, six times the moonlight (a night picture, not a
    /// camera's daylight-bright long exposure).
    fn key_lux(&self) -> f32 {
        let normal = camera_key_lux(&self.scene);
        if let Some(s) = self.natural_dominates() {
            use crate::moon::SkyKind;
            return match s.kind {
                SkyKind::Sun | SkyKind::Twilight => s.horizontal_lux,
                SkyKind::Moon | SkyKind::Night => (s.horizontal_lux * 6.0).max(0.05),
            };
        }
        if self.settings.lamps_off && !self.settings.emergency_mode {
            return 0.05;
        }
        if self.settings.emergency_mode {
            normal / 2f32.powf(EMERGENCY_ADAPT_STOPS * self.adaptation())
        } else {
            normal
        }
    }

    /// How far the eye has adapted to the emergency lighting (0 = the
    /// moment the power failed, 1 = adapted); 0 outside emergency mode.
    pub fn adaptation(&self) -> f32 {
        if self.settings.emergency_mode {
            1.0 - (-self.adapt_secs / EMERGENCY_ADAPT_TAU).exp()
        } else {
            0.0
        }
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
            || s.quality != old.quality
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
        if s.sky != old.sky || s.lamps_off != old.lamps_off {
            // other light everywhere: the picture and the points start over
            self.reset_token += 1;
            self.probe_reset_token += 1;
        }
        if s.glare != old.glare {
            self.glare_dirty = true;
        }
        if s.control != old.control && !self.scene_dirty {
            self.update_sensing();
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

    /// Centre line of escape route `i` of the shown plan (plan (x, z)).
    fn route_path(&self, i: usize) -> Option<&[Vec2]> {
        if self.settings.params.kind != SceneKind::Planned {
            return None;
        }
        let em = self.settings.params.plan.as_ref()?.emergency.as_ref()?;
        em.routes
            .get(i)
            .map(|(p, _)| p.as_slice())
            .filter(|p| p.len() >= 2)
    }

    /// Length (m) of each escape route of the shown plan.
    pub fn route_lengths(&self) -> Vec<f32> {
        if self.settings.params.kind != SceneKind::Planned {
            return Vec::new();
        }
        self.settings
            .params
            .plan
            .as_ref()
            .and_then(|p| p.emergency.as_ref())
            .map(|em| em.routes.iter().map(|(p, _)| path_length(p)).collect())
            .unwrap_or_default()
    }

    /// Put the camera on the walked route: eye height, looking a little
    /// down at the route ahead (it turns before a corner).
    fn place_walk_camera(&mut self) {
        let Some(w) = self.walk else {
            return;
        };
        let Some(path) = self.route_path(w.route) else {
            return;
        };
        let len = path_length(path);
        let s = w.t.clamp(0.0, 1.0) * len;
        let here = point_along(path, s);
        let mut d = point_along(path, (s + WALK_LOOK_AHEAD).min(len)) - here;
        if d.length() < 0.3 {
            // at the end: keep the direction of the last stretch
            d = here - point_along(path, (s - 1.0).max(0.0));
        }
        let d = d.normalize_or(Vec2::X);
        let (pitch, distance) = (8.0_f32.to_radians(), 0.5);
        let eye = Vec3::new(here.x, WALK_EYE_HEIGHT, here.y);
        let forward = Vec3::new(d.x * pitch.cos(), -pitch.sin(), d.y * pitch.cos());
        self.camera.set_view(
            CameraView {
                target: eye + forward * distance,
                yaw: (-d.x).atan2(-d.y),
                pitch,
                distance,
            },
            false,
        );
        self.camera_changed();
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
        self.touch();
    }

    /// Something moved (camera, object): Quick for a moment if `auto_quick`.
    fn touch(&mut self) {
        self.idle_secs = 0.0;
    }

    /// The camera or an object is moving (and `auto_quick` is on).
    pub fn interacting(&self) -> bool {
        self.settings.auto_quick && self.idle_secs < INTERACTION_SECS
    }

    /// Quality in use this frame (Quick while moving).
    pub fn effective_quality(&self) -> Quality {
        if self.interacting() {
            Quality::Quick
        } else {
            self.settings.quality
        }
    }

    /// Bounces in use this frame (direct light only while moving).
    pub fn effective_bounces(&self) -> u32 {
        if self.interacting() {
            1
        } else {
            self.settings.bounces.clamp(1, 6)
        }
    }

    /// Even fill (lux) standing in for the reflected light when only direct
    /// light is traced: indoors the flux reflected around a room of mean
    /// reflectance 0.5 (Φ ρ / (A (1 − ρ)) = Φ / A; glasshouse ρ 0.3),
    /// outdoors a share of the average illuminance (ground ~10 %, facade
    /// scene 15 %). Follows the lamp levels under sensor control.
    fn compute_ambient(&self) -> f32 {
        if self.photometry.is_none() {
            return 0.0;
        }
        let n = self.scene.lamps.len();
        if n == 0 {
            return 0.0;
        }
        let level = (0..n).map(|i| self.lamp_level(i)).sum::<f32>() / n as f32;
        // luminaire flux of every lamp at its level (each lamp its own type)
        let flux = (0..n).map(|i| self.lamp_flux(i)).sum::<f32>();
        if let Some(plan) = self
            .settings
            .params
            .plan
            .as_ref()
            .filter(|p| p.indoor && self.settings.params.kind == SceneKind::Planned)
        {
            // floor + ceiling + walls of the planned outline, reflectance
            // weighted by area
            let (a, per) = outline_area_perimeter(&plan.outline);
            let walls = per * plan.height;
            let total = (2.0 * a + walls).max(1.0);
            let [rc, rw, rf] = plan.reflectances;
            let rho = ((rc + rf) * a + rw * walls) / total;
            let rho = rho.clamp(0.05, 0.9);
            return flux * rho / (total * (1.0 - rho));
        }
        match interior_box(&self.settings.params) {
            Some(b) => {
                let area = 2.0 * (b.x * b.z + (b.x + b.z) * b.y);
                let rho = if self.settings.params.kind == SceneKind::Greenhouse {
                    0.3
                } else {
                    0.5
                };
                flux * rho / (area.max(1.0) * (1.0 - rho))
            }
            None => {
                let share = if self.settings.params.kind == SceneKind::Facade {
                    0.15
                } else {
                    0.1
                };
                share * self.avg_lux * level
            }
        }
    }

    /// The photometry of lamp `i` (its type in a plan with several).
    fn lamp_photometry(&self, i: usize) -> Option<&Photometry> {
        let t = self.scene.lamps.get(i).map_or(0, |l| l.table as usize);
        match t {
            0 => self.photometry.as_ref(),
            t => self
                .extra_photometries
                .get(t - 1)
                .or(self.photometry.as_ref()),
        }
    }

    /// Luminaire flux (lm) of lamp `i` at its current level.
    fn lamp_flux(&self, i: usize) -> f32 {
        let Some(p) = self.lamp_photometry(i) else {
            return 0.0;
        };
        let lor = (p.ldt.light_output_ratio as f32 / 100.0).clamp(0.05, 1.0);
        p.ldt.luminous_flux() as f32 * lor * self.settings.dimming * self.lamp_level(i)
    }

    /// Render size for a canvas of `css_w × css_h` CSS pixels at `dpr`,
    /// capped like the web viewer; returns true when it changed.
    pub fn resize(&mut self, css_w: f32, css_h: f32, dpr: f32) -> bool {
        let css_w = css_w.max(1.0);
        let css_h = css_h.max(1.0);
        let mut pr = dpr.clamp(0.5, MAX_PIXEL_RATIO) * self.render_scale;
        let cap = self.max_pixels.min(self.effective_quality().pixel_cap());
        if css_w * css_h * pr * pr > cap {
            pr = (cap / (css_w * css_h)).sqrt();
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
        // Frames that stay very slow: split the trace further, so no single
        // submission runs long enough for the browser to reset the GPU.
        if dt_ms > 150.0 {
            self.very_slow_frames += 1;
            self.fast_frames = 0;
            if self.very_slow_frames >= 3 && self.band_boost < 8 {
                self.very_slow_frames = 0;
                self.band_boost *= 2;
            }
        } else if dt_ms < 40.0 {
            self.very_slow_frames = 0;
            self.fast_frames += 1;
            if self.fast_frames >= 120 && self.band_boost > 1 {
                self.fast_frames = 0;
                self.band_boost /= 2;
            }
        }
    }

    /// Bands of rows per trace pass: estimated work (pixels × shading
    /// points × lamps sampled × primitives) over `WORK_PER_DISPATCH`, times
    /// the boost from slow frames.
    pub fn trace_bands(&self) -> u32 {
        let pixels = (self.size.x * self.size.y) as f32;
        let prims = self.scene.prims.len().max(8) as f32;
        let lamps = self.scene.lamps.len().max(1) as f32;
        let bounces = self.effective_bounces() as f32;
        let quality = self.effective_quality();
        let mode = quality.lamp_mode();
        let first = if mode > 1.5 { 1.0 } else { lamps };
        let rest = if mode > 0.5 { 1.0 } else { lamps };
        let mut per_path = (1.0 + first) + (bounces - 1.0) * (1.0 + rest);
        if self.settings.haze > 0.0 && quality != Quality::Quick {
            per_path += if mode > 0.5 { 1.0 } else { lamps };
        }
        let work = pixels * per_path * prims;
        let estimate = (work / WORK_PER_DISPATCH).ceil().max(1.0) as u32;
        (estimate * self.band_boost).clamp(1, MAX_BANDS)
    }

    /// Advance the camera, rebuild the scene if needed, refresh exposure and glare.
    /// Returns true when the scene was rebuilt (grids and probes changed).
    pub fn update(&mut self, dt_secs: f32) -> bool {
        if self.settings.emergency_mode {
            self.adapt_secs += dt_secs.max(0.0);
        }
        if let Some(w) = self.walk.filter(|w| w.playing) {
            let len = self.route_lengths().get(w.route).copied().unwrap_or(0.0);
            let t = if len > 0.0 {
                w.t + WALK_SPEED * dt_secs.max(0.0) / len
            } else {
                1.0
            };
            self.walk = Some(Walk {
                t: t.min(1.0),
                playing: t < 1.0,
                ..w
            });
            self.place_walk_camera();
        }
        if self.camera.update(dt_secs) {
            self.camera_changed();
        }
        self.idle_secs += dt_secs.max(0.0);
        // Moving ↔ standing still switches the quality: start the picture over.
        let moving = self.interacting();
        if moving != self.was_interacting {
            self.was_interacting = moving;
            self.reset_token += 1;
        }
        let rebuilt = if self.scene_dirty {
            self.rebuild_scene();
            // a rebuild sets the scene's home view: back onto the route
            if self.walk.is_some() {
                self.place_walk_camera();
            }
            true
        } else {
            false
        };
        let key = self.key_lux();
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
        // a scene switch or a plan replaces the params
        self.settings.params.emergency = self.settings.emergency_mode;

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

        let (max, avg) =
            estimate_illuminance(&self.scene, &photometry.table, self.settings.dimming);
        self.avg_lux = avg;
        let key = self.key_lux();
        self.exposure = exposure_for(key, self.settings.exposure_ev);
        let mount = self
            .scene
            .lamps
            .iter()
            .fold(1.0_f32, |h, l| h.max(l.position.y));
        let anchor_lux = photometry.table.max_cd * self.settings.dimming / (mount * mount).max(1.0);
        self.peak_lux = max.max(anchor_lux).max(1e-3);
        let f = self.display_factor();
        if self.settings.emergency_mode {
            // EN 1838 lives between 0.5 and a few lux, China up to 10
            self.lux_max = self
                .settings
                .params
                .plan
                .as_ref()
                .and_then(|p| p.emergency.as_ref())
                .map_or(EMERGENCY_LUX_MAX, |e| e.standard.lux_max.max(1.0));
            self.iso_step = self.lux_max / 10.0;
        } else if f > 0.0 {
            self.lux_max = nice_ceil(max.max(1e-3) * 1.02 * f) / f;
            self.iso_step = nice_ceil(self.lux_max * f / 10.0) / f;
        } else {
            self.lux_max = nice_ceil(max.max(1e-3) * 1.02);
            self.iso_step = nice_ceil(self.lux_max / 10.0);
        }
        self.glare_dirty = true;
        self.reset_token += 1;
        self.update_sensing();
    }

    /// Show a plan from a planner (`None` forgets it). The scene switches
    /// to `SceneKind::Planned` with the plan's sensors; moves start over.
    pub fn set_plan(&mut self, plan: Option<Arc<ScenePlan>>) {
        self.plan = plan.clone();
        let Some(plan) = plan else {
            return;
        };
        let housing = self.settings.params.housing_half;
        let mut params = SceneParams::defaults(SceneKind::Planned);
        params.housing_half = housing;
        params.show_figure = self.settings.params.show_figure;
        params.sensors = plan
            .sensors
            .iter()
            .take(MAX_SENSORS)
            .map(|s| SceneSensor { x: s.x, z: s.y })
            .collect();
        params.sensor_height = plan.sensor_height;
        self.extra_photometries = plan
            .photometries
            .iter()
            .filter_map(|t| {
                let ldt = Eulumdat::parse(&t.ldt).ok()?;
                Some(Photometry::new(&t.name, ldt, 0))
            })
            .collect();
        self.rebuild_stack();
        params.plan = Some(plan);
        self.settings.params = params;
        self.settings.auto_rotate = false;
        self.camera_mode = CameraMode::Animate;
        self.scene_dirty = true;
    }

    /// The scene as a document (`sync` = the panel settings it was made
    /// with; `id`, `name`, `luminaire` from the host).
    pub fn scene_doc(
        &self,
        sync: &SyncSettings,
        id: &str,
        name: &str,
        luminaire: Option<String>,
    ) -> crate::doc::SceneDoc {
        let p = &self.settings.params;
        crate::doc::SceneDoc {
            version: crate::doc::SCENE_DOC_VERSION,
            id: id.to_string(),
            name: name.to_string(),
            settings: sync.clone(),
            plan: if p.kind == SceneKind::Planned {
                p.plan.as_deref().cloned()
            } else {
                None
            },
            moves: p.moves.iter().map(|(o, v)| (*o, [v.x, v.y])).collect(),
            sensors: p.sensors.clone(),
            view: Some(self.camera.view),
            sensor_file: self.sensor_file.as_ref().map(|(n, _)| n.clone()),
            luminaire,
        }
    }

    /// Show a saved scene: its settings (as `apply_sync`), plan, moved
    /// objects, sensors and camera. The host also puts `doc.settings` into
    /// its panel; applying them again changes nothing.
    pub fn apply_doc(&mut self, doc: &crate::doc::SceneDoc) {
        if let Some(plan) = doc.plan.clone() {
            self.plan = Some(Arc::new(plan));
        }
        // a fresh start in the doc's scene type, then its settings
        let kind = scene_from_id(&doc.settings.scene);
        if kind == SceneKind::Planned && self.plan.is_some() {
            self.set_plan(self.plan.clone());
        } else {
            let housing = self.settings.params.housing_half;
            self.settings.params = SceneParams::defaults(kind);
            self.settings.params.housing_half = housing;
            self.scene_dirty = true;
        }
        self.apply_sync(&doc.settings);
        self.edit(|s| {
            s.params.moves = doc
                .moves
                .iter()
                .map(|(o, v)| (*o, Vec2::new(v[0], v[1])))
                .collect();
            s.params.sensors = doc.sensors.iter().take(MAX_SENSORS).copied().collect();
        });
        if let Some(view) = doc.view {
            self.camera.set_view(view, false);
            self.camera_mode = CameraMode::Keep;
        }
    }

    /// Changes worth saving (moved objects, sensors, camera, plan).
    pub fn doc_fingerprint(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        let p = &self.settings.params;
        let f = |v: f32| (v * 100.0).round() as i64;
        for (o, v) in &p.moves {
            o.hash(&mut h);
            (f(v.x), f(v.y)).hash(&mut h);
        }
        for s in &p.sensors {
            (f(s.x), f(s.z)).hash(&mut h);
        }
        let v = self.camera.view;
        (
            f(v.target.x),
            f(v.target.y),
            f(v.target.z),
            f(v.yaw),
            f(v.pitch),
            f(v.distance),
        )
            .hash(&mut h);
        p.plan.as_ref().map(|pl| pl.name.clone()).hash(&mut h);
        self.sensor_file
            .as_ref()
            .map(|(n, _)| n.clone())
            .hash(&mut h);
        h.finish()
    }

    /// Use `file` for every sensor (`None`: sensors detect nothing).
    pub fn set_sensor_file(&mut self, file: Option<(String, Arc<SensorFile>)>) {
        let same = match (&self.sensor_file, &file) {
            (Some(a), Some(b)) => Arc::ptr_eq(&a.1, &b.1),
            (None, None) => true,
            _ => false,
        };
        if same {
            return;
        }
        self.sensor_file = file;
        self.sensor_version += 1;
        self.update_sensing();
    }

    /// Detection area at the sensor height (cached) and what the sensors
    /// see; a change of the lamp levels restarts the picture.
    fn update_sensing(&mut self) {
        let mount = sensor_mount(&self.settings.params);
        let motion = self.settings.control.motion;
        let key = self
            .sensor_file
            .as_ref()
            .map(|_| (self.sensor_version, motion, (mount * 1000.0).round() as i32));
        if key != self.area_key {
            self.area_key = key;
            self.area = self
                .sensor_file
                .as_ref()
                .and_then(|(_, f)| f.area_at(motion, mount as f64, 0.0, false));
            let vkey = self
                .sensor_file
                .as_ref()
                .map(|_| (self.sensor_version, motion));
            if vkey != self.volume_key {
                self.volume_key = vkey;
                self.volume = self
                    .sensor_file
                    .as_ref()
                    .and_then(|(_, f)| f.motion_volume(motion, false))
                    .map(Arc::new);
            }
            self.volume_wire = self
                .volume
                .as_deref()
                .map(|v| volume_wireframe(v, mount))
                .unwrap_or_default();
        }
        let sensing = evaluate(
            &self.scene,
            self.area.as_ref(),
            self.volume.as_deref(),
            &self.settings.control,
        );
        if sensing.lamp_level != self.sensing.lamp_level {
            self.reset_token += 1;
            self.probe_reset_token += 1;
            self.glare_dirty = true;
        }
        self.sensing = sensing;
        self.ambient_lux = self.compute_ambient();
    }

    /// Output (0–1) of lamp `i` under sensor control.
    pub fn lamp_level(&self, i: usize) -> f32 {
        let role = self
            .scene
            .lamps
            .get(i)
            .map(|l| l.role.level(self.settings.emergency_mode))
            .unwrap_or(1.0);
        if self.settings.lamps_off && !self.settings.emergency_mode {
            return 0.0;
        }
        self.sensing.lamp_level.get(i).copied().unwrap_or(1.0) * role
    }

    /// Where sensor `i` goes when it is added: at a pole (outdoors) or
    /// next to a luminaire (indoors), else in front of the camera.
    pub fn default_sensor_position(params: &SceneParams, i: usize) -> SceneSensor {
        let scene = build_scene(params);
        let mut lums: Vec<_> = scene
            .movables
            .iter()
            .filter(|m| matches!(m.id, ObjectId::Luminaire(_)))
            .collect();
        lums.sort_by_key(|m| m.id);
        if lums.is_empty() {
            return SceneSensor {
                x: scene.view.target.x + 2.0 * i as f32,
                z: scene.view.target.z,
            };
        }
        let m = lums[i % lums.len()];
        let round = (i / lums.len()) as f32;
        let (x, z) = if scene.outdoor && !m.prims.is_empty() {
            (m.anchor.x, m.anchor.z)
        } else {
            (m.handle.x + 0.6, m.handle.z)
        };
        SceneSensor {
            x: x + 1.5 * round,
            z,
        }
    }

    /// Put object `id` with its anchor at the ground position `xz`.
    pub fn move_object(&mut self, id: ObjectId, xz: Vec2) {
        self.touch();
        let xz = xz.clamp(Vec2::splat(-250.0), Vec2::splat(250.0));
        self.edit(|s| match id {
            ObjectId::Sensor(i) => {
                if let Some(sn) = s.params.sensors.get_mut(i as usize) {
                    *sn = SceneSensor { x: xz.x, z: xz.y };
                }
            }
            _ => match s.params.moves.iter_mut().find(|m| m.0 == id) {
                Some(m) => m.1 = xz,
                None => s.params.moves.push((id, xz)),
            },
        });
    }

    /// Back to the computed places (sensors stay where they are).
    pub fn reset_moves(&mut self) {
        self.edit(|s| s.params.moves.clear());
    }

    /// Ray from the eye through a CSS pixel of a `css_w × css_h` canvas.
    pub fn screen_ray(&self, css_x: f32, css_y: f32, css_w: f32, css_h: f32) -> (Vec3, Vec3) {
        let (_, inv) = self.camera.matrices(css_w.max(1.0) / css_h.max(1.0));
        let nx = 2.0 * css_x / css_w.max(1.0) - 1.0;
        let ny = 1.0 - 2.0 * css_y / css_h.max(1.0);
        let unproject = |z: f32| {
            let p = inv * Vec4::new(nx, ny, z, 1.0);
            p.truncate() / p.w
        };
        let near = unproject(0.0);
        let far = unproject(1.0);
        (near, (far - near).normalize_or_zero())
    }

    fn hit_plane(&self, css_x: f32, css_y: f32, css_w: f32, css_h: f32, y: f32) -> Option<Vec2> {
        let (o, d) = self.screen_ray(css_x, css_y, css_w, css_h);
        if d.y.abs() < 1e-4 {
            return None;
        }
        let t = (y - o.y) / d.y;
        if !(0.0..2000.0).contains(&t) {
            return None;
        }
        let p = o + d * t;
        Some(Vec2::new(p.x, p.z))
    }

    fn project_css(&self, p: Vec3, css_w: f32, css_h: f32) -> Option<(f32, f32)> {
        let (vp, _) = self.camera.matrices(css_w.max(1.0) / css_h.max(1.0));
        let (x, y, _) = OrbitCamera::project(&vp, p)?;
        Some(((x * 0.5 + 0.5) * css_w, (1.0 - (y * 0.5 + 0.5)) * css_h))
    }

    /// Grab points of all movable objects that are on screen.
    pub fn object_handles(&self, css_w: f32, css_h: f32) -> Vec<ObjectHandle> {
        let seen: Vec<ObjectId> = self.sensing.detected.iter().flatten().copied().collect();
        self.scene
            .movables
            .iter()
            .filter_map(|m| {
                let (x, y) = self.project_css(m.handle, css_w, css_h)?;
                if x < -40.0 || y < -40.0 || x > css_w + 40.0 || y > css_h + 40.0 {
                    return None;
                }
                let (active, level) = match m.id {
                    ObjectId::Sensor(i) => (
                        self.sensing
                            .detected
                            .get(i as usize)
                            .is_some_and(|d| !d.is_empty()),
                        1.0,
                    ),
                    ObjectId::Luminaire(i) => {
                        let l = self.lamp_level(i as usize);
                        (l >= 0.999, l)
                    }
                    id => (seen.contains(&id), 1.0),
                };
                Some(ObjectHandle {
                    id: m.id,
                    x,
                    y,
                    active,
                    level,
                })
            })
            .collect()
    }

    /// Start dragging the object whose grab point is under (`css_x`,
    /// `css_y`); false if there is none.
    pub fn begin_drag(&mut self, css_x: f32, css_y: f32, css_w: f32, css_h: f32) -> bool {
        let best = self
            .object_handles(css_w, css_h)
            .into_iter()
            .map(|h| (h.id, (h.x - css_x).hypot(h.y - css_y)))
            .filter(|&(_, d)| d <= PICK_RADIUS)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        let Some((id, _)) = best else {
            return false;
        };
        let Some(m) = self.scene.movables.iter().find(|m| m.id == id) else {
            return false;
        };
        let (plane_y, anchor) = (m.handle.y, Vec2::new(m.anchor.x, m.anchor.z));
        let Some(hit) = self.hit_plane(css_x, css_y, css_w, css_h, plane_y) else {
            return false;
        };
        self.drag = Some(ObjectDrag {
            id,
            plane_y,
            grab: anchor - hit,
        });
        true
    }

    /// Move the dragged object under the pointer.
    pub fn drag_to(&mut self, css_x: f32, css_y: f32, css_w: f32, css_h: f32) {
        let Some(d) = self.drag else {
            return;
        };
        if let Some(hit) = self.hit_plane(css_x, css_y, css_w, css_h, d.plane_y) {
            self.move_object(d.id, hit + d.grab);
        }
    }

    pub fn end_drag(&mut self) {
        self.drag = None;
    }

    /// The object being dragged.
    pub fn dragging(&self) -> Option<ObjectId> {
        self.drag.map(|d| d.id)
    }

    /// The room's floor outline on screen (closed scenes): sensor areas end
    /// at the walls, so a host clips their drawing to it.
    pub fn room_outline_screen(&self, css_w: f32, css_h: f32) -> Option<Vec<(f32, f32)>> {
        let outline = crate::scene::room_outline(&self.settings.params)?;
        let pts: Vec<(f32, f32)> = outline
            .iter()
            .filter_map(|q| self.project_css(Vec3::new(q.x, 0.03, q.y), css_w, css_h))
            .collect();
        (pts.len() == outline.len() && pts.len() >= 3).then_some(pts)
    }

    /// Each sensor's detection volume as a wireframe on screen (polylines;
    /// broken where a point is behind the camera or, indoors, outside the
    /// room), with whether the sensor detects something.
    pub fn sensor_volume_lines(&self, css_w: f32, css_h: f32) -> Vec<SensorVolumeLines> {
        if self.volume_wire.is_empty() {
            return Vec::new();
        }
        let room = crate::scene::room_outline(&self.settings.params);
        let (vp, _) = self.camera.matrices(css_w.max(1.0) / css_h.max(1.0));
        self.scene
            .movables
            .iter()
            .filter_map(|m| match m.id {
                ObjectId::Sensor(i) => Some((i as usize, m.handle)),
                _ => None,
            })
            .map(|(i, head)| {
                let mut lines = Vec::new();
                for wire in &self.volume_wire {
                    let mut cur: Vec<(f32, f32)> = Vec::new();
                    for q in wire {
                        let p = head + *q;
                        let inside = room
                            .as_ref()
                            .is_none_or(|o| crate::scene::outline_contains(o, Vec2::new(p.x, p.z)));
                        let on_screen = inside.then(|| OrbitCamera::project(&vp, p)).flatten().map(
                            |(x, y, _)| ((x * 0.5 + 0.5) * css_w, (1.0 - (y * 0.5 + 0.5)) * css_h),
                        );
                        match on_screen {
                            Some(pt) => cur.push(pt),
                            None => {
                                if cur.len() >= 2 {
                                    lines.push(std::mem::take(&mut cur));
                                }
                                cur.clear();
                            }
                        }
                    }
                    if cur.len() >= 2 {
                        lines.push(cur);
                    }
                }
                let detecting = self.sensing.detected.get(i).is_some_and(|d| !d.is_empty());
                SensorVolumeLines { lines, detecting }
            })
            .collect()
    }

    /// Each sensor's detection area on the ground, on screen.
    pub fn sensor_outlines(&self, css_w: f32, css_h: f32) -> Vec<SensorOutline> {
        let Some(area) = self.area.as_ref().filter(|a| !a.footprint.is_empty()) else {
            return Vec::new();
        };
        self.scene
            .movables
            .iter()
            .filter_map(|m| match m.id {
                ObjectId::Sensor(i) => Some((i as usize, m.anchor)),
                _ => None,
            })
            .filter_map(|(i, foot)| {
                let points: Vec<(f32, f32)> = area
                    .footprint
                    .outline
                    .iter()
                    .filter_map(|&(ox, oy)| {
                        // plan y = world z (C90 = +Z)
                        let p = Vec3::new(foot.x + ox as f32, 0.03, foot.z + oy as f32);
                        self.project_css(p, css_w, css_h)
                    })
                    .collect();
                (points.len() >= 3).then(|| SensorOutline {
                    sensor: i,
                    points,
                    detecting: self.sensing.detected.get(i).is_some_and(|d| !d.is_empty()),
                })
            })
            .collect()
    }

    /// The plan's emergency markings on screen, `None` when the scene has
    /// none or they are switched off (`Settings::show_emergency_marks`).
    /// Parts behind the camera are clipped, so they also work when walking
    /// a route.
    pub fn emergency_marks(&self, css_w: f32, css_h: f32) -> Option<EmergencyMarks> {
        if !self.settings.show_emergency_marks || self.settings.params.kind != SceneKind::Planned {
            return None;
        }
        let em = self.settings.params.plan.as_ref()?.emergency.as_ref()?;
        let (css_w, css_h) = (css_w.max(1.0), css_h.max(1.0));
        let (vp, _) = self.camera.matrices(css_w / css_h);
        let clip = |x: f32, y: f32, z: f32| vp * Vec4::new(x, y, z, 1.0);
        let screen = |c: Vec4| {
            (
                (c.x / c.w * 0.5 + 0.5) * css_w,
                (1.0 - (c.y / c.w * 0.5 + 0.5)) * css_h,
            )
        };
        let polyline = |pts: &[Vec2], closed: bool, y: f32| -> Vec<Vec<(f32, f32)>> {
            let mut lines: Vec<Vec<(f32, f32)>> = Vec::new();
            let mut cur: Vec<(f32, f32)> = Vec::new();
            let n = pts.len();
            let segs = if closed { n } else { n.saturating_sub(1) };
            for i in 0..segs {
                let (a, b) = (pts[i], pts[(i + 1) % n]);
                let (a0, b0) = (clip(a.x, y, a.y), clip(b.x, y, b.y));
                match clip_segment(a0, b0) {
                    Some((ca, cb)) => {
                        // a cut start begins a new line, a cut end ends it
                        if a0.w < NEAR_W || cur.is_empty() {
                            if cur.len() >= 2 {
                                lines.push(std::mem::take(&mut cur));
                            }
                            cur.clear();
                            cur.push(screen(ca));
                        }
                        cur.push(screen(cb));
                        if b0.w < NEAR_W {
                            lines.push(std::mem::take(&mut cur));
                        }
                    }
                    None => {
                        if cur.len() >= 2 {
                            lines.push(std::mem::take(&mut cur));
                        }
                        cur.clear();
                    }
                }
            }
            if cur.len() >= 2 {
                lines.push(cur);
            }
            lines
        };
        let mut marks = EmergencyMarks::default();
        for (path, width) in &em.routes {
            let last = path.len().saturating_sub(2);
            for (i, w) in path.windows(2).enumerate() {
                let (a, b) = (w[0], w[1]);
                let len = (b - a).length();
                if len < 1e-3 {
                    continue;
                }
                let dir = (b - a) / len;
                let side = Vec2::new(-dir.y, dir.x) * (0.5 * width);
                // square ends at the inner corners, so a turn has no gap
                let a = if i > 0 { a - dir * (0.5 * width) } else { a };
                let b = if i < last { b + dir * (0.5 * width) } else { b };
                let quad = [a + side, b + side, b - side, a - side].map(|q| clip(q.x, 0.01, q.y));
                let poly = clip_near(&quad);
                if poly.len() >= 3 {
                    marks.bands.push(poly.into_iter().map(screen).collect());
                }
            }
            marks.centre_lines.extend(polyline(path, false, 0.012));
        }
        if let Some(area) = em.open_area.as_ref() {
            marks.open_area = polyline(area, true, 0.012);
        }
        // the vertical probes, in the order of the vertical points
        let probe_of_grid = |g: usize| {
            self.layout
                .points
                .iter()
                .position(|p| p.grid == g)
                .and_then(|k| self.probe_values.get(k).copied())
        };
        let mut vertical_grids = self
            .scene
            .grids
            .iter()
            .enumerate()
            .filter(|(_, g)| g.name == GRID_EMPHASIS)
            .map(|(gi, _)| gi);
        for pt in &em.points {
            let lux = if pt.vertical {
                vertical_grids.next().and_then(probe_of_grid)
            } else {
                None
            };
            let c = clip(pt.position.x, pt.height, pt.position.y);
            if c.w < NEAR_W {
                continue;
            }
            let (x, y) = screen(c);
            marks.points.push(EmergencyMark {
                x,
                y,
                symbol: pt.symbol.clone(),
                vertical: pt.vertical,
                lux,
            });
        }
        Some(marks)
    }

    fn compute_glare(&self) -> Vec<f32> {
        let Some(table) = self.ies_table() else {
            return Vec::new();
        };
        let eye = self.camera.eye();
        let has_cutaway = self.scene.prims.iter().any(|b| b.cutaway.is_some());
        self.scene
            .lamps
            .iter()
            .enumerate()
            .map(|(li, lamp)| {
                if !self.settings.glare {
                    return 0.0;
                }
                let d = eye - lamp.position;
                let d2 = d.length_squared();
                let dist = d2.sqrt();
                if dist < 0.2 {
                    return 0.0;
                }
                let i = table.sample_dir_in(lamp.table as u32, &lamp.rotation, d / dist)
                    * self.settings.dimming
                    * self.lamp_level(li);
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
        let quality = self.effective_quality();
        let bounces = self.effective_bounces();
        let tint = kelvin_to_linear_rgb(s.cct);
        let tint_luma = (0.2126 * tint.x + 0.7152 * tint.y + 0.0722 * tint.z).max(1e-4);
        let shape = self.shape;
        let (c_size, g_size) = self
            .photometry
            .as_ref()
            .map(|p| (p.table.c_size, p.table.g_size))
            .unwrap_or((1, 1));
        let ies_version = self.ies_version();
        let tables = self.ies_table().map(|t| t.tables.max(1)).unwrap_or(1);

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
                    reflectance: b.reflectance,
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
                    // w: attenuation by sensor control (0 = full output)
                    r0: r.x_axis.extend(1.0 - self.lamp_level(i)).to_array(),
                    // w: photometry table in the stack
                    r1: r
                        .y_axis
                        .extend(u32::from(l.table).min(tables - 1) as f32)
                        .to_array(),
                    r2: r.z_axis.extend(0.0).to_array(),
                }
            })
            .collect();

        let params = Params {
            inv_view_proj: inv_vp,
            // w: even fill for direct-light-only tracing (0 = off)
            cam_pos: eye.extend(if bounces == 1 { self.ambient_lux } else { 0.0 }),
            tint: tint.extend(tint_luma),
            housing_half: shape
                .housing_half
                .extend(if shape.round { 1.0 } else { 0.0 }),
            lum: Vec4::new(shape.half_x, shape.half_z, s.dimming, quality.lamp_mode()),
            counts: UVec4::new(c_size, g_size, lamps.len() as u32, prims.len() as u32),
            flags: UVec4::new(self.scene.ground, 0, bounces, s.mode.gpu_id()),
            // w: bit 0 isolines, 1 denoise, 2 probes direct only (EN 1838)
            frame: UVec4::new(
                0,
                0,
                0,
                s.isolines as u32 | (s.denoise as u32) << 1 | (s.emergency_mode as u32) << 2,
            ),
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
                if quality == Quality::Quick {
                    0.0
                } else {
                    s.haze * s.haze * 0.12
                },
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
            moon: Vec4::ZERO,
            moon_sky: Vec4::ZERO,
        };
        let mut params = params;
        if let Some(sky) = self.sky_light() {
            // the beam's colour with the lamps' luma, so the lux of
            // reflected natural light come out right (`/ tint.w`); the sky
            // is uniform: radiance = horizontal illuminance / π
            let c = kelvin_to_linear_rgb(sky.beam_cct);
            let luma = (0.2126 * c.x + 0.7152 * c.y + 0.0722 * c.z).max(1e-4);
            let beam = if sky.dir.y > 0.0 { sky.beam_lux } else { 0.0 };
            params.moon = sky.dir.extend(beam);
            params.moon_sky = (c / luma * tint_luma).extend(sky.sky_lux / std::f32::consts::PI);
        }
        let (lux_max, iso_step) = self.shown_lux_range();
        params.f2.z = lux_max;
        params.f2.w = iso_step;

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
            bands: self.trace_bands(),
            probe_reset_token: self.probe_reset_token,
            max_samples: s.max_samples.max(1),
            probe_max_samples: s.probe_max_samples.max(16),
            passes_per_frame: s.passes_per_frame.clamp(1, 8),
            trace: s.active && self.photometry.is_some(),
            // the points keep their bounces: they pause while things move
            probe: s.active
                && s.mode == ViewMode::Points
                && self.photometry.is_some()
                && !self.interacting(),
        }
    }

    /// Em, Emin, Emax and Uo = Emin / Em per calculation grid from the
    /// probe values; grids of one name count as one (the segments of the
    /// escape routes).
    pub fn grid_statistics(&self) -> Vec<GridStat> {
        if self.probe_values.len() < self.layout.points.len() || self.probe_values.is_empty() {
            return Vec::new();
        }
        let mut names: Vec<&str> = Vec::new();
        for g in &self.scene.grids {
            if !names.contains(&g.name) {
                names.push(g.name);
            }
        }
        let mut out = Vec::new();
        for name in names {
            let vals: Vec<f32> = self
                .layout
                .points
                .iter()
                .enumerate()
                .filter(|(_, p)| self.scene.grids.get(p.grid).is_some_and(|g| g.name == name))
                .filter_map(|(k, _)| self.probe_values.get(k).copied())
                .collect();
            if vals.is_empty() {
                continue;
            }
            let em = vals.iter().sum::<f32>() / vals.len() as f32;
            let emin = vals.iter().cloned().fold(f32::MAX, f32::min);
            let emax = vals.iter().cloned().fold(0.0, f32::max);
            out.push(GridStat {
                name: name.to_string(),
                em,
                emin,
                uo: if em > 0.0 { emin / em } else { 0.0 },
                emax,
            });
        }
        out
    }

    pub fn status(&self) -> Status {
        let mut st = self.status_without_moon();
        st.sky = self.settings.sky.filter(|_| self.scene.outdoor);
        st.outdoor = self.scene.outdoor;
        let (lux_max, _) = self.shown_lux_range();
        st.lux_max = lux_max;
        st
    }

    fn status_without_moon(&self) -> Status {
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
            sky: None,
            outdoor: false,
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
            mel_der: self.settings.mel_der.unwrap_or(0.0),
            photoperiod: self.settings.params.photoperiod,
            lux_max: self.lux_max,
            iso_step: self.iso_step,
            key_lux: self.key_lux(),
            grids: self.grid_statistics(),
            sensors: self.settings.params.sensors.len() as u32,
            sensors_detecting: self
                .sensing
                .detected
                .iter()
                .map(|d| !d.is_empty())
                .collect(),
            person_detected: self
                .sensing
                .detected
                .iter()
                .flatten()
                .any(|id| *id == ObjectId::Person),
            cars_detected: {
                let mut cars: Vec<ObjectId> = self
                    .sensing
                    .detected
                    .iter()
                    .flatten()
                    .filter(|id| matches!(id, ObjectId::Car(_)))
                    .copied()
                    .collect();
                cars.sort();
                cars.dedup();
                cars.len() as u32
            },
            lamps_on: self.sensing.lamps_on() as u32,
            lamps: self.scene.lamps.len() as u32,
            sensor_diameter: self
                .area
                .as_ref()
                .filter(|a| !a.footprint.is_empty())
                .map(|a| a.footprint.equivalent_diameter() as f32),
            sensor_height: sensor_mount(&self.settings.params),
            moved: self.settings.params.moves.len() as u32,
            emergency_lamps: self
                .scene
                .lamps
                .iter()
                .filter(|l| l.role != LampRole::Normal)
                .count() as u32,
            routes: self.route_lengths(),
            walk: self.walk.map(|w| w.t),
            walk_playing: self.walk.is_some_and(|w| w.playing),
            adaptation: self.adaptation(),
        }
    }

    /// Apply settings sent by a host.
    pub fn apply_sync(&mut self, sync: &SyncSettings) {
        let old = self.settings.clone();
        let kind = scene_from_id(&sync.scene);
        if kind != self.settings.params.kind {
            if kind == SceneKind::Planned && self.plan.is_some() {
                // back to the plan (its sensors, a fresh start)
                self.set_plan(self.plan.clone());
            } else {
                let housing = self.settings.params.housing_half;
                self.settings.params = SceneParams::defaults(kind);
                self.settings.params.housing_half = housing;
                self.camera_mode = CameraMode::Animate;
            }
        }
        let planned = self.settings.params.kind == SceneKind::Planned;
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
        self.settings.mel_der = sync.mel_der.filter(|f| *f > 0.0);
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
        if sync.sensor_height > 0.0 {
            p.sensor_height = sync.sensor_height.clamp(0.5, 20.0);
        }
        let n = (sync.sensors as usize).min(MAX_SENSORS);
        p.sensors.truncate(n);
        while self.settings.params.sensors.len() < n {
            let i = self.settings.params.sensors.len();
            let pos = Self::default_sensor_position(&self.settings.params, i);
            self.settings.params.sensors.push(pos);
        }
        let p = &mut self.settings.params;
        self.settings.control = SensorControl {
            mode: ControlMode::from_id(&sync.sensor_control),
            idle_level: sync.sensor_idle.clamp(0.0, 1.0),
            motion: match sync.sensor_motion.as_str() {
                "radial" => Motion::Radial,
                "presence" => Motion::Presence,
                _ => Motion::Tangential,
            },
            dims: sync.sensor_dim,
        };
        match sync.rotation {
            Some(r) => {
                p.rotation = r;
                self.settings.auto_rotate = false;
            }
            None => self.settings.auto_rotate = true,
        }
        if planned {
            // planned luminaires keep their own directions
            self.settings.auto_rotate = false;
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
        self.settings.quality = Quality::from_id(&sync.quality);
        self.settings.denoise = sync.denoise;
        self.settings.auto_quick = sync.auto_quick;
        self.settings.show_sensor_volume = sync.sensor_volume;
        self.settings.show_emergency_marks = sync.emergency_marks;
        let walk_key = (sync.walk_route, sync.walk, sync.walk_play);
        if walk_key != self.last_walk {
            self.last_walk = walk_key;
            match sync.walk {
                Some(t) => {
                    self.walk = Some(Walk {
                        route: sync.walk_route as usize,
                        t: t.clamp(0.0, 1.0),
                        playing: sync.walk_play,
                    });
                    self.place_walk_camera();
                }
                None => {
                    if self.walk.take().is_some() {
                        self.camera.set_view(self.scene.view, true);
                        self.camera_changed();
                    }
                }
            }
        }
        if sync.emergency_replay != self.last_replay {
            // "power failure again": the eye starts over
            self.last_replay = sync.emergency_replay;
            self.adapt_secs = 0.0;
        }
        if sync.emergency_mode != self.settings.emergency_mode {
            self.adapt_secs = 0.0;
            // other lamps, another exposure and false-colour range
            self.settings.emergency_mode = sync.emergency_mode;
            self.settings.params.emergency = sync.emergency_mode;
            self.scene_dirty = true;
        }
        if sync.reset_view != self.last_reset_view {
            self.last_reset_view = sync.reset_view;
            self.camera_mode = CameraMode::Animate;
            self.scene_dirty = true;
        }
        self.settings.sky = sync.moon_time().and_then(|t| t.sky());
        // "luminaires off" belongs to the natural light: without it, the
        // lamps stay on (it used to darken every later scene, hidden)
        self.settings.lamps_off = sync.lamps_off && sync.moon;
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
        // the points of emphasis carry their value on the markings
        let marked = |g: usize| {
            self.settings.show_emergency_marks
                && self
                    .scene
                    .grids
                    .get(g)
                    .is_some_and(|g| g.name == GRID_EMPHASIS)
        };
        for (k, p) in layout.points.iter().enumerate() {
            if marked(p.grid) {
                continue;
            }
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
            if discs.len() >= 1500 {
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

    fn ceiling_sensor() -> Arc<SensorFile> {
        let ldt = include_str!("../../eulumdat/tests/fixtures/sensors/demo_ceiling_360.ldt");
        Arc::new(SensorFile::from_bytes(ldt.as_bytes(), "sensor/sensldt").unwrap())
    }

    fn parking_with_sensors(n: u32, idle: f32) -> Viewer {
        let mut v = Viewer::default();
        v.load_sample(0);
        let mut s = SyncSettings::for_scene(SceneKind::Parking);
        s.count = 3;
        s.sensors = n;
        s.sensor_height = 3.0;
        s.sensor_idle = idle;
        v.apply_sync(&s);
        v.set_sensor_file(Some(("demo".into(), ceiling_sensor())));
        v.resize(800.0, 500.0, 1.0);
        // let the camera finish its move to the new scene
        for _ in 0..50 {
            v.update(0.1);
        }
        v
    }

    #[test]
    fn sensors_dim_and_light_the_lamps() {
        let mut v = parking_with_sensors(1, 0.2);
        assert_eq!(v.settings.params.sensors.len(), 1);
        // the first sensor sits at the first pole; send the person away
        v.move_object(ObjectId::Person, Vec2::new(60.0, 60.0));
        v.update(0.016);
        let f = v.frame_input();
        assert!(!v.sensing.any() || !v.sensing.detected[0].contains(&ObjectId::Person));
        let st = v.status();
        assert_eq!(st.sensors, 1);
        assert!(!st.person_detected);
        // parked cars may still be seen; with nobody there the lamps idle
        if !v.sensing.any() {
            assert!(f.lamps.iter().all(|l| (l.r0[3] - 0.8).abs() < 1e-6));
            assert_eq!(st.lamps_on, 0);
        }
        // the person walks under the sensor: full output
        let s0 = v.settings.params.sensors[0];
        let token = v.reset_token;
        v.move_object(ObjectId::Person, Vec2::new(s0.x, s0.z));
        v.update(0.016);
        assert!(v.reset_token > token, "a change restarts the picture");
        assert!(v.status().person_detected);
        let f = v.frame_input();
        assert!(f.lamps.iter().all(|l| l.r0[3].abs() < 1e-6));
        assert!(v.status().sensor_diameter.unwrap() > 1.0);
        // without a sensor file nothing is detected
        v.set_sensor_file(None);
        assert!(!v.status().person_detected);
    }

    #[test]
    fn dragging_moves_the_object_under_the_pointer() {
        let mut v = parking_with_sensors(0, 0.1);
        let (w, h) = (800.0, 500.0);
        let person = v
            .object_handles(w, h)
            .into_iter()
            .find(|o| o.id == ObjectId::Person)
            .expect("the person is on screen");
        assert!(v.begin_drag(person.x + 3.0, person.y - 2.0, w, h));
        assert_eq!(v.dragging(), Some(ObjectId::Person));
        let before = v
            .scene
            .movables
            .iter()
            .find(|m| m.id == ObjectId::Person)
            .unwrap()
            .anchor;
        v.drag_to(person.x + 80.0, person.y + 30.0, w, h);
        v.update(0.016);
        let after = v
            .scene
            .movables
            .iter()
            .find(|m| m.id == ObjectId::Person)
            .unwrap()
            .anchor;
        assert!((after - before).length() > 0.5, "{before} → {after}");
        // the grab point follows the pointer
        let moved = v
            .object_handles(w, h)
            .into_iter()
            .find(|o| o.id == ObjectId::Person)
            .unwrap();
        assert!(
            (moved.x - (person.x + 77.0)).abs() < 4.0,
            "{} vs {}",
            moved.x,
            person.x + 77.0
        );
        assert!(
            (moved.y - (person.y + 32.0)).abs() < 4.0,
            "{} vs {}",
            moved.y,
            person.y + 32.0
        );
        v.end_drag();
        assert_eq!(v.dragging(), None);
        // nothing under an empty spot
        assert!(!v.begin_drag(2.0, 2.0, w, h));
        // reset puts it back
        v.reset_moves();
        v.update(0.016);
        let back = v
            .scene
            .movables
            .iter()
            .find(|m| m.id == ObjectId::Person)
            .unwrap()
            .anchor;
        assert!((back - before).length() < 1e-5);
    }

    #[test]
    fn sensor_outlines_follow_the_sensor() {
        let mut v = parking_with_sensors(2, 0.1);
        let outlines = v.sensor_outlines(800.0, 500.0);
        assert_eq!(outlines.len(), 2);
        assert!(outlines.iter().all(|o| o.points.len() >= 3));
        let cx =
            |o: &SensorOutline| o.points.iter().map(|p| p.0).sum::<f32>() / o.points.len() as f32;
        let x0 = cx(&outlines[0]);
        let s1 = v.settings.params.sensors[1];
        v.move_object(ObjectId::Sensor(1), Vec2::new(s1.x + 4.0, s1.z));
        v.update(0.016);
        let after = v.sensor_outlines(800.0, 500.0);
        assert!((cx(&after[0]) - x0).abs() < 1e-3, "sensor 0 stays");
        assert!(
            (cx(&after[1]) - cx(&outlines[1])).abs() > 5.0,
            "sensor 1 moved"
        );
    }

    #[test]
    fn quality_sets_resolution_lamp_sampling_and_bands() {
        let mut v = Viewer::default();
        v.load_sample(0);
        let mut s = SyncSettings::for_scene(SceneKind::Street);
        s.count = 4;
        s.street_layout = "opposite".into();
        s.apply_quality("high");
        v.apply_sync(&s);
        v.resize(1500.0, 900.0, 1.0);
        v.update(0.016);
        assert_eq!(v.scene.lamps.len(), 8);
        let high = v.frame_input();
        assert_eq!(high.params.lum.w, 0.0);
        assert!(high.params.f1.z > 0.0, "haze on");
        // 1.35 MP × 3 bounces × 8 lamps × ~180 prims: split into several
        // submissions
        assert!(high.bands >= 4, "{} bands", high.bands);

        s.apply_quality("quick");
        v.apply_sync(&s);
        v.resize(1500.0, 900.0, 1.0);
        v.update(0.016);
        let quick = v.frame_input();
        assert_eq!(quick.params.lum.w, 2.0);
        assert_eq!(quick.params.f1.z, 0.0, "no haze in quick");
        assert_eq!(quick.params.flags.z, 1, "direct light only");
        assert!((quick.size.x * quick.size.y) as f32 <= Quality::Quick.pixel_cap() * 1.01);
        assert!(quick.bands < high.bands);
        assert_eq!(quick.bands, 1);
    }

    #[test]
    fn slow_frames_split_the_trace_further() {
        let mut v = Viewer::default();
        v.load_sample(0);
        v.resize(800.0, 500.0, 1.0);
        v.update(0.016);
        let before = v.trace_bands();
        for _ in 0..6 {
            v.note_frame_time(400.0);
        }
        assert!(
            v.trace_bands() >= 4 * before.min(8),
            "{} → {}",
            before,
            v.trace_bands()
        );
        for _ in 0..400 {
            v.note_frame_time(10.0);
        }
        assert_eq!(v.trace_bands(), before);
    }

    #[test]
    fn quick_while_moving_then_back() {
        let mut v = Viewer::default();
        v.load_sample(0);
        let mut s = SyncSettings::for_scene(SceneKind::Street);
        s.apply_quality("high");
        assert!(s.auto_quick);
        v.apply_sync(&s);
        v.resize(800.0, 500.0, 1.0);
        for _ in 0..30 {
            v.update(0.1);
        }
        assert!(!v.interacting());
        assert_eq!(v.frame_input().params.flags.z, 3);
        // orbiting: direct light only, one random lamp, no haze, no probes
        v.orbit(10.0, 0.0);
        v.update(0.016);
        assert!(v.interacting());
        let f = v.frame_input();
        assert_eq!(f.params.flags.z, 1);
        assert_eq!(f.params.lum.w, 2.0);
        assert_eq!(f.params.f1.z, 0.0);
        assert!(!f.probe);
        // standing still again: back to High, the picture starts over
        let token = v.reset_token;
        for _ in 0..10 {
            v.update(0.1);
        }
        assert!(!v.interacting());
        assert!(v.reset_token > token);
        assert_eq!(v.frame_input().params.flags.z, 3);
        // without auto_quick nothing switches
        v.edit(|s| s.auto_quick = false);
        v.orbit(10.0, 0.0);
        v.update(0.016);
        assert_eq!(v.frame_input().params.flags.z, 3);
    }

    #[test]
    fn quick_adds_an_even_fill_for_the_reflected_light() {
        let mut v = Viewer::default();
        v.load_sample(3); // office batwing
        let mut s = SyncSettings::for_scene(SceneKind::Office);
        s.count = 4;
        s.apply_quality("quick");
        v.apply_sync(&s);
        v.resize(400.0, 300.0, 1.0);
        for _ in 0..30 {
            v.update(0.1);
        }
        let f = v.frame_input();
        let fill = f.params.cam_pos.w;
        // 4 luminaires of a few thousand lm in a 6 × 4.8 × 2.8 m room: the
        // reflected light is some tens to a few hundred lux
        assert!(fill > 20.0 && fill < 600.0, "fill {fill} lx");
        s.apply_quality("high");
        v.apply_sync(&s);
        v.update(0.1);
        assert_eq!(
            v.frame_input().params.cam_pos.w,
            0.0,
            "traced, not estimated"
        );
        // denoise is a display flag (bit 1)
        assert_eq!(v.frame_input().params.frame.w & 2, 2);
        s.denoise = false;
        v.apply_sync(&s);
        assert_eq!(v.frame_input().params.frame.w & 2, 0);
    }

    fn plan_room(rho_wall: f32) -> Arc<ScenePlan> {
        Arc::new(ScenePlan {
            name: "room".into(),
            indoor: true,
            outline: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(6.0, 0.0),
                Vec2::new(6.0, 5.0),
                Vec2::new(0.0, 5.0),
            ],
            height: 3.0,
            reflectances: [0.7, rho_wall, 0.2],
            plane: 0.8,
            luminaires: vec![crate::scene::PlanLuminaire {
                position: Vec3::new(3.0, 2.95, 2.5),
                rotation_deg: 0.0,
                tilt_deg: 0.0,
                pole: None,
                pendant: false,
                ..Default::default()
            }],
            sensors: vec![Vec2::new(1.0, 1.0)],
            sensor_height: 3.0,
            note: None,
            ..Default::default()
        })
    }

    #[test]
    fn a_plan_opens_and_comes_back() {
        let mut v = Viewer::default();
        v.load_sample(3);
        v.set_plan(Some(plan_room(0.5)));
        v.resize(400.0, 300.0, 1.0);
        v.update(0.016);
        assert_eq!(v.scene.kind, SceneKind::Planned);
        assert_eq!(v.settings.params.sensors.len(), 1);
        // the panel's settings keep the plan
        let mut s = SyncSettings::for_scene(SceneKind::Planned);
        s.sensors = 1;
        v.apply_sync(&s);
        v.update(0.016);
        assert_eq!(v.scene.kind, SceneKind::Planned);
        assert!(v.settings.params.plan.is_some());
        // another scene, then back: the plan returns
        v.apply_sync(&SyncSettings::for_scene(SceneKind::Street));
        v.update(0.016);
        assert_eq!(v.scene.kind, SceneKind::Street);
        v.apply_sync(&s);
        v.update(0.016);
        assert_eq!(v.scene.kind, SceneKind::Planned);
        assert_eq!(v.scene.lamps.len(), 1);
        // the reflectances reach the GPU primitives
        let f = v.frame_input();
        assert!(f.prims.iter().any(|g| (g.reflectance - 0.5).abs() < 1e-6));
    }

    /// A room with a normal luminaire (type 0) and an emergency luminaire
    /// (type 1) over an escape route.
    fn emergency_room() -> Arc<ScenePlan> {
        let mut plan = (*plan_room(0.5)).clone();
        // no sensor control: the lamps run at full output
        plan.sensors.clear();
        plan.luminaires.push(crate::scene::PlanLuminaire {
            position: Vec3::new(1.5, 2.95, 2.5),
            photometry: 1,
            role: crate::scene::LampRole::Emergency,
            ..Default::default()
        });
        plan.photometries = vec![crate::scene::PlanPhotometry {
            name: "EM".into(),
            ldt: include_str!("../../eulumdat/tests/fixtures/emergency/demo_em_route.ldt").into(),
        }];
        plan.emergency = Some(crate::scene::PlanEmergency {
            routes: vec![(vec![Vec2::new(0.5, 2.5), Vec2::new(5.5, 2.5)], 2.0)],
            open_area: None,
            points: vec![],
            ..Default::default()
        });
        Arc::new(plan)
    }

    #[test]
    fn emergency_mode_switches_the_lamps_and_measures_the_route() {
        let mut v = Viewer::default();
        v.load_sample(3);
        v.set_plan(Some(emergency_room()));
        v.edit(|s| s.mode = ViewMode::Points);
        v.resize(64.0, 40.0, 1.0);
        v.update(0.016);
        // two tables stacked, the second lamp reads the second
        assert_eq!(v.ies_table().unwrap().tables, 2);
        let f = v.frame_input();
        assert_eq!(f.lamps.len(), 2);
        assert_eq!(f.lamps[1].r1[3], 1.0);
        // normal operation: the emergency luminaire is off
        assert_eq!(f.lamps[0].r0[3], 0.0);
        assert_eq!(f.lamps[1].r0[3], 1.0);
        assert!(v
            .scene
            .grids
            .iter()
            .all(|g| g.name == "workplane" || g.name == crate::scene::GRID_EYE));

        let mut sync = SyncSettings::for_scene(SceneKind::Planned);
        sync.mode = "points".into();
        sync.auto_quick = false;
        sync.emergency_mode = true;
        v.apply_sync(&sync);
        v.update(0.016);
        let f = v.frame_input();
        assert_eq!(f.lamps[0].r0[3], 1.0, "normal lighting off");
        assert_eq!(f.lamps[1].r0[3], 0.0, "emergency lighting on");
        assert_ne!(f.params.frame.w & 4, 0, "probes take direct light only");
        assert!(v.scene.grids.iter().any(|g| g.name == "escape route"));
        assert!(v.lux_max <= EMERGENCY_LUX_MAX);
        assert!(f.probe, "points are measured");

        let table = v.ies_table().unwrap().clone();
        let mut cpu = crate::cpu::CpuTracer::new();
        for _ in 0..4 {
            cpu.probe_pass(&f, &table, 16);
        }
        v.probe_values = cpu.probe_values().to_vec();
        let stats = v.grid_statistics();
        // the segments count as one grid, the band's edges as another
        assert_eq!(stats.len(), 2, "{stats:?}");
        let route = stats
            .iter()
            .find(|g| g.name == crate::scene::GRID_ROUTE)
            .unwrap();
        // 180 lm emergency optic at 2.95 m: a few lux on the floor below
        // it, far below the normal lighting's hundreds
        assert!(route.emax > 1.0 && route.emax < 60.0, "{route:?}");
        assert!(route.emin >= 0.0 && route.emin <= route.em, "{route:?}");
    }

    #[test]
    fn the_eye_adapts_after_the_power_fails() {
        let mut v = Viewer::default();
        v.load_sample(3);
        v.set_plan(Some(emergency_room()));
        v.resize(64.0, 40.0, 1.0);
        v.update(0.016);
        let normal = v.exposure;
        let mut sync = SyncSettings::for_scene(SceneKind::Planned);
        sync.emergency_mode = true;
        v.apply_sync(&sync);
        v.update(0.0);
        // the moment the power fails: the normal exposure
        assert!(
            (v.exposure / normal - 1.0).abs() < 1e-3,
            "{} vs {normal}",
            v.exposure
        );
        for _ in 0..300 {
            v.update(0.1);
        }
        // 30 s later: about four stops more, not more
        let stops = (v.exposure / normal).log2();
        assert!(
            stops > 3.9 && stops <= EMERGENCY_ADAPT_STOPS + 1e-4,
            "{stops}"
        );
        assert!(v.status().adaptation > 0.99);
        // power failure again: back to the start
        sync.emergency_replay += 1;
        v.apply_sync(&sync);
        v.update(0.0);
        assert!((v.exposure / normal - 1.0).abs() < 1e-3);
        // power back: the normal exposure at once
        sync.emergency_mode = false;
        v.apply_sync(&sync);
        v.update(0.0);
        assert!((v.exposure / normal - 1.0).abs() < 1e-6);
        assert_eq!(v.status().adaptation, 0.0);
    }

    #[test]
    fn walking_the_route_and_its_markings() {
        let mut plan = (*emergency_room()).clone();
        // an L-shaped route and a fire extinguisher on the far wall
        plan.emergency.as_mut().unwrap().routes = vec![(
            vec![
                Vec2::new(0.5, 1.0),
                Vec2::new(5.0, 1.0),
                Vec2::new(5.0, 4.5),
            ],
            1.0,
        )];
        plan.emergency.as_mut().unwrap().points = vec![crate::scene::PlanPoint {
            position: Vec2::new(3.0, 5.0),
            height: 1.2,
            facing_deg: 270.0,
            symbol: "F".into(),
            vertical: true,
        }];
        let mut v = Viewer::default();
        v.load_sample(3);
        v.set_plan(Some(Arc::new(plan)));
        v.resize(320.0, 200.0, 1.0);
        v.update(0.016);
        assert_eq!(v.status().routes.len(), 1);
        assert!((v.status().routes[0] - 8.0).abs() < 1e-4);
        assert_eq!(v.status().emergency_lamps, 1);

        // overview: one band per segment, one centre line, the extinguisher
        let m = v.emergency_marks(320.0, 200.0).expect("markings");
        assert_eq!(m.bands.len(), 2);
        assert_eq!(m.centre_lines.len(), 1);
        assert_eq!(m.points.len(), 1);
        assert!(m.points[0].lux.is_none(), "measured in emergency mode only");

        // walk half way: the eye is on the route at eye height, looking on
        let mut sync = SyncSettings::for_scene(SceneKind::Planned);
        sync.emergency_mode = true;
        sync.walk = Some(0.25);
        v.apply_sync(&sync);
        v.update(0.016);
        let eye = v.camera.eye();
        assert!(
            (eye - Vec3::new(2.5, WALK_EYE_HEIGHT, 1.0)).length() < 0.05,
            "{eye}"
        );
        let ahead = v.camera.view.target - eye;
        assert!(
            ahead.x > 0.4 && ahead.y < 0.0,
            "looks along the route: {ahead}"
        );
        // the band behind the eye is cut at the camera, nothing is NaN
        let m = v.emergency_marks(320.0, 200.0).expect("markings");
        assert!(!m.bands.is_empty());
        for p in m
            .bands
            .iter()
            .flatten()
            .chain(m.centre_lines.iter().flatten())
        {
            assert!(p.0.is_finite() && p.1.is_finite());
        }
        // the extinguisher is measured vertically, facing into the room
        let g = v
            .scene
            .grids
            .iter()
            .find(|g| g.name == GRID_EMPHASIS)
            .expect("vertical grid");
        assert!((g.receiver.normal - Vec3::NEG_Z).length() < 1e-5);
        assert!(v
            .layout
            .points
            .iter()
            .any(|p| v.scene.grids[p.grid].name == GRID_EMPHASIS));

        // playing walks on and stops at the end
        sync.walk_play = true;
        v.apply_sync(&sync);
        for _ in 0..200 {
            v.update(0.05);
        }
        assert_eq!(v.status().walk, Some(1.0));
        assert!(!v.status().walk_playing);
        let eye = v.camera.eye();
        assert!(
            (eye.x - 5.0).abs() < 0.05 && (eye.z - 4.5).abs() < 0.05,
            "{eye}"
        );

        // markings off = as the escaping person sees it
        sync.emergency_marks = false;
        v.apply_sync(&sync);
        assert!(v.emergency_marks(320.0, 200.0).is_none());
        // back to the overview
        sync.walk = None;
        v.apply_sync(&sync);
        assert_eq!(v.status().walk, None);
    }

    /// The landing page's L-shaped office (crates/eulumdat/tests/emergency.rs
    /// `landing_page_l_office`): the Light scene's emergency check agrees
    /// with the EN 1838 core.
    #[test]
    fn emergency_scene_matches_the_en1838_core() {
        use eulumdat::emergency::{self, EmphasisKind, EmphasisPoint, EscapeRoute, Limits};
        let ldt = Eulumdat::parse(include_str!(
            "../../eulumdat/tests/fixtures/emergency/demo_em_route.ldt"
        ))
        .unwrap();
        let limits = Limits::en1838();
        let set = emergency::Settings::default();
        let room = vec![
            (0.0, 0.0),
            (20.0, 0.0),
            (20.0, 7.5),
            (10.0, 7.5),
            (10.0, 15.0),
            (0.0, 15.0),
        ];
        let routes = vec![EscapeRoute {
            name: "1".into(),
            path: vec![(19.5, 3.75), (5.0, 3.75), (5.0, 14.5)],
            width: 2.0,
            place: Default::default(),
        }];
        let pt = |kind, x, y, f| EmphasisPoint {
            kind,
            x,
            y,
            z: None,
            facing: Some(f),
        };
        let points = vec![
            pt(EmphasisKind::FireEquipment, 15.0, 7.4, 270.0),
            pt(EmphasisKind::FirstAid, 0.1, 10.0, 0.0),
        ];
        let flux = emergency::luminaire_flux_lm(&ldt);
        let spacing = emergency::spacing_table(&ldt, flux, &[3.0], 2.0, &limits, &set).remove(0);
        let input = emergency::AutoInput {
            ldt: &ldt,
            flux_lm: flux,
            height: 3.0,
            ra: Some(80.0),
            spacing: &spacing,
            routes: &routes,
            room: Some(&room),
            open_area: Some(&room),
            open_area_place: emergency::PlaceCategory::CrowdedPlace,
            points: &points,
        };
        let pos = emergency::auto_layout(&input, &limits, &set);
        let rep = emergency::evaluate(&input.plan(&pos), &limits, &set);

        // the same plan as the Interior designer hands it to the Light scene
        let v2 = |&(x, y): &(f64, f64)| Vec2::new(x as f32, y as f32);
        let mf = set.maintenance_factor;
        let plan = ScenePlan {
            name: "L office".into(),
            indoor: true,
            outline: room.iter().map(v2).collect(),
            height: 3.0,
            reflectances: [0.7, 0.5, 0.2],
            plane: 0.8,
            luminaires: pos
                .iter()
                .map(|&(x, y, r)| crate::scene::PlanLuminaire {
                    position: Vec3::new(x as f32, 2.95, y as f32),
                    rotation_deg: r as f32,
                    photometry: 1,
                    role: crate::scene::LampRole::Emergency,
                    ..Default::default()
                })
                .collect(),
            photometries: vec![crate::scene::PlanPhotometry {
                name: "EM".into(),
                ldt: emergency::at_flux(&ldt, mf * flux).to_ldt(),
            }],
            emergency: Some(crate::scene::PlanEmergency {
                routes: vec![(routes[0].path.iter().map(v2).collect(), 2.0)],
                open_area: Some(room.iter().map(v2).collect()),
                points: points
                    .iter()
                    .map(|p| crate::scene::PlanPoint {
                        position: Vec2::new(p.x as f32, p.y as f32),
                        height: set.emphasis_height_m as f32,
                        facing_deg: p.facing.unwrap() as f32,
                        symbol: "F".into(),
                        vertical: true,
                    })
                    .collect(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut v = Viewer::default();
        v.load_sample(3);
        v.set_plan(Some(Arc::new(plan)));
        let mut sync = SyncSettings::for_scene(SceneKind::Planned);
        sync.mode = "points".into();
        sync.auto_quick = false;
        sync.emergency_mode = true;
        v.apply_sync(&sync);
        v.resize(64.0, 40.0, 1.0);
        v.update(0.016);
        let f = v.frame_input();
        let table = v.ies_table().unwrap().clone();
        let mut cpu = crate::cpu::CpuTracer::new();
        for _ in 0..2 {
            cpu.probe_pass(&f, &table, 8);
        }
        v.probe_values = cpu.probe_values().to_vec();
        let stats = v.grid_statistics();
        let get = |n: &str| stats.iter().find(|g| g.name == n).unwrap().clone();
        let (route, area, eq) = (
            get(crate::scene::GRID_ROUTE),
            get(crate::scene::GRID_OPEN_AREA),
            get(GRID_EMPHASIS),
        );
        let vertical: Vec<f32> = v
            .layout
            .points
            .iter()
            .zip(&v.probe_values)
            .filter(|(p, _)| v.scene.grids[p.grid].name == GRID_EMPHASIS)
            .map(|(_, e)| *e)
            .collect();
        let core = [
            rep.routes[0].centre.min_lx,
            rep.open_areas[0].stats.min_lx,
            rep.points[0].vertical_lx.unwrap(),
            rep.points[1].vertical_lx.unwrap(),
        ];
        let scene = [route.emin, area.emin, vertical[0], vertical[1]];
        // the central band: its edges with the centre line
        let band = get(crate::scene::GRID_ROUTE_BAND).emin.min(route.emin) as f64;
        let core_band = rep.routes[0].band.min_lx;
        assert!(
            (band - core_band).abs() / core_band < 0.08,
            "band: core {core_band:.2} scene {band:.2}"
        );
        eprintln!(
            "core {core:.2?} | scene {scene:.2?} | equipment Emin {:.2}",
            eq.emin
        );
        // direct light both; the scene's luminaires hang 5 cm lower, its
        // route points are 0.5 m apart (core 0.1 m), and it is traced
        for (c, s) in core.iter().zip(scene) {
            assert!(
                (s as f64 - c).abs() / c < 0.06,
                "core {core:.2?} scene {scene:.2?}"
            );
        }
    }

    #[test]
    fn street_plan_matches_the_street_designer() {
        // the Street designer's default road with the road luminaire,
        // opposite poles: its middle span against the same span traced
        use eulumdat::street::{Arrangement, StreetLayout};
        let ldt = Eulumdat::parse(include_str!(
            "../../eulumdat-wasm/templates/road_luminaire.ldt"
        ))
        .unwrap();
        let layout = StreetLayout {
            arrangement: Arrangement::Opposite,
            ..StreetLayout::default()
        };
        let core = layout.compute(&ldt, 1.0);
        // three spans, as the designer computes; the grid over the middle
        // one comes from cropping the traced points to it
        let s = layout.pole_spacing_m as f32;
        let strip = StreetLayout {
            length_m: 3.0 * layout.pole_spacing_m,
            ..layout.clone()
        };
        let plan = crate::scene::plan_from_street(&strip, "street".into());
        assert!(plan.street);
        assert_eq!(plan.luminaires.len(), 8, "4 positions × 2 sides");
        let mut v = Viewer::default();
        v.load("road", ldt, Some(SceneKind::Planned));
        v.set_plan(Some(Arc::new(plan)));
        let mut sync = SyncSettings::for_scene(SceneKind::Planned);
        sync.mode = "points".into();
        sync.auto_quick = false;
        sync.bounces = 1;
        v.apply_sync(&sync);
        v.resize(64.0, 40.0, 1.0);
        v.update(0.016);
        let f = v.frame_input();
        let table = v.ies_table().unwrap().clone();
        let mut cpu = crate::cpu::CpuTracer::new();
        for _ in 0..2 {
            cpu.probe_pass(&f, &table, 8);
        }
        v.probe_values = cpu.probe_values().to_vec();
        let mid: Vec<f32> = v
            .layout
            .points
            .iter()
            .zip(&v.probe_values)
            .filter(|(p, _)| p.position.x >= s && p.position.x <= 2.0 * s)
            .map(|(_, e)| *e)
            .collect();
        let em = mid.iter().sum::<f32>() / mid.len() as f32;
        eprintln!(
            "street: designer avg {:.1} min {:.1} | scene middle span Em {:.1} ({} points)",
            core.avg_lux,
            core.min_lux,
            em,
            mid.len()
        );
        assert!(
            (em as f64 - core.avg_lux).abs() / core.avg_lux < 0.1,
            "designer {:.1} scene {:.1}",
            core.avg_lux,
            em
        );
    }

    #[test]
    fn outdoor_plan_matches_the_area_designer() {
        // the Outdoor designer's 2 × 3 grid on 60 × 40 m, road luminaire at 8 m
        use eulumdat::area::{compute_area_illuminance, LuminairePlace};
        let ldt = Eulumdat::parse(include_str!(
            "../../eulumdat-wasm/templates/road_luminaire.ldt"
        ))
        .unwrap();
        let poles: Vec<(f64, f64)> = [10.0, 30.0, 50.0]
            .iter()
            .flat_map(|&x| [10.0, 30.0].map(|y| (x, y)))
            .collect();
        let places: Vec<LuminairePlace> = poles
            .iter()
            .enumerate()
            .map(|(i, &(x, y))| LuminairePlace::simple(i, x, y, 8.0))
            .collect();
        let core = compute_area_illuminance(&ldt, &places, 60.0, 40.0, 40, 1.0);
        let plan = ScenePlan {
            name: "60 × 40 m".into(),
            indoor: false,
            outline: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(60.0, 0.0),
                Vec2::new(60.0, 40.0),
                Vec2::new(0.0, 40.0),
            ],
            reflectances: [0.0, 0.0, 0.1],
            luminaires: poles
                .iter()
                .map(|&(x, y)| crate::scene::PlanLuminaire {
                    position: Vec3::new(x as f32, 8.0, y as f32),
                    pole: Some(Vec2::new(x as f32, y as f32)),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        };
        let mut v = Viewer::default();
        v.load("road", ldt, Some(SceneKind::Planned));
        v.set_plan(Some(Arc::new(plan)));
        let mut sync = SyncSettings::for_scene(SceneKind::Planned);
        sync.mode = "points".into();
        sync.auto_quick = false;
        sync.bounces = 1;
        v.apply_sync(&sync);
        v.resize(64.0, 40.0, 1.0);
        v.update(0.016);
        let f = v.frame_input();
        let table = v.ies_table().unwrap().clone();
        let mut cpu = crate::cpu::CpuTracer::new();
        for _ in 0..2 {
            cpu.probe_pass(&f, &table, 8);
        }
        v.probe_values = cpu.probe_values().to_vec();
        let g = v.grid_statistics()[0].clone();
        eprintln!(
            "core avg {:.1} min {:.1} max {:.1} | scene Em {:.1} Emin {:.2} ({} points)",
            core.avg_lux,
            core.min_lux,
            core.max_lux,
            g.em,
            g.emin,
            v.layout.points.len()
        );
        assert!(
            (g.em as f64 - core.avg_lux).abs() / core.avg_lux < 0.08,
            "core {:.1} scene {:.1}",
            core.avg_lux,
            g.em
        );
    }

    #[test]
    fn brighter_walls_give_more_light_on_the_workplane() {
        let em = |rho: f32| {
            let mut v = Viewer::default();
            v.load_sample(3);
            v.set_plan(Some(plan_room(rho)));
            v.edit(|s| {
                s.mode = ViewMode::Points;
                s.bounces = 4;
            });
            v.resize(64.0, 40.0, 1.0);
            v.update(0.016);
            let table = v.photometry.as_ref().unwrap().table.clone();
            let input = v.frame_input();
            let mut cpu = crate::cpu::CpuTracer::new();
            for _ in 0..6 {
                cpu.probe_pass(&input, &table, 16);
            }
            v.probe_values = cpu.probe_values().to_vec();
            v.grid_statistics()[0].em
        };
        let (dark, light) = (em(0.1), em(0.8));
        assert!(light > dark * 1.08, "{light} vs {dark}");
    }

    #[test]
    fn a_scene_doc_round_trips() {
        let mut v = Viewer::default();
        v.load_sample(0);
        v.set_plan(Some(plan_room(0.6)));
        let mut sync = SyncSettings::for_scene(SceneKind::Planned);
        sync.sensors = 1;
        sync.apply_quality("quick");
        v.apply_sync(&sync);
        v.resize(400.0, 300.0, 1.0);
        for _ in 0..30 {
            v.update(0.1);
        }
        v.move_object(ObjectId::Person, Vec2::new(4.0, 3.0));
        v.move_object(ObjectId::Sensor(0), Vec2::new(2.0, 2.0));
        v.update(0.016);
        let before = v.doc_fingerprint();
        let doc = v.scene_doc(&sync, "abc", "My room", Some("lum1".into()));
        let json = serde_json::to_string(&doc).unwrap();
        let back: crate::doc::SceneDoc = serde_json::from_str(&json).unwrap();
        assert_eq!(back, doc);
        assert!(back.plan.is_some());
        assert_eq!(back.luminaire.as_deref(), Some("lum1"));

        // a fresh viewer in another scene shows it again
        let mut w = Viewer::default();
        w.load_sample(0);
        w.apply_sync(&SyncSettings::for_scene(SceneKind::Street));
        w.resize(400.0, 300.0, 1.0);
        w.update(0.016);
        w.apply_doc(&back);
        w.apply_sync(&back.settings); // the host's panel applies them again
        w.update(0.016);
        assert_eq!(w.scene.kind, SceneKind::Planned);
        assert_eq!(w.settings.quality, Quality::Quick);
        let person = w
            .scene
            .movables
            .iter()
            .find(|m| m.id == ObjectId::Person)
            .unwrap();
        assert!((person.anchor.x - 4.0).abs() < 1e-5 && (person.anchor.z - 3.0).abs() < 1e-5);
        assert_eq!(
            w.settings.params.sensors,
            vec![SceneSensor { x: 2.0, z: 2.0 }]
        );
        assert_eq!(w.camera.view, v.camera.view);
        assert_eq!(w.doc_fingerprint(), before);
        // older docs without the optional parts still read
        let min = r#"{"id":"x","name":"n","settings":{"scene":"street"}}"#;
        let d: crate::doc::SceneDoc = serde_json::from_str(min).unwrap();
        assert_eq!(d.version, crate::doc::SCENE_DOC_VERSION);
        assert!(d.moves.is_empty() && d.plan.is_none());
    }
}
