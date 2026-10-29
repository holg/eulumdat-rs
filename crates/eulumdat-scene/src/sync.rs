//! Types exchanged with a host UI (the editor panel): settings in, status out.

use crate::scene::{SceneKind, SceneParams};
use serde::{Deserialize, Serialize};

/// Settings a host sends. Missing fields keep the defaults below.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncSettings {
    /// street | parking | facade | office
    pub scene: String,
    /// render | isolux | points
    pub mode: String,
    pub isolines: bool,
    pub height: f32,
    pub tilt: f32,
    /// `None` aims the throw automatically.
    pub rotation: Option<f32>,
    pub count: u32,
    pub spacing: f32,
    pub wall_distance: f32,
    pub ceiling_height: f32,
    /// render | brick
    pub wall_finish: String,
    /// single | opposite | staggered
    pub street_layout: String,
    pub show_figure: bool,
    /// `None` takes the colour temperature from the file.
    pub cct: Option<f32>,
    pub haze: f32,
    pub glare: bool,
    pub exposure_ev: f32,
    pub bounces: u32,
    /// Draw the status strip, help, legend and statistics on the canvas.
    pub overlay: bool,
    /// Keep tracing; a host pauses the viewer while it is hidden.
    pub active: bool,
    /// Bumped by the host to return to the home view.
    pub reset_view: u64,
    /// Samples per pixel before the picture is done.
    pub samples: u32,
    /// Samples per measurement point before the points are done.
    pub point_samples: u32,
    /// Trace passes per frame.
    pub passes: u32,
    /// gpu | cpu — which tracer the host runs (the viewer itself does not care).
    pub compute: String,
}

impl SyncSettings {
    /// Placement defaults for a scene.
    pub fn for_scene(kind: SceneKind) -> Self {
        let p = SceneParams::defaults(kind);
        Self {
            scene: scene_id(kind).to_string(),
            mode: "render".into(),
            isolines: false,
            height: p.height,
            tilt: 0.0,
            rotation: None,
            count: 1,
            spacing: p.spacing,
            wall_distance: p.wall_distance,
            ceiling_height: p.ceiling_height,
            wall_finish: "render".into(),
            street_layout: "single".into(),
            show_figure: true,
            cct: None,
            haze: 0.05,
            glare: true,
            exposure_ev: 0.0,
            bounces: 3,
            overlay: false,
            active: true,
            reset_view: 0,
            samples: crate::gpu::MAX_SAMPLES,
            point_samples: crate::gpu::PROBE_MAX_SAMPLES,
            passes: 1,
            compute: "gpu".into(),
        }
    }
}

impl Default for SyncSettings {
    fn default() -> Self {
        Self::for_scene(SceneKind::Street)
    }
}

pub fn scene_id(kind: SceneKind) -> &'static str {
    match kind {
        SceneKind::Street => "street",
        SceneKind::Parking => "parking",
        SceneKind::Facade => "facade",
        SceneKind::Office => "office",
    }
}

pub fn scene_from_id(id: &str) -> SceneKind {
    match id {
        "parking" => SceneKind::Parking,
        "facade" => SceneKind::Facade,
        "office" => SceneKind::Office,
        _ => SceneKind::Street,
    }
}

/// Per-grid illuminance statistics: average, minimum and uniformity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GridStat {
    pub name: String,
    pub em: f32,
    pub emin: f32,
    pub uo: f32,
}

/// What the viewer reports back to a host.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Status {
    pub name: String,
    pub lumens: f64,
    pub peak_cd: f32,
    pub throw_c: f64,
    pub asymmetry: f64,
    pub cct_file: Option<f32>,
    pub cct: f32,
    pub rotation: f32,
    pub samples: u32,
    pub max_samples: u32,
    pub probe_samples: u32,
    pub probe_max_samples: u32,
    /// Measurement points in the current grids.
    pub point_count: u32,
    /// Triangles of the L3D body, 0 when the housing box is drawn.
    pub body_tris: u32,
    pub lux_max: f32,
    pub iso_step: f32,
    pub key_lux: f32,
    pub grids: Vec<GridStat>,
}
