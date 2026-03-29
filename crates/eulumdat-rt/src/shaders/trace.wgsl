// eulumdat-rt: Monte Carlo photon tracing compute shader
//
// Phase 2: Full tracing with geometry intersection + materials.
// Each invocation traces one photon through the scene.

struct TraceConfig {
    detector_c_bins: u32,
    detector_g_bins: u32,
    detector_c_res: f32,
    detector_g_res: f32,
    seed_offset: u32,
    num_photons: u32,
    source_type: u32,       // 0=isotropic, 1=lambertian
    source_flux: f32,
    num_primitives: u32,
    max_bounces: u32,
    rr_threshold: f32,      // Russian roulette energy threshold
    _padding: u32,
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

@group(0) @binding(0) var<storage, read_write> detector_bins: array<atomic<u32>>;
@group(0) @binding(1) var<uniform> config: TraceConfig;
@group(0) @binding(2) var<storage, read> primitives: array<GpuPrimitive>;
@group(0) @binding(3) var<storage, read> materials: array<GpuMaterial>;

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

const PI: f32 = 3.14159265358979323846;
const EPSILON: f32 = 1e-6;

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

// ============================================================================
// Direction sampling
// ============================================================================

fn random_sphere() -> vec3<f32> {
    let z = 2.0 * random_f32() - 1.0;
    let r = sqrt(max(1.0 - z * z, 0.0));
    let phi = 2.0 * PI * random_f32();
    return vec3<f32>(r * cos(phi), r * sin(phi), z);
}

fn random_cosine_hemisphere_down() -> vec3<f32> {
    let u1 = random_f32();
    let u2 = random_f32();
    let r = sqrt(u1);
    let theta = 2.0 * PI * u2;
    return vec3<f32>(r * cos(theta), r * sin(theta), -sqrt(max(1.0 - u1, 0.0)));
}

fn build_onb(n: vec3<f32>) -> mat3x3<f32> {
    var a: vec3<f32>;
    if (abs(n.x) > 0.9) { a = vec3<f32>(0.0, 1.0, 0.0); }
    else { a = vec3<f32>(1.0, 0.0, 0.0); }
    let t = normalize(cross(n, a));
    let b = normalize(cross(n, t));
    return mat3x3<f32>(t, b, n);
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

// ============================================================================
// Ray-Sheet intersection
// ============================================================================

struct HitRecord {
    t: f32,
    point: vec3<f32>,
    normal: vec3<f32>,
    front_face: bool,
    material_id: u32,
    valid: bool,
}

fn intersect_sheet(
    ray_origin: vec3<f32>,
    ray_dir: vec3<f32>,
    prim: GpuPrimitive,
    t_min: f32,
    t_max: f32,
) -> HitRecord {
    var hit: HitRecord;
    hit.valid = false;

    let center = vec3<f32>(prim.params[0], prim.params[1], prim.params[2]);
    let normal = vec3<f32>(prim.params[3], prim.params[4], prim.params[5]);
    let u_axis = vec3<f32>(prim.params[6], prim.params[7], prim.params[8]);
    let half_w = prim.params[9];
    let half_h = prim.params[10];

    let denom = dot(ray_dir, normal);
    if (abs(denom) < EPSILON) { return hit; }

    let t = dot(center - ray_origin, normal) / denom;
    if (t < t_min || t > t_max) { return hit; }

    let p = ray_origin + t * ray_dir;
    let local = p - center;
    let v_axis = cross(normal, u_axis);
    let u = dot(local, u_axis);
    let v = dot(local, v_axis);

    if (abs(u) > half_w || abs(v) > half_h) { return hit; }

    hit.valid = true;
    hit.t = t;
    hit.point = p;
    hit.material_id = prim.material_id;
    hit.front_face = denom < 0.0;
    if (hit.front_face) { hit.normal = normal; }
    else { hit.normal = -normal; }

    return hit;
}

fn intersect_scene(ray_origin: vec3<f32>, ray_dir: vec3<f32>) -> HitRecord {
    var closest: HitRecord;
    closest.valid = false;
    closest.t = 1e10;

    for (var i = 0u; i < config.num_primitives; i++) {
        let prim = primitives[i];
        var hit: HitRecord;

        switch (prim.ptype) {
            case 0u: { // PRIM_SHEET
                hit = intersect_sheet(ray_origin, ray_dir, prim, EPSILON, closest.t);
            }
            default: {
                continue;
            }
        }

        if (hit.valid && hit.t < closest.t) {
            closest = hit;
        }
    }

    return closest;
}

// ============================================================================
// Material interaction
// ============================================================================

const INTERACTION_ABSORBED: u32 = 0u;
const INTERACTION_REFLECTED: u32 = 1u;
const INTERACTION_TRANSMITTED: u32 = 2u;

struct Interaction {
    itype: u32,
    new_origin: vec3<f32>,
    new_dir: vec3<f32>,
    attenuation: f32,
}

fn interact_material(
    ray_dir: vec3<f32>,
    hit: HitRecord,
    mat: GpuMaterial,
) -> Interaction {
    var result: Interaction;
    result.attenuation = 1.0;

    switch (mat.mtype) {
        case 0u: { // MAT_ABSORBER
            result.itype = INTERACTION_ABSORBED;
            return result;
        }
        case 1u: { // MAT_DIFFUSE_REFL
            if (random_f32() > mat.reflectance) {
                result.itype = INTERACTION_ABSORBED;
                return result;
            }
            let new_dir = random_cosine_hemisphere(hit.normal);
            result.itype = INTERACTION_REFLECTED;
            result.new_origin = hit.point + new_dir * EPSILON;
            result.new_dir = new_dir;
            return result;
        }
        case 2u: { // MAT_SPECULAR_REFL
            if (random_f32() > mat.reflectance) {
                result.itype = INTERACTION_ABSORBED;
                return result;
            }
            let new_dir = reflect_dir(ray_dir, hit.normal);
            result.itype = INTERACTION_REFLECTED;
            result.new_origin = hit.point + new_dir * EPSILON;
            result.new_dir = normalize(new_dir);
            return result;
        }
        case 4u: { // MAT_CLEAR_TRANS
            var eta: f32;
            var cos_i: f32;
            if (hit.front_face) {
                eta = 1.0 / mat.ior;
                cos_i = clamp(-dot(ray_dir, hit.normal), 0.0, 1.0);
            } else {
                eta = mat.ior;
                cos_i = clamp(-dot(ray_dir, hit.normal), 0.0, 1.0);
            }
            let fr = max(fresnel_schlick(cos_i, eta), mat.min_reflectance);
            if (random_f32() < fr) {
                let refl = reflect_dir(ray_dir, hit.normal);
                result.itype = INTERACTION_REFLECTED;
                result.new_origin = hit.point + refl * EPSILON;
                result.new_dir = normalize(refl);
                return result;
            }
            let refr = refract_dir(ray_dir, hit.normal, eta);
            if (length(refr) < 0.5) { // TIR
                let refl = reflect_dir(ray_dir, hit.normal);
                result.itype = INTERACTION_REFLECTED;
                result.new_origin = hit.point + refl * EPSILON;
                result.new_dir = normalize(refl);
                return result;
            }
            let per_surface_tau = sqrt(mat.transmittance);
            result.itype = INTERACTION_TRANSMITTED;
            result.new_origin = hit.point + normalize(refr) * EPSILON;
            result.new_dir = normalize(refr);
            result.attenuation = per_surface_tau;
            return result;
        }
        case 5u: { // MAT_DIFFUSE_TRANS (thin-sheet model)
            var eta: f32;
            var cos_i: f32;
            if (hit.front_face) {
                eta = 1.0 / mat.ior;
                cos_i = clamp(-dot(ray_dir, hit.normal), 0.0, 1.0);
            } else {
                eta = mat.ior;
                cos_i = clamp(-dot(ray_dir, hit.normal), 0.0, 1.0);
            }
            // Entry Fresnel
            let fr = max(fresnel_schlick(cos_i, eta), mat.min_reflectance);
            if (random_f32() < fr) {
                let refl = reflect_dir(ray_dir, hit.normal);
                result.itype = INTERACTION_REFLECTED;
                result.new_origin = hit.point + refl * EPSILON;
                result.new_dir = normalize(refl);
                return result;
            }
            // Absorption (thin-sheet: probability = transmittance)
            let tau = exp(-mat.absorption_coeff * mat.thickness);
            if (random_f32() > tau) {
                result.itype = INTERACTION_ABSORBED;
                return result;
            }
            // Refract in
            let refr = refract_dir(ray_dir, hit.normal, eta);
            if (length(refr) < 0.5) {
                result.itype = INTERACTION_ABSORBED;
                return result;
            }
            // Angular diffusion
            var exit_dir: vec3<f32>;
            if (mat.scattering_coeff > 0.0) {
                exit_dir = sample_henyey_greenstein(normalize(refr), mat.asymmetry);
            } else {
                exit_dir = normalize(refr);
            }
            // Exit Fresnel
            var exit_eta: f32;
            if (hit.front_face) { exit_eta = mat.ior; }
            else { exit_eta = 1.0 / mat.ior; }
            let cos_exit = abs(dot(exit_dir, hit.normal));
            let exit_fr = max(fresnel_schlick(cos_exit, exit_eta), mat.min_reflectance);
            if (random_f32() < exit_fr) {
                result.itype = INTERACTION_ABSORBED; // simplified
                return result;
            }
            // Refract out
            let exit_normal = select(hit.normal, -hit.normal, hit.front_face);
            let exit_refr = refract_dir(exit_dir, exit_normal, exit_eta);
            if (length(exit_refr) < 0.5) {
                result.itype = INTERACTION_ABSORBED;
                return result;
            }
            let exit_point = hit.point + exit_normal * mat.thickness + normalize(exit_refr) * EPSILON;
            result.itype = INTERACTION_TRANSMITTED;
            result.new_origin = exit_point;
            result.new_dir = normalize(exit_refr);
            return result;
        }
        default: {
            result.itype = INTERACTION_ABSORBED;
            return result;
        }
    }
}

// ============================================================================
// Detector
// ============================================================================

fn direction_to_bin(dir: vec3<f32>) -> vec2<u32> {
    let gamma = acos(clamp(-dir.z, -1.0, 1.0));
    let gamma_deg = degrees(gamma);
    var c = degrees(atan2(dir.y, dir.x));
    if (c < 0.0) { c += 360.0; }
    let ci = u32(c / config.detector_c_res) % config.detector_c_bins;
    let gi = min(u32(gamma_deg / config.detector_g_res + 0.5), config.detector_g_bins - 1u);
    return vec2<u32>(ci, gi);
}

fn record_detector(dir: vec3<f32>, energy: f32) {
    let bin = direction_to_bin(dir);
    let idx = bin.x * config.detector_g_bins + bin.y;
    // Fixed-point: use 1000 (not 1M) to avoid u32 overflow at high photon counts.
    // Max per bin: 4,294,967 photons with energy=1.0 before overflow.
    let energy_fixed = u32(energy * 1000.0);
    atomicAdd(&detector_bins[idx], energy_fixed);
}

// ============================================================================
// Main kernel
// ============================================================================

@compute @workgroup_size(256)
fn trace_photons(@builtin(global_invocation_id) id: vec3<u32>) {
    let photon_idx = id.x;
    if (photon_idx >= config.num_photons) { return; }

    pcg_init(photon_idx, config.seed_offset);

    // Sample source direction
    var dir: vec3<f32>;
    switch (config.source_type) {
        case 0u: { dir = random_sphere(); }
        case 1u: { dir = random_cosine_hemisphere_down(); }
        default: { dir = random_sphere(); }
    }

    var origin = vec3<f32>(0.0, 0.0, 0.0);
    var energy: f32 = 1.0;

    // Trace loop
    for (var bounce = 0u; bounce < config.max_bounces; bounce++) {
        let hit = intersect_scene(origin, dir);

        if (!hit.valid) {
            // Escaped scene — record on detector
            record_detector(dir, energy);
            return;
        }

        let mat = materials[hit.material_id];
        let interaction = interact_material(dir, hit, mat);

        switch (interaction.itype) {
            case 0u: { // ABSORBED
                return;
            }
            case 1u: { // REFLECTED
                origin = interaction.new_origin;
                dir = interaction.new_dir;
                energy *= interaction.attenuation;
            }
            case 2u: { // TRANSMITTED
                origin = interaction.new_origin;
                dir = interaction.new_dir;
                energy *= interaction.attenuation;
            }
            default: { return; }
        }

        // Russian roulette
        if (energy < config.rr_threshold) {
            let survive = energy / config.rr_threshold;
            if (random_f32() > survive) { return; }
            energy = config.rr_threshold;
        }
    }
}
