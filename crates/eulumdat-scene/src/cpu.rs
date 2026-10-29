//! CPU tracer: the same light transport as `trace.wgsl`, in Rust.
//!
//! It consumes the very same [`FrameInput`] the GPU gets, so the two paths
//! can be compared sample for sample in spirit: same NEE against the IES
//! table, same GGX + Lambert BSDF, same materials, same present step. It is
//! far slower than the GPU (a few samples per second at preview size), so
//! it serves as a fallback where WebGPU is missing and as the reference for
//! the grid values.

use crate::gpu::{GpuLamp, GpuPrim, Params, PRIM_CAPSULE, PRIM_CYLINDER, PRIM_SPHERE};
use crate::mesh::{LuminaireMesh, TRI_LUMINOUS};
use crate::photometry::IesTable;
use crate::viewer::FrameInput;
use glam::{Mat3, UVec2, Vec2, Vec3, Vec4};

const PI: f32 = std::f32::consts::PI;
const TWO_PI: f32 = 2.0 * PI;
const INV_PI: f32 = 1.0 / PI;
const EPS: f32 = 1.0e-3;
const FAR: f32 = 1.0e20;
/// Just inside ±π/2, as the shader clamps the equiangular sample.
const HALF_PI_IN: f32 = std::f32::consts::FRAC_PI_2 - 1.0e-4;
const LUMA: Vec3 = Vec3::new(0.2126, 0.7152, 0.0722);

mod mat {
    pub const PAVING: i32 = 0;
    pub const ASPHALT: i32 = 1;
    pub const PARKING: i32 = 2;
    pub const CYCLE: i32 = 3;
    pub const KERB: i32 = 4;
    pub const RENDER: i32 = 5;
    pub const BRICK: i32 = 6;
    pub const GLASS: i32 = 7;
    pub const CARPET: i32 = 8;
    pub const WALL: i32 = 9;
    pub const CEILING: i32 = 10;
    pub const METAL: i32 = 11;
    pub const DESK: i32 = 12;
    pub const CAR: i32 = 13;
    pub const FABRIC: i32 = 14;
    pub const PERSON: i32 = 15;
    pub const JACKET: i32 = 16;
    pub const HEDGE: i32 = 17;
    pub const SOIL: i32 = 18;
    pub const WHITE: i32 = 19;
    pub const RUBBER: i32 = 20;
    pub const RIM: i32 = 21;
    pub const PLASTIC: i32 = 22;
    pub const SKIN: i32 = 23;
    pub const TROUSERS: i32 = 24;
    pub const SHOE: i32 = 25;
    pub const HEADLIGHT: i32 = 26;
    pub const TAILLIGHT: i32 = 27;
    pub const CAR_SILVER: i32 = 28;
    pub const CAR_RED: i32 = 29;
    pub const HOUSING: i32 = 30;
    pub const LUMINOUS: i32 = 31;
}

/// PCG, as in the shader.
struct Rng(u32);

impl Rng {
    fn seed(a: u32, b: u32, frame: u32, salt: u32) -> Self {
        let mut r = Rng(a
            .wrapping_add(b.wrapping_mul(1973))
            .wrapping_add(frame.wrapping_mul(9277))
            .wrapping_add(salt));
        r.next();
        r.next();
        r
    }
    fn next(&mut self) -> f32 {
        let state = self.0.wrapping_mul(747796405).wrapping_add(2891336453);
        let word = ((state >> ((state >> 28).wrapping_add(4))) ^ state).wrapping_mul(277803737);
        self.0 = word;
        ((word >> 22) ^ word) as f32 * (1.0 / 4294967296.0)
    }
}

fn fract(x: f32) -> f32 {
    x - x.floor()
}

fn hash12(p: Vec2) -> f32 {
    let mut p3 = Vec3::new(
        fract(p.x * 0.1031),
        fract(p.y * 0.1031),
        fract(p.x * 0.1031),
    );
    let d = p3.dot(Vec3::new(p3.y, p3.z, p3.x) + 33.33);
    p3 += Vec3::splat(d);
    fract((p3.x + p3.y) * p3.z)
}

fn vnoise(p: Vec2) -> f32 {
    let i = p.floor();
    let f = p - i;
    let u = f * f * (Vec2::splat(3.0) - 2.0 * f);
    let a = hash12(i);
    let b = hash12(i + Vec2::new(1.0, 0.0));
    let c = hash12(i + Vec2::new(0.0, 1.0));
    let d = hash12(i + Vec2::new(1.0, 1.0));
    let x0 = a + (b - a) * u.x;
    let x1 = c + (d - c) * u.x;
    x0 + (x1 - x0) * u.y
}

fn fbm(p: Vec2) -> f32 {
    0.5 * vnoise(p) + 0.3 * vnoise(p * 2.7 + 13.1) + 0.2 * vnoise(p * 7.3 + 5.7)
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn step(edge: f32, x: f32) -> f32 {
    if x >= edge {
        1.0
    } else {
        0.0
    }
}

#[derive(Clone, Copy)]
struct Material {
    albedo: Vec3,
    rough: f32,
    metal: f32,
}

fn mk(a: Vec3, r: f32, m: f32) -> Material {
    Material {
        albedo: a,
        rough: r,
        metal: m,
    }
}

struct Lamp {
    pos: Vec3,
    glare: f32,
    rot: Mat3,
}

enum ShapeP {
    Obb { center: Vec3, half: Vec3, rot: Mat3 },
    Sphere { c: Vec3, r: f32 },
    Capsule { a: Vec3, b: Vec3, r: f32 },
    Cylinder { a: Vec3, b: Vec3, r: f32 },
}

struct Prim {
    shape: ShapeP,
    material: i32,
    flags: u32,
}

/// Nearest hit of one primitive in (EPS, t_max).
fn hit_prim(pr: &Prim, ro: Vec3, rd: Vec3, t_max: f32) -> Option<(f32, Vec3)> {
    match &pr.shape {
        ShapeP::Obb { center, half, rot } => {
            let rt = rot.transpose();
            let o = rt * (ro - *center);
            let d = rt * rd;
            let (t0, t1) = slabs(o, safe_inv(d), -*half, *half);
            if t0 <= t1 && t1 > EPS {
                let t = if t0 > EPS { t0 } else { t1 };
                if t < t_max {
                    return Some((t, *rot * box_normal(o + d * t, -*half, *half)));
                }
            }
            None
        }
        ShapeP::Sphere { c, r } => hit_sphere(*c, *r, ro, rd, t_max),
        ShapeP::Capsule { a, b, r } => {
            let ba = *b - *a;
            let oa = ro - *a;
            let baba = ba.dot(ba);
            let bard = ba.dot(rd);
            let baoa = ba.dot(oa);
            let k2 = baba - bard * bard;
            let k1 = baba * rd.dot(oa) - baoa * bard;
            let k0 = baba * oa.dot(oa) - baoa * baoa - r * r * baba;
            let hh = k1 * k1 - k2 * k0;
            if hh < 0.0 {
                return None;
            }
            let sq = hh.sqrt();
            let t = (-k1 - sq) / k2.max(1e-9);
            let y = baoa + t * bard;
            if y > 0.0 && y < baba && t > EPS && t < t_max {
                return Some((t, (oa + t * rd - ba * y / baba) / *r));
            }
            let cap = if y > 0.0 { *b } else { *a };
            hit_sphere(cap, *r, ro, rd, t_max)
        }
        ShapeP::Cylinder { a, b, r } => {
            let ba = *b - *a;
            let oc = ro - *a;
            let baba = ba.dot(ba);
            let bard = ba.dot(rd);
            let baoc = ba.dot(oc);
            let k2 = baba - bard * bard;
            let k1 = baba * oc.dot(rd) - baoc * bard;
            let k0 = baba * oc.dot(oc) - baoc * baoc - r * r * baba;
            let hh = k1 * k1 - k2 * k0;
            if hh < 0.0 {
                return None;
            }
            let sq = hh.sqrt();
            let t = (-k1 - sq) / k2.max(1e-9);
            let y = baoc + t * bard;
            if y > 0.0 && y < baba && t > EPS && t < t_max {
                return Some((t, (oc + t * rd - ba * y / baba) / *r));
            }
            if bard.abs() < 1e-9 {
                return None;
            }
            let tc = (if y > 0.0 { baba } else { 0.0 } - baoc) / bard;
            if (k1 + k2 * tc).abs() < sq && tc > EPS && tc < t_max {
                return Some((tc, ba * y.signum() / baba.sqrt()));
            }
            None
        }
    }
}

fn hit_sphere(c: Vec3, r: f32, ro: Vec3, rd: Vec3, t_max: f32) -> Option<(f32, Vec3)> {
    let oc = ro - c;
    let b = oc.dot(rd);
    let cc = oc.dot(oc) - r * r;
    let disc = b * b - cc;
    if disc < 0.0 {
        return None;
    }
    let sq = disc.sqrt();
    let mut t = -b - sq;
    if t <= EPS {
        t = -b + sq;
    }
    if t <= EPS || t >= t_max {
        return None;
    }
    Some((t, (ro + rd * t - c).normalize()))
}

/// Scene as the shader sees it.
struct Scene<'a> {
    p: &'a Params,
    prims: Vec<Prim>,
    lamps: Vec<Lamp>,
    table: &'a IesTable,
    mesh: Option<&'a LuminaireMesh>,
}

impl<'a> Scene<'a> {
    fn new(input: &'a FrameInput, table: &'a IesTable) -> Self {
        let v4 = |a: [f32; 4]| Vec3::new(a[0], a[1], a[2]);
        let prims = input
            .prims
            .iter()
            .map(|g: &GpuPrim| {
                let shape = match g.kind {
                    PRIM_SPHERE => ShapeP::Sphere {
                        c: v4(g.p0),
                        r: g.p0[3],
                    },
                    PRIM_CAPSULE => ShapeP::Capsule {
                        a: v4(g.p0),
                        b: v4(g.p1),
                        r: g.p0[3],
                    },
                    PRIM_CYLINDER => ShapeP::Cylinder {
                        a: v4(g.p0),
                        b: v4(g.p1),
                        r: g.p0[3],
                    },
                    _ => ShapeP::Obb {
                        center: v4(g.p0),
                        half: v4(g.p1),
                        rot: Mat3::from_cols(v4(g.r0), v4(g.r1), v4(g.r2)),
                    },
                };
                Prim {
                    shape,
                    material: g.material as i32,
                    flags: g.flags,
                }
            })
            .collect();
        let lamps = input
            .lamps
            .iter()
            .map(|l: &GpuLamp| Lamp {
                pos: Vec3::new(l.pos[0], l.pos[1], l.pos[2]),
                glare: l.pos[3],
                rot: Mat3::from_cols(
                    Vec3::new(l.r0[0], l.r0[1], l.r0[2]),
                    Vec3::new(l.r1[0], l.r1[1], l.r1[2]),
                    Vec3::new(l.r2[0], l.r2[1], l.r2[2]),
                ),
            })
            .collect();
        Self {
            p: &input.params,
            prims,
            lamps,
            table,
            mesh: input.mesh.as_deref(),
        }
    }

    fn ies_candela(&self, li: usize, dir: Vec3) -> f32 {
        self.table.sample_dir(&self.lamps[li].rot, dir) * self.p.lum.z
    }

    fn sample_emitter_local(&self, rng: &mut Rng) -> Vec3 {
        let u1 = rng.next();
        let u2 = rng.next();
        if self.p.housing_half.w > 0.5 {
            let r = u1.sqrt() * self.p.lum.x;
            let a = TWO_PI * u2;
            Vec3::new(r * a.cos(), 0.0, r * a.sin())
        } else {
            Vec3::new(
                (u1 * 2.0 - 1.0) * self.p.lum.x,
                0.0,
                (u2 * 2.0 - 1.0) * self.p.lum.y,
            )
        }
    }

    fn trace_scene(&self, ro: Vec3, rd: Vec3, primary: bool) -> Hit {
        let mut h = Hit {
            t: FAR,
            n: Vec3::Y,
            mat: -1,
            lamp: -1,
        };
        let ground = self.p.flags.x;
        if ground > 0 && rd.y < 0.0 && ro.y > 0.0 {
            h.t = -ro.y / rd.y;
            h.mat = match ground {
                2 => mat::ASPHALT,
                3 => mat::PARKING,
                _ => mat::PAVING,
            };
        }
        for pr in &self.prims {
            if primary && (pr.flags & 2) != 0 {
                continue;
            }
            if let Some((t, n)) = hit_prim(pr, ro, rd, h.t) {
                h.t = t;
                h.n = n;
                h.mat = pr.material;
            }
        }
        if let Some(mesh) = self.mesh {
            for (li, l) in self.lamps.iter().enumerate() {
                let rt = l.rot.transpose();
                let o = rt * (ro - l.pos);
                let d = rt * rd;
                if let Some((t, n, flags)) = mesh.hit(o, d, h.t) {
                    h.t = t;
                    h.n = l.rot * n;
                    h.mat = if flags & TRI_LUMINOUS != 0 {
                        mat::LUMINOUS
                    } else {
                        mat::HOUSING
                    };
                    h.lamp = li as i32;
                }
            }
        }
        h
    }

    fn occluded(&self, ro: Vec3, rd: Vec3, t_max: f32) -> bool {
        for pr in &self.prims {
            if (pr.flags & 1) == 0 {
                continue;
            }
            if hit_prim(pr, ro, rd, t_max).is_some() {
                return true;
            }
        }
        if let Some(mesh) = self.mesh {
            for l in &self.lamps {
                let rt = l.rot.transpose();
                if mesh
                    .hit_ex(rt * (ro - l.pos), rt * rd, t_max, true)
                    .is_some()
                {
                    return true;
                }
            }
        }
        false
    }

    fn material(&self, id: i32, p: Vec3, n: Vec3) -> (Material, Vec3) {
        let xz = Vec2::new(p.x, p.z);
        let xy = Vec2::new(p.x, p.y);
        let paint = |a: Vec3, on: bool| {
            if on {
                Vec3::new(0.62, 0.62, 0.58) * (0.8 + 0.2 * vnoise(xz * 24.0))
            } else {
                a
            }
        };
        let asphalt = |tone: Vec3| tone * (0.7 + 0.4 * fbm(xz * 2.2) + 0.3 * vnoise(xz * 75.0));
        let m = match id {
            mat::PAVING => {
                if n.y < 0.5 {
                    return (mk(Vec3::new(0.33, 0.33, 0.32), 0.9, 0.0), n);
                }
                let tile = xz / 0.4;
                let cell = tile.floor();
                let f = tile - cell;
                let joint = step(f.x.min(1.0 - f.x).min(f.y.min(1.0 - f.y)), 0.03);
                let band = step(fract(p.x / 2.4), 0.17);
                let v = 0.9 + 0.2 * hash12(cell) + 0.08 * vnoise(xz * 30.0);
                let a = Vec3::new(0.34, 0.34, 0.33).lerp(Vec3::new(0.22, 0.21, 0.2), band) * v;
                mk(a.lerp(Vec3::splat(0.14), joint), 0.85, 0.0)
            }
            mat::ASPHALT => {
                let az = p.z.abs();
                if az < self.p.f0.z {
                    let a = asphalt(Vec3::new(0.085, 0.088, 0.095));
                    let dash = az < 0.07 && fract((p.x + 2.0) / 12.0) * 12.0 < 4.0;
                    let edge = (az - (self.p.f0.z - 0.25)).abs() < 0.07;
                    mk(paint(a, dash || edge), 0.75, 0.0)
                } else {
                    mk(
                        Vec3::new(0.09, 0.075, 0.06) * (0.7 + 0.6 * fbm(xz * 1.9)),
                        1.0,
                        0.0,
                    )
                }
            }
            mat::PARKING => {
                let a = asphalt(Vec3::new(0.08, 0.082, 0.088));
                let az = p.z.abs();
                let in_row = az > 3.0 && az < 8.0;
                let bay = in_row && fract(p.x / 2.5) * 2.5 < 0.1;
                let rim = (az - 3.0).abs() < 0.05 || (az - 8.0).abs() < 0.05;
                mk(paint(a, bay || rim), 0.75, 0.0)
            }
            mat::CYCLE => mk(asphalt(Vec3::new(0.26, 0.09, 0.06)), 0.8, 0.0),
            mat::KERB => mk(Vec3::new(0.42, 0.42, 0.4), 0.85, 0.0),
            mat::RENDER => mk(
                Vec3::new(0.62, 0.6, 0.55) * (0.95 + 0.08 * fbm(xy * 2.5 + p.z)),
                0.95,
                0.0,
            ),
            mat::BRICK => {
                if n.z.abs() < 0.5 {
                    return (mk(Vec3::new(0.36, 0.3, 0.27), 0.9, 0.0), n);
                }
                let bw = 0.24;
                let bh = 0.071;
                let row = (p.y / bh).floor();
                let x = p.x / bw + 0.5 * (row - 2.0 * (row / 2.0).floor());
                let col = x.floor();
                let f = Vec2::new(fract(x) * bw, fract(p.y / bh) * bh);
                if f.x < 0.012 || f.y < 0.012 {
                    return (
                        mk(
                            Vec3::new(0.5, 0.48, 0.44) * (0.9 + 0.2 * vnoise(xy * 80.0)),
                            1.0,
                            0.0,
                        ),
                        n,
                    );
                }
                let h = hash12(Vec2::new(col, row));
                mk(
                    Vec3::new(0.38, 0.17, 0.11).lerp(Vec3::new(0.5, 0.27, 0.18), h)
                        * (0.85 + 0.3 * vnoise(xy * 40.0)),
                    0.9,
                    0.0,
                )
            }
            mat::GLASS => mk(Vec3::new(0.05, 0.06, 0.07), 0.12, 0.6),
            mat::CARPET => {
                if n.y < 0.5 {
                    return (mk(Vec3::new(0.15, 0.17, 0.2), 1.0, 0.0), n);
                }
                let tile = (xz / 0.5).floor();
                let turn = (tile.x + tile.y) - 2.0 * ((tile.x + tile.y) / 2.0).floor();
                let dir = if turn > 0.5 { p.z } else { p.x };
                let weave = 0.92 + 0.08 * (dir * 180.0).sin() * vnoise(xz * 60.0);
                mk(
                    Vec3::new(0.2, 0.24, 0.3) * weave * (0.95 + 0.1 * hash12(tile)),
                    1.0,
                    0.0,
                )
            }
            mat::WALL => mk(
                Vec3::new(0.78, 0.8, 0.78) * (0.97 + 0.04 * fbm(xy * 4.0 + p.z)),
                0.95,
                0.0,
            ),
            mat::CEILING => mk(Vec3::splat(0.85), 0.95, 0.0),
            mat::METAL => mk(Vec3::new(0.2, 0.21, 0.22), 0.4, 0.5),
            mat::DESK => mk(
                Vec3::new(0.6, 0.55, 0.47) * (0.95 + 0.1 * vnoise(xz * Vec2::new(2.0, 40.0))),
                0.35,
                0.0,
            ),
            mat::CAR => mk(Vec3::new(0.1, 0.14, 0.3), 0.28, 0.55),
            mat::FABRIC => mk(Vec3::new(0.12, 0.12, 0.13), 1.0, 0.0),
            mat::PERSON => mk(Vec3::new(0.45, 0.4, 0.36), 0.8, 0.0),
            mat::JACKET => mk(Vec3::new(0.55, 0.32, 0.12), 0.9, 0.0),
            mat::HEDGE => mk(
                Vec3::new(0.08, 0.17, 0.05) * (0.6 + 0.8 * fbm(xz * 5.0 + p.y)),
                1.0,
                0.0,
            ),
            mat::SOIL => mk(
                Vec3::new(0.1, 0.08, 0.06) * (0.7 + 0.6 * fbm(xz * 1.9)),
                1.0,
                0.0,
            ),
            mat::WHITE => mk(Vec3::new(0.8, 0.8, 0.78), 0.5, 0.0),
            mat::RUBBER => mk(Vec3::splat(0.03), 0.85, 0.0),
            mat::RIM => mk(Vec3::new(0.55, 0.56, 0.58), 0.3, 0.8),
            mat::PLASTIC => mk(Vec3::new(0.05, 0.05, 0.055), 0.6, 0.0),
            mat::SKIN => mk(Vec3::new(0.6, 0.42, 0.32), 0.65, 0.0),
            mat::TROUSERS => mk(Vec3::new(0.1, 0.12, 0.2), 0.95, 0.0),
            mat::SHOE => mk(Vec3::new(0.06, 0.05, 0.045), 0.5, 0.0),
            mat::HEADLIGHT => mk(Vec3::new(0.85, 0.87, 0.9), 0.15, 0.3),
            mat::TAILLIGHT => mk(Vec3::new(0.6, 0.05, 0.04), 0.2, 0.3),
            mat::CAR_SILVER => mk(Vec3::new(0.55, 0.56, 0.58), 0.28, 0.6),
            mat::CAR_RED => mk(Vec3::new(0.42, 0.05, 0.05), 0.28, 0.5),
            mat::HOUSING => mk(Vec3::new(0.62, 0.63, 0.64), 0.45, 0.35),
            mat::LUMINOUS => mk(Vec3::splat(0.4), 0.5, 0.0),
            _ => mk(Vec3::splat(0.6), 0.6, 0.0),
        };
        (m, n)
    }

    /// Reflected radiance and illuminance at `p` from direct light.
    fn direct_light(&self, rng: &mut Rng, p: Vec3, n: Vec3, v: Vec3, m: &Material) -> (Vec3, f32) {
        let mut e = 0.0;
        let mut sum = Vec3::ZERO;
        for (li, lamp) in self.lamps.iter().enumerate() {
            let sp = lamp.pos + lamp.rot * self.sample_emitter_local(rng);
            let to_l = sp - p;
            let d2 = to_l.length_squared();
            let d = d2.sqrt();
            let wi = to_l / d.max(1e-6);
            let ndotl = n.dot(wi);
            if ndotl <= 0.0 {
                continue;
            }
            let intensity = self.ies_candela(li, -wi);
            if intensity <= 0.0 {
                continue;
            }
            if self.occluded(p + n * EPS, wi, d - 2.0 * EPS) {
                continue;
            }
            let en = intensity / d2.max(1e-4);
            e += en * ndotl;
            sum += eval_bsdf(n, v, wi, m).0 * en;
        }
        (sum * self.p.tint.truncate(), e)
    }

    fn hit_luminaires(&self, ro: Vec3, rd: Vec3, t_max: f32) -> (f32, Option<usize>, i32, Vec3) {
        let mut best = t_max;
        let mut lamp = None;
        let mut part = 0;
        let mut nw = Vec3::Y;
        let hh = self.p.housing_half.truncate();
        let hmin = Vec3::new(-hh.x, 0.0, -hh.z);
        let hmax = Vec3::new(hh.x, hh.y * 2.0, hh.z);
        for (li, l) in self.lamps.iter().enumerate() {
            let rt = l.rot.transpose();
            let o = rt * (ro - l.pos);
            let d = rt * rd;
            let (t0, t1) = slabs(o, safe_inv(d), hmin, hmax);
            if t0 <= t1 && t0 > EPS && t0 < best {
                best = t0;
                lamp = Some(li);
                let lp = o + d * t0;
                let ln = box_normal(lp, hmin, hmax);
                let inside = if self.p.housing_half.w > 0.5 {
                    Vec2::new(lp.x, lp.z).length() <= self.p.lum.x
                } else {
                    lp.x.abs() <= self.p.lum.x && lp.z.abs() <= self.p.lum.y
                };
                part = if ln.y < -0.5 && inside { 1 } else { 2 };
                nw = l.rot * ln;
            }
        }
        (best, lamp, part, nw)
    }

    fn haze_inscatter(&self, rng: &mut Rng, ro: Vec3, rd: Vec3, t_max: f32) -> Vec3 {
        let haze = self.p.f1.z;
        let mut sum = Vec3::ZERO;
        for (li, l) in self.lamps.iter().enumerate() {
            let delta = (l.pos - ro).dot(rd);
            let dd = (ro + rd * delta - l.pos).length().max(1e-3);
            let th_a = (-delta).atan2(dd);
            let th_b = (t_max - delta).atan2(dd);
            let ang = th_a + (th_b - th_a) * rng.next();
            let t = delta + dd * ang.clamp(-HALF_PI_IN, HALF_PI_IN).tan();
            if !(t >= 0.0 && t <= t_max) {
                continue;
            }
            let pdf = dd / ((th_b - th_a) * (dd * dd + (t - delta) * (t - delta))).max(1e-6);
            let x = ro + rd * t;
            let to_l = l.pos - x;
            let d2 = to_l.length_squared();
            let d = d2.sqrt();
            let wi = to_l / d.max(1e-6);
            let intensity = self.ies_candela(li, -wi);
            if intensity <= 0.0 || self.occluded(x, wi, d - 0.05) {
                continue;
            }
            let tr = (-haze * (t + d)).exp();
            sum += Vec3::splat(
                intensity / d2.max(1e-3) * henyey_greenstein(wi.dot(rd), 0.35) * haze * tr / pdf,
            );
        }
        sum * self.p.tint.truncate()
    }

    fn sky(&self, rd: Vec3) -> Vec3 {
        if self.p.f0.w < 0.5 {
            return Vec3::ZERO;
        }
        let h = rd.y.clamp(0.0, 1.0);
        Vec3::new(0.075, 0.06, 0.045).lerp(Vec3::new(0.006, 0.008, 0.018), h.powf(0.35))
            * self.p.f1.x
    }

    /// One camera sample for pixel `(px, py)`.
    fn trace_pixel(&self, rng: &mut Rng, px: u32, py: u32, w: u32, h: u32) -> (Vec3, Vec4) {
        let ndc = Vec2::new(
            (px as f32 + rng.next()) / w as f32 * 2.0 - 1.0,
            1.0 - (py as f32 + rng.next()) / h as f32 * 2.0,
        );
        let near = self.p.inv_view_proj * Vec4::new(ndc.x, ndc.y, 0.0, 1.0);
        let far = self.p.inv_view_proj * Vec4::new(ndc.x, ndc.y, 1.0, 1.0);
        let cam_pos = self.p.cam_pos.truncate();
        let cam_rd = (far.truncate() / far.w - near.truncate() / near.w).normalize();
        let mut ro = cam_pos;
        let mut rd = cam_rd;
        let mut radiance = Vec3::ZERO;
        let mut throughput = Vec3::ONE;
        let mut irr_weight = Vec3::ZERO;
        let mut indirect_e = 0.0;
        let mut primary_e = 0.0;
        let mut coverage = 0.0;
        let mut primary_t = FAR;
        let max_bounces = self.p.flags.z;
        let clamp_v = self.p.f2.x;

        for bounce in 0..6u32 {
            if bounce >= max_bounces {
                break;
            }
            let primary = bounce == 0;
            let hit = self.trace_scene(ro, rd, primary);
            if hit.mat == mat::LUMINOUS {
                if primary && hit.lamp >= 0 {
                    let li = hit.lamp as usize;
                    let nadir = self.lamps[li].rot * Vec3::NEG_Y;
                    let c = (-rd).dot(nadir).max(0.03);
                    let area = if self.p.housing_half.w > 0.5 {
                        PI * self.p.lum.x * self.p.lum.x
                    } else {
                        4.0 * self.p.lum.x * self.p.lum.y
                    };
                    primary_t = hit.t;
                    radiance += Vec3::splat(self.ies_candela(li, -rd) / (area * c))
                        * self.p.tint.truncate();
                }
                break;
            }
            if primary && self.mesh.is_none() {
                let (tl, lamp, part, ln) = self.hit_luminaires(ro, rd, hit.t);
                if let Some(li) = lamp {
                    primary_t = tl;
                    if part == 1 {
                        let nadir = self.lamps[li].rot * Vec3::NEG_Y;
                        let c = (-rd).dot(nadir).max(0.03);
                        let area = if self.p.housing_half.w > 0.5 {
                            PI * self.p.lum.x * self.p.lum.x
                        } else {
                            4.0 * self.p.lum.x * self.p.lum.y
                        };
                        radiance += Vec3::splat(self.ies_candela(li, -rd) / (area * c))
                            * self.p.tint.truncate();
                    } else {
                        radiance += Vec3::new(0.05, 0.052, 0.056)
                            * (0.35 + 0.65 * ln.y.max(0.0))
                            * self.p.f1.y
                            * 40.0;
                    }
                    break;
                }
            }
            if hit.mat < 0 {
                if primary {
                    radiance += self.sky(rd);
                } else {
                    let fill = if self.p.f0.w > 0.5 {
                        Vec3::new(0.8, 0.88, 1.0)
                    } else {
                        Vec3::ONE
                    };
                    radiance += throughput * self.p.f1.y * fill;
                }
                break;
            }
            let p = ro + rd * hit.t;
            let mut geom_n = hit.n;
            if geom_n.dot(rd) > 0.0 {
                geom_n = -geom_n;
            }
            let (m, mut n) = self.material(hit.mat, p, geom_n);
            if n.dot(rd) > 0.0 {
                n = geom_n;
            }
            let v = -rd;
            let (ld, e) = self.direct_light(rng, p, n, v, &m);
            if primary {
                primary_e = e;
                coverage = 1.0;
                primary_t = hit.t;
                radiance += ld;
            } else {
                let c = throughput * ld;
                let mx = c.max_element();
                let k = if mx > clamp_v { clamp_v / mx } else { 1.0 };
                radiance += c * k;
                indirect_e += (irr_weight * ld * k).dot(LUMA) / self.p.tint.w;
            }
            let Some((l, weight, pdf)) = sample_bsdf(rng, n, v, &m) else {
                break;
            };
            if primary {
                irr_weight = Vec3::splat(n.dot(l).max(0.0) / pdf);
            } else {
                irr_weight *= weight;
            }
            throughput *= weight;
            if bounce >= 2 {
                let q = throughput.max_element().clamp(0.05, 0.95);
                if rng.next() > q {
                    break;
                }
                throughput /= q;
                irr_weight /= q;
            }
            ro = p + geom_n * EPS;
            rd = l;
        }
        if self.p.f1.z > 0.0 {
            let t_max = if primary_t > 1.0e6 { 500.0 } else { primary_t };
            radiance = radiance * (-self.p.f1.z * t_max).exp()
                + self.haze_inscatter(rng, cam_pos, cam_rd, t_max);
        }
        if self.p.f1.w > 0.0 {
            let mut veil = 0.0;
            for l in &self.lamps {
                let to_l = (l.pos - cam_pos).normalize();
                let ang = cam_rd.dot(to_l).clamp(-1.0, 1.0).acos().to_degrees();
                veil += l.glare * 10.0 / (ang * ang + 0.35);
            }
            radiance += Vec3::splat(veil * self.p.f1.w) * self.p.tint.truncate();
        }
        if !radiance.is_finite() {
            radiance = Vec3::ZERO;
        }
        let mut e = primary_e + indirect_e;
        if !e.is_finite() {
            e = primary_e;
        }
        (radiance, Vec4::new(e, coverage, primary_t.min(1.0e4), 1.0))
    }

    /// Illuminance at a measurement point, one sample.
    fn probe_irradiance(&self, rng: &mut Rng, p: Vec3, n: Vec3) -> f32 {
        let black = mk(Vec3::ZERO, 1.0, 0.0);
        let mut e = self.direct_light(rng, p, n, n, &black).1;
        let u1 = rng.next();
        let phi = TWO_PI * rng.next();
        let r = u1.sqrt();
        let mut rd =
            tangent_frame(n) * Vec3::new(r * phi.cos(), r * phi.sin(), (1.0 - u1).max(0.0).sqrt());
        let mut ro = p + n * EPS;
        let mut irr_weight = Vec3::splat(PI);
        let max_bounces = self.p.flags.z;
        let clamp_v = self.p.f2.x;
        for bounce in 1..6u32 {
            if bounce >= max_bounces {
                break;
            }
            let hit = self.trace_scene(ro, rd, false);
            if hit.mat < 0 {
                break;
            }
            let hp = ro + rd * hit.t;
            let mut geom_n = hit.n;
            if geom_n.dot(rd) > 0.0 {
                geom_n = -geom_n;
            }
            let (m, mut nn) = self.material(hit.mat, hp, geom_n);
            if nn.dot(rd) > 0.0 {
                nn = geom_n;
            }
            let v = -rd;
            let (ld, _) = self.direct_light(rng, hp, nn, v, &m);
            let c = irr_weight * INV_PI * ld;
            let mx = c.max_element();
            let k = if mx > clamp_v { clamp_v / mx } else { 1.0 };
            e += (irr_weight * ld * k).dot(LUMA) / self.p.tint.w;
            let Some((l, weight, _)) = sample_bsdf(rng, nn, v, &m) else {
                break;
            };
            irr_weight *= weight;
            if bounce >= 2 {
                let q = (irr_weight.max_element() * INV_PI).clamp(0.05, 0.95);
                if rng.next() > q {
                    break;
                }
                irr_weight /= q;
            }
            ro = hp + geom_n * EPS;
            rd = l;
        }
        e
    }
}

struct Hit {
    t: f32,
    n: Vec3,
    mat: i32,
    /// Lamp whose body or face was hit, else −1.
    lamp: i32,
}

fn safe_inv(d: Vec3) -> Vec3 {
    let fix = |x: f32| if x.abs() < 1e-9 { 1e-9 } else { x };
    Vec3::new(1.0 / fix(d.x), 1.0 / fix(d.y), 1.0 / fix(d.z))
}

fn slabs(ro: Vec3, inv: Vec3, bmin: Vec3, bmax: Vec3) -> (f32, f32) {
    let t0 = (bmin - ro) * inv;
    let t1 = (bmax - ro) * inv;
    let ts = t0.min(t1);
    let tb = t0.max(t1);
    (ts.max_element(), tb.min_element())
}

fn box_normal(p: Vec3, bmin: Vec3, bmax: Vec3) -> Vec3 {
    let c = 0.5 * (bmin + bmax);
    let h = (0.5 * (bmax - bmin)).max(Vec3::splat(1e-5));
    let d = (p - c) / h;
    let a = d.abs();
    if a.x >= a.y && a.x >= a.z {
        Vec3::new(d.x.signum(), 0.0, 0.0)
    } else if a.y >= a.z {
        Vec3::new(0.0, d.y.signum(), 0.0)
    } else {
        Vec3::new(0.0, 0.0, d.z.signum())
    }
}

fn tangent_frame(n: Vec3) -> Mat3 {
    let s = if n.z >= 0.0 { 1.0 } else { -1.0 };
    let a = -1.0 / (s + n.z);
    let b = n.x * n.y * a;
    let t = Vec3::new(1.0 + s * n.x * n.x * a, s * b, -s * n.x);
    let bb = Vec3::new(b, s + n.y * n.y * a, -n.y);
    Mat3::from_cols(t, bb, n)
}

fn ggx_d(ndoth: f32, a2: f32) -> f32 {
    let d = ndoth * ndoth * (a2 - 1.0) + 1.0;
    a2 / (PI * d * d).max(1e-8)
}

fn smith_g(ndotl: f32, ndotv: f32, a2: f32) -> f32 {
    let gl = ndotv * (a2 + (1.0 - a2) * ndotl * ndotl).sqrt();
    let gv = ndotl * (a2 + (1.0 - a2) * ndotv * ndotv).sqrt();
    2.0 * ndotl * ndotv / (gl + gv).max(1e-8)
}

fn fresnel(c: f32, f0: Vec3) -> Vec3 {
    f0 + (Vec3::ONE - f0) * (1.0 - c).max(0.0).powi(5)
}

fn spec_prob(m: &Material) -> f32 {
    let f0 = Vec3::splat(0.04).lerp(m.albedo, m.metal);
    (0.5 * (1.0 - (1.0 - m.metal) * (1.0 - f0.dot(Vec3::splat(0.333))))).clamp(0.1, 0.9)
}

/// Returns `(f · NdotL, pdf)`.
fn eval_bsdf(n: Vec3, v: Vec3, l: Vec3, m: &Material) -> (Vec3, f32) {
    let ndotl = n.dot(l);
    let ndotv = n.dot(v);
    if ndotl <= 0.0 || ndotv <= 0.0 {
        return (Vec3::ZERO, 0.0);
    }
    let a = m.rough.max(0.04) * m.rough.max(0.04);
    let a2 = a * a;
    let h = (l + v).normalize();
    let ndoth = n.dot(h).clamp(0.0, 1.0);
    let vdoth = v.dot(h).clamp(0.0, 1.0);
    let f0 = Vec3::splat(0.04).lerp(m.albedo, m.metal);
    let fr = fresnel(vdoth, f0);
    let d = ggx_d(ndoth, a2);
    let g = smith_g(ndotl, ndotv, a2);
    let spec = (d * g * fr) / (4.0 * ndotl * ndotv).max(1e-8);
    let diff = (Vec3::ONE - fr) * (1.0 - m.metal) * m.albedo * INV_PI;
    let ps = spec_prob(m);
    let pdf = (1.0 - ps) * ndotl * INV_PI + ps * (d * ndoth) / (4.0 * vdoth).max(1e-8);
    ((diff + spec) * ndotl, pdf)
}

fn sample_ggx_vndf(vl: Vec3, a: f32, u1: f32, u2: f32) -> Vec3 {
    let vh = Vec3::new(a * vl.x, a * vl.y, vl.z).normalize();
    let lensq = vh.x * vh.x + vh.y * vh.y;
    let t1 = if lensq > 0.0 {
        Vec3::new(-vh.y, vh.x, 0.0) / lensq.sqrt()
    } else {
        Vec3::X
    };
    let t2 = vh.cross(t1);
    let r = u1.sqrt();
    let phi = TWO_PI * u2;
    let p1 = r * phi.cos();
    let mut p2 = r * phi.sin();
    let s = 0.5 * (1.0 + vh.z);
    p2 = (1.0 - s) * (1.0 - p1 * p1).sqrt() + s * p2;
    let nh = p1 * t1 + p2 * t2 + (1.0 - p1 * p1 - p2 * p2).max(0.0).sqrt() * vh;
    Vec3::new(a * nh.x, a * nh.y, nh.z.max(0.0)).normalize()
}

/// Returns `(direction, weight, pdf)`.
fn sample_bsdf(rng: &mut Rng, n: Vec3, v: Vec3, m: &Material) -> Option<(Vec3, Vec3, f32)> {
    let a = m.rough.max(0.04) * m.rough.max(0.04);
    let frame = tangent_frame(n);
    let u0 = rng.next();
    let u1 = rng.next();
    let u2 = rng.next();
    let l = if u0 < spec_prob(m) {
        let h = frame * sample_ggx_vndf(frame.transpose() * v, a, u1, u2);
        let l = reflect(-v, h);
        if n.dot(l) <= 0.0 {
            return None;
        }
        l
    } else {
        let r = u1.sqrt();
        let phi = TWO_PI * u2;
        frame * Vec3::new(r * phi.cos(), r * phi.sin(), (1.0 - u1).max(0.0).sqrt())
    };
    let (f, pdf) = eval_bsdf(n, v, l, m);
    if pdf <= 1e-8 {
        return None;
    }
    Some((l, f / pdf, pdf))
}

fn reflect(i: Vec3, n: Vec3) -> Vec3 {
    i - 2.0 * n.dot(i) * n
}

fn henyey_greenstein(c: f32, g: f32) -> f32 {
    let d = 1.0 + g * g - 2.0 * g * c;
    (1.0 - g * g) / (4.0 * PI * d * d.sqrt())
}

fn hable(x: Vec3) -> Vec3 {
    let (a, b, c, d, e, f) = (0.15, 0.50, 0.10, 0.20, 0.02, 0.30);
    ((x * (a * x + Vec3::splat(c * b)) + Vec3::splat(d * e))
        / (x * (a * x + Vec3::splat(b)) + Vec3::splat(d * f)))
        - Vec3::splat(e / f)
}

fn filmic(x: Vec3) -> Vec3 {
    let white = hable(Vec3::splat(11.2));
    (hable(x * 2.0) / white).clamp(Vec3::ZERO, Vec3::ONE)
}

fn srgb_to_linear(c: Vec3) -> Vec3 {
    c.max(Vec3::ZERO).powf(2.2)
}

fn linear_to_srgb8(c: Vec3) -> [u8; 3] {
    let enc = |x: f32| {
        let x = x.clamp(0.0, 1.0);
        let s = if x <= 0.0031308 {
            12.92 * x
        } else {
            1.055 * x.powf(1.0 / 2.4) - 0.055
        };
        (s * 255.0 + 0.5) as u8
    };
    [enc(c.x), enc(c.y), enc(c.z)]
}

fn lux_ramp(e: f32, lux_max: f32) -> f32 {
    let x = e / lux_max;
    x.max(0.0).sqrt() * 0.15 + x * 0.85
}

/// Progressive CPU accumulation of the same picture and point values the
/// GPU produces, at whatever size the `FrameInput` asks for.
pub struct CpuTracer {
    size: UVec2,
    /// Running mean radiance and sample count per pixel.
    color: Vec<Vec4>,
    /// Running mean of (E, coverage, depth) per pixel.
    lux: Vec<Vec4>,
    cursor: usize,
    frame: u32,
    samples: u32,
    probe_out: Vec<f32>,
    probe_samples: u32,
    reset_token: u64,
    probe_reset_token: u64,
    probes_version: u64,
}

impl Default for CpuTracer {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuTracer {
    pub fn new() -> Self {
        Self {
            size: UVec2::ZERO,
            color: Vec::new(),
            lux: Vec::new(),
            cursor: 0,
            frame: 0,
            samples: 0,
            probe_out: Vec::new(),
            probe_samples: 0,
            reset_token: 0,
            probe_reset_token: 0,
            probes_version: 0,
        }
    }

    pub fn samples(&self) -> u32 {
        self.samples
    }

    pub fn probe_samples(&self) -> u32 {
        self.probe_samples
    }

    pub fn size(&self) -> UVec2 {
        self.size
    }

    /// Completed full passes over the image.
    pub fn frame_count(&self) -> u32 {
        self.frame
    }

    fn sync(&mut self, input: &FrameInput) {
        let size = UVec2::new(input.size.x.max(1), input.size.y.max(1));
        if size != self.size {
            self.size = size;
            let n = (size.x * size.y) as usize;
            self.color = vec![Vec4::ZERO; n];
            self.lux = vec![Vec4::ZERO; n];
            self.cursor = 0;
            self.samples = 0;
        }
        if input.reset_token != self.reset_token {
            self.reset_token = input.reset_token;
            self.color.fill(Vec4::ZERO);
            self.lux.fill(Vec4::ZERO);
            self.cursor = 0;
            self.samples = 0;
        }
        if input.probes_version != self.probes_version
            || input.probe_reset_token != self.probe_reset_token
        {
            self.probes_version = input.probes_version;
            self.probe_reset_token = input.probe_reset_token;
            self.probe_out = vec![0.0; input.probes.len()];
            self.probe_samples = 0;
        }
    }

    /// Trace up to `budget` pixels (one sample each), continuing where the
    /// last call stopped. Returns the number of pixels traced.
    pub fn trace_pass(&mut self, input: &FrameInput, table: &IesTable, budget: usize) -> usize {
        self.sync(input);
        if !input.trace || self.samples >= input.max_samples {
            return 0;
        }
        let scene = Scene::new(input, table);
        let (w, h) = (self.size.x, self.size.y);
        let total = (w * h) as usize;
        let mut done = 0;
        while done < budget {
            let i = self.cursor;
            let px = (i as u32) % w;
            let py = (i as u32) / w;
            let mut rng = Rng::seed(px, py, self.frame, 7);
            let (radiance, lux_sample) = scene.trace_pixel(&mut rng, px, py, w, h);
            let c = &mut self.color[i];
            let count = c.w + 1.0;
            let blend = 1.0 / count;
            let rgb = c.truncate().lerp(radiance, blend);
            *c = rgb.extend(count);
            let l = &mut self.lux[i];
            *l = l.lerp(lux_sample, blend);
            done += 1;
            self.cursor += 1;
            if self.cursor >= total {
                self.cursor = 0;
                self.frame = self.frame.wrapping_add(1);
                self.samples += 1;
                if self.samples >= input.max_samples {
                    break;
                }
            }
        }
        done
    }

    /// One probe pass: `per_point` samples for every measurement point.
    pub fn probe_pass(&mut self, input: &FrameInput, table: &IesTable, per_point: u32) {
        self.sync(input);
        if !input.probe || input.probes.is_empty() || self.probe_samples >= input.probe_max_samples
        {
            return;
        }
        let scene = Scene::new(input, table);
        let blend = per_point as f32 / (self.probe_samples as f32 + per_point as f32);
        for (k, pr) in input.probes.iter().enumerate() {
            let p = Vec3::new(pr[0], pr[1], pr[2]);
            let n = Vec3::new(pr[4], pr[5], pr[6]);
            let mut rng = Rng::seed(
                k as u32 * 7919,
                0,
                self.frame.wrapping_add(self.probe_samples),
                11,
            );
            let mut sum = 0.0;
            for _ in 0..per_point {
                sum += scene.probe_irradiance(&mut rng, p, n);
            }
            let e = sum / per_point as f32;
            let e = if e.is_finite() { e } else { 0.0 };
            self.probe_out[k] = self.probe_out[k] + (e - self.probe_out[k]) * blend;
        }
        self.probe_samples += per_point;
    }

    pub fn probe_values(&self) -> &[f32] {
        &self.probe_out
    }

    /// The presented picture as sRGB RGBA8, row-major from the top.
    pub fn present(&self, params: &Params) -> Vec<u8> {
        let (w, h) = (self.size.x as i32, self.size.y as i32);
        let mut out = vec![0u8; (w * h * 4) as usize];
        if self.color.is_empty() {
            return out;
        }
        let lux_at = |x: i32, y: i32| -> Vec4 {
            let cx = x.clamp(0, w - 1);
            let cy = y.clamp(0, h - 1);
            self.lux[(cy * w + cx) as usize]
        };
        let smooth_e = |px: i32, py: i32| -> f32 {
            let l0 = lux_at(px, py);
            let e0 = l0.x / l0.y.max(1e-4);
            let mut sum_e = 0.0;
            let mut sum_w = 0.0;
            for dy in -2..=2 {
                for dx in -2..=2 {
                    let l = lux_at(px + dx, py + dy);
                    if l.y < 0.5 {
                        continue;
                    }
                    let en = l.x / l.y;
                    let r = (en - e0) / (0.2 * e0).max(1e-3);
                    let wgt = (-r * r).exp();
                    sum_e += en * wgt;
                    sum_w += wgt;
                }
            }
            if sum_w > 0.0 {
                sum_e / sum_w
            } else {
                e0
            }
        };
        let mode = params.flags.w;
        let isolines = params.frame.w > 0;
        for py in 0..h {
            for px in 0..w {
                let i = (py * w + px) as usize;
                let radiance = self.color[i].truncate();
                let lux = self.lux[i];
                let mut display = filmic(radiance * params.f2.y);
                let e = smooth_e(px, py);
                if mode == 1 {
                    let shade = display.dot(LUMA);
                    display = if lux.y < 0.02 {
                        srgb_to_linear(Vec3::new(0.03, 0.03, 0.035)) + shade * 0.2
                    } else {
                        let heat =
                            srgb_to_linear(crate::measure::heat_ramp(lux_ramp(e, params.f2.z)));
                        heat * (0.45 + 0.75 * (shade * 3.0).clamp(0.3, 1.0))
                    };
                } else if mode == 2 {
                    display *= 0.42;
                }
                if isolines && lux.y > 0.5 {
                    let iso = params.f2.w;
                    let f = e / iso;
                    let fx = (smooth_e(px + 1, py) - e).abs() / iso;
                    let fy = (smooth_e(px, py + 1) - e).abs() / iso;
                    let wd = (fx + fy).max(1e-4);
                    let d = (fract(f + 0.5) - 0.5).abs();
                    let line =
                        (1.0 - smoothstep(0.5 * wd, 1.5 * wd, d)) * step(wd, 0.45) * step(0.5, f);
                    let line_color = if mode == 1 {
                        Vec3::splat(0.92)
                    } else {
                        srgb_to_linear(Vec3::new(0.376, 0.647, 0.98))
                    };
                    display = display.lerp(line_color, line * 0.8);
                }
                let [r, g, b] = linear_to_srgb8(display);
                let o = i * 4;
                out[o] = r;
                out[o + 1] = g;
                out[o + 2] = b;
                out[o + 3] = 255;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::viewer::Viewer;

    #[test]
    fn cpu_grid_values_are_physically_plausible() {
        let mut v = Viewer::default();
        v.load_sample(3); // batwing in the office
        v.edit(|s| s.mode = crate::viewer::ViewMode::Points);
        v.resize(64.0, 40.0, 1.0);
        v.update(0.016);
        let input = v.frame_input();
        let table = &v.photometry.as_ref().unwrap().table;
        let mut cpu = CpuTracer::new();
        cpu.probe_pass(&input, table, 8);
        v.probe_values = cpu.probe_values().to_vec();
        let stats = v.grid_statistics();
        let work = stats
            .iter()
            .find(|g| g.name.starts_with("workplane"))
            .unwrap();
        // 4000 lm batwing in a 6 × 4.8 m office: a few hundred lux on the desks.
        assert!(work.em > 150.0 && work.em < 900.0, "Em {}", work.em);
        assert!(work.uo > 0.2 && work.uo <= 1.0, "Uo {}", work.uo);
        // And a few camera samples produce a picture.
        let n = cpu.trace_pass(&input, table, 64 * 40);
        assert_eq!(n, 64 * 40);
        let img = cpu.present(&input.params);
        assert_eq!(img.len(), 64 * 40 * 4);
        assert!(img.iter().step_by(4).any(|&r| r > 0));
    }
}
