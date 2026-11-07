//! Measurement points on the calculation grids, their GPU layout and the
//! screen-space thinning used by the overlay.

use super::scene::BuiltScene;
use glam::Vec3;

/// Height of the measurement points above their surface (m).
const LIFT: f32 = 0.02;

#[derive(Clone, Debug)]
pub struct MeasurePoint {
    pub position: Vec3,
    pub normal: Vec3,
    pub grid: usize,
    pub i: usize,
    pub j: usize,
}

#[derive(Clone, Debug, Default)]
pub struct MeasureLayout {
    pub points: Vec<MeasurePoint>,
    /// Per grid, point index at slot `i * nv + j`, or −1 where the point sits inside an object.
    pub slots: Vec<Vec<i32>>,
    /// `nv` per grid, so a slot index can be decoded.
    pub nv: Vec<usize>,
}

pub fn measurement_layout(scene: &BuiltScene) -> MeasureLayout {
    let mut points = Vec::new();
    let mut slots = Vec::new();
    let mut nvs = Vec::new();
    for (gi, g) in scene.grids.iter().enumerate() {
        let mut slot = vec![-1i32; g.nu * g.nv];
        for i in 0..g.nu {
            for j in 0..g.nv {
                let s = (i as f32 + 0.5) / g.nu as f32;
                let t = (j as f32 + 0.5) / g.nv as f32;
                let r = &g.receiver;
                let p = r.origin + r.u * s + r.v * t + r.normal * LIFT;
                if let Some(outline) = g.outline.as_ref() {
                    let q = glam::Vec2::new(p.x, p.z);
                    if !crate::scene::outline_contains(outline, q)
                        || (g.border > 0.0
                            && crate::scene::distance_to_outline(outline, q) < g.border)
                    {
                        continue;
                    }
                }
                if scene.prims.iter().any(|b| b.contains(p)) {
                    continue;
                }
                slot[i * g.nv + j] = points.len() as i32;
                points.push(MeasurePoint {
                    position: p,
                    normal: r.normal,
                    grid: gi,
                    i,
                    j,
                });
            }
        }
        slots.push(slot);
        nvs.push(g.nv);
    }
    MeasureLayout {
        points,
        slots,
        nv: nvs,
    }
}

/// Position + normal pairs as the probe pass reads them.
pub fn probe_buffer_data(layout: &MeasureLayout) -> Vec<[f32; 8]> {
    layout
        .points
        .iter()
        .map(|p| {
            [
                p.position.x,
                p.position.y,
                p.position.z,
                1.0,
                p.normal.x,
                p.normal.y,
                p.normal.z,
                0.0,
            ]
        })
        .collect()
}

pub fn format_lux(e: f32) -> String {
    if e.is_nan() || e <= 0.005 {
        "0".to_string()
    } else if e >= 10.0 {
        format!("{}", e.round() as i64)
    } else if e >= 1.0 {
        format!("{e:.1}")
    } else {
        format!("{e:.2}")
    }
}

/// Level at which the neighbours of point `k` along either grid axis stop
/// overlapping a box of `w × h` screen pixels. `sx`/`sy` are screen positions
/// (NaN when not projected).
pub fn lod_level(layout: &MeasureLayout, k: usize, sx: &[f32], sy: &[f32], w: f32, h: f32) -> u32 {
    let p = &layout.points[k];
    let slots = &layout.slots[p.grid];
    let nv = layout.nv[p.grid];
    let nu = slots.len() / nv.max(1);
    let mut level = 0u32;
    for (di, dj) in [(1i64, 0i64), (0, 1), (-1, 0), (0, -1)] {
        let ni = p.i as i64 + di;
        let nj = p.j as i64 + dj;
        if ni < 0 || nj < 0 || ni as usize >= nu || nj as usize >= nv {
            continue;
        }
        let q = slots[ni as usize * nv + nj as usize];
        if q < 0 {
            continue;
        }
        let q = q as usize;
        if sx[q].is_nan() {
            continue;
        }
        let f = ((sx[q] - sx[k]).abs() / w).max((sy[q] - sy[k]).abs() / h);
        let l = if f >= 1.0 {
            0
        } else {
            (1.0 / f.max(1e-6)).log2().ceil() as u32
        };
        level = level.max(l);
    }
    level
}

pub const MAX_LOD: u32 = 5;

pub fn keep_at_level(p: &MeasurePoint, level: u32) -> bool {
    if level > MAX_LOD {
        return false;
    }
    let step = 1usize << level;
    p.i.is_multiple_of(step) && p.j.is_multiple_of(step)
}

/// Sequential illuminance ramp, viridis-like (dark violet → teal → yellow),
/// as sRGB. Perceptually ordered, so brighter always reads brighter.
/// Shared by the isolux view, the legends and the point pills.
pub fn heat_ramp(t: f32) -> Vec3 {
    let t = t.clamp(0.0, 1.0);
    let c0 = Vec3::new(0.277_73, 0.005_41, 0.334_1);
    let c1 = Vec3::new(0.105_09, 1.404_61, 1.384_59);
    let c2 = Vec3::new(-0.330_86, 0.214_85, 0.095_1);
    let c3 = Vec3::new(-4.634_23, -5.799_1, -19.332_44);
    let c4 = Vec3::new(6.228_27, 14.179_93, 56.690_55);
    let c5 = Vec3::new(4.776_38, -13.745_15, -65.353_03);
    let c6 = Vec3::new(-5.435_46, 4.645_85, 26.312_44);
    (c0 + t * (c1 + t * (c2 + t * (c3 + t * (c4 + t * (c5 + t * c6))))))
        .clamp(Vec3::ZERO, Vec3::ONE)
}

/// Same perceptual stretch as the shader: a little square-root lift so low
/// values are not all black.
pub fn lux_ramp_t(e: f32, lux_max: f32) -> f32 {
    let x = (e / lux_max.max(1e-6)).max(0.0);
    x.sqrt() * 0.15 + x * 0.85
}
