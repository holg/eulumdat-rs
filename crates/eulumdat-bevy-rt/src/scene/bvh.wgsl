#define_import_path eulumdat_rt::bvh

#import eulumdat_rt::common::{GpuPrimitive, HitRecord, EPSILON}
#import eulumdat_rt::intersect::intersect_primitive

struct BvhNode {
    aabb_min: vec3<f32>,
    left_or_prim: u32,
    aabb_max: vec3<f32>,
    right_or_count: u32,
}

fn ray_aabb(origin: vec3<f32>, dir: vec3<f32>, aabb_min: vec3<f32>, aabb_max: vec3<f32>, t_max: f32) -> bool {
    let inv_dir = 1.0 / dir;
    let t1 = (aabb_min - origin) * inv_dir;
    let t2 = (aabb_max - origin) * inv_dir;
    let t_near = min(t1, t2);
    let t_far = max(t1, t2);
    let enter = max(max(t_near.x, t_near.y), t_near.z);
    let exit = min(min(t_far.x, t_far.y), t_far.z);
    return enter <= exit && exit >= 0.0 && enter < t_max;
}

fn is_leaf(node: BvhNode) -> bool {
    return (node.right_or_count & 0x80000000u) != 0u;
}

fn leaf_count(node: BvhNode) -> u32 {
    return node.right_or_count & 0x7FFFFFFFu;
}

fn intersect_scene_bvh(
    origin: vec3<f32>,
    dir: vec3<f32>,
    bvh_nodes: ptr<storage, array<BvhNode>>,
    primitives: ptr<storage, array<GpuPrimitive>>,
    num_primitives: u32,
) -> HitRecord {
    var closest: HitRecord;
    closest.valid = false;
    closest.t = 1e10;

    // For very small scenes, linear scan is faster than BVH
    if (num_primitives <= 16u) {
        for (var i = 0u; i < num_primitives; i++) {
            let hit = intersect_primitive(origin, dir, (*primitives)[i], EPSILON, closest.t);
            if (hit.valid && hit.t < closest.t) {
                closest = hit;
            }
        }
        return closest;
    }

    // Stack-based BVH traversal
    var stack: array<u32, 32>;
    var sp = 0u;
    stack[0] = 0u;
    sp = 1u;

    while (sp > 0u) {
        sp -= 1u;
        let node_idx = stack[sp];
        let node = (*bvh_nodes)[node_idx];

        if (!ray_aabb(origin, dir, node.aabb_min, node.aabb_max, closest.t)) {
            continue;
        }

        if (is_leaf(node)) {
            let count = leaf_count(node);
            for (var i = 0u; i < count; i++) {
                let prim_idx = node.left_or_prim + i;
                let hit = intersect_primitive(origin, dir, (*primitives)[prim_idx], EPSILON, closest.t);
                if (hit.valid && hit.t < closest.t) {
                    closest = hit;
                }
            }
        } else {
            stack[sp] = node.left_or_prim;
            sp += 1u;
            stack[sp] = node.right_or_count;
            sp += 1u;
        }
    }

    return closest;
}
