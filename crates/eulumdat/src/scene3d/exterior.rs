//! Exterior scene builder for the Area Lighting Designer.
//!
//! Builds a 3D face list representing ground plane with illuminance heatmap,
//! poles, arms, luminaire heads, and optional light cones.

use crate::area::{AreaResult, LuminairePlace};
use crate::diagram::color::heatmap_color;

use super::SceneFace;

/// Build the exterior scene face list from area calculation results.
///
/// The returned faces are unsorted — call `render_scene_svg()` to z-sort and render.
pub fn build_exterior_scene(
    result: &AreaResult,
    placements: &[LuminairePlace],
    show_cones: bool,
) -> Vec<SceneFace> {
    let mut faces = Vec::new();
    let aw = result.area_width;
    let ad = result.area_depth;
    let max_lux = result.max_lux.max(0.001);
    let n = result.grid_resolution;

    // Center the scene around origin for better projection
    let cx = aw / 2.0;
    let cy = ad / 2.0;

    // ─── Ground heatmap ─────────────────────────────────────────────
    // Cap grid cells for SVG performance
    let step = if n > 20 { n / 20 } else { 1 };
    let grid_n = n.div_ceil(step);

    let dx = aw / grid_n as f64;
    let dy = ad / grid_n as f64;

    for row in 0..grid_n {
        for col in 0..grid_n {
            let src_row = (row * step).min(n - 1);
            let src_col = (col * step).min(n - 1);

            let is_inside = result
                .mask
                .as_ref()
                .map(|m| m[src_row][src_col])
                .unwrap_or(true);

            let lux = result.lux_grid[src_row][src_col];
            let normalized = lux / max_lux;
            let color = heatmap_color(normalized);
            let fill = color.to_rgb_string();
            let opacity = if is_inside { 1.0 } else { 0.15 };

            let x0 = col as f64 * dx - cx;
            let x1 = x0 + dx;
            let y0 = row as f64 * dy - cy;
            let y1 = y0 + dy;

            faces.push(
                SceneFace::quad(
                    (x0, y0, 0.0),
                    (x1, y0, 0.0),
                    (x1, y1, 0.0),
                    (x0, y1, 0.0),
                    &fill,
                    &fill,
                    opacity,
                )
                .with_stroke_width(0.2),
            );
        }
    }

    // Ground outline
    faces.push(
        SceneFace::quad(
            (-cx, -cy, 0.0),
            (cx, -cy, 0.0),
            (cx, cy, 0.0),
            (-cx, cy, 0.0),
            "none",
            "#666",
            1.0,
        )
        .with_stroke_width(1.5),
    );

    // ─── Ground slab edges (gives 3D depth) ──────────────────────────
    let slab_h = 0.15; // visual thickness of the ground slab
                       // Front edge (y = -cy)
    faces.push(
        SceneFace::quad(
            (-cx, -cy, -slab_h),
            (cx, -cy, -slab_h),
            (cx, -cy, 0.0),
            (-cx, -cy, 0.0),
            "#aab",
            "#888",
            0.7,
        )
        .with_stroke_width(0.5),
    );
    // Right edge (x = cx)
    faces.push(
        SceneFace::quad(
            (cx, -cy, -slab_h),
            (cx, cy, -slab_h),
            (cx, cy, 0.0),
            (cx, -cy, 0.0),
            "#99a",
            "#888",
            0.7,
        )
        .with_stroke_width(0.5),
    );

    // ─── Dimension labels ─────────────────────────────────────────────
    // Width (bottom edge)
    faces.push(
        SceneFace::quad(
            (0.0, -cy - 0.4, 0.0),
            (0.0, -cy - 0.4, 0.0),
            (0.0, -cy - 0.4, 0.0),
            (0.0, -cy - 0.4, 0.0),
            "none",
            "#555",
            1.0,
        )
        .with_label(&format!("{:.0}m", aw), 10.0),
    );
    // Depth (right edge)
    faces.push(
        SceneFace::quad(
            (cx + 0.4, 0.0, 0.0),
            (cx + 0.4, 0.0, 0.0),
            (cx + 0.4, 0.0, 0.0),
            (cx + 0.4, 0.0, 0.0),
            "none",
            "#555",
            1.0,
        )
        .with_label(&format!("{:.0}m", ad), 10.0),
    );

    // ─── Poles, arms, luminaires ────────────────────────────────────
    let pole_hw = 0.08; // half-width of pole in meters (visual only)

    for lum in placements {
        let pos: (f64, f64) = lum.effective_position();
        let px = pos.0 - cx;
        let py = pos.1 - cy;
        let h = lum.mounting_height;

        // Pole: thin vertical quad
        faces.push(
            SceneFace::quad(
                (px - pole_hw, py, 0.0),
                (px + pole_hw, py, 0.0),
                (px + pole_hw, py, h),
                (px - pole_hw, py, h),
                "#888",
                "#666",
                0.7,
            )
            .with_stroke_width(0.5),
        );
        // Second crossed quad for depth
        faces.push(
            SceneFace::quad(
                (px, py - pole_hw, 0.0),
                (px, py + pole_hw, 0.0),
                (px, py + pole_hw, h),
                (px, py - pole_hw, h),
                "#888",
                "#666",
                0.7,
            )
            .with_stroke_width(0.5),
        );

        // Arm segment (if arm_length > 0)
        if lum.arm_length > 0.0 {
            let base_x = lum.x - cx;
            let base_y = lum.y - cy;
            let arm_hw = 0.04;
            faces.push(
                SceneFace::quad(
                    (base_x, base_y - arm_hw, h),
                    (base_x, base_y + arm_hw, h),
                    (px, py + arm_hw, h),
                    (px, py - arm_hw, h),
                    "#999",
                    "#777",
                    0.6,
                )
                .with_stroke_width(0.3),
            );
        }

        // Pole base plate
        let base_r = 0.15;
        faces.push(
            SceneFace::quad(
                (px - base_r, py - base_r, 0.0),
                (px + base_r, py - base_r, 0.0),
                (px + base_r, py + base_r, 0.0),
                (px - base_r, py + base_r, 0.0),
                "#666",
                "#555",
                0.8,
            )
            .with_stroke_width(0.3),
        );

        // Luminaire head
        let lum_size = 0.4;
        faces.push(SceneFace::quad(
            (px - lum_size, py - lum_size * 0.5, h),
            (px + lum_size, py - lum_size * 0.5, h),
            (px + lum_size, py + lum_size * 0.5, h),
            (px - lum_size, py + lum_size * 0.5, h),
            "rgba(255,200,50,0.9)",
            "rgb(200,160,30)",
            1.0,
        ));
        // Luminaire head side (gives depth)
        let head_h = 0.08;
        faces.push(
            SceneFace::quad(
                (px - lum_size, py - lum_size * 0.5, h - head_h),
                (px + lum_size, py - lum_size * 0.5, h - head_h),
                (px + lum_size, py - lum_size * 0.5, h),
                (px - lum_size, py - lum_size * 0.5, h),
                "rgb(180,140,20)",
                "rgb(160,120,10)",
                0.8,
            )
            .with_stroke_width(0.3),
        );

        // Light cone (optional): 4 triangles forming a pyramid to ground
        if show_cones {
            let cone_r = h * 0.6; // approximate beam spread
            let cone_faces = [
                [
                    (px, py, h),
                    (px - cone_r, py - cone_r, 0.0),
                    (px + cone_r, py - cone_r, 0.0),
                ],
                [
                    (px, py, h),
                    (px - cone_r, py + cone_r, 0.0),
                    (px + cone_r, py + cone_r, 0.0),
                ],
                [
                    (px, py, h),
                    (px - cone_r, py - cone_r, 0.0),
                    (px - cone_r, py + cone_r, 0.0),
                ],
                [
                    (px, py, h),
                    (px + cone_r, py - cone_r, 0.0),
                    (px + cone_r, py + cone_r, 0.0),
                ],
            ];
            for tri in &cone_faces {
                faces.push(SceneFace {
                    vertices: vec![tri[0], tri[1], tri[2]],
                    fill: "rgba(255,220,80,0.15)".to_string(),
                    stroke: "rgba(255,200,50,0.25)".to_string(),
                    stroke_width: 0.3,
                    opacity: 0.3,
                    dash: None,
                    label: None,
                });
            }
        }
    }

    faces
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_exterior_basic() {
        let result = AreaResult {
            lux_grid: vec![vec![10.0; 5]; 5],
            min_lux: 10.0,
            avg_lux: 10.0,
            max_lux: 10.0,
            uniformity_min_avg: 1.0,
            uniformity_avg_min: 1.0,
            uniformity_min_max: 1.0,
            area_width: 20.0,
            area_depth: 15.0,
            grid_resolution: 5,
            mask: None,
        };
        let placements = vec![LuminairePlace::simple(0, 10.0, 7.5, 8.0)];
        let faces = build_exterior_scene(&result, &placements, false);
        // 5x5 ground cells + 1 outline + 2 pole quads + 1 luminaire head = 29
        assert!(faces.len() >= 25, "got {} faces", faces.len());
    }
}
