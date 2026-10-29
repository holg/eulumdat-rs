//! Types exchanged with a host UI (the editor panel): settings in, status out.

use crate::scene::{SceneKind, SceneParams};
use serde::{Deserialize, Serialize};

fn default_canopy() -> f32 {
    1.0
}
fn default_photoperiod() -> f32 {
    16.0
}

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
    /// Greenhouse: canopy height (m) and photoperiod (h/day).
    #[serde(default = "default_canopy")]
    pub canopy_height: f32,
    #[serde(default = "default_photoperiod")]
    pub photoperiod: f32,
    /// µmol per (lx·s) of the loaded spectrum; `None` = assume a white LED.
    #[serde(default)]
    pub ppfd_factor: Option<f32>,
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
    /// Apply `key=value` pairs from a link (`scene=street&mode=points&height=8`).
    /// `scene` is applied first and resets the placement to that scene's
    /// defaults; unknown keys and unparsable values are ignored. Returns
    /// true when at least one key was recognised.
    pub fn apply_query(&mut self, pairs: &[(String, String)]) -> bool {
        let mut any = false;
        if let Some((_, v)) = pairs.iter().find(|(k, _)| k == "scene") {
            *self = Self::for_scene(scene_from_id(v));
            any = true;
        }
        let f = |v: &str| v.parse::<f32>().ok();
        let b = |v: &str| matches!(v, "1" | "true" | "on" | "yes");
        for (k, v) in pairs {
            let hit = match k.as_str() {
                "scene" => true,
                "mode" | "view" => {
                    self.mode = match v.as_str() {
                        "photo" | "render" => "render".into(),
                        "isolux" | "illuminance" | "lux" => "isolux".into(),
                        "points" | "grid" => "points".into(),
                        _ => self.mode.clone(),
                    };
                    self.isolines = self.mode == "isolux";
                    true
                }
                "isolines" => {
                    self.isolines = b(v);
                    true
                }
                "height" => f(v).map(|x| self.height = x).is_some(),
                "tilt" => f(v).map(|x| self.tilt = x).is_some(),
                "rotation" => f(v).map(|x| self.rotation = Some(x)).is_some(),
                "count" => v
                    .parse::<u32>()
                    .map(|x| self.count = x.clamp(1, 16))
                    .is_ok(),
                "spacing" => f(v).map(|x| self.spacing = x).is_some(),
                "wall" | "wall_distance" => f(v).map(|x| self.wall_distance = x).is_some(),
                "ceiling" | "ceiling_height" => f(v).map(|x| self.ceiling_height = x).is_some(),
                "finish" => {
                    self.wall_finish = v.clone();
                    true
                }
                "layout" => {
                    self.street_layout = v.clone();
                    true
                }
                "figure" => {
                    self.show_figure = b(v);
                    true
                }
                "canopy" | "canopy_height" => f(v).map(|x| self.canopy_height = x).is_some(),
                "photoperiod" | "hours" => f(v).map(|x| self.photoperiod = x).is_some(),
                "ppfd" | "ppfd_factor" => f(v).map(|x| self.ppfd_factor = Some(x)).is_some(),
                "cct" => f(v).map(|x| self.cct = Some(x)).is_some(),
                "haze" => f(v).map(|x| self.haze = x.clamp(0.0, 1.0)).is_some(),
                "glare" => {
                    self.glare = b(v);
                    true
                }
                "ev" | "exposure" => f(v).map(|x| self.exposure_ev = x).is_some(),
                "bounces" => v
                    .parse::<u32>()
                    .map(|x| self.bounces = x.clamp(1, 6))
                    .is_ok(),
                "samples" => v.parse::<u32>().map(|x| self.samples = x).is_ok(),
                "compute" => {
                    self.compute = v.clone();
                    true
                }
                _ => false,
            };
            any |= hit;
        }
        any
    }

    pub fn for_scene(kind: SceneKind) -> Self {
        let p = SceneParams::defaults(kind);
        Self {
            scene: scene_id(kind).to_string(),
            mode: "render".into(),
            isolines: false,
            height: p.height,
            tilt: 0.0,
            rotation: None,
            count: p.count,
            spacing: p.spacing,
            wall_distance: p.wall_distance,
            ceiling_height: p.ceiling_height,
            wall_finish: "render".into(),
            street_layout: "single".into(),
            show_figure: true,
            canopy_height: p.canopy_height,
            photoperiod: p.photoperiod,
            ppfd_factor: None,
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
        SceneKind::Greenhouse => "greenhouse",
    }
}

pub fn scene_from_id(id: &str) -> SceneKind {
    match id {
        "parking" => SceneKind::Parking,
        "facade" => SceneKind::Facade,
        "office" => SceneKind::Office,
        "greenhouse" | "grow" => SceneKind::Greenhouse,
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
    /// Display factor µmol/(lx·s) for PPFD scenes, 0 when values are lux.
    pub ppfd_factor: f32,
    /// True when `ppfd_factor` came from the luminaire's spectrum.
    pub ppfd_from_spectrum: bool,
    pub photoperiod: f32,
    pub lux_max: f32,
    pub iso_step: f32,
    pub key_lux: f32,
    pub grids: Vec<GridStat>,
}

#[cfg(test)]
mod tests {
    #[test]
    fn query_sets_scene_then_fields() {
        let mut s = super::SyncSettings::default();
        let pairs: Vec<(String, String)> = [
            ("mode", "grid"),
            ("scene", "office"),
            ("height", "2.6"),
            ("count", "4"),
            ("junk", "x"),
        ]
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        assert!(s.apply_query(&pairs));
        assert_eq!(s.scene, "office");
        assert_eq!(s.mode, "points");
        assert!((s.height - 2.6).abs() < 1e-6);
        assert_eq!(s.count, 4);
    }
}
