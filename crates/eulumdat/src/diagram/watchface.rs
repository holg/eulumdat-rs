//! Watch face diagram generation for Apple Watch
//!
//! Generates simplified polar diagrams optimized for watch faces.
//! The design uses the polar grid as watch markings with the
//! photometric distribution curve overlaid.
//!
//! # Sizes
//!
//! - Apple Watch 45mm: 396×484 pixels
//! - Apple Watch 41mm: 368×448 pixels
//! - Complication (accessoryCircular): 120×120 pixels max
//!
//! # Example
//!
//! ```rust,no_run
//! use eulumdat::{Eulumdat, diagram::{PolarDiagram, WatchFaceStyle}};
//!
//! let ldt = Eulumdat::from_file("luminaire.ldt").unwrap();
//! let polar = PolarDiagram::from_eulumdat(&ldt);
//! let svg = polar.to_watch_face_svg(396, 396, &WatchFaceStyle::default());
//! ```

use super::PolarDiagram;

/// Watch face style configuration
#[derive(Debug, Clone)]
pub struct WatchFaceStyle {
    /// Background color (use "transparent" for PNG with alpha)
    pub background: String,
    /// Grid/hour marker color
    pub grid_color: String,
    /// Main curve color (C0-C180)
    pub curve_primary: String,
    /// Secondary curve color (C90-C270)
    pub curve_secondary: String,
    /// Fill opacity for curves (0.0-1.0)
    pub fill_opacity: f64,
    /// Whether to show hour markers (12, 3, 6, 9)
    pub show_hour_markers: bool,
    /// Whether to show minute tick marks
    pub show_minute_ticks: bool,
    /// Whether to show the secondary (C90-C270) curve
    pub show_secondary_curve: bool,
    /// Stroke width for curves
    pub curve_stroke_width: f64,
    /// Grid line width
    pub grid_stroke_width: f64,
}

impl Default for WatchFaceStyle {
    fn default() -> Self {
        Self {
            background: "#000000".to_string(),
            grid_color: "#333333".to_string(),
            curve_primary: "#00D4FF".to_string(), // Cyan/blue glow
            curve_secondary: "#FF6B6B".to_string(), // Coral red
            fill_opacity: 0.2,
            show_hour_markers: true,
            show_minute_ticks: true,
            show_secondary_curve: true,
            curve_stroke_width: 2.0,
            grid_stroke_width: 1.0,
        }
    }
}

impl WatchFaceStyle {
    /// Dark style with cyan curves (default)
    pub fn dark() -> Self {
        Self::default()
    }

    /// Light style with blue curves
    pub fn light() -> Self {
        Self {
            background: "#FFFFFF".to_string(),
            grid_color: "#E0E0E0".to_string(),
            curve_primary: "#0066CC".to_string(),
            curve_secondary: "#CC3333".to_string(),
            fill_opacity: 0.15,
            ..Default::default()
        }
    }

    /// Minimal style - curves only, no grid
    pub fn minimal() -> Self {
        Self {
            background: "transparent".to_string(),
            grid_color: "#222222".to_string(),
            curve_primary: "#FFFFFF".to_string(),
            curve_secondary: "#888888".to_string(),
            fill_opacity: 0.1,
            show_hour_markers: false,
            show_minute_ticks: false,
            show_secondary_curve: false,
            curve_stroke_width: 2.5,
            grid_stroke_width: 0.5,
        }
    }

    /// Complication style - extra simplified for 120x120
    pub fn complication() -> Self {
        Self {
            background: "transparent".to_string(),
            grid_color: "#444444".to_string(),
            curve_primary: "#00D4FF".to_string(),
            curve_secondary: "#FF6B6B".to_string(),
            fill_opacity: 0.3,
            show_hour_markers: false,
            show_minute_ticks: false,
            show_secondary_curve: false,
            curve_stroke_width: 3.0,
            grid_stroke_width: 1.0,
        }
    }

    /// California style - warm amber tones
    pub fn california() -> Self {
        Self {
            background: "#1A1A2E".to_string(),
            grid_color: "#2D2D44".to_string(),
            curve_primary: "#FFB347".to_string(), // Warm amber
            curve_secondary: "#FF6B6B".to_string(),
            fill_opacity: 0.25,
            ..Default::default()
        }
    }
}

impl PolarDiagram {
    /// Generate watch face SVG
    ///
    /// Creates a circular SVG suitable for Apple Watch faces.
    /// The polar grid doubles as watch hour/minute markers.
    ///
    /// # Arguments
    /// * `width` - SVG width in pixels
    /// * `height` - SVG height in pixels (typically same as width for watch)
    /// * `style` - Watch face styling options
    pub fn to_watch_face_svg(&self, width: u32, height: u32, style: &WatchFaceStyle) -> String {
        let size = width.min(height) as f64;
        let center = size / 2.0;
        let margin = size * 0.08; // 8% margin
        let radius = (size / 2.0) - margin;
        let scale = self.scale.scale_max / radius;

        let mut svg = String::new();

        // SVG header with circular clip path
        svg.push_str(&format!(
            r#"<svg viewBox="0 0 {size} {size}" xmlns="http://www.w3.org/2000/svg">"#
        ));

        // Circular clip path for watch face
        svg.push_str(&format!(
            r#"<defs><clipPath id="watchClip"><circle cx="{center}" cy="{center}" r="{radius}"/></clipPath></defs>"#
        ));

        // Background
        if style.background != "transparent" {
            svg.push_str(&format!(
                r#"<circle cx="{center}" cy="{center}" r="{}" fill="{}"/>"#,
                center, style.background
            ));
        }

        // Outer ring
        svg.push_str(&format!(
            r#"<circle cx="{center}" cy="{center}" r="{:.1}" fill="none" stroke="{}" stroke-width="{}"/>"#,
            radius, style.grid_color, style.grid_stroke_width * 2.0
        ));

        // Concentric grid circles (like watch chapter ring)
        let num_circles = 4;
        for i in 1..=num_circles {
            let r = radius * (i as f64) / (num_circles as f64);
            svg.push_str(&format!(
                r#"<circle cx="{center}" cy="{center}" r="{r:.1}" fill="none" stroke="{}" stroke-width="{}" opacity="0.5"/>"#,
                style.grid_color, style.grid_stroke_width
            ));
        }

        // Hour markers (at 30° intervals = 12 positions)
        if style.show_hour_markers || style.show_minute_ticks {
            for i in 0..12 {
                let angle_deg = i as f64 * 30.0;
                let angle_rad = (angle_deg - 90.0).to_radians(); // 0° at top

                let is_major = i % 3 == 0; // 12, 3, 6, 9
                let inner_r = if is_major {
                    radius * 0.85
                } else {
                    radius * 0.92
                };
                let outer_r = radius * 0.98;

                let x1 = center + inner_r * angle_rad.cos();
                let y1 = center + inner_r * angle_rad.sin();
                let x2 = center + outer_r * angle_rad.cos();
                let y2 = center + outer_r * angle_rad.sin();

                let stroke_width = if is_major {
                    style.grid_stroke_width * 2.0
                } else {
                    style.grid_stroke_width
                };

                svg.push_str(&format!(
                    r#"<line x1="{x1:.1}" y1="{y1:.1}" x2="{x2:.1}" y2="{y2:.1}" stroke="{}" stroke-width="{stroke_width:.1}" stroke-linecap="round"/>"#,
                    style.grid_color
                ));
            }
        }

        // Minute tick marks (60 positions)
        if style.show_minute_ticks {
            for i in 0..60 {
                if i % 5 == 0 {
                    continue; // Skip hour positions
                }
                let angle_deg = i as f64 * 6.0;
                let angle_rad = (angle_deg - 90.0).to_radians();

                let inner_r = radius * 0.95;
                let outer_r = radius * 0.98;

                let x1 = center + inner_r * angle_rad.cos();
                let y1 = center + inner_r * angle_rad.sin();
                let x2 = center + outer_r * angle_rad.cos();
                let y2 = center + outer_r * angle_rad.sin();

                svg.push_str(&format!(
                    r#"<line x1="{x1:.1}" y1="{y1:.1}" x2="{x2:.1}" y2="{y2:.1}" stroke="{}" stroke-width="{:.1}" opacity="0.4"/>"#,
                    style.grid_color, style.grid_stroke_width * 0.5
                ));
            }
        }

        // Radial lines for photometric angles (every 30°)
        for i in 0..6 {
            let angle_deg = i as f64 * 30.0;
            let angle_rad = angle_deg.to_radians();

            // Lines from center outward (both sides)
            let x_left = center - radius * 0.8 * angle_rad.sin();
            let y_left = center + radius * 0.8 * angle_rad.cos();
            let x_right = center + radius * 0.8 * angle_rad.sin();
            let y_right = center + radius * 0.8 * angle_rad.cos();

            svg.push_str(&format!(
                r#"<line x1="{center}" y1="{center}" x2="{x_left:.1}" y2="{y_left:.1}" stroke="{}" stroke-width="{}" opacity="0.3"/>"#,
                style.grid_color, style.grid_stroke_width
            ));
            if i > 0 {
                svg.push_str(&format!(
                    r#"<line x1="{center}" y1="{center}" x2="{x_right:.1}" y2="{y_right:.1}" stroke="{}" stroke-width="{}" opacity="0.3"/>"#,
                    style.grid_color, style.grid_stroke_width
                ));
            }
        }

        // C0-C180 curve (primary)
        let path_c0_c180 = self.c0_c180_curve.to_svg_path(center, center, scale);
        if !path_c0_c180.is_empty() {
            // Fill
            svg.push_str(&format!(
                r#"<path d="{}" fill="{}" fill-opacity="{}" stroke="none" clip-path="url(#watchClip)"/>"#,
                path_c0_c180, style.curve_primary, style.fill_opacity
            ));
            // Stroke
            svg.push_str(&format!(
                r#"<path d="{}" fill="none" stroke="{}" stroke-width="{}" stroke-linecap="round" stroke-linejoin="round" clip-path="url(#watchClip)"/>"#,
                path_c0_c180, style.curve_primary, style.curve_stroke_width
            ));
        }

        // C90-C270 curve (secondary)
        if style.show_secondary_curve && self.show_c90_c270() {
            let path_c90_c270 = self.c90_c270_curve.to_svg_path(center, center, scale);
            if !path_c90_c270.is_empty() {
                // Fill
                svg.push_str(&format!(
                    r#"<path d="{}" fill="{}" fill-opacity="{}" stroke="none" clip-path="url(#watchClip)"/>"#,
                    path_c90_c270, style.curve_secondary, style.fill_opacity * 0.7
                ));
                // Stroke (dashed)
                svg.push_str(&format!(
                    r#"<path d="{}" fill="none" stroke="{}" stroke-width="{}" stroke-dasharray="{},{}" stroke-linecap="round" clip-path="url(#watchClip)"/>"#,
                    path_c90_c270,
                    style.curve_secondary,
                    style.curve_stroke_width,
                    style.curve_stroke_width * 3.0,
                    style.curve_stroke_width * 2.0
                ));
            }
        }

        // Center dot
        svg.push_str(&format!(
            r#"<circle cx="{center}" cy="{center}" r="{:.1}" fill="{}"/>"#,
            size * 0.015,
            style.curve_primary
        ));

        svg.push_str("</svg>");
        svg
    }

    /// Generate a minimal complication SVG (120x120 max)
    ///
    /// This is a simplified version for accessoryCircular complications.
    pub fn to_complication_svg(&self, size: u32) -> String {
        self.to_watch_face_svg(size, size, &WatchFaceStyle::complication())
    }

    /// Generate PNG-ready SVG for Photos face background
    ///
    /// Creates an SVG sized for Apple Watch Photos face:
    /// - 45mm: 396×484
    /// - 41mm: 368×448
    ///
    /// The diagram is centered with space at bottom for complications.
    pub fn to_photos_face_svg(&self, width: u32, height: u32, style: &WatchFaceStyle) -> String {
        // For Photos face, we center the circular diagram with extra space at bottom
        let diagram_size = width.min(height - 60); // Leave space for complications
        let offset_x = (width - diagram_size) / 2;
        let offset_y = 20; // Small top margin

        let inner_svg = self.to_watch_face_svg(diagram_size, diagram_size, style);

        // Wrap in container SVG with proper sizing
        format!(
            r#"<svg viewBox="0 0 {width} {height}" xmlns="http://www.w3.org/2000/svg">
<rect width="{width}" height="{height}" fill="{}"/>
<g transform="translate({offset_x}, {offset_y})">
{}
</g>
</svg>"#,
            style.background,
            // Strip the outer svg tags from inner
            inner_svg
                .strip_prefix(&format!(r#"<svg viewBox="0 0 {diagram_size} {diagram_size}" xmlns="http://www.w3.org/2000/svg">"#))
                .unwrap_or(&inner_svg)
                .strip_suffix("</svg>")
                .unwrap_or(&inner_svg)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Eulumdat, Symmetry};

    fn create_test_ldt() -> Eulumdat {
        Eulumdat {
            symmetry: Symmetry::BothPlanes,
            c_angles: vec![0.0, 30.0, 60.0, 90.0],
            g_angles: vec![0.0, 30.0, 60.0, 90.0, 120.0, 150.0, 180.0],
            intensities: vec![
                vec![100.0, 95.0, 80.0, 50.0, 20.0, 5.0, 0.0], // C0
                vec![95.0, 90.0, 75.0, 45.0, 18.0, 4.0, 0.0],  // C30
                vec![90.0, 85.0, 70.0, 40.0, 15.0, 3.0, 0.0],  // C60
                vec![85.0, 80.0, 65.0, 35.0, 12.0, 2.0, 0.0],  // C90
            ],
            ..Default::default()
        }
    }

    #[test]
    fn test_watch_face_svg() {
        let ldt = create_test_ldt();
        let polar = PolarDiagram::from_eulumdat(&ldt);
        let svg = polar.to_watch_face_svg(396, 396, &WatchFaceStyle::default());

        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        assert!(svg.contains("watchClip"));
        assert!(svg.contains("#00D4FF")); // Default curve color
    }

    #[test]
    fn test_complication_svg() {
        let ldt = create_test_ldt();
        let polar = PolarDiagram::from_eulumdat(&ldt);
        let svg = polar.to_complication_svg(120);

        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("viewBox=\"0 0 120 120\""));
    }

    #[test]
    fn test_photos_face_svg() {
        let ldt = create_test_ldt();
        let polar = PolarDiagram::from_eulumdat(&ldt);
        let svg = polar.to_photos_face_svg(396, 484, &WatchFaceStyle::dark());

        assert!(svg.contains("viewBox=\"0 0 396 484\""));
        assert!(svg.contains("transform=\"translate"));
    }

    #[test]
    fn test_all_styles() {
        let ldt = create_test_ldt();
        let polar = PolarDiagram::from_eulumdat(&ldt);

        // All styles should produce valid SVG
        for style in [
            WatchFaceStyle::dark(),
            WatchFaceStyle::light(),
            WatchFaceStyle::minimal(),
            WatchFaceStyle::complication(),
            WatchFaceStyle::california(),
        ] {
            let svg = polar.to_watch_face_svg(200, 200, &style);
            assert!(
                svg.starts_with("<svg"),
                "Style {:?} failed",
                style.background
            );
        }
    }
}
