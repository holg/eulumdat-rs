//! Triangle mesh of the luminaire body, traced per lamp instance.
//!
//! A GLDF container carries the manufacturer's L3D geometry; this module
//! turns it into a flat triangle list with a BVH in the luminaire's local
//! frame (Y up, emission towards −Y, photometric centre at the origin) so
//! the WGSL and CPU tracers show and shadow the real body instead of the
//! grey housing box. Everything here is dependency-free; the L3D archive
//! reader lives in [`crate::l3d`] behind the `l3d` feature.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

/// Upper bound on triangles uploaded to the GPU (1.5 MB of triangles).
pub const MAX_MESH_TRIS: usize = 32_768;
/// Leaf size of the BVH.
const LEAF_TRIS: usize = 4;

/// Triangle flag: part of the light-emitting face.
pub const TRI_LUMINOUS: u32 = 1;

/// One triangle as the shaders read it: `v0.w` carries the flags as bits.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct GpuTri {
    pub v0: [f32; 4],
    pub e1: [f32; 4],
    pub e2: [f32; 4],
}

/// BVH node as the shaders read it: `bmin.w` = first child (inner) or first
/// triangle (leaf) as bits, `bmax.w` = triangle count (0 for inner nodes).
/// Children of an inner node are consecutive.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct GpuNode {
    pub bmin: [f32; 4],
    pub bmax: [f32; 4],
}

/// Luminaire body ready for upload.
#[derive(Clone, Debug, Default)]
pub struct LuminaireMesh {
    pub tris: Vec<GpuTri>,
    pub nodes: Vec<GpuNode>,
    /// Bounds in the luminaire frame.
    pub bmin: Vec3,
    pub bmax: Vec3,
    /// Half extents of the emitting face (x, z), when it is known.
    pub face_half: Option<(f32, f32)>,
    /// Round emitting face.
    pub face_round: bool,
}

/// A triangle in world units with its flags.
#[derive(Clone, Copy, Debug)]
pub struct Tri {
    pub v: [Vec3; 3],
    pub flags: u32,
}

/// One OBJ part of an L3D model with its placement (column-major 4×4,
/// includes the unit scale to metres), in the L3D frame (Z up).
#[derive(Clone, Debug)]
pub struct MeshPart {
    pub obj: String,
    pub mat: [f32; 16],
}

/// Light-emitting object in the L3D frame (metres).
#[derive(Clone, Copy, Debug)]
pub struct EmittingFace {
    pub centre: Vec3,
    /// Half sizes (x, y) of a rectangle, or (radius, radius) of a circle.
    pub half: (f32, f32),
    pub round: bool,
}

/// Geometry handed over by the L3D reader (or any other source).
#[derive(Clone, Debug, Default)]
pub struct MeshSource {
    pub parts: Vec<MeshPart>,
    pub faces: Vec<EmittingFace>,
}

/// Minimal Wavefront OBJ reader: positions and faces (fan-triangulated),
/// everything else ignored.
pub fn parse_obj(text: &str) -> Vec<[Vec3; 3]> {
    let mut pos: Vec<Vec3> = Vec::new();
    let mut tris = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        let mut it = line.split_whitespace();
        match it.next() {
            Some("v") => {
                let mut c = [0.0f32; 3];
                for (i, s) in it.take(3).enumerate() {
                    c[i] = s.parse().unwrap_or(0.0);
                }
                pos.push(Vec3::from(c));
            }
            Some("f") => {
                let idx: Vec<usize> = it
                    .filter_map(|s| s.split('/').next())
                    .filter_map(|s| s.parse::<i64>().ok())
                    .filter_map(|i| {
                        let n = pos.len() as i64;
                        let k = if i < 0 { n + i } else { i - 1 };
                        (k >= 0 && k < n).then_some(k as usize)
                    })
                    .collect();
                for i in 1..idx.len().saturating_sub(1) {
                    tris.push([pos[idx[0]], pos[idx[i]], pos[idx[i + 1]]]);
                }
            }
            _ => {}
        }
    }
    tris
}

/// L3D (Z up, mount plate towards +Z) → scene luminaire frame (Y up).
fn l3d_to_scene(p: Vec3) -> Vec3 {
    Vec3::new(p.x, p.z, -p.y)
}

impl LuminaireMesh {
    /// Build from an L3D-style source. The photometric centre becomes the
    /// first emitting face (else the lowest point of the body); emission is
    /// along −Y. `None` when there is no usable triangle.
    pub fn from_source(src: &MeshSource) -> Option<Self> {
        let mut tris: Vec<Tri> = Vec::new();
        for part in &src.parts {
            let m = Mat4::from_cols_array(&part.mat);
            for t in parse_obj(&part.obj) {
                let v = [
                    l3d_to_scene(m.transform_point3(t[0])),
                    l3d_to_scene(m.transform_point3(t[1])),
                    l3d_to_scene(m.transform_point3(t[2])),
                ];
                tris.push(Tri { v, flags: 0 });
            }
        }
        if tris.is_empty() {
            return None;
        }
        let (mut bmin, mut bmax) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for t in &tris {
            for v in t.v {
                bmin = bmin.min(v);
                bmax = bmax.max(v);
            }
        }
        let face = src.faces.first().copied();
        let mut origin = match face {
            Some(f) => l3d_to_scene(f.centre),
            None => Vec3::new(0.5 * (bmin.x + bmax.x), bmin.y, 0.5 * (bmin.z + bmax.z)),
        };
        // Many L3D files put the emitting object on the mounting plane with
        // the body hanging below it. Light leaving there would be stopped by
        // the body itself, so a thin body below the face moves the emitting
        // plane to its underside. A long reach below (a free-standing
        // luminaire's stand, a pendant's rod) keeps the face where it is.
        if origin.y > bmin.y + 0.002 && origin.y - bmin.y < 0.15 {
            origin.y = bmin.y;
        }
        for t in &mut tris {
            for v in &mut t.v {
                *v -= origin;
            }
        }
        // The emitting face itself, so primary rays see the luminous area
        // at the photometric centre (just below the body to avoid z-fighting).
        let mut face_half = None;
        let mut face_round = false;
        if let Some(f) = face {
            let y = -0.002;
            face_round = f.round;
            face_half = Some(f.half);
            let (hx, hz) = f.half;
            if f.round {
                let n = 24;
                for i in 0..n {
                    let a0 = i as f32 / n as f32 * std::f32::consts::TAU;
                    let a1 = (i + 1) as f32 / n as f32 * std::f32::consts::TAU;
                    tris.push(Tri {
                        v: [
                            Vec3::new(0.0, y, 0.0),
                            Vec3::new(hx * a1.cos(), y, hx * a1.sin()),
                            Vec3::new(hx * a0.cos(), y, hx * a0.sin()),
                        ],
                        flags: TRI_LUMINOUS,
                    });
                }
            } else {
                let c = [
                    Vec3::new(-hx, y, -hz),
                    Vec3::new(hx, y, -hz),
                    Vec3::new(hx, y, hz),
                    Vec3::new(-hx, y, hz),
                ];
                tris.push(Tri {
                    v: [c[0], c[2], c[1]],
                    flags: TRI_LUMINOUS,
                });
                tris.push(Tri {
                    v: [c[0], c[3], c[2]],
                    flags: TRI_LUMINOUS,
                });
            }
        }
        if tris.len() > MAX_MESH_TRIS {
            tris.truncate(MAX_MESH_TRIS);
        }
        let mut mesh = Self::from_tris(&tris);
        mesh.face_half = face_half;
        mesh.face_round = face_round;
        Some(mesh)
    }

    /// Build the BVH over triangles already in the luminaire frame.
    pub fn from_tris(tris: &[Tri]) -> Self {
        let mut b = Builder {
            tris,
            centroids: tris
                .iter()
                .map(|t| (t.v[0] + t.v[1] + t.v[2]) / 3.0)
                .collect(),
            order: (0..tris.len()).collect(),
            nodes: Vec::new(),
            out: Vec::with_capacity(tris.len()),
        };
        if !tris.is_empty() {
            b.nodes.push(GpuNode::default());
            b.build(0, tris.len(), 0);
        }
        let (mut bmin, mut bmax) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for t in tris {
            for v in t.v {
                bmin = bmin.min(v);
                bmax = bmax.max(v);
            }
        }
        if tris.is_empty() {
            bmin = Vec3::ZERO;
            bmax = Vec3::ZERO;
        }
        Self {
            tris: b.out,
            nodes: b.nodes,
            bmin,
            bmax,
            face_half: None,
            face_round: false,
        }
    }

    /// Packed nodes-then-triangles buffer for the shaders (`vec4<f32>` rows).
    pub fn packed(&self) -> Vec<[f32; 4]> {
        let mut out = Vec::with_capacity(self.nodes.len() * 2 + self.tris.len() * 3);
        for n in &self.nodes {
            out.push(n.bmin);
            out.push(n.bmax);
        }
        for t in &self.tris {
            out.push(t.v0);
            out.push(t.e1);
            out.push(t.e2);
        }
        out
    }

    /// Closest hit in the luminaire frame: (t, normal, flags).
    pub fn hit(&self, ro: Vec3, rd: Vec3, t_max: f32) -> Option<(f32, Vec3, u32)> {
        self.hit_ex(ro, rd, t_max, false)
    }

    /// As [`hit`](Self::hit); `shadow` skips the luminous face so shadow
    /// rays towards the lamp are not blocked by it.
    pub fn hit_ex(&self, ro: Vec3, rd: Vec3, t_max: f32, shadow: bool) -> Option<(f32, Vec3, u32)> {
        if self.nodes.is_empty() {
            return None;
        }
        let inv = Vec3::new(1.0 / nz(rd.x), 1.0 / nz(rd.y), 1.0 / nz(rd.z));
        let mut best = t_max;
        let mut out = None;
        let mut stack = [0u32; 48];
        let mut sp = 1usize;
        while sp > 0 {
            sp -= 1;
            let n = &self.nodes[stack[sp] as usize];
            let bmin = Vec3::new(n.bmin[0], n.bmin[1], n.bmin[2]);
            let bmax = Vec3::new(n.bmax[0], n.bmax[1], n.bmax[2]);
            let t0 = (bmin - ro) * inv;
            let t1 = (bmax - ro) * inv;
            let tmin = t0.min(t1).max_element().max(0.0);
            let tmax = t0.max(t1).min_element();
            if tmin > tmax || tmin > best {
                continue;
            }
            let first = n.bmin[3].to_bits();
            let count = n.bmax[3].to_bits();
            if count == 0 {
                if sp + 2 <= stack.len() {
                    stack[sp] = first;
                    stack[sp + 1] = first + 1;
                    sp += 2;
                }
            } else {
                for i in first..first + count {
                    let t = &self.tris[i as usize];
                    if shadow && (t.v0[3].to_bits() & TRI_LUMINOUS) != 0 {
                        continue;
                    }
                    let v0 = Vec3::new(t.v0[0], t.v0[1], t.v0[2]);
                    let e1 = Vec3::new(t.e1[0], t.e1[1], t.e1[2]);
                    let e2 = Vec3::new(t.e2[0], t.e2[1], t.e2[2]);
                    if let Some(th) = hit_tri(ro, rd, v0, e1, e2, best) {
                        best = th;
                        out = Some((th, e1.cross(e2).normalize_or_zero(), t.v0[3].to_bits()));
                    }
                }
            }
        }
        out
    }
}

fn nz(x: f32) -> f32 {
    if x.abs() < 1e-9 {
        1e-9
    } else {
        x
    }
}

/// Möller–Trumbore, two-sided.
pub fn hit_tri(ro: Vec3, rd: Vec3, v0: Vec3, e1: Vec3, e2: Vec3, t_max: f32) -> Option<f32> {
    let p = rd.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-9 {
        return None;
    }
    let inv = 1.0 / det;
    let s = ro - v0;
    let u = s.dot(p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = rd.dot(q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = e2.dot(q) * inv;
    (t > 1e-4 && t < t_max).then_some(t)
}

struct Builder<'a> {
    tris: &'a [Tri],
    centroids: Vec<Vec3>,
    order: Vec<usize>,
    nodes: Vec<GpuNode>,
    out: Vec<GpuTri>,
}

impl Builder<'_> {
    fn build(&mut self, start: usize, end: usize, idx: usize) {
        let (mut bmin, mut bmax) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for &i in &self.order[start..end] {
            for v in self.tris[i].v {
                bmin = bmin.min(v);
                bmax = bmax.max(v);
            }
        }
        let count = end - start;
        if count <= LEAF_TRIS {
            let first = self.out.len() as u32;
            for &i in &self.order[start..end] {
                let t = &self.tris[i];
                self.out.push(GpuTri {
                    v0: [t.v[0].x, t.v[0].y, t.v[0].z, f32::from_bits(t.flags)],
                    e1: (t.v[1] - t.v[0]).extend(0.0).to_array(),
                    e2: (t.v[2] - t.v[0]).extend(0.0).to_array(),
                });
            }
            self.nodes[idx] = GpuNode {
                bmin: [bmin.x, bmin.y, bmin.z, f32::from_bits(first)],
                bmax: [bmax.x, bmax.y, bmax.z, f32::from_bits(count as u32)],
            };
            return;
        }
        let ext = bmax - bmin;
        let axis = if ext.x >= ext.y && ext.x >= ext.z {
            0
        } else if ext.y >= ext.z {
            1
        } else {
            2
        };
        let mid = start + count / 2;
        let centroids = &self.centroids;
        self.order[start..end].select_nth_unstable_by(count / 2, |&a, &b| {
            centroids[a][axis]
                .partial_cmp(&centroids[b][axis])
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        // Both children are allocated before recursing so they stay consecutive.
        let left = self.nodes.len();
        self.nodes.push(GpuNode::default());
        self.nodes.push(GpuNode::default());
        self.nodes[idx] = GpuNode {
            bmin: [bmin.x, bmin.y, bmin.z, f32::from_bits(left as u32)],
            bmax: [bmax.x, bmax.y, bmax.z, f32::from_bits(0)],
        };
        self.build(start, mid, left);
        self.build(mid, end, left + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obj_box_hits_from_outside() {
        let obj =
            "v -1 -1 -1\nv 1 -1 -1\nv 1 1 -1\nv -1 1 -1\nv -1 -1 1\nv 1 -1 1\nv 1 1 1\nv -1 1 1\n\
                   f 1 2 3 4\nf 5 8 7 6\nf 1 5 6 2\nf 2 6 7 3\nf 3 7 8 4\nf 5 1 4 8\n";
        let tris = parse_obj(obj);
        assert_eq!(tris.len(), 12);
        let src = MeshSource {
            parts: vec![MeshPart {
                obj: obj.to_string(),
                mat: Mat4::IDENTITY.to_cols_array(),
            }],
            faces: vec![],
        };
        let mesh = LuminaireMesh::from_source(&src).unwrap();
        // Origin moved to the bottom centre: body spans y in [0, 2].
        assert!((mesh.bmin.y).abs() < 1e-5 && (mesh.bmax.y - 2.0).abs() < 1e-5);
        let hit = mesh.hit(Vec3::new(0.0, 1.0, -5.0), Vec3::Z, 100.0).unwrap();
        assert!((hit.0 - 4.0).abs() < 1e-4, "t = {}", hit.0);
        assert!(mesh
            .hit(Vec3::new(5.0, 1.0, -5.0), Vec3::Z, 100.0)
            .is_none());
    }

    #[test]
    fn bvh_children_are_consecutive() {
        let mut tris = Vec::new();
        for i in 0..100 {
            let x = i as f32 * 0.1;
            tris.push(Tri {
                v: [
                    Vec3::new(x, 0.0, 0.0),
                    Vec3::new(x + 0.05, 0.0, 0.0),
                    Vec3::new(x, 0.05, 0.0),
                ],
                flags: 0,
            });
        }
        let mesh = LuminaireMesh::from_tris(&tris);
        assert_eq!(mesh.tris.len(), 100);
        for n in &mesh.nodes {
            if n.bmax[3].to_bits() == 0 {
                let l = n.bmin[3].to_bits() as usize;
                assert!(l + 1 < mesh.nodes.len());
            }
        }
        let hit = mesh
            .hit(Vec3::new(5.02, 0.01, 1.0), Vec3::NEG_Z, 10.0)
            .unwrap();
        assert!((hit.0 - 1.0).abs() < 1e-4);
    }
}
