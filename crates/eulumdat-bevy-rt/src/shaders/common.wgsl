#define_import_path eulumdat_rt::common

// Shared structs and utilities for photometric raytracing.

struct GpuPrimitive {
    ptype: u32,
    material_id: u32,
    _pad0: u32,
    _pad1: u32,
    // Sheet: center(3), normal(3), u_axis(3), half_width, half_height, thickness
    params: array<f32, 12>,
}

struct GpuMaterial {
    mtype: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
    reflectance: f32,
    ior: f32,
    transmittance: f32,
    min_reflectance: f32,
    absorption_coeff: f32,
    scattering_coeff: f32,
    asymmetry: f32,
    thickness: f32,
}

struct HitRecord {
    t: f32,
    point: vec3<f32>,
    normal: vec3<f32>,
    front_face: bool,
    material_id: u32,
    valid: bool,
}

struct Interaction {
    itype: u32,
    new_origin: vec3<f32>,
    new_dir: vec3<f32>,
    attenuation: f32,
}

// Material types
const MAT_ABSORBER: u32 = 0u;
const MAT_DIFFUSE_REFL: u32 = 1u;
const MAT_SPECULAR_REFL: u32 = 2u;
const MAT_MIXED_REFL: u32 = 3u;
const MAT_CLEAR_TRANS: u32 = 4u;
const MAT_DIFFUSE_TRANS: u32 = 5u;

// Primitive types
const PRIM_SHEET: u32 = 0u;

// Interaction results
const INTERACTION_ABSORBED: u32 = 0u;
const INTERACTION_REFLECTED: u32 = 1u;
const INTERACTION_TRANSMITTED: u32 = 2u;

const PI: f32 = 3.14159265358979323846;
const EPSILON: f32 = 1e-6;

// ============================================================================
// RNG: PCG
// ============================================================================

var<private> rng_state: u32;

fn pcg_init(id: u32, seed: u32) {
    rng_state = id * 747796405u + seed * 2891336453u + 1u;
    rng_state = rng_state * 747796405u + 2891336453u;
}

fn pcg_next() -> u32 {
    let state = rng_state;
    rng_state = state * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn random_f32() -> f32 {
    return f32(pcg_next()) / 4294967295.0;
}

// ============================================================================
// Vector math
// ============================================================================

fn reflect_dir(incoming: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    return incoming - 2.0 * dot(incoming, normal) * normal;
}

fn refract_dir(incoming: vec3<f32>, normal: vec3<f32>, eta: f32) -> vec3<f32> {
    let cos_i = clamp(-dot(incoming, normal), -1.0, 1.0);
    let sin2_t = eta * eta * (1.0 - cos_i * cos_i);
    if (sin2_t > 1.0) { return vec3<f32>(0.0); } // TIR sentinel
    let cos_t = sqrt(1.0 - sin2_t);
    return eta * incoming + (eta * cos_i - cos_t) * normal;
}

fn fresnel_schlick(cos_theta: f32, eta_ratio: f32) -> f32 {
    let r0 = pow((1.0 - eta_ratio) / (1.0 + eta_ratio), 2.0);
    return r0 + (1.0 - r0) * pow(1.0 - cos_theta, 5.0);
}

fn build_onb(n: vec3<f32>) -> mat3x3<f32> {
    var a: vec3<f32>;
    if (abs(n.x) > 0.9) { a = vec3<f32>(0.0, 1.0, 0.0); }
    else { a = vec3<f32>(1.0, 0.0, 0.0); }
    let t = normalize(cross(n, a));
    let b = normalize(cross(n, t));
    return mat3x3<f32>(t, b, n);
}

fn random_sphere() -> vec3<f32> {
    let z = 2.0 * random_f32() - 1.0;
    let r = sqrt(max(1.0 - z * z, 0.0));
    let phi = 2.0 * PI * random_f32();
    return vec3<f32>(r * cos(phi), r * sin(phi), z);
}

fn random_cosine_hemisphere(normal: vec3<f32>) -> vec3<f32> {
    let u1 = random_f32();
    let u2 = random_f32();
    let r = sqrt(u1);
    let theta = 2.0 * PI * u2;
    let local = vec3<f32>(r * cos(theta), r * sin(theta), sqrt(max(1.0 - u1, 0.0)));
    let onb = build_onb(normal);
    return normalize(onb * local);
}

fn sample_henyey_greenstein(incoming: vec3<f32>, g: f32) -> vec3<f32> {
    let xi = random_f32();
    var cos_theta: f32;
    if (abs(g) < 1e-6) {
        cos_theta = 1.0 - 2.0 * xi;
    } else {
        let term = (1.0 - g * g) / (1.0 - g + 2.0 * g * xi);
        cos_theta = (1.0 + g * g - term * term) / (2.0 * g);
    }
    let sin_theta = sqrt(max(1.0 - cos_theta * cos_theta, 0.0));
    let phi = 2.0 * PI * random_f32();
    let onb = build_onb(incoming);
    let local = vec3<f32>(sin_theta * cos(phi), sin_theta * sin(phi), cos_theta);
    return normalize(onb * local);
}
