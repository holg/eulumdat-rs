// eulumdat-rt: Monte Carlo photon tracing compute shader
//
// Phase 1: Isotropic source in free space — validates pipeline against CPU reference.
// Each invocation traces one photon: sample direction, record on detector.

struct TraceConfig {
    detector_c_bins: u32,
    detector_g_bins: u32,
    detector_c_res: f32,    // degrees per C-bin
    detector_g_res: f32,    // degrees per G-bin
    seed_offset: u32,       // changes per dispatch
    num_photons: u32,       // total this dispatch
    source_type: u32,       // 0=isotropic, 1=lambertian, 2=from_lvk
    source_flux: f32,
}

@group(0) @binding(0) var<storage, read_write> detector_bins: array<atomic<u32>>;
@group(0) @binding(1) var<uniform> config: TraceConfig;

// ============================================================================
// RNG: PCG (Permuted Congruential Generator)
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
// Direction sampling
// ============================================================================

const PI: f32 = 3.14159265358979323846;

/// Uniform random direction on the unit sphere
fn random_sphere() -> vec3<f32> {
    let z = 2.0 * random_f32() - 1.0;
    let r = sqrt(max(1.0 - z * z, 0.0));
    let phi = 2.0 * PI * random_f32();
    return vec3<f32>(r * cos(phi), r * sin(phi), z);
}

/// Cosine-weighted hemisphere around normal (0, 0, -1) = nadir
fn random_cosine_hemisphere_down() -> vec3<f32> {
    let u1 = random_f32();
    let u2 = random_f32();
    let r = sqrt(u1);
    let theta = 2.0 * PI * u2;
    let x = r * cos(theta);
    let y = r * sin(theta);
    let z = -sqrt(max(1.0 - u1, 0.0)); // negative Z = downward
    return vec3<f32>(x, y, z);
}

// ============================================================================
// Detector
// ============================================================================

/// Convert direction to CIE photometric coordinates
/// gamma = 0 → nadir (-Z), gamma = 180 → zenith (+Z)
/// C = 0 → +X, C = 90 → +Y
fn direction_to_bin(dir: vec3<f32>) -> vec2<u32> {
    let gamma = acos(clamp(-dir.z, -1.0, 1.0)); // radians, 0 = down
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

    // Fixed-point: energy * 1,000,000 as u32
    let energy_fixed = u32(energy * 1000000.0);
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
        case 0u: { // Isotropic
            dir = random_sphere();
        }
        case 1u: { // Lambertian (downward hemisphere)
            dir = random_cosine_hemisphere_down();
        }
        default: {
            dir = random_sphere();
        }
    }

    // Phase 1: free space — no geometry, photon goes straight to detector
    record_detector(dir, 1.0);
}
