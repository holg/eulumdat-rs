//! SVG rendering for Zonal Cavity Method results.
//!
//! Generates room plan, section view, CU table, and illuminance heatmap SVGs.

use crate::calculations::{CuTable, CU_RCR_VALUES, CU_REFLECTANCES};
use crate::diagram::color::heatmap_color;
use crate::diagram::contour::marching_squares;
use crate::diagram::SvgTheme;
use crate::units::UnitSystem;

use super::compute::{CavityResults, LuminaireLayout, PpbResult, Room};

/// SVG generator for zonal cavity results.
pub struct ZonalSvg;

impl ZonalSvg {
    /// Room plan view: top-down rectangle with luminaire grid dots.
    pub fn room_plan(
        room: &Room,
        layout: &LuminaireLayout,
        theme: &SvgTheme,
        units: UnitSystem,
    ) -> String {
        let svg_w = 500.0_f64;
        let svg_h = 400.0_f64;
        let margin = 50.0;

        let plot_w = svg_w - 2.0 * margin;
        let plot_h = svg_h - 2.0 * margin;

        // Scale room to fit
        let scale_x = plot_w / room.length;
        let scale_y = plot_h / room.width;
        let scale = scale_x.min(scale_y);
        let room_w = room.length * scale;
        let room_h = room.width * scale;
        let ox = margin + (plot_w - room_w) / 2.0;
        let oy = margin + (plot_h - room_h) / 2.0;

        let bg = &theme.background;
        let fg = &theme.text;
        let grid_color = &theme.grid;
        let accent = if layout.spacing_ok {
            "#22c55e" // green
        } else {
            "#ef4444" // red
        };

        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {svg_w} {svg_h}" preserveAspectRatio="xMidYMid meet">"#
        );
        svg.push_str(&format!(
            r#"<rect width="{svg_w}" height="{svg_h}" fill="{bg}"/>"#
        ));

        // Room outline
        svg.push_str(&format!(
            r#"<rect x="{ox}" y="{oy}" width="{room_w}" height="{room_h}" fill="none" stroke="{fg}" stroke-width="2"/>"#
        ));

        // Grid lines (spacing)
        if layout.rows > 0 && layout.cols > 0 {
            for r in 1..layout.rows {
                let x = ox + (layout.offset_x + r as f64 * layout.spacing_x) * scale
                    - layout.spacing_x * scale / 2.0;
                svg.push_str(&format!(
                    r#"<line x1="{x}" y1="{oy}" x2="{x}" y2="{}" stroke="{grid_color}" stroke-width="0.5" stroke-dasharray="4,4"/>"#,
                    oy + room_h
                ));
            }
            for c in 1..layout.cols {
                let y = oy + (layout.offset_y + c as f64 * layout.spacing_y) * scale
                    - layout.spacing_y * scale / 2.0;
                svg.push_str(&format!(
                    r#"<line x1="{ox}" y1="{y}" x2="{}" y2="{y}" stroke="{grid_color}" stroke-width="0.5" stroke-dasharray="4,4"/>"#,
                    ox + room_w
                ));
            }
        }

        // Luminaire dots
        for r in 0..layout.rows {
            for c in 0..layout.cols {
                let x = ox + (layout.offset_x + r as f64 * layout.spacing_x) * scale;
                let y = oy + (layout.offset_y + c as f64 * layout.spacing_y) * scale;
                svg.push_str(&format!(
                    r#"<circle cx="{x}" cy="{y}" r="4" fill="{accent}" stroke="{fg}" stroke-width="1"/>"#
                ));
            }
        }

        // Dimension labels
        let dist_label = units.distance_label();
        let l_disp = units.convert_meters(room.length);
        let w_disp = units.convert_meters(room.width);

        // Length label (bottom)
        svg.push_str(&format!(
            r#"<text x="{}" y="{}" fill="{fg}" font-size="11" text-anchor="middle">{:.1} {dist_label}</text>"#,
            ox + room_w / 2.0,
            oy + room_h + 18.0,
            l_disp
        ));
        // Width label (right)
        svg.push_str(&format!(
            r#"<text x="{}" y="{}" fill="{fg}" font-size="11" text-anchor="middle" transform="rotate(-90,{},{})">{:.1} {dist_label}</text>"#,
            ox + room_w + 18.0,
            oy + room_h / 2.0,
            ox + room_w + 18.0,
            oy + room_h / 2.0,
            w_disp
        ));

        // Spacing labels
        if layout.rows > 0 && layout.cols > 0 {
            let sx_disp = units.convert_meters(layout.spacing_x);
            let sy_disp = units.convert_meters(layout.spacing_y);
            svg.push_str(&format!(
                r#"<text x="{}" y="{}" fill="{accent}" font-size="9" text-anchor="middle">Sx={:.2} {dist_label} (S/MH={:.2})</text>"#,
                ox + room_w / 2.0,
                oy - 8.0,
                sx_disp,
                layout.s_mh_x
            ));
            svg.push_str(&format!(
                r#"<text x="{}" y="{}" fill="{accent}" font-size="9" text-anchor="start" transform="rotate(-90,{},{})">Sy={:.2} {dist_label} (S/MH={:.2})</text>"#,
                ox - 8.0,
                oy + room_h / 2.0,
                ox - 8.0,
                oy + room_h / 2.0,
                sy_disp,
                layout.s_mh_y
            ));
        }

        // Count label
        svg.push_str(&format!(
            r#"<text x="{}" y="{}" fill="{fg}" font-size="10" text-anchor="end">{} luminaires ({}×{})</text>"#,
            svg_w - 10.0,
            svg_h - 8.0,
            layout.count,
            layout.rows,
            layout.cols
        ));

        svg.push_str("</svg>");
        svg
    }

    /// Section view: side cross-section showing 3 cavities.
    pub fn section_view(
        room: &Room,
        cavity: &CavityResults,
        theme: &SvgTheme,
        units: UnitSystem,
    ) -> String {
        let svg_w = 400.0_f64;
        let svg_h = 300.0_f64;
        let margin_x = 60.0;
        let margin_y = 30.0;

        let plot_w = svg_w - 2.0 * margin_x;
        let plot_h = svg_h - 2.0 * margin_y;

        let total_h = room.height;
        let scale = plot_h / total_h;

        let bg = &theme.background;
        let fg = &theme.text;

        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {svg_w} {svg_h}" preserveAspectRatio="xMidYMid meet">"#
        );
        svg.push_str(&format!(
            r#"<rect width="{svg_w}" height="{svg_h}" fill="{bg}"/>"#
        ));

        let x0 = margin_x;
        let x1 = margin_x + plot_w;
        let y_ceiling = margin_y;
        let y_luminaire = margin_y + room.hcc() * scale;
        let y_workplane = margin_y + (room.hcc() + room.hrc()) * scale;
        let y_floor = margin_y + total_h * scale;

        // Cavity fills
        let cc_fill = "rgba(200,200,255,0.15)";
        let rc_fill = "rgba(255,255,200,0.15)";
        let fc_fill = "rgba(200,255,200,0.15)";

        // Ceiling cavity
        svg.push_str(&format!(
            r#"<rect x="{x0}" y="{y_ceiling}" width="{plot_w}" height="{}" fill="{cc_fill}"/>"#,
            y_luminaire - y_ceiling
        ));
        // Room cavity
        svg.push_str(&format!(
            r#"<rect x="{x0}" y="{y_luminaire}" width="{plot_w}" height="{}" fill="{rc_fill}"/>"#,
            y_workplane - y_luminaire
        ));
        // Floor cavity
        svg.push_str(&format!(
            r#"<rect x="{x0}" y="{y_workplane}" width="{plot_w}" height="{}" fill="{fc_fill}"/>"#,
            y_floor - y_workplane
        ));

        // Horizontal lines
        for (y, label, dash) in [
            (y_ceiling, "Ceiling", false),
            (y_luminaire, "Luminaire plane", true),
            (y_workplane, "Workplane", true),
            (y_floor, "Floor", false),
        ] {
            let sw = if dash { "1" } else { "2" };
            let da = if dash {
                r#" stroke-dasharray="6,3""#
            } else {
                ""
            };
            svg.push_str(&format!(
                r#"<line x1="{x0}" y1="{y}" x2="{x1}" y2="{y}" stroke="{fg}" stroke-width="{sw}"{da}/>"#
            ));
            svg.push_str(&format!(
                r#"<text x="{}" y="{}" fill="{fg}" font-size="9" text-anchor="end">{label}</text>"#,
                x0 - 4.0,
                y + 3.0
            ));
        }

        // Walls
        svg.push_str(&format!(
            r#"<line x1="{x0}" y1="{y_ceiling}" x2="{x0}" y2="{y_floor}" stroke="{fg}" stroke-width="2"/>"#
        ));
        svg.push_str(&format!(
            r#"<line x1="{x1}" y1="{y_ceiling}" x2="{x1}" y2="{y_floor}" stroke="{fg}" stroke-width="2"/>"#
        ));

        // Cavity labels with ratios
        let dist_label = units.distance_label();
        let mid_x = margin_x + plot_w / 2.0;

        if room.hcc() > 0.01 {
            let mid_y = (y_ceiling + y_luminaire) / 2.0;
            svg.push_str(&format!(
                r#"<text x="{mid_x}" y="{mid_y}" fill="{fg}" font-size="10" text-anchor="middle" dominant-baseline="middle">CCR={:.2} ({:.2} {dist_label})</text>"#,
                cavity.ccr,
                units.convert_meters(room.hcc())
            ));
        }

        {
            let mid_y = (y_luminaire + y_workplane) / 2.0;
            svg.push_str(&format!(
                r#"<text x="{mid_x}" y="{mid_y}" fill="{fg}" font-size="10" text-anchor="middle" dominant-baseline="middle">RCR={:.2} ({:.2} {dist_label})</text>"#,
                cavity.rcr,
                units.convert_meters(room.hrc())
            ));
        }

        if room.hfc() > 0.01 {
            let mid_y = (y_workplane + y_floor) / 2.0;
            svg.push_str(&format!(
                r#"<text x="{mid_x}" y="{mid_y}" fill="{fg}" font-size="10" text-anchor="middle" dominant-baseline="middle">FCR={:.2} ({:.2} {dist_label})</text>"#,
                cavity.fcr,
                units.convert_meters(room.hfc())
            ));
        }

        // Effective reflectances
        svg.push_str(&format!(
            r#"<text x="{}" y="{}" fill="{fg}" font-size="8" text-anchor="end">ρcc_eff={:.0}%</text>"#,
            x1 + 55.0,
            (y_ceiling + y_luminaire) / 2.0 + 3.0,
            cavity.rho_cc_eff * 100.0
        ));
        svg.push_str(&format!(
            r#"<text x="{}" y="{}" fill="{fg}" font-size="8" text-anchor="end">ρfc_eff={:.0}%</text>"#,
            x1 + 55.0,
            (y_workplane + y_floor) / 2.0 + 3.0,
            cavity.rho_fc_eff * 100.0
        ));

        svg.push_str("</svg>");
        svg
    }

    /// CU table SVG with highlighted operating point.
    pub fn cu_table_svg(
        cu_table: &CuTable,
        rcr: f64,
        rho_cc_eff: f64,
        rho_w: f64,
        theme: &SvgTheme,
    ) -> String {
        let cell_w = 38.0;
        let cell_h = 20.0;
        let header_h = 40.0;
        let row_label_w = 40.0;

        let num_cols = cu_table.reflectances.len().min(CU_REFLECTANCES.len());
        let num_rows = cu_table.values.len().min(CU_RCR_VALUES.len());

        let svg_w = row_label_w + num_cols as f64 * cell_w + 10.0;
        let svg_h = header_h + num_rows as f64 * cell_h + 10.0;

        let bg = &theme.background;
        let fg = &theme.text;

        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {svg_w} {svg_h}" preserveAspectRatio="xMidYMid meet">"#
        );
        svg.push_str(&format!(
            r#"<rect width="{svg_w}" height="{svg_h}" fill="{bg}"/>"#
        ));

        // Find operating column
        let op_col = find_operating_column(cu_table, rho_cc_eff, rho_w);
        let op_row_low = rcr.floor() as usize;
        let op_row_high = (op_row_low + 1).min(num_rows.saturating_sub(1));

        // Header: reflectance combos
        svg.push_str(&format!(
            r#"<text x="4" y="12" fill="{fg}" font-size="8" font-weight="bold">RCR</text>"#
        ));
        for (col, &(rc, rw, _)) in cu_table.reflectances.iter().enumerate().take(num_cols) {
            let x = row_label_w + col as f64 * cell_w + cell_w / 2.0;
            svg.push_str(&format!(
                r#"<text x="{x}" y="12" fill="{fg}" font-size="7" text-anchor="middle">{rc}/{rw}</text>"#
            ));
        }

        // Data rows
        for (row, rcr_val) in CU_RCR_VALUES.iter().enumerate().take(num_rows) {
            let y = header_h + row as f64 * cell_h;

            // RCR label
            svg.push_str(&format!(
                r#"<text x="20" y="{}" fill="{fg}" font-size="9" text-anchor="middle" dominant-baseline="middle">{rcr_val}</text>"#,
                y + cell_h / 2.0
            ));

            for col in 0..num_cols {
                let x = row_label_w + col as f64 * cell_w;
                let val = cu_table
                    .values
                    .get(row)
                    .and_then(|r| r.get(col))
                    .copied()
                    .unwrap_or(0.0);

                // Highlight operating point
                let is_op = col == op_col && (row == op_row_low || row == op_row_high);
                if is_op {
                    svg.push_str(&format!(
                        r#"<rect x="{x}" y="{y}" width="{cell_w}" height="{cell_h}" fill="rgba(59,130,246,0.3)" stroke="{}" stroke-width="1"/>"#,
                        "#3b82f6"
                    ));
                }

                svg.push_str(&format!(
                    r#"<text x="{}" y="{}" fill="{fg}" font-size="8" text-anchor="middle" dominant-baseline="middle">{:.0}</text>"#,
                    x + cell_w / 2.0,
                    y + cell_h / 2.0,
                    val
                ));
            }
        }

        svg.push_str("</svg>");
        svg
    }

    /// Illuminance heatmap view (point-by-point overlay).
    pub fn illuminance_view(
        ppb: &PpbResult,
        room: &Room,
        theme: &SvgTheme,
        units: UnitSystem,
    ) -> String {
        Self::illuminance_view_opts(ppb, room, theme, units, false)
    }

    /// Render illuminance heatmap with optional numeric value labels.
    pub fn illuminance_view_opts(
        ppb: &PpbResult,
        room: &Room,
        theme: &SvgTheme,
        units: UnitSystem,
        show_values: bool,
    ) -> String {
        let svg_w = 550.0_f64;
        let svg_h = 450.0_f64;
        let margin_left = 50.0;
        let margin_right = 80.0;
        let margin_top = 30.0;
        let margin_bottom = 40.0;

        let plot_w = svg_w - margin_left - margin_right;
        let plot_h = svg_h - margin_top - margin_bottom;

        // Scale room to fit
        let scale_x = plot_w / room.length;
        let scale_y = plot_h / room.width;
        let scale = scale_x.min(scale_y);
        let room_px_w = room.length * scale;
        let room_px_h = room.width * scale;
        let ox = margin_left + (plot_w - room_px_w) / 2.0;
        let oy = margin_top + (plot_h - room_px_h) / 2.0;

        let bg = &theme.background;
        let fg = &theme.text;
        let n = ppb.grid_resolution;
        let max_lux = ppb.max_lux.max(1.0);

        let cell_w = room_px_w / n as f64;
        let cell_h = room_px_h / n as f64;

        let illu_label = units.illuminance_label();

        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {svg_w} {svg_h}" preserveAspectRatio="xMidYMid meet">"#
        );
        svg.push_str(&format!(
            r#"<rect width="{svg_w}" height="{svg_h}" fill="{bg}"/>"#
        ));

        // Value label density (when show_values enabled)
        let label_step = ((n as f64 / 10.0).ceil() as usize).max(1);
        let font_size = (cell_w * label_step as f64 * 0.35)
            .min(cell_h * label_step as f64 * 0.4)
            .clamp(5.0, 11.0);

        // Heatmap cells
        for (row, grid_row) in ppb.lux_grid.iter().enumerate() {
            for (col, &lux) in grid_row.iter().enumerate() {
                let normalized = lux / max_lux;
                let color = heatmap_color(normalized);
                let sx = ox + col as f64 * cell_w;
                let sy = oy + row as f64 * cell_h;
                svg.push_str(&format!(
                    r#"<rect x="{sx}" y="{sy}" width="{}" height="{}" fill="{}"/>"#,
                    cell_w + 0.5,
                    cell_h + 0.5,
                    color.to_rgb_string()
                ));
            }
        }

        // Contour lines
        let contour_levels: Vec<f64> = match units {
            UnitSystem::Imperial => [0.5, 1.0, 2.0, 5.0, 10.0, 25.0, 50.0, 100.0]
                .iter()
                .map(|&fc| fc * 10.764)
                .collect(),
            UnitSystem::Metric => {
                vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0]
            }
        };

        let x_coords: Vec<f64> = (0..n).map(|col| ox + (col as f64 + 0.5) * cell_w).collect();
        let y_coords: Vec<f64> = (0..n).map(|row| oy + (row as f64 + 0.5) * cell_h).collect();

        for &level in &contour_levels {
            if level > max_lux || level <= 0.0 {
                continue;
            }
            let cl = marching_squares(&ppb.lux_grid, &x_coords, &y_coords, level);
            for path in &cl.paths {
                svg.push_str(&format!(
                    r#"<path d="{path}" fill="none" stroke="rgba(255,255,255,0.7)" stroke-width="1"/>"#
                ));
            }
            // Label on first path
            if let Some(first_path) = cl.paths.first() {
                if let Some(start) = parse_first_point(first_path) {
                    let display_val = units.convert_lux(level);
                    svg.push_str(&format!(
                        r#"<text x="{}" y="{}" fill="white" font-size="7" text-shadow="0 0 2px black">{:.0}</text>"#,
                        start.0 + 2.0,
                        start.1 - 2.0,
                        display_val
                    ));
                }
            }
        }

        // Room outline
        svg.push_str(&format!(
            r#"<rect x="{ox}" y="{oy}" width="{room_px_w}" height="{room_px_h}" fill="none" stroke="{fg}" stroke-width="1.5"/>"#
        ));

        // Value labels (rendered on top of contours and outline)
        if show_values {
            for (row, grid_row) in ppb.lux_grid.iter().enumerate() {
                for (col, &lux) in grid_row.iter().enumerate() {
                    if row % label_step == label_step / 2 && col % label_step == label_step / 2 {
                        let normalized = lux / max_lux;
                        let display_val = units.convert_lux(lux);
                        let text_color = if normalized < 0.45 {
                            "white"
                        } else {
                            "#1a1a1a"
                        };
                        let cx = ox + (col as f64 + label_step as f64 / 2.0) * cell_w;
                        let cy = oy + (row as f64 + label_step as f64 / 2.0) * cell_h;
                        svg.push_str(&format!(
                            r#"<text x="{cx:.1}" y="{cy:.1}" fill="{text_color}" font-size="{font_size:.1}" text-anchor="middle" dominant-baseline="central" font-family="monospace" stroke="{bg}" stroke-width="2" paint-order="stroke">{:.0}</text>"#,
                            display_val
                        ));
                    }
                }
            }
        }

        // Color legend
        let legend_x = ox + room_px_w + 15.0;
        let legend_h = room_px_h * 0.7;
        let legend_top = oy + (room_px_h - legend_h) / 2.0;
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
                r#"<text x="{}" y="{y}" fill="{fg}" font-size="8" dominant-baseline="middle">{:.0} {illu_label}</text>"#,
                legend_x + legend_w + 4.0,
                display_val
            ));
        }

        // Statistics
        let stats_y = svg_h - 8.0;
        let min_d = units.convert_lux(ppb.min_lux);
        let avg_d = units.convert_lux(ppb.avg_lux);
        let max_d = units.convert_lux(ppb.max_lux);
        svg.push_str(&format!(
            r#"<text x="{margin_left}" y="{stats_y}" fill="{fg}" font-size="9">Min: {:.0}  Avg: {:.0}  Max: {:.0} {illu_label}  |  U₀: {:.2}  Ud: {:.2}</text>"#,
            min_d,
            avg_d,
            max_d,
            ppb.uniformity_min_avg,
            ppb.uniformity_min_max
        ));

        svg.push_str("</svg>");
        svg
    }

    /// Render a spreadsheet-style illuminance table as SVG.
    ///
    /// Shows numeric lux/fc values in a grid with colored cell backgrounds.
    /// Required for US standards compliance (IES LM-83, ASHRAE 90.1).
    pub fn illuminance_table(
        ppb: &PpbResult,
        room: &Room,
        theme: &SvgTheme,
        units: UnitSystem,
    ) -> String {
        let n = ppb.grid_resolution;
        // Downsample for readable table: max ~10×10 cells
        let step = ((n as f64 / 10.0).ceil() as usize).max(1);
        let cols = n.div_ceil(step);
        let rows = n.div_ceil(step);

        let cell_px = 48.0;
        let header_w = 40.0;
        let header_h = 22.0;
        let svg_w = header_w + cols as f64 * cell_px + 10.0;
        let svg_h = header_h + rows as f64 * cell_px * 0.5 + 30.0;

        let bg = &theme.background;
        let fg = &theme.text;
        let max_lux = ppb.max_lux.max(1.0);
        let illu_label = units.illuminance_label();

        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {svg_w} {svg_h}" preserveAspectRatio="xMidYMid meet">"#
        );
        svg.push_str(&format!(
            r#"<rect width="{svg_w}" height="{svg_h}" fill="{bg}"/>"#
        ));

        // Column headers (room width positions in meters)
        for ci in 0..cols {
            let src_col = (ci * step + step / 2).min(n - 1);
            let x_m = (src_col as f64 + 0.5) / n as f64 * room.width;
            let x = header_w + ci as f64 * cell_px + cell_px / 2.0;
            svg.push_str(&format!(
                r#"<text x="{x}" y="{}" fill="{fg}" font-size="7" text-anchor="middle">{:.1}m</text>"#,
                header_h - 4.0, x_m
            ));
        }

        // Row headers + data cells
        let cell_h = cell_px * 0.5;
        for ri in 0..rows {
            let src_row = (ri * step + step / 2).min(n - 1);
            let y_m = (src_row as f64 + 0.5) / n as f64 * room.length;
            let y = header_h + ri as f64 * cell_h;

            // Row header
            svg.push_str(&format!(
                r#"<text x="{}" y="{}" fill="{fg}" font-size="7" text-anchor="end" dominant-baseline="central">{:.1}m</text>"#,
                header_w - 4.0, y + cell_h / 2.0, y_m
            ));

            for ci in 0..cols {
                // Average over the step×step block for more representative values
                let mut sum = 0.0;
                let mut count = 0;
                for dr in 0..step {
                    for dc in 0..step {
                        let r = ri * step + dr;
                        let c = ci * step + dc;
                        if r < n && c < n {
                            sum += ppb.lux_grid[r][c];
                            count += 1;
                        }
                    }
                }
                let lux = if count > 0 { sum / count as f64 } else { 0.0 };
                let normalized = lux / max_lux;
                let color = heatmap_color(normalized);
                let text_color = if normalized < 0.45 {
                    "white"
                } else {
                    "#1a1a1a"
                };
                let display_val = units.convert_lux(lux);

                let x = header_w + ci as f64 * cell_px;

                // Cell background
                svg.push_str(&format!(
                    r#"<rect x="{x}" y="{y}" width="{cell_px}" height="{cell_h}" fill="{}" stroke="{fg}" stroke-width="0.3"/>"#,
                    color.to_rgb_string()
                ));
                // Value
                svg.push_str(&format!(
                    r#"<text x="{}" y="{}" fill="{text_color}" font-size="9" text-anchor="middle" dominant-baseline="central" font-family="monospace">{:.0}</text>"#,
                    x + cell_px / 2.0,
                    y + cell_h / 2.0,
                    display_val
                ));
            }
        }

        // Statistics footer
        let stats_y = svg_h - 8.0;
        let min_d = units.convert_lux(ppb.min_lux);
        let avg_d = units.convert_lux(ppb.avg_lux);
        let max_d = units.convert_lux(ppb.max_lux);
        svg.push_str(&format!(
            r#"<text x="5" y="{stats_y}" fill="{fg}" font-size="8">Min: {:.0}  Avg: {:.0}  Max: {:.0} {illu_label}  |  U₀: {:.2}  Ud: {:.2}</text>"#,
            min_d, avg_d, max_d,
            ppb.uniformity_min_avg, ppb.uniformity_min_max
        ));

        svg.push_str("</svg>");
        svg
    }
}

/// Find the operating column index in the CU table for given reflectances.
fn find_operating_column(cu_table: &CuTable, rho_cc: f64, rho_w: f64) -> usize {
    let rc_pct = (rho_cc * 100.0).round() as i32;
    let rw_pct = (rho_w * 100.0).round() as i32;

    cu_table
        .reflectances
        .iter()
        .enumerate()
        .min_by_key(|(_, &(rc, rw, _))| {
            let dc = (rc as i32 - rc_pct).abs();
            let dw = (rw as i32 - rw_pct).abs();
            dc * 2 + dw
        })
        .map(|(i, _)| i)
        .unwrap_or(0)
}

/// Parse first M x,y point from an SVG path string.
fn parse_first_point(path: &str) -> Option<(f64, f64)> {
    let s = path.strip_prefix('M')?.trim();
    let mut parts = s.split_whitespace();
    let first = parts.next()?;
    let mut coords = first.split([',', ' ']);
    let x: f64 = coords.next()?.parse().ok()?;
    let y: f64 = coords.next().or_else(|| parts.next())?.parse().ok()?;
    Some((x, y))
}
