//! Interior scene builder for the Zonal Cavity Designer.
//!
//! Builds a 3D face list representing a room with walls, ceiling, floor,
//! luminaire grid, workplane (optional heatmap), and cavity visualization.

use crate::diagram::color::heatmap_color;
use crate::zonal::{CavityResults, LuminaireLayout, PpbResult, Reflectances, Room};

use super::{reflectance_to_rgb, SceneFace};

/// Build the interior scene face list from zonal cavity results.
///
/// The scene is centered at the room's center for better projection.
pub fn build_interior_scene(
    room: &Room,
    layout: &LuminaireLayout,
    reflectances: &Reflectances,
    _cavity: &CavityResults,
    ppb: Option<&PpbResult>,
    show_cavities: bool,
    show_cones: bool,
) -> Vec<SceneFace> {
    let mut faces = Vec::new();
    let l = room.length;
    let w = room.width;
    let h = room.height;

    // Center the room around origin
    let hx = l / 2.0;
    let hy = w / 2.0;

    let floor_color = reflectance_to_rgb(reflectances.floor);
    let wall_color = reflectance_to_rgb(reflectances.wall);
    let ceil_color = reflectance_to_rgb(reflectances.ceiling);

    // ─── Floor ──────────────────────────────────────────────────────
    faces.push(
        SceneFace::quad(
            (-hx, -hy, 0.0),
            (hx, -hy, 0.0),
            (hx, hy, 0.0),
            (-hx, hy, 0.0),
            &floor_color,
            "#666",
            0.9,
        )
        .with_stroke_width(1.5),
    );

    // ─── Walls ──────────────────────────────────────────────────────
    // Back wall (y = hy) — semi-transparent (50–80% per spec)
    faces.push(
        SceneFace::quad(
            (-hx, hy, 0.0),
            (hx, hy, 0.0),
            (hx, hy, h),
            (-hx, hy, h),
            &wall_color,
            "#888",
            0.6,
        )
        .with_stroke_width(1.0),
    );

    // Left wall (x = -hx) — semi-transparent (50–80% per spec)
    faces.push(
        SceneFace::quad(
            (-hx, -hy, 0.0),
            (-hx, hy, 0.0),
            (-hx, hy, h),
            (-hx, -hy, h),
            &wall_color,
            "#888",
            0.6,
        )
        .with_stroke_width(1.0),
    );

    // Right wall (x = hx) — very transparent (interior visible)
    faces.push(
        SceneFace::quad(
            (hx, -hy, 0.0),
            (hx, hy, 0.0),
            (hx, hy, h),
            (hx, -hy, h),
            &wall_color,
            "#888",
            0.15,
        )
        .with_stroke_width(0.5),
    );

    // Front wall (y = -hy) — very transparent (interior visible)
    faces.push(
        SceneFace::quad(
            (-hx, -hy, 0.0),
            (hx, -hy, 0.0),
            (hx, -hy, h),
            (-hx, -hy, h),
            &wall_color,
            "#888",
            0.15,
        )
        .with_stroke_width(0.5),
    );

    // ─── Ceiling ────────────────────────────────────────────────────
    faces.push(
        SceneFace::quad(
            (-hx, -hy, h),
            (hx, -hy, h),
            (hx, hy, h),
            (-hx, hy, h),
            &ceil_color,
            "#888",
            0.25,
        )
        .with_stroke_width(1.0),
    );

    // ─── Workplane ──────────────────────────────────────────────────
    let wp_z = room.workplane_height;

    if let Some(ppb) = ppb {
        // Heatmap on workplane
        let max_lux = ppb.max_lux.max(0.001);
        let n = ppb.grid_resolution;
        let step = if n > 16 { n / 16 } else { 1 };
        let grid_n = n.div_ceil(step);
        let dx = l / grid_n as f64;
        let dy = w / grid_n as f64;

        for row in 0..grid_n {
            for col in 0..grid_n {
                let src_row = (row * step).min(n - 1);
                let src_col = (col * step).min(n - 1);
                let lux = ppb.lux_grid[src_row][src_col];
                let color = heatmap_color(lux / max_lux);
                let fill = color.to_rgba_string(0.6);

                let x0 = col as f64 * dx - hx;
                let x1 = x0 + dx;
                let y0 = row as f64 * dy - hy;
                let y1 = y0 + dy;

                faces.push(
                    SceneFace::quad(
                        (x0, y0, wp_z),
                        (x1, y0, wp_z),
                        (x1, y1, wp_z),
                        (x0, y1, wp_z),
                        &fill,
                        &fill,
                        0.7,
                    )
                    .with_stroke_width(0.1),
                );
            }
        }
    } else if wp_z > 0.01 {
        // Simple workplane indicator
        faces.push(
            SceneFace::quad(
                (-hx, -hy, wp_z),
                (hx, -hy, wp_z),
                (hx, hy, wp_z),
                (-hx, hy, wp_z),
                "rgba(100,150,255,0.15)",
                "#4488cc",
                0.3,
            )
            .with_stroke_width(0.8)
            .with_dash("4,3"),
        );
    }

    // ─── Luminaire grid ─────────────────────────────────────────────
    let lum_z = h - room.suspension_length;
    let lum_size = 0.2_f64.max(l / (layout.rows.max(1) as f64 * 6.0)); // adaptive size

    for r in 0..layout.rows {
        for c in 0..layout.cols {
            let lx = layout.offset_x + r as f64 * layout.spacing_x - hx;
            let ly = layout.offset_y + c as f64 * layout.spacing_y - hy;

            // Luminaire rectangle
            faces.push(SceneFace::quad(
                (lx - lum_size, ly - lum_size, lum_z),
                (lx + lum_size, ly - lum_size, lum_z),
                (lx + lum_size, ly + lum_size, lum_z),
                (lx - lum_size, ly + lum_size, lum_z),
                "rgba(255,200,50,0.9)",
                "rgb(200,160,30)",
                1.0,
            ));

            // Suspension rod (if suspended)
            if room.suspension_length > 0.05 {
                let rod_hw = 0.02;
                faces.push(
                    SceneFace::quad(
                        (lx - rod_hw, ly, lum_z),
                        (lx + rod_hw, ly, lum_z),
                        (lx + rod_hw, ly, h),
                        (lx - rod_hw, ly, h),
                        "#999",
                        "#777",
                        0.5,
                    )
                    .with_stroke_width(0.3),
                );
            }

            // Light cone (luminaire → workplane): 4 triangles forming a pyramid
            if show_cones {
                let cone_r = (lum_z - wp_z) * 0.45;
                let cone_faces = [
                    // Front
                    [
                        (lx, ly, lum_z),
                        (lx - cone_r, ly - cone_r, wp_z),
                        (lx + cone_r, ly - cone_r, wp_z),
                    ],
                    // Back
                    [
                        (lx, ly, lum_z),
                        (lx - cone_r, ly + cone_r, wp_z),
                        (lx + cone_r, ly + cone_r, wp_z),
                    ],
                    // Left
                    [
                        (lx, ly, lum_z),
                        (lx - cone_r, ly - cone_r, wp_z),
                        (lx - cone_r, ly + cone_r, wp_z),
                    ],
                    // Right
                    [
                        (lx, ly, lum_z),
                        (lx + cone_r, ly - cone_r, wp_z),
                        (lx + cone_r, ly + cone_r, wp_z),
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
    }

    // ─── Cavity visualization ───────────────────────────────────────
    // Draws the three-cavity diagram directly onto the room walls —
    // boundary lines on back + left walls with colored zone fills and labels.
    if show_cavities {
        let dash = "6,3";

        // Luminaire plane (horizontal dashed boundary)
        if room.suspension_length > 0.05 {
            faces.push(
                SceneFace::quad(
                    (-hx, -hy, lum_z),
                    (hx, -hy, lum_z),
                    (hx, hy, lum_z),
                    (-hx, hy, lum_z),
                    "none",
                    "#4488cc",
                    0.4,
                )
                .with_stroke_width(1.0)
                .with_dash(dash),
            );

            // Ceiling cavity zone on back wall (ceiling to luminaire plane)
            faces.push(
                SceneFace::quad(
                    (-hx, hy, lum_z),
                    (hx, hy, lum_z),
                    (hx, hy, h),
                    (-hx, hy, h),
                    "rgba(68,136,204,0.08)",
                    "none",
                    0.5,
                )
                .with_stroke_width(0.0),
            );
            // Ceiling cavity zone on left wall
            faces.push(
                SceneFace::quad(
                    (-hx, -hy, lum_z),
                    (-hx, hy, lum_z),
                    (-hx, hy, h),
                    (-hx, -hy, h),
                    "rgba(68,136,204,0.08)",
                    "none",
                    0.5,
                )
                .with_stroke_width(0.0),
            );

            // Boundary line on back wall at luminaire plane
            faces.push(
                SceneFace::quad(
                    (-hx, hy, lum_z),
                    (hx, hy, lum_z),
                    (hx, hy, lum_z),
                    (-hx, hy, lum_z),
                    "none",
                    "#4488cc",
                    0.8,
                )
                .with_stroke_width(1.5)
                .with_dash(dash),
            );
            // Boundary line on left wall at luminaire plane
            faces.push(
                SceneFace::quad(
                    (-hx, -hy, lum_z),
                    (-hx, hy, lum_z),
                    (-hx, hy, lum_z),
                    (-hx, -hy, lum_z),
                    "none",
                    "#4488cc",
                    0.8,
                )
                .with_stroke_width(1.5)
                .with_dash(dash),
            );
        }

        // Workplane boundary
        if wp_z > 0.01 {
            if ppb.is_some() {
                // Dashed outline around the heatmap
                faces.push(
                    SceneFace::quad(
                        (-hx, -hy, wp_z),
                        (hx, -hy, wp_z),
                        (hx, hy, wp_z),
                        (-hx, hy, wp_z),
                        "none",
                        "#4488cc",
                        0.5,
                    )
                    .with_stroke_width(1.0)
                    .with_dash(dash),
                );
            }

            // Floor cavity zone on back wall (floor to workplane)
            if room.hfc() > 0.05 {
                faces.push(
                    SceneFace::quad(
                        (-hx, hy, 0.0),
                        (hx, hy, 0.0),
                        (hx, hy, wp_z),
                        (-hx, hy, wp_z),
                        "rgba(68,204,102,0.08)",
                        "none",
                        0.5,
                    )
                    .with_stroke_width(0.0),
                );
                // Floor cavity on left wall
                faces.push(
                    SceneFace::quad(
                        (-hx, -hy, 0.0),
                        (-hx, hy, 0.0),
                        (-hx, hy, wp_z),
                        (-hx, -hy, wp_z),
                        "rgba(68,204,102,0.08)",
                        "none",
                        0.5,
                    )
                    .with_stroke_width(0.0),
                );
            }

            // Boundary line on back wall at workplane
            faces.push(
                SceneFace::quad(
                    (-hx, hy, wp_z),
                    (hx, hy, wp_z),
                    (hx, hy, wp_z),
                    (-hx, hy, wp_z),
                    "none",
                    "#cc6644",
                    0.8,
                )
                .with_stroke_width(1.5)
                .with_dash(dash),
            );
            // Boundary line on left wall at workplane
            faces.push(
                SceneFace::quad(
                    (-hx, -hy, wp_z),
                    (-hx, hy, wp_z),
                    (-hx, hy, wp_z),
                    (-hx, -hy, wp_z),
                    "none",
                    "#cc6644",
                    0.8,
                )
                .with_stroke_width(1.5)
                .with_dash(dash),
            );
        }

        // Room cavity zone on back wall (workplane to luminaire plane) — subtle tint
        let rc_top = if room.suspension_length > 0.05 {
            lum_z
        } else {
            h
        };
        faces.push(
            SceneFace::quad(
                (-hx, hy, wp_z),
                (hx, hy, wp_z),
                (hx, hy, rc_top),
                (-hx, hy, rc_top),
                "rgba(204,102,68,0.06)",
                "none",
                0.4,
            )
            .with_stroke_width(0.0),
        );
        // Room cavity on left wall
        faces.push(
            SceneFace::quad(
                (-hx, -hy, wp_z),
                (-hx, hy, wp_z),
                (-hx, hy, rc_top),
                (-hx, -hy, rc_top),
                "rgba(204,102,68,0.06)",
                "none",
                0.4,
            )
            .with_stroke_width(0.0),
        );

        // Cavity height labels on left wall (front-left corner for visibility)
        let label_x = -hx;
        let label_y = -hy;

        if room.hcc() > 0.1 {
            let mid_z = h - room.hcc() / 2.0;
            faces.push(
                SceneFace::quad(
                    (label_x, label_y, mid_z - 0.01),
                    (label_x, label_y, mid_z + 0.01),
                    (label_x, label_y, mid_z + 0.01),
                    (label_x, label_y, mid_z - 0.01),
                    "none",
                    "#4488cc",
                    1.0,
                )
                .with_label(&format!("hcc={:.2}", room.hcc()), 9.0),
            );
        }

        {
            let mid_z = wp_z + room.hrc() / 2.0;
            faces.push(
                SceneFace::quad(
                    (label_x, label_y, mid_z - 0.01),
                    (label_x, label_y, mid_z + 0.01),
                    (label_x, label_y, mid_z + 0.01),
                    (label_x, label_y, mid_z - 0.01),
                    "none",
                    "#cc6644",
                    1.0,
                )
                .with_label(&format!("hrc={:.2}", room.hrc()), 9.0),
            );
        }

        if room.hfc() > 0.1 {
            let mid_z = wp_z / 2.0;
            faces.push(
                SceneFace::quad(
                    (label_x, label_y, mid_z - 0.01),
                    (label_x, label_y, mid_z + 0.01),
                    (label_x, label_y, mid_z + 0.01),
                    (label_x, label_y, mid_z - 0.01),
                    "none",
                    "#44cc66",
                    1.0,
                )
                .with_label(&format!("hfc={:.2}", room.hfc()), 9.0),
            );
        }
    }

    // ─── Dimension labels ───────────────────────────────────────────
    // Length label (bottom edge)
    faces.push(
        SceneFace::quad(
            (0.0, -hy - 0.3, 0.0),
            (0.0, -hy - 0.3, 0.0),
            (0.0, -hy - 0.3, 0.0),
            (0.0, -hy - 0.3, 0.0),
            "none",
            "#555",
            1.0,
        )
        .with_label(&format!("{:.1}m", l), 10.0),
    );

    // Width label (right edge)
    faces.push(
        SceneFace::quad(
            (hx + 0.3, 0.0, 0.0),
            (hx + 0.3, 0.0, 0.0),
            (hx + 0.3, 0.0, 0.0),
            (hx + 0.3, 0.0, 0.0),
            "none",
            "#555",
            1.0,
        )
        .with_label(&format!("{:.1}m", w), 10.0),
    );

    // Height label (left edge)
    faces.push(
        SceneFace::quad(
            (-hx - 0.3, -hy, h / 2.0),
            (-hx - 0.3, -hy, h / 2.0),
            (-hx - 0.3, -hy, h / 2.0),
            (-hx - 0.3, -hy, h / 2.0),
            "none",
            "#555",
            1.0,
        )
        .with_label(&format!("{:.1}m", h), 10.0),
    );

    faces
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_interior_basic() {
        let room = Room::new(10.0, 8.0, 3.0, 0.80, 0.0);
        let layout = crate::zonal::find_best_layout(9, 10.0, 8.0, 1.5, 2.2);
        let reflectances = Reflectances::new(0.70, 0.50, 0.20);
        let cavity = crate::zonal::compute_cavity_ratios(&room, &reflectances);

        let faces =
            build_interior_scene(&room, &layout, &reflectances, &cavity, None, false, false);
        // floor(1) + 4 walls + ceiling(1) + workplane(1) + 9 luminaires + 3 dim labels = ~20
        assert!(faces.len() >= 15, "got {} faces", faces.len());
    }

    #[test]
    fn test_build_interior_with_cavities() {
        let room = Room::new(10.0, 8.0, 3.0, 0.80, 0.15);
        let layout = crate::zonal::find_best_layout(4, 10.0, 8.0, 1.5, 2.05);
        let reflectances = Reflectances::new(0.70, 0.50, 0.20);
        let cavity = crate::zonal::compute_cavity_ratios(&room, &reflectances);

        let faces = build_interior_scene(&room, &layout, &reflectances, &cavity, None, true, false);
        // Should have cavity visualization faces
        assert!(faces.len() > 20, "got {} faces", faces.len());
    }

    #[test]
    fn test_interior_renders_svg() {
        let room = Room::new(10.0, 8.0, 3.0, 0.80, 0.0);
        let layout = crate::zonal::find_best_layout(9, 10.0, 8.0, 1.5, 2.2);
        let reflectances = Reflectances::new(0.70, 0.50, 0.20);
        let cavity = crate::zonal::compute_cavity_ratios(&room, &reflectances);

        let faces =
            build_interior_scene(&room, &layout, &reflectances, &cavity, None, false, false);
        let svg_w = 600.0;
        let svg_h = 450.0;
        let preset = super::super::CameraPreset::FrontRight;
        let mut cam = preset.to_camera(svg_w, svg_h, 1.0);
        cam.scale =
            super::super::fit_scale(room.length, room.width, room.height, svg_w, svg_h, &cam);

        assert!(cam.scale > 1.0, "scale should be > 1, got {}", cam.scale);

        let svg = super::super::render_scene_svg(&faces, &cam, svg_w, svg_h, "#f8f9fa");
        let poly_count = svg.matches("<polygon").count();
        assert!(
            poly_count >= 15,
            "expected >=15 polygons, got {}",
            poly_count
        );
        assert!(svg.contains("rgb("), "should have colored fills");
    }
}
