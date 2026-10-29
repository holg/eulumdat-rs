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

use glam::{Mat3, Quat, Vec3};

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
}

impl SceneKind {
    pub const ALL: [SceneKind; 4] = [
        SceneKind::Street,
        SceneKind::Parking,
        SceneKind::Facade,
        SceneKind::Office,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SceneKind::Street => "Street",
            SceneKind::Parking => "Parking lot",
            SceneKind::Facade => "Facade",
            SceneKind::Office => "Office",
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
    /// The luminaire stands on the ground (its body reaches down to it):
    /// no pole, no arm.
    pub freestanding: bool,
    /// Half extents of the luminaire housing (m), used to keep it clear of the ceiling.
    pub housing_half: Vec3,
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
            housing_half: Vec3::new(0.1, 0.04, 0.1),
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
                ..base
            },
            SceneKind::Office => Self {
                height: 2.8 - CEILING_CLEARANCE,
                spacing: 2.0,
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
        }
    }

    pub fn count_options(&self) -> &'static [u32] {
        match self.kind {
            SceneKind::Street => &[1, 2, 3, 4],
            SceneKind::Parking => &[1, 2, 3, 4, 6],
            SceneKind::Facade => &[1, 2, 3, 4, 5],
            SceneKind::Office => &[1, 2, 4, 6],
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
}

impl ScenePrim {
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

#[derive(Clone, Debug)]
pub struct SceneLamp {
    pub position: Vec3,
    /// Luminaire local → world.
    pub rotation: Mat3,
}

/// A calculation surface: `origin + s·u + t·v`, `normal` facing the light.
#[derive(Clone, Debug)]
pub struct Receiver {
    pub origin: Vec3,
    pub u: Vec3,
    pub v: Vec3,
    pub normal: Vec3,
}

/// A calculation grid with `nu × nv` measurement points at the cell centres.
#[derive(Clone, Debug)]
pub struct MeasureGrid {
    pub name: &'static str,
    pub receiver: Receiver,
    pub nu: usize,
    pub nv: usize,
}

/// Home camera: orbit around `target`.
#[derive(Clone, Copy, Debug, PartialEq)]
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
}

pub const MAX_PRIMS: usize = 192;
pub const MAX_LAMPS: usize = 16;

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
    }
}

/// EN 13201-3 road field: three rows per lane, longitudinal step no more than 3 m.
fn road_grid(name: &'static str, r: Receiver, lanes: u32) -> MeasureGrid {
    MeasureGrid {
        name,
        nu: ((r.u.length() / 3.0).ceil() as usize).clamp(1, ROAD_GRID_MAX_AXIS),
        nv: 3 * lanes as usize,
        receiver: r,
    }
}

/// Fixed spacing, for strips next to the road (same columns as the road).
fn strip_grid(name: &'static str, r: Receiver, nu: usize, step: f32) -> MeasureGrid {
    MeasureGrid {
        name,
        nu,
        nv: ((r.v.length() / step).ceil() as usize).clamp(1, MAX_GRID_AXIS),
        receiver: r,
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
    let mut boxes: Vec<ScenePrim> = Vec::new();
    let mut lamps: Vec<SceneLamp> = Vec::new();
    let rot = luminaire_rotation(p.rotation, p.tilt, false);
    let rot_far = luminaire_rotation(p.rotation, p.tilt, true);
    let deg = |d: f32| d.to_radians();
    let finish = match p.wall_finish {
        WallFinish::Render => mat::RENDER,
        WallFinish::Brick => mat::BRICK,
    };

    match p.kind {
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
            car(
                v3(-9.0, 0.0, park0 + PARKING_STRIP / 2.0),
                0.0,
                mat::CAR,
                &mut boxes,
            );
            car(
                v3(7.5, 0.0, park0 + PARKING_STRIP / 2.0),
                180.0,
                mat::CAR_SILVER,
                &mut boxes,
            );
            // Poles on the near sidewalk edge.
            let pole_z = park1 + 0.5;
            let xs = row_positions(p.count, p.spacing);
            for &x in &xs {
                let pos = pole(x, pole_z, 1.0, p.height, !p.freestanding, &mut boxes);
                lamps.push(SceneLamp {
                    position: pos,
                    rotation: rot,
                });
            }
            let far_pole_z = cyc1 - 0.5;
            let far_xs: Vec<f32> = match p.street_layout {
                StreetLayout::Single => vec![],
                StreetLayout::Opposite => xs.clone(),
                StreetLayout::Staggered => xs.iter().map(|x| x + p.spacing / 2.0).collect(),
            };
            for x in far_xs {
                let pos = pole(x, far_pole_z, -1.0, p.height, !p.freestanding, &mut boxes);
                lamps.push(SceneLamp {
                    position: pos,
                    rotation: rot_far,
                });
            }
            if p.show_figure {
                figure(v3(2.5, KERB_H, park1 + 1.0), &mut boxes);
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
                });
            }
            if p.count > 3 {
                let far = p.count - 3;
                for x in row_positions(far, p.spacing) {
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
                    });
                }
            }
            // Bays run across the lot, so the cars stand nose-in along Z.
            for (cx, cz, yaw, paint) in [
                (-6.25, 5.5, 90.0, mat::CAR),
                (1.25, 5.5, 90.0, mat::CAR_SILVER),
                (-1.25, -5.5, -90.0, mat::CAR_RED),
                (8.75, -5.5, -90.0, mat::CAR),
            ] {
                car(v3(cx, 0.0, cz), yaw, paint, &mut boxes);
            }
            // Hedge behind the pole row.
            boxes.push(bx(
                v3(-half_length, 0.0, lot_half_z + 1.2),
                v3(half_length, 0.9, lot_half_z + 1.8),
                mat::HEDGE,
            ));
            if p.show_figure {
                figure(v3(4.0, 0.0, 0.6), &mut boxes);
            }
            let lot = receiver(
                v3(-half_length, 0.0, -lot_half_z),
                v3(2.0 * half_length, 0.0, 0.0),
                v3(0.0, 0.0, 2.0 * lot_half_z),
                Vec3::Y,
            );
            BuiltScene {
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
                lamps.push(SceneLamp {
                    position: v3(x, p.height, 0.0),
                    rotation: rot,
                });
            }
            if p.show_figure {
                figure(v3(-half_width + 3.0, 0.0, -d + 2.2), &mut boxes);
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
                    lamps.push(SceneLamp {
                        position: v3(x, y, z),
                        rotation: rot,
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
                }
            }
            if p.show_figure {
                figure(v3(-0.2, 0.0, 1.5), &mut boxes);
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
}
