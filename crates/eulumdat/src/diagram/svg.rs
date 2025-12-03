//! SVG rendering for photometric diagrams
//!
//! This module generates complete SVG strings for each diagram type.
//! The SVGs can be used directly in browsers, iOS (via WebView or SVG libraries),
//! Android, or any other platform that supports SVG.
//!
//! # Example
//!
//! ```rust,no_run
//! use eulumdat::{Eulumdat, diagram::{PolarDiagram, SvgTheme}};
//!
//! let ldt = Eulumdat::from_file("luminaire.ldt").unwrap();
//! let polar = PolarDiagram::from_eulumdat(&ldt);
//! let svg = polar.to_svg(500.0, 500.0, &SvgTheme::light());
//! // svg is a complete SVG string ready to render
//! ```

use super::{ButterflyDiagram, CartesianDiagram, HeatmapDiagram, PolarDiagram};

/// Theme configuration for SVG diagrams
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SvgTheme {
    /// Background color
    pub background: String,
    /// Plot surface color
    pub surface: String,
    /// Grid line color
    pub grid: String,
    /// Axis line color (darker grid)
    pub axis: String,
    /// Primary text color
    pub text: String,
    /// Secondary text color
    pub text_secondary: String,
    /// Legend background
    pub legend_bg: String,
    /// C0-C180 curve color (typically blue)
    pub curve_c0_c180: String,
    /// C0-C180 fill color
    pub curve_c0_c180_fill: String,
    /// C90-C270 curve color (typically red)
    pub curve_c90_c270: String,
    /// C90-C270 fill color
    pub curve_c90_c270_fill: String,
    /// Font family
    pub font_family: String,
}

impl Default for SvgTheme {
    fn default() -> Self {
        Self::light()
    }
}

impl SvgTheme {
    /// Light theme (default)
    pub fn light() -> Self {
        Self {
            background: "#ffffff".to_string(),
            surface: "#f8fafc".to_string(),
            grid: "#e2e8f0".to_string(),
            axis: "#94a3b8".to_string(),
            text: "#1e293b".to_string(),
            text_secondary: "#64748b".to_string(),
            legend_bg: "rgba(255,255,255,0.9)".to_string(),
            curve_c0_c180: "#3b82f6".to_string(),
            curve_c0_c180_fill: "rgba(59,130,246,0.15)".to_string(),
            curve_c90_c270: "#ef4444".to_string(),
            curve_c90_c270_fill: "rgba(239,68,68,0.15)".to_string(),
            font_family: "system-ui, -apple-system, sans-serif".to_string(),
        }
    }

    /// Dark theme
    pub fn dark() -> Self {
        Self {
            background: "#0f172a".to_string(),
            surface: "#1e293b".to_string(),
            grid: "#334155".to_string(),
            axis: "#64748b".to_string(),
            text: "#f1f5f9".to_string(),
            text_secondary: "#94a3b8".to_string(),
            legend_bg: "rgba(30,41,59,0.9)".to_string(),
            curve_c0_c180: "#60a5fa".to_string(),
            curve_c0_c180_fill: "rgba(96,165,250,0.2)".to_string(),
            curve_c90_c270: "#f87171".to_string(),
            curve_c90_c270_fill: "rgba(248,113,113,0.2)".to_string(),
            font_family: "system-ui, -apple-system, sans-serif".to_string(),
        }
    }

    /// Theme using CSS variables (for web with dynamic theming)
    pub fn css_variables() -> Self {
        Self {
            background: "var(--diagram-bg, #ffffff)".to_string(),
            surface: "var(--diagram-surface, #f8fafc)".to_string(),
            grid: "var(--diagram-grid, #e2e8f0)".to_string(),
            axis: "var(--diagram-axis, #94a3b8)".to_string(),
            text: "var(--diagram-text, #1e293b)".to_string(),
            text_secondary: "var(--diagram-text-secondary, #64748b)".to_string(),
            legend_bg: "var(--diagram-legend-bg, rgba(255,255,255,0.9))".to_string(),
            curve_c0_c180: "var(--diagram-c90, #3b82f6)".to_string(),
            curve_c0_c180_fill: "var(--diagram-c90-fill, rgba(59,130,246,0.15))".to_string(),
            curve_c90_c270: "var(--diagram-c0, #ef4444)".to_string(),
            curve_c90_c270_fill: "var(--diagram-c0-fill, rgba(239,68,68,0.15))".to_string(),
            font_family: "system-ui, -apple-system, sans-serif".to_string(),
        }
    }

    /// Get a color for a C-plane index
    pub fn c_plane_color(&self, index: usize) -> &str {
        const COLORS: &[&str] = &[
            "#3b82f6", // blue
            "#ef4444", // red
            "#22c55e", // green
            "#f97316", // orange
            "#8b5cf6", // purple
            "#ec4899", // pink
            "#06b6d4", // cyan
            "#eab308", // yellow
        ];
        COLORS[index % COLORS.len()]
    }
}

impl PolarDiagram {
    /// Generate complete SVG string for the polar diagram
    pub fn to_svg(&self, width: f64, height: f64, theme: &SvgTheme) -> String {
        let size = width.min(height);
        let center = size / 2.0;
        let margin = 60.0;
        let radius = (size / 2.0) - margin;
        let scale = self.scale.scale_max / radius;

        let mut svg = String::new();

        // SVG header
        svg.push_str(&format!(
            r#"<svg viewBox="0 0 {size} {size}" xmlns="http://www.w3.org/2000/svg">"#
        ));

        // Background
        svg.push_str(&format!(
            r#"<rect x="0" y="0" width="{size}" height="{size}" fill="{}"/>"#,
            theme.background
        ));

        // Grid circles
        let num_circles = self.scale.grid_values.len();
        for (i, &value) in self.scale.grid_values.iter().enumerate() {
            let r = value / scale;
            let is_major = i == num_circles - 1 || i == num_circles / 2;
            let stroke_color = if is_major { &theme.axis } else { &theme.grid };
            let stroke_width = if is_major { "1.5" } else { "1" };

            svg.push_str(&format!(
                r#"<circle cx="{center}" cy="{center}" r="{r:.1}" fill="none" stroke="{stroke_color}" stroke-width="{stroke_width}"/>"#
            ));

            // Intensity label
            svg.push_str(&format!(
                r#"<text x="{:.1}" y="{:.1}" font-size="11" fill="{}" font-family="{}">{:.0}</text>"#,
                center + 5.0,
                center + r + 12.0,
                theme.text_secondary,
                theme.font_family,
                value
            ));
        }

        // Radial lines every 30°
        for i in 0..=6 {
            if i == 3 {
                continue; // Skip 90° (drawn separately)
            }
            let angle_deg = i as f64 * 30.0;
            let angle_rad = angle_deg.to_radians();

            let x1_left = center - radius * angle_rad.sin();
            let y1_left = center + radius * angle_rad.cos();
            let x1_right = center + radius * angle_rad.sin();
            let y1_right = center + radius * angle_rad.cos();

            svg.push_str(&format!(
                r#"<line x1="{center}" y1="{center}" x2="{x1_left:.1}" y2="{y1_left:.1}" stroke="{}" stroke-width="1"/>"#,
                theme.grid
            ));
            svg.push_str(&format!(
                r#"<line x1="{center}" y1="{center}" x2="{x1_right:.1}" y2="{y1_right:.1}" stroke="{}" stroke-width="1"/>"#,
                theme.grid
            ));

            // Angle labels
            if angle_deg > 0.0 && angle_deg < 180.0 {
                let label_offset = radius + 18.0;
                let label_x_left = center - label_offset * angle_rad.sin();
                let label_y_left = center + label_offset * angle_rad.cos();
                let label_x_right = center + label_offset * angle_rad.sin();
                let label_y_right = center + label_offset * angle_rad.cos();

                svg.push_str(&format!(
                    r#"<text x="{label_x_left:.1}" y="{label_y_left:.1}" text-anchor="middle" dominant-baseline="middle" font-size="11" fill="{}" font-family="{}">{angle_deg:.0}°</text>"#,
                    theme.text_secondary, theme.font_family
                ));
                svg.push_str(&format!(
                    r#"<text x="{label_x_right:.1}" y="{label_y_right:.1}" text-anchor="middle" dominant-baseline="middle" font-size="11" fill="{}" font-family="{}">{angle_deg:.0}°</text>"#,
                    theme.text_secondary, theme.font_family
                ));
            }
        }

        // 180° label at top
        svg.push_str(&format!(
            r#"<text x="{center}" y="{:.1}" text-anchor="middle" font-size="11" fill="{}" font-family="{}">180°</text>"#,
            center - radius - 20.0,
            theme.text_secondary,
            theme.font_family
        ));

        // 90° horizontal line
        svg.push_str(&format!(
            r#"<line x1="{:.1}" y1="{center}" x2="{:.1}" y2="{center}" stroke="{}" stroke-width="1.5"/>"#,
            center - radius,
            center + radius,
            theme.axis
        ));

        // 90° labels
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{center}" text-anchor="middle" dominant-baseline="middle" font-size="11" fill="{}" font-family="{}">90°</text>"#,
            center - radius - 20.0,
            theme.text_secondary,
            theme.font_family
        ));
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{center}" text-anchor="middle" dominant-baseline="middle" font-size="11" fill="{}" font-family="{}">90°</text>"#,
            center + radius + 20.0,
            theme.text_secondary,
            theme.font_family
        ));

        // C0-C180 curve
        let path_c0_c180 = self.c0_c180_curve.to_svg_path(center, center, scale);
        if !path_c0_c180.is_empty() {
            svg.push_str(&format!(
                r#"<path d="{}" fill="{}" stroke="{}" stroke-width="2.5"/>"#,
                path_c0_c180, theme.curve_c0_c180_fill, theme.curve_c0_c180
            ));
        }

        // C90-C270 curve
        if self.show_c90_c270() {
            let path_c90_c270 = self.c90_c270_curve.to_svg_path(center, center, scale);
            if !path_c90_c270.is_empty() {
                svg.push_str(&format!(
                    r#"<path d="{}" fill="{}" stroke="{}" stroke-width="2.5" stroke-dasharray="6,4"/>"#,
                    path_c90_c270,
                    theme.curve_c90_c270_fill,
                    theme.curve_c90_c270
                ));
            }
        }

        // Center point
        svg.push_str(&format!(
            r#"<circle cx="{center}" cy="{center}" r="3" fill="{}"/>"#,
            theme.text
        ));

        // Legend
        svg.push_str(&format!(
            r#"<g transform="translate(15, {:.1})">"#,
            size - 55.0
        ));
        svg.push_str(&format!(
            r#"<rect x="0" y="0" width="16" height="16" fill="{}" stroke="{}" stroke-width="2" rx="2"/>"#,
            theme.curve_c0_c180_fill,
            theme.curve_c0_c180
        ));
        svg.push_str(&format!(
            r#"<text x="22" y="12" font-size="12" fill="{}" font-family="{}">C0-C180</text>"#,
            theme.text, theme.font_family
        ));
        svg.push_str("</g>");

        if self.show_c90_c270() {
            svg.push_str(&format!(
                r#"<g transform="translate(15, {:.1})">"#,
                size - 32.0
            ));
            svg.push_str(&format!(
                r#"<rect x="0" y="0" width="16" height="16" fill="{}" stroke="{}" stroke-width="2" stroke-dasharray="4,2" rx="2"/>"#,
                theme.curve_c90_c270_fill,
                theme.curve_c90_c270
            ));
            svg.push_str(&format!(
                r#"<text x="22" y="12" font-size="12" fill="{}" font-family="{}">C90-C270</text>"#,
                theme.text, theme.font_family
            ));
            svg.push_str("</g>");
        }

        // Unit label
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" text-anchor="end" font-size="11" fill="{}" font-family="{}">cd/1000lm</text>"#,
            size - 15.0,
            size - 15.0,
            theme.text_secondary,
            theme.font_family
        ));

        svg.push_str("</svg>");
        svg
    }
}

impl CartesianDiagram {
    /// Generate complete SVG string for the cartesian diagram
    pub fn to_svg(&self, width: f64, height: f64, theme: &SvgTheme) -> String {
        let margin_left = self.margin_left;
        let margin_top = self.margin_top;
        let plot_width = self.plot_width;
        let plot_height = self.plot_height;
        let y_max = self.scale.scale_max;

        let mut svg = String::new();

        // SVG header
        svg.push_str(&format!(
            r#"<svg viewBox="0 0 {width} {height}" xmlns="http://www.w3.org/2000/svg">"#
        ));

        // Background
        svg.push_str(&format!(
            r#"<rect x="0" y="0" width="{width}" height="{height}" fill="{}"/>"#,
            theme.background
        ));

        // Plot area background
        svg.push_str(&format!(
            r#"<rect x="{margin_left}" y="{margin_top}" width="{plot_width}" height="{plot_height}" fill="{}" stroke="{}" stroke-width="1"/>"#,
            theme.surface,
            theme.axis
        ));

        // Y-axis grid lines and labels
        for &v in &self.y_ticks {
            let y = margin_top + plot_height * (1.0 - v / y_max);
            svg.push_str(&format!(
                r#"<line x1="{margin_left}" y1="{y:.1}" x2="{:.1}" y2="{y:.1}" stroke="{}" stroke-width="1"/>"#,
                margin_left + plot_width,
                theme.grid
            ));
            svg.push_str(&format!(
                r#"<text x="{:.1}" y="{y:.1}" text-anchor="end" dominant-baseline="middle" font-size="11" fill="{}" font-family="{}">{v:.0}</text>"#,
                margin_left - 8.0,
                theme.text_secondary,
                theme.font_family
            ));
        }

        // X-axis grid lines and labels
        for &v in &self.x_ticks {
            let x = margin_left + plot_width * (v / self.max_gamma);
            svg.push_str(&format!(
                r#"<line x1="{x:.1}" y1="{margin_top}" x2="{x:.1}" y2="{:.1}" stroke="{}" stroke-width="1"/>"#,
                margin_top + plot_height,
                theme.grid
            ));
            svg.push_str(&format!(
                r#"<text x="{x:.1}" y="{:.1}" text-anchor="middle" font-size="11" fill="{}" font-family="{}">{v:.0}°</text>"#,
                margin_top + plot_height + 18.0,
                theme.text_secondary,
                theme.font_family
            ));
        }

        // Intensity curves
        for curve in &self.curves {
            let path = curve.to_svg_path();
            svg.push_str(&format!(
                r#"<path d="{}" fill="none" stroke="{}" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/>"#,
                path,
                curve.color.to_rgb_string()
            ));
        }

        // Axis labels
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" font-size="12" fill="{}" font-family="{}">Gamma (γ)</text>"#,
            margin_left + plot_width / 2.0,
            height - 8.0,
            theme.text,
            theme.font_family
        ));

        svg.push_str(&format!(
            r#"<text x="18" y="{:.1}" text-anchor="middle" font-size="12" fill="{}" font-family="{}" transform="rotate(-90, 18, {:.1})">Intensity (cd/klm)</text>"#,
            margin_top + plot_height / 2.0,
            theme.text,
            theme.font_family,
            margin_top + plot_height / 2.0
        ));

        // Legend
        let legend_height = self.curves.len() as f64 * 18.0 + 10.0;
        svg.push_str(&format!(
            r#"<g transform="translate({:.1}, {:.1})">"#,
            margin_left + 10.0,
            margin_top + 10.0
        ));
        svg.push_str(&format!(
            r#"<rect x="-5" y="-5" width="90" height="{legend_height:.1}" fill="{}" stroke="{}" stroke-width="1" rx="4"/>"#,
            theme.legend_bg,
            theme.axis
        ));

        for (i, curve) in self.curves.iter().enumerate() {
            let y = i as f64 * 18.0 + 8.0;
            svg.push_str(&format!(
                r#"<line x1="0" y1="{y:.1}" x2="18" y2="{y:.1}" stroke="{}" stroke-width="2.5"/>"#,
                curve.color.to_rgb_string()
            ));
            svg.push_str(&format!(
                r#"<text x="24" y="{:.1}" font-size="11" fill="{}" font-family="{}">{}</text>"#,
                y + 4.0,
                theme.text,
                theme.font_family,
                curve.label
            ));
        }
        svg.push_str("</g>");

        // Max intensity label
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="20" text-anchor="end" font-size="11" fill="{}" font-family="{}">Max: {:.0} cd/klm</text>"#,
            width - 15.0,
            theme.text_secondary,
            theme.font_family,
            self.scale.max_intensity
        ));

        svg.push_str("</svg>");
        svg
    }
}

impl HeatmapDiagram {
    /// Generate complete SVG string for the heatmap diagram
    pub fn to_svg(&self, width: f64, height: f64, theme: &SvgTheme) -> String {
        if self.is_empty() {
            return format!(
                r#"<svg viewBox="0 0 {width} {height}" xmlns="http://www.w3.org/2000/svg"><rect width="{width}" height="{height}" fill="{}"/><text x="{:.1}" y="{:.1}" text-anchor="middle" fill="{}">No data</text></svg>"#,
                theme.background,
                width / 2.0,
                height / 2.0,
                theme.text_secondary
            );
        }

        let mut svg = String::new();

        // SVG header
        svg.push_str(&format!(
            r#"<svg viewBox="0 0 {width} {height}" xmlns="http://www.w3.org/2000/svg">"#
        ));

        // Background
        svg.push_str(&format!(
            r#"<rect x="0" y="0" width="{width}" height="{height}" fill="{}"/>"#,
            theme.background
        ));

        // Title
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="25" text-anchor="middle" font-size="14" fill="{}" font-weight="600" font-family="{}">Intensity Heatmap (Candela)</text>"#,
            width / 2.0,
            theme.text,
            theme.font_family
        ));

        // Plot area border
        svg.push_str(&format!(
            r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="none" stroke="{}" stroke-width="1"/>"#,
            self.margin_left,
            self.margin_top,
            self.plot_width,
            self.plot_height,
            theme.grid
        ));

        // Heatmap cells
        for cell in &self.cells {
            svg.push_str(&format!(
                r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="{}"><title>C{:.0}° γ{:.0}°: {:.1} cd ({:.1} cd/klm)</title></rect>"#,
                cell.x,
                cell.y,
                cell.width,
                cell.height,
                cell.color.to_rgb_string(),
                cell.c_angle,
                cell.g_angle,
                cell.candela,
                cell.intensity
            ));
        }

        // X-axis labels (C-angles)
        let num_c = self.c_angles.len();
        let step = if num_c <= 10 {
            1
        } else if num_c <= 20 {
            2
        } else {
            5
        };
        let cell_width = self.plot_width / num_c as f64;
        for (i, &c) in self.c_angles.iter().enumerate() {
            if i % step == 0 {
                let x = self.margin_left + (i as f64 + 0.5) * cell_width;
                svg.push_str(&format!(
                    r#"<text x="{x:.1}" y="{:.1}" text-anchor="middle" font-size="9" fill="{}" font-family="{}">{c:.0}</text>"#,
                    self.margin_top + self.plot_height + 15.0,
                    theme.text_secondary,
                    theme.font_family
                ));
            }
        }

        // Y-axis labels (G-angles)
        let num_g = self.g_angles.len();
        let step = if num_g <= 10 {
            1
        } else if num_g <= 20 {
            2
        } else {
            5
        };
        let cell_height = self.plot_height / num_g as f64;
        for (i, &g) in self.g_angles.iter().enumerate() {
            if i % step == 0 {
                let y = self.margin_top + (i as f64 + 0.5) * cell_height;
                svg.push_str(&format!(
                    r#"<text x="{:.1}" y="{y:.1}" text-anchor="end" dominant-baseline="middle" font-size="9" fill="{}" font-family="{}">{g:.0}</text>"#,
                    self.margin_left - 8.0,
                    theme.text_secondary,
                    theme.font_family
                ));
            }
        }

        // Axis titles
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" font-size="12" fill="{}" font-family="{}">C-Plane Angle (°)</text>"#,
            self.margin_left + self.plot_width / 2.0,
            height - 10.0,
            theme.text,
            theme.font_family
        ));

        svg.push_str(&format!(
            r#"<text x="18" y="{:.1}" text-anchor="middle" font-size="12" fill="{}" font-family="{}" transform="rotate(-90, 18, {:.1})">Gamma Angle (°)</text>"#,
            self.margin_top + self.plot_height / 2.0,
            theme.text,
            theme.font_family,
            self.margin_top + self.plot_height / 2.0
        ));

        // Color legend
        let legend_x = width - 80.0;
        let legend_width = 20.0;
        let num_segments = 50;
        let segment_height = self.plot_height / num_segments as f64;

        for (normalized, color, _) in &self.legend_entries {
            let i = ((1.0 - normalized) * (num_segments as f64 - 1.0)) as usize;
            let sy = self.margin_top + i as f64 * segment_height;
            svg.push_str(&format!(
                r#"<rect x="{legend_x:.1}" y="{sy:.1}" width="{legend_width:.1}" height="{:.1}" fill="{}"/>"#,
                segment_height + 0.5,
                color.to_rgb_string()
            ));
        }

        // Legend border
        svg.push_str(&format!(
            r#"<rect x="{legend_x:.1}" y="{:.1}" width="{legend_width:.1}" height="{:.1}" fill="none" stroke="{}" stroke-width="1"/>"#,
            self.margin_top,
            self.plot_height,
            theme.grid
        ));

        // Legend labels
        let num_labels = 5;
        for i in 0..=num_labels {
            let frac = i as f64 / num_labels as f64;
            let value = self.max_candela * (1.0 - frac);
            let ly = self.margin_top + frac * self.plot_height;

            svg.push_str(&format!(
                r#"<line x1="{:.1}" y1="{ly:.1}" x2="{:.1}" y2="{ly:.1}" stroke="{}" stroke-width="1"/>"#,
                legend_x + legend_width,
                legend_x + legend_width + 5.0,
                theme.grid
            ));
            svg.push_str(&format!(
                r#"<text x="{:.1}" y="{ly:.1}" dominant-baseline="middle" font-size="9" fill="{}" font-family="{}">{value:.0}</text>"#,
                legend_x + legend_width + 8.0,
                theme.text_secondary,
                theme.font_family
            ));
        }

        // Legend title
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" font-size="10" fill="{}" font-family="{}">cd</text>"#,
            legend_x + legend_width / 2.0,
            self.margin_top - 8.0,
            theme.text,
            theme.font_family
        ));

        // Max value indicator
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="25" text-anchor="end" font-size="11" fill="{}" font-family="{}">Max: {:.0} cd</text>"#,
            width - 15.0,
            theme.text_secondary,
            theme.font_family,
            self.max_candela
        ));

        svg.push_str("</svg>");
        svg
    }
}

impl ButterflyDiagram {
    /// Generate complete SVG string for the butterfly diagram
    pub fn to_svg(&self, width: f64, height: f64, theme: &SvgTheme) -> String {
        let cx = width / 2.0;
        let cy = height / 2.0 + 25.0;
        let margin = 70.0;
        let max_radius = (width.min(height) / 2.0) - margin;

        let mut svg = String::new();

        // SVG header
        svg.push_str(&format!(
            r#"<svg viewBox="0 0 {width} {height}" xmlns="http://www.w3.org/2000/svg">"#
        ));

        // Background
        svg.push_str(&format!(
            r#"<rect x="0" y="0" width="{width}" height="{height}" fill="{}"/>"#,
            theme.background
        ));

        // Plot area background (ellipse)
        svg.push_str(&format!(
            r#"<ellipse cx="{cx}" cy="{cy}" rx="{:.1}" ry="{:.1}" fill="{}" stroke="{}" stroke-width="1"/>"#,
            max_radius + 10.0,
            (max_radius + 10.0) * 0.5,
            theme.surface,
            theme.axis
        ));

        // Grid circles
        for (i, points) in self.grid_circles.iter().enumerate() {
            let value = self.scale.grid_values.get(i).copied().unwrap_or(0.0);
            if points.len() > 1 {
                let mut path = format!("M {:.1} {:.1}", points[0].x, points[0].y);
                for p in &points[1..] {
                    path.push_str(&format!(" L {:.1} {:.1}", p.x, p.y));
                }
                path.push_str(" Z");
                svg.push_str(&format!(
                    r#"<path d="{path}" fill="none" stroke="{}" stroke-width="1"/>"#,
                    theme.grid
                ));

                // Intensity label
                svg.push_str(&format!(
                    r#"<text x="{:.1}" y="{:.1}" font-size="10" fill="{}" font-family="{}">{value:.0}</text>"#,
                    cx + 5.0,
                    cy - (value / self.scale.scale_max) * max_radius * 0.5 - 5.0,
                    theme.text_secondary,
                    theme.font_family
                ));
            }
        }

        // C-plane direction lines with labels
        for (c_angle, start, end) in &self.c_plane_lines {
            svg.push_str(&format!(
                r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="1"/>"#,
                start.x, start.y, end.x, end.y,
                theme.axis
            ));

            // Label at end
            let label_offset = 1.15;
            let lx = cx + (end.x - cx) * label_offset;
            let ly = cy + (end.y - cy) * label_offset;
            svg.push_str(&format!(
                r#"<text x="{lx:.1}" y="{ly:.1}" text-anchor="middle" dominant-baseline="middle" font-size="10" fill="{}" font-family="{}">C{:.0}</text>"#,
                theme.text_secondary,
                theme.font_family,
                c_angle
            ));
        }

        // Butterfly wings (back to front)
        for wing in self.wings.iter().rev() {
            let path = wing.to_svg_path();
            svg.push_str(&format!(
                r#"<path d="{path}" fill="{}" stroke="{}" stroke-width="1.5" opacity="0.85"/>"#,
                wing.fill_color.to_rgba_string(0.6),
                wing.stroke_color.to_rgb_string()
            ));
        }

        // Center point
        svg.push_str(&format!(
            r#"<circle cx="{cx}" cy="{cy}" r="4" fill="{}"/>"#,
            theme.text
        ));

        // Labels
        svg.push_str(&format!(
            r#"<text x="{cx}" y="25" text-anchor="middle" font-size="11" fill="{}" font-family="{}">0° (nadir)</text>"#,
            theme.text,
            theme.font_family
        ));

        svg.push_str(&format!(
            r#"<text x="15" y="25" font-size="12" fill="{}" font-weight="500" font-family="{}">3D Butterfly Diagram</text>"#,
            theme.text,
            theme.font_family
        ));

        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" text-anchor="end" font-size="11" fill="{}" font-family="{}">cd/klm</text>"#,
            width - 15.0,
            height - 12.0,
            theme.text_secondary,
            theme.font_family
        ));

        svg.push_str(&format!(
            r#"<text x="15" y="{:.1}" font-size="11" fill="{}" font-family="{}">Symmetry: {}</text>"#,
            height - 12.0,
            theme.text_secondary,
            theme.font_family,
            self.symmetry.description()
        ));

        svg.push_str("</svg>");
        svg
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Eulumdat;

    #[allow(clippy::field_reassign_with_default)]
    fn create_test_ldt() -> Eulumdat {
        let mut ldt = Eulumdat::default();
        ldt.symmetry = crate::Symmetry::BothPlanes;
        ldt.c_angles = vec![0.0, 30.0, 60.0, 90.0];
        ldt.g_angles = vec![0.0, 30.0, 60.0, 90.0];
        ldt.intensities = vec![
            vec![100.0, 90.0, 70.0, 40.0],
            vec![95.0, 85.0, 65.0, 35.0],
            vec![90.0, 80.0, 60.0, 30.0],
            vec![85.0, 75.0, 55.0, 25.0],
        ];
        ldt
    }

    #[test]
    fn test_polar_to_svg() {
        let ldt = create_test_ldt();
        let polar = PolarDiagram::from_eulumdat(&ldt);
        let svg = polar.to_svg(500.0, 500.0, &SvgTheme::light());

        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        assert!(svg.contains("C0-C180"));
        assert!(svg.contains("cd/1000lm"));
    }

    #[test]
    fn test_cartesian_to_svg() {
        let ldt = create_test_ldt();
        let cartesian = CartesianDiagram::from_eulumdat(&ldt, 500.0, 380.0, 8);
        let svg = cartesian.to_svg(500.0, 380.0, &SvgTheme::light());

        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        assert!(svg.contains("Gamma"));
    }

    #[test]
    fn test_theme_css_variables() {
        let theme = SvgTheme::css_variables();
        assert!(theme.background.starts_with("var("));
    }

    #[test]
    fn test_dark_theme() {
        let ldt = create_test_ldt();
        let polar = PolarDiagram::from_eulumdat(&ldt);
        let svg = polar.to_svg(500.0, 500.0, &SvgTheme::dark());

        assert!(svg.contains("#0f172a")); // Dark background
    }
}
