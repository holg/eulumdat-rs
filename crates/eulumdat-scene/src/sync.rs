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
    /// Presence / motion sensors in the scene (0 = none, at most
    /// `scene::MAX_SENSORS`); where they stand is the viewer's (dragged).
    #[serde(default)]
    pub sensors: u32,
    /// Sensor height (m); 0 = the scene's default.
    #[serde(default)]
    pub sensor_height: f32,
    /// tangential | radial | presence
    #[serde(default = "default_sensor_motion")]
    pub sensor_motion: String,
    /// together | nearest
    #[serde(default = "default_sensor_control")]
    pub sensor_control: String,
    /// Output (0–1) of lamps whose sensor detects nothing.
    #[serde(default = "default_sensor_idle")]
    pub sensor_idle: f32,
    /// The sensors switch the luminaires (to `sensor_idle` when nothing
    /// is detected); off = they only detect, the luminaires stay at full
    /// output. A plan from a designer arrives with it off.
    #[serde(default = "default_true")]
    pub sensor_dim: bool,
    /// quick | balanced | high (`viewer::Quality`); the panel's presets
    /// also set `bounces` and `samples`.
    #[serde(default = "default_quality")]
    pub quality: String,
    /// Smooth the reflected-light noise of the picture (display only).
    #[serde(default = "default_true")]
    pub denoise: bool,
    /// Quick while the camera or an object moves.
    #[serde(default = "default_true")]
    pub auto_quick: bool,
    /// Draw each sensor's detection volume in 3D over the picture.
    #[serde(default)]
    pub sensor_volume: bool,
    /// Emergency mode (EN 1838): normal lighting off, emergency luminaires on.
    #[serde(default)]
    pub emergency_mode: bool,
    /// Draw the plan's emergency markings (route band, open area,
    /// equipment) over the picture; off = as a person escaping sees it.
    #[serde(default = "default_true")]
    pub emergency_marks: bool,
    /// Walk an escape route at eye height: position along it (0–1), `None`
    /// = the orbit camera.
    #[serde(default)]
    pub walk: Option<f32>,
    /// Which route to walk.
    #[serde(default)]
    pub walk_route: u32,
    /// Walk on by itself (at walking pace) from `walk`.
    #[serde(default)]
    pub walk_play: bool,
    /// Bumped to replay the power failure (the eye adapts again).
    #[serde(default)]
    pub emergency_replay: u32,
    /// The moon over outdoor scenes ([`crate::moon`]): on, the night
    /// (local date "YYYY-MM-DD" and wall-clock hour), the place (latitude,
    /// longitude, standard UTC offset, DST rule "eu" | "us" | "south" |
    /// "none") and north in the scene (degrees from −Z towards +X).
    #[serde(default)]
    pub moon: bool,
    #[serde(default = "default_moon_date")]
    pub moon_date: String,
    #[serde(default = "default_moon_hour")]
    pub moon_hour: f32,
    #[serde(default = "default_moon_lat")]
    pub moon_lat: f64,
    #[serde(default = "default_moon_lon")]
    pub moon_lon: f64,
    #[serde(default = "default_moon_utc")]
    pub moon_utc: f32,
    #[serde(default = "default_moon_dst")]
    pub moon_dst: String,
    #[serde(default)]
    pub north_deg: f32,
    /// All luminaires off: the scene in moonlight alone.
    #[serde(default)]
    pub lamps_off: bool,
}

fn default_moon_date() -> String {
    // a full moon
    "2026-01-03".into()
}

fn default_moon_hour() -> f32 {
    23.0
}

fn default_moon_lat() -> f64 {
    52.52
}

fn default_moon_lon() -> f64 {
    13.40
}

fn default_moon_utc() -> f32 {
    1.0
}

fn default_moon_dst() -> String {
    "eu".into()
}

fn default_true() -> bool {
    true
}

fn default_quality() -> String {
    "high".into()
}

fn default_sensor_motion() -> String {
    "tangential".into()
}

fn default_sensor_control() -> String {
    "together".into()
}

fn default_sensor_idle() -> f32 {
    0.1
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
                "sensors" => v
                    .parse::<u32>()
                    .map(|x| self.sensors = x.min(crate::scene::MAX_SENSORS as u32))
                    .is_ok(),
                "sensorh" | "sensor_height" => f(v).map(|x| self.sensor_height = x).is_some(),
                "sensormotion" => {
                    self.sensor_motion = v.clone();
                    true
                }
                "sensorctl" => {
                    self.sensor_control = v.clone();
                    true
                }
                "sensordim" => {
                    self.sensor_dim = b(v);
                    true
                }
                "quality" => {
                    self.quality = v.clone();
                    true
                }
                "denoise" => {
                    self.denoise = b(v);
                    true
                }
                "autoquick" => {
                    self.auto_quick = b(v);
                    true
                }
                "sensorvol" => {
                    self.sensor_volume = b(v);
                    true
                }
                "emergency" => {
                    self.emergency_mode = b(v);
                    true
                }
                "emmarks" => {
                    self.emergency_marks = b(v);
                    true
                }
                "walk" => f(v).map(|x| self.walk = Some(x.clamp(0.0, 1.0))).is_some(),
                "moon" | "sky" => {
                    self.moon = b(v);
                    true
                }
                "moondate" => crate::moon::parse_date(v)
                    .map(|_| self.moon_date = v.to_string())
                    .is_some(),
                "moontime" => f(v).map(|x| self.moon_hour = x.clamp(0.0, 23.99)).is_some(),
                "moonlat" => v
                    .parse::<f64>()
                    .map(|x| self.moon_lat = x.clamp(-90.0, 90.0))
                    .is_ok(),
                "moonlon" => v
                    .parse::<f64>()
                    .map(|x| self.moon_lon = x.clamp(-180.0, 180.0))
                    .is_ok(),
                "moonutc" => f(v).map(|x| self.moon_utc = x.clamp(-12.0, 14.0)).is_some(),
                "moondst" => {
                    self.moon_dst = v.to_string();
                    true
                }
                "north" => f(v).map(|x| self.north_deg = x.rem_euclid(360.0)).is_some(),
                "lampsoff" => {
                    self.lamps_off = b(v);
                    true
                }
                "walkroute" => f(v).map(|x| self.walk_route = x.max(0.0) as u32).is_some(),
                "sensoridle" => f(v).map(|x| self.sensor_idle = x.clamp(0.0, 1.0)).is_some(),
                _ => false,
            };
            any |= hit;
        }
        any
    }

    /// Quality preset: `quality` plus the bounces and samples that go with
    /// it (quick = direct light only, 64 samples; balanced = one bounce of
    /// reflected light, 256; high = three bounces, 768).
    pub fn apply_quality(&mut self, id: &str) {
        let (q, bounces, samples) = match id {
            "quick" => ("quick", 1, 64),
            "balanced" => ("balanced", 2, 256),
            _ => ("high", 3, crate::gpu::MAX_SAMPLES),
        };
        self.quality = q.into();
        self.bounces = bounces;
        self.samples = samples;
    }

    /// Another scene with this one's quality, tracer and sensor settings
    /// (the placement starts from the scene's defaults).
    pub fn switched_to(&self, kind: SceneKind) -> Self {
        Self {
            quality: self.quality.clone(),
            denoise: self.denoise,
            auto_quick: self.auto_quick,
            bounces: self.bounces,
            samples: self.samples,
            point_samples: self.point_samples,
            passes: self.passes,
            compute: self.compute.clone(),
            haze: self.haze,
            glare: self.glare,
            mode: self.mode.clone(),
            isolines: self.isolines,
            sensors: self.sensors,
            sensor_motion: self.sensor_motion.clone(),
            sensor_control: self.sensor_control.clone(),
            sensor_idle: self.sensor_idle,
            sensor_dim: self.sensor_dim,
            sensor_volume: self.sensor_volume,
            emergency_mode: self.emergency_mode,
            emergency_marks: self.emergency_marks,
            // a counter: kept, so the next bump is a change
            emergency_replay: self.emergency_replay,
            moon: self.moon,
            moon_date: self.moon_date.clone(),
            moon_hour: self.moon_hour,
            moon_lat: self.moon_lat,
            moon_lon: self.moon_lon,
            moon_utc: self.moon_utc,
            moon_dst: self.moon_dst.clone(),
            north_deg: self.north_deg,
            // another scene: its luminaires on
            lamps_off: false,
            ..Self::for_scene(kind)
        }
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
            sensors: 0,
            sensor_height: 0.0,
            sensor_motion: default_sensor_motion(),
            sensor_control: default_sensor_control(),
            sensor_idle: default_sensor_idle(),
            sensor_dim: true,
            quality: default_quality(),
            denoise: true,
            auto_quick: true,
            sensor_volume: false,
            emergency_mode: false,
            emergency_marks: true,
            walk: None,
            walk_route: 0,
            walk_play: false,
            emergency_replay: 0,
            moon: false,
            moon_date: default_moon_date(),
            moon_hour: default_moon_hour(),
            moon_lat: default_moon_lat(),
            moon_lon: default_moon_lon(),
            moon_utc: default_moon_utc(),
            moon_dst: default_moon_dst(),
            north_deg: 0.0,
            lamps_off: false,
        }
    }

    /// When and where the moon is taken, `None` when it is off.
    pub fn moon_time(&self) -> Option<crate::moon::MoonTime> {
        self.moon.then(|| crate::moon::MoonTime {
            date: self.moon_date.clone(),
            hour: self.moon_hour,
            latitude: self.moon_lat,
            longitude: self.moon_lon,
            utc_offset: self.moon_utc,
            dst: self.moon_dst.clone(),
            north_deg: self.north_deg,
        })
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
        SceneKind::Planned => "planned",
    }
}

pub fn scene_from_id(id: &str) -> SceneKind {
    match id {
        "parking" => SceneKind::Parking,
        "facade" => SceneKind::Facade,
        "office" => SceneKind::Office,
        "greenhouse" | "grow" => SceneKind::Greenhouse,
        "planned" => SceneKind::Planned,
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
    /// Highest point value (EN 1838 limits Emax / Emin along routes).
    #[serde(default)]
    pub emax: f32,
}

/// What the viewer reports back to a host.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Status {
    /// The natural light over the scene (on and outdoors), for the panel.
    pub sky: Option<crate::moon::SkyLight>,
    /// The scene is outdoors (the moon can light it).
    pub outdoor: bool,
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
    /// Sensors placed, and which of them detect something.
    pub sensors: u32,
    pub sensors_detecting: Vec<bool>,
    pub person_detected: bool,
    pub cars_detected: u32,
    /// Lamps at full output / all lamps.
    pub lamps_on: u32,
    pub lamps: u32,
    /// Ø of one sensor's detection area on the ground (m), `None` without
    /// a sensor file or when nothing is detected at that height.
    pub sensor_diameter: Option<f32>,
    /// Sensor height in use (m).
    pub sensor_height: f32,
    /// Objects moved by hand (luminaires, person, cars).
    pub moved: u32,
    /// Lamps that light in emergency mode (emergency or combined).
    pub emergency_lamps: u32,
    /// Length (m) of each escape route of the plan.
    pub routes: Vec<f32>,
    /// Walking a route: position along it (0–1) and whether it walks on.
    pub walk: Option<f32>,
    pub walk_playing: bool,
    /// Eye adaptation in emergency mode (0 = power just failed, 1 = adapted).
    pub adaptation: f32,
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
        // a scene switch keeps quality and sensors
        let mut q = super::SyncSettings::default();
        q.apply_quality("quick");
        q.sensors = 2;
        let o = q.switched_to(crate::scene::SceneKind::Office);
        assert_eq!(
            (o.quality.as_str(), o.bounces, o.samples, o.sensors),
            ("quick", 1, 64, 2)
        );
        assert_eq!(o.scene, "office");
        assert_eq!(s.mode, "points");
        assert!((s.height - 2.6).abs() < 1e-6);
        assert_eq!(s.count, 4);
    }
}
