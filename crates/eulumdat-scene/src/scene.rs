//! Parametric scenes for the path-traced viewer: primitives, lamps, calculation
//! surfaces and a home camera view.
//!
//! Bevy's Y-up frame: X runs along the road / facade, Z across it, Y is up.
//! The luminaire's own frame has nadir at −Y, C0 at +X and C90 at +Z, the
//! same as `photometric::mesh`. Asymmetric optics are aimed at −Z, where the
//! carriageway, the parking bays and the facade lie.
//!
//! Calculation grids follow EN 12464-1 (task areas, p = 0.2·5^log10(d)) and
//! EN 13201-3 (road: three rows per lane, longitudinal step ≤ 3 m). The
//! office measures on the 0.75 m workplane, not on the floor.

use eulumdat::{Eulumdat, PhotometricCalculations};
use glam::{Mat3, Quat, Vec2, Vec3};

/// Which place the luminaire is put in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum SceneKind {
    /// Two-lane urban street: parking strip, cycle lane, sidewalks, a
    /// building facade on the far side for spill light.
    #[default]
    Street,
    /// Parking lot with two rows of bays and an aisle.
    Parking,
    /// Building facade with window reveals and a forecourt.
    Facade,
    /// Office with two desks, workplane grid at 0.75 m.
    Office,
    /// Glasshouse with two crop benches; the grids lie on the canopy and
    /// the host shows them in PPFD.
    Greenhouse,
    /// Built from a planner (`SceneParams::plan`): the Interior designer's
    /// room or the Outdoor designer's area with their luminaires and sensors.
    Planned,
}

impl SceneKind {
    pub const ALL: [SceneKind; 5] = [
        SceneKind::Street,
        SceneKind::Parking,
        SceneKind::Facade,
        SceneKind::Office,
        SceneKind::Greenhouse,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SceneKind::Street => "Street",
            SceneKind::Parking => "Parking lot",
            SceneKind::Facade => "Facade",
            SceneKind::Office => "Office",
            SceneKind::Greenhouse => "Greenhouse",
            SceneKind::Planned => "Planned",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum StreetLayout {
    #[default]
    Single,
    Opposite,
    Staggered,
}

impl StreetLayout {
    pub fn next(self) -> Self {
        match self {
            StreetLayout::Single => StreetLayout::Opposite,
            StreetLayout::Opposite => StreetLayout::Staggered,
            StreetLayout::Staggered => StreetLayout::Single,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            StreetLayout::Single => "one side",
            StreetLayout::Opposite => "opposite",
            StreetLayout::Staggered => "staggered",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum WallFinish {
    #[default]
    Render,
    Brick,
}

/// Everything the user can change about the placement.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneParams {
    pub kind: SceneKind,
    /// Mounting height of the luminous opening above the floor (m).
    pub height: f32,
    /// Tilt in degrees about the axis along the road / facade (world X);
    /// positive raises the beam towards the road, the bays or the facade
    /// (−Z), independent of the yaw. Lamps on the far side tilt the other way.
    pub tilt: f32,
    /// Rotation of the C0 axis around the vertical in degrees.
    pub rotation: f32,
    pub count: u32,
    pub spacing: f32,
    pub wall_distance: f32,
    pub ceiling_height: f32,
    pub wall_finish: WallFinish,
    pub street_layout: StreetLayout,
    pub show_figure: bool,
    /// Greenhouse: top of the crop above the floor (m); the PPFD grids lie here.
    pub canopy_height: f32,
    /// Greenhouse: hours of light per day, for the daily light integral.
    pub photoperiod: f32,
    /// The luminaire stands on the ground (its body reaches down to it):
    /// no pole, no arm.
    pub freestanding: bool,
    /// Half extents of the luminaire housing (m), used to keep it clear of the ceiling.
    pub housing_half: Vec3,
    /// Presence / motion sensors (ground position; height `sensor_height`).
    pub sensors: Vec<SceneSensor>,
    /// Sensor height above the ground (m); in the office the sensors sit in
    /// the ceiling.
    pub sensor_height: f32,
    /// Objects moved by hand: the new ground position (x, z) of their anchor.
    pub moves: Vec<(ObjectId, Vec2)>,
    /// The plan of a `SceneKind::Planned` scene.
    pub plan: Option<std::sync::Arc<ScenePlan>>,
    /// Emergency mode: a planned scene measures its escape routes and open
    /// area on the floor (EN 1838) instead of the workplane.
    pub emergency: bool,
}

impl SceneParams {
    pub fn defaults(kind: SceneKind) -> Self {
        let base = Self {
            kind,
            height: 8.0,
            tilt: 0.0,
            rotation: 0.0,
            count: 1,
            spacing: 30.0,
            freestanding: false,
            wall_distance: 1.0,
            ceiling_height: 2.8,
            wall_finish: WallFinish::Render,
            street_layout: StreetLayout::Single,
            show_figure: true,
            canopy_height: 1.0,
            photoperiod: 16.0,
            housing_half: Vec3::new(0.1, 0.04, 0.1),
            sensors: Vec::new(),
            sensor_height: 5.0,
            moves: Vec::new(),
            plan: None,
            emergency: false,
        };
        match kind {
            SceneKind::Street => base,
            SceneKind::Parking => Self {
                height: 6.0,
                spacing: 18.0,
                ..base
            },
            SceneKind::Facade => Self {
                height: 3.0,
                spacing: 2.5,
                sensor_height: 3.0,
                ..base
            },
            SceneKind::Office => Self {
                height: 2.8 - CEILING_CLEARANCE,
                spacing: 2.0,
                sensor_height: 2.8,
                ..base
            },
            SceneKind::Greenhouse => Self {
                height: 2.2,
                spacing: 2.2,
                count: 8,
                ceiling_height: 4.0,
                sensor_height: 3.0,
                ..base
            },
            SceneKind::Planned => Self {
                height: 3.0,
                sensor_height: 3.0,
                ..base
            },
        }
    }

    /// Mounting height range for this scene.
    pub fn height_range(&self) -> (f32, f32) {
        match self.kind {
            SceneKind::Street => (4.0, 14.0),
            SceneKind::Parking => (3.0, 14.0),
            SceneKind::Facade => (0.2, 9.0),
            SceneKind::Office => (1.2, room_mount_limit(self).max(1.2)),
            SceneKind::Greenhouse => (
                self.canopy_height + 0.2,
                (GREENHOUSE_EAVE + GREENHOUSE_RISE - 0.15).max(self.canopy_height + 0.3),
            ),
            SceneKind::Planned => (0.2, 40.0),
        }
    }

    pub fn count_options(&self) -> &'static [u32] {
        match self.kind {
            SceneKind::Street => &[1, 2, 3, 4],
            SceneKind::Parking => &[1, 2, 3, 4, 6],
            SceneKind::Facade => &[1, 2, 3, 4, 5],
            SceneKind::Office => &[1, 2, 4, 6],
            SceneKind::Greenhouse => &[1, 2, 4, 6, 8, 12],
            SceneKind::Planned => &[],
        }
    }
}

/// Material ids; must match the `MAT_*` constants in `trace.wgsl`.
pub mod mat {
    pub const PAVING: u32 = 0;
    pub const ASPHALT: u32 = 1;
    pub const PARKING: u32 = 2;
    pub const CYCLE: u32 = 3;
    pub const KERB: u32 = 4;
    pub const RENDER: u32 = 5;
    pub const BRICK: u32 = 6;
    pub const GLASS: u32 = 7;
    pub const CARPET: u32 = 8;
    pub const WALL: u32 = 9;
    pub const CEILING: u32 = 10;
    pub const METAL: u32 = 11;
    pub const DESK: u32 = 12;
    pub const CAR: u32 = 13;
    pub const FABRIC: u32 = 14;
    pub const PERSON: u32 = 15;
    pub const JACKET: u32 = 16;
    pub const HEDGE: u32 = 17;
    pub const SOIL: u32 = 18;
    pub const WHITE: u32 = 19;
    pub const RUBBER: u32 = 20;
    pub const RIM: u32 = 21;
    pub const PLASTIC: u32 = 22;
    pub const SKIN: u32 = 23;
    pub const TROUSERS: u32 = 24;
    pub const SHOE: u32 = 25;
    pub const HEADLIGHT: u32 = 26;
    pub const TAILLIGHT: u32 = 27;
    pub const CAR_SILVER: u32 = 28;
    pub const CAR_RED: u32 = 29;
    /// Luminaire body from an L3D mesh.
    pub const HOUSING: u32 = 30;
    /// Light-emitting face of an L3D mesh (shown by primary rays only).
    pub const LUMINOUS: u32 = 31;
}

/// Analytic primitives the tracer intersects.
#[derive(Clone, Debug)]
pub enum Shape {
    /// Axis-aligned box.
    Box {
        min: Vec3,
        max: Vec3,
    },
    /// Oriented box: `rot` is local → world.
    Obb {
        center: Vec3,
        half: Vec3,
        rot: Mat3,
    },
    Sphere {
        center: Vec3,
        radius: f32,
    },
    /// Segment `a`–`b` with hemispherical ends.
    Capsule {
        a: Vec3,
        b: Vec3,
        radius: f32,
    },
    /// Capped cylinder along `a`–`b`.
    Cylinder {
        a: Vec3,
        b: Vec3,
        radius: f32,
    },
}

#[derive(Clone, Debug)]
pub struct ScenePrim {
    pub shape: Shape,
    pub material: u32,
    pub casts_shadow: bool,
    /// Hidden from the camera (but still bouncing light) when the camera is
    /// on the outer side: `(point on the surface, inward normal)`.
    pub cutaway: Option<(Vec3, Vec3)>,
    /// Reflectance (0–1) overriding the material's brightness (its colour
    /// is kept); 0 = the material as it is. Planned rooms use the
    /// designer's ceiling / wall / floor values.
    pub reflectance: f32,
}

impl ScenePrim {
    /// The point lies inside the primitive (exact, for the measurement
    /// point layout: an oblique wall's bounds would drop points far from it).
    pub fn contains(&self, p: Vec3) -> bool {
        match &self.shape {
            Shape::Box { min, max } => p.cmpge(*min).all() && p.cmple(*max).all(),
            Shape::Obb { center, half, rot } => {
                let l = rot.transpose() * (p - *center);
                l.abs().cmple(*half).all()
            }
            Shape::Sphere { center, radius } => (p - *center).length_squared() <= radius * radius,
            Shape::Capsule { a, b, radius } | Shape::Cylinder { a, b, radius } => {
                let ab = *b - *a;
                let t = ((p - *a).dot(ab) / ab.length_squared().max(1e-12)).clamp(0.0, 1.0);
                (p - (*a + ab * t)).length_squared() <= radius * radius
            }
        }
    }

    /// Axis-aligned bounds, for the point layout and the CPU occlusion tests.
    pub fn aabb(&self) -> (Vec3, Vec3) {
        match &self.shape {
            Shape::Box { min, max } => (*min, *max),
            Shape::Obb { center, half, rot } => {
                let e = Vec3::new(
                    rot.x_axis.x.abs() * half.x
                        + rot.y_axis.x.abs() * half.y
                        + rot.z_axis.x.abs() * half.z,
                    rot.x_axis.y.abs() * half.x
                        + rot.y_axis.y.abs() * half.y
                        + rot.z_axis.y.abs() * half.z,
                    rot.x_axis.z.abs() * half.x
                        + rot.y_axis.z.abs() * half.y
                        + rot.z_axis.z.abs() * half.z,
                );
                (*center - e, *center + e)
            }
            Shape::Sphere { center, radius } => (*center - *radius, *center + *radius),
            Shape::Capsule { a, b, radius } | Shape::Cylinder { a, b, radius } => {
                (a.min(*b) - *radius, a.max(*b) + *radius)
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SceneLamp {
    pub position: Vec3,
    /// Luminaire local → world.
    pub rotation: Mat3,
    /// Photometry: 0 = the scene's luminaire, n = the plan's extra type n.
    pub table: u8,
    /// What it does in normal and in emergency mode.
    pub role: LampRole,
}

/// What a luminaire does in normal operation and in emergency mode (EN 1838).
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum LampRole {
    /// Normal lighting only (off in emergency mode).
    #[default]
    Normal,
    /// Emergency lighting only (off in normal operation).
    Emergency,
    /// Both; in emergency mode at `emergency` × its normal flux.
    Combined { emergency: f32 },
}

impl LampRole {
    /// Output (0–1) in normal operation or in emergency mode.
    pub fn level(self, emergency_mode: bool) -> f32 {
        match (self, emergency_mode) {
            (LampRole::Normal, false) | (LampRole::Emergency, true) => 1.0,
            (LampRole::Normal, true) | (LampRole::Emergency, false) => 0.0,
            (LampRole::Combined { .. }, false) => 1.0,
            (LampRole::Combined { emergency }, true) => emergency.clamp(0.0, 1.0),
        }
    }
}

/// Something in the scene that can be moved on the ground by hand.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum ObjectId {
    /// Index in `BuiltScene::lamps` (with its pole, arm or wire).
    Luminaire(u8),
    Person,
    Car(u8),
    /// Index in `SceneParams::sensors`.
    Sensor(u8),
}

/// A movable object: where it stands, where it is grabbed, and what is
/// drawn for it.
#[derive(Clone, Debug)]
pub struct Movable {
    pub id: ObjectId,
    /// Ground point it stands on (pole foot, feet, car centre, sensor foot).
    pub anchor: Vec3,
    /// Point to grab it by (luminaire head, chest, roof, sensor head).
    pub handle: Vec3,
    /// Its primitives in `BuiltScene::prims`.
    pub prims: std::ops::Range<usize>,
    /// Its lamp in `BuiltScene::lamps`.
    pub lamp: Option<usize>,
    /// Further lamps on the same pole (they move with it).
    pub extra_lamps: Vec<usize>,
    /// Half length and half width on the ground (along `yaw_deg`), for the
    /// detection test.
    pub half: Vec2,
    /// Turn around the vertical (degrees, local +X towards +Z).
    pub yaw_deg: f32,
}

/// A presence / motion sensor (its ground position).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SceneSensor {
    pub x: f32,
    pub z: f32,
}

/// Most sensors in a scene.
pub const MAX_SENSORS: usize = 16;
/// Radius of the drawn sensor head (m).
pub const SENSOR_HEAD_RADIUS: f32 = 0.08;

/// A calculation surface: `origin + s·u + t·v`, `normal` facing the light.
#[derive(Clone, Debug)]
pub struct Receiver {
    pub origin: Vec3,
    pub u: Vec3,
    pub v: Vec3,
    pub normal: Vec3,
}

/// Grid names of a planned scene in emergency mode (grids of one name are
/// evaluated together).
pub const GRID_ROUTE: &str = "escape route";
/// The central band's edges (a quarter of the width off the centre line).
pub const GRID_ROUTE_BAND: &str = "escape route band";
/// Stairs / refuge routes (China), centre line and band edges.
pub const GRID_STAIRS: &str = "stairs / refuge";
pub const GRID_STAIRS_BAND: &str = "stairs / refuge band";
pub const GRID_OPEN_AREA: &str = "open area (floor)";
pub const GRID_EMPHASIS: &str = "point of emphasis (vertical)";

/// A calculation grid with `nu × nv` measurement points at the cell centres.
#[derive(Clone, Debug)]
pub struct MeasureGrid {
    pub name: &'static str,
    pub receiver: Receiver,
    pub nu: usize,
    pub nv: usize,
    /// Only points inside this outline (world x, z) are measured.
    pub outline: Option<Vec<Vec2>>,
    /// … and at least this far from its edges (m; EN 1838 leaves a 0.5 m
    /// border of an open area out).
    pub border: f32,
}

/// Home camera: orbit around `target`.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CameraView {
    pub target: Vec3,
    /// Radians, 0 puts the eye on +Z looking towards −Z.
    pub yaw: f32,
    /// Radians above the horizon.
    pub pitch: f32,
    pub distance: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraLimits {
    pub min_pitch: f32,
    pub max_pitch: f32,
    pub min_distance: f32,
    pub max_distance: f32,
}

impl Default for CameraLimits {
    fn default() -> Self {
        Self {
            min_pitch: 2.0_f32.to_radians(),
            max_pitch: 89.5_f32.to_radians(),
            min_distance: 0.5,
            max_distance: 100.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BuiltScene {
    pub kind: SceneKind,
    pub prims: Vec<ScenePrim>,
    pub lamps: Vec<SceneLamp>,
    /// 0 = no ground plane, 1 = paving, 2 = street asphalt, 3 = parking asphalt.
    pub ground: u32,
    pub outdoor: bool,
    /// Exposure key in lux. `None` uses the outdoor / indoor default.
    pub key_lux: Option<f32>,
    pub view: CameraView,
    pub limits: CameraLimits,
    pub receivers: Vec<Receiver>,
    pub grids: Vec<MeasureGrid>,
    /// Luminaires, the person, the cars and the sensors that can be moved.
    pub movables: Vec<Movable>,
}

pub const MAX_PRIMS: usize = 256;
pub const MAX_LAMPS: usize = 128;

/// Lane width per EN 13201, two lanes.
pub const LANE_WIDTH: f32 = 3.5;
pub const ROAD_HALF_WIDTH: f32 = LANE_WIDTH;
const PARKING_STRIP: f32 = 2.0;
const CYCLE_LANE: f32 = 1.5;
const SIDEWALK: f32 = 2.0;
const KERB_H: f32 = 0.12;
const ARM_OVERHANG: f32 = 1.5;
const OFFICE_W: f32 = 6.0;
const OFFICE_D: f32 = 4.8;
/// EN 12464-1 workplane height and marginal zone.
pub const WORKPLANE: f32 = 0.75;
const MARGIN: f32 = 0.5;
/// Gap left between the highest point of the luminaire body and the ceiling.
pub const CEILING_CLEARANCE: f32 = 0.05;
/// Glasshouse: 10 × 6 m bay, eave height and ridge rise above the eave.
const GREENHOUSE_L: f32 = 10.0;
const GREENHOUSE_W: f32 = 6.0;
const GREENHOUSE_EAVE: f32 = 2.6;
const GREENHOUSE_RISE: f32 = 1.4;
const BENCH_TOP: f32 = 0.8;
/// Bench centre lines (z) and width.
const BENCH_Z: [f32; 2] = [-1.5, 1.5];
const BENCH_W: f32 = 1.8;

/// Luminaire local → world rotation: first the yaw around the vertical
/// (which aims the C0 axis), then the tilt about the world X axis, the one
/// that runs along the road or the facade. Positive tilt swings the beam
/// towards −Z, where the road, the bays and the facade are; `flip` is for
/// lamps on the far side of a street, which get 180° of yaw and tilt
/// towards +Z instead.
pub fn luminaire_rotation(rotation_deg: f32, tilt_deg: f32, flip: bool) -> Mat3 {
    let yaw = rotation_deg.to_radians() + if flip { std::f32::consts::PI } else { 0.0 };
    let tilt = tilt_deg.to_radians() * if flip { -1.0 } else { 1.0 };
    let q = Quat::from_rotation_x(tilt) * Quat::from_rotation_y(yaw);
    Mat3::from_quat(q)
}

/// Highest mounting height (luminous opening) at which the whole tilted
/// body, from the luminous face up to the top of the housing, stays below
/// the ceiling.
pub fn room_mount_limit(p: &SceneParams) -> f32 {
    let h = p.housing_half;
    let rot = luminaire_rotation(p.rotation, p.tilt, false);
    let mut rise: f32 = 0.0;
    for x in [-h.x, h.x] {
        for z in [-h.z, h.z] {
            for y in [0.0, 2.0 * h.y] {
                rise = rise.max((rot * Vec3::new(x, y, z)).y);
            }
        }
    }
    p.ceiling_height - rise - CEILING_CLEARANCE
}

fn prim(shape: Shape, material: u32) -> ScenePrim {
    ScenePrim {
        shape,
        material,
        casts_shadow: true,
        cutaway: None,
        reflectance: 0.0,
    }
}

fn bx(min: Vec3, max: Vec3, material: u32) -> ScenePrim {
    prim(
        Shape::Box {
            min: min.min(max),
            max: min.max(max),
        },
        material,
    )
}

fn bx_soft(min: Vec3, max: Vec3, material: u32) -> ScenePrim {
    ScenePrim {
        casts_shadow: false,
        ..bx(min, max, material)
    }
}

fn obb(center: Vec3, half: Vec3, rot: Mat3, material: u32) -> ScenePrim {
    prim(Shape::Obb { center, half, rot }, material)
}

fn sphere(center: Vec3, radius: f32, material: u32) -> ScenePrim {
    prim(Shape::Sphere { center, radius }, material)
}

fn capsule(a: Vec3, b: Vec3, radius: f32, material: u32) -> ScenePrim {
    prim(Shape::Capsule { a, b, radius }, material)
}

fn cylinder(a: Vec3, b: Vec3, radius: f32, material: u32) -> ScenePrim {
    prim(Shape::Cylinder { a, b, radius }, material)
}

fn v3(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

/// A scene from a planner: the Interior designer's room or the Outdoor
/// designer's area, with the planned luminaires and sensors.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ScenePlan {
    /// Shown in the panel ("Classroom 10.0 × 8.0 m").
    pub name: String,
    /// A room (walls, ceiling, workplane grid) or an open area (ground grid).
    pub indoor: bool,
    /// Outline on the ground (x, z) in m, either winding.
    pub outline: Vec<Vec2>,
    /// Room height (m).
    pub height: f32,
    /// Reflectances of ceiling, walls and floor (0–1).
    pub reflectances: [f32; 3],
    /// Height of the measurement plane (workplane indoors, 0 outdoors).
    pub plane: f32,
    pub luminaires: Vec<PlanLuminaire>,
    /// Sensor ground positions (x, z) and their height.
    pub sensors: Vec<Vec2>,
    pub sensor_height: f32,
    /// Shown under the name (e.g. a limitation of the conversion).
    pub note: Option<String>,
    /// Further luminaire types (`PlanLuminaire::photometry` 1, 2, …), e.g.
    /// the emergency luminaire; the scene's own luminaire is type 0.
    #[serde(default)]
    pub photometries: Vec<PlanPhotometry>,
    /// Emergency lighting of the plan (routes, open area, equipment).
    #[serde(default)]
    pub emergency: Option<PlanEmergency>,
    /// Show it in emergency mode (the designer showed emergency conditions).
    #[serde(default)]
    pub emergency_mode: bool,
}

/// A further luminaire type of a plan (self-contained: the file's text).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PlanPhotometry {
    pub name: String,
    /// EULUMDAT text (scaled to the flux it is used at).
    pub ldt: String,
}

/// Emergency lighting drawn on a plan, for showing and checking it in the
/// scene (plan coordinates (x, z) in m).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PlanEmergency {
    /// Escape routes: centre line and width.
    pub routes: Vec<(Vec<Vec2>, f32)>,
    /// Per route: stairs / front room / refuge (China: its own, higher
    /// minimum; measured as its own grids).
    #[serde(default)]
    pub route_stairs: Vec<bool>,
    /// The standard and what it requires, for the check in the scene.
    #[serde(default)]
    pub standard: EmStandard,
    /// The open (anti-panic) area, if the room is one.
    pub open_area: Option<Vec<Vec2>>,
    /// Points of emphasis: position, height, facing (deg, from +x towards
    /// +z), symbol.
    pub points: Vec<PlanPoint>,
}

/// What the plan's standard requires (lx; a minimum, average or ratio of 0
/// = no such rule). Default = EN 1838.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct EmStandard {
    pub name: String,
    /// Routes: centre line, band (`route_band_share` of the width), Emax /
    /// Emin on the centre line (or over the band when `ratio_over_band`),
    /// average over the band.
    pub route_centre_lx: f32,
    pub route_band_lx: f32,
    pub route_ratio: f32,
    pub route_avg_lx: f32,
    pub route_band_share: f32,
    /// The strip left out along each side is at most this wide (EN
    /// 1838:2024: 0.5 m; 0 = no cap).
    pub route_edge_max_m: f32,
    pub ratio_over_band: bool,
    /// The open area / room: average.
    pub area_avg_lx: f32,
    /// Stairs / refuge routes (China): over the central band.
    pub stairs_lx: f32,
    /// The open area / room: floor minimum, Emax / Emin, border left out.
    pub area_lx: f32,
    pub area_ratio: f32,
    pub area_border_m: f32,
    /// Vertical illuminance at equipment.
    pub vertical_lx: f32,
    /// Top of the false colours in emergency mode.
    pub lux_max: f32,
}

impl Default for EmStandard {
    fn default() -> Self {
        Self {
            name: "EN 1838:2013".into(),
            route_centre_lx: 1.0,
            route_band_lx: 0.5,
            route_ratio: 40.0,
            route_avg_lx: 0.0,
            route_band_share: 0.5,
            route_edge_max_m: 0.0,
            ratio_over_band: false,
            area_avg_lx: 0.0,
            stairs_lx: 0.0,
            area_lx: 0.5,
            area_ratio: 40.0,
            area_border_m: 0.5,
            vertical_lx: 5.0,
            lux_max: 5.0,
        }
    }
}

/// A point of emphasis (fire equipment, first aid, exit …) on a plan.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PlanPoint {
    pub position: Vec2,
    pub height: f32,
    pub facing_deg: f32,
    /// Short label (an icon, or "F", "+", "X" …).
    pub symbol: String,
    /// Measured vertically, facing `facing_deg` (EN 1838: fire equipment,
    /// first aid, call points; exits are not).
    #[serde(default)]
    pub vertical: bool,
}

/// A planned luminaire.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PlanLuminaire {
    /// Light centre (x, mounting height, z).
    pub position: Vec3,
    /// C0 direction in the plan: degrees from +x towards +z (the planners'
    /// convention, C = atan2(dz, dx)).
    pub rotation_deg: f32,
    /// Tilt about the C90 axis (degrees): the beam swings towards C0.
    pub tilt_deg: f32,
    /// Pole foot (x, z): a pole, and an arm from its top when the
    /// luminaire sits away from it. Luminaires on one foot share the pole.
    pub pole: Option<Vec2>,
    /// Hangs from the ceiling on a rod.
    pub pendant: bool,
    /// Luminaire type: 0 = the scene's luminaire, n = `ScenePlan::photometries[n - 1]`.
    #[serde(default)]
    pub photometry: u8,
    /// Normal, emergency or combined.
    #[serde(default)]
    pub role: LampRole,
}

impl ScenePlan {
    /// A 10 × 8 m room with one luminaire (when a planned scene has no plan).
    pub fn example() -> Self {
        Self {
            name: "Room 10 × 8 m".into(),
            indoor: true,
            outline: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(10.0, 0.0),
                Vec2::new(10.0, 8.0),
                Vec2::new(0.0, 8.0),
            ],
            height: 3.0,
            reflectances: [0.7, 0.5, 0.2],
            plane: 0.8,
            luminaires: vec![PlanLuminaire {
                position: Vec3::new(5.0, 2.95, 4.0),
                rotation_deg: 0.0,
                tilt_deg: 0.0,
                pole: None,
                pendant: false,
                photometry: 0,
                role: LampRole::Normal,
            }],
            sensors: Vec::new(),
            sensor_height: 3.0,
            note: None,
            photometries: Vec::new(),
            emergency: None,
            emergency_mode: false,
        }
    }
}

/// Luminaire local → world for a planned luminaire: C0 to (cos r, 0, sin r)
/// in world (x, z), then the tilt about the luminaire's C90 axis.
pub fn plan_rotation(rotation_deg: f32, tilt_deg: f32) -> Mat3 {
    Mat3::from_rotation_y(-rotation_deg.to_radians()) * Mat3::from_rotation_z(tilt_deg.to_radians())
}

/// Bounding box of an outline (min, max).
pub fn outline_bounds(outline: &[Vec2]) -> (Vec2, Vec2) {
    let lo = outline.iter().fold(Vec2::splat(f32::MAX), |a, p| a.min(*p));
    let hi = outline.iter().fold(Vec2::splat(f32::MIN), |a, p| a.max(*p));
    if lo.x > hi.x {
        (Vec2::ZERO, Vec2::ONE)
    } else {
        (lo, hi)
    }
}

/// Point-in-polygon (even–odd).
/// Distance (m) from `p` to the nearest edge of a closed outline.
pub fn distance_to_outline(outline: &[Vec2], p: Vec2) -> f32 {
    let n = outline.len();
    (0..n)
        .map(|i| {
            let (a, b) = (outline[i], outline[(i + 1) % n]);
            let ab = b - a;
            let t = ((p - a).dot(ab) / ab.length_squared().max(1e-12)).clamp(0.0, 1.0);
            (a + ab * t - p).length()
        })
        .fold(f32::INFINITY, f32::min)
}

pub fn outline_contains(outline: &[Vec2], q: Vec2) -> bool {
    let n = outline.len();
    let mut inside = false;
    let mut j = n.wrapping_sub(1);
    for i in 0..n {
        let (a, b) = (outline[i], outline[j]);
        if (a.y > q.y) != (b.y > q.y) && q.x < (b.x - a.x) * (q.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Area and perimeter of an outline.
pub fn outline_area_perimeter(outline: &[Vec2]) -> (f32, f32) {
    let n = outline.len();
    let (mut a2, mut per) = (0.0, 0.0);
    for i in 0..n {
        let (p, q) = (outline[i], outline[(i + 1) % n]);
        a2 += p.x * q.y - q.x * p.y;
        per += (q - p).length();
    }
    (0.5 * a2.abs(), per)
}

/// A point well inside the outline (the centre of the bounds if it is
/// inside, else the inside point of a 12 × 12 scan farthest from the edges).
fn inside_point(outline: &[Vec2]) -> Vec2 {
    let (lo, hi) = outline_bounds(outline);
    let c = (lo + hi) * 0.5;
    if outline_contains(outline, c) {
        return c;
    }
    let edge_dist = |q: Vec2| {
        let n = outline.len();
        (0..n)
            .map(|i| {
                let (a, b) = (outline[i], outline[(i + 1) % n]);
                let ab = b - a;
                let t = ((q - a).dot(ab) / ab.length_squared().max(1e-9)).clamp(0.0, 1.0);
                (q - (a + ab * t)).length()
            })
            .fold(f32::MAX, f32::min)
    };
    let mut best = (outline[0], -1.0);
    for i in 0..12 {
        for j in 0..12 {
            let q = lo + (hi - lo) * Vec2::new((i as f32 + 0.5) / 12.0, (j as f32 + 0.5) / 12.0);
            if outline_contains(outline, q) {
                let d = edge_dist(q);
                if d > best.1 {
                    best = (q, d);
                }
            }
        }
    }
    best.0
}

/// Scene from a plan: a room (floor, ceiling, one wall per outline edge,
/// all cut away on the camera side, with the planned reflectances) or an
/// open area (paving with a kerb along the outline); the luminaires with
/// their poles, arms or rods; the person; a grid inside the outline.
fn build_planned(p: &SceneParams, plan: &ScenePlan) -> BuiltScene {
    let mut boxes: Vec<ScenePrim> = Vec::new();
    let mut lamps: Vec<SceneLamp> = Vec::new();
    let mut movables: Vec<Movable> = Vec::new();
    let outline: Vec<Vec2> = if plan.outline.len() >= 3 {
        plan.outline.clone()
    } else {
        ScenePlan::example().outline
    };
    let (lo, hi) = outline_bounds(&outline);
    let size = hi - lo;
    let centre = (lo + hi) * 0.5;
    let h = plan.height.max(2.0);
    let [rho_c, rho_w, rho_f] = plan.reflectances;
    let n = outline.len();
    let (signed, _) = {
        let mut a2 = 0.0;
        for i in 0..n {
            let (a, b) = (outline[i], outline[(i + 1) % n]);
            a2 += a.x * b.y - b.x * a.y;
        }
        (a2, 0.0)
    };
    // Inward normal of an edge: left of its direction when the outline
    // runs counter-clockwise in (x, z).
    let inward = |d: Vec2| {
        let left = Vec2::new(-d.y, d.x);
        if signed >= 0.0 {
            left
        } else {
            -left
        }
    };
    let edge_box = |a: Vec2, b: Vec2, thick: f32, y0: f32, y1: f32, outside: bool| {
        let d = b - a;
        let len = d.length().max(1e-3);
        let dir = d / len;
        let nin = inward(dir);
        let off = if outside {
            -nin * (thick * 0.5)
        } else {
            Vec2::ZERO
        };
        let mid = (a + b) * 0.5 + off;
        let x = Vec3::new(dir.x, 0.0, dir.y);
        let z = x.cross(Vec3::Y);
        let rot = Mat3::from_cols(x, Vec3::Y, z);
        (
            obb(
                Vec3::new(mid.x, 0.5 * (y0 + y1), mid.y),
                Vec3::new(0.5 * len + thick, 0.5 * (y1 - y0), 0.5 * thick),
                rot,
                mat::WALL,
            ),
            nin,
            (a + b) * 0.5,
        )
    };
    if plan.indoor {
        let t = 0.25;
        let mut floor = bx(
            v3(lo.x - t, -t, lo.y - t),
            v3(hi.x + t, 0.0, hi.y + t),
            mat::CARPET,
        );
        floor.reflectance = rho_f;
        boxes.push(floor);
        let mut ceiling = bx(
            v3(lo.x - t, h, lo.y - t),
            v3(hi.x + t, h + t, hi.y + t),
            mat::CEILING,
        );
        ceiling.reflectance = rho_c;
        ceiling.cutaway = Some((v3(centre.x, h, centre.y), Vec3::NEG_Y));
        boxes.push(ceiling);
        for i in 0..n {
            let (a, b) = (outline[i], outline[(i + 1) % n]);
            if (b - a).length() < 0.05 {
                continue;
            }
            let (mut wall, nin, mid) = edge_box(a, b, t, 0.0, h, true);
            wall.reflectance = rho_w;
            wall.cutaway = Some((v3(mid.x, 0.0, mid.y), Vec3::new(nin.x, 0.0, nin.y)));
            boxes.push(wall);
        }
    } else {
        for i in 0..n {
            let (a, b) = (outline[i], outline[(i + 1) % n]);
            if (b - a).length() < 0.05 {
                continue;
            }
            let (mut kerb, _, _) = edge_box(a, b, 0.15, 0.0, KERB_H, false);
            kerb.material = mat::KERB;
            boxes.push(kerb);
        }
    }

    // Luminaires; luminaires on one pole foot share that pole and move with it.
    let mut pole_owner: Vec<(Vec2, usize)> = Vec::new();
    // over the lamp budget the emergency lamps stay (emergency mode needs
    // every one of them), normal ones fill the rest in plan order
    let special = plan
        .luminaires
        .iter()
        .filter(|l| l.role != LampRole::Normal)
        .count();
    let mut normal_left = MAX_LAMPS.saturating_sub(special);
    let kept = plan.luminaires.iter().filter(|l| {
        if l.role != LampRole::Normal {
            true
        } else if normal_left > 0 {
            normal_left -= 1;
            true
        } else {
            false
        }
    });
    for l in kept.take(MAX_LAMPS) {
        let start = boxes.len();
        let rot = plan_rotation(l.rotation_deg, l.tilt_deg);
        let y = l.position.y;
        let lamp_xz = Vec2::new(l.position.x, l.position.z);
        lamps.push(SceneLamp {
            position: l.position,
            rotation: rot,
            table: l.photometry.min(plan.photometries.len() as u8),
            role: l.role,
        });
        let li = lamps.len() - 1;
        if let Some(foot) = l.pole {
            if let Some(&(_, owner)) = pole_owner.iter().find(|(q, _)| (*q - foot).length() < 0.05)
            {
                // the arm of a further luminaire on an existing pole
                if (lamp_xz - foot).length() > 0.05 {
                    let mut arm = cylinder(
                        v3(foot.x, y + 0.16, foot.y),
                        v3(lamp_xz.x, y + 0.16, lamp_xz.y),
                        0.04,
                        mat::METAL,
                    );
                    arm.casts_shadow = false;
                    boxes.push(arm);
                }
                let end = boxes.len();
                if let Some(m) = movables.get_mut(owner) {
                    m.extra_lamps.push(li);
                    // its arm moves with the pole: widen the owner's range
                    // when it is contiguous, else it stays (rare)
                    if m.prims.end == start {
                        m.prims.end = end;
                    }
                }
                continue;
            }
            boxes.push(cylinder(
                v3(foot.x, 0.0, foot.y),
                v3(foot.x, y * 0.45, foot.y),
                0.09,
                mat::METAL,
            ));
            // a luminaire on its own pole (no arm) sits on the shaft: the
            // shaft ends under it and does not shade its light (it used to
            // run through the luminaire and took ~20 % of the light off the
            // ground; the planners ignore poles, a real spigot enters from
            // the side)
            let post_top = (lamp_xz - foot).length() <= 0.05;
            let mut shaft = cylinder(
                v3(foot.x, y * 0.45, foot.y),
                v3(foot.x, if post_top { y - 0.03 } else { y + 0.25 }, foot.y),
                0.065,
                mat::METAL,
            );
            shaft.casts_shadow = !post_top;
            boxes.push(shaft);
            if !post_top {
                let mut arm = cylinder(
                    v3(foot.x, y + 0.16, foot.y),
                    v3(lamp_xz.x, y + 0.16, lamp_xz.y),
                    0.04,
                    mat::METAL,
                );
                arm.casts_shadow = false;
                boxes.push(arm);
            }
            pole_owner.push((foot, movables.len()));
            track_lamp(
                &mut movables,
                &lamps,
                start..boxes.len(),
                v3(foot.x, 0.0, foot.y),
            );
        } else {
            if plan.indoor && l.pendant && y < h - 0.05 {
                boxes.push(bx_soft(
                    v3(lamp_xz.x - 0.005, y + 0.08, lamp_xz.y - 0.005),
                    v3(lamp_xz.x + 0.005, h, lamp_xz.y + 0.005),
                    mat::METAL,
                ));
            }
            track_lamp(
                &mut movables,
                &lamps,
                start..boxes.len(),
                v3(lamp_xz.x, 0.0, lamp_xz.y),
            );
        }
    }
    // no person in emergency mode: EN 1838 measures the empty room (a
    // figure would shade the route)
    if p.show_figure && !(p.emergency && plan.emergency.is_some()) {
        let at = inside_point(&outline);
        track_figure(v3(at.x, 0.0, at.y), &mut boxes, &mut movables);
    }

    let plane = if plan.indoor {
        plan.plane.clamp(0.0, h - 0.1)
    } else {
        0.0
    };
    let r = receiver(
        v3(lo.x, plane, lo.y),
        v3(size.x.max(0.1), 0.0, 0.0),
        v3(0.0, 0.0, size.y.max(0.1)),
        Vec3::Y,
    );
    let mut grid = norm_grid(if plan.indoor { "workplane" } else { "ground" }, r.clone());
    grid.outline = Some(outline.clone());
    let mut grids = vec![grid];
    if let Some(em) = plan.emergency.as_ref().filter(|_| p.emergency) {
        // EN 1838 measures on the floor: points every 0.5 m along each
        // route's centre line, and the open area
        grids.clear();
        for (ri, (path, width)) in em.routes.iter().enumerate() {
            let stairs = em.route_stairs.get(ri).copied().unwrap_or(false);
            let (centre_name, band_name) = if stairs {
                (GRID_STAIRS, GRID_STAIRS_BAND)
            } else {
                (GRID_ROUTE, GRID_ROUTE_BAND)
            };
            for w in path.windows(2) {
                let a = v3(w[0].x, 0.02, w[0].y);
                let u = v3(w[1].x, 0.02, w[1].y) - a;
                let len = u.length();
                if len < 0.1 {
                    continue;
                }
                let n = Vec3::new(-u.z, 0.0, u.x) / len;
                let side = n * 0.02;
                // the centre line, and the band's edges (EN 1838: the
                // central half, a quarter of the width to each side)
                let st = &em.standard;
                let mut edge = width * (1.0 - st.route_band_share.clamp(0.0, 1.0)) / 2.0;
                if st.route_edge_max_m > 0.0 {
                    edge = edge.min(st.route_edge_max_m);
                }
                let q = ((width / 2.0 - edge) / width.max(1e-3)).max(0.0);
                for (off, name) in [(0.0, centre_name), (q, band_name), (-q, band_name)] {
                    let o = a + n * (off * width);
                    grids.push(MeasureGrid {
                        name,
                        receiver: receiver(o - side * 0.5, u, side, Vec3::Y),
                        nu: ((len / 0.5).ceil() as usize).clamp(1, MAX_GRID_AXIS),
                        nv: 1,
                        outline: None,
                        border: 0.0,
                    });
                }
            }
        }
        if let Some(area) = em.open_area.as_ref() {
            let floor = receiver(
                v3(lo.x, 0.02, lo.y),
                v3(size.x.max(0.1), 0.0, 0.0),
                v3(0.0, 0.0, size.y.max(0.1)),
                Vec3::Y,
            );
            // 0.5 m between points (the grid's cap allowing), the border left out
            let n = |l: f32| ((l / 0.5).ceil() as usize).clamp(1, MAX_GRID_AXIS);
            grids.push(MeasureGrid {
                name: GRID_OPEN_AREA,
                nu: n(size.x),
                nv: n(size.y),
                receiver: floor,
                outline: Some(area.clone()),
                border: em.standard.area_border_m.max(0.0),
            });
        }
        // vertical illuminance at the equipment, 5 cm in front of it
        for pt in em.points.iter().filter(|q| q.vertical) {
            let f = pt.facing_deg.to_radians();
            let n = Vec3::new(f.cos(), 0.0, f.sin());
            let side = Vec3::new(-n.z, 0.0, n.x) * 0.02;
            let up = Vec3::Y * 0.02;
            let c = v3(pt.position.x, pt.height, pt.position.y) + n * 0.05;
            grids.push(MeasureGrid {
                name: GRID_EMPHASIS,
                receiver: receiver(c - (side + up) * 0.5, side, up, n),
                nu: 1,
                nv: 1,
                outline: None,
                border: 0.0,
            });
        }
    }
    let diag = size.length().max(4.0);
    BuiltScene {
        kind: SceneKind::Planned,
        prims: boxes,
        lamps,
        ground: if plan.indoor { 0 } else { 1 },
        outdoor: !plan.indoor,
        key_lux: None,
        view: CameraView {
            target: v3(centre.x, if plan.indoor { 0.6 } else { 0.0 }, centre.y),
            yaw: 35.0_f32.to_radians(),
            pitch: if plan.indoor { 50.0_f32 } else { 35.0 }.to_radians(),
            distance: diag * if plan.indoor { 1.15 } else { 1.0 } + 3.0,
        },
        limits: CameraLimits {
            max_distance: diag * 4.0 + 50.0,
            ..Default::default()
        },
        receivers: vec![r],
        grids,
        movables,
    }
}

/// The lamp just pushed, with its primitives `prims`, standing at `anchor`.
fn track_lamp(
    movables: &mut Vec<Movable>,
    lamps: &[SceneLamp],
    prims: std::ops::Range<usize>,
    anchor: Vec3,
) {
    let i = lamps.len() - 1;
    movables.push(Movable {
        id: ObjectId::Luminaire(i as u8),
        anchor,
        handle: lamps[i].position,
        prims,
        lamp: Some(i),
        extra_lamps: Vec::new(),
        half: Vec2::splat(0.3),
        yaw_deg: 0.0,
    });
}

fn track_figure(base: Vec3, out: &mut Vec<ScenePrim>, movables: &mut Vec<Movable>) {
    let start = out.len();
    figure(base, out);
    movables.push(Movable {
        id: ObjectId::Person,
        anchor: base,
        handle: base + Vec3::new(0.0, 1.1, 0.0),
        prims: start..out.len(),
        lamp: None,
        extra_lamps: Vec::new(),
        half: Vec2::new(0.25, 0.25),
        yaw_deg: 0.0,
    });
}

fn track_car(
    base: Vec3,
    yaw_deg: f32,
    paint: u32,
    out: &mut Vec<ScenePrim>,
    movables: &mut Vec<Movable>,
) {
    let start = out.len();
    car(base, yaw_deg, paint, out);
    let n = movables
        .iter()
        .filter(|m| matches!(m.id, ObjectId::Car(_)))
        .count();
    movables.push(Movable {
        id: ObjectId::Car(n as u8),
        anchor: base,
        handle: base + Vec3::new(0.0, 1.5, 0.0),
        prims: start..out.len(),
        lamp: None,
        extra_lamps: Vec::new(),
        half: Vec2::new(2.2, 0.9),
        yaw_deg,
    });
}

impl Shape {
    /// The same shape shifted by `d`.
    pub fn translate(&mut self, d: Vec3) {
        match self {
            Shape::Box { min, max } => {
                *min += d;
                *max += d;
            }
            Shape::Obb { center, .. } | Shape::Sphere { center, .. } => *center += d,
            Shape::Capsule { a, b, .. } | Shape::Cylinder { a, b, .. } => {
                *a += d;
                *b += d;
            }
        }
    }
}

/// Put moved objects at their new ground positions (the anchor goes to
/// (x, z), everything that belongs to the object follows).
pub fn apply_moves(scene: &mut BuiltScene, moves: &[(ObjectId, Vec2)]) {
    for &(id, xz) in moves {
        let Some(m) = scene.movables.iter_mut().find(|m| m.id == id) else {
            continue;
        };
        let d = Vec3::new(xz.x - m.anchor.x, 0.0, xz.y - m.anchor.z);
        if d.length_squared() < 1e-12 {
            continue;
        }
        for prim in scene.prims.get_mut(m.prims.clone()).into_iter().flatten() {
            prim.shape.translate(d);
            if let Some((point, _)) = prim.cutaway.as_mut() {
                *point += d;
            }
        }
        for li in m.lamp.into_iter().chain(m.extra_lamps.iter().copied()) {
            if let Some(l) = scene.lamps.get_mut(li) {
                l.position += d;
            }
        }
        m.anchor += d;
        m.handle += d;
    }
}

/// Inside size (width × depth × mean height, m) of the closed scenes (office,
/// greenhouse), for the estimate of the reflected light; `None` outdoors.
pub fn interior_box(p: &SceneParams) -> Option<Vec3> {
    match p.kind {
        SceneKind::Office => Some(Vec3::new(OFFICE_W, p.ceiling_height, OFFICE_D)),
        SceneKind::Greenhouse => Some(Vec3::new(
            GREENHOUSE_L,
            GREENHOUSE_EAVE + 0.5 * GREENHOUSE_RISE,
            GREENHOUSE_W,
        )),
        SceneKind::Planned => p.plan.as_ref().filter(|pl| pl.indoor).map(|pl| {
            let (lo, hi) = outline_bounds(&pl.outline);
            Vec3::new(hi.x - lo.x, pl.height, hi.y - lo.y)
        }),
        _ => None,
    }
}

/// Floor outline (x, z) of a closed scene, where sensor areas end (walls
/// stop detection); `None` outdoors.
pub fn room_outline(p: &SceneParams) -> Option<Vec<Vec2>> {
    match p.kind {
        SceneKind::Office => {
            let (hw, hd) = (OFFICE_W / 2.0, OFFICE_D / 2.0);
            Some(vec![
                Vec2::new(-hw, -hd),
                Vec2::new(hw, -hd),
                Vec2::new(hw, hd),
                Vec2::new(-hw, hd),
            ])
        }
        SceneKind::Planned => p
            .plan
            .as_ref()
            .filter(|pl| pl.indoor)
            .map(|pl| pl.outline.clone()),
        _ => None,
    }
}

/// Height of the sensor heads: in the office the ceiling, in the
/// greenhouse under the ridge.
pub fn sensor_mount(p: &SceneParams) -> f32 {
    match p.kind {
        SceneKind::Office => p.ceiling_height - 0.02,
        SceneKind::Planned if p.plan.as_ref().is_some_and(|pl| pl.indoor) => {
            p.plan.as_ref().map_or(3.0, |pl| pl.height) - 0.02
        }
        SceneKind::Greenhouse => p
            .sensor_height
            .clamp(0.5, GREENHOUSE_EAVE + GREENHOUSE_RISE - 0.1),
        _ => p.sensor_height.clamp(0.5, 20.0),
    }
}

/// The sensors as small white heads; outdoors on a slim post unless they
/// stand at a pole (then they sit on it).
fn add_sensors(p: &SceneParams, scene: &mut BuiltScene) {
    let h = sensor_mount(p);
    let poles: Vec<Vec3> = scene
        .movables
        .iter()
        .filter(|m| matches!(m.id, ObjectId::Luminaire(_)) && !m.prims.is_empty())
        .map(|m| m.anchor)
        .collect();
    for (i, s) in p.sensors.iter().take(MAX_SENSORS).enumerate() {
        let start = scene.prims.len();
        let foot = v3(s.x, 0.0, s.z);
        let on_pole = poles
            .iter()
            .any(|q| Vec2::new(q.x - s.x, q.z - s.z).length() < 0.4);
        if scene.outdoor && !on_pole {
            let mut post = cylinder(
                foot,
                v3(s.x, h - SENSOR_HEAD_RADIUS, s.z),
                0.035,
                mat::METAL,
            );
            post.casts_shadow = false;
            scene.prims.push(post);
        }
        let mut head = sphere(v3(s.x, h, s.z), SENSOR_HEAD_RADIUS, mat::WHITE);
        head.casts_shadow = false;
        scene.prims.push(head);
        scene.movables.push(Movable {
            id: ObjectId::Sensor(i as u8),
            anchor: foot,
            handle: v3(s.x, h, s.z),
            prims: start..scene.prims.len(),
            lamp: None,
            extra_lamps: Vec::new(),
            half: Vec2::ZERO,
            yaw_deg: 0.0,
        });
    }
}

/// A person for scale: a 1.75 m mannequin standing at `base`, facing +X.
/// Sphere head, capsule neck, torso and limbs, box shoes.
fn figure(base: Vec3, out: &mut Vec<ScenePrim>) {
    let p = |x: f32, y: f32, z: f32| base + Vec3::new(x, y, z);
    out.push(sphere(p(0.0, 1.63, 0.0), 0.11, mat::SKIN));
    out.push(capsule(
        p(0.0, 1.46, 0.0),
        p(0.0, 1.55, 0.0),
        0.05,
        mat::SKIN,
    ));
    out.push(capsule(
        p(0.0, 1.05, 0.0),
        p(0.0, 1.40, 0.0),
        0.16,
        mat::JACKET,
    ));
    out.push(capsule(
        p(0.0, 1.43, -0.19),
        p(0.0, 1.43, 0.19),
        0.085,
        mat::JACKET,
    ));
    for side in [-1.0, 1.0] {
        let z = |v: f32| side * v;
        out.push(capsule(
            p(0.0, 1.41, z(0.24)),
            p(0.03, 1.12, z(0.27)),
            0.055,
            mat::JACKET,
        ));
        out.push(capsule(
            p(0.03, 1.12, z(0.27)),
            p(0.07, 0.86, z(0.25)),
            0.045,
            mat::JACKET,
        ));
        out.push(sphere(p(0.08, 0.80, z(0.25)), 0.047, mat::SKIN));
        out.push(capsule(
            p(0.0, 1.0, z(0.1)),
            p(0.0, 0.54, z(0.11)),
            0.08,
            mat::TROUSERS,
        ));
        out.push(capsule(
            p(0.0, 0.54, z(0.11)),
            p(0.0, 0.1, z(0.11)),
            0.06,
            mat::TROUSERS,
        ));
        out.push(obb(
            p(0.05, 0.04, z(0.11)),
            Vec3::new(0.14, 0.04, 0.055),
            Mat3::IDENTITY,
            mat::SHOE,
        ));
    }
}

/// A parked car, 4.4 m long, nose towards local +X turned by `yaw_deg`
/// around the vertical: body, sloped hood and trunk, glazed cabin with
/// slanted windscreen and rear window, bumpers, tyres with rims, lights.
fn car(base: Vec3, yaw_deg: f32, paint: u32, out: &mut Vec<ScenePrim>) {
    let rot = Mat3::from_rotation_y(yaw_deg.to_radians());
    let tf = |v: Vec3| base + rot * v;
    let part = |c: Vec3, half: Vec3, tilt_deg: f32, m: u32| {
        obb(
            tf(c),
            half,
            rot * Mat3::from_rotation_z(tilt_deg.to_radians()),
            m,
        )
    };
    out.push(part(v3(0.0, 0.52, 0.0), v3(2.15, 0.24, 0.86), 0.0, paint));
    out.push(part(v3(1.35, 0.84, 0.0), v3(0.82, 0.08, 0.78), -5.0, paint));
    out.push(part(v3(-1.6, 0.9, 0.0), v3(0.52, 0.12, 0.8), 3.0, paint));
    out.push(part(
        v3(-0.1, 1.05, 0.0),
        v3(0.95, 0.28, 0.72),
        0.0,
        mat::GLASS,
    ));
    out.push(part(v3(-0.1, 1.35, 0.0), v3(0.9, 0.04, 0.7), 0.0, paint));
    out.push(part(
        v3(0.92, 1.05, 0.0),
        v3(0.03, 0.31, 0.68),
        28.0,
        mat::GLASS,
    ));
    out.push(part(
        v3(-1.12, 1.05, 0.0),
        v3(0.03, 0.28, 0.68),
        -32.0,
        mat::GLASS,
    ));
    out.push(part(
        v3(2.2, 0.42, 0.0),
        v3(0.06, 0.14, 0.86),
        0.0,
        mat::PLASTIC,
    ));
    out.push(part(
        v3(-2.2, 0.42, 0.0),
        v3(0.06, 0.14, 0.86),
        0.0,
        mat::PLASTIC,
    ));
    for x in [-1.4, 1.4] {
        for side in [-1.0, 1.0] {
            let z = side * 0.8;
            out.push(cylinder(
                tf(v3(x, 0.33, z - 0.1)),
                tf(v3(x, 0.33, z + 0.1)),
                0.33,
                mat::RUBBER,
            ));
            out.push(cylinder(
                tf(v3(x, 0.33, z - 0.105)),
                tf(v3(x, 0.33, z + 0.105)),
                0.19,
                mat::RIM,
            ));
        }
    }
    for side in [-1.0, 1.0] {
        out.push(sphere(
            tf(v3(2.17, 0.62, side * 0.62)),
            0.07,
            mat::HEADLIGHT,
        ));
        out.push(sphere(
            tf(v3(-2.17, 0.66, side * 0.62)),
            0.06,
            mat::TAILLIGHT,
        ));
    }
}

/// Pole with a straight arm, lamp at the arm end. `side` +1 puts the pole
/// on +Z and the lamp towards −Z.
fn pole(x: f32, pole_z: f32, side: f32, height: f32, draw: bool, out: &mut Vec<ScenePrim>) -> Vec3 {
    let lamp_z = pole_z - side * ARM_OVERHANG;
    if !draw {
        // Free-standing luminaire: its own body reaches the ground.
        return v3(x, height, lamp_z);
    }
    out.push(cylinder(
        v3(x, 0.0, pole_z),
        v3(x, height * 0.45, pole_z),
        0.09,
        mat::METAL,
    ));
    out.push(cylinder(
        v3(x, height * 0.45, pole_z),
        v3(x, height + 0.25, pole_z),
        0.065,
        mat::METAL,
    ));
    let mut arm = cylinder(
        v3(x, height + 0.16, pole_z),
        v3(x, height + 0.16, lamp_z),
        0.04,
        mat::METAL,
    );
    arm.casts_shadow = false;
    out.push(arm);
    v3(x, height, lamp_z)
}

fn row_positions(count: u32, spacing: f32) -> Vec<f32> {
    (0..count)
        .map(|i| (i as f32 - (count as f32 - 1.0) / 2.0) * spacing)
        .collect()
}

const MAX_GRID_AXIS: usize = 40;
const ROAD_GRID_MAX_AXIS: usize = 120;

/// EN 12464-1 grid size: p = 0.2 · 5^log10(d), d being the longer side of the area.
fn norm_grid(name: &'static str, r: Receiver) -> MeasureGrid {
    let lu = r.u.length();
    let lv = r.v.length();
    let p = 0.2 * 5.0_f32.powf(lu.max(lv).max(1.0).log10());
    let n = |l: f32| ((l / p).ceil() as usize).clamp(1, MAX_GRID_AXIS);
    MeasureGrid {
        name,
        nu: n(lu),
        nv: n(lv),
        receiver: r,
        outline: None,
        border: 0.0,
    }
}

/// EN 13201-3 road field: three rows per lane, longitudinal step no more than 3 m.
fn road_grid(name: &'static str, r: Receiver, lanes: u32) -> MeasureGrid {
    MeasureGrid {
        name,
        nu: ((r.u.length() / 3.0).ceil() as usize).clamp(1, ROAD_GRID_MAX_AXIS),
        nv: 3 * lanes as usize,
        receiver: r,
        outline: None,
        border: 0.0,
    }
}

/// Fixed spacing, for strips next to the road (same columns as the road).
fn strip_grid(name: &'static str, r: Receiver, nu: usize, step: f32) -> MeasureGrid {
    MeasureGrid {
        name,
        nu,
        nv: ((r.v.length() / step).ceil() as usize).clamp(1, MAX_GRID_AXIS),
        receiver: r,
        outline: None,
        border: 0.0,
    }
}

fn receiver(origin: Vec3, u: Vec3, v: Vec3, normal: Vec3) -> Receiver {
    Receiver {
        origin,
        u,
        v,
        normal,
    }
}

/// Facade piers and spandrels in front of a back plane, so the windows read
/// as reveals. `z_face` is the outer face, the wall extends to `z_face - depth`.
#[allow(clippy::too_many_arguments)]
fn facade(
    x0: f32,
    x1: f32,
    z_face: f32,
    height: f32,
    finish: u32,
    bay: f32,
    out: &mut Vec<ScenePrim>,
    budget: usize,
) {
    let depth = 0.4;
    let reveal = 0.18;
    // Back plane with the glass.
    out.push(bx(
        v3(x0, 0.0, z_face - depth),
        v3(x1, height, z_face - reveal),
        mat::GLASS,
    ));
    // Spandrels: ground floor plinth, bands between storeys, parapet.
    let storey = 3.0;
    let mut y = 0.0;
    while y < height && out.len() < budget {
        let top = (y + 1.0).min(height);
        out.push(bx(v3(x0, y, z_face - reveal), v3(x1, top, z_face), finish));
        y += storey;
    }
    // Piers between window bays.
    let mut x = x0;
    while x <= x1 + 0.01 && out.len() < budget {
        out.push(bx(
            v3(x - 0.35, 0.0, z_face - reveal),
            v3(x + 0.35, height, z_face),
            finish,
        ));
        x += bay;
    }
}

pub fn build_scene(p: &SceneParams) -> BuiltScene {
    let mut scene = build_base(p);
    apply_moves(&mut scene, &p.moves);
    add_sensors(p, &mut scene);
    scene
}

fn build_base(p: &SceneParams) -> BuiltScene {
    let mut boxes: Vec<ScenePrim> = Vec::new();
    let mut lamps: Vec<SceneLamp> = Vec::new();
    let mut movables: Vec<Movable> = Vec::new();
    let rot = luminaire_rotation(p.rotation, p.tilt, false);
    let rot_far = luminaire_rotation(p.rotation, p.tilt, true);
    let deg = |d: f32| d.to_radians();
    let finish = match p.wall_finish {
        WallFinish::Render => mat::RENDER,
        WallFinish::Brick => mat::BRICK,
    };

    match p.kind {
        SceneKind::Planned => match p.plan.as_ref() {
            Some(plan) => build_planned(p, plan),
            None => build_planned(p, &ScenePlan::example()),
        },
        SceneKind::Street => {
            let half_length = 24.0_f32.max(p.count as f32 * p.spacing / 2.0 + p.height * 2.5);
            let xa = -half_length - 20.0;
            let xb = half_length + 20.0;
            // Near side (+Z): parking strip, kerb, sidewalk, soil beyond.
            let park0 = ROAD_HALF_WIDTH;
            let park1 = park0 + PARKING_STRIP;
            let walk1 = park1 + SIDEWALK;
            boxes.push(bx(v3(xa, 0.0, park0), v3(xb, 0.01, park1), mat::PARKING));
            boxes.push(bx(v3(xa, 0.0, park1), v3(xb, KERB_H, walk1), mat::PAVING));
            boxes.push(bx(
                v3(xa, KERB_H - 0.02, park1 - 0.12),
                v3(xb, KERB_H, park1),
                mat::KERB,
            ));
            // Far side (−Z): cycle lane, kerb, sidewalk, facade.
            let cyc1 = -(ROAD_HALF_WIDTH + CYCLE_LANE);
            let fwalk1 = cyc1 - SIDEWALK;
            boxes.push(bx(
                v3(xa, 0.0, cyc1),
                v3(xb, 0.01, -ROAD_HALF_WIDTH),
                mat::CYCLE,
            ));
            boxes.push(bx(v3(xa, 0.0, fwalk1), v3(xb, KERB_H, cyc1), mat::PAVING));
            boxes.push(bx(
                v3(xa, KERB_H - 0.02, cyc1),
                v3(xb, KERB_H, cyc1 + 0.12),
                mat::KERB,
            ));
            facade(
                -half_length - 6.0,
                half_length + 6.0,
                fwalk1 - 0.3,
                9.0,
                finish,
                3.6,
                &mut boxes,
                // Leaves room for 8 poles, 2 cars and the person.
                MAX_PRIMS - 120,
            );
            // Parked cars in the strip, clear of the poles.
            track_car(
                v3(-9.0, 0.0, park0 + PARKING_STRIP / 2.0),
                0.0,
                mat::CAR,
                &mut boxes,
                &mut movables,
            );
            track_car(
                v3(7.5, 0.0, park0 + PARKING_STRIP / 2.0),
                180.0,
                mat::CAR_SILVER,
                &mut boxes,
                &mut movables,
            );
            // Poles on the near sidewalk edge.
            let pole_z = park1 + 0.5;
            let xs = row_positions(p.count, p.spacing);
            for &x in &xs {
                let start = boxes.len();
                let pos = pole(x, pole_z, 1.0, p.height, !p.freestanding, &mut boxes);
                lamps.push(SceneLamp {
                    position: pos,
                    rotation: rot,
                    ..Default::default()
                });
                track_lamp(
                    &mut movables,
                    &lamps,
                    start..boxes.len(),
                    v3(x, 0.0, pole_z),
                );
            }
            let far_pole_z = cyc1 - 0.5;
            let far_xs: Vec<f32> = match p.street_layout {
                StreetLayout::Single => vec![],
                StreetLayout::Opposite => xs.clone(),
                StreetLayout::Staggered => xs.iter().map(|x| x + p.spacing / 2.0).collect(),
            };
            for x in far_xs {
                let start = boxes.len();
                let pos = pole(x, far_pole_z, -1.0, p.height, !p.freestanding, &mut boxes);
                lamps.push(SceneLamp {
                    position: pos,
                    rotation: rot_far,
                    ..Default::default()
                });
                track_lamp(
                    &mut movables,
                    &lamps,
                    start..boxes.len(),
                    v3(x, 0.0, far_pole_z),
                );
            }
            if p.show_figure {
                track_figure(v3(2.5, KERB_H, park1 + 1.0), &mut boxes, &mut movables);
            }
            let along = v3(2.0 * half_length, 0.0, 0.0);
            let road = receiver(
                v3(-half_length, 0.0, -ROAD_HALF_WIDTH),
                along,
                v3(0.0, 0.0, 2.0 * ROAD_HALF_WIDTH),
                Vec3::Y,
            );
            let road_grid_ = road_grid("carriageway", road.clone(), 2);
            let columns = road_grid_.nu;
            let cycle = receiver(
                v3(-half_length, 0.0, cyc1),
                along,
                v3(0.0, 0.0, CYCLE_LANE),
                Vec3::Y,
            );
            let far_walk = receiver(
                v3(-half_length, KERB_H, fwalk1),
                along,
                v3(0.0, 0.0, SIDEWALK),
                Vec3::Y,
            );
            let near_walk = receiver(
                v3(-half_length, KERB_H, park1),
                along,
                v3(0.0, 0.0, SIDEWALK),
                Vec3::Y,
            );
            let facade_face = receiver(
                v3(-half_length, 0.0, fwalk1 - 0.3),
                along,
                v3(0.0, 9.0, 0.0),
                Vec3::Z,
            );
            BuiltScene {
                movables: std::mem::take(&mut movables),
                kind: p.kind,
                prims: boxes,
                lamps,
                ground: 2,
                outdoor: true,
                key_lux: None,
                // From the near sidewalk, looking down the road with the facade opposite.
                view: CameraView {
                    target: v3(-3.0, 1.5, -1.0),
                    yaw: deg(62.0),
                    pitch: deg(17.0),
                    distance: 24.0_f32
                        .max(p.height * 2.4 + (p.count as f32 - 1.0) * p.spacing * 0.4),
                },
                limits: CameraLimits {
                    min_pitch: deg(2.0),
                    max_distance: 220.0,
                    ..Default::default()
                },
                receivers: vec![road, cycle.clone(), far_walk.clone()],
                grids: vec![
                    road_grid_,
                    strip_grid("cycle lane", cycle, columns, 0.75),
                    strip_grid("far sidewalk", far_walk, columns, 1.0),
                    strip_grid("near sidewalk", near_walk, columns, 1.0),
                    strip_grid("facade", facade_face, columns, 1.5),
                ],
            }
        }

        SceneKind::Parking => {
            // Two rows of bays (z 3..8 and −8..−3) around a 6 m aisle; poles
            // on the +Z edge, more than three add a row on the −Z edge.
            let lot_half_z = 8.5;
            let cols = p.count.min(3);
            let xs = row_positions(cols, p.spacing);
            let half_length = 14.0_f32.max(cols as f32 * p.spacing / 2.0 + p.height * 1.5);
            for &x in &xs {
                let start = boxes.len();
                let pos = pole(
                    x,
                    lot_half_z + 0.6,
                    1.0,
                    p.height,
                    !p.freestanding,
                    &mut boxes,
                );
                lamps.push(SceneLamp {
                    position: pos,
                    rotation: rot,
                    ..Default::default()
                });
                track_lamp(
                    &mut movables,
                    &lamps,
                    start..boxes.len(),
                    v3(x, 0.0, lot_half_z + 0.6),
                );
            }
            if p.count > 3 {
                let far = p.count - 3;
                for x in row_positions(far, p.spacing) {
                    let start = boxes.len();
                    let pos = pole(
                        x + p.spacing / 2.0,
                        -lot_half_z - 0.6,
                        -1.0,
                        p.height,
                        !p.freestanding,
                        &mut boxes,
                    );
                    lamps.push(SceneLamp {
                        position: pos,
                        rotation: rot_far,
                        ..Default::default()
                    });
                    track_lamp(
                        &mut movables,
                        &lamps,
                        start..boxes.len(),
                        v3(x + p.spacing / 2.0, 0.0, -lot_half_z - 0.6),
                    );
                }
            }
            // Bays run across the lot, so the cars stand nose-in along Z.
            for (cx, cz, yaw, paint) in [
                (-6.25, 5.5, 90.0, mat::CAR),
                (1.25, 5.5, 90.0, mat::CAR_SILVER),
                (-1.25, -5.5, -90.0, mat::CAR_RED),
                (8.75, -5.5, -90.0, mat::CAR),
            ] {
                track_car(v3(cx, 0.0, cz), yaw, paint, &mut boxes, &mut movables);
            }
            // Hedge behind the pole row.
            boxes.push(bx(
                v3(-half_length, 0.0, lot_half_z + 1.2),
                v3(half_length, 0.9, lot_half_z + 1.8),
                mat::HEDGE,
            ));
            if p.show_figure {
                track_figure(v3(4.0, 0.0, 0.6), &mut boxes, &mut movables);
            }
            let lot = receiver(
                v3(-half_length, 0.0, -lot_half_z),
                v3(2.0 * half_length, 0.0, 0.0),
                v3(0.0, 0.0, 2.0 * lot_half_z),
                Vec3::Y,
            );
            BuiltScene {
                movables: std::mem::take(&mut movables),
                kind: p.kind,
                prims: boxes,
                lamps,
                ground: 3,
                outdoor: true,
                key_lux: None,
                view: CameraView {
                    target: v3(0.0, 1.0, 0.0),
                    yaw: deg(145.0),
                    pitch: deg(32.0),
                    distance: 22.0_f32.max(p.height * 2.6 + (cols as f32 - 1.0) * p.spacing * 0.5),
                },
                limits: CameraLimits {
                    min_pitch: deg(3.0),
                    max_distance: 160.0,
                    ..Default::default()
                },
                receivers: vec![lot.clone()],
                grids: vec![norm_grid("parking lot", lot)],
            }
        }

        SceneKind::Facade => {
            let d = p.wall_distance;
            let height = 7.0_f32.max(p.height + 2.5);
            let half_width = 7.0_f32.max(p.count as f32 * p.spacing / 2.0 + 3.5);
            facade(
                -half_width - 2.0,
                half_width + 2.0,
                -d,
                height,
                finish,
                3.0,
                &mut boxes,
                MAX_PRIMS - 40,
            );
            // Entrance canopy and a planter in front.
            boxes.push(bx_soft(
                v3(-1.6, 3.1, -d),
                v3(1.6, 3.25, -d + 1.4),
                mat::METAL,
            ));
            boxes.push(bx(
                v3(half_width - 3.0, 0.0, -d + 3.0),
                v3(half_width - 0.8, 0.5, -d + 3.8),
                mat::KERB,
            ));
            boxes.push(bx(
                v3(half_width - 2.9, 0.5, -d + 3.1),
                v3(half_width - 0.9, 1.1, -d + 3.7),
                mat::HEDGE,
            ));
            for x in row_positions(p.count, p.spacing) {
                let start = boxes.len();
                lamps.push(SceneLamp {
                    position: v3(x, p.height, 0.0),
                    rotation: rot,
                    ..Default::default()
                });
                track_lamp(&mut movables, &lamps, start..start, v3(x, 0.0, 0.0));
            }
            if p.show_figure {
                track_figure(
                    v3(-half_width + 3.0, 0.0, -d + 2.2),
                    &mut boxes,
                    &mut movables,
                );
            }
            let along = v3(2.0 * half_width, 0.0, 0.0);
            let face = receiver(
                v3(-half_width, 0.0, -d),
                along,
                v3(0.0, height, 0.0),
                Vec3::Z,
            );
            let forecourt = receiver(v3(-half_width, 0.0, -d), along, v3(0.0, 0.0, 7.0), Vec3::Y);
            BuiltScene {
                movables: std::mem::take(&mut movables),
                kind: p.kind,
                prims: boxes,
                lamps,
                ground: 1,
                outdoor: true,
                key_lux: Some(120.0),
                view: CameraView {
                    target: v3(0.0, height * 0.35, -d),
                    yaw: deg(-30.0),
                    pitch: deg(12.0),
                    distance: 10.0_f32.max(half_width * 1.5),
                },
                limits: CameraLimits {
                    min_pitch: deg(1.0),
                    max_distance: 90.0,
                    ..Default::default()
                },
                receivers: vec![face.clone(), forecourt.clone()],
                grids: vec![norm_grid("facade", face), norm_grid("forecourt", forecourt)],
            }
        }

        SceneKind::Office => {
            let h = p.ceiling_height;
            let hw = OFFICE_W / 2.0;
            let hd = OFFICE_D / 2.0;
            let t = 0.25;
            let cut = |b: ScenePrim, point: Vec3, inward: Vec3| ScenePrim {
                cutaway: Some((point, inward)),
                ..b
            };
            boxes.push(bx(
                v3(-hw - t, -t, -hd - t),
                v3(hw + t, 0.0, hd + t),
                mat::CARPET,
            ));
            boxes.push(cut(
                bx(
                    v3(-hw - t, h, -hd - t),
                    v3(hw + t, h + t, hd + t),
                    mat::CEILING,
                ),
                v3(0.0, h, 0.0),
                Vec3::NEG_Y,
            ));
            boxes.push(cut(
                bx(v3(-hw - t, 0.0, -hd - t), v3(-hw, h, hd + t), mat::WALL),
                v3(-hw, 0.0, 0.0),
                Vec3::X,
            ));
            boxes.push(cut(
                bx(v3(hw, 0.0, -hd - t), v3(hw + t, h, hd + t), mat::WALL),
                v3(hw, 0.0, 0.0),
                Vec3::NEG_X,
            ));
            boxes.push(cut(
                bx(v3(-hw, 0.0, -hd - t), v3(hw, h, -hd), mat::WALL),
                v3(0.0, 0.0, -hd),
                Vec3::Z,
            ));
            boxes.push(cut(
                bx(v3(-hw, 0.0, hd), v3(hw, h, hd + t), mat::WALL),
                v3(0.0, 0.0, hd),
                Vec3::NEG_Z,
            ));
            // Window band in the −Z wall, dark at night.
            boxes.push(cut(
                bx(
                    v3(-hw + 0.6, 0.9, -hd - 0.02),
                    v3(hw - 0.6, 2.3, -hd + 0.01),
                    mat::GLASS,
                ),
                v3(0.0, 0.0, -hd),
                Vec3::Z,
            ));
            // Two desks facing the window, monitors, chairs.
            for (dx, dz) in [(-1.3, -0.4), (1.3, -0.4)] {
                boxes.push(bx(
                    v3(dx - 0.8, 0.72, dz - 0.4),
                    v3(dx + 0.8, 0.75, dz + 0.4),
                    mat::DESK,
                ));
                for (lx, lz) in [(-0.75, -0.35), (0.72, -0.35), (-0.75, 0.32), (0.72, 0.32)] {
                    boxes.push(bx(
                        v3(dx + lx, 0.0, dz + lz),
                        v3(dx + lx + 0.03, 0.72, dz + lz + 0.03),
                        mat::METAL,
                    ));
                }
                boxes.push(bx(
                    v3(dx - 0.3, 0.75, dz - 0.3),
                    v3(dx + 0.3, 1.12, dz - 0.26),
                    mat::GLASS,
                ));
                boxes.push(bx(
                    v3(dx - 0.25, 0.0, dz + 0.6),
                    v3(dx + 0.25, 0.46, dz + 1.1),
                    mat::FABRIC,
                ));
                boxes.push(bx(
                    v3(dx - 0.25, 0.46, dz + 1.02),
                    v3(dx + 0.25, 0.95, dz + 1.1),
                    mat::FABRIC,
                ));
            }
            // Cabinet along the +X wall.
            boxes.push(bx(v3(hw - 0.45, 0.0, -1.0), v3(hw, 1.8, 1.4), mat::WHITE));

            let (cols, rows) = match p.count {
                2 => (2, 1),
                4 => (2, 2),
                6 => (3, 2),
                _ => (1, 1),
            };
            let y = p.height.min(room_mount_limit(p));
            for r in 0..rows {
                for c in 0..cols {
                    let x = (c as f32 - (cols as f32 - 1.0) / 2.0) * (OFFICE_W / cols as f32);
                    let z = (r as f32 - (rows as f32 - 1.0) / 2.0) * (OFFICE_D / rows as f32);
                    let start = boxes.len();
                    lamps.push(SceneLamp {
                        position: v3(x, y, z),
                        rotation: rot,
                        ..Default::default()
                    });
                    let top = rot * Vec3::Y;
                    let anchor = v3(x + top.x * 0.08, y + 0.08, z + top.z * 0.08);
                    if anchor.y < h - 0.02 {
                        boxes.push(bx_soft(
                            v3(anchor.x - 0.005, anchor.y, anchor.z - 0.005),
                            v3(anchor.x + 0.005, h, anchor.z + 0.005),
                            mat::METAL,
                        ));
                    }
                    track_lamp(&mut movables, &lamps, start..boxes.len(), v3(x, 0.0, z));
                }
            }
            if p.show_figure {
                track_figure(v3(-0.2, 0.0, 1.5), &mut boxes, &mut movables);
            }
            let up = v3(0.0, h, 0.0);
            let workplane = receiver(
                v3(-hw + MARGIN, WORKPLANE, -hd + MARGIN),
                v3(OFFICE_W - 2.0 * MARGIN, 0.0, 0.0),
                v3(0.0, 0.0, OFFICE_D - 2.0 * MARGIN),
                Vec3::Y,
            );
            let floor = receiver(
                v3(-hw, 0.0, -hd),
                v3(OFFICE_W, 0.0, 0.0),
                v3(0.0, 0.0, OFFICE_D),
                Vec3::Y,
            );
            let walls = [
                (
                    "wall +Z",
                    receiver(v3(-hw, 0.0, hd), v3(OFFICE_W, 0.0, 0.0), up, Vec3::NEG_Z),
                ),
                (
                    "wall -Z",
                    receiver(v3(-hw, 0.0, -hd), v3(OFFICE_W, 0.0, 0.0), up, Vec3::Z),
                ),
                (
                    "wall -X",
                    receiver(v3(-hw, 0.0, -hd), v3(0.0, 0.0, OFFICE_D), up, Vec3::X),
                ),
                (
                    "wall +X",
                    receiver(v3(hw, 0.0, -hd), v3(0.0, 0.0, OFFICE_D), up, Vec3::NEG_X),
                ),
            ];
            let mut grids = vec![norm_grid("workplane 0.75 m", workplane.clone())];
            for (name, r) in walls.iter() {
                grids.push(norm_grid(name, r.clone()));
            }
            BuiltScene {
                movables: std::mem::take(&mut movables),
                kind: p.kind,
                prims: boxes,
                lamps,
                ground: 0,
                outdoor: false,
                key_lux: None,
                view: CameraView {
                    target: v3(0.2, 0.9, 0.0),
                    yaw: deg(38.0),
                    pitch: deg(30.0),
                    distance: 11.0,
                },
                limits: CameraLimits {
                    min_pitch: deg(2.0),
                    max_distance: 40.0,
                    ..Default::default()
                },
                receivers: vec![workplane, floor, walls[0].1.clone(), walls[2].1.clone()],
                grids,
            }
        }
        SceneKind::Greenhouse => {
            let hl = GREENHOUSE_L / 2.0;
            let hw = GREENHOUSE_W / 2.0;
            let eave = GREENHOUSE_EAVE;
            let ridge = eave + GREENHOUSE_RISE;
            let t = 0.02;
            let cut = |b: ScenePrim, point: Vec3, inward: Vec3| ScenePrim {
                cutaway: Some((point, inward)),
                ..b
            };
            // Concrete floor with a soil strip under each bench.
            boxes.push(bx(
                v3(-hl - 0.3, -0.25, -hw - 0.3),
                v3(hl + 0.3, 0.0, hw + 0.3),
                mat::PAVING,
            ));
            // Glass walls (dark at night) and gables, cut away towards the eye.
            boxes.push(cut(
                bx(v3(-hl, 0.0, -hw - t), v3(hl, eave, -hw), mat::GLASS),
                v3(0.0, 0.0, -hw),
                Vec3::Z,
            ));
            boxes.push(cut(
                bx(v3(-hl, 0.0, hw), v3(hl, eave, hw + t), mat::GLASS),
                v3(0.0, 0.0, hw),
                Vec3::NEG_Z,
            ));
            boxes.push(cut(
                bx(v3(-hl - t, 0.0, -hw), v3(-hl, eave, hw), mat::GLASS),
                v3(-hl, 0.0, 0.0),
                Vec3::X,
            ));
            boxes.push(cut(
                bx(v3(hl, 0.0, -hw), v3(hl + t, eave, hw), mat::GLASS),
                v3(hl, 0.0, 0.0),
                Vec3::NEG_X,
            ));
            // Ridge roof: two glass panes leaning on the ridge beam.
            let slope = (GREENHOUSE_RISE / hw).atan();
            let pane_half = (hw * hw + GREENHOUSE_RISE * GREENHOUSE_RISE).sqrt() / 2.0;
            for side in [-1.0_f32, 1.0] {
                let centre = v3(0.0, eave + GREENHOUSE_RISE / 2.0, side * hw / 2.0);
                let rot = Mat3::from_quat(Quat::from_rotation_x(-side * slope));
                boxes.push(cut(
                    obb(centre, v3(hl, t, pane_half), rot, mat::GLASS),
                    centre,
                    v3(0.0, -1.0, 0.0),
                ));
            }
            // Aluminium frame: posts every 2.5 m, eave rails and ridge beam.
            let mut x = -hl;
            while x <= hl + 0.01 {
                for z in [-hw, hw] {
                    boxes.push(cylinder(v3(x, 0.0, z), v3(x, eave, z), 0.04, mat::METAL));
                }
                x += 2.5;
            }
            for z in [-hw, hw] {
                boxes.push(cylinder(
                    v3(-hl, eave, z),
                    v3(hl, eave, z),
                    0.03,
                    mat::METAL,
                ));
            }
            boxes.push(cylinder(
                v3(-hl, ridge, 0.0),
                v3(hl, ridge, 0.0),
                0.04,
                mat::METAL,
            ));
            // Two benches with a dense crop on top, canopy at `canopy_height`.
            let canopy = p.canopy_height.max(BENCH_TOP + 0.05);
            for &bz in BENCH_Z.iter() {
                let (z0, z1) = (bz - BENCH_W / 2.0, bz + BENCH_W / 2.0);
                boxes.push(bx(
                    v3(-hl + 0.5, BENCH_TOP - 0.04, z0),
                    v3(hl - 0.5, BENCH_TOP, z1),
                    mat::WHITE,
                ));
                let mut lx = -hl + 0.7;
                while lx < hl - 0.5 {
                    for lz in [z0 + 0.1, z1 - 0.1] {
                        boxes.push(cylinder(
                            v3(lx, 0.0, lz),
                            v3(lx, BENCH_TOP - 0.04, lz),
                            0.025,
                            mat::METAL,
                        ));
                    }
                    lx += 2.0;
                }
                // Crop: a soil layer and a foliage slab whose top is the canopy.
                boxes.push(bx(
                    v3(-hl + 0.55, BENCH_TOP, z0 + 0.05),
                    v3(hl - 0.55, BENCH_TOP + 0.12, z1 - 0.05),
                    mat::SOIL,
                ));
                let mut slab = bx(
                    v3(-hl + 0.55, BENCH_TOP + 0.12, z0 + 0.05),
                    v3(hl - 0.55, canopy, z1 - 0.05),
                    mat::HEDGE,
                );
                slab.casts_shadow = true;
                boxes.push(slab);
            }
            // Luminaires: `count` fixtures split over the two benches, in a
            // row along X with `spacing`, hung from the ridge on wires.
            let per_bench = p.count.div_ceil(2).max(1);
            let rows: &[f32] = if p.count >= 2 {
                &BENCH_Z
            } else {
                &BENCH_Z[..1]
            };
            let y = p.height.min(ridge - 0.15).max(canopy + 0.2);
            let mut placed = 0;
            for &bz in rows {
                for c in 0..per_bench {
                    if placed >= p.count {
                        break;
                    }
                    let x = (c as f32 - (per_bench as f32 - 1.0) / 2.0) * p.spacing;
                    let start = boxes.len();
                    lamps.push(SceneLamp {
                        position: v3(x, y, bz),
                        rotation: rot,
                        ..Default::default()
                    });
                    boxes.push(bx_soft(
                        v3(x - 0.004, y + 0.08, bz - 0.004),
                        v3(x + 0.004, ridge, bz + 0.004),
                        mat::METAL,
                    ));
                    track_lamp(&mut movables, &lamps, start..boxes.len(), v3(x, 0.0, bz));
                    placed += 1;
                }
            }
            if p.show_figure {
                track_figure(v3(hl - 1.6, 0.0, 0.0), &mut boxes, &mut movables);
            }
            let canopy_a = receiver(
                v3(-hl + 0.6, canopy, BENCH_Z[0] - BENCH_W / 2.0 + 0.1),
                v3(GREENHOUSE_L - 1.2, 0.0, 0.0),
                v3(0.0, 0.0, BENCH_W - 0.2),
                Vec3::Y,
            );
            let canopy_b = receiver(
                v3(-hl + 0.6, canopy, BENCH_Z[1] - BENCH_W / 2.0 + 0.1),
                v3(GREENHOUSE_L - 1.2, 0.0, 0.0),
                v3(0.0, 0.0, BENCH_W - 0.2),
                Vec3::Y,
            );
            let aisle = receiver(
                v3(-hl, 0.0, -0.6),
                v3(GREENHOUSE_L, 0.0, 0.0),
                v3(0.0, 0.0, 1.2),
                Vec3::Y,
            );
            let grids = vec![
                norm_grid("canopy A", canopy_a.clone()),
                norm_grid("canopy B", canopy_b.clone()),
                norm_grid("aisle", aisle.clone()),
            ];
            BuiltScene {
                movables: std::mem::take(&mut movables),
                kind: p.kind,
                prims: boxes,
                lamps,
                ground: 0,
                outdoor: false,
                key_lux: Some(600.0),
                view: CameraView {
                    target: v3(0.0, 1.1, 0.0),
                    yaw: deg(34.0),
                    pitch: deg(26.0),
                    distance: 15.0,
                },
                limits: CameraLimits {
                    min_pitch: deg(2.0),
                    max_distance: 45.0,
                    ..Default::default()
                },
                receivers: vec![canopy_a, canopy_b, aisle],
                grids,
            }
        }
    }
}

/// Default luminaire orientation per scene, aiming an asymmetric throw at the
/// carriageway, the bays or the facade. All lie on the −Z side of the lamp,
/// i.e. world azimuth −90° measured from +X towards +Z; a yaw of `r` turns
/// local C angle `c` to world azimuth `c − r`.
pub fn auto_rotation(kind: SceneKind, throw_c: f32, asymmetry: f32) -> f32 {
    let asymmetric = asymmetry > 0.12;
    let r = match kind {
        SceneKind::Street | SceneKind::Parking | SceneKind::Facade if asymmetric => throw_c + 90.0,
        _ => 0.0,
    };
    let r = (r % 360.0 + 540.0) % 360.0 - 180.0;
    r.round()
}

/// The scene that suits a luminaire: an asymmetric throw with road-class
/// flux is a street light, a narrow beam a facade projector, a big
/// symmetric emitter an area light, everything else belongs in the office.
pub fn suggest_scene(ldt: &Eulumdat) -> SceneKind {
    let throw = PhotometricCalculations::throw_direction(ldt);
    let flux = ldt.luminous_flux();
    let peak = ldt.max_intensity();
    let name = ldt.luminaire_name.to_ascii_lowercase();
    if ["grow", "hortic", "ppfd", "greenhouse", "plant"]
        .iter()
        .any(|k| name.contains(k))
    {
        SceneKind::Greenhouse
    } else if ldt.downward_flux_fraction < 50.0 {
        // Uplights belong indoors, whatever their throw.
        SceneKind::Office
    } else if peak > 600.0 {
        // Narrow beam (cd/klm): a projector for a facade.
        SceneKind::Facade
    } else if throw.asymmetry > 0.12 && flux >= 4000.0 {
        SceneKind::Street
    } else if flux >= 12000.0 {
        SceneKind::Parking
    } else {
        SceneKind::Office
    }
}

/// True when a cutaway surface is hidden from an eye at `eye`.
pub fn hidden_from(b: &ScenePrim, eye: Vec3) -> bool {
    match b.cutaway {
        Some((point, inward)) => (eye - point).dot(inward) < 0.0,
        None => false,
    }
}

/// Ray-vs-primitive bounds test from the eye for the glare term and the point overlay.
/// Cutaway surfaces the camera looks through do not block.
pub fn segment_blocked(scene: &BuiltScene, from: Vec3, to: Vec3) -> bool {
    let dir = to - from;
    for b in &scene.prims {
        if !b.casts_shadow || hidden_from(b, from) {
            continue;
        }
        let (bmin, bmax) = b.aabb();
        let mut t0: f32 = 0.0;
        let mut t1: f32 = 0.995;
        let mut hit = true;
        for a in 0..3 {
            if dir[a].abs() < 1e-9 {
                if from[a] < bmin[a] || from[a] > bmax[a] {
                    hit = false;
                    break;
                }
                continue;
            }
            let mut ta = (bmin[a] - from[a]) / dir[a];
            let mut tb = (bmax[a] - from[a]) / dir[a];
            if ta > tb {
                std::mem::swap(&mut ta, &mut tb);
            }
            t0 = t0.max(ta);
            t1 = t1.min(tb);
            if t0 > t1 {
                hit = false;
                break;
            }
        }
        if hit {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    #[test]
    fn greenhouse_fits_the_budgets() {
        let mut p = SceneParams::defaults(SceneKind::Greenhouse);
        p.count = 12;
        let s = build_scene(&p);
        assert!(s.prims.len() <= MAX_PRIMS, "{} prims", s.prims.len());
        assert_eq!(s.lamps.len(), 12);
        assert_eq!(s.grids.len(), 3);
        assert!(s.grids[0].receiver.origin.y > BENCH_TOP);
    }

    #[test]
    fn suggested_scene_follows_the_luminaire() {
        let road = Eulumdat::parse(include_str!(
            "../../eulumdat-wasm/templates/road_luminaire.ldt"
        ))
        .unwrap();
        assert_eq!(suggest_scene(&road), SceneKind::Street);
        let office = Eulumdat::parse(include_str!(
            "../../eulumdat-wasm/templates/fluorescent_luminaire.ldt"
        ))
        .unwrap();
        assert_eq!(suggest_scene(&office), SceneKind::Office);
        let projector =
            Eulumdat::parse(include_str!("../../eulumdat-wasm/templates/projector.ldt")).unwrap();
        assert_eq!(suggest_scene(&projector), SceneKind::Facade);
    }

    use super::*;

    #[test]
    fn tilt_raises_the_beam_towards_the_road_whatever_the_yaw() {
        for yaw in [0.0, 90.0, -180.0, 37.0] {
            let r = luminaire_rotation(yaw, 30.0, false);
            let nadir = r * Vec3::NEG_Y;
            assert!(nadir.z < -0.4, "yaw {yaw}: {nadir:?}");
            assert!(nadir.y < -0.8, "yaw {yaw}: {nadir:?}");
            assert!(nadir.x.abs() < 1e-5, "yaw {yaw}: {nadir:?}");
        }
        // Far side of the street: towards +Z, i.e. still towards the road.
        let nadir = luminaire_rotation(0.0, 30.0, true) * Vec3::NEG_Y;
        assert!(nadir.z > 0.4, "{nadir:?}");
    }

    #[test]
    fn yaw_turns_c0_towards_neg_z() {
        // auto_rotation aims the throw at −Z: a throw along C0 needs +90°.
        let r = luminaire_rotation(auto_rotation(SceneKind::Street, 0.0, 1.0), 0.0, false);
        let c0 = r * Vec3::X;
        assert!(c0.z < -0.99, "{c0:?}");
    }

    #[test]
    fn scenes_fit_the_gpu_limits() {
        for kind in SceneKind::ALL {
            let mut p = SceneParams::defaults(kind);
            p.count = *p.count_options().last().unwrap();
            p.street_layout = StreetLayout::Opposite;
            // every sensor away from the poles: each gets a post
            p.sensors = (0..MAX_SENSORS)
                .map(|i| SceneSensor {
                    x: 0.7 * i as f32 - 1.0,
                    z: 0.3,
                })
                .collect();
            let s = build_scene(&p);
            assert!(
                s.prims.len() <= MAX_PRIMS,
                "{kind:?}: {} prims",
                s.prims.len()
            );
            assert!(
                s.lamps.len() <= MAX_LAMPS,
                "{kind:?}: {} lamps",
                s.lamps.len()
            );
            assert!(!s.grids.is_empty());
            for g in &s.grids {
                assert!(g.nu * g.nv > 0, "{kind:?}: empty grid {}", g.name);
            }
        }
    }

    #[test]
    fn office_measures_on_the_workplane() {
        let s = build_scene(&SceneParams::defaults(SceneKind::Office));
        let g = &s.grids[0];
        assert_eq!(g.name, "workplane 0.75 m");
        assert!((g.receiver.origin.y - WORKPLANE).abs() < 1e-6);
    }

    #[test]
    fn every_scene_has_movable_lamps_and_a_person() {
        for kind in SceneKind::ALL {
            let mut p = SceneParams::defaults(kind);
            p.count = *p.count_options().last().unwrap();
            let s = build_scene(&p);
            for i in 0..s.lamps.len() {
                let m = s
                    .movables
                    .iter()
                    .find(|m| m.id == ObjectId::Luminaire(i as u8))
                    .unwrap_or_else(|| panic!("{kind:?}: lamp {i} not movable"));
                assert_eq!(m.lamp, Some(i));
                assert_eq!(m.handle, s.lamps[i].position);
            }
            assert!(s.movables.iter().any(|m| m.id == ObjectId::Person));
        }
        let cars = |k| {
            build_scene(&SceneParams::defaults(k))
                .movables
                .iter()
                .filter(|m| matches!(m.id, ObjectId::Car(_)))
                .count()
        };
        assert_eq!(cars(SceneKind::Street), 2);
        assert_eq!(cars(SceneKind::Parking), 4);
    }

    #[test]
    fn moving_an_object_moves_its_parts() {
        let mut p = SceneParams::defaults(SceneKind::Street);
        p.count = 2;
        let before = build_scene(&p);
        let pole = before
            .movables
            .iter()
            .find(|m| m.id == ObjectId::Luminaire(1))
            .unwrap()
            .clone();
        let car = before
            .movables
            .iter()
            .find(|m| m.id == ObjectId::Car(0))
            .unwrap()
            .clone();
        let d = Vec3::new(3.0, 0.0, -1.0);
        p.moves = vec![
            (
                ObjectId::Luminaire(1),
                Vec2::new(pole.anchor.x + d.x, pole.anchor.z + d.z),
            ),
            (
                ObjectId::Car(0),
                Vec2::new(car.anchor.x + d.x, car.anchor.z + d.z),
            ),
        ];
        let after = build_scene(&p);
        assert_eq!(after.prims.len(), before.prims.len());
        assert!((after.lamps[1].position - (before.lamps[1].position + d)).length() < 1e-5);
        assert!((after.lamps[0].position - before.lamps[0].position).length() < 1e-6);
        for range in [pole.prims.clone(), car.prims.clone()] {
            assert!(!range.is_empty());
            for i in range {
                let (a0, _) = before.prims[i].aabb();
                let (a1, _) = after.prims[i].aabb();
                assert!((a1 - (a0 + d)).length() < 1e-4, "prim {i}");
            }
        }
        // everything else stays
        let (o0, _) = before.prims[0].aabb();
        let (o1, _) = after.prims[0].aabb();
        assert_eq!(o0, o1);
    }

    #[test]
    fn sensors_sit_on_poles_or_posts() {
        let mut p = SceneParams::defaults(SceneKind::Parking);
        let base = build_scene(&p);
        let pole = base
            .movables
            .iter()
            .find(|m| m.id == ObjectId::Luminaire(0))
            .unwrap()
            .anchor;
        p.sensors = vec![
            SceneSensor {
                x: pole.x,
                z: pole.z,
            },
            SceneSensor { x: 0.0, z: 0.0 },
        ];
        p.sensor_height = 4.0;
        let s = build_scene(&p);
        let on_pole = s
            .movables
            .iter()
            .find(|m| m.id == ObjectId::Sensor(0))
            .unwrap();
        let free = s
            .movables
            .iter()
            .find(|m| m.id == ObjectId::Sensor(1))
            .unwrap();
        assert_eq!(on_pole.prims.len(), 1, "head only");
        assert_eq!(free.prims.len(), 2, "post and head");
        assert!((on_pole.handle.y - 4.0).abs() < 1e-6);
        // the office keeps them in the ceiling
        let mut o = SceneParams::defaults(SceneKind::Office);
        o.sensors = vec![SceneSensor { x: 0.0, z: 0.0 }];
        o.sensor_height = 9.0;
        let s = build_scene(&o);
        let h = s.movables.last().unwrap().handle.y;
        assert!((h - (o.ceiling_height - 0.02)).abs() < 1e-6);
    }

    fn l_room() -> ScenePlan {
        ScenePlan {
            name: "L room".into(),
            indoor: true,
            outline: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(10.0, 0.0),
                Vec2::new(10.0, 4.0),
                Vec2::new(4.0, 4.0),
                Vec2::new(4.0, 8.0),
                Vec2::new(0.0, 8.0),
            ],
            height: 3.0,
            reflectances: [0.7, 0.5, 0.2],
            plane: 0.8,
            luminaires: [(2.0, 2.0), (7.0, 2.0), (2.0, 6.0)]
                .iter()
                .map(|&(x, z)| PlanLuminaire {
                    position: Vec3::new(x, 2.5, z),
                    rotation_deg: 0.0,
                    tilt_deg: 0.0,
                    pole: None,
                    pendant: true,
                    ..Default::default()
                })
                .collect(),
            sensors: vec![Vec2::new(5.0, 2.0)],
            sensor_height: 3.0,
            note: None,
            ..Default::default()
        }
    }

    #[test]
    fn a_plan_over_the_lamp_budget_keeps_its_emergency_lamps() {
        let mut plan = l_room();
        plan.luminaires = (0..MAX_LAMPS + 20)
            .map(|i| PlanLuminaire {
                position: Vec3::new(1.0 + (i % 9) as f32, 2.9, 1.0 + (i / 9 % 6) as f32),
                role: if i >= MAX_LAMPS + 9 {
                    LampRole::Emergency
                } else {
                    LampRole::Normal
                },
                ..Default::default()
            })
            .collect();
        let mut p = SceneParams::defaults(SceneKind::Planned);
        p.plan = Some(std::sync::Arc::new(plan));
        let s = build_scene(&p);
        assert_eq!(s.lamps.len(), MAX_LAMPS);
        let em = s
            .lamps
            .iter()
            .filter(|l| l.role == LampRole::Emergency)
            .count();
        assert_eq!(em, 11, "every emergency lamp stays");
    }

    #[test]
    fn planned_room_follows_the_outline() {
        let mut p = SceneParams::defaults(SceneKind::Planned);
        p.plan = Some(std::sync::Arc::new(l_room()));
        let s = build_scene(&p);
        assert_eq!(s.kind, SceneKind::Planned);
        assert_eq!(s.lamps.len(), 3);
        // floor, ceiling, 6 walls (+ rods, person)
        let walls = s.prims.iter().filter(|b| b.material == mat::WALL).count();
        assert_eq!(walls, 6);
        assert!(s
            .prims
            .iter()
            .any(|b| b.material == mat::WALL && (b.reflectance - 0.5).abs() < 1e-6));
        assert!(s
            .prims
            .iter()
            .any(|b| b.material == mat::CEILING && (b.reflectance - 0.7).abs() < 1e-6));
        // every wall is hidden from a camera outside the room on its side
        assert!(s
            .prims
            .iter()
            .filter(|b| b.material == mat::WALL)
            .all(|b| b.cutaway.is_some()));
        // the grid measures only inside the L, on the workplane
        let layout = crate::measure::measurement_layout(&s);
        assert!(!layout.points.is_empty());
        let outline = &l_room().outline;
        for pt in &layout.points {
            assert!(outline_contains(
                outline,
                Vec2::new(pt.position.x, pt.position.z)
            ));
            assert!((pt.position.y - 0.8).abs() < 0.05);
        }
        // the notch (x 4..10, z 4..8) has no points
        assert!(!layout
            .points
            .iter()
            .any(|pt| pt.position.x > 4.2 && pt.position.z > 4.2));
        // area and perimeter of the L
        let (a, per) = outline_area_perimeter(outline);
        assert!((a - 56.0).abs() < 1e-4);
        assert!((per - 36.0).abs() < 1e-4);
        // the person stands inside
        let person = s
            .movables
            .iter()
            .find(|m| m.id == ObjectId::Person)
            .unwrap();
        assert!(outline_contains(
            outline,
            Vec2::new(person.anchor.x, person.anchor.z)
        ));
    }

    #[test]
    fn plan_rotation_matches_the_planners() {
        // C0 at 90° points along +z (plan y); tilt swings the nadir to C0
        let r = plan_rotation(90.0, 0.0);
        assert!((r * Vec3::X - Vec3::Z).length() < 1e-5);
        let t = plan_rotation(0.0, 30.0) * Vec3::NEG_Y;
        assert!(t.x > 0.49 && t.x < 0.51 && t.y < -0.8, "{t}");
    }

    #[test]
    fn luminaires_on_one_pole_move_together() {
        let foot = Vec2::new(5.0, 5.0);
        let lum = |dx: f32| PlanLuminaire {
            position: Vec3::new(5.0 + dx, 8.0, 5.0),
            rotation_deg: if dx > 0.0 { 0.0 } else { 180.0 },
            tilt_deg: 5.0,
            pole: Some(foot),
            pendant: false,
            ..Default::default()
        };
        let plan = ScenePlan {
            name: "area".into(),
            indoor: false,
            outline: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(20.0, 0.0),
                Vec2::new(20.0, 12.0),
                Vec2::new(0.0, 12.0),
            ],
            height: 0.0,
            reflectances: [0.0, 0.0, 0.1],
            plane: 0.0,
            luminaires: vec![lum(1.5), lum(-1.5)],
            sensors: vec![],
            sensor_height: 4.0,
            note: None,
            ..Default::default()
        };
        let mut p = SceneParams::defaults(SceneKind::Planned);
        p.plan = Some(std::sync::Arc::new(plan));
        p.show_figure = false;
        let s = build_scene(&p);
        assert!(s.outdoor);
        assert_eq!(s.lamps.len(), 2);
        // one pole (2 cylinders) + 2 arms + 4 kerbs
        let metal = s.prims.iter().filter(|b| b.material == mat::METAL).count();
        assert_eq!(metal, 4);
        let pole = s
            .movables
            .iter()
            .find(|m| m.id == ObjectId::Luminaire(0))
            .unwrap();
        assert_eq!(pole.extra_lamps, vec![1]);
        assert!(!s.movables.iter().any(|m| m.id == ObjectId::Luminaire(1)));
        p.moves = vec![(ObjectId::Luminaire(0), Vec2::new(8.0, 5.0))];
        let moved = build_scene(&p);
        for i in 0..2 {
            let d = moved.lamps[i].position - s.lamps[i].position;
            assert!(
                (d - Vec3::new(3.0, 0.0, 0.0)).length() < 1e-5,
                "lamp {i}: {d}"
            );
        }
        assert!(moved.prims.len() == s.prims.len());
    }
}
