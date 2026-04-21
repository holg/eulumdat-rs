//! SVG plan-view renderer for a street lighting layout.
//!
//! Given a [`StreetLayout`] and a computed [`AreaResult`] (lux grid), this
//! produces a top-down image of the road with lane markings, sidewalks,
//! pole markers, an illuminance heatmap with isolux contours, and a
//! color-scale legend. Optional red-tint overlay highlights cells that
//! fall below a compliance threshold.
//!
//! The output is a self-contained `<svg>` string — ready to feed into
//! `inner_html` in Leptos, a Typst template, or a static HTML page.

use super::layout::StreetLayout;
use crate::area::AreaResult;
use crate::diagram::color::heatmap_color;
use crate::diagram::contour::marching_squares;

/// Visual theme: picks background + text + lane-marking colors.
///
/// For now the renderer ships two presets. Callers pick one based on their
/// host theme; a future version may read CSS custom properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StreetTheme {
    #[default]
    Dark,
    Light,
}

struct Palette {
    bg: &'static str,
    road: &'static str,
    sidewalk: &'static str,
    curb: &'static str,
    lane_marking: &'static str,
    edge_line: &'static str,
    pole_near: &'static str,
    pole_far: &'static str,
    text: &'static str,
}

impl StreetTheme {
    fn palette(self) -> Palette {
        match self {
            Self::Dark => Palette {
                bg: "#1b1f24",
                road: "#2b2f36",
                sidewalk: "#3a3f45",
                curb: "#5a5f66",
                lane_marking: "#e8e8e8",
                edge_line: "#ffffff",
                pole_near: "#4fc3f7",
                pole_far: "#ff8a65",
                text: "#e8e8e8",
            },
            Self::Light => Palette {
                bg: "#f6f6f6",
                road: "#3a3f45",
                sidewalk: "#c8c8c8",
                curb: "#888888",
                lane_marking: "#f5f5f5",
                edge_line: "#ffffff",
                pole_near: "#0277bd",
                pole_far: "#d84315",
                text: "#1b1f24",
            },
        }
    }
}

/// Compliance threshold used for the optional red-tint overlay.
///
/// Cells whose lux < `avg * ratio_floor` are painted red to show where the
/// design fails the selected standard's uniformity criterion. Use 0.4 for
/// typical roadway specs (min/avg ≥ 0.4 ⇔ avg/min ≤ 2.5); RP-8 Major at
/// 3:1 would use 0.33, EN 13201 C-classes 0.6, and so on. Passing `None`
/// disables the overlay entirely.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FailureOverlay {
    /// Minimum ratio of cell lux to average lux. Cells below this are tinted red.
    pub ratio_floor: f64,
}

/// Options controlling the rendered plan view.
#[derive(Debug, Clone, Copy)]
pub struct PlanViewOptions {
    pub theme: StreetTheme,
    pub failure_overlay: Option<FailureOverlay>,
}

impl Default for PlanViewOptions {
    fn default() -> Self {
        Self {
            theme: StreetTheme::Dark,
            failure_overlay: None,
        }
    }
}

/// Render a plan view of the street with lux heatmap and pole markers.
///
/// The SVG's logical coordinate system is laid out so the road runs along
/// the X axis with length `layout.pole_spacing_m` (one evaluation cycle —
/// the same range `StreetLayout::compute` uses). This keeps the heatmap
/// grid aligned with the poles visible at the edges of the frame.
pub fn plan_view_heatmap(
    layout: &StreetLayout,
    result: &AreaResult,
    svg_width: f64,
    svg_height: f64,
    opts: PlanViewOptions,
) -> String {
    let palette = opts.theme.palette();

    // Margins leave room for the legend on the right and scale bar at bottom.
    let margin_left = 24.0;
    let margin_right = 90.0;
    let margin_top = 24.0;
    let margin_bottom = 40.0;
    let plot_w = (svg_width - margin_left - margin_right).max(50.0);
    let plot_h = (svg_height - margin_top - margin_bottom).max(50.0);

    // World dimensions (metres). Length is one pole-spacing cycle to match
    // the evaluation window used by StreetLayout::compute().
    let world_len = result.area_width.max(1.0);
    let road_width = layout.roadway_width_m().max(0.1);
    let sidewalk = layout.sidewalk_width_m.max(0.0);
    let total_world_h = road_width + 2.0 * sidewalk;

    // Fit the road into the plot box preserving aspect ratio. Roads are
    // long-and-thin, so height is the typical binding dimension.
    let scale_x = plot_w / world_len;
    let scale_y = plot_h / total_world_h;
    let scale = scale_x.min(scale_y);
    let draw_w = world_len * scale;
    let draw_h = total_world_h * scale;

    // Center the road in the plot box.
    let offset_x = margin_left + (plot_w - draw_w) / 2.0;
    let offset_y = margin_top + (plot_h - draw_h) / 2.0;

    // World-to-screen helpers (closures capture `scale`, `offset_*`).
    let wx = |m: f64| -> f64 { offset_x + m * scale };
    // World Y = 0 is the near-curb roadway edge. Sidewalk sits above it in
    // world coords; flip so SVG Y grows downward visually like a plan view.
    let wy = |m: f64| -> f64 { offset_y + (m + sidewalk) * scale };

    let mut svg = String::new();
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {svg_width} {svg_height}" font-family="sans-serif">"#,
    ));
    svg.push_str(&format!(
        r#"<rect width="{svg_width}" height="{svg_height}" fill="{}"/>"#,
        palette.bg
    ));

    // ── Sidewalks ──────────────────────────────────────────────────────────
    if sidewalk > 0.0 {
        // Near sidewalk (y = -sidewalk .. 0)
        svg.push_str(&format!(
            r#"<rect x="{x:.2}" y="{y:.2}" width="{w:.2}" height="{h:.2}" fill="{fill}"/>"#,
            x = wx(0.0),
            y = wy(-sidewalk),
            w = draw_w,
            h = sidewalk * scale,
            fill = palette.sidewalk
        ));
        // Far sidewalk (y = road_width .. road_width + sidewalk)
        svg.push_str(&format!(
            r#"<rect x="{x:.2}" y="{y:.2}" width="{w:.2}" height="{h:.2}" fill="{fill}"/>"#,
            x = wx(0.0),
            y = wy(road_width),
            w = draw_w,
            h = sidewalk * scale,
            fill = palette.sidewalk
        ));
    }

    // ── Road surface ──────────────────────────────────────────────────────
    svg.push_str(&format!(
        r#"<rect x="{x:.2}" y="{y:.2}" width="{w:.2}" height="{h:.2}" fill="{fill}"/>"#,
        x = wx(0.0),
        y = wy(0.0),
        w = draw_w,
        h = road_width * scale,
        fill = palette.road
    ));

    // ── Illuminance heatmap (on road surface) ─────────────────────────────
    let n = result.grid_resolution;
    if n > 0 && !result.lux_grid.is_empty() && result.avg_lux > 0.0 {
        let max_lux = result.max_lux.max(1e-6);
        let cell_w = world_len / n as f64 * scale;
        let cell_h = road_width / n as f64 * scale;
        for (row, grid_row) in result.lux_grid.iter().enumerate() {
            for (col, &lux) in grid_row.iter().enumerate() {
                let normalized = (lux / max_lux).clamp(0.0, 1.0);
                let color = heatmap_color(normalized);
                let sx = wx(0.0) + col as f64 * cell_w;
                let sy = wy(0.0) + row as f64 * cell_h;
                svg.push_str(&format!(
                    r#"<rect x="{sx:.2}" y="{sy:.2}" width="{w:.2}" height="{h:.2}" fill="{c}" opacity="0.75"/>"#,
                    w = cell_w + 0.5,
                    h = cell_h + 0.5,
                    c = color.to_rgb_string()
                ));

                // Red-tint overlay for cells below the failure threshold.
                if let Some(overlay) = opts.failure_overlay {
                    let threshold = result.avg_lux * overlay.ratio_floor;
                    if lux < threshold {
                        let w = cell_w + 0.5;
                        let h = cell_h + 0.5;
                        svg.push_str(&format!(
                            r##"<rect x="{sx:.2}" y="{sy:.2}" width="{w:.2}" height="{h:.2}" fill="#ff3b3b" opacity="0.45"/>"##,
                        ));
                    }
                }
            }
        }

        // ── Isolux contours ───────────────────────────────────────────────
        let contour_levels = [2.0, 5.0, 10.0, 20.0, 50.0];
        let x_coords: Vec<f64> = (0..n)
            .map(|col| wx(0.0) + (col as f64 + 0.5) * cell_w)
            .collect();
        let y_coords: Vec<f64> = (0..n)
            .map(|row| wy(0.0) + (row as f64 + 0.5) * cell_h)
            .collect();
        for &level in &contour_levels {
            if level > max_lux || level <= 0.0 {
                continue;
            }
            let cl = marching_squares(&result.lux_grid, &x_coords, &y_coords, level);
            for path in &cl.paths {
                svg.push_str(&format!(
                    r#"<path d="{path}" fill="none" stroke="rgba(255,255,255,0.7)" stroke-width="0.9"/>"#,
                ));
            }
        }
    }

    // ── Lane markings ─────────────────────────────────────────────────────
    // Interior dashed lines between lanes, solid edge lines at outer bounds.
    let dash = format!(
        r#"stroke="{}" stroke-width="1.4" stroke-dasharray="8,6""#,
        palette.lane_marking
    );
    for i in 1..layout.num_lanes {
        let y = wy(i as f64 * layout.lane_width_m);
        svg.push_str(&format!(
            r#"<line x1="{x1:.2}" y1="{y:.2}" x2="{x2:.2}" y2="{y:.2}" {dash}/>"#,
            x1 = wx(0.0),
            x2 = wx(world_len),
        ));
    }
    // Edge lines (solid white)
    let edge_w = 1.8;
    svg.push_str(&format!(
        r#"<line x1="{x1:.2}" y1="{y:.2}" x2="{x2:.2}" y2="{y:.2}" stroke="{c}" stroke-width="{edge_w}"/>"#,
        x1 = wx(0.0),
        x2 = wx(world_len),
        y = wy(0.0),
        c = palette.edge_line
    ));
    svg.push_str(&format!(
        r#"<line x1="{x1:.2}" y1="{y:.2}" x2="{x2:.2}" y2="{y:.2}" stroke="{c}" stroke-width="{edge_w}"/>"#,
        x1 = wx(0.0),
        x2 = wx(world_len),
        y = wy(road_width),
        c = palette.edge_line
    ));

    // ── Curbs (between road and sidewalk) ─────────────────────────────────
    if sidewalk > 0.0 {
        for curb_y in [0.0, road_width] {
            svg.push_str(&format!(
                r#"<line x1="{x1:.2}" y1="{y:.2}" x2="{x2:.2}" y2="{y:.2}" stroke="{c}" stroke-width="0.8"/>"#,
                x1 = wx(0.0),
                x2 = wx(world_len),
                y = wy(curb_y),
                c = palette.curb
            ));
        }
    }

    // ── Pole markers + arm indicators ─────────────────────────────────────
    // Use the same placement logic as the simulation so what the user sees
    // on screen matches the computed illuminance.
    for p in layout.placements() {
        // Only draw poles inside the evaluation window.
        if p.x < -0.5 || p.x > world_len + 0.5 {
            continue;
        }
        let is_near = p.y < 0.0;
        let color = if is_near {
            palette.pole_near
        } else {
            palette.pole_far
        };
        let px = wx(p.x);
        let py = wy(p.y);
        // Arm: from pole base toward luminaire head (tip lies at the
        // effective (x, y) used in compute_area_illuminance).
        let (ex, ey) = p.effective_position();
        let arm_x = wx(ex);
        let arm_y = wy(ey);
        svg.push_str(&format!(
            r#"<line x1="{px:.2}" y1="{py:.2}" x2="{arm_x:.2}" y2="{arm_y:.2}" stroke="{color}" stroke-width="1.6"/>"#,
        ));
        // Pole base circle
        svg.push_str(&format!(
            r##"<circle cx="{px:.2}" cy="{py:.2}" r="3.5" fill="{color}" stroke="#000" stroke-width="0.6"/>"##,
        ));
        // Luminaire head (smaller, at arm tip)
        svg.push_str(&format!(
            r##"<circle cx="{arm_x:.2}" cy="{arm_y:.2}" r="2.2" fill="#fff" stroke="{color}" stroke-width="1.2"/>"##,
        ));
    }

    // ── Scale bar ─────────────────────────────────────────────────────────
    {
        let target_m = pick_scale_length(world_len);
        let bar_len = target_m * scale;
        let bar_y = svg_height - margin_bottom + 14.0;
        let bar_x = margin_left;
        svg.push_str(&format!(
            r#"<line x1="{bar_x:.2}" y1="{bar_y:.2}" x2="{x2:.2}" y2="{bar_y:.2}" stroke="{c}" stroke-width="2"/>"#,
            x2 = bar_x + bar_len,
            c = palette.text
        ));
        svg.push_str(&format!(
            r#"<line x1="{bar_x:.2}" y1="{y1:.2}" x2="{bar_x:.2}" y2="{y2:.2}" stroke="{c}" stroke-width="2"/>"#,
            y1 = bar_y - 4.0,
            y2 = bar_y + 4.0,
            c = palette.text
        ));
        svg.push_str(&format!(
            r#"<line x1="{x:.2}" y1="{y1:.2}" x2="{x:.2}" y2="{y2:.2}" stroke="{c}" stroke-width="2"/>"#,
            x = bar_x + bar_len,
            y1 = bar_y - 4.0,
            y2 = bar_y + 4.0,
            c = palette.text
        ));
        svg.push_str(&format!(
            r#"<text x="{tx:.2}" y="{ty:.2}" fill="{c}" font-size="10" text-anchor="start">{target_m:.0} m</text>"#,
            tx = bar_x + bar_len + 6.0,
            ty = bar_y + 3.5,
            c = palette.text
        ));
    }

    // ── Illuminance legend (color scale on the right) ─────────────────────
    if result.max_lux > 0.0 {
        let legend_x = svg_width - margin_right + 14.0;
        let legend_w = 14.0;
        let legend_h = plot_h * 0.7;
        let legend_top = margin_top + (plot_h - legend_h) / 2.0;
        let segments = 24;
        let seg_h = legend_h / segments as f64;
        for i in 0..segments {
            let t = 1.0 - (i as f64 + 0.5) / segments as f64;
            let color = heatmap_color(t);
            svg.push_str(&format!(
                r#"<rect x="{legend_x:.2}" y="{y:.2}" width="{legend_w}" height="{seg_h:.2}" fill="{c}"/>"#,
                y = legend_top + i as f64 * seg_h,
                c = color.to_rgb_string()
            ));
        }
        // 0 / mid / max labels
        let tick = |lux: f64, y: f64| {
            let tx = legend_x + legend_w + 4.0;
            let ty = y + 3.0;
            let c = palette.text;
            format!(
                r#"<text x="{tx:.2}" y="{ty:.2}" fill="{c}" font-size="9" text-anchor="start">{lux:.0}</text>"#,
            )
        };
        svg.push_str(&tick(result.max_lux, legend_top));
        svg.push_str(&tick(result.max_lux * 0.5, legend_top + legend_h * 0.5));
        svg.push_str(&tick(0.0, legend_top + legend_h));
        svg.push_str(&format!(
            r#"<text x="{tx:.2}" y="{ty:.2}" fill="{c}" font-size="9" text-anchor="start">lux</text>"#,
            tx = legend_x,
            ty = legend_top - 6.0,
            c = palette.text
        ));
    }

    // ── Key strip (pole colors + road elements), bottom-right ─────────────
    {
        let key_x = svg_width - margin_right + 8.0;
        let key_y_top = margin_top + plot_h * 0.7 + 20.0;
        let line_h = 14.0;
        let entries: [(&str, &str); 4] = [
            ("near pole", palette.pole_near),
            ("far pole", palette.pole_far),
            ("marking", palette.lane_marking),
            ("curb", palette.curb),
        ];
        for (i, (label, color)) in entries.iter().enumerate() {
            let y = key_y_top + i as f64 * line_h;
            svg.push_str(&format!(
                r#"<rect x="{x:.2}" y="{y:.2}" width="10" height="10" fill="{color}"/>"#,
                x = key_x,
            ));
            svg.push_str(&format!(
                r#"<text x="{tx:.2}" y="{ty:.2}" fill="{c}" font-size="9" text-anchor="start">{label}</text>"#,
                tx = key_x + 14.0,
                ty = y + 9.0,
                c = palette.text
            ));
        }
    }

    svg.push_str("</svg>");
    svg
}

/// Pick a round-number scale-bar length for a given world span.
fn pick_scale_length(world_len: f64) -> f64 {
    let target = world_len / 6.0;
    let candidates = [1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0];
    *candidates
        .iter()
        .rev()
        .find(|&&c| c <= target)
        .unwrap_or(&1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::street::Arrangement;
    use crate::Eulumdat;

    fn load_road() -> Eulumdat {
        let content =
            std::fs::read_to_string("../eulumdat-wasm/templates/road_luminaire.ldt").unwrap();
        Eulumdat::parse(&content).unwrap()
    }

    fn compute_result(layout: &StreetLayout) -> AreaResult {
        let ldt = load_road();
        layout.compute(&ldt, 0.8)
    }

    #[test]
    fn renders_basic_elements() {
        let layout = StreetLayout::default();
        let result = compute_result(&layout);
        let svg = plan_view_heatmap(&layout, &result, 800.0, 300.0, PlanViewOptions::default());
        // SVG root + road rect + legend label present.
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("viewBox=\"0 0 800 300\""));
        assert!(svg.contains("fill=\"#2b2f36\"")); // dark road
        assert!(svg.contains("lux"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn paints_sidewalks_when_configured() {
        let layout = StreetLayout {
            sidewalk_width_m: 2.0,
            ..Default::default()
        };
        let result = compute_result(&layout);
        let svg = plan_view_heatmap(&layout, &result, 800.0, 300.0, PlanViewOptions::default());
        // Sidewalk color (dark theme) shows up twice (both sides).
        let hits = svg.matches("#3a3f45").count();
        assert!(hits >= 2, "expected ≥2 sidewalk fills, got {hits}");
    }

    #[test]
    fn omits_sidewalks_when_zero_width() {
        let layout = StreetLayout {
            sidewalk_width_m: 0.0,
            ..Default::default()
        };
        let result = compute_result(&layout);
        let svg = plan_view_heatmap(&layout, &result, 800.0, 300.0, PlanViewOptions::default());
        // Sidewalk fill should not be referenced when width == 0.
        assert!(!svg.contains("#3a3f45"));
    }

    #[test]
    fn red_overlay_only_appears_when_requested() {
        let layout = StreetLayout::default();
        let result = compute_result(&layout);

        let plain = plan_view_heatmap(&layout, &result, 800.0, 300.0, PlanViewOptions::default());
        assert!(!plain.contains("#ff3b3b"));

        let with_overlay = plan_view_heatmap(
            &layout,
            &result,
            800.0,
            300.0,
            PlanViewOptions {
                theme: StreetTheme::Dark,
                // Aggressive threshold — guarantees most cells fail so we see
                // the red tint appear at least once.
                failure_overlay: Some(FailureOverlay { ratio_floor: 10.0 }),
            },
        );
        assert!(with_overlay.contains("#ff3b3b"));
    }

    #[test]
    fn staggered_layout_shows_both_pole_colors() {
        let layout = StreetLayout {
            arrangement: Arrangement::Staggered,
            ..Default::default()
        };
        let result = compute_result(&layout);
        let svg = plan_view_heatmap(&layout, &result, 800.0, 300.0, PlanViewOptions::default());
        // Dark-theme near + far pole colors both present.
        assert!(svg.contains("#4fc3f7"), "near-pole color missing");
        assert!(svg.contains("#ff8a65"), "far-pole color missing");
    }

    #[test]
    fn light_theme_switches_palette() {
        let layout = StreetLayout::default();
        let result = compute_result(&layout);
        let svg = plan_view_heatmap(
            &layout,
            &result,
            800.0,
            300.0,
            PlanViewOptions {
                theme: StreetTheme::Light,
                failure_overlay: None,
            },
        );
        assert!(svg.contains("#f6f6f6"), "light background missing");
        // Dark-theme road color (unique to dark palette) must not appear.
        assert!(
            !svg.contains("#2b2f36"),
            "dark-theme road color leaked into light theme"
        );
    }

    #[test]
    fn picks_sensible_scale_bar_lengths() {
        assert_eq!(pick_scale_length(30.0), 5.0);
        assert_eq!(pick_scale_length(120.0), 20.0);
        assert_eq!(pick_scale_length(600.0), 100.0);
        assert_eq!(pick_scale_length(2.0), 1.0); // tiny road
    }
}
