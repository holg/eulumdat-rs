// Photometric detector raytracer — goniophotometer mode.
//
// Traces photons from source through scene, records on spherical detector.
// Matches CIE 171:2006 photon tracing methodology.

#import eulumdat_rt::common::{
    GpuPrimitive, GpuMaterial, HitRecord,
    PI, EPSILON,
    pcg_init, random_f32,
    random_sphere, random_cosine_hemisphere,
}
#import eulumdat_rt::intersect::intersect_primitive
#import eulumdat_rt::bvh::{BvhNode, intersect_scene_bvh}
#import eulumdat_rt::material::interact_material

struct DetectorConfig {
    detector_c_bins: u32,
    detector_g_bins: u32,
    detector_c_res: f32,
    detector_g_res: f32,
    seed_offset: u32,
    num_photons: u32,
    source_type: u32,    // 0=isotropic, 1=lambertian, 2=from_lvk, 3=area
    source_flux: f32,
    num_primitives: u32,
    max_bounces: u32,
    rr_threshold: f32,
    _pad: u32,
}

// Bind group 0: Scene
@group(0) @binding(0) var<storage, read> primitives: array<GpuPrimitive>;
@group(0) @binding(1) var<storage, read> materials: array<GpuMaterial>;
@group(0) @binding(2) var<storage, read> bvh_nodes: array<BvhNode>;

// Bind group 1: Detector
@group(1) @binding(0) var<storage, read_write> detector_bins: array<atomic<u32>>;
@group(1) @binding(1) var<uniform> config: DetectorConfig;

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
    let energy_fixed = u32(energy * 1000.0);
    atomicAdd(&detector_bins[idx], energy_fixed);
}

@compute @workgroup_size(256)
fn trace_photons(@builtin(global_invocation_id) id: vec3<u32>) {
    let photon_idx = id.x;
    if (photon_idx >= config.num_photons) { return; }

    pcg_init(photon_idx, config.seed_offset);

    // Sample source direction
    var dir: vec3<f32>;
    var origin = vec3<f32>(0.0, 0.0, 0.0);
    switch (config.source_type) {
        case 0u: { dir = random_sphere(); }
        case 1u: {
            let u1 = random_f32();
            let u2 = random_f32();
            let r = sqrt(u1);
            let theta = 2.0 * PI * u2;
            dir = vec3<f32>(r * cos(theta), r * sin(theta), -sqrt(max(1.0 - u1, 0.0)));
        }
        default: { dir = random_sphere(); }
    }
    var energy: f32 = 1.0;

    // Trace loop
    for (var bounce = 0u; bounce < config.max_bounces; bounce++) {
        let hit = intersect_scene_bvh(
            origin, dir,
            &bvh_nodes, &primitives,
            config.num_primitives,
        );

        if (!hit.valid) {
            record_detector(dir, energy);
            return;
        }

        let mat = materials[hit.material_id];
        let interaction = interact_material(dir, hit, mat);

        switch (interaction.itype) {
            case 0u: { return; } // ABSORBED
            case 1u, case 2u: { // REFLECTED or TRANSMITTED
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
