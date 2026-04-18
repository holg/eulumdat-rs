//! CPU-side BVH construction for software ray traversal.

use super::types::BvhNode;
use eulumdat_rt::GpuPrimitive;

/// Build a simple BVH from a list of primitives.
///
/// For small scenes (< 16 primitives), returns a single leaf node.
/// For larger scenes, recursively splits using the SAH heuristic.
pub fn build_bvh(primitives: &[GpuPrimitive]) -> (Vec<BvhNode>, Vec<GpuPrimitive>) {
    if primitives.is_empty() {
        return (vec![BvhNode::default()], vec![]);
    }

    let mut sorted_prims = primitives.to_vec();
    let mut nodes = Vec::new();
    build_recursive(&mut nodes, &mut sorted_prims, 0, primitives.len());
    (nodes, sorted_prims)
}

fn build_recursive(
    nodes: &mut Vec<BvhNode>,
    primitives: &mut [GpuPrimitive],
    start: usize,
    end: usize,
) -> usize {
    let count = end - start;
    let node_idx = nodes.len();
    let (aabb_min, aabb_max) = compute_aabb(&primitives[start..end]);

    // Leaf node for small groups
    if count <= 4 {
        nodes.push(BvhNode {
            aabb_min,
            left_or_prim: start as u32,
            aabb_max,
            right_or_count: count as u32 | 0x80000000, // High bit = leaf flag
        });
        return node_idx;
    }

    // Find best split axis (longest extent)
    let extent = [
        aabb_max[0] - aabb_min[0],
        aabb_max[1] - aabb_min[1],
        aabb_max[2] - aabb_min[2],
    ];
    let axis = if extent[0] >= extent[1] && extent[0] >= extent[2] {
        0
    } else if extent[1] >= extent[2] {
        1
    } else {
        2
    };

    // Sort by centroid on the split axis
    let slice = &mut primitives[start..end];
    slice.sort_by(|a, b| {
        let ca = prim_centroid(a)[axis];
        let cb = prim_centroid(b)[axis];
        ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
    });

    let mid = start + count / 2;

    // Placeholder node — will fill in children
    nodes.push(BvhNode {
        aabb_min,
        left_or_prim: 0,
        aabb_max,
        right_or_count: 0,
    });

    let left = build_recursive(nodes, primitives, start, mid);
    let right = build_recursive(nodes, primitives, mid, end);

    nodes[node_idx].left_or_prim = left as u32;
    nodes[node_idx].right_or_count = right as u32;

    node_idx
}

fn compute_aabb(primitives: &[GpuPrimitive]) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];

    for prim in primitives {
        let c = prim_centroid(prim);
        // Expand AABB around primitive (approximate with centroid + half-extents)
        let half = prim_half_extent(prim);
        for i in 0..3 {
            min[i] = min[i].min(c[i] - half[i]);
            max[i] = max[i].max(c[i] + half[i]);
        }
    }

    (min, max)
}

fn prim_centroid(prim: &GpuPrimitive) -> [f32; 3] {
    // Sheet center is params[0..3]
    [prim.params[0], prim.params[1], prim.params[2]]
}

fn prim_half_extent(prim: &GpuPrimitive) -> [f32; 3] {
    // For sheets: half_w (params[9]) and half_h (params[10])
    // Approximate AABB by max of half_w and half_h in all axes
    let hw = prim.params[9];
    let hh = prim.params[10];
    let r = hw.max(hh);
    [r, r, r]
}

impl Default for BvhNode {
    fn default() -> Self {
        Self {
            aabb_min: [0.0; 3],
            left_or_prim: 0,
            aabb_max: [0.0; 3],
            right_or_count: 0x80000000, // Empty leaf
        }
    }
}
