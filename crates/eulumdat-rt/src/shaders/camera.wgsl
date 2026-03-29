// eulumdat-rt: Camera ray tracing compute shader
//
// Traces rays from a camera through the scene, accumulates color on a pixel buffer.
// Same physics as trace.wgsl — Fresnel, Snell, Beer-Lambert, Henyey-Greenstein.
// But instead of recording on a detector sphere, records on a 2D image plane.

struct CameraConfig {
    width: u32,
    height: u32,
    samples_per_pixel: u32,
    max_bounces: u32,
    // Camera position and orientation
    cam_pos: vec3<f32>,
    _pad0: f32,
    cam_forward: vec3<f32>,
    _pad1: f32,
    cam_right: vec3<f32>,
    _pad2: f32,
    cam_up: vec3<f32>,
    fov_tan: f32,       // tan(fov/2)
    // Scene
    num_primitives: u32,
    seed_offset: u32,
    // Source (emissive object)
    source_intensity: f32, // cd
    source_radius: f32,    // for soft shadows
}

// Reuse material/primitive structs from trace.wgsl
struct GpuPrimitive {
    ptype: u32,
    material_id: u32,
    _pad0: u32,
    _pad1: u32,
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

// Output: accumulated color per pixel (R, G, B, sample_count as f32)
@group(0) @binding(0) var<storage, read_write> pixels: array<atomic<u32>>;
@group(0) @binding(1) var<uniform> config: CameraConfig;
@group(0) @binding(2) var<storage, read> primitives: array<GpuPrimitive>;
@group(0) @binding(3) var<storage, read> materials: array<GpuMaterial>;

// ============================================================================
// RNG
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
// Vector math (same as trace.wgsl)
// ============================================================================

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

// ============================================================================
// Ray-Sheet intersection (same as trace.wgsl)
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
            case 0u: { hit = intersect_sheet(ray_origin, ray_dir, prim, EPSILON, closest.t); }
            default: { continue; }
        }
        if (hit.valid && hit.t < closest.t) {
            closest = hit;
        }
    }
    return closest;
}

// ============================================================================
// Path tracing: trace a camera ray and return color
// ============================================================================

fn trace_path(ray_origin: vec3<f32>, ray_dir: vec3<f32>) -> vec3<f32> {
    var origin = ray_origin;
    var dir = ray_dir;
    var throughput = vec3<f32>(1.0, 1.0, 1.0);
    var color = vec3<f32>(0.0, 0.0, 0.0);

    for (var bounce = 0u; bounce < config.max_bounces; bounce++) {
        let hit = intersect_scene(origin, dir);

        if (!hit.valid) {
            // Sky / environment: dark blue gradient
            let t = 0.5 * (dir.y + 1.0);
            let sky = mix(vec3<f32>(0.02, 0.02, 0.05), vec3<f32>(0.1, 0.15, 0.3), t);
            color += throughput * sky;
            break;
        }

        let mat = materials[hit.material_id];

        // Emissive check: if material has high reflectance, it glows
        // (simplified: source indicator would be a separate emissive object)

        // Material interaction for camera rays
        switch (mat.mtype) {
            case 0u: { // Absorber — black surface
                color += throughput * vec3<f32>(0.01, 0.01, 0.01);
                break;
            }
            case 1u: { // Diffuse reflector
                // Direct lighting from source
                let to_source = normalize(-origin);
                let n_dot_l = max(dot(hit.normal, to_source), 0.0);
                let diffuse_color = vec3<f32>(mat.reflectance, mat.reflectance, mat.reflectance);
                color += throughput * diffuse_color * n_dot_l * config.source_intensity * 0.01;

                // Continue bouncing
                let new_dir = random_cosine_hemisphere(hit.normal);
                origin = hit.point + new_dir * EPSILON;
                dir = new_dir;
                throughput *= diffuse_color;
            }
            case 4u: { // Clear transmitter
                var eta: f32;
                if (hit.front_face) { eta = 1.0 / mat.ior; }
                else { eta = mat.ior; }
                let cos_i = abs(dot(dir, hit.normal));
                let fr = max(fresnel_schlick(cos_i, eta), mat.min_reflectance);

                // Glass color: slightly tinted
                let glass_color = vec3<f32>(0.9, 0.95, 1.0);

                if (random_f32() < fr) {
                    // Reflect
                    let refl = reflect_dir(dir, hit.normal);
                    origin = hit.point + refl * EPSILON;
                    dir = normalize(refl);
                    throughput *= glass_color;
                } else {
                    // Transmit (simplified — no refraction for now)
                    origin = hit.point + dir * EPSILON * 10.0;
                    throughput *= glass_color * sqrt(mat.transmittance);
                }
            }
            case 5u: { // Diffuse transmitter (opal)
                let tau = exp(-mat.absorption_coeff * mat.thickness);
                let opal_color = vec3<f32>(0.9, 0.92, 0.95) * tau;

                if (random_f32() > tau) {
                    // Absorbed
                    color += throughput * vec3<f32>(0.01, 0.01, 0.02);
                    break;
                }

                // Transmitted with scattering
                let scattered = random_cosine_hemisphere(-hit.normal);
                origin = hit.point + scattered * EPSILON;
                dir = scattered;
                throughput *= opal_color;
            }
            default: {
                // Unknown material — show as magenta
                color += throughput * vec3<f32>(1.0, 0.0, 1.0);
                break;
            }
        }

        // Russian roulette
        let max_component = max(throughput.x, max(throughput.y, throughput.z));
        if (max_component < 0.01) {
            if (random_f32() > max_component * 10.0) { break; }
            throughput /= max_component * 10.0;
        }
    }

    return color;
}

// ============================================================================
// Main kernel: one invocation per pixel sample
// ============================================================================

@compute @workgroup_size(16, 16)
fn trace_camera(@builtin(global_invocation_id) id: vec3<u32>) {
    let px = id.x;
    let py = id.y;
    if (px >= config.width || py >= config.height) { return; }

    let pixel_idx = py * config.width + px;

    for (var sample = 0u; sample < config.samples_per_pixel; sample++) {
        pcg_init(pixel_idx * config.samples_per_pixel + sample, config.seed_offset);

        // Jittered pixel coordinates
        let u = (f32(px) + random_f32()) / f32(config.width);
        let v = (f32(py) + random_f32()) / f32(config.height);

        // Camera ray
        let aspect = f32(config.width) / f32(config.height);
        let screen_x = (2.0 * u - 1.0) * aspect * config.fov_tan;
        let screen_y = (1.0 - 2.0 * v) * config.fov_tan;

        let ray_dir = normalize(
            config.cam_forward + screen_x * config.cam_right + screen_y * config.cam_up
        );

        let color = trace_path(config.cam_pos, ray_dir);

        // Accumulate color as fixed-point RGB (scale by 1000)
        let base = pixel_idx * 4u;
        atomicAdd(&pixels[base + 0u], u32(clamp(color.x, 0.0, 100.0) * 1000.0));
        atomicAdd(&pixels[base + 1u], u32(clamp(color.y, 0.0, 100.0) * 1000.0));
        atomicAdd(&pixels[base + 2u], u32(clamp(color.z, 0.0, 100.0) * 1000.0));
        atomicAdd(&pixels[base + 3u], 1u); // sample count
    }
}
