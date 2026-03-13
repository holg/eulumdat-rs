//! SVG rendering for area lighting designer.
//!
//! Provides plan view (top-down pole positions) and combined ISO view
//! (multi-luminaire illuminance heatmap with contours).

use super::compute::{AreaResult, LuminairePlace};
use crate::diagram::color::heatmap_color;
use crate::diagram::contour::marching_squares;
use crate::diagram::SvgTheme;
use crate::units::UnitSystem;

/// SVG output for area lighting views.
pub struct AreaSvg;

impl AreaSvg {
    /// Render a top-down plan view showing pole positions and area outline.
    ///
    /// `pole_positions` — `(id, x, y)` of each pole center (for selection/labels).
    /// `placements` — luminaire placements (for rotation indicators).
    /// `selected_pole` — which pole ID is selected.
    /// `luminaires_per_pole` — how many luminaires per pole (for grouping).
    pub fn plan_view_with_poles(
        pole_positions: &[(usize, f64, f64)],
        placements: &[LuminairePlace],
        area_width: f64,
        area_depth: f64,
        svg_width: f64,
        svg_height: f64,
        theme: &SvgTheme,
        selected_pole: Option<usize>,
        luminaires_per_pole: usize,
        units: UnitSystem,
    ) -> String {
        let margin = 40.0;
        let plot_w = svg_width - 2.0 * margin;
        let plot_h = svg_height - 2.0 * margin;

        let scale_x = plot_w / area_width;
        let scale_y = plot_h / area_depth;

        let is_dark = theme.background.contains("0f172a")
            || theme.background.contains("1e1e")
            || theme.background.contains("dark");
        let (bg, text_color, grid_color) = if is_dark {
            ("#1e1e1e", "#e0e0e0", "rgba(255,255,255,0.15)")
        } else {
            ("#ffffff", "#333333", "rgba(0,0,0,0.1)")
        };

        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {svg_width} {svg_height}" font-family="sans-serif">
<rect width="{svg_width}" height="{svg_height}" fill="{bg}"/>
"#
        );

        // Area outline
        svg.push_str(&format!(
            r#"<rect x="{margin}" y="{margin}" width="{plot_w}" height="{plot_h}" fill="none" stroke="{text_color}" stroke-width="1.5" stroke-dasharray="4,2"/>"#
        ));

        // Grid lines every 10m
        let grid_step = 10.0;
        let mut gx = grid_step;
        while gx < area_width {
            let sx = margin + gx * scale_x;
            svg.push_str(&format!(
                r#"<line x1="{sx}" y1="{margin}" x2="{sx}" y2="{}" stroke="{grid_color}" stroke-width="0.5"/>"#,
                margin + plot_h
            ));
            gx += grid_step;
        }
        let mut gy = grid_step;
        while gy < area_depth {
            let sy = margin + gy * scale_y;
            svg.push_str(&format!(
                r#"<line x1="{margin}" y1="{sy}" x2="{}" y2="{sy}" stroke="{grid_color}" stroke-width="0.5"/>"#,
                margin + plot_w
            ));
            gy += grid_step;
        }

        // Dimension labels
        let dl = units.distance_label();
        let w_display = units.convert_meters(area_width);
        let d_display = units.convert_meters(area_depth);
        svg.push_str(&format!(
            r#"<text x="{}" y="{}" text-anchor="middle" fill="{text_color}" font-size="10">{w_display:.0} {dl}</text>"#,
            margin + plot_w / 2.0,
            svg_height - 5.0,
        ));
        svg.push_str(&format!(
            r#"<text x="12" y="{}" text-anchor="middle" fill="{text_color}" font-size="10" transform="rotate(-90, 12, {})">{d_display:.0} {dl}</text>"#,
            margin + plot_h / 2.0,
            margin + plot_h / 2.0,
        ));

        // Draw luminaire positions (small dots + rotation arrows)
        let lpp = luminaires_per_pole.max(1);
        for (pole_idx, &(pole_id, _px, _py)) in pole_positions.iter().enumerate() {
            let is_selected = selected_pole == Some(pole_id);
            let start = pole_idx * lpp;
            let end = (start + lpp).min(placements.len());

            for lum in &placements[start..end] {
                let (ex, ey) = lum.effective_position();
                let sx = margin + ex * scale_x;
                let sy = margin + ey * scale_y;

                let (fill, stroke) = if is_selected {
                    ("rgba(255,165,0,0.5)", "rgb(255,120,0)")
                } else {
                    ("rgba(70,130,230,0.4)", "rgb(50,100,200)")
                };

                // Small luminaire dot
                svg.push_str(&format!(
                    r#"<circle cx="{sx}" cy="{sy}" r="3" fill="{fill}" stroke="{stroke}" stroke-width="0.8"/>"#
                ));

                // Rotation direction indicator
                let dir_len = 10.0;
                let rot_rad = lum.rotation.to_radians();
                let dx = dir_len * rot_rad.sin();
                let dy = -dir_len * rot_rad.cos();
                svg.push_str(&format!(
                    r#"<line x1="{sx}" y1="{sy}" x2="{}" y2="{}" stroke="{stroke}" stroke-width="1" stroke-opacity="0.6"/>"#,
                    sx + dx, sy + dy
                ));
            }
        }

        // Draw pole centers (larger dots, on top)
        for &(pole_id, px, py) in pole_positions {
            let sx = margin + px * scale_x;
            let sy = margin + py * scale_y;

            let is_selected = selected_pole == Some(pole_id);
            let (fill, stroke, r) = if is_selected {
                ("rgb(255,165,0)", "rgb(255,120,0)", 6.0)
            } else {
                ("rgb(70,130,230)", "rgb(50,100,200)", 5.0)
            };

            svg.push_str(&format!(
                r#"<circle cx="{sx}" cy="{sy}" r="{r}" fill="{fill}" stroke="{stroke}" stroke-width="1.5"/>"#
            ));

            // Pole label
            svg.push_str(&format!(
                r#"<text x="{}" y="{}" text-anchor="start" fill="{text_color}" font-size="9">{}</text>"#,
                sx + 8.0, sy - 6.0, pole_id + 1
            ));
        }

        svg.push_str("</svg>");
        svg
    }

    /// Plan view with optional polygon outline instead of rectangle.
    pub fn plan_view_with_polygon(
        pole_positions: &[(usize, f64, f64)],
        placements: &[LuminairePlace],
        polygon: &super::polygon::AreaPolygon,
        svg_width: f64,
        svg_height: f64,
        theme: &SvgTheme,
        selected_pole: Option<usize>,
        luminaires_per_pole: usize,
        units: UnitSystem,
    ) -> String {
        let (bx0, by0, bx1, by1) = polygon.bounding_box();
        let bbox_w = bx1 - bx0;
        let bbox_h = by1 - by0;
        let margin = 40.0;
        let plot_w = svg_width - 2.0 * margin;
        let plot_h = svg_height - 2.0 * margin;
        let scale_x = plot_w / bbox_w;
        let scale_y = plot_h / bbox_h;

        let is_dark = theme.background.contains("0f172a")
            || theme.background.contains("1e1e")
            || theme.background.contains("dark");
        let (bg, text_color, grid_color) = if is_dark {
            ("#1e1e1e", "#e0e0e0", "rgba(255,255,255,0.15)")
        } else {
            ("#ffffff", "#333333", "rgba(0,0,0,0.1)")
        };

        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {svg_width} {svg_height}" font-family="sans-serif">
<rect width="{svg_width}" height="{svg_height}" fill="{bg}"/>
"#
        );

        // Polygon outline (replaces rectangle)
        let pts = polygon.to_svg_points(bx0, by0, scale_x, scale_y, margin);
        svg.push_str(&format!(
            r#"<polygon points="{pts}" fill="none" stroke="{text_color}" stroke-width="1.5" stroke-dasharray="4,2"/>"#
        ));

        // Grid lines
        let grid_step = 10.0;
        let mut gx = (bx0 / grid_step).ceil() * grid_step;
        while gx < bx1 {
            let sx = margin + (gx - bx0) * scale_x;
            svg.push_str(&format!(
                r#"<line x1="{sx}" y1="{margin}" x2="{sx}" y2="{}" stroke="{grid_color}" stroke-width="0.5"/>"#,
                margin + plot_h
            ));
            gx += grid_step;
        }
        let mut gy = (by0 / grid_step).ceil() * grid_step;
        while gy < by1 {
            let sy = margin + (gy - by0) * scale_y;
            svg.push_str(&format!(
                r#"<line x1="{margin}" y1="{sy}" x2="{}" y2="{sy}" stroke="{grid_color}" stroke-width="0.5"/>"#,
                margin + plot_w
            ));
            gy += grid_step;
        }

        // Dimension labels
        let dl = units.distance_label();
        let w_display = units.convert_meters(bbox_w);
        let d_display = units.convert_meters(bbox_h);
        svg.push_str(&format!(
            r#"<text x="{}" y="{}" text-anchor="middle" fill="{text_color}" font-size="10">{w_display:.0} {dl}</text>"#,
            margin + plot_w / 2.0,
            svg_height - 5.0,
        ));
        svg.push_str(&format!(
            r#"<text x="12" y="{}" text-anchor="middle" fill="{text_color}" font-size="10" transform="rotate(-90, 12, {})">{d_display:.0} {dl}</text>"#,
            margin + plot_h / 2.0,
            margin + plot_h / 2.0,
        ));

        // Luminaire positions
        let lpp = luminaires_per_pole.max(1);
        for (pole_idx, &(pole_id, _px, _py)) in pole_positions.iter().enumerate() {
            let is_selected = selected_pole == Some(pole_id);
            let start = pole_idx * lpp;
            let end = (start + lpp).min(placements.len());
            for lum in &placements[start..end] {
                let (ex, ey) = lum.effective_position();
                let sx = margin + (ex - bx0) * scale_x;
                let sy = margin + (ey - by0) * scale_y;
                let (fill, stroke) = if is_selected {
                    ("rgba(255,165,0,0.5)", "rgb(255,120,0)")
                } else {
                    ("rgba(70,130,230,0.4)", "rgb(50,100,200)")
                };
                svg.push_str(&format!(
                    r#"<circle cx="{sx}" cy="{sy}" r="3" fill="{fill}" stroke="{stroke}" stroke-width="0.8"/>"#
                ));
                let dir_len = 10.0;
                let rot_rad = lum.rotation.to_radians();
                let ddx = dir_len * rot_rad.sin();
                let ddy = -dir_len * rot_rad.cos();
                svg.push_str(&format!(
                    r#"<line x1="{sx}" y1="{sy}" x2="{}" y2="{}" stroke="{stroke}" stroke-width="1" stroke-opacity="0.6"/>"#,
                    sx + ddx, sy + ddy
                ));
            }
        }

        // Pole centers
        for &(pole_id, px, py) in pole_positions {
            let sx = margin + (px - bx0) * scale_x;
            let sy = margin + (py - by0) * scale_y;
            let is_selected = selected_pole == Some(pole_id);
            let (fill, stroke, r) = if is_selected {
                ("rgb(255,165,0)", "rgb(255,120,0)", 6.0)
            } else {
                ("rgb(70,130,230)", "rgb(50,100,200)", 5.0)
            };
            svg.push_str(&format!(
                r#"<circle cx="{sx}" cy="{sy}" r="{r}" fill="{fill}" stroke="{stroke}" stroke-width="1.5"/>"#
            ));
            svg.push_str(&format!(
                r#"<text x="{}" y="{}" text-anchor="start" fill="{text_color}" font-size="9">{}</text>"#,
                sx + 8.0, sy - 6.0, pole_id + 1
            ));
        }

        // Polygon vertex markers (editable dots)
        for (i, &(vx, vy)) in polygon.vertices.iter().enumerate() {
            let sx = margin + (vx - bx0) * scale_x;
            let sy = margin + (vy - by0) * scale_y;
            svg.push_str(&format!(
                r#"<circle cx="{sx}" cy="{sy}" r="4" fill="rgba(220,50,50,0.6)" stroke="red" stroke-width="1" data-vertex="{i}"/>"#
            ));
        }

        svg.push_str("</svg>");
        svg
    }

    /// Simple plan view (no pole separation) — backward compatible.
    pub fn plan_view(
        placements: &[LuminairePlace],
        area_width: f64,
        area_depth: f64,
        svg_width: f64,
        svg_height: f64,
        theme: &SvgTheme,
        selected_id: Option<usize>,
    ) -> String {
        // Convert placements to pole positions (one-to-one)
        let poles: Vec<(usize, f64, f64)> = placements.iter().map(|p| (p.id, p.x, p.y)).collect();
        Self::plan_view_with_poles(&poles, placements, area_width, area_depth, svg_width, svg_height, theme, selected_id, 1, UnitSystem::default())
    }

    /// Render combined illuminance heatmap with contours.
    pub fn iso_view(
        result: &AreaResult,
        svg_width: f64,
        svg_height: f64,
        theme: &SvgTheme,
        units: UnitSystem,
    ) -> String {
        let margin_left = 50.0;
        let margin_right = 70.0;
        let margin_top = 30.0;
        let margin_bottom = 45.0;

        let plot_w = svg_width - margin_left - margin_right;
        let plot_h = svg_height - margin_top - margin_bottom;

        let n = result.grid_resolution;
        let cell_w = plot_w / n as f64;
        let cell_h = plot_h / n as f64;

        let max_lux = result.max_lux;

        let is_dark = theme.background.contains("0f172a")
            || theme.background.contains("1e1e")
            || theme.background.contains("dark");
        let (bg, text_color) = if is_dark {
            ("#1e1e1e", "#e0e0e0")
        } else {
            ("#ffffff", "#333333")
        };

        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {svg_width} {svg_height}" font-family="sans-serif">
<rect width="{svg_width}" height="{svg_height}" fill="{bg}"/>
"#
        );

        // Heatmap cells (with optional polygon mask)
        if max_lux > 0.0 {
            for (row, grid_row) in result.lux_grid.iter().enumerate() {
                for (col, &lux) in grid_row.iter().enumerate() {
                    let normalized = lux / max_lux;
                    let color = heatmap_color(normalized);
                    let sx = margin_left + col as f64 * cell_w;
                    let sy = margin_top + row as f64 * cell_h;

                    let is_inside = result
                        .mask
                        .as_ref()
                        .map(|m| m[row][col])
                        .unwrap_or(true);

                    if is_inside {
                        svg.push_str(&format!(
                            r#"<rect x="{sx}" y="{sy}" width="{}" height="{}" fill="{}"/>"#,
                            cell_w + 0.5,
                            cell_h + 0.5,
                            color.to_rgb_string()
                        ));
                    } else {
                        svg.push_str(&format!(
                            r#"<rect x="{sx}" y="{sy}" width="{}" height="{}" fill="{}" opacity="0.15"/>"#,
                            cell_w + 0.5,
                            cell_h + 0.5,
                            color.to_rgb_string()
                        ));
                    }
                }
            }
        }

        // Contour lines
        let illu_label = units.illuminance_label();
        let contour_levels: Vec<f64> = match units {
            UnitSystem::Imperial => [0.5, 1.0, 2.0, 5.0, 10.0, 25.0, 50.0, 100.0]
                .iter()
                .map(|&fc| fc * 10.764)
                .collect(),
            UnitSystem::Metric => {
                vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0]
            }
        };

        let x_coords: Vec<f64> = (0..n)
            .map(|col| margin_left + (col as f64 + 0.5) * cell_w)
            .collect();
        let y_coords: Vec<f64> = (0..n)
            .map(|row| margin_top + (row as f64 + 0.5) * cell_h)
            .collect();

        for &level in &contour_levels {
            if level > max_lux || level <= 0.0 {
                continue;
            }
            let cl = marching_squares(&result.lux_grid, &x_coords, &y_coords, level);
            let display_val = units.convert_lux(level);
            let label = fmt_lux(display_val);

            for path in &cl.paths {
                svg.push_str(&format!(
                    r#"<path d="{path}" fill="none" stroke="rgba(255,255,255,0.7)" stroke-width="1"/>"#
                ));
            }

            // Label on first path segment if available
            if let Some(first_path) = cl.paths.first() {
                if let Some(pos) = extract_path_midpoint(first_path) {
                    svg.push_str(&format!(
                        r#"<text x="{}" y="{}" fill="{text_color}" font-size="8" text-anchor="middle" dominant-baseline="middle">{label} {illu_label}</text>"#,
                        pos.0, pos.1
                    ));
                }
            }
        }

        // Color legend
        let legend_x = svg_width - margin_right + 10.0;
        let legend_h = plot_h * 0.7;
        let legend_top = margin_top + (plot_h - legend_h) / 2.0;
        let legend_w = 12.0;
        let num_segments = 30;

        for i in 0..num_segments {
            let t = i as f64 / num_segments as f64;
            let color = heatmap_color(1.0 - t);
            let seg_h = legend_h / num_segments as f64;
            svg.push_str(&format!(
                r#"<rect x="{legend_x}" y="{}" width="{legend_w}" height="{}" fill="{}"/>"#,
                legend_top + t * legend_h,
                seg_h + 0.5,
                color.to_rgb_string()
            ));
        }

        // Legend labels
        for i in 0..=4 {
            let t = i as f64 / 4.0;
            let lux_val = max_lux * (1.0 - t);
            let display_val = units.convert_lux(lux_val);
            let y = legend_top + t * legend_h;
            svg.push_str(&format!(
                r#"<text x="{}" y="{y}" fill="{text_color}" font-size="8" dominant-baseline="middle">{} {illu_label}</text>"#,
                legend_x + legend_w + 4.0,
                fmt_lux(display_val),
            ));
        }

        // Statistics text
        let stats_y = svg_height - 8.0;
        let min_d = fmt_lux(units.convert_lux(result.min_lux));
        let avg_d = fmt_lux(units.convert_lux(result.avg_lux));
        let max_d = fmt_lux(units.convert_lux(result.max_lux));
        svg.push_str(&format!(
            r#"<text x="{margin_left}" y="{stats_y}" fill="{text_color}" font-size="9">Min: {min_d}  Avg: {avg_d}  Max: {max_d} {illu_label}  |  U₀: {:.2}  Ud: {:.2}</text>"#,
            result.uniformity_min_avg, result.uniformity_min_max
        ));

        svg.push_str("</svg>");
        svg
    }
}

/// An overlay contour set from a different configuration (e.g. different height).
pub struct ContourOverlay {
    /// The illuminance grid to draw contours from
    pub result: AreaResult,
    /// Label for this overlay (e.g. "8m", "12m")
    pub label: String,
    /// CSS color for the contour lines (e.g. "rgba(255,100,100,0.8)")
    pub color: String,
}

impl AreaSvg {
    /// Render ISO view with additional contour overlays from other configurations.
    ///
    /// The heatmap is drawn from `result`. Each overlay adds colored contour lines on top.
    pub fn iso_view_with_overlays(
        result: &AreaResult,
        overlays: &[ContourOverlay],
        svg_width: f64,
        svg_height: f64,
        theme: &SvgTheme,
        units: UnitSystem,
    ) -> String {
        if overlays.is_empty() {
            return Self::iso_view(result, svg_width, svg_height, theme, units);
        }

        // Generate base SVG without closing tag
        let mut svg = Self::iso_view(result, svg_width, svg_height, theme, units);
        // Remove closing </svg> to append overlays
        if let Some(pos) = svg.rfind("</svg>") {
            svg.truncate(pos);
        }

        let margin_left = 50.0;
        let margin_right = 70.0;
        let margin_top = 30.0;
        let margin_bottom = 45.0;
        let plot_w = svg_width - margin_left - margin_right;
        let plot_h = svg_height - margin_top - margin_bottom;

        let is_dark = theme.background.contains("0f172a")
            || theme.background.contains("1e1e")
            || theme.background.contains("dark");
        let text_color = if is_dark { "#e0e0e0" } else { "#333333" };

        for overlay in overlays {
            let n = overlay.result.grid_resolution;
            if n == 0 {
                continue;
            }
            let cell_w = plot_w / n as f64;
            let cell_h = plot_h / n as f64;

            let x_coords: Vec<f64> = (0..n)
                .map(|col| margin_left + (col as f64 + 0.5) * cell_w)
                .collect();
            let y_coords: Vec<f64> = (0..n)
                .map(|row| margin_top + (row as f64 + 0.5) * cell_h)
                .collect();

            let contour_levels: Vec<f64> = match units {
                UnitSystem::Imperial => [0.5, 1.0, 2.0, 5.0, 10.0, 25.0, 50.0, 100.0]
                    .iter()
                    .map(|&fc| fc * 10.764)
                    .collect(),
                UnitSystem::Metric => {
                    vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0]
                }
            };

            let max_lux = overlay.result.max_lux;
            for &level in &contour_levels {
                if level > max_lux || level <= 0.0 {
                    continue;
                }
                let cl = marching_squares(
                    &overlay.result.lux_grid,
                    &x_coords,
                    &y_coords,
                    level,
                );
                for path in &cl.paths {
                    svg.push_str(&format!(
                        r#"<path d="{path}" fill="none" stroke="{}" stroke-width="1.2" stroke-dasharray="4,3"/>"#,
                        overlay.color
                    ));
                }
            }

            // Overlay label in bottom-right of plot area
            let label_x = margin_left + plot_w - 5.0;
            let label_y = margin_top + plot_h - 5.0 - (overlays.len() as f64 - 1.0) * 12.0;
            // Find the index of this overlay to offset labels
            if let Some(idx) = overlays.iter().position(|o| std::ptr::eq(o, overlay)) {
                let ly = label_y + idx as f64 * 12.0;
                svg.push_str(&format!(
                    r#"<text x="{label_x}" y="{ly}" fill="{}" font-size="8" text-anchor="end" font-weight="600">{}</text>"#,
                    overlay.color, overlay.label
                ));
            }
        }

        // Overlay legend at bottom
        let legend_y = svg_height - 20.0;
        let mut lx = margin_left;
        for overlay in overlays {
            svg.push_str(&format!(
                r#"<line x1="{lx}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="2" stroke-dasharray="4,3"/>"#,
                legend_y, lx + 15.0, legend_y, overlay.color
            ));
            svg.push_str(&format!(
                r#"<text x="{}" y="{legend_y}" fill="{text_color}" font-size="7" dominant-baseline="middle">{}</text>"#,
                lx + 18.0, overlay.label
            ));
            lx += 18.0 + overlay.label.len() as f64 * 5.0 + 10.0;
        }

        svg.push_str("</svg>");
        svg
    }
}

impl AreaSvg {
    /// Render a pseudo-3D perspective room view with illuminance on floor, back wall, and left wall.
    ///
    /// Computes wall illuminance from the LDT data, projects heatmaps onto all visible
    /// surfaces in perspective. Pure SVG, no WebGL needed.
    pub fn room_view(
        result: &AreaResult,
        placements: &[LuminairePlace],
        ldt: &crate::Eulumdat,
        mounting_height: f64,
        proration_factor: f64,
        svg_width: f64,
        svg_height: f64,
        theme: &SvgTheme,
        units: UnitSystem,
    ) -> String {
        let is_dark = theme.background.contains("0f172a")
            || theme.background.contains("1e1e")
            || theme.background.contains("dark");
        let (bg, text_color, wall_stroke, ceiling_fill) = if is_dark {
            ("#1a1a2e", "#d0d0d0", "#444466", "#252540")
        } else {
            ("#f0f4f8", "#333333", "#99aabb", "#c8d4e0")
        };

        let area_w = result.area_width;
        let area_d = result.area_depth;
        let n = result.grid_resolution;

        // Wall grid resolution (fewer cells than floor for performance)
        let wall_res = (n / 2).max(6).min(16);

        // --- Compute wall illuminance grids ---
        // Back wall: at y = area_d, spanning x = [0, area_w], z = [0, mounting_height]
        // Normal points toward viewer: (0, -1, 0)
        let back_wall_points: Vec<Vec<(f64, f64, f64)>> = (0..wall_res)
            .map(|row| {
                let z = mounting_height * (1.0 - (row as f64 + 0.5) / wall_res as f64);
                (0..wall_res)
                    .map(|col| {
                        let x = area_w * (col as f64 + 0.5) / wall_res as f64;
                        (x, area_d, z)
                    })
                    .collect()
            })
            .collect();
        let back_wall_lux = super::compute::compute_wall_illuminance(
            ldt, placements, &back_wall_points, (0.0, -1.0, 0.0), proration_factor,
        );

        // Left wall: at x = 0, spanning y = [0, area_d], z = [0, mounting_height]
        // Normal points right: (1, 0, 0)
        let left_wall_points: Vec<Vec<(f64, f64, f64)>> = (0..wall_res)
            .map(|row| {
                let z = mounting_height * (1.0 - (row as f64 + 0.5) / wall_res as f64);
                (0..wall_res)
                    .map(|col| {
                        let y = area_d * (col as f64 + 0.5) / wall_res as f64;
                        (0.0, y, z)
                    })
                    .collect()
            })
            .collect();
        let left_wall_lux = super::compute::compute_wall_illuminance(
            ldt, placements, &left_wall_points, (1.0, 0.0, 0.0), proration_factor,
        );

        // Find global max for consistent color scale across all surfaces
        let floor_max = result.max_lux;
        let back_max = back_wall_lux.iter().flat_map(|r| r.iter()).cloned()
            .fold(0.0_f64, f64::max);
        let left_max = left_wall_lux.iter().flat_map(|r| r.iter()).cloned()
            .fold(0.0_f64, f64::max);
        let global_max = floor_max.max(back_max).max(left_max).max(0.001);

        // --- Perspective projection parameters ---
        let floor_bl = (svg_width * 0.08, svg_height * 0.92);
        let floor_br = (svg_width * 0.92, svg_height * 0.92);
        let floor_tl = (svg_width * 0.25, svg_height * 0.42);
        let floor_tr = (svg_width * 0.75, svg_height * 0.42);

        let wall_h_svg = floor_bl.1 - floor_tl.1;
        let max_scene_h = area_w.max(area_d).max(1.0);
        let ceil_frac = (mounting_height / max_scene_h).min(0.95).max(0.3);

        let ceil_tl = (floor_tl.0, floor_tl.1 - wall_h_svg * ceil_frac);
        let ceil_tr = (floor_tr.0, floor_tr.1 - wall_h_svg * ceil_frac);
        let left_ceil_near = (floor_bl.0, floor_bl.1 - wall_h_svg * ceil_frac);

        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {svg_width} {svg_height}" font-family="sans-serif">
<rect width="{svg_width}" height="{svg_height}" fill="{bg}"/>
"#
        );

        // --- Back wall heatmap ---
        render_wall_heatmap(
            &mut svg,
            &back_wall_lux,
            wall_res,
            global_max,
            // quad corners: bottom-left, bottom-right, top-left, top-right
            floor_tl, floor_tr, ceil_tl, ceil_tr,
        );
        // Back wall outline
        svg.push_str(&format!(
            r#"<polygon points="{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}" fill="none" stroke="{wall_stroke}" stroke-width="1"/>"#,
            floor_tl.0, floor_tl.1, floor_tr.0, floor_tr.1,
            ceil_tr.0, ceil_tr.1, ceil_tl.0, ceil_tl.1,
        ));

        // --- Left wall heatmap ---
        render_wall_heatmap(
            &mut svg,
            &left_wall_lux,
            wall_res,
            global_max,
            // quad: near-bottom, far-bottom, near-top, far-top
            // Left wall goes from near (floor_bl) to far (floor_tl)
            floor_bl, floor_tl, left_ceil_near, ceil_tl,
        );
        // Left wall outline
        svg.push_str(&format!(
            r#"<polygon points="{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}" fill="none" stroke="{wall_stroke}" stroke-width="1"/>"#,
            floor_bl.0, floor_bl.1, floor_tl.0, floor_tl.1,
            ceil_tl.0, ceil_tl.1, left_ceil_near.0, left_ceil_near.1,
        ));

        // --- Ceiling (semi-transparent) ---
        let right_ceil_near = (floor_br.0, floor_br.1 - wall_h_svg * ceil_frac);
        svg.push_str(&format!(
            r#"<polygon points="{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}" fill="{ceiling_fill}" stroke="{wall_stroke}" stroke-width="0.5" opacity="0.3"/>"#,
            ceil_tl.0, ceil_tl.1, ceil_tr.0, ceil_tr.1,
            right_ceil_near.0, right_ceil_near.1,
            left_ceil_near.0, left_ceil_near.1,
        ));

        // --- Floor heatmap ---
        if global_max > 0.0 && n > 0 {
            for row in 0..n {
                for col in 0..n {
                    let is_inside = result.mask.as_ref().map(|m| m[row][col]).unwrap_or(true);
                    if !is_inside {
                        continue;
                    }

                    let lux = result.lux_grid[row][col];
                    let normalized = lux / global_max;
                    let color = heatmap_color(normalized);

                    let u0 = col as f64 / n as f64;
                    let u1 = (col + 1) as f64 / n as f64;
                    let v0 = row as f64 / n as f64;
                    let v1 = (row + 1) as f64 / n as f64;

                    let p00 = lerp_quad(floor_bl, floor_br, floor_tl, floor_tr, u0, v0);
                    let p10 = lerp_quad(floor_bl, floor_br, floor_tl, floor_tr, u1, v0);
                    let p11 = lerp_quad(floor_bl, floor_br, floor_tl, floor_tr, u1, v1);
                    let p01 = lerp_quad(floor_bl, floor_br, floor_tl, floor_tr, u0, v1);

                    svg.push_str(&format!(
                        r#"<polygon points="{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}" fill="{}" stroke="{}" stroke-width="0.2"/>"#,
                        p00.0, p00.1, p10.0, p10.1, p11.0, p11.1, p01.0, p01.1,
                        color.to_rgb_string(), color.to_rgb_string(),
                    ));
                }
            }
        }

        // Floor outline
        svg.push_str(&format!(
            r#"<polygon points="{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}" fill="none" stroke="{wall_stroke}" stroke-width="1.5"/>"#,
            floor_bl.0, floor_bl.1, floor_br.0, floor_br.1,
            floor_tr.0, floor_tr.1, floor_tl.0, floor_tl.1,
        ));

        // --- Grid lines on floor ---
        let grid_step = adaptive_grid_step(area_w.max(area_d));
        let mut gx = grid_step;
        while gx < area_w {
            let u = gx / area_w;
            let p_near = lerp_line(floor_bl, floor_br, u);
            let p_far = lerp_line(floor_tl, floor_tr, u);
            svg.push_str(&format!(
                r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{wall_stroke}" stroke-width="0.4" opacity="0.4"/>"#,
                p_near.0, p_near.1, p_far.0, p_far.1,
            ));
            gx += grid_step;
        }
        let mut gy = grid_step;
        while gy < area_d {
            let v = gy / area_d;
            let p_left = lerp_line(floor_bl, floor_tl, v);
            let p_right = lerp_line(floor_br, floor_tr, v);
            svg.push_str(&format!(
                r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{wall_stroke}" stroke-width="0.4" opacity="0.4"/>"#,
                p_left.0, p_left.1, p_right.0, p_right.1,
            ));
            gy += grid_step;
        }

        // --- Poles and luminaires ---
        for lum in placements {
            let (ex, ey) = lum.effective_position();
            let u = (ex / area_w).clamp(0.0, 1.0);
            let v = (ey / area_d).clamp(0.0, 1.0);

            let floor_pos = lerp_quad(floor_bl, floor_br, floor_tl, floor_tr, u, v);

            let lum_h_frac = (lum.mounting_height / mounting_height).min(1.0);
            let ceil_pos = lerp_quad(
                left_ceil_near,
                right_ceil_near,
                ceil_tl,
                ceil_tr,
                u, v,
            );
            let lum_x = floor_pos.0 + (ceil_pos.0 - floor_pos.0) * lum_h_frac;
            let lum_y = floor_pos.1 + (ceil_pos.1 - floor_pos.1) * lum_h_frac;

            // Pole line
            svg.push_str(&format!(
                r##"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#888" stroke-width="1.2" opacity="0.6"/>"##,
                floor_pos.0, floor_pos.1, lum_x, lum_y,
            ));

            // Light cone
            let cone_half = 8.0 + 12.0 * (1.0 - v);
            svg.push_str(&format!(
                r#"<polygon points="{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}" fill="rgba(255,220,100,0.12)" stroke="none"/>"#,
                lum_x, lum_y,
                floor_pos.0 - cone_half, floor_pos.1,
                floor_pos.0 + cone_half, floor_pos.1,
            ));

            // Luminaire head
            let head_r = 3.0 + 2.0 * (1.0 - v);
            svg.push_str(&format!(
                r#"<circle cx="{:.1}" cy="{:.1}" r="{head_r:.1}" fill="rgba(255,200,50,0.9)" stroke="rgb(200,160,30)" stroke-width="1"/>"#,
                lum_x, lum_y,
            ));

            // Floor shadow
            svg.push_str(&format!(
                r#"<ellipse cx="{:.1}" cy="{:.1}" rx="{:.1}" ry="{:.1}" fill="rgba(0,0,0,0.15)"/>"#,
                floor_pos.0, floor_pos.1, 4.0 + 3.0 * (1.0 - v), 1.5 + 1.0 * (1.0 - v),
            ));
        }

        // --- Dimension labels ---
        let dl = units.distance_label();
        let w_display = units.convert_meters(area_w);
        let d_display = units.convert_meters(area_d);
        let h_display = units.convert_meters(mounting_height);

        let w_mid = ((floor_bl.0 + floor_br.0) / 2.0, floor_bl.1 + 14.0);
        svg.push_str(&format!(
            r#"<text x="{:.0}" y="{:.0}" text-anchor="middle" fill="{text_color}" font-size="10">{w_display:.0} {dl}</text>"#,
            w_mid.0, w_mid.1,
        ));

        let d_mid = ((floor_bl.0 + floor_tl.0) / 2.0 - 12.0, (floor_bl.1 + floor_tl.1) / 2.0);
        let angle = ((floor_tl.1 - floor_bl.1) / (floor_tl.0 - floor_bl.0)).atan().to_degrees();
        svg.push_str(&format!(
            r#"<text x="{:.0}" y="{:.0}" text-anchor="middle" fill="{text_color}" font-size="10" transform="rotate({angle:.1}, {:.0}, {:.0})">{d_display:.0} {dl}</text>"#,
            d_mid.0, d_mid.1, d_mid.0, d_mid.1,
        ));

        let h_label_x = floor_bl.0 - 8.0;
        let h_label_top = left_ceil_near.1;
        let h_label_mid = (floor_bl.1 + h_label_top) / 2.0;
        svg.push_str(&format!(
            r#"<line x1="{h_label_x:.0}" y1="{:.0}" x2="{h_label_x:.0}" y2="{:.0}" stroke="{text_color}" stroke-width="0.8"/>"#,
            floor_bl.1, h_label_top,
        ));
        svg.push_str(&format!(
            r#"<text x="{:.0}" y="{:.0}" text-anchor="middle" fill="{text_color}" font-size="9" transform="rotate(-90, {:.0}, {:.0})">{h_display:.1} {dl}</text>"#,
            h_label_x - 10.0, h_label_mid, h_label_x - 10.0, h_label_mid,
        ));

        // --- Statistics bar ---
        let illu_label = units.illuminance_label();
        let min_d = fmt_lux(units.convert_lux(result.min_lux));
        let avg_d = fmt_lux(units.convert_lux(result.avg_lux));
        let max_d = fmt_lux(units.convert_lux(result.max_lux));
        svg.push_str(&format!(
            r#"<text x="{:.0}" y="{:.0}" fill="{text_color}" font-size="9">Floor — Min: {min_d}  Avg: {avg_d}  Max: {max_d} {illu_label}  |  U₀: {:.2}  |  Back wall max: {} {illu_label}  |  Left wall max: {} {illu_label}</text>"#,
            floor_bl.0, svg_height - 6.0,
            result.uniformity_min_avg,
            fmt_lux(units.convert_lux(back_max)),
            fmt_lux(units.convert_lux(left_max)),
        ));

        // --- Color legend ---
        if global_max > 0.0 {
            let legend_x = svg_width - 70.0;
            let legend_h = 80.0;
            let legend_top = svg_height - legend_h - 25.0;
            let legend_w = 10.0;
            let num_seg = 20;

            for i in 0..num_seg {
                let t = i as f64 / num_seg as f64;
                let color = heatmap_color(1.0 - t);
                let seg_h = legend_h / num_seg as f64;
                svg.push_str(&format!(
                    r#"<rect x="{legend_x}" y="{:.1}" width="{legend_w}" height="{:.1}" fill="{}"/>"#,
                    legend_top + t * legend_h, seg_h + 0.5, color.to_rgb_string(),
                ));
            }

            let illu = units.illuminance_label();
            for i in [0, num_seg / 2, num_seg] {
                let t = i as f64 / num_seg as f64;
                let lux_val = global_max * (1.0 - t);
                let y = legend_top + t * legend_h;
                svg.push_str(&format!(
                    r#"<text x="{:.0}" y="{y:.1}" fill="{text_color}" font-size="7" dominant-baseline="middle">{} {illu}</text>"#,
                    legend_x + legend_w + 3.0, fmt_lux(units.convert_lux(lux_val)),
                ));
            }
        }

        svg.push_str("</svg>");
        svg
    }
}

/// Render a wall heatmap onto a perspective quad.
fn render_wall_heatmap(
    svg: &mut String,
    lux_grid: &[Vec<f64>],
    wall_res: usize,
    global_max: f64,
    bl: (f64, f64),
    br: (f64, f64),
    tl: (f64, f64),
    tr: (f64, f64),
) {
    if global_max <= 0.0 {
        return;
    }
    for row in 0..wall_res {
        for col in 0..wall_res {
            let lux = lux_grid[row][col];
            let normalized = lux / global_max;
            let color = heatmap_color(normalized);

            let u0 = col as f64 / wall_res as f64;
            let u1 = (col + 1) as f64 / wall_res as f64;
            let v0 = row as f64 / wall_res as f64;
            let v1 = (row + 1) as f64 / wall_res as f64;

            // v goes top→bottom for walls (row 0 = ceiling, row N = floor)
            // but the quad tl/tr is ceiling and bl/br is floor
            // So we invert: v=0 maps to top (tl/tr), v=1 maps to bottom (bl/br)
            let p00 = lerp_quad(bl, br, tl, tr, u0, 1.0 - v0);
            let p10 = lerp_quad(bl, br, tl, tr, u1, 1.0 - v0);
            let p11 = lerp_quad(bl, br, tl, tr, u1, 1.0 - v1);
            let p01 = lerp_quad(bl, br, tl, tr, u0, 1.0 - v1);

            svg.push_str(&format!(
                r#"<polygon points="{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}" fill="{}" stroke="{}" stroke-width="0.15"/>"#,
                p00.0, p00.1, p10.0, p10.1, p11.0, p11.1, p01.0, p01.1,
                color.to_rgb_string(), color.to_rgb_string(),
            ));
        }
    }
}

/// Bilinear interpolation on a quad defined by 4 corners.
///
/// `bl`=bottom-left, `br`=bottom-right, `tl`=top-left(far), `tr`=top-right(far).
/// `u` goes left→right [0,1], `v` goes near→far [0,1].
fn lerp_quad(
    bl: (f64, f64), br: (f64, f64),
    tl: (f64, f64), tr: (f64, f64),
    u: f64, v: f64,
) -> (f64, f64) {
    let near_x = bl.0 + (br.0 - bl.0) * u;
    let near_y = bl.1 + (br.1 - bl.1) * u;
    let far_x = tl.0 + (tr.0 - tl.0) * u;
    let far_y = tl.1 + (tr.1 - tl.1) * u;
    (near_x + (far_x - near_x) * v, near_y + (far_y - near_y) * v)
}

/// Linear interpolation between two points.
fn lerp_line(a: (f64, f64), b: (f64, f64), t: f64) -> (f64, f64) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

/// Choose a grid step that gives ~4-8 lines.
fn adaptive_grid_step(extent: f64) -> f64 {
    if extent <= 20.0 { 5.0 }
    else if extent <= 50.0 { 10.0 }
    else if extent <= 120.0 { 20.0 }
    else { 50.0 }
}

/// Format lux/fc value with adaptive precision.
fn fmt_lux(val: f64) -> String {
    let abs = val.abs();
    if abs < 0.1 {
        format!("{val:.2}")
    } else if abs < 10.0 {
        format!("{val:.1}")
    } else {
        format!("{val:.0}")
    }
}

/// Extract approximate midpoint from an SVG path string for label placement.
fn extract_path_midpoint(path: &str) -> Option<(f64, f64)> {
    let coords: Vec<(f64, f64)> = path
        .split(&['M', 'L', ' '][..])
        .filter_map(|s| {
            let parts: Vec<&str> = s.split(',').collect();
            if parts.len() == 2 {
                Some((parts[0].parse::<f64>().ok()?, parts[1].parse::<f64>().ok()?))
            } else {
                None
            }
        })
        .collect();

    if coords.is_empty() {
        return None;
    }

    let mid = coords.len() / 2;
    Some(coords[mid])
}
