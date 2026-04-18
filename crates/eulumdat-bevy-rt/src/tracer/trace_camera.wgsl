// Monolithic photometric camera raytracer for Bevy.
// Adapted from eulumdat-rt camera.wgsl — proven physics, no shader imports.
// Outputs to texture_storage_2d instead of atomic pixel buffer.

struct CameraConfig {
    width: u32,
    height: u32,
    samples_per_pixel: u32,
    max_bounces: u32,
    cam_pos: vec3<f32>,
    pad0: f32,
    cam_forward: vec3<f32>,
    pad1: f32,
    cam_right: vec3<f32>,
    pad2: f32,
    cam_up: vec3<f32>,
    fov_tan: f32,
    num_primitives: u32,
    seed_offset: u32,
    source_intensity: f32,
    source_radius: f32,
    source_pos: vec3<f32>,
    pad3: f32,
    lvk_c_steps: u32,
    lvk_g_steps: u32,
    lvk_g_max: f32,
    lvk_max_intensity: f32,
}

struct GpuPrimitive {
    ptype: u32,
    material_id: u32,
    pad0: u32,
    pad1: u32,
    params: array<f32, 12>,
}

struct GpuMaterial {
    mtype: u32,
    pad0: u32,
    pad1: u32,
    pad2: u32,
    reflectance: f32,
    ior: f32,
    transmittance: f32,
    min_reflectance: f32,
    absorption_coeff: f32,
    scattering_coeff: f32,
    asymmetry: f32,
    thickness: f32,
}

// Bind group 0: Scene
@group(0) @binding(0) var<storage, read> primitives: array<GpuPrimitive>;
@group(0) @binding(1) var<storage, read> materials: array<GpuMaterial>;
@group(0) @binding(2) var<storage, read> bvh_reserved: array<u32>;
@group(0) @binding(3) var<uniform> scene_pad: vec4<u32>;
@group(0) @binding(4) var<storage, read> lvk_data: array<f32>;

// Bind group 1: Camera output
@group(1) @binding(0) var output: texture_storage_2d<rgba16float, write>;
@group(1) @binding(1) var<uniform> config: CameraConfig;

// RNG
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
fn random_f32() -> f32 { return f32(pcg_next()) / 4294967295.0; }

const PI: f32 = 3.14159265358979323846;
const EPSILON: f32 = 1e-5;

fn reflect_dir(incoming: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    return incoming - 2.0 * dot(incoming, normal) * normal;
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
fn random_cosine_hemisphere(normal: vec3<f32>) -> vec3<f32> {
    let u1 = random_f32();
    let u2 = random_f32();
    let r = sqrt(u1);
    let theta = 2.0 * PI * u2;
    let local = vec3<f32>(r * cos(theta), r * sin(theta), sqrt(max(1.0 - u1, 0.0)));
    let onb = build_onb(normal);
    return normalize(onb * local);
}

// Intersection
struct HitRecord { t: f32, point: vec3<f32>, normal: vec3<f32>, front_face: bool, material_id: u32, valid: bool, }

fn intersect_sheet(ray_origin: vec3<f32>, ray_dir: vec3<f32>, prim: GpuPrimitive, t_min: f32, t_max: f32) -> HitRecord {
    var hit: HitRecord; hit.valid = false;
    let center = vec3<f32>(prim.params[0], prim.params[1], prim.params[2]);
    let normal = vec3<f32>(prim.params[3], prim.params[4], prim.params[5]);
    let u_axis = vec3<f32>(prim.params[6], prim.params[7], prim.params[8]);
    let half_w = prim.params[9]; let half_h = prim.params[10];
    let denom = dot(ray_dir, normal);
    if (abs(denom) < EPSILON) { return hit; }
    let t = dot(center - ray_origin, normal) / denom;
    if (t < t_min || t > t_max) { return hit; }
    let p = ray_origin + t * ray_dir;
    let local = p - center;
    let v_axis = cross(normal, u_axis);
    let u = dot(local, u_axis); let v = dot(local, v_axis);
    if (abs(u) > half_w || abs(v) > half_h) { return hit; }
    hit.valid = true; hit.t = t; hit.point = p; hit.material_id = prim.material_id;
    hit.front_face = denom < 0.0;
    if (hit.front_face) { hit.normal = normal; } else { hit.normal = -normal; }
    return hit;
}

fn intersect_scene(ray_origin: vec3<f32>, ray_dir: vec3<f32>) -> HitRecord {
    var closest: HitRecord; closest.valid = false; closest.t = 1e10;
    for (var i = 0u; i < config.num_primitives; i++) {
        let prim = primitives[i];
        var hit: HitRecord;
        switch (prim.ptype) {
            case 0u: { hit = intersect_sheet(ray_origin, ray_dir, prim, EPSILON, closest.t); }
            default: { continue; }
        }
        if (hit.valid && hit.t < closest.t) { closest = hit; }
    }
    return closest;
}

// LVK intensity lookup
fn lvk_intensity(dir_from_source: vec3<f32>) -> f32 {
    if (config.lvk_g_steps == 0u || config.lvk_c_steps == 0u) { return 1.0; }
    let gamma_rad = acos(clamp(-dir_from_source.y, -1.0, 1.0));
    let gamma_deg = degrees(gamma_rad);
    if (gamma_deg > config.lvk_g_max) { return 0.0; }
    var c_deg = degrees(atan2(dir_from_source.z, dir_from_source.x));
    if (c_deg < 0.0) { c_deg += 360.0; }
    let ci = u32(c_deg / (360.0 / f32(config.lvk_c_steps))) % config.lvk_c_steps;
    let gi = min(u32(gamma_deg / (config.lvk_g_max / f32(config.lvk_g_steps - 1u)) + 0.5), config.lvk_g_steps - 1u);
    let idx = ci * config.lvk_g_steps + gi;
    return lvk_data[idx] / max(config.lvk_max_intensity, 0.001);
}

fn sample_light_point() -> vec3<f32> {
    let r = config.source_radius;
    let u = random_f32(); let v = random_f32();
    let theta = 2.0 * PI * u; let phi = acos(2.0 * v - 1.0);
    return config.source_pos + vec3<f32>(r * sin(phi) * cos(theta), r * sin(phi) * sin(theta), r * cos(phi));
}

fn direct_lighting(hit_point: vec3<f32>, hit_normal: vec3<f32>) -> vec3<f32> {
    let light_pos = sample_light_point();
    let to_light = light_pos - hit_point;
    let dist2 = dot(to_light, to_light); let dist = sqrt(dist2);
    let light_dir = to_light / dist;
    let shadow_hit = intersect_scene(hit_point + light_dir * EPSILON * 2.0, light_dir);
    var shadow_atten = 1.0;
    if (shadow_hit.valid && shadow_hit.t < dist - EPSILON * 4.0) {
        let shadow_mat = materials[shadow_hit.material_id];
        if (shadow_mat.mtype == 4u) { shadow_atten = shadow_mat.transmittance; }
        else if (shadow_mat.mtype == 5u) { shadow_atten = exp(-shadow_mat.absorption_coeff * shadow_mat.thickness) * 0.5; }
        else { shadow_atten = 0.0; }
    }
    let n_dot_l = max(dot(hit_normal, light_dir), 0.0);
    let dir_from_source = -light_dir;
    let lvk_weight = lvk_intensity(dir_from_source);
    let light_color = vec3<f32>(1.0, 0.95, 0.85) * config.source_intensity * lvk_weight;
    let light_area = 4.0 * PI * config.source_radius * config.source_radius;
    return light_color * n_dot_l * light_area / (4.0 * PI * dist2) * shadow_atten;
}

fn trace_path(ray_origin: vec3<f32>, ray_dir: vec3<f32>) -> vec3<f32> {
    var origin = ray_origin; var dir = ray_dir;
    var throughput = vec3<f32>(1.0); var color = vec3<f32>(0.0);
    for (var bounce = 0u; bounce < config.max_bounces; bounce++) {
        let hit = intersect_scene(origin, dir);
        if (!hit.valid) {
            let t = clamp(0.5 * (dir.y + 1.0), 0.0, 1.0);
            color += throughput * mix(vec3<f32>(0.03, 0.03, 0.05), vec3<f32>(0.08, 0.10, 0.18), t);
            break;
        }
        let to_source = config.source_pos - hit.point;
        if (length(to_source) < config.source_radius * 1.5 && bounce == 0u) {
            color += throughput * vec3<f32>(1.0, 0.95, 0.85) * config.source_intensity * 0.5;
            break;
        }
        let mat = materials[hit.material_id];
        switch (mat.mtype) {
            case 0u: { break; }
            case 1u: {
                let albedo = vec3<f32>(mat.reflectance);
                color += throughput * albedo * direct_lighting(hit.point, hit.normal) / PI;
                let new_dir = random_cosine_hemisphere(hit.normal);
                origin = hit.point + new_dir * EPSILON * 2.0; dir = new_dir; throughput *= albedo;
            }
            case 2u: {
                let refl = reflect_dir(dir, hit.normal);
                origin = hit.point + refl * EPSILON * 2.0; dir = normalize(refl); throughput *= mat.reflectance;
            }
            case 4u: {
                var eta: f32; var n: vec3<f32>;
                if (hit.front_face) { eta = 1.0 / mat.ior; n = hit.normal; }
                else { eta = mat.ior; n = -hit.normal; }
                let cos_i = abs(dot(dir, n));
                let fr = max(fresnel_schlick(cos_i, eta), mat.min_reflectance);
                if (random_f32() < fr) {
                    let refl = reflect_dir(dir, n);
                    origin = hit.point + refl * EPSILON * 2.0; dir = normalize(refl);
                } else {
                    origin = hit.point + dir * EPSILON * 10.0; throughput *= sqrt(mat.transmittance);
                }
                throughput *= vec3<f32>(0.95, 0.97, 1.0);
            }
            case 5u: {
                let tau = exp(-mat.absorption_coeff * mat.thickness);
                if (random_f32() > tau) { color += throughput * vec3<f32>(0.02, 0.015, 0.01); break; }
                color += throughput * direct_lighting(hit.point, -hit.normal) * tau * 0.3 / PI;
                let scattered = random_cosine_hemisphere(-hit.normal);
                origin = hit.point - hit.normal * (mat.thickness + EPSILON);
                dir = scattered; throughput *= vec3<f32>(0.92, 0.94, 0.96) * tau;
            }
            default: { color += throughput * vec3<f32>(1.0, 0.0, 1.0); break; }
        }
        if (bounce > 1u) {
            let p = max(throughput.x, max(throughput.y, throughput.z));
            if (p < 0.1) { if (random_f32() > p) { break; } throughput /= p; }
        }
    }
    return color;
}

@compute @workgroup_size(8, 8)
fn trace_camera(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= config.width || id.y >= config.height) { return; }
    let pixel_idx = id.y * config.width + id.x;
    pcg_init(pixel_idx, config.seed_offset);

    let u = (f32(id.x) + random_f32()) / f32(config.width);
    let v = (f32(id.y) + random_f32()) / f32(config.height);
    let aspect = f32(config.width) / f32(config.height);
    let screen_x = (2.0 * u - 1.0) * aspect * config.fov_tan;
    let screen_y = (1.0 - 2.0 * v) * config.fov_tan;
    let ray_dir = normalize(config.cam_forward + screen_x * config.cam_right + screen_y * config.cam_up);

    let color = trace_path(config.cam_pos, ray_dir);

    // Reinhard + gamma
    let mapped = pow(color / (color + vec3<f32>(1.0)), vec3<f32>(1.0 / 2.2));
    textureStore(output, vec2<i32>(id.xy), vec4<f32>(mapped, 1.0));
}
