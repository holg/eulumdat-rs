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

use super::layout::{Arrangement, StreetLayout};
use super::optimize::OptimizationCandidate;
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
/// Different standards express their failure criterion differently:
///
/// - **RP-8 / CJJ 45** use uniformity ratios (`min/avg ≥ U₀` or
///   `avg/min ≤ ratio`) that translate to a *relative* floor — cells
///   dimmer than `avg × ratio_floor` fail.
/// - **EN 13201 C/P-classes** use an *absolute* minimum illuminance
///   (`E_min ≥ L lux`), so the floor is independent of the grid's mean.
///
/// Use [`Self::RatioFloor`] for the first family and [`Self::AbsoluteLux`]
/// for the second. Passing `None` to the renderer disables the overlay.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FailureOverlay {
    /// Tint cells where `lux < avg_lux × min_over_avg`.
    ///
    /// `min_over_avg = 0.4` is a reasonable roadway default
    /// (min/avg ≥ 0.4 ⇔ avg/min ≤ 2.5). RP-8 Major/Collector at 3:1 uses
    /// ≈ 0.33; CJJ 45 Class III uses 0.35.
    RatioFloor { min_over_avg: f64 },
    /// Tint cells where `lux < min_lux` (EN 13201 absolute minimum).
    AbsoluteLux { min_lux: f64 },
}

impl FailureOverlay {
    /// Convenience constructor for the common ratio-based case.
    pub fn ratio(min_over_avg: f64) -> Self {
        Self::RatioFloor { min_over_avg }
    }

    /// Convenience constructor for the absolute-lux case.
    pub fn absolute(min_lux: f64) -> Self {
        Self::AbsoluteLux { min_lux }
    }

    /// Does the cell at `cell_lux` (on a grid whose mean is `avg_lux`)
    /// count as failing this overlay?
    pub fn is_failing(&self, cell_lux: f64, avg_lux: f64) -> bool {
        match *self {
            Self::RatioFloor { min_over_avg } => cell_lux < avg_lux * min_over_avg,
            Self::AbsoluteLux { min_lux } => cell_lux < min_lux,
        }
    }
}

/// Options controlling the rendered plan view.
#[derive(Debug, Clone, Copy)]
pub struct PlanViewOptions {
    pub theme: StreetTheme,
    pub failure_overlay: Option<FailureOverlay>,
    /// When true, overlay tiny dots at every illuminance grid cell
    /// centre. Standards pin the grid spacing (RP-8 / EN 13201 each
    /// dictate it from `pole_spacing` and lane width), so this layer
    /// helps verify the right cells are being measured.
    pub show_grid_points: bool,
}

impl Default for PlanViewOptions {
    fn default() -> Self {
        Self {
            theme: StreetTheme::Dark,
            failure_overlay: None,
            show_grid_points: false,
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
    // Only paint the solid sidewalk fill when there is no heatmap data
    // covering the sidewalk band. When `compute_with_sidewalks` fed this
    // renderer, the heatmap cells already span the sidewalks; painting the
    // solid fill on top would hide them.
    let heatmap_covers_sidewalks =
        sidewalk > 0.0 && !result.lux_grid.is_empty() && result.area_depth > road_width + 1e-6;
    if sidewalk > 0.0 && !heatmap_covers_sidewalks {
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

    // ── Illuminance heatmap ───────────────────────────────────────────────
    //
    // If the caller supplied a sidewalk-widened grid (`area_depth` exceeds
    // `road_width`), the extra rows sit symmetrically on both sides of the
    // road — anchor the heatmap at world Y = -grid_y_pad_m so sidewalk cells
    // land in the sidewalk band. For a plain roadway grid `grid_y_pad_m`
    // is 0 and behavior matches the original renderer.
    let n = result.grid_resolution;
    let grid_y_span = result.area_depth.max(road_width);
    let grid_y_pad_m = ((grid_y_span - road_width) / 2.0).max(0.0);
    if n > 0 && !result.lux_grid.is_empty() && result.avg_lux > 0.0 {
        let max_lux = result.max_lux.max(1e-6);
        let cell_w = world_len / n as f64 * scale;
        let cell_h = grid_y_span / n as f64 * scale;
        let grid_top_world_y = -grid_y_pad_m;
        for (row, grid_row) in result.lux_grid.iter().enumerate() {
            for (col, &lux) in grid_row.iter().enumerate() {
                let normalized = (lux / max_lux).clamp(0.0, 1.0);
                let color = heatmap_color(normalized);
                let sx = wx(0.0) + col as f64 * cell_w;
                let sy = wy(grid_top_world_y) + row as f64 * cell_h;
                svg.push_str(&format!(
                    r#"<rect x="{sx:.2}" y="{sy:.2}" width="{w:.2}" height="{h:.2}" fill="{c}" opacity="0.75"/>"#,
                    w = cell_w + 0.5,
                    h = cell_h + 0.5,
                    c = color.to_rgb_string()
                ));

                // Red-tint overlay for cells below the failure threshold.
                // Only applied to cells on the roadway — sidewalks aren't
                // subject to the same criteria.
                if let Some(overlay) = opts.failure_overlay {
                    let cell_world_y =
                        grid_top_world_y + (row as f64 + 0.5) * grid_y_span / n as f64;
                    let on_road = (0.0..=road_width).contains(&cell_world_y);
                    if on_road && overlay.is_failing(lux, result.avg_lux) {
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
            .map(|row| wy(grid_top_world_y) + (row as f64 + 0.5) * cell_h)
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

        // ── Calculation grid points (Richard #6) ──────────────────────────
        // Tiny crosses at every cell centre so the user can verify the
        // standard's prescribed grid is what's being measured (RP-8: 10
        // longitudinal × n_lanes × 4 transverse; EN 13201 likewise).
        if opts.show_grid_points {
            for &cy in &y_coords {
                for &cx in &x_coords {
                    svg.push_str(&format!(
                        r##"<circle cx="{cx:.2}" cy="{cy:.2}" r="1.2" fill="none" stroke="#ffffff" stroke-width="0.6" opacity="0.85"/>"##,
                    ));
                }
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

// ─────────────────────────────────────────────────────────────────────────
// Layout trade-off chart (Pareto scatter)
// ─────────────────────────────────────────────────────────────────────────

/// Visual options for [`layout_tradeoff_chart`].
#[derive(Debug, Clone, Copy)]
pub struct LayoutTradeoffOptions {
    pub theme: StreetTheme,
    /// Optional index into the candidate slice; if set, that marker is
    /// drawn with a halo so the user can spot the layout currently
    /// selected in their UI.
    pub highlight_idx: Option<usize>,
}

impl Default for LayoutTradeoffOptions {
    fn default() -> Self {
        Self {
            theme: StreetTheme::Dark,
            highlight_idx: None,
        }
    }
}

/// Render a layout-trade-off scatter plot (Pareto front of optimizer candidates).
///
/// Each candidate plots at `(poles_per_km, avg_illuminance_lux)`. The
/// `frontier_indices` slice (typically the output of
/// [`super::optimize::pareto_front_tradeoff`]) draws a polyline through
/// the Pareto-optimal points so engineers can read off the
/// "no-design-strictly-better-than-this" choices at a glance.
///
/// Marker color encodes the overall uniformity U₀ (min/avg) on a
/// red→amber→green scale; marker size scales with luminous flux per
/// km so brighter installations visually pop. The highlighted index
/// (if any) is drawn with a white halo.
///
/// Returns a self-contained `<svg>` element. For an empty candidate
/// list, returns a tiny "no data" SVG so callers can drop the result
/// into an `inner_html` slot without conditionally wrapping it.
pub fn layout_tradeoff_chart(
    candidates: &[OptimizationCandidate],
    frontier_indices: &[usize],
    svg_width: f64,
    svg_height: f64,
    opts: LayoutTradeoffOptions,
) -> String {
    let palette = opts.theme.palette();

    let mut svg = String::new();
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {svg_width} {svg_height}" font-family="sans-serif">"#,
    ));
    svg.push_str(&format!(
        r#"<rect width="{svg_width}" height="{svg_height}" fill="{}"/>"#,
        palette.bg
    ));

    if candidates.is_empty() {
        svg.push_str(&format!(
            r#"<text x="{x:.1}" y="{y:.1}" fill="{c}" font-size="13" text-anchor="middle">No passing layouts to plot</text>"#,
            x = svg_width / 2.0,
            y = svg_height / 2.0,
            c = palette.text,
        ));
        svg.push_str("</svg>");
        return svg;
    }

    // Plot box.
    let margin_left = 60.0;
    let margin_right = 24.0;
    let margin_top = 28.0;
    let margin_bottom = 56.0;
    let plot_w = (svg_width - margin_left - margin_right).max(50.0);
    let plot_h = (svg_height - margin_top - margin_bottom).max(50.0);

    // Domain.
    let mut x_min = f64::INFINITY;
    let mut x_max = f64::NEG_INFINITY;
    let mut y_min = f64::INFINITY;
    let mut y_max = f64::NEG_INFINITY;
    let mut flux_max = 0.0_f64;
    for c in candidates {
        x_min = x_min.min(c.poles_per_km);
        x_max = x_max.max(c.poles_per_km);
        y_min = y_min.min(c.design.avg_illuminance_lux);
        y_max = y_max.max(c.design.avg_illuminance_lux);
        flux_max = flux_max.max(c.flux_per_km);
    }
    // Add ~5% padding on each axis so markers don't touch the frame.
    let x_pad = ((x_max - x_min) * 0.05).max(1.0);
    let y_pad = ((y_max - y_min) * 0.05).max(0.5);
    let x_lo = (x_min - x_pad).max(0.0);
    let x_hi = x_max + x_pad;
    let y_lo = (y_min - y_pad).max(0.0);
    let y_hi = y_max + y_pad;

    let to_x = |v: f64| -> f64 { margin_left + plot_w * ((v - x_lo) / (x_hi - x_lo).max(1e-9)) };
    // SVG Y grows downward; flip so higher lux is higher on screen.
    let to_y =
        |v: f64| -> f64 { margin_top + plot_h * (1.0 - (v - y_lo) / (y_hi - y_lo).max(1e-9)) };

    // Plot frame.
    svg.push_str(&format!(
        r#"<rect x="{x:.1}" y="{y:.1}" width="{w:.1}" height="{h:.1}" fill="none" stroke="{c}" stroke-width="1"/>"#,
        x = margin_left,
        y = margin_top,
        w = plot_w,
        h = plot_h,
        c = palette.curb,
    ));

    // Gridlines + tick labels (5 ticks each axis).
    for i in 0..=4 {
        let frac = i as f64 / 4.0;
        let xv = x_lo + frac * (x_hi - x_lo);
        let xpx = to_x(xv);
        svg.push_str(&format!(
            r##"<line x1="{xpx:.1}" y1="{y1:.1}" x2="{xpx:.1}" y2="{y2:.1}" stroke="{c}" stroke-width="0.5" stroke-dasharray="2,3" opacity="0.5"/>"##,
            y1 = margin_top,
            y2 = margin_top + plot_h,
            c = palette.curb,
        ));
        svg.push_str(&format!(
            r#"<text x="{xpx:.1}" y="{ty:.1}" fill="{c}" font-size="10" text-anchor="middle">{label:.0}</text>"#,
            ty = margin_top + plot_h + 14.0,
            c = palette.text,
            label = xv,
        ));

        let yv = y_lo + frac * (y_hi - y_lo);
        let ypx = to_y(yv);
        svg.push_str(&format!(
            r##"<line x1="{x1:.1}" y1="{ypx:.1}" x2="{x2:.1}" y2="{ypx:.1}" stroke="{c}" stroke-width="0.5" stroke-dasharray="2,3" opacity="0.5"/>"##,
            x1 = margin_left,
            x2 = margin_left + plot_w,
            c = palette.curb,
        ));
        svg.push_str(&format!(
            r#"<text x="{tx:.1}" y="{ypx:.1}" fill="{c}" font-size="10" text-anchor="end" dominant-baseline="middle">{label:.1}</text>"#,
            tx = margin_left - 6.0,
            c = palette.text,
            label = yv,
        ));
    }

    // Axis labels.
    svg.push_str(&format!(
        r#"<text x="{x:.1}" y="{y:.1}" fill="{c}" font-size="11" text-anchor="middle">Poles per km (Power axis)</text>"#,
        x = margin_left + plot_w / 2.0,
        y = svg_height - 12.0,
        c = palette.text,
    ));
    svg.push_str(&format!(
        r#"<text x="14" y="{y:.1}" fill="{c}" font-size="11" text-anchor="middle" transform="rotate(-90 14 {y:.1})">Average illuminance (lux) — quality axis</text>"#,
        y = margin_top + plot_h / 2.0,
        c = palette.text,
    ));

    // Frontier polyline (lower-X / higher-Y dominance edge).
    // We use a neutral dashed line (theme-aware text color) instead of
    // green so the frontier doesn't visually collide with high-U₀ green
    // markers — Richard's feedback after the road-lighting deep-dive.
    if frontier_indices.len() >= 2 {
        let path: String = frontier_indices
            .iter()
            .enumerate()
            .map(|(i, &idx)| {
                let c = &candidates[idx];
                let cmd = if i == 0 { 'M' } else { 'L' };
                format!(
                    "{cmd}{:.1},{:.1}",
                    to_x(c.poles_per_km),
                    to_y(c.design.avg_illuminance_lux)
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        svg.push_str(&format!(
            r##"<path d="{path}" fill="none" stroke="{c}" stroke-width="1.4" stroke-dasharray="6,3" opacity="0.7"/>"##,
            c = palette.text,
        ));
    }

    // Markers (drawn after the frontier so they sit on top of the line).
    // Each marker is wrapped in a <g class="tradeoff-marker" data-idx="N">
    // so the WASM editor can wire up click-to-apply via event delegation.
    // A <title> child gives free native browser tooltips on hover.
    let arrangement_label = |a: Arrangement| match a {
        Arrangement::SingleSide => "single-side",
        Arrangement::Opposite => "opposite",
        Arrangement::Staggered => "staggered",
    };
    for (i, c) in candidates.iter().enumerate() {
        let cx = to_x(c.poles_per_km);
        let cy = to_y(c.design.avg_illuminance_lux);
        let r = tradeoff_marker_radius(c.flux_per_km, flux_max);
        let fill = uniformity_color(c.design.uniformity_overall);
        let on_frontier = frontier_indices.contains(&i);

        svg.push_str(&format!(
            r#"<g class="tradeoff-marker" data-idx="{i}" data-spacing="{spacing:.1}" data-height="{height:.1}" data-arrangement="{arr}" style="cursor:pointer">"#,
            spacing = c.pole_spacing_m,
            height = c.mounting_height_m,
            arr = arrangement_label(c.arrangement),
        ));

        // Highlight halo for the user's currently-selected candidate.
        if Some(i) == opts.highlight_idx {
            svg.push_str(&format!(
                r##"<circle cx="{cx:.1}" cy="{cy:.1}" r="{rr:.1}" fill="none" stroke="#ffffff" stroke-width="2"/>"##,
                rr = r + 4.0,
            ));
        }

        // Frontier markers get a thicker outline (no green fill — keeps
        // U₀ color encoding readable).
        let stroke_w = if on_frontier { 1.8 } else { 0.6 };
        svg.push_str(&format!(
            r#"<circle cx="{cx:.1}" cy="{cy:.1}" r="{r:.1}" fill="{fill}" stroke="{stroke}" stroke-width="{stroke_w}"/>"#,
            stroke = palette.text,
        ));
        // Native tooltip — visible on hover in every browser.
        let frontier_tag = if on_frontier { " · Pareto" } else { "" };
        svg.push_str(&format!(
            "<title>{spacing:.0} m × {height:.0} m, {arr}\n{poles:.1} poles/km · U₀={u:.2} · {avg:.0} lx · {flux:.0} lm/km{tag}\nClick to apply</title>",
            spacing = c.pole_spacing_m,
            height = c.mounting_height_m,
            arr = arrangement_label(c.arrangement),
            poles = c.poles_per_km,
            u = c.design.uniformity_overall,
            avg = c.design.avg_illuminance_lux,
            flux = c.flux_per_km,
            tag = frontier_tag,
        ));
        svg.push_str("</g>");
    }

    // Mini-legend: U₀ color ramp + frontier line.
    let lx = margin_left + plot_w - 168.0;
    let ly = margin_top + 8.0;
    svg.push_str(&format!(
        r#"<rect x="{lx:.1}" y="{ly:.1}" width="160" height="62" fill="{bg}" opacity="0.85" stroke="{c}" stroke-width="0.5" rx="3"/>"#,
        bg = palette.road,
        c = palette.curb,
    ));
    svg.push_str(&format!(
        r#"<text x="{x:.1}" y="{y:.1}" fill="{c}" font-size="10" font-weight="bold">U₀ (min/avg)</text>"#,
        x = lx + 8.0,
        y = ly + 14.0,
        c = palette.text,
    ));
    for (j, &(u, label)) in [(0.2_f64, "low"), (0.4, "mid"), (0.6, "high")]
        .iter()
        .enumerate()
    {
        let xx = lx + 8.0 + j as f64 * 50.0;
        svg.push_str(&format!(
            r#"<circle cx="{cx:.1}" cy="{cy:.1}" r="4" fill="{fill}"/>"#,
            cx = xx + 4.0,
            cy = ly + 30.0,
            fill = uniformity_color(u),
        ));
        svg.push_str(&format!(
            r#"<text x="{tx:.1}" y="{ty:.1}" fill="{c}" font-size="9">{label}</text>"#,
            tx = xx + 12.0,
            ty = ly + 33.0,
            c = palette.text,
        ));
    }

    // Pareto frontier legend entry.
    let pareto_y = ly + 50.0;
    svg.push_str(&format!(
        r##"<line x1="{x1:.1}" y1="{y:.1}" x2="{x2:.1}" y2="{y:.1}" stroke="{c}" stroke-width="1.4" stroke-dasharray="6,3" opacity="0.8"/>"##,
        x1 = lx + 8.0,
        x2 = lx + 28.0,
        y = pareto_y,
        c = palette.text,
    ));
    svg.push_str(&format!(
        r#"<text x="{tx:.1}" y="{ty:.1}" fill="{c}" font-size="9">Pareto frontier</text>"#,
        tx = lx + 34.0,
        ty = pareto_y + 3.0,
        c = palette.text,
    ));

    svg.push_str("</svg>");
    svg
}

/// Marker radius scaled by flux/km; clamped so a single big-flux
/// candidate doesn't blow out the chart.
fn tradeoff_marker_radius(flux: f64, flux_max: f64) -> f64 {
    if flux_max <= 0.0 {
        return 4.0;
    }
    // 3.5 → 8.0 px radius across the flux range.
    3.5 + 4.5 * (flux / flux_max).clamp(0.0, 1.0)
}

/// Map U₀ (min/avg) onto a red → amber → green ramp. RP-8 / EN 13201
/// targets 0.4 as a typical floor; values above 0.6 are excellent.
fn uniformity_color(u0: f64) -> &'static str {
    let u = u0.clamp(0.0, 1.0);
    if u >= 0.55 {
        "#22c55e" // green
    } else if u >= 0.40 {
        "#eab308" // amber
    } else if u >= 0.25 {
        "#f97316" // orange
    } else {
        "#ef4444" // red
    }
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
                failure_overlay: Some(FailureOverlay::ratio(10.0)),
                show_grid_points: false,
            },
        );
        assert!(with_overlay.contains("#ff3b3b"));
    }

    #[test]
    fn absolute_lux_overlay_triggers_on_low_cells() {
        let layout = StreetLayout::default();
        let result = compute_result(&layout);

        // Set the absolute floor above the grid maximum — every cell must fail.
        let floor = result.max_lux + 1.0;
        let svg = plan_view_heatmap(
            &layout,
            &result,
            800.0,
            300.0,
            PlanViewOptions {
                theme: StreetTheme::Dark,
                failure_overlay: Some(FailureOverlay::absolute(floor)),
                show_grid_points: false,
            },
        );
        assert!(
            svg.contains("#ff3b3b"),
            "absolute overlay should tint cells"
        );

        // And an impossibly low floor → zero tints.
        let svg_none = plan_view_heatmap(
            &layout,
            &result,
            800.0,
            300.0,
            PlanViewOptions {
                theme: StreetTheme::Dark,
                failure_overlay: Some(FailureOverlay::absolute(-1.0)),
                show_grid_points: false,
            },
        );
        assert!(!svg_none.contains("#ff3b3b"));
    }

    #[test]
    fn is_failing_matches_variant_semantics() {
        let ratio = FailureOverlay::ratio(0.5);
        assert!(ratio.is_failing(4.0, 10.0)); // 4 < 10 * 0.5 = 5
        assert!(!ratio.is_failing(6.0, 10.0));

        let abs = FailureOverlay::absolute(8.0);
        assert!(abs.is_failing(5.0, 100.0)); // avg irrelevant
        assert!(!abs.is_failing(9.0, 1.0));
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
                show_grid_points: false,
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

    fn synth(p: f64, lux: f64, flux: f64, u0: f64) -> OptimizationCandidate {
        OptimizationCandidate {
            pole_spacing_m: 1000.0 / p,
            mounting_height_m: 10.0,
            arrangement: Arrangement::SingleSide,
            design: crate::standards::DesignResult {
                avg_illuminance_lux: lux,
                min_illuminance_lux: lux * u0,
                max_illuminance_lux: lux * 1.5,
                avg_luminance_cd_m2: None,
                uniformity_overall: u0,
                uniformity_longitudinal: None,
                threshold_increment_pct: None,
            },
            cost: p,
            poles_per_km: p,
            flux_per_km: flux,
        }
    }

    #[test]
    fn layout_tradeoff_chart_empty_input_produces_placeholder_svg() {
        let svg = layout_tradeoff_chart(&[], &[], 400.0, 300.0, LayoutTradeoffOptions::default());
        assert!(svg.contains("<svg"));
        assert!(svg.contains("No passing layouts"));
    }

    #[test]
    fn layout_tradeoff_chart_renders_markers_and_frontier() {
        let cands = vec![
            synth(40.0, 25.0, 400_000.0, 0.45),
            synth(50.0, 30.0, 500_000.0, 0.55),
            synth(50.0, 20.0, 500_000.0, 0.30),
        ];
        let frontier = crate::street::optimize::pareto_front_tradeoff(&cands);
        let svg = layout_tradeoff_chart(
            &cands,
            &frontier,
            500.0,
            350.0,
            LayoutTradeoffOptions {
                theme: StreetTheme::Dark,
                highlight_idx: Some(0),
            },
        );
        // Sanity: well-formed SVG with axis labels.
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        assert!(svg.contains("Poles per km"));
        assert!(svg.contains("Average illuminance"));
        // Three markers + a frontier path + halo for the highlighted one.
        let circle_count = svg.matches("<circle").count();
        assert!(circle_count >= 3, "expected ≥3 markers, got {circle_count}");
        assert!(svg.contains(r#"<path d="M"#), "frontier polyline missing");
    }
}
