// Progressive path tracer for the photometric scene viewer.
//
// Three compute entry points share one bind group:
//   trace   – one camera sample per pixel, blended into the running means
//             (radiance, and illuminance at the primary hit)
//   probe   – illuminance at the measurement points
//   present – tonemapped render / false-colour isolux / dimmed points view
//
// Ported from the MIT-licensed Taman photometric-viewer (GLSL, WebGL2):
// PCG RNG, GGX + Lambert BSDF, next-event estimation with E = I / r² and the
// IES lookup in the luminaire frame (γ = acos(−y), C = atan2(z, x)).
// World is Y-up. Light is sampled from the IES/LDT table as an R32Float
// texture: column = C (1°), row = γ (0.5°), absolute candela.

struct Params {
    inv_view_proj: mat4x4<f32>,
    cam_pos: vec4<f32>,      // xyz
    tint: vec4<f32>,         // rgb linear tint, w = luma of the tint
    housing_half: vec4<f32>, // xyz housing half extents, w = round (1) / rect (0)
    lum: vec4<f32>,          // half_x, half_z of the luminous face, ies scale (dimming), unused
    counts: vec4<u32>,       // ies c size, ies g size, lamp count, box count
    flags: vec4<u32>,        // ground (0 none, 1 concrete, 2 street), grid lines, max bounces, mode
    frame: vec4<u32>,        // frame index, probe count, probe samples per pass, isolines
    f0: vec4<f32>,           // blend, probe blend, road half width, outdoor
    f1: vec4<f32>,           // sky scale, ambient, haze, glare
    f2: vec4<f32>,           // firefly clamp, exposure, lux max, iso step
    size: vec4<u32>,         // width, height, unused, unused
}

// kind 0 oriented box (p0 centre, p1 half extents, r0..r2 rotation columns),
// 1 sphere (p0.xyz centre, p0.w radius), 2 capsule, 3 capped cylinder
// (p0.xyz a, p0.w radius, p1.xyz b). flags: bit 0 casts shadow, bit 1 hidden
// from primary rays (cutaway).
struct Prim {
    kind: u32,
    material: u32,
    flags: u32,
    pad: u32,
    p0: vec4<f32>,
    p1: vec4<f32>,
    r0: vec4<f32>,
    r1: vec4<f32>,
    r2: vec4<f32>,
}

struct Lamp {
    pos: vec4<f32>, // xyz, w = illuminance at the eye (glare)
    r0: vec4<f32>,  // rotation columns, local → world
    r1: vec4<f32>,
    r2: vec4<f32>,
}

@group(0) @binding(0) var<uniform> P: Params;
@group(0) @binding(1) var<storage, read> prims: array<Prim>;
@group(0) @binding(2) var<storage, read> lamps: array<Lamp>;
@group(0) @binding(3) var ies_tex: texture_2d<f32>;
@group(0) @binding(4) var<storage, read_write> acc_color: array<vec4<f32>>;
@group(0) @binding(5) var<storage, read_write> acc_lux: array<vec4<f32>>;
@group(0) @binding(6) var output: texture_storage_2d<rgba16float, write>;
@group(0) @binding(7) var<storage, read> probe_pos: array<vec4<f32>>;
@group(0) @binding(8) var<storage, read_write> probe_out: array<vec4<f32>>;
// Luminaire body: P.size.z BVH nodes (2 rows each: bmin + first, bmax +
// count as bits) followed by P.size.w triangles (3 rows: v0 + flags, e1, e2).
@group(0) @binding(9) var<storage, read> mesh: array<vec4<f32>>;

const PI: f32 = 3.14159265358979;
const TWO_PI: f32 = 6.28318530717959;
const INV_PI: f32 = 0.31830988618379;
const EPS: f32 = 1.0e-3;
const FAR: f32 = 1.0e20;

const MAT_PAVING: i32 = 0;
const MAT_ASPHALT: i32 = 1;
const MAT_PARKING: i32 = 2;
const MAT_CYCLE: i32 = 3;
const MAT_KERB: i32 = 4;
const MAT_RENDER: i32 = 5;
const MAT_BRICK: i32 = 6;
const MAT_GLASS: i32 = 7;
const MAT_CARPET: i32 = 8;
const MAT_WALL: i32 = 9;
const MAT_CEILING: i32 = 10;
const MAT_METAL: i32 = 11;
const MAT_DESK: i32 = 12;
const MAT_CAR: i32 = 13;
const MAT_FABRIC: i32 = 14;
const MAT_PERSON: i32 = 15;
const MAT_JACKET: i32 = 16;
const MAT_HEDGE: i32 = 17;
const MAT_SOIL: i32 = 18;
const MAT_WHITE: i32 = 19;
const MAT_RUBBER: i32 = 20;
const MAT_RIM: i32 = 21;
const MAT_PLASTIC: i32 = 22;
const MAT_SKIN: i32 = 23;
const MAT_TROUSERS: i32 = 24;
const MAT_SHOE: i32 = 25;
const MAT_HEADLIGHT: i32 = 26;
const MAT_TAILLIGHT: i32 = 27;
const MAT_HOUSING: i32 = 30;
const MAT_LUMINOUS: i32 = 31;
const MAT_CAR_SILVER: i32 = 28;
const MAT_CAR_RED: i32 = 29;

const LUMA: vec3<f32> = vec3<f32>(0.2126, 0.7152, 0.0722);

// ---------- RNG (PCG) ----------
var<private> g_seed: u32;

fn rnd() -> f32 {
    let state = g_seed * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    g_seed = word;
    return f32((word >> 22u) ^ word) * (1.0 / 4294967296.0);
}

fn hash12(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash12(i), hash12(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash12(i + vec2<f32>(0.0, 1.0)), hash12(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y,
    );
}

fn fbm(p: vec2<f32>) -> f32 {
    return 0.5 * vnoise(p) + 0.3 * vnoise(p * 2.7 + 13.1) + 0.2 * vnoise(p * 7.3 + 5.7);
}

// ---------- IES ----------
fn lamp_rot(li: u32) -> mat3x3<f32> {
    let l = lamps[li];
    return mat3x3<f32>(l.r0.xyz, l.r1.xyz, l.r2.xyz);
}

fn ies_load(ix: i32, iy: i32) -> f32 {
    return textureLoad(ies_tex, vec2<i32>(ix, iy), 0).r;
}

// Candela from lamp `li` towards world direction `dir` (unit, away from the lamp).
fn ies_candela(li: u32, dir: vec3<f32>) -> f32 {
    let l = transpose(lamp_rot(li)) * dir;
    let g = acos(clamp(-l.y, -1.0, 1.0));
    var c = atan2(l.z, l.x);
    if (c < 0.0) { c += TWO_PI; }
    let cs = i32(P.counts.x);
    let gs = i32(P.counts.y);
    let x = c / TWO_PI * f32(cs);
    let y = g / PI * f32(gs - 1);
    let xf = floor(x);
    let yf = floor(y);
    let tx = x - xf;
    let ty = clamp(y - yf, 0.0, 1.0);
    let x0 = ((i32(xf) % cs) + cs) % cs;
    let x1 = (x0 + 1) % cs;
    let y0 = clamp(i32(yf), 0, gs - 1);
    let y1 = min(y0 + 1, gs - 1);
    let a = mix(ies_load(x0, y0), ies_load(x1, y0), tx);
    let b = mix(ies_load(x0, y1), ies_load(x1, y1), tx);
    return mix(a, b, ty) * P.lum.z;
}

// Point on the luminous face in the luminaire frame (face in the x–z plane, y = 0).
fn sample_emitter_local() -> vec3<f32> {
    let u1 = rnd();
    let u2 = rnd();
    if (P.housing_half.w > 0.5) {
        let r = sqrt(u1) * P.lum.x;
        let a = TWO_PI * u2;
        return vec3<f32>(r * cos(a), 0.0, r * sin(a));
    }
    return vec3<f32>((u1 * 2.0 - 1.0) * P.lum.x, 0.0, (u2 * 2.0 - 1.0) * P.lum.y);
}

// ---------- Geometry ----------
fn safe_inv(d: vec3<f32>) -> vec3<f32> {
    let tiny = abs(d) < vec3<f32>(1.0e-9);
    return 1.0 / select(d, vec3<f32>(1.0e-9), tiny);
}

fn slabs(ro: vec3<f32>, inv: vec3<f32>, bmin: vec3<f32>, bmax: vec3<f32>) -> vec2<f32> {
    let t0 = (bmin - ro) * inv;
    let t1 = (bmax - ro) * inv;
    let ts = min(t0, t1);
    let tb = max(t0, t1);
    return vec2<f32>(max(max(ts.x, ts.y), ts.z), min(min(tb.x, tb.y), tb.z));
}

fn box_normal(p: vec3<f32>, bmin: vec3<f32>, bmax: vec3<f32>) -> vec3<f32> {
    let c = 0.5 * (bmin + bmax);
    let h = max(0.5 * (bmax - bmin), vec3<f32>(1.0e-5));
    let d = (p - c) / h;
    let a = abs(d);
    if (a.x >= a.y && a.x >= a.z) { return vec3<f32>(sign(d.x), 0.0, 0.0); }
    if (a.y >= a.z) { return vec3<f32>(0.0, sign(d.y), 0.0); }
    return vec3<f32>(0.0, 0.0, sign(d.z));
}

struct Hit {
    t: f32,
    n: vec3<f32>,
    mat: i32,
    lamp: i32, // lamp whose body or face was hit, else -1
}

// ---------- Luminaire body mesh ----------
struct MHit {
    t: f32,
    n: vec3<f32>,
    flags: u32,
}

// Moeller-Trumbore, two-sided; -1 when missed.
fn tri_hit(ro: vec3<f32>, rd: vec3<f32>, v0: vec3<f32>, e1: vec3<f32>, e2: vec3<f32>, t_max: f32) -> f32 {
    let pv = cross(rd, e2);
    let det = dot(e1, pv);
    if (abs(det) < 1.0e-9) { return -1.0; }
    let inv = 1.0 / det;
    let sv = ro - v0;
    let u = dot(sv, pv) * inv;
    if (u < 0.0 || u > 1.0) { return -1.0; }
    let qv = cross(sv, e1);
    let v = dot(rd, qv) * inv;
    if (v < 0.0 || u + v > 1.0) { return -1.0; }
    let t = dot(e2, qv) * inv;
    if (t > 1.0e-4 && t < t_max) { return t; }
    return -1.0;
}

// Closest body hit in the luminaire frame; `shadow` skips the luminous face.
fn hit_mesh_local(ro: vec3<f32>, rd: vec3<f32>, t_max: f32, shadow: bool) -> MHit {
    var out: MHit;
    out.t = t_max;
    out.flags = 0u;
    out.n = vec3<f32>(0.0, 1.0, 0.0);
    let nn = P.size.z;
    if (nn == 0u) { return out; }
    let inv = safe_inv(rd);
    var stack: array<u32, 32>;
    var sp = 1u;
    stack[0] = 0u;
    let base = nn * 2u;
    loop {
        if (sp == 0u) { break; }
        sp -= 1u;
        let ni = stack[sp];
        let b0 = mesh[ni * 2u];
        let b1 = mesh[ni * 2u + 1u];
        let t0 = (b0.xyz - ro) * inv;
        let t1 = (b1.xyz - ro) * inv;
        let tmin = max(max(min(t0.x, t1.x), min(t0.y, t1.y)), max(min(t0.z, t1.z), 0.0));
        let tmax = min(min(max(t0.x, t1.x), max(t0.y, t1.y)), max(t0.z, t1.z));
        if (tmin > tmax || tmin > out.t) { continue; }
        let first = bitcast<u32>(b0.w);
        let count = bitcast<u32>(b1.w);
        if (count == 0u) {
            if (sp + 2u <= 32u) {
                stack[sp] = first;
                stack[sp + 1u] = first + 1u;
                sp += 2u;
            }
        } else {
            for (var i = first; i < first + count; i++) {
                let a = mesh[base + i * 3u];
                let fl = bitcast<u32>(a.w);
                if (shadow && (fl & 1u) != 0u) { continue; }
                let e1 = mesh[base + i * 3u + 1u].xyz;
                let e2 = mesh[base + i * 3u + 2u].xyz;
                let t = tri_hit(ro, rd, a.xyz, e1, e2, out.t);
                if (t > 0.0) {
                    out.t = t;
                    out.n = normalize(cross(e1, e2));
                    out.flags = fl;
                }
            }
        }
    }
    return out;
}

// Nearest hit of one primitive in (EPS, t_max); t = FAR when none.
struct PHit {
    t: f32,
    n: vec3<f32>,
}

fn miss() -> PHit {
    var m: PHit;
    m.t = FAR;
    m.n = vec3<f32>(0.0, 1.0, 0.0);
    return m;
}

fn hit_obb(pr: Prim, ro: vec3<f32>, rd: vec3<f32>, t_max: f32) -> PHit {
    let r = mat3x3<f32>(pr.r0.xyz, pr.r1.xyz, pr.r2.xyz);
    let rt = transpose(r);
    let o = rt * (ro - pr.p0.xyz);
    let d = rt * rd;
    let half = pr.p1.xyz;
    let tt = slabs(o, safe_inv(d), -half, half);
    if (tt.x <= tt.y && tt.y > EPS) {
        let t = select(tt.y, tt.x, tt.x > EPS);
        if (t < t_max) {
            var h: PHit;
            h.t = t;
            h.n = r * box_normal(o + d * t, -half, half);
            return h;
        }
    }
    return miss();
}

fn hit_sphere(c: vec3<f32>, rad: f32, ro: vec3<f32>, rd: vec3<f32>, t_max: f32) -> PHit {
    let oc = ro - c;
    let b = dot(oc, rd);
    let cc = dot(oc, oc) - rad * rad;
    let disc = b * b - cc;
    if (disc < 0.0) { return miss(); }
    let sq = sqrt(disc);
    var t = -b - sq;
    if (t <= EPS) { t = -b + sq; }
    if (t <= EPS || t >= t_max) { return miss(); }
    var h: PHit;
    h.t = t;
    h.n = normalize(ro + rd * t - c);
    return h;
}

fn hit_capsule(a: vec3<f32>, b: vec3<f32>, rad: f32, ro: vec3<f32>, rd: vec3<f32>, t_max: f32) -> PHit {
    let ba = b - a;
    let oa = ro - a;
    let baba = dot(ba, ba);
    let bard = dot(ba, rd);
    let baoa = dot(ba, oa);
    let rdoa = dot(rd, oa);
    let oaoa = dot(oa, oa);
    let k2 = baba - bard * bard;
    let k1 = baba * rdoa - baoa * bard;
    let k0 = baba * oaoa - baoa * baoa - rad * rad * baba;
    let hh = k1 * k1 - k2 * k0;
    if (hh < 0.0) { return miss(); }
    let sq = sqrt(hh);
    let t = (-k1 - sq) / max(k2, 1.0e-9);
    let y = baoa + t * bard;
    if (y > 0.0 && y < baba && t > EPS && t < t_max) {
        var h: PHit;
        h.t = t;
        h.n = (oa + t * rd - ba * y / baba) / rad;
        return h;
    }
    // End caps.
    let cap = select(a, b, y > 0.0);
    return hit_sphere(cap, rad, ro, rd, t_max);
}

fn hit_cylinder(a: vec3<f32>, b: vec3<f32>, rad: f32, ro: vec3<f32>, rd: vec3<f32>, t_max: f32) -> PHit {
    let ba = b - a;
    let oc = ro - a;
    let baba = dot(ba, ba);
    let bard = dot(ba, rd);
    let baoc = dot(ba, oc);
    let k2 = baba - bard * bard;
    let k1 = baba * dot(oc, rd) - baoc * bard;
    let k0 = baba * dot(oc, oc) - baoc * baoc - rad * rad * baba;
    let hh = k1 * k1 - k2 * k0;
    if (hh < 0.0) { return miss(); }
    let sq = sqrt(hh);
    let t = (-k1 - sq) / max(k2, 1.0e-9);
    let y = baoc + t * bard;
    if (y > 0.0 && y < baba && t > EPS && t < t_max) {
        var h: PHit;
        h.t = t;
        h.n = (oc + t * rd - ba * y / baba) / rad;
        return h;
    }
    if (abs(bard) < 1.0e-9) { return miss(); }
    let tc = (select(0.0, baba, y > 0.0) - baoc) / bard;
    if (abs(k1 + k2 * tc) < sq && tc > EPS && tc < t_max) {
        var h: PHit;
        h.t = tc;
        h.n = ba * sign(y) / sqrt(baba);
        return h;
    }
    return miss();
}

fn hit_prim(pr: Prim, ro: vec3<f32>, rd: vec3<f32>, t_max: f32) -> PHit {
    switch (pr.kind) {
        case 1u: { return hit_sphere(pr.p0.xyz, pr.p0.w, ro, rd, t_max); }
        case 2u: { return hit_capsule(pr.p0.xyz, pr.p1.xyz, pr.p0.w, ro, rd, t_max); }
        case 3u: { return hit_cylinder(pr.p0.xyz, pr.p1.xyz, pr.p0.w, ro, rd, t_max); }
        default: { return hit_obb(pr, ro, rd, t_max); }
    }
}

fn trace_scene(ro: vec3<f32>, rd: vec3<f32>, primary: bool) -> Hit {
    var h: Hit;
    h.t = FAR;
    h.mat = -1;
    h.lamp = -1;
    h.n = vec3<f32>(0.0, 1.0, 0.0);
    let ground = P.flags.x;
    if (ground > 0u && rd.y < 0.0 && ro.y > 0.0) {
        h.t = -ro.y / rd.y;
        h.mat = MAT_PAVING;
        if (ground == 2u) { h.mat = MAT_ASPHALT; }
        if (ground == 3u) { h.mat = MAT_PARKING; }
    }
    let n = P.counts.w;
    for (var i = 0u; i < n; i++) {
        let pr = prims[i];
        if (primary && (pr.flags & 2u) != 0u) { continue; }
        let ph = hit_prim(pr, ro, rd, h.t);
        if (ph.t < h.t) {
            h.t = ph.t;
            h.n = ph.n;
            h.mat = i32(pr.material);
        }
    }
    if (P.size.w > 0u) {
        let count = P.counts.z;
        for (var li = 0u; li < count; li++) {
            let r = lamp_rot(li);
            let rt = transpose(r);
            let mh = hit_mesh_local(rt * (ro - lamps[li].pos.xyz), rt * rd, h.t, false);
            if (mh.t < h.t) {
                h.t = mh.t;
                h.n = r * mh.n;
                h.mat = select(MAT_HOUSING, MAT_LUMINOUS, (mh.flags & 1u) != 0u);
                h.lamp = i32(li);
            }
        }
    }
    return h;
}

fn occluded(ro: vec3<f32>, rd: vec3<f32>, t_max: f32) -> bool {
    let n = P.counts.w;
    for (var i = 0u; i < n; i++) {
        let pr = prims[i];
        if ((pr.flags & 1u) == 0u) { continue; }
        if (hit_prim(pr, ro, rd, t_max).t < t_max) { return true; }
    }
    if (P.size.w > 0u) {
        let count = P.counts.z;
        for (var li = 0u; li < count; li++) {
            let rt = transpose(lamp_rot(li));
            if (hit_mesh_local(rt * (ro - lamps[li].pos.xyz), rt * rd, t_max, true).t < t_max) { return true; }
        }
    }
    return false;
}

// ---------- Materials ----------
struct Mat {
    albedo: vec3<f32>,
    rough: f32,
    metal: f32,
}

struct MatN {
    m: Mat,
    n: vec3<f32>,
}

fn make_mat(a: vec3<f32>, r: f32, m: f32) -> Mat {
    var o: Mat;
    o.albedo = a;
    o.rough = r;
    o.metal = m;
    return o;
}

fn grid_line(x: f32, w: f32) -> f32 {
    let d = abs(fract(x + 0.5) - 0.5);
    return 1.0 - smoothstep(w * 0.5, w, d);
}

// Painted line on asphalt: white with worn edges.
fn paint(a: vec3<f32>, p: vec3<f32>, on: bool) -> vec3<f32> {
    if (!on) { return a; }
    return vec3<f32>(0.62, 0.62, 0.58) * (0.8 + 0.2 * vnoise(p.xz * 24.0));
}

fn asphalt_base(p: vec3<f32>, tone: vec3<f32>) -> vec3<f32> {
    let grain = 0.7 + 0.4 * fbm(p.xz * 2.2) + 0.3 * vnoise(p.xz * 75.0);
    return tone * grain;
}

fn material(id: i32, p: vec3<f32>, n_in: vec3<f32>) -> MatN {
    var out: MatN;
    out.n = n_in;
    let n = n_in;
    if (id == MAT_PAVING) {
        // 0.4 m concrete slabs with a 1.2 m module of darker setts.
        if (n.y < 0.5) {
            out.m = make_mat(vec3<f32>(0.33, 0.33, 0.32), 0.9, 0.0);
            return out;
        }
        let tile = p.xz / 0.4;
        let cell = floor(tile);
        let f = fract(tile);
        let joint = step(min(min(f.x, 1.0 - f.x), min(f.y, 1.0 - f.y)), 0.03);
        let band = step(fract(p.x / 2.4), 0.17);
        let v = 0.9 + 0.2 * hash12(cell) + 0.08 * vnoise(p.xz * 30.0);
        var a = mix(vec3<f32>(0.34, 0.34, 0.33), vec3<f32>(0.22, 0.21, 0.2), band) * v;
        a = mix(a, vec3<f32>(0.14), joint);
        out.m = make_mat(a, 0.85, 0.0);
        return out;
    }
    if (id == MAT_ASPHALT) {
        let az = abs(p.z);
        if (az < P.f0.z) {
            var a = asphalt_base(p, vec3<f32>(0.085, 0.088, 0.095));
            // Centre line: 4 m dashes on a 12 m period. Edge lines solid.
            let dash = az < 0.07 && fract((p.x + 2.0) / 12.0) * 12.0 < 4.0;
            let edge = abs(az - (P.f0.z - 0.25)) < 0.07;
            out.m = make_mat(paint(a, p, dash || edge), 0.75, 0.0);
            return out;
        }
        out.m = make_mat(vec3<f32>(0.09, 0.075, 0.06) * (0.7 + 0.6 * fbm(p.xz * 1.9)), 1.0, 0.0);
        return out;
    }
    if (id == MAT_PARKING) {
        var a = asphalt_base(p, vec3<f32>(0.08, 0.082, 0.088));
        let az = abs(p.z);
        // Two rows of 2.5 × 5 m bays between |z| = 3 and 8.
        let in_row = az > 3.0 && az < 8.0;
        let bay = in_row && fract(p.x / 2.5) * 2.5 < 0.1;
        let rim = abs(az - 3.0) < 0.05 || abs(az - 8.0) < 0.05;
        out.m = make_mat(paint(a, p, bay || rim), 0.75, 0.0);
        return out;
    }
    if (id == MAT_CYCLE) {
        let a = asphalt_base(p, vec3<f32>(0.26, 0.09, 0.06));
        out.m = make_mat(a, 0.8, 0.0);
        return out;
    }
    if (id == MAT_KERB) { out.m = make_mat(vec3<f32>(0.42, 0.42, 0.4), 0.85, 0.0); return out; }
    if (id == MAT_RENDER) {
        out.m = make_mat(vec3<f32>(0.62, 0.6, 0.55) * (0.95 + 0.08 * fbm(p.xy * 2.5 + p.z)), 0.95, 0.0);
        return out;
    }
    if (id == MAT_BRICK) {
        // Facades face ±Z; their ends are plain.
        if (abs(n.z) < 0.5) {
            out.m = make_mat(vec3<f32>(0.36, 0.3, 0.27), 0.9, 0.0);
            return out;
        }
        let bw = 0.24;
        let bh = 0.071;
        let row = floor(p.y / bh);
        let x = p.x / bw + 0.5 * (row - 2.0 * floor(row / 2.0));
        let col = floor(x);
        let f = vec2<f32>(fract(x) * bw, fract(p.y / bh) * bh);
        let mortar = 0.012;
        if (f.x < mortar || f.y < mortar) {
            out.m = make_mat(vec3<f32>(0.5, 0.48, 0.44) * (0.9 + 0.2 * vnoise(p.xy * 80.0)), 1.0, 0.0);
            return out;
        }
        let h = hash12(vec2<f32>(col, row));
        let a = mix(vec3<f32>(0.38, 0.17, 0.11), vec3<f32>(0.5, 0.27, 0.18), h) * (0.85 + 0.3 * vnoise(p.xy * 40.0));
        out.m = make_mat(a, 0.9, 0.0);
        return out;
    }
    if (id == MAT_GLASS) { out.m = make_mat(vec3<f32>(0.05, 0.06, 0.07), 0.12, 0.6); return out; }
    if (id == MAT_CARPET) {
        if (n.y < 0.5) {
            out.m = make_mat(vec3<f32>(0.15, 0.17, 0.2), 1.0, 0.0);
            return out;
        }
        // Carpet tiles 0.5 m, quarter-turned.
        let tile = floor(p.xz / 0.5);
        let turn = (tile.x + tile.y) - 2.0 * floor((tile.x + tile.y) / 2.0);
        let dir = select(p.x, p.z, turn > 0.5);
        let weave = 0.92 + 0.08 * sin(dir * 180.0) * vnoise(p.xz * 60.0);
        let a = vec3<f32>(0.2, 0.24, 0.3) * weave * (0.95 + 0.1 * hash12(tile));
        out.m = make_mat(a, 1.0, 0.0);
        return out;
    }
    if (id == MAT_WALL) { out.m = make_mat(vec3<f32>(0.78, 0.8, 0.78) * (0.97 + 0.04 * fbm(p.xy * 4.0 + p.z)), 0.95, 0.0); return out; }
    if (id == MAT_CEILING) { out.m = make_mat(vec3<f32>(0.85), 0.95, 0.0); return out; }
    if (id == MAT_METAL) { out.m = make_mat(vec3<f32>(0.2, 0.21, 0.22), 0.4, 0.5); return out; }
    if (id == MAT_DESK) { out.m = make_mat(vec3<f32>(0.6, 0.55, 0.47) * (0.95 + 0.1 * vnoise(p.xz * vec2<f32>(2.0, 40.0))), 0.35, 0.0); return out; }
    if (id == MAT_CAR) { out.m = make_mat(vec3<f32>(0.1, 0.14, 0.3), 0.28, 0.55); return out; }
    if (id == MAT_FABRIC) { out.m = make_mat(vec3<f32>(0.12, 0.12, 0.13), 1.0, 0.0); return out; }
    if (id == MAT_PERSON) { out.m = make_mat(vec3<f32>(0.45, 0.4, 0.36), 0.8, 0.0); return out; }
    if (id == MAT_JACKET) { out.m = make_mat(vec3<f32>(0.55, 0.32, 0.12), 0.9, 0.0); return out; }
    if (id == MAT_HEDGE) {
        out.m = make_mat(vec3<f32>(0.08, 0.17, 0.05) * (0.6 + 0.8 * fbm(p.xz * 5.0 + p.y)), 1.0, 0.0);
        return out;
    }
    if (id == MAT_SOIL) { out.m = make_mat(vec3<f32>(0.1, 0.08, 0.06) * (0.7 + 0.6 * fbm(p.xz * 1.9)), 1.0, 0.0); return out; }
    if (id == MAT_WHITE) { out.m = make_mat(vec3<f32>(0.8, 0.8, 0.78), 0.5, 0.0); return out; }
    if (id == MAT_RUBBER) { out.m = make_mat(vec3<f32>(0.03, 0.03, 0.03), 0.85, 0.0); return out; }
    if (id == MAT_RIM) { out.m = make_mat(vec3<f32>(0.55, 0.56, 0.58), 0.3, 0.8); return out; }
    if (id == MAT_PLASTIC) { out.m = make_mat(vec3<f32>(0.05, 0.05, 0.055), 0.6, 0.0); return out; }
    if (id == MAT_SKIN) { out.m = make_mat(vec3<f32>(0.6, 0.42, 0.32), 0.65, 0.0); return out; }
    if (id == MAT_TROUSERS) { out.m = make_mat(vec3<f32>(0.1, 0.12, 0.2), 0.95, 0.0); return out; }
    if (id == MAT_SHOE) { out.m = make_mat(vec3<f32>(0.06, 0.05, 0.045), 0.5, 0.0); return out; }
    if (id == MAT_HEADLIGHT) { out.m = make_mat(vec3<f32>(0.85, 0.87, 0.9), 0.15, 0.3); return out; }
    if (id == MAT_TAILLIGHT) { out.m = make_mat(vec3<f32>(0.6, 0.05, 0.04), 0.2, 0.3); return out; }
    if (id == MAT_CAR_SILVER) { out.m = make_mat(vec3<f32>(0.55, 0.56, 0.58), 0.28, 0.6); return out; }
    if (id == MAT_CAR_RED) { out.m = make_mat(vec3<f32>(0.42, 0.05, 0.05), 0.28, 0.5); return out; }
    if (id == MAT_HOUSING) { out.m = make_mat(vec3<f32>(0.62, 0.63, 0.64), 0.45, 0.35); return out; }
    if (id == MAT_LUMINOUS) { out.m = make_mat(vec3<f32>(0.4), 0.5, 0.0); return out; }
    out.m = make_mat(vec3<f32>(0.6), 0.6, 0.0);
    return out;
}

// ---------- BSDF ----------
fn tangent_frame(n: vec3<f32>) -> mat3x3<f32> {
    let s = select(-1.0, 1.0, n.z >= 0.0);
    let a = -1.0 / (s + n.z);
    let b = n.x * n.y * a;
    let t = vec3<f32>(1.0 + s * n.x * n.x * a, s * b, -s * n.x);
    let bb = vec3<f32>(b, s + n.y * n.y * a, -n.y);
    return mat3x3<f32>(t, bb, n);
}

fn ggx_d(ndoth: f32, a2: f32) -> f32 {
    let d = ndoth * ndoth * (a2 - 1.0) + 1.0;
    return a2 / max(1.0e-8, PI * d * d);
}

fn smith_g(ndotl: f32, ndotv: f32, a2: f32) -> f32 {
    let gl = ndotv * sqrt(a2 + (1.0 - a2) * ndotl * ndotl);
    let gv = ndotl * sqrt(a2 + (1.0 - a2) * ndotv * ndotv);
    return 2.0 * ndotl * ndotv / max(1.0e-8, gl + gv);
}

fn fresnel(c: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (1.0 - f0) * pow(max(1.0 - c, 0.0), 5.0);
}

fn spec_prob(m: Mat) -> f32 {
    let f0 = mix(vec3<f32>(0.04), m.albedo, m.metal);
    return clamp(0.5 * (1.0 - (1.0 - m.metal) * (1.0 - dot(f0, vec3<f32>(0.333)))), 0.1, 0.9);
}

struct BsdfEval {
    f: vec3<f32>, // f · NdotL
    pdf: f32,
}

fn eval_bsdf(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, m: Mat) -> BsdfEval {
    var r: BsdfEval;
    r.f = vec3<f32>(0.0);
    r.pdf = 0.0;
    let ndotl = dot(n, l);
    let ndotv = dot(n, v);
    if (ndotl <= 0.0 || ndotv <= 0.0) { return r; }
    let a = max(0.04, m.rough) * max(0.04, m.rough);
    let a2 = a * a;
    let h = normalize(l + v);
    let ndoth = clamp(dot(n, h), 0.0, 1.0);
    let vdoth = clamp(dot(v, h), 0.0, 1.0);
    let f0 = mix(vec3<f32>(0.04), m.albedo, m.metal);
    let fr = fresnel(vdoth, f0);
    let d = ggx_d(ndoth, a2);
    let g = smith_g(ndotl, ndotv, a2);
    let spec = (d * g * fr) / max(1.0e-8, 4.0 * ndotl * ndotv);
    let diff = (1.0 - fr) * (1.0 - m.metal) * m.albedo * INV_PI;
    let ps = spec_prob(m);
    r.pdf = (1.0 - ps) * ndotl * INV_PI + ps * (d * ndoth) / max(1.0e-8, 4.0 * vdoth);
    r.f = (diff + spec) * ndotl;
    return r;
}

fn sample_ggx_vndf(vl: vec3<f32>, a: f32, u1: f32, u2: f32) -> vec3<f32> {
    let vh = normalize(vec3<f32>(a * vl.x, a * vl.y, vl.z));
    let lensq = vh.x * vh.x + vh.y * vh.y;
    var t1: vec3<f32>;
    if (lensq > 0.0) {
        t1 = vec3<f32>(-vh.y, vh.x, 0.0) * inverseSqrt(lensq);
    } else {
        t1 = vec3<f32>(1.0, 0.0, 0.0);
    }
    let t2 = cross(vh, t1);
    let r = sqrt(u1);
    let phi = TWO_PI * u2;
    let p1 = r * cos(phi);
    var p2 = r * sin(phi);
    let s = 0.5 * (1.0 + vh.z);
    p2 = (1.0 - s) * sqrt(1.0 - p1 * p1) + s * p2;
    let nh = p1 * t1 + p2 * t2 + sqrt(max(0.0, 1.0 - p1 * p1 - p2 * p2)) * vh;
    return normalize(vec3<f32>(a * nh.x, a * nh.y, max(0.0, nh.z)));
}

struct BsdfSample {
    ok: bool,
    l: vec3<f32>,
    weight: vec3<f32>,
    pdf: f32,
}

fn sample_bsdf(n: vec3<f32>, v: vec3<f32>, m: Mat) -> BsdfSample {
    var s: BsdfSample;
    s.ok = false;
    s.pdf = 0.0;
    let a = max(0.04, m.rough) * max(0.04, m.rough);
    let frame = tangent_frame(n);
    let u0 = rnd();
    let u1 = rnd();
    let u2 = rnd();
    var l: vec3<f32>;
    if (u0 < spec_prob(m)) {
        let h = frame * sample_ggx_vndf(transpose(frame) * v, a, u1, u2);
        l = reflect(-v, h);
        if (dot(n, l) <= 0.0) { return s; }
    } else {
        let r = sqrt(u1);
        let phi = TWO_PI * u2;
        l = frame * vec3<f32>(r * cos(phi), r * sin(phi), sqrt(max(0.0, 1.0 - u1)));
    }
    let e = eval_bsdf(n, v, l, m);
    if (e.pdf <= 1.0e-8) { return s; }
    s.ok = true;
    s.l = l;
    s.weight = e.f / e.pdf;
    s.pdf = e.pdf;
    return s;
}

// ---------- Lights ----------
struct Direct {
    l: vec3<f32>, // reflected radiance
    e: f32,       // illuminance (lux)
}

fn direct_light(p: vec3<f32>, n: vec3<f32>, v: vec3<f32>, m: Mat) -> Direct {
    var out: Direct;
    out.e = 0.0;
    var sum = vec3<f32>(0.0);
    let count = P.counts.z;
    for (var li = 0u; li < count; li++) {
        let sp = lamps[li].pos.xyz + lamp_rot(li) * sample_emitter_local();
        let to_l = sp - p;
        let d2 = dot(to_l, to_l);
        let d = sqrt(d2);
        let wi = to_l / max(d, 1.0e-6);
        let ndotl = dot(n, wi);
        if (ndotl <= 0.0) { continue; }
        let intensity = ies_candela(li, -wi);
        if (intensity <= 0.0) { continue; }
        if (occluded(p + n * EPS, wi, d - 2.0 * EPS)) { continue; }
        let en = intensity / max(d2, 1.0e-4);
        out.e += en * ndotl;
        sum += eval_bsdf(n, v, wi, m).f * en;
    }
    out.l = sum * P.tint.rgb;
    return out;
}

struct LumHit {
    t: f32,
    lamp: i32,
    part: i32, // 1 = luminous face, 2 = housing
    n: vec3<f32>,
}

// Primary-ray hit test against the luminaire housings.
fn hit_luminaires(ro: vec3<f32>, rd: vec3<f32>, t_max: f32) -> LumHit {
    var out: LumHit;
    out.t = t_max;
    out.lamp = -1;
    out.part = 0;
    out.n = vec3<f32>(0.0, 1.0, 0.0);
    let hh = P.housing_half.xyz;
    let hmin = vec3<f32>(-hh.x, 0.0, -hh.z);
    let hmax = vec3<f32>(hh.x, hh.y * 2.0, hh.z);
    let count = P.counts.z;
    for (var li = 0u; li < count; li++) {
        let r = lamp_rot(li);
        let rt = transpose(r);
        let o = rt * (ro - lamps[li].pos.xyz);
        let d = rt * rd;
        let tt = slabs(o, safe_inv(d), hmin, hmax);
        if (tt.x <= tt.y && tt.x > EPS && tt.x < out.t) {
            out.t = tt.x;
            out.lamp = i32(li);
            let lp = o + d * tt.x;
            let ln = box_normal(lp, hmin, hmax);
            var inside: bool;
            if (P.housing_half.w > 0.5) {
                inside = length(lp.xz) <= P.lum.x;
            } else {
                inside = abs(lp.x) <= P.lum.x && abs(lp.z) <= P.lum.y;
            }
            out.part = select(2, 1, ln.y < -0.5 && inside);
            out.n = r * ln;
        }
    }
    return out;
}

fn henyey_greenstein(c: f32, g: f32) -> f32 {
    let d = 1.0 + g * g - 2.0 * g * c;
    return (1.0 - g * g) / (4.0 * PI * d * sqrt(d));
}

// Single scattering in a homogeneous medium, equiangular sampling per lamp.
fn haze_inscatter(ro: vec3<f32>, rd: vec3<f32>, t_max: f32) -> vec3<f32> {
    var sum = vec3<f32>(0.0);
    let haze = P.f1.z;
    let count = P.counts.z;
    for (var li = 0u; li < count; li++) {
        let lp = lamps[li].pos.xyz;
        let delta = dot(lp - ro, rd);
        let dd = max(length(ro + rd * delta - lp), 1.0e-3);
        let th_a = atan2(-delta, dd);
        let th_b = atan2(t_max - delta, dd);
        let ang = mix(th_a, th_b, rnd());
        let t = delta + dd * tan(clamp(ang, -1.5707, 1.5707));
        if (!(t >= 0.0 && t <= t_max)) { continue; }
        let pdf = dd / max(1.0e-6, (th_b - th_a) * (dd * dd + (t - delta) * (t - delta)));
        let x = ro + rd * t;
        let to_l = lp - x;
        let d2 = dot(to_l, to_l);
        let d = sqrt(d2);
        let wi = to_l / max(d, 1.0e-6);
        let intensity = ies_candela(li, -wi);
        if (intensity <= 0.0) { continue; }
        if (occluded(x, wi, d - 0.05)) { continue; }
        let tr = exp(-haze * (t + d));
        sum += intensity / max(d2, 1.0e-3) * henyey_greenstein(dot(wi, rd), 0.35) * haze * tr / pdf;
    }
    return sum * P.tint.rgb;
}

fn sky(rd: vec3<f32>) -> vec3<f32> {
    if (P.f0.w < 0.5) { return vec3<f32>(0.0); }
    // Urban night sky: warm glow at the horizon fading to a deep blue zenith.
    let h = clamp(rd.y, 0.0, 1.0);
    let horizon = vec3<f32>(0.075, 0.06, 0.045);
    let zenith = vec3<f32>(0.006, 0.008, 0.018);
    return mix(horizon, zenith, pow(h, 0.35)) * P.f1.x;
}

// ---------- Camera pass ----------
@compute @workgroup_size(8, 8, 1)
fn trace(@builtin(global_invocation_id) id: vec3<u32>) {
    let w = P.size.x;
    let h = P.size.y;
    if (id.x >= w || id.y >= h) { return; }
    g_seed = id.x + id.y * 1973u + P.frame.x * 9277u + 7u;
    rnd();
    rnd();

    let ndc = vec2<f32>(
        (f32(id.x) + rnd()) / f32(w) * 2.0 - 1.0,
        1.0 - (f32(id.y) + rnd()) / f32(h) * 2.0,
    );
    let near_p = P.inv_view_proj * vec4<f32>(ndc, 0.0, 1.0);
    let far_p = P.inv_view_proj * vec4<f32>(ndc, 1.0, 1.0);
    var ro = P.cam_pos.xyz;
    let cam_rd = normalize(far_p.xyz / far_p.w - near_p.xyz / near_p.w);
    var rd = cam_rd;

    var radiance = vec3<f32>(0.0);
    var throughput = vec3<f32>(1.0);
    // Same path weight as throughput, except the first BSDF is replaced by
    // cos / pdf, so it turns incoming radiance at the primary hit into irradiance.
    var irr_weight = vec3<f32>(0.0);
    var indirect_e = 0.0;
    var primary_e = 0.0;
    var coverage = 0.0;
    var primary_t = FAR;
    let max_bounces = P.flags.z;
    let clamp_v = P.f2.x;

    for (var bounce = 0u; bounce < 6u; bounce++) {
        if (bounce >= max_bounces) { break; }
        let primary = bounce == 0u;
        let hit = trace_scene(ro, rd, primary);

        if (hit.mat == MAT_LUMINOUS) {
            if (primary && hit.lamp >= 0) {
                let li = u32(hit.lamp);
                let nadir = lamp_rot(li) * vec3<f32>(0.0, -1.0, 0.0);
                let c = max(0.03, dot(-rd, nadir));
                var area: f32;
                if (P.housing_half.w > 0.5) {
                    area = PI * P.lum.x * P.lum.x;
                } else {
                    area = 4.0 * P.lum.x * P.lum.y;
                }
                primary_t = hit.t;
                radiance += ies_candela(li, -rd) / (area * c) * P.tint.rgb;
            }
            break;
        }

        if (primary && P.size.w == 0u) {
            let lh = hit_luminaires(ro, rd, hit.t);
            if (lh.lamp >= 0) {
                primary_t = lh.t;
                if (lh.part == 1) {
                    let nadir = lamp_rot(u32(lh.lamp)) * vec3<f32>(0.0, -1.0, 0.0);
                    let c = max(0.03, dot(-rd, nadir));
                    var area: f32;
                    if (P.housing_half.w > 0.5) {
                        area = PI * P.lum.x * P.lum.x;
                    } else {
                        area = 4.0 * P.lum.x * P.lum.y;
                    }
                    radiance += ies_candela(u32(lh.lamp), -rd) / (area * c) * P.tint.rgb;
                } else {
                    radiance += vec3<f32>(0.05, 0.052, 0.056) * (0.35 + 0.65 * max(0.0, lh.n.y)) * P.f1.y * 40.0;
                }
                break;
            }
        }

        if (hit.mat < 0) {
            if (primary) {
                radiance += sky(rd);
            } else {
                var fill = vec3<f32>(1.0);
                if (P.f0.w > 0.5) { fill = vec3<f32>(0.8, 0.88, 1.0); }
                radiance += throughput * P.f1.y * fill;
            }
            break;
        }

        let p = ro + rd * hit.t;
        var geom_n = hit.n;
        if (dot(geom_n, rd) > 0.0) { geom_n = -geom_n; }
        let mn = material(hit.mat, p, geom_n);
        var n = mn.n;
        if (dot(n, rd) > 0.0) { n = geom_n; }
        let m = mn.m;
        let v = -rd;

        let dl = direct_light(p, n, v, m);
        if (primary) {
            primary_e = dl.e;
            coverage = 1.0;
            primary_t = hit.t;
            radiance += dl.l;
        } else {
            let c = throughput * dl.l;
            let mx = max(c.r, max(c.g, c.b));
            let k = select(1.0, clamp_v / mx, mx > clamp_v);
            radiance += c * k;
            indirect_e += dot(irr_weight * dl.l * k, LUMA) / P.tint.w;
        }

        let s = sample_bsdf(n, v, m);
        if (!s.ok) { break; }
        if (primary) {
            irr_weight = vec3<f32>(max(dot(n, s.l), 0.0) / s.pdf);
        } else {
            irr_weight *= s.weight;
        }
        throughput *= s.weight;
        if (bounce >= 2u) {
            let q = clamp(max(throughput.r, max(throughput.g, throughput.b)), 0.05, 0.95);
            if (rnd() > q) { break; }
            throughput /= q;
            irr_weight /= q;
        }
        ro = p + geom_n * EPS;
        rd = s.l;
    }

    if (P.f1.z > 0.0) {
        // Misses have no surface, so stop them at a far plane instead of FAR.
        let t_max = select(primary_t, 500.0, primary_t > 1.0e6);
        radiance = radiance * exp(-P.f1.z * t_max) + haze_inscatter(P.cam_pos.xyz, cam_rd, t_max);
    }
    if (P.f1.w > 0.0) {
        var veil = 0.0;
        let count = P.counts.z;
        for (var li = 0u; li < count; li++) {
            let to_l = normalize(lamps[li].pos.xyz - P.cam_pos.xyz);
            let ang = degrees(acos(clamp(dot(cam_rd, to_l), -1.0, 1.0)));
            veil += lamps[li].pos.w * 10.0 / (ang * ang + 0.35);
        }
        radiance += veil * P.f1.w * P.tint.rgb;
    }

    let bad = radiance.r != radiance.r || radiance.g != radiance.g || radiance.b != radiance.b
        || abs(radiance.r) > 1.0e30 || abs(radiance.g) > 1.0e30 || abs(radiance.b) > 1.0e30;
    if (bad) { radiance = vec3<f32>(0.0); }
    var e = primary_e + indirect_e;
    if (e != e || abs(e) > 1.0e30) { e = primary_e; }
    let lux_sample = vec4<f32>(e, coverage, min(primary_t, 1.0e4), 1.0);

    let idx = id.y * w + id.x;
    let blend = P.f0.x;
    if (blend >= 1.0) {
        acc_color[idx] = vec4<f32>(radiance, 1.0);
        acc_lux[idx] = lux_sample;
    } else {
        let pc = acc_color[idx];
        let pl = acc_lux[idx];
        acc_color[idx] = vec4<f32>(mix(pc.rgb, radiance, blend), 1.0);
        acc_lux[idx] = mix(pl, lux_sample, blend);
    }
}

// ---------- Measurement points ----------
// Same estimator as the primary-hit illuminance in the camera pass, starting
// from a point and a normal instead of a camera ray.
fn probe_irradiance(p: vec3<f32>, n: vec3<f32>) -> f32 {
    let black = make_mat(vec3<f32>(0.0), 1.0, 0.0);
    var e = direct_light(p, n, n, black).e;
    let u1 = rnd();
    let phi = TWO_PI * rnd();
    let r = sqrt(u1);
    var rd = tangent_frame(n) * vec3<f32>(r * cos(phi), r * sin(phi), sqrt(max(0.0, 1.0 - u1)));
    var ro = p + n * EPS;
    // Cosine sampling, so cos / pdf = PI.
    var irr_weight = vec3<f32>(PI);
    let max_bounces = P.flags.z;
    let clamp_v = P.f2.x;
    for (var bounce = 1u; bounce < 6u; bounce++) {
        if (bounce >= max_bounces) { break; }
        let hit = trace_scene(ro, rd, false);
        if (hit.mat < 0) { break; }
        let hp = ro + rd * hit.t;
        var geom_n = hit.n;
        if (dot(geom_n, rd) > 0.0) { geom_n = -geom_n; }
        let mn = material(hit.mat, hp, geom_n);
        var nn = mn.n;
        if (dot(nn, rd) > 0.0) { nn = geom_n; }
        let v = -rd;
        let dl = direct_light(hp, nn, v, mn.m);
        let c = irr_weight * INV_PI * dl.l;
        let mx = max(c.r, max(c.g, c.b));
        let k = select(1.0, clamp_v / mx, mx > clamp_v);
        e += dot(irr_weight * dl.l * k, LUMA) / P.tint.w;
        let s = sample_bsdf(nn, v, mn.m);
        if (!s.ok) { break; }
        irr_weight *= s.weight;
        if (bounce >= 2u) {
            let q = clamp(max(irr_weight.r, max(irr_weight.g, irr_weight.b)) * INV_PI, 0.05, 0.95);
            if (rnd() > q) { break; }
            irr_weight /= q;
        }
        ro = hp + geom_n * EPS;
        rd = s.l;
    }
    return e;
}

@compute @workgroup_size(64, 1, 1)
fn probe(@builtin(global_invocation_id) id: vec3<u32>) {
    let k = id.x;
    if (k >= P.frame.y) { return; }
    g_seed = k * 7919u + P.frame.x * 9277u + 11u;
    rnd();
    rnd();
    let p = probe_pos[2u * k].xyz;
    let n = probe_pos[2u * k + 1u].xyz;
    let samples = P.frame.z;
    var sum = 0.0;
    for (var s = 0u; s < 32u; s++) {
        if (s >= samples) { break; }
        sum += probe_irradiance(p, n);
    }
    var e = sum / f32(max(samples, 1u));
    if (e != e || abs(e) > 1.0e30) { e = 0.0; }
    let blend = P.f0.y;
    var prev = 0.0;
    if (blend < 1.0) { prev = probe_out[k].x; }
    probe_out[k] = vec4<f32>(mix(prev, e, blend), 1.0, 0.0, 0.0);
}

// ---------- Present ----------
// Filmic curve (Hable), softer shoulder than ACES: lamps bloom less abruptly.
fn hable(x: vec3<f32>) -> vec3<f32> {
    let a = 0.15; let b = 0.50; let c = 0.10; let d = 0.20; let e = 0.02; let f = 0.30;
    return ((x * (a * x + c * b) + d * e) / (x * (a * x + b) + d * f)) - e / f;
}

fn filmic(x: vec3<f32>) -> vec3<f32> {
    let white = hable(vec3<f32>(11.2));
    return clamp(hable(x * 2.0) / white, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    return pow(max(c, vec3<f32>(0.0)), vec3<f32>(2.2));
}

// Sequential illuminance ramp, viridis-like (dark violet → teal → yellow).
fn ramp(t: f32) -> vec3<f32> {
    let x = clamp(t, 0.0, 1.0);
    let c0 = vec3<f32>(0.2777273, 0.0054073, 0.3340998);
    let c1 = vec3<f32>(0.1050930, 1.4046135, 1.3845902);
    let c2 = vec3<f32>(-0.3308618, 0.2148476, 0.0950952);
    let c3 = vec3<f32>(-4.6342305, -5.7991010, -19.332441);
    let c4 = vec3<f32>(6.2282700, 14.179933, 56.690550);
    let c5 = vec3<f32>(4.7763850, -13.745145, -65.353035);
    let c6 = vec3<f32>(-5.4354560, 4.6458526, 26.312435);
    return clamp(c0 + x * (c1 + x * (c2 + x * (c3 + x * (c4 + x * (c5 + x * c6))))), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn lux_ramp(e: f32, lux_max: f32) -> f32 {
    let x = e / lux_max;
    return sqrt(max(x, 0.0)) * 0.15 + x * 0.85;
}

fn lux_at(x: i32, y: i32) -> vec4<f32> {
    let w = i32(P.size.x);
    let h = i32(P.size.y);
    let cx = clamp(x, 0, w - 1);
    let cy = clamp(y, 0, h - 1);
    return acc_lux[u32(cy * w + cx)];
}

// Illuminance read over a 5 × 5 spot, like a meter head. Interreflected light
// converges slowly; neighbours that differ a lot belong to another surface
// and are left out, so edges stay sharp.
fn smooth_e(px: i32, py: i32) -> f32 {
    let l0 = lux_at(px, py);
    let e0 = l0.x / max(l0.y, 1.0e-4);
    var sum_e = 0.0;
    var sum_w = 0.0;
    for (var dy = -2; dy <= 2; dy++) {
        for (var dx = -2; dx <= 2; dx++) {
            let l = lux_at(px + dx, py + dy);
            if (l.y < 0.5) { continue; }
            let en = l.x / l.y;
            let r = (en - e0) / max(0.2 * e0, 1.0e-3);
            let wgt = exp(-r * r);
            sum_e += en * wgt;
            sum_w += wgt;
        }
    }
    if (sum_w > 0.0) { return sum_e / sum_w; }
    return e0;
}

@compute @workgroup_size(8, 8, 1)
fn present(@builtin(global_invocation_id) id: vec3<u32>) {
    let w = P.size.x;
    let h = P.size.y;
    if (id.x >= w || id.y >= h) { return; }
    let idx = id.y * w + id.x;
    let radiance = acc_color[idx].rgb;
    let lux = acc_lux[idx];
    var display = filmic(radiance * P.f2.y);
    let px = i32(id.x);
    let py = i32(id.y);
    let e = smooth_e(px, py);

    let mode = P.flags.w;
    if (mode == 1u) {
        let shade = dot(display, LUMA);
        if (lux.y < 0.02) {
            display = srgb_to_linear(vec3<f32>(0.03, 0.03, 0.035)) + shade * 0.2;
        } else {
            let heat = srgb_to_linear(ramp(lux_ramp(e, P.f2.z)));
            display = heat * (0.45 + 0.75 * clamp(shade * 3.0, 0.3, 1.0));
        }
    } else if (mode == 2u) {
        // Dim the same picture so the points read, keeping its colour.
        display *= 0.42;
    }

    if (P.frame.w > 0u && lux.y > 0.5) {
        let iso = P.f2.w;
        // Line width from the smoothed field, like fwidth() in a fragment shader.
        let f = e / iso;
        let fx = abs(smooth_e(px + 1, py) - e) / iso;
        let fy = abs(smooth_e(px, py + 1) - e) / iso;
        let wd = max(fx + fy, 1.0e-4);
        let d = abs(fract(f + 0.5) - 0.5);
        let line = (1.0 - smoothstep(0.5 * wd, 1.5 * wd, d)) * step(wd, 0.45) * step(0.5, f);
        var line_color = srgb_to_linear(vec3<f32>(0.376, 0.647, 0.98));
        if (mode == 1u) { line_color = vec3<f32>(0.92); }
        display = mix(display, line_color, line * 0.8);
    }

    textureStore(output, vec2<i32>(px, py), vec4<f32>(display, 1.0));
}
