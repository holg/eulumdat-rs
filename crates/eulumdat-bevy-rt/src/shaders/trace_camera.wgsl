// Photometric camera raytracer — Bevy compute shader entry point.
//
// Progressive path tracing through photometric scenes.
// Uses Bevy's ViewUniform for camera, accumulates on Rgba32Float texture.

#import eulumdat_rt::common::{
    GpuPrimitive, GpuMaterial, HitRecord,
    PI, EPSILON,
    pcg_init, random_f32,
    reflect_dir, fresnel_schlick, build_onb, random_cosine_hemisphere,
}
#import eulumdat_rt::intersect::intersect_primitive
#import eulumdat_rt::bvh::{BvhNode, intersect_scene_bvh}
#import eulumdat_rt::material::interact_material

#import bevy_render::view::View

// Bind group 0: Scene
@group(0) @binding(0) var<storage, read> primitives: array<GpuPrimitive>;
@group(0) @binding(1) var<storage, read> materials: array<GpuMaterial>;
@group(0) @binding(2) var<storage, read> bvh_nodes: array<BvhNode>;
@group(0) @binding(3) var<uniform> config: TracerConfig;
@group(0) @binding(4) var<storage, read> cdf_data: array<f32>;

struct TracerConfig {
    num_primitives: u32,
    max_bounces: u32,
    seed_offset: u32,
    _pad: u32,
}

// Bind group 1: Camera
@group(1) @binding(0) var accumulation: texture_storage_2d<rgba32float, read_write>;
@group(1) @binding(1) var output: texture_storage_2d<rgba16float, write_only>;
@group(1) @binding(2) var<uniform> view: View;

fn trace_path(ray_origin: vec3<f32>, ray_dir: vec3<f32>) -> vec3<f32> {
    var origin = ray_origin;
    var dir = ray_dir;
    var throughput = vec3<f32>(1.0, 1.0, 1.0);
    var color = vec3<f32>(0.0, 0.0, 0.0);

    for (var bounce = 0u; bounce < config.max_bounces; bounce++) {
        let hit = intersect_scene_bvh(
            origin, dir,
            &bvh_nodes, &primitives,
            config.num_primitives,
        );

        if (!hit.valid) {
            // Sky / environment
            let t = clamp(0.5 * (dir.y + 1.0), 0.0, 1.0);
            let sky = mix(
                vec3<f32>(0.03, 0.03, 0.05),
                vec3<f32>(0.08, 0.10, 0.18),
                t
            );
            color += throughput * sky;
            break;
        }

        let mat = materials[hit.material_id];
        let interaction = interact_material(dir, hit, mat);

        switch (interaction.itype) {
            case 0u: { // ABSORBED
                break;
            }
            case 1u, case 2u: { // REFLECTED or TRANSMITTED
                origin = interaction.new_origin;
                dir = interaction.new_dir;
                throughput *= interaction.attenuation;
            }
            default: { break; }
        }

        // Russian roulette after bounce 2
        if (bounce > 1u) {
            let p = max(throughput.x, max(throughput.y, throughput.z));
            if (p < 0.1) {
                if (random_f32() > p) { break; }
                throughput /= p;
            }
        }
    }

    return color;
}

@compute @workgroup_size(8, 8)
fn trace_camera(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(accumulation);
    if (id.x >= dims.x || id.y >= dims.y) { return; }

    let pixel_idx = id.y * dims.x + id.x;
    pcg_init(pixel_idx, config.seed_offset);

    // Camera ray from Bevy's View uniform
    let u = (f32(id.x) + random_f32()) / f32(dims.x);
    let v = (f32(id.y) + random_f32()) / f32(dims.y);

    // NDC to world ray using inverse view-projection
    let ndc = vec4<f32>(u * 2.0 - 1.0, 1.0 - v * 2.0, 0.0, 1.0);
    let world_pos = view.inverse_clip_from_world * ndc;
    let target = world_pos.xyz / world_pos.w;
    let cam_pos = view.world_position.xyz;
    let ray_dir = normalize(target - cam_pos);

    let color = trace_path(cam_pos, ray_dir);

    // Accumulate
    let prev = textureLoad(accumulation, vec2<i32>(id.xy));
    let count = prev.w + 1.0;
    let accumulated = vec4<f32>(prev.xyz + color, count);
    textureStore(accumulation, vec2<i32>(id.xy), accumulated);

    // Output averaged color
    let avg = accumulated.xyz / count;
    // Simple Reinhard tonemap
    let mapped = avg / (avg + vec3<f32>(1.0));
    textureStore(output, vec2<i32>(id.xy), vec4<f32>(mapped, 1.0));
}
