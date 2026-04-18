//! SVG rendering for photometric diagrams
//!
//! This module generates complete SVG strings for each diagram type.
//! The SVGs can be used directly in browsers, iOS (via WebView or SVG libraries),
//! Android, or any other platform that supports SVG.
//!
//! # Responsive SVGs
//!
//! All SVGs use `viewBox` for coordinate systems, allowing them to scale to any size.
//! Use `DetailLevel` to control what gets rendered at different sizes:
//!
//! - `Full`: All labels, legends, grid lines (for large displays)
//! - `Standard`: Normal detail level (default)
//! - `Compact`: Reduced labels, smaller fonts (for medium displays)
//! - `Minimal`: Essential elements only (for small/mobile displays)
//!
//! # Example
//!
//! ```rust,no_run
//! use eulumdat::{Eulumdat, diagram::{PolarDiagram, SvgTheme, DetailLevel}};
//!
//! let ldt = Eulumdat::from_file("luminaire.ldt").unwrap();
//! let polar = PolarDiagram::from_eulumdat(&ldt);
//!
//! // Full detail for large displays
//! let svg = polar.to_svg_responsive(500.0, 500.0, &SvgTheme::light(), DetailLevel::Full);
//!
//! // Minimal for mobile
//! let svg = polar.to_svg_responsive(300.0, 300.0, &SvgTheme::light(), DetailLevel::Minimal);
//! ```

use super::{
    ButterflyDiagram, CartesianCurve, CartesianDiagram, ConeDiagram, DiagramScale,
    FloodlightCartesianDiagram, HeatmapDiagram, IsocandelaDiagram, IsoluxDiagram, PolarDiagram,
    YScale,
};
use crate::units::UnitSystem;

/// Format an illuminance value with appropriate precision.
///
/// Uses more decimal places for small values so they don't all show as "0".
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

/// Detail level for SVG rendering
///
/// Controls what elements are rendered based on display size.
/// Use `from_width()` to automatically select appropriate level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DetailLevel {
    /// Full detail: all labels, legends, grid lines, summary boxes
    /// Recommended for widths >= 500px
    Full,
    /// Standard detail level (default)
    /// Recommended for widths >= 400px
    #[default]
    Standard,
    /// Compact: reduced labels, smaller fonts, simplified legends
    /// Recommended for widths >= 300px
    Compact,
    /// Minimal: essential elements only, no legend, minimal labels
    /// Recommended for widths < 300px
    Minimal,
}

impl DetailLevel {
    /// Select appropriate detail level based on container width
    pub fn from_width(width: f64) -> Self {
        if width >= 500.0 {
            Self::Full
        } else if width >= 400.0 {
            Self::Standard
        } else if width >= 300.0 {
            Self::Compact
        } else {
            Self::Minimal
        }
    }

    /// Get font size multiplier for this detail level
    pub fn font_scale(&self) -> f64 {
        match self {
            Self::Full => 1.0,
            Self::Standard => 1.0,
            Self::Compact => 0.85,
            Self::Minimal => 0.75,
        }
    }

    /// Whether to show the legend
    pub fn show_legend(&self) -> bool {
        !matches!(self, Self::Minimal)
    }

    /// Whether to show axis labels
    pub fn show_axis_labels(&self) -> bool {
        !matches!(self, Self::Minimal)
    }

    /// Whether to show summary/info boxes
    pub fn show_summary(&self) -> bool {
        matches!(self, Self::Full | Self::Standard)
    }

    /// Number of grid divisions to show
    pub fn grid_divisions(&self) -> usize {
        match self {
            Self::Full => 5,
            Self::Standard => 5,
            Self::Compact => 4,
            Self::Minimal => 3,
        }
    }

    /// Angle label step (degrees between labels)
    pub fn angle_label_step(&self) -> f64 {
        match self {
            Self::Full => 30.0,
            Self::Standard => 30.0,
            Self::Compact => 45.0,
            Self::Minimal => 90.0,
        }
    }
}

/// Localized labels for SVG diagram text
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SvgLabels {
    /// Intensity unit (e.g., "cd/1000lm")
    pub intensity_unit: String,
    /// Gamma axis label (e.g., "Gamma (γ)")
    pub gamma_axis: String,
    /// Intensity axis label (e.g., "Intensity (cd/klm)")
    pub intensity_axis: String,
    /// C0-C180 plane label
    pub plane_c0_c180: String,
    /// C90-C270 plane label
    pub plane_c90_c270: String,
    /// Beam angle label
    pub beam: String,
    /// Field angle label
    pub field: String,
    /// Beam 50% threshold label
    pub beam_50_percent: String,
    /// Field 10% threshold label
    pub field_10_percent: String,
    /// CIE classification prefix
    pub cie_label: String,
    /// Efficacy prefix
    pub efficacy_label: String,
    /// Maximum prefix
    pub max_label: String,
    /// Spacing/height ratio prefix
    pub sh_ratio_label: String,
    /// C-plane axis label
    pub c_plane_axis: String,
    /// Gamma angle axis label
    pub gamma_angle_axis: String,
    /// Heatmap title
    pub heatmap_title: String,
    /// No data placeholder
    pub no_data: String,
    // BUG rating labels
    /// Forward light zone label
    pub bug_forward_light: String,
    /// Back light zone label
    pub bug_back_light: String,
    /// Uplight zone label
    pub bug_uplight: String,
    /// Total label
    pub bug_total: String,
    /// Sum label
    pub bug_sum: String,
    /// Low zone label
    pub bug_zone_low: String,
    /// Medium zone label
    pub bug_zone_medium: String,
    /// High zone label
    pub bug_zone_high: String,
    /// Very high zone label
    pub bug_zone_very_high: String,
}

impl Default for SvgLabels {
    fn default() -> Self {
        Self::english()
    }
}

impl SvgLabels {
    /// English labels (default)
    pub fn english() -> Self {
        Self {
            intensity_unit: "cd/1000lm".to_string(),
            gamma_axis: "Gamma (γ)".to_string(),
            intensity_axis: "Intensity (cd/klm)".to_string(),
            plane_c0_c180: "C0-C180".to_string(),
            plane_c90_c270: "C90-C270".to_string(),
            beam: "Beam".to_string(),
            field: "Field".to_string(),
            beam_50_percent: "Beam 50%".to_string(),
            field_10_percent: "Field 10%".to_string(),
            cie_label: "CIE:".to_string(),
            efficacy_label: "Eff:".to_string(),
            max_label: "Max:".to_string(),
            sh_ratio_label: "S/H:".to_string(),
            c_plane_axis: "C-Plane Angle (°)".to_string(),
            gamma_angle_axis: "Gamma Angle (°)".to_string(),
            heatmap_title: "Intensity Heatmap (Candela)".to_string(),
            no_data: "No data".to_string(),
            bug_forward_light: "Forward Light".to_string(),
            bug_back_light: "Back Light".to_string(),
            bug_uplight: "Uplight".to_string(),
            bug_total: "Total".to_string(),
            bug_sum: "Sum".to_string(),
            bug_zone_low: "Low".to_string(),
            bug_zone_medium: "Medium".to_string(),
            bug_zone_high: "High".to_string(),
            bug_zone_very_high: "Very High".to_string(),
        }
    }

    /// Create labels from eulumdat-i18n Locale
    #[cfg(feature = "i18n")]
    pub fn from_locale(locale: &eulumdat_i18n::Locale) -> Self {
        Self {
            intensity_unit: locale.diagram.units.intensity.clone(),
            gamma_axis: locale.diagram.axis.gamma.clone(),
            intensity_axis: locale.diagram.axis.intensity.clone(),
            plane_c0_c180: locale.diagram.plane.c0_c180.clone(),
            plane_c90_c270: locale.diagram.plane.c90_c270.clone(),
            beam: locale.diagram.angle.beam.clone(),
            field: locale.diagram.angle.field.clone(),
            beam_50_percent: locale.diagram.angle.beam_50.clone(),
            field_10_percent: locale.diagram.angle.field_10.clone(),
            cie_label: locale.diagram.metrics.cie.clone(),
            efficacy_label: locale.diagram.metrics.efficacy.clone(),
            max_label: locale.diagram.metrics.max.clone(),
            sh_ratio_label: locale.diagram.metrics.sh_ratio.clone(),
            c_plane_axis: locale.diagram.axis.c_plane.clone(),
            gamma_angle_axis: locale.diagram.axis.gamma_angle.clone(),
            heatmap_title: locale.diagram.title.heatmap.clone(),
            no_data: locale.diagram.placeholder.no_data.clone(),
            bug_forward_light: locale.diagram.bug.forward_light.clone(),
            bug_back_light: locale.diagram.bug.back_light.clone(),
            bug_uplight: locale.diagram.bug.uplight.clone(),
            bug_total: locale.diagram.bug.total.clone(),
            bug_sum: locale.diagram.bug.sum.clone(),
            bug_zone_low: locale.diagram.bug.zone_low.clone(),
            bug_zone_medium: locale.diagram.bug.zone_medium.clone(),
            bug_zone_high: locale.diagram.bug.zone_high.clone(),
            bug_zone_very_high: locale.diagram.bug.zone_very_high.clone(),
        }
    }
}

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
    /// Localized labels for diagram text
    pub labels: SvgLabels,
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
            labels: SvgLabels::default(),
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
            labels: SvgLabels::default(),
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
            labels: SvgLabels::default(),
        }
    }

    /// Set labels for this theme (for i18n)
    pub fn with_labels(mut self, labels: SvgLabels) -> Self {
        self.labels = labels;
        self
    }

    /// Create theme with locale labels
    #[cfg(feature = "i18n")]
    pub fn light_with_locale(locale: &eulumdat_i18n::Locale) -> Self {
        Self::light().with_labels(SvgLabels::from_locale(locale))
    }

    /// Create dark theme with locale labels
    #[cfg(feature = "i18n")]
    pub fn dark_with_locale(locale: &eulumdat_i18n::Locale) -> Self {
        Self::dark().with_labels(SvgLabels::from_locale(locale))
    }

    /// Create CSS variables theme with locale labels (for web with dynamic theming + i18n)
    #[cfg(feature = "i18n")]
    pub fn css_variables_with_locale(locale: &eulumdat_i18n::Locale) -> Self {
        Self::css_variables().with_labels(SvgLabels::from_locale(locale))
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

/// Rendering configuration for the isometric isolux SVG view.
///
/// Controls layout margins, visual sizes, contour rendering, colors,
/// and font sizes. All current hardcoded values are preserved as defaults.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IsometricConfig {
    // ── Layout margins ──
    /// Horizontal margin in pixels
    pub margin_x: f64,
    /// Top margin in pixels
    pub margin_top: f64,
    /// Bottom margin in pixels
    pub margin_bottom: f64,
    /// Space reserved for the legend on the right
    pub legend_width: f64,

    // ── Pole & mini polar ──
    /// Height of the vertical pole in pixels
    pub pole_height_px: f64,
    /// Radius of the mini polar diagram at the pole top
    pub polar_radius: f64,

    // ── Contour rendering ──
    /// Fractional lux levels at which to draw contour lines (relative to max_lux)
    pub contour_fractions: Vec<f64>,
    /// Number of angular samples when tracing each contour
    pub contour_num_angles: usize,
    /// Radial march steps for contour tracing
    pub contour_march_steps: usize,

    // ── Legend bar ──
    /// Width of the gradient legend bar in pixels
    pub legend_bar_width: f64,
    /// Number of gradient segments in the legend
    pub legend_num_segments: usize,
    /// Legend height as a fraction of available height
    pub legend_height_fraction: f64,

    // ── Font sizes ──
    /// Title font size
    pub title_font_size: f64,
    /// Distance/axis label font size
    pub label_font_size: f64,
    /// Contour illuminance label font size
    pub contour_label_font_size: f64,
    /// Info line font size
    pub info_font_size: f64,

    // ── Grayscale mapping ──
    /// Maximum darkening factor for the grayscale gradient (0.0–1.0)
    pub grayscale_max_darken: f64,

    // ── Isometric-specific colors ──
    /// Grid line color on the ground plane
    pub grid_color: String,
    /// Grid line stroke width
    pub grid_stroke_width: f64,
    /// Vertical pole color
    pub pole_color: String,
    /// Vertical pole stroke width
    pub pole_stroke_width: f64,
    /// Mini polar diagram fill color
    pub polar_fill: String,
    /// Mini polar diagram stroke color
    pub polar_stroke: String,
    /// Mini polar diagram stroke width
    pub polar_stroke_width: f64,

    // ── Contour line styling ──
    /// Minimum gray value for contour lines (0–255, lower = darker)
    pub contour_gray_min: f64,
    /// Gray range added for lighter contours
    pub contour_gray_range: f64,
    /// Minimum contour stroke width
    pub contour_stroke_width_min: f64,
    /// Additional stroke width scaled by normalized lux
    pub contour_stroke_width_range: f64,
}

impl Default for IsometricConfig {
    fn default() -> Self {
        Self {
            margin_x: 50.0,
            margin_top: 50.0,
            margin_bottom: 30.0,
            legend_width: 50.0,

            pole_height_px: 90.0,
            polar_radius: 25.0,

            contour_fractions: vec![0.1, 0.2, 0.35, 0.5, 0.7, 0.85],
            contour_num_angles: 72,
            contour_march_steps: 80,

            legend_bar_width: 12.0,
            legend_num_segments: 40,
            legend_height_fraction: 0.6,

            title_font_size: 14.0,
            label_font_size: 9.0,
            contour_label_font_size: 8.0,
            info_font_size: 10.0,

            grayscale_max_darken: 0.7,

            grid_color: "rgba(180,180,180,0.4)".to_string(),
            grid_stroke_width: 0.5,
            pole_color: "rgb(153,153,153)".to_string(),
            pole_stroke_width: 1.5,
            polar_fill: "rgba(220,80,80,0.25)".to_string(),
            polar_stroke: "rgb(200,80,80)".to_string(),
            polar_stroke_width: 1.0,

            contour_gray_min: 80.0,
            contour_gray_range: 120.0,
            contour_stroke_width_min: 0.8,
            contour_stroke_width_range: 1.2,
        }
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
            r#"<text x="22" y="12" font-size="12" fill="{}" font-family="{}">{}</text>"#,
            theme.text, theme.font_family, theme.labels.plane_c0_c180
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
                r#"<text x="22" y="12" font-size="12" fill="{}" font-family="{}">{}</text>"#,
                theme.text, theme.font_family, theme.labels.plane_c90_c270
            ));
            svg.push_str("</g>");
        }

        // Unit label
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" text-anchor="end" font-size="11" fill="{}" font-family="{}">{}</text>"#,
            size - 15.0,
            size - 15.0,
            theme.text_secondary,
            theme.font_family,
            theme.labels.intensity_unit
        ));

        svg.push_str("</svg>");
        svg
    }

    /// Generate responsive SVG string for the polar diagram
    ///
    /// This version adds CSS classes for responsive behavior and adjusts
    /// detail level based on the `DetailLevel` parameter. Elements are tagged
    /// with classes like `detail-full`, `detail-standard`, etc. that can be
    /// hidden via CSS at different breakpoints.
    ///
    /// For automatic sizing, use `width="100%"` and `height="100%"` and let
    /// the viewBox handle scaling.
    pub fn to_svg_responsive(
        &self,
        width: f64,
        height: f64,
        theme: &SvgTheme,
        detail: DetailLevel,
    ) -> String {
        let size = width.min(height);
        let center = size / 2.0;
        let margin = match detail {
            DetailLevel::Full | DetailLevel::Standard => 60.0,
            DetailLevel::Compact => 50.0,
            DetailLevel::Minimal => 40.0,
        };
        let radius = (size / 2.0) - margin;
        let scale = self.scale.scale_max / radius;
        let font_scale = detail.font_scale();

        let mut svg = String::new();

        // SVG header with responsive attributes
        svg.push_str(&format!(
            r#"<svg viewBox="0 0 {size} {size}" xmlns="http://www.w3.org/2000/svg" class="diagram-polar" preserveAspectRatio="xMidYMid meet">"#
        ));

        // Embedded responsive styles
        svg.push_str(&format!(
            r#"<style>
.diagram-label {{ font-family: {}; }}
.detail-compact, .detail-full {{ display: block; }}
@media (max-width: 400px) {{ .detail-full {{ display: none; }} }}
@media (max-width: 300px) {{ .detail-compact, .detail-full {{ display: none; }} }}
</style>"#,
            theme.font_family
        ));

        // Background
        svg.push_str(&format!(
            r#"<rect x="0" y="0" width="{size}" height="{size}" fill="{}"/>"#,
            theme.background
        ));

        // Grid circles (reduced based on detail level)
        let grid_step = match detail {
            DetailLevel::Full | DetailLevel::Standard => 1,
            DetailLevel::Compact => 2,
            DetailLevel::Minimal => 2,
        };
        let num_circles = self.scale.grid_values.len();
        for (i, &value) in self.scale.grid_values.iter().enumerate() {
            if i % grid_step != 0 && i != num_circles - 1 {
                continue;
            }
            let r = value / scale;
            let is_major = i == num_circles - 1 || i == num_circles / 2;
            let stroke_color = if is_major { &theme.axis } else { &theme.grid };
            let stroke_width = if is_major { "1.5" } else { "1" };

            svg.push_str(&format!(
                r#"<circle cx="{center}" cy="{center}" r="{r:.1}" fill="none" stroke="{stroke_color}" stroke-width="{stroke_width}"/>"#
            ));

            // Intensity label (hide on minimal)
            if detail.show_axis_labels() {
                let class = if i == num_circles - 1 {
                    ""
                } else {
                    r#" class="detail-compact""#
                };
                svg.push_str(&format!(
                    r#"<text x="{:.1}" y="{:.1}" font-size="{:.0}" fill="{}"{class}>{:.0}</text>"#,
                    center + 5.0,
                    center + r + 12.0,
                    11.0 * font_scale,
                    theme.text_secondary,
                    value
                ));
            }
        }

        // Radial lines (step based on detail level)
        let angle_step = detail.angle_label_step() as usize / 30;
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

            // Only draw lines for angles we'll label (or always for minimal set)
            let draw_line = i % angle_step == 0 || matches!(detail, DetailLevel::Full);
            if draw_line {
                svg.push_str(&format!(
                    r#"<line x1="{center}" y1="{center}" x2="{x1_left:.1}" y2="{y1_left:.1}" stroke="{}" stroke-width="1"/>"#,
                    theme.grid
                ));
                svg.push_str(&format!(
                    r#"<line x1="{center}" y1="{center}" x2="{x1_right:.1}" y2="{y1_right:.1}" stroke="{}" stroke-width="1"/>"#,
                    theme.grid
                ));
            }

            // Angle labels (based on detail level)
            if detail.show_axis_labels() && angle_deg > 0.0 && angle_deg < 180.0 {
                let show_this_label = (i * 30) as f64 % detail.angle_label_step() == 0.0;
                if show_this_label {
                    let label_offset = radius + 18.0;
                    let label_x_left = center - label_offset * angle_rad.sin();
                    let label_y_left = center + label_offset * angle_rad.cos();
                    let label_x_right = center + label_offset * angle_rad.sin();
                    let label_y_right = center + label_offset * angle_rad.cos();

                    svg.push_str(&format!(
                        r#"<text x="{label_x_left:.1}" y="{label_y_left:.1}" text-anchor="middle" dominant-baseline="middle" font-size="{:.0}" fill="{}">{angle_deg:.0}°</text>"#,
                        11.0 * font_scale, theme.text_secondary
                    ));
                    svg.push_str(&format!(
                        r#"<text x="{label_x_right:.1}" y="{label_y_right:.1}" text-anchor="middle" dominant-baseline="middle" font-size="{:.0}" fill="{}">{angle_deg:.0}°</text>"#,
                        11.0 * font_scale, theme.text_secondary
                    ));
                }
            }
        }

        // 180° label at top
        if detail.show_axis_labels() {
            svg.push_str(&format!(
                r#"<text x="{center}" y="{:.1}" text-anchor="middle" font-size="{:.0}" fill="{}" class="detail-compact">180°</text>"#,
                center - radius - 15.0,
                11.0 * font_scale,
                theme.text_secondary
            ));
        }

        // 90° horizontal line
        svg.push_str(&format!(
            r#"<line x1="{:.1}" y1="{center}" x2="{:.1}" y2="{center}" stroke="{}" stroke-width="1.5"/>"#,
            center - radius,
            center + radius,
            theme.axis
        ));

        // 90° labels
        if detail.show_axis_labels() {
            svg.push_str(&format!(
                r#"<text x="{:.1}" y="{center}" text-anchor="middle" dominant-baseline="middle" font-size="{:.0}" fill="{}">90°</text>"#,
                center - radius - 15.0,
                11.0 * font_scale,
                theme.text_secondary
            ));
            svg.push_str(&format!(
                r#"<text x="{:.1}" y="{center}" text-anchor="middle" dominant-baseline="middle" font-size="{:.0}" fill="{}">90°</text>"#,
                center + radius + 15.0,
                11.0 * font_scale,
                theme.text_secondary
            ));
        }

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

        // Legend (hide on minimal)
        if detail.show_legend() {
            svg.push_str(&format!(
                r#"<g transform="translate(15, {:.1})" class="detail-compact">"#,
                size - 55.0
            ));
            svg.push_str(&format!(
                r#"<rect x="0" y="0" width="16" height="16" fill="{}" stroke="{}" stroke-width="2" rx="2"/>"#,
                theme.curve_c0_c180_fill,
                theme.curve_c0_c180
            ));
            svg.push_str(&format!(
                r#"<text x="22" y="12" font-size="{:.0}" fill="{}">{}</text>"#,
                12.0 * font_scale,
                theme.text,
                theme.labels.plane_c0_c180
            ));
            svg.push_str("</g>");

            if self.show_c90_c270() {
                svg.push_str(&format!(
                    r#"<g transform="translate(15, {:.1})" class="detail-compact">"#,
                    size - 32.0
                ));
                svg.push_str(&format!(
                    r#"<rect x="0" y="0" width="16" height="16" fill="{}" stroke="{}" stroke-width="2" stroke-dasharray="4,2" rx="2"/>"#,
                    theme.curve_c90_c270_fill,
                    theme.curve_c90_c270
                ));
                svg.push_str(&format!(
                    r#"<text x="22" y="12" font-size="{:.0}" fill="{}">{}</text>"#,
                    12.0 * font_scale,
                    theme.text,
                    theme.labels.plane_c90_c270
                ));
                svg.push_str("</g>");
            }
        }

        // Unit label
        if detail.show_axis_labels() {
            svg.push_str(&format!(
                r#"<text x="{:.1}" y="{:.1}" text-anchor="end" font-size="{:.0}" fill="{}" class="detail-compact">{}</text>"#,
                size - 15.0,
                size - 15.0,
                11.0 * font_scale,
                theme.text_secondary,
                theme.labels.intensity_unit
            ));
        }

        svg.push_str("</svg>");
        svg
    }

    /// Generate SVG with photometric summary overlay
    ///
    /// Adds beam/field angle markers and a summary info box
    pub fn to_svg_with_summary(
        &self,
        width: f64,
        height: f64,
        theme: &SvgTheme,
        summary: &crate::calculations::PhotometricSummary,
    ) -> String {
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
                continue;
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

        // Color constants for markers
        let green = "#22c55e"; // IES beam angle
        let orange = "#f97316"; // Field angle
        let blue = "#3b82f6"; // CIE beam angle (for batwing)

        // === BEAM ANGLE MARKER (IES - 50% of max intensity) ===
        // Note: beam_angle is now the full angle per CIE S 017:2020
        // We need the half angle for drawing (angle from nadir to edge)
        let half_beam = summary.beam_angle / 2.0;
        if half_beam > 0.0 && half_beam < 90.0 {
            let beam_rad = half_beam.to_radians();
            // Draw arc from center to beam angle on both sides
            let arc_radius = radius * 0.85;
            let x1 = center - arc_radius * beam_rad.sin();
            let y1 = center + arc_radius * beam_rad.cos();
            let x2 = center + arc_radius * beam_rad.sin();
            let y2 = center + arc_radius * beam_rad.cos();

            // Dashed arc for beam angle
            svg.push_str(&format!(
                r#"<line x1="{}" y1="{}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="1.5" stroke-dasharray="4,3" opacity="0.8"/>"#,
                center, center, x1, y1, green
            ));
            svg.push_str(&format!(
                r#"<line x1="{}" y1="{}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="1.5" stroke-dasharray="4,3" opacity="0.8"/>"#,
                center, center, x2, y2, green
            ));

            // Small circles at the end points
            svg.push_str(&format!(
                r#"<circle cx="{:.1}" cy="{:.1}" r="4" fill="{}" opacity="0.8"/>"#,
                x1, y1, green
            ));
            svg.push_str(&format!(
                r#"<circle cx="{:.1}" cy="{:.1}" r="4" fill="{}" opacity="0.8"/>"#,
                x2, y2, green
            ));
        }

        // === FIELD ANGLE MARKER (10% intensity) ===
        // Note: field_angle is now the full angle per CIE S 017:2020
        let half_field = summary.field_angle / 2.0;
        if half_field > 0.0 && half_field < 90.0 {
            let field_rad = half_field.to_radians();
            let arc_radius = radius * 0.9;
            let x1 = center - arc_radius * field_rad.sin();
            let y1 = center + arc_radius * field_rad.cos();
            let x2 = center + arc_radius * field_rad.sin();
            let y2 = center + arc_radius * field_rad.cos();

            // Dotted arc for field angle
            svg.push_str(&format!(
                r#"<line x1="{}" y1="{}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="1.5" stroke-dasharray="2,3" opacity="0.7"/>"#,
                center, center, x1, y1, orange
            ));
            svg.push_str(&format!(
                r#"<line x1="{}" y1="{}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="1.5" stroke-dasharray="2,3" opacity="0.7"/>"#,
                center, center, x2, y2, orange
            ));

            // Small diamonds at the end points
            svg.push_str(&format!(
                r#"<rect x="{:.1}" y="{:.1}" width="6" height="6" fill="{}" opacity="0.7" transform="rotate(45 {} {})"/>"#,
                x1 - 3.0, y1 - 3.0, orange, x1, y1
            ));
            svg.push_str(&format!(
                r#"<rect x="{:.1}" y="{:.1}" width="6" height="6" fill="{}" opacity="0.7" transform="rotate(45 {} {})"/>"#,
                x2 - 3.0, y2 - 3.0, orange, x2, y2
            ));
        }

        // === CIE BEAM ANGLE MARKER (50% of center intensity) - only for batwing ===
        // Note: beam_angle_cie is now the full angle per CIE S 017:2020
        let half_cie_beam = summary.beam_angle_cie / 2.0;
        if summary.is_batwing && half_cie_beam > 0.0 && half_cie_beam < 90.0 {
            let cie_rad = half_cie_beam.to_radians();
            let arc_radius = radius * 0.80; // Slightly inside IES marker
            let x1 = center - arc_radius * cie_rad.sin();
            let y1 = center + arc_radius * cie_rad.cos();
            let x2 = center + arc_radius * cie_rad.sin();
            let y2 = center + arc_radius * cie_rad.cos();

            // Solid lines for CIE beam angle (blue)
            svg.push_str(&format!(
                r#"<line x1="{}" y1="{}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="1.5" opacity="0.8"/>"#,
                center, center, x1, y1, blue
            ));
            svg.push_str(&format!(
                r#"<line x1="{}" y1="{}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="1.5" opacity="0.8"/>"#,
                center, center, x2, y2, blue
            ));

            // Small triangles at the end points
            svg.push_str(&format!(
                r#"<polygon points="{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}" fill="{}" opacity="0.8"/>"#,
                x1, y1 - 5.0, x1 - 4.0, y1 + 3.0, x1 + 4.0, y1 + 3.0, blue
            ));
            svg.push_str(&format!(
                r#"<polygon points="{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}" fill="{}" opacity="0.8"/>"#,
                x2, y2 - 5.0, x2 - 4.0, y2 + 3.0, x2 + 4.0, y2 + 3.0, blue
            ));
        }

        // === UPWARD BEAM ANGLE MARKER (for uplights/direct-indirect) ===
        let purple = "#a855f7"; // Upward beam angle color
        let half_upward_beam = summary.upward_beam_angle / 2.0;
        if half_upward_beam > 0.0 && half_upward_beam < 90.0 {
            // Draw upward beam angle lines in upper hemisphere
            // For upward light, we draw from center going UP (negative y direction)
            let upward_rad = half_upward_beam.to_radians();
            let arc_radius = radius * 0.85;
            // Note: y is inverted (negative) for upper hemisphere
            let x1 = center - arc_radius * upward_rad.sin();
            let y1 = center - arc_radius * upward_rad.cos(); // Negative for upward
            let x2 = center + arc_radius * upward_rad.sin();
            let y2 = center - arc_radius * upward_rad.cos(); // Negative for upward

            // Dashed lines for upward beam angle
            svg.push_str(&format!(
                r#"<line x1="{}" y1="{}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="1.5" stroke-dasharray="4,3" opacity="0.8"/>"#,
                center, center, x1, y1, purple
            ));
            svg.push_str(&format!(
                r#"<line x1="{}" y1="{}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="1.5" stroke-dasharray="4,3" opacity="0.8"/>"#,
                center, center, x2, y2, purple
            ));

            // Small circles at end points
            svg.push_str(&format!(
                r#"<circle cx="{:.1}" cy="{:.1}" r="4" fill="{}" opacity="0.8"/>"#,
                x1, y1, purple
            ));
            svg.push_str(&format!(
                r#"<circle cx="{:.1}" cy="{:.1}" r="4" fill="{}" opacity="0.8"/>"#,
                x2, y2, purple
            ));
        }

        // === UPWARD FIELD ANGLE MARKER ===
        let pink = "#ec4899"; // Upward field angle color
        let half_upward_field = summary.upward_field_angle / 2.0;
        if half_upward_field > 0.0 && half_upward_field < 90.0 {
            let upward_field_rad = half_upward_field.to_radians();
            let arc_radius = radius * 0.9;
            let x1 = center - arc_radius * upward_field_rad.sin();
            let y1 = center - arc_radius * upward_field_rad.cos();
            let x2 = center + arc_radius * upward_field_rad.sin();
            let y2 = center - arc_radius * upward_field_rad.cos();

            // Dotted lines for upward field angle
            svg.push_str(&format!(
                r#"<line x1="{}" y1="{}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="1.5" stroke-dasharray="2,3" opacity="0.7"/>"#,
                center, center, x1, y1, pink
            ));
            svg.push_str(&format!(
                r#"<line x1="{}" y1="{}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="1.5" stroke-dasharray="2,3" opacity="0.7"/>"#,
                center, center, x2, y2, pink
            ));

            // Small diamonds at end points
            svg.push_str(&format!(
                r#"<rect x="{:.1}" y="{:.1}" width="6" height="6" fill="{}" opacity="0.7" transform="rotate(45 {} {})"/>"#,
                x1 - 3.0, y1 - 3.0, pink, x1, y1
            ));
            svg.push_str(&format!(
                r#"<rect x="{:.1}" y="{:.1}" width="6" height="6" fill="{}" opacity="0.7" transform="rotate(45 {} {})"/>"#,
                x2 - 3.0, y2 - 3.0, pink, x2, y2
            ));
        }

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

        // Max intensity marker at peak
        if summary.max_intensity > 0.0 {
            // Find approximate peak position (nadir, 0°)
            let _peak_y = center + (summary.max_intensity / scale).min(radius);
            svg.push_str(&format!(
                r#"<text x="{:.1}" y="{:.1}" text-anchor="start" font-size="10" fill="{}" font-family="{}" font-weight="bold">↑ {:.0}</text>"#,
                center + 8.0,
                center + 15.0,
                theme.text,
                theme.font_family,
                summary.max_intensity
            ));
        }

        // === SUMMARY INFO BOX ===
        let box_x = size - 145.0;
        let box_y = 10.0;
        let box_w = 135.0;
        // Calculate box height based on what's shown
        let has_upward = summary.upward_beam_angle > 0.0;
        let extra_lines =
            (if summary.is_batwing { 1 } else { 0 }) + (if has_upward { 2 } else { 0 });
        let box_h = 95.0 + (extra_lines as f64 * 14.0);

        svg.push_str(&format!(
            r#"<rect x="{box_x}" y="{box_y}" width="{box_w}" height="{box_h}" fill="{}" stroke="{}" stroke-width="1" rx="4" opacity="0.95"/>"#,
            theme.legend_bg,
            theme.axis
        ));

        // Summary text
        let text_x = box_x + 8.0;
        let mut text_y = box_y + 16.0;
        let line_height = 14.0;

        svg.push_str(&format!(
            r#"<text x="{text_x}" y="{text_y}" font-size="10" fill="{}" font-family="{}" font-weight="bold">{} {}</text>"#,
            theme.text, theme.font_family, theme.labels.cie_label, summary.cie_flux_codes
        ));
        text_y += line_height;

        // Show IES beam angle (always) - label as "Down" if there's also upward
        let beam_label = if has_upward {
            format!("{} ↓", theme.labels.beam)
        } else {
            theme.labels.beam.clone()
        };
        svg.push_str(&format!(
            r#"<text x="{}" y="{}" font-size="10" fill="{}"><tspan fill="{}">●</tspan> {} {:.0}°</text>"#,
            text_x, text_y, theme.text, green, beam_label, summary.beam_angle
        ));
        text_y += line_height;

        // Show CIE beam angle if batwing (differs from IES)
        if summary.is_batwing {
            svg.push_str(&format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}"><tspan fill="{}">▲</tspan> {} {:.0}° (CIE)</text>"#,
                text_x, text_y, theme.text, blue, theme.labels.beam, summary.beam_angle_cie
            ));
            text_y += line_height;
        }

        // Show upward beam/field angles if present
        if has_upward {
            svg.push_str(&format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}"><tspan fill="{}">●</tspan> {} ↑ {:.0}°</text>"#,
                text_x, text_y, theme.text, purple, theme.labels.beam, summary.upward_beam_angle
            ));
            text_y += line_height;

            svg.push_str(&format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}"><tspan fill="{}">◆</tspan> {} ↑ {:.0}°</text>"#,
                text_x, text_y, theme.text, pink, theme.labels.field, summary.upward_field_angle
            ));
            text_y += line_height;
        }

        svg.push_str(&format!(
            r#"<text x="{}" y="{}" font-size="10" fill="{}"><tspan fill="{}">◆</tspan> {} {:.0}°</text>"#,
            text_x, text_y, theme.text, orange, theme.labels.field, summary.field_angle
        ));
        text_y += line_height;

        svg.push_str(&format!(
            r#"<text x="{text_x}" y="{text_y}" font-size="10" fill="{}" font-family="{}">{} {:.0} lm/W</text>"#,
            theme.text, theme.font_family, theme.labels.efficacy_label, summary.luminaire_efficacy
        ));
        text_y += line_height;

        svg.push_str(&format!(
            r#"<text x="{text_x}" y="{text_y}" font-size="10" fill="{}" font-family="{}">{} {:.1}×{:.1}</text>"#,
            theme.text, theme.font_family, theme.labels.sh_ratio_label, summary.spacing_c0, summary.spacing_c90
        ));

        // Legend (moved down)
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
            r#"<text x="22" y="12" font-size="12" fill="{}" font-family="{}">{}</text>"#,
            theme.text, theme.font_family, theme.labels.plane_c0_c180
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
                r#"<text x="22" y="12" font-size="12" fill="{}" font-family="{}">{}</text>"#,
                theme.text, theme.font_family, theme.labels.plane_c90_c270
            ));
            svg.push_str("</g>");
        }

        // Unit label
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" text-anchor="end" font-size="11" fill="{}" font-family="{}">{}</text>"#,
            size - 15.0,
            size - 15.0,
            theme.text_secondary,
            theme.font_family,
            theme.labels.intensity_unit
        ));

        svg.push_str("</svg>");
        svg
    }

    /// Generate SVG with beam and field angle overlays.
    ///
    /// This creates a Wikipedia-quality illustration showing:
    /// - The luminous intensity distribution curve
    /// - Beam angle (50% threshold) with arc and annotation
    /// - Field angle (10% threshold) with arc and annotation
    /// - Optionally shows both IES (max) and CIE (center-beam) definitions
    ///
    /// This is ideal for educational materials explaining beam angle concepts.
    pub fn to_svg_with_beam_field_angles(
        &self,
        width: f64,
        height: f64,
        theme: &SvgTheme,
        analysis: &crate::calculations::BeamFieldAnalysis,
        show_both_definitions: bool,
    ) -> String {
        let size = width.min(height);
        let center = size / 2.0;
        let margin = 70.0; // Extra margin for labels
        let radius = (size / 2.0) - margin;
        let scale = self.scale.scale_max / radius;

        let mut svg = String::new();

        // SVG header
        svg.push_str(&format!(
            r#"<svg viewBox="0 0 {size} {size}" xmlns="http://www.w3.org/2000/svg" class="diagram-polar-beam-angle">"#
        ));

        // Defs for markers and colors
        svg.push_str(
            r##"<defs>
    <marker id="arrowhead" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">
        <polygon points="0 0, 10 3.5, 0 7" fill="#22c55e"/>
    </marker>
    <marker id="arrowhead-cie" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">
        <polygon points="0 0, 10 3.5, 0 7" fill="#3b82f6"/>
    </marker>
</defs>"##,
        );

        // Background
        svg.push_str(&format!(
            r#"<rect x="0" y="0" width="{size}" height="{size}" fill="{}"/>"#,
            theme.background
        ));

        // Grid circles (fewer for cleaner look)
        for (i, &value) in self.scale.grid_values.iter().enumerate() {
            let r = value / scale;
            let num_circles = self.scale.grid_values.len();
            let is_major = i == num_circles - 1;
            let stroke_color = if is_major { &theme.axis } else { &theme.grid };

            svg.push_str(&format!(
                r#"<circle cx="{center}" cy="{center}" r="{r:.1}" fill="none" stroke="{stroke_color}" stroke-width="1" opacity="0.5"/>"#
            ));
        }

        // Radial lines every 30° (lighter)
        for i in 0..=6 {
            if i == 3 {
                continue;
            }
            let angle_deg = i as f64 * 30.0;
            let angle_rad = angle_deg.to_radians();

            let x_left = center - radius * angle_rad.sin();
            let y_left = center + radius * angle_rad.cos();
            let x_right = center + radius * angle_rad.sin();
            let y_right = center + radius * angle_rad.cos();

            svg.push_str(&format!(
                r#"<line x1="{center}" y1="{center}" x2="{x_left:.1}" y2="{y_left:.1}" stroke="{}" stroke-width="0.5" opacity="0.5"/>"#,
                theme.grid
            ));
            svg.push_str(&format!(
                r#"<line x1="{center}" y1="{center}" x2="{x_right:.1}" y2="{y_right:.1}" stroke="{}" stroke-width="0.5" opacity="0.5"/>"#,
                theme.grid
            ));

            // Angle labels
            if angle_deg > 0.0 && angle_deg <= 90.0 {
                let label_offset = radius + 15.0;
                let label_x_right = center + label_offset * angle_rad.sin();
                let label_y_right = center + label_offset * angle_rad.cos();

                svg.push_str(&format!(
                    r#"<text x="{label_x_right:.1}" y="{label_y_right:.1}" text-anchor="middle" dominant-baseline="middle" font-size="10" fill="{}" font-family="{}">{angle_deg:.0}°</text>"#,
                    theme.text_secondary, theme.font_family
                ));
            }
        }

        // 90° horizontal line
        svg.push_str(&format!(
            r#"<line x1="{:.1}" y1="{center}" x2="{:.1}" y2="{center}" stroke="{}" stroke-width="1"/>"#,
            center - radius,
            center + radius,
            theme.axis
        ));

        // Intensity distribution curve (yellow fill like Wikipedia example)
        let path = self.c0_c180_curve.to_svg_path(center, center, scale);
        let intensity_color = "#eab308";
        if !path.is_empty() {
            svg.push_str(&format!(
                r#"<path d="{path}" fill="rgba(234,179,8,0.4)" stroke="{intensity_color}" stroke-width="2"/>"#
            ));
        }

        // Center point
        svg.push_str(&format!(
            r#"<circle cx="{center}" cy="{center}" r="4" fill="{}"/>"#,
            theme.text
        ));

        // Helper function to draw beam/field angle arc
        let draw_angle_arc = |svg: &mut String,
                              half_angle: f64,
                              color: &str,
                              label: &str,
                              offset: f64,
                              dashed: bool| {
            if half_angle <= 0.0 || half_angle > 90.0 {
                return;
            }

            let arc_radius = radius * 0.85 - offset;
            let angle_rad = half_angle.to_radians();

            // Arc endpoints (symmetric around nadir)
            let x1 = center + arc_radius * angle_rad.sin();
            let y1 = center + arc_radius * angle_rad.cos();
            let x2 = center - arc_radius * angle_rad.sin();
            let y2 = center + arc_radius * angle_rad.cos();

            // Draw arc
            let large_arc = if half_angle > 45.0 { 1 } else { 0 };
            let dash = if dashed {
                r#" stroke-dasharray="6,3""#
            } else {
                ""
            };
            svg.push_str(&format!(
                r#"<path d="M {x1:.1} {y1:.1} A {arc_radius:.1} {arc_radius:.1} 0 {large_arc} 1 {x2:.1} {y2:.1}" fill="none" stroke="{color}" stroke-width="2.5"{dash}/>"#
            ));

            // Draw radial lines to show the angle
            svg.push_str(&format!(
                r#"<line x1="{center}" y1="{center}" x2="{x1:.1}" y2="{y1:.1}" stroke="{color}" stroke-width="1.5"{dash}/>"#
            ));
            svg.push_str(&format!(
                r#"<line x1="{center}" y1="{center}" x2="{x2:.1}" y2="{y2:.1}" stroke="{color}" stroke-width="1.5"{dash}/>"#
            ));

            // Label with angle value
            let label_y = center + arc_radius + 18.0;
            svg.push_str(&format!(
                r#"<text x="{center}" y="{label_y:.1}" text-anchor="middle" font-size="12" font-weight="bold" fill="{color}" font-family="{}">{label} {:.0}°</text>"#,
                theme.font_family, half_angle * 2.0
            ));
        };

        // Draw beam angle (IES - green, solid)
        // Note: analysis angles are now full angles per CIE S 017:2020, need half for drawing
        draw_angle_arc(
            &mut svg,
            analysis.beam_angle_ies / 2.0,
            "#22c55e",
            "Beam (IES):",
            0.0,
            false,
        );

        // Draw field angle (IES - orange, dashed)
        draw_angle_arc(
            &mut svg,
            analysis.field_angle_ies / 2.0,
            "#f97316",
            "Field (IES):",
            30.0,
            true,
        );

        // If showing both definitions and they differ significantly
        if show_both_definitions && analysis.is_batwing {
            // Draw CIE beam angle (blue, solid)
            draw_angle_arc(
                &mut svg,
                analysis.beam_angle_cie / 2.0,
                "#3b82f6",
                "Beam (CIE):",
                60.0,
                false,
            );
        }

        // 50% and 10% threshold circles
        let beam_threshold_r = analysis.beam_threshold_ies / scale;
        let field_threshold_r = analysis.field_threshold_ies / scale;
        let beam_color = "#22c55e";
        let field_color = "#f97316";

        if beam_threshold_r > 0.0 && beam_threshold_r < radius {
            svg.push_str(&format!(
                r#"<circle cx="{center}" cy="{center}" r="{beam_threshold_r:.1}" fill="none" stroke="{beam_color}" stroke-width="1" stroke-dasharray="4,4" opacity="0.7"/>"#
            ));
            let tx = center + beam_threshold_r + 3.0;
            let ty = center - 3.0;
            svg.push_str(&format!(
                r#"<text x="{tx:.1}" y="{ty:.1}" font-size="9" fill="{beam_color}" font-family="{}">50%</text>"#,
                theme.font_family
            ));
        }

        if field_threshold_r > 0.0 && field_threshold_r < radius {
            svg.push_str(&format!(
                r#"<circle cx="{center}" cy="{center}" r="{field_threshold_r:.1}" fill="none" stroke="{field_color}" stroke-width="1" stroke-dasharray="4,4" opacity="0.7"/>"#
            ));
            let tx = center + field_threshold_r + 3.0;
            let ty = center - 3.0;
            svg.push_str(&format!(
                r#"<text x="{tx:.1}" y="{ty:.1}" font-size="9" fill="{field_color}" font-family="{}">10%</text>"#,
                theme.font_family
            ));
        }

        // Title
        svg.push_str(&format!(
            r#"<text x="{center}" y="25" text-anchor="middle" font-size="14" font-weight="bold" fill="{}" font-family="{}">Luminous Intensity Distribution</text>"#,
            theme.text, theme.font_family
        ));

        // Legend box
        let legend_x = 15.0;
        let legend_y = size - 90.0;
        svg.push_str(&format!(
            r#"<rect x="{legend_x}" y="{legend_y}" width="170" height="75" fill="{}" stroke="{}" rx="4"/>"#,
            theme.legend_bg, theme.grid
        ));

        // Legend items
        let cie_color = "#3b82f6";
        let lx1 = legend_x + 8.0;
        let lx2 = legend_x + 28.0;
        let ltx = legend_x + 35.0;

        svg.push_str(&format!(
            r#"<line x1="{lx1}" y1="{}" x2="{lx2}" y2="{}" stroke="{intensity_color}" stroke-width="3"/>"#,
            legend_y + 15.0, legend_y + 15.0
        ));
        svg.push_str(&format!(
            r#"<text x="{ltx}" y="{}" font-size="10" fill="{}" font-family="{}">Intensity (cd/klm)</text>"#,
            legend_y + 18.0, theme.text, theme.font_family
        ));

        svg.push_str(&format!(
            r#"<line x1="{lx1}" y1="{}" x2="{lx2}" y2="{}" stroke="{beam_color}" stroke-width="2.5"/>"#,
            legend_y + 32.0, legend_y + 32.0
        ));
        svg.push_str(&format!(
            r#"<text x="{ltx}" y="{}" font-size="10" fill="{}" font-family="{}">Beam angle (50% I_max)</text>"#,
            legend_y + 35.0, theme.text, theme.font_family
        ));

        svg.push_str(&format!(
            r#"<line x1="{lx1}" y1="{}" x2="{lx2}" y2="{}" stroke="{field_color}" stroke-width="2.5" stroke-dasharray="6,3"/>"#,
            legend_y + 49.0, legend_y + 49.0
        ));
        svg.push_str(&format!(
            r#"<text x="{ltx}" y="{}" font-size="10" fill="{}" font-family="{}">Field angle (10% I_max)</text>"#,
            legend_y + 52.0, theme.text, theme.font_family
        ));

        if show_both_definitions && analysis.is_batwing {
            svg.push_str(&format!(
                r#"<line x1="{lx1}" y1="{}" x2="{lx2}" y2="{}" stroke="{cie_color}" stroke-width="2.5"/>"#,
                legend_y + 66.0, legend_y + 66.0
            ));
            svg.push_str(&format!(
                r#"<text x="{ltx}" y="{}" font-size="10" fill="{}" font-family="{}">Beam angle CIE (50% I_center)</text>"#,
                legend_y + 69.0, theme.text, theme.font_family
            ));
        }

        // Distribution type annotation (for batwing)
        if analysis.is_batwing {
            svg.push_str(&format!(
                r#"<text x="{}" y="{}" text-anchor="end" font-size="10" fill="{}" font-family="{}">Distribution: {}</text>"#,
                size - 15.0, 45.0, theme.text_secondary, theme.font_family, analysis.distribution_type()
            ));
            svg.push_str(&format!(
                r#"<text x="{}" y="{}" text-anchor="end" font-size="10" fill="{}" font-family="{}">I_center/I_max: {:.0}%</text>"#,
                size - 15.0, 58.0, theme.text_secondary, theme.font_family, analysis.center_to_max_ratio() * 100.0
            ));
        }

        // Unit label
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" text-anchor="end" font-size="10" fill="{}" font-family="{}">{}</text>"#,
            size - 15.0,
            size - 15.0,
            theme.text_secondary,
            theme.font_family,
            theme.labels.intensity_unit
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
            r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" font-size="12" fill="{}" font-family="{}">{}</text>"#,
            margin_left + plot_width / 2.0,
            height - 8.0,
            theme.text,
            theme.font_family,
            theme.labels.gamma_axis
        ));

        svg.push_str(&format!(
            r#"<text x="18" y="{:.1}" text-anchor="middle" font-size="12" fill="{}" font-family="{}" transform="rotate(-90, 18, {:.1})">{}</text>"#,
            margin_top + plot_height / 2.0,
            theme.text,
            theme.font_family,
            margin_top + plot_height / 2.0,
            theme.labels.intensity_axis
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
            r#"<text x="{:.1}" y="20" text-anchor="end" font-size="11" fill="{}" font-family="{}">{} {:.0} cd/klm</text>"#,
            width - 15.0,
            theme.text_secondary,
            theme.font_family,
            theme.labels.max_label,
            self.scale.max_intensity
        ));

        svg.push_str("</svg>");
        svg
    }

    /// Generate SVG with beam/field angle markers.
    ///
    /// Adds vertical lines at beam (50%) and field (10%) angles.
    pub fn to_svg_with_summary(
        &self,
        width: f64,
        height: f64,
        theme: &SvgTheme,
        summary: &crate::calculations::PhotometricSummary,
    ) -> String {
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

        // === BEAM ANGLE MARKER (50%) ===
        // Note: beam_angle is now the full angle per CIE S 017:2020
        // For the Cartesian diagram, we plot gamma (half angle) on x-axis
        let green = "#22c55e";
        let orange = "#f97316";

        let half_beam = summary.beam_angle / 2.0;
        if half_beam > 0.0 && half_beam < self.max_gamma {
            let beam_x = margin_left + plot_width * (half_beam / self.max_gamma);
            svg.push_str(&format!(
                r#"<line x1="{beam_x:.1}" y1="{margin_top}" x2="{beam_x:.1}" y2="{:.1}" stroke="{}" stroke-width="2" stroke-dasharray="6,4" opacity="0.8"/>"#,
                margin_top + plot_height,
                green
            ));
            // Display full angle in label per CIE definition
            svg.push_str(&format!(
                r#"<text x="{beam_x:.1}" y="{:.1}" text-anchor="middle" font-size="9" fill="{}" font-weight="bold">{} {:.0}°</text>"#,
                margin_top - 5.0,
                green,
                theme.labels.beam,
                summary.beam_angle
            ));
        }

        // === FIELD ANGLE MARKER (10%) ===
        // Note: field_angle is now the full angle per CIE S 017:2020
        let half_field = summary.field_angle / 2.0;
        if half_field > 0.0 && half_field < self.max_gamma {
            let field_x = margin_left + plot_width * (half_field / self.max_gamma);
            svg.push_str(&format!(
                r#"<line x1="{field_x:.1}" y1="{margin_top}" x2="{field_x:.1}" y2="{:.1}" stroke="{}" stroke-width="2" stroke-dasharray="4,3" opacity="0.7"/>"#,
                margin_top + plot_height,
                orange
            ));
            // Display full angle in label per CIE definition
            svg.push_str(&format!(
                r#"<text x="{field_x:.1}" y="{:.1}" text-anchor="middle" font-size="9" fill="{}" font-weight="bold">{} {:.0}°</text>"#,
                margin_top - 5.0,
                orange,
                theme.labels.field,
                summary.field_angle
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
            r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" font-size="12" fill="{}" font-family="{}">{}</text>"#,
            margin_left + plot_width / 2.0,
            height - 8.0,
            theme.text,
            theme.font_family,
            theme.labels.gamma_axis
        ));

        svg.push_str(&format!(
            r#"<text x="18" y="{:.1}" text-anchor="middle" font-size="12" fill="{}" font-family="{}" transform="rotate(-90, 18, {:.1})">{}</text>"#,
            margin_top + plot_height / 2.0,
            theme.text,
            theme.font_family,
            margin_top + plot_height / 2.0,
            theme.labels.intensity_axis
        ));

        // Legend with beam/field info
        let legend_height = self.curves.len() as f64 * 18.0 + 45.0; // Extra space for beam/field
        svg.push_str(&format!(
            r#"<g transform="translate({:.1}, {:.1})">"#,
            margin_left + 10.0,
            margin_top + 10.0
        ));
        svg.push_str(&format!(
            r#"<rect x="-5" y="-5" width="100" height="{legend_height:.1}" fill="{}" stroke="{}" stroke-width="1" rx="4"/>"#,
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

        // Beam/field angle legend entries
        let base_y = self.curves.len() as f64 * 18.0 + 15.0;
        svg.push_str(&format!(
            r#"<line x1="0" y1="{:.1}" x2="18" y2="{:.1}" stroke="{}" stroke-width="2" stroke-dasharray="6,4"/>"#,
            base_y, base_y, green
        ));
        svg.push_str(&format!(
            r#"<text x="24" y="{:.1}" font-size="10" fill="{}">{}</text>"#,
            base_y + 4.0,
            theme.text,
            theme.labels.beam_50_percent
        ));

        svg.push_str(&format!(
            r#"<line x1="0" y1="{:.1}" x2="18" y2="{:.1}" stroke="{}" stroke-width="2" stroke-dasharray="4,3"/>"#,
            base_y + 16.0, base_y + 16.0, orange
        ));
        svg.push_str(&format!(
            r#"<text x="24" y="{:.1}" font-size="10" fill="{}">{}</text>"#,
            base_y + 20.0,
            theme.text,
            theme.labels.field_10_percent
        ));

        svg.push_str("</g>");

        // Summary info box (top right)
        let info_x = width - 130.0;
        let info_y = margin_top + 10.0;
        svg.push_str(&format!(
            r#"<rect x="{info_x}" y="{info_y}" width="115" height="55" fill="{}" stroke="{}" stroke-width="1" rx="4" opacity="0.95"/>"#,
            theme.legend_bg,
            theme.axis
        ));

        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" font-size="9" fill="{}">{} {}</text>"#,
            info_x + 5.0,
            info_y + 14.0,
            theme.text,
            theme.labels.cie_label,
            summary.cie_flux_codes
        ));
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" font-size="9" fill="{}">{} {:.0} lm/W</text>"#,
            info_x + 5.0,
            info_y + 28.0,
            theme.text,
            theme.labels.efficacy_label,
            summary.luminaire_efficacy
        ));
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" font-size="9" fill="{}">{} {:.0} cd/klm</text>"#,
            info_x + 5.0,
            info_y + 42.0,
            theme.text,
            theme.labels.max_label,
            summary.max_intensity
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
                r#"<svg viewBox="0 0 {width} {height}" xmlns="http://www.w3.org/2000/svg"><rect width="{width}" height="{height}" fill="{}"/><text x="{:.1}" y="{:.1}" text-anchor="middle" fill="{}">{}</text></svg>"#,
                theme.background,
                width / 2.0,
                height / 2.0,
                theme.text_secondary,
                theme.labels.no_data
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
            r#"<text x="{:.1}" y="25" text-anchor="middle" font-size="14" fill="{}" font-weight="600" font-family="{}">{}</text>"#,
            width / 2.0,
            theme.text,
            theme.font_family,
            theme.labels.heatmap_title
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
            r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" font-size="12" fill="{}" font-family="{}">{}</text>"#,
            self.margin_left + self.plot_width / 2.0,
            height - 10.0,
            theme.text,
            theme.font_family,
            theme.labels.c_plane_axis
        ));

        svg.push_str(&format!(
            r#"<text x="18" y="{:.1}" text-anchor="middle" font-size="12" fill="{}" font-family="{}" transform="rotate(-90, 18, {:.1})">{}</text>"#,
            self.margin_top + self.plot_height / 2.0,
            theme.text,
            theme.font_family,
            self.margin_top + self.plot_height / 2.0,
            theme.labels.gamma_angle_axis
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

    /// Generate SVG with zonal lumens breakdown overlay.
    ///
    /// Adds horizontal zone boundary lines and a zonal breakdown panel.
    pub fn to_svg_with_summary(
        &self,
        width: f64,
        height: f64,
        theme: &SvgTheme,
        summary: &crate::calculations::PhotometricSummary,
    ) -> String {
        if self.is_empty() {
            return self.to_svg(width, height, theme);
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
            r#"<text x="{:.1}" y="25" text-anchor="middle" font-size="14" fill="{}" font-weight="600" font-family="{}">{}</text>"#,
            width / 2.0,
            theme.text,
            theme.font_family,
            theme.labels.heatmap_title
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
        let num_c = self.c_angles.len().max(1);
        let num_g = self.g_angles.len().max(1);
        let val_step = ((num_c.max(num_g) as f64 / 8.0).ceil() as usize).max(1);
        let val_font = (self.plot_width / num_c as f64 * val_step as f64 * 0.3)
            .min(self.plot_height / num_g as f64 * val_step as f64 * 0.35)
            .clamp(5.0, 11.0);

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

            if self.show_values
                && cell.c_index % val_step == val_step / 2
                && cell.g_index % val_step == val_step / 2
            {
                let text_color = if cell.normalized < 0.45 { "#ffffff" } else { "#1a1a1a" };
                let cx = cell.x + cell.width * val_step as f64 / 2.0;
                let cy = cell.y + cell.height * val_step as f64 / 2.0;
                svg.push_str(&format!(
                    r#"<text x="{cx:.1}" y="{cy:.1}" fill="{text_color}" font-size="{val_font:.1}" text-anchor="middle" dominant-baseline="central" font-family="monospace">{:.0}</text>"#,
                    cell.intensity
                ));
            }
        }

        // === ZONE BOUNDARY LINES ===
        let zone_angles = [30.0, 60.0, 90.0, 120.0, 150.0];
        let num_g = self.g_angles.len();
        let cell_height = self.plot_height / num_g as f64;

        for &angle in &zone_angles {
            // Find the Y position for this gamma angle
            if let Some(idx) = self.g_angles.iter().position(|&g| (g - angle).abs() < 1.0) {
                let y = self.margin_top + idx as f64 * cell_height;
                let white = "#ffffff";
                let black = "#000000";

                // Draw a dashed line
                svg.push_str(&format!(
                    r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="2" stroke-dasharray="4,2" opacity="0.7"/>"#,
                    self.margin_left, y,
                    self.margin_left + self.plot_width, y,
                    if angle == 90.0 { white } else { black }
                ));

                // Label
                svg.push_str(&format!(
                    r#"<text x="{:.1}" y="{:.1}" font-size="9" fill="{}" font-family="{}" font-weight="bold">{:.0}°</text>"#,
                    self.margin_left + 4.0,
                    y - 3.0,
                    theme.text,
                    theme.font_family,
                    angle
                ));
            }
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
        let step = if num_g <= 10 {
            1
        } else if num_g <= 20 {
            2
        } else {
            5
        };
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
            r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" font-size="12" fill="{}" font-family="{}">{}</text>"#,
            self.margin_left + self.plot_width / 2.0,
            height - 10.0,
            theme.text,
            theme.font_family,
            theme.labels.c_plane_axis
        ));

        svg.push_str(&format!(
            r#"<text x="18" y="{:.1}" text-anchor="middle" font-size="12" fill="{}" font-family="{}" transform="rotate(-90, 18, {:.1})">{}</text>"#,
            self.margin_top + self.plot_height / 2.0,
            theme.text,
            theme.font_family,
            self.margin_top + self.plot_height / 2.0,
            theme.labels.gamma_angle_axis
        ));

        // === ZONAL LUMENS BREAKDOWN PANEL ===
        let panel_x = width - 135.0;
        let panel_y = self.margin_top;
        let panel_w = 125.0;
        let panel_h = 125.0;

        svg.push_str(&format!(
            r#"<rect x="{panel_x}" y="{panel_y}" width="{panel_w}" height="{panel_h}" fill="{}" stroke="{}" stroke-width="1" rx="4" opacity="0.95"/>"#,
            theme.legend_bg,
            theme.axis
        ));

        // Panel title
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" font-size="10" fill="{}" font-family="{}" font-weight="bold">Zonal Lumens</text>"#,
            panel_x + 8.0,
            panel_y + 15.0,
            theme.text,
            theme.font_family
        ));

        // Zonal breakdown
        let zonal = &summary.zonal_lumens;
        let zones = [
            ("0-30°", zonal.zone_0_30),
            ("30-60°", zonal.zone_30_60),
            ("60-90°", zonal.zone_60_90),
            ("90-120°", zonal.zone_90_120),
            ("120-150°", zonal.zone_120_150),
            ("150-180°", zonal.zone_150_180),
        ];

        let bar_x = panel_x + 55.0;
        let bar_w = 60.0;
        let mut y = panel_y + 28.0;
        let line_h = 14.0;

        for (label, value) in zones {
            // Label
            svg.push_str(&format!(
                r#"<text x="{:.1}" y="{:.1}" text-anchor="end" font-size="9" fill="{}" font-family="{}">{}</text>"#,
                bar_x - 5.0,
                y + 3.0,
                theme.text_secondary,
                theme.font_family,
                label
            ));

            // Bar background
            svg.push_str(&format!(
                r#"<rect x="{bar_x}" y="{:.1}" width="{bar_w}" height="8" fill="{}" opacity="0.3" rx="2"/>"#,
                y - 4.0,
                theme.grid
            ));

            // Bar fill (scale to max 100%)
            let fill_w = (value / 100.0).min(1.0) * bar_w;
            let color = if y < panel_y + 70.0 {
                "#22c55e"
            } else {
                "#f97316"
            }; // Green for downward, orange for upward
            svg.push_str(&format!(
                r#"<rect x="{bar_x}" y="{:.1}" width="{:.1}" height="8" fill="{}" rx="2"/>"#,
                y - 4.0,
                fill_w,
                color
            ));

            // Value
            svg.push_str(&format!(
                r#"<text x="{:.1}" y="{:.1}" font-size="8" fill="{}" font-family="{}">{:.0}%</text>"#,
                bar_x + bar_w + 3.0,
                y + 2.0,
                theme.text_secondary,
                theme.font_family,
                value
            ));

            y += line_h;
        }

        // CIE code at bottom
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" font-size="9" fill="{}" font-family="{}">{} {}</text>"#,
            panel_x + 8.0,
            panel_y + panel_h - 8.0,
            theme.text,
            theme.font_family,
            theme.labels.cie_label,
            summary.cie_flux_codes
        ));

        // Max value indicator
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="25" text-anchor="end" font-size="11" fill="{}" font-family="{}">Max: {:.0} cd</text>"#,
            panel_x - 10.0,
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

/// Localized labels for cone diagram
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ConeDiagramLabels {
    /// "Beam Angle" label
    pub beam_angle: String,
    /// "Field Angle" label
    pub field_angle: String,
    /// "Mounting Height" label
    pub mounting_height: String,
    /// "Beam Diameter" label
    pub beam_diameter: String,
    /// "Field Diameter" label
    pub field_diameter: String,
    /// "50%" label
    pub intensity_50: String,
    /// "10%" label
    pub intensity_10: String,
    /// Floor label
    pub floor: String,
    /// Meter unit (m)
    pub meter: String,
    /// C-Plane label (e.g. "C-Plane")
    pub c_plane_label: String,
}

impl Default for ConeDiagramLabels {
    fn default() -> Self {
        Self {
            beam_angle: "Beam Angle".to_string(),
            field_angle: "Field Angle".to_string(),
            mounting_height: "Mounting Height".to_string(),
            beam_diameter: "Beam ⌀".to_string(),
            field_diameter: "Field ⌀".to_string(),
            intensity_50: "50%".to_string(),
            intensity_10: "10%".to_string(),
            floor: "Floor".to_string(),
            meter: "m".to_string(),
            c_plane_label: "C-Plane".to_string(),
        }
    }
}

impl ConeDiagramLabels {
    /// German labels
    pub fn german() -> Self {
        Self {
            beam_angle: "Abstrahlwinkel".to_string(),
            field_angle: "Feldwinkel".to_string(),
            mounting_height: "Montagehöhe".to_string(),
            beam_diameter: "Strahl ⌀".to_string(),
            field_diameter: "Feld ⌀".to_string(),
            intensity_50: "50%".to_string(),
            intensity_10: "10%".to_string(),
            floor: "Boden".to_string(),
            meter: "m".to_string(),
            c_plane_label: "C-Ebene".to_string(),
        }
    }

    /// Chinese labels
    pub fn chinese() -> Self {
        Self {
            beam_angle: "光束角".to_string(),
            field_angle: "照射角".to_string(),
            mounting_height: "安装高度".to_string(),
            beam_diameter: "光束 ⌀".to_string(),
            field_diameter: "照射 ⌀".to_string(),
            intensity_50: "50%".to_string(),
            intensity_10: "10%".to_string(),
            floor: "地面".to_string(),
            meter: "m".to_string(),
            c_plane_label: "C平面".to_string(),
        }
    }

    /// French labels
    pub fn french() -> Self {
        Self {
            beam_angle: "Angle de faisceau".to_string(),
            field_angle: "Angle de champ".to_string(),
            mounting_height: "Hauteur de montage".to_string(),
            beam_diameter: "Faisceau ⌀".to_string(),
            field_diameter: "Champ ⌀".to_string(),
            intensity_50: "50%".to_string(),
            intensity_10: "10%".to_string(),
            floor: "Sol".to_string(),
            meter: "m".to_string(),
            c_plane_label: "Plan C".to_string(),
        }
    }

    /// Italian labels
    pub fn italian() -> Self {
        Self {
            beam_angle: "Angolo del fascio".to_string(),
            field_angle: "Angolo di campo".to_string(),
            mounting_height: "Altezza di montaggio".to_string(),
            beam_diameter: "Fascio ⌀".to_string(),
            field_diameter: "Campo ⌀".to_string(),
            intensity_50: "50%".to_string(),
            intensity_10: "10%".to_string(),
            floor: "Pavimento".to_string(),
            meter: "m".to_string(),
            c_plane_label: "Piano C".to_string(),
        }
    }

    /// Russian labels
    pub fn russian() -> Self {
        Self {
            beam_angle: "Угол луча".to_string(),
            field_angle: "Угол поля".to_string(),
            mounting_height: "Высота монтажа".to_string(),
            beam_diameter: "Луч ⌀".to_string(),
            field_diameter: "Поле ⌀".to_string(),
            intensity_50: "50%".to_string(),
            intensity_10: "10%".to_string(),
            floor: "Пол".to_string(),
            meter: "м".to_string(),
            c_plane_label: "C-плоскость".to_string(),
        }
    }

    /// Spanish labels
    pub fn spanish() -> Self {
        Self {
            beam_angle: "Ángulo del haz".to_string(),
            field_angle: "Ángulo de campo".to_string(),
            mounting_height: "Altura de montaje".to_string(),
            beam_diameter: "Haz ⌀".to_string(),
            field_diameter: "Campo ⌀".to_string(),
            intensity_50: "50%".to_string(),
            intensity_10: "10%".to_string(),
            floor: "Suelo".to_string(),
            meter: "m".to_string(),
            c_plane_label: "Plano C".to_string(),
        }
    }

    /// Portuguese (Brazil) labels
    pub fn portuguese_brazil() -> Self {
        Self {
            beam_angle: "Ângulo do feixe".to_string(),
            field_angle: "Ângulo de campo".to_string(),
            mounting_height: "Altura de montagem".to_string(),
            beam_diameter: "Feixe ⌀".to_string(),
            field_diameter: "Campo ⌀".to_string(),
            intensity_50: "50%".to_string(),
            intensity_10: "10%".to_string(),
            floor: "Piso".to_string(),
            meter: "m".to_string(),
            c_plane_label: "Plano C".to_string(),
        }
    }
}

impl ConeDiagram {
    /// Generate SVG string for the cone diagram
    ///
    /// Shows a side-view of the light cone with beam and field angles,
    /// the luminaire at top, and floor with diameter annotations.
    pub fn to_svg(&self, width: f64, height: f64, theme: &SvgTheme) -> String {
        self.to_svg_with_labels(width, height, theme, &ConeDiagramLabels::default())
    }

    /// Generate SVG with unit system for distance labels.
    pub fn to_svg_with_units(
        &self,
        width: f64,
        height: f64,
        theme: &SvgTheme,
        labels: &ConeDiagramLabels,
        units: UnitSystem,
    ) -> String {
        let mut labels = labels.clone();
        labels.meter = units.distance_label().to_string();
        // Create a copy with converted distance values for display
        let mut converted = self.clone();
        converted.mounting_height = units.convert_meters(self.mounting_height);
        converted.beam_diameter = units.convert_meters(self.beam_diameter);
        converted.field_diameter = units.convert_meters(self.field_diameter);
        converted.to_svg_with_labels(width, height, theme, &labels)
    }

    /// Generate SVG string with custom labels (for i18n)
    pub fn to_svg_with_labels(
        &self,
        width: f64,
        height: f64,
        theme: &SvgTheme,
        labels: &ConeDiagramLabels,
    ) -> String {
        let margin_top = 60.0;
        let margin_bottom = 80.0;
        let margin_side = 60.0;

        let cone_height = height - margin_top - margin_bottom;
        let cone_width = width - 2.0 * margin_side;

        // Center X coordinate
        let cx = width / 2.0;
        // Luminaire Y (top)
        let luminaire_y = margin_top;
        // Floor Y (bottom)
        let floor_y = margin_top + cone_height;

        // Calculate cone spread at floor
        let beam_half_angle = self.beam_angle.to_radians();
        let field_half_angle = self.field_angle.to_radians();

        // Scale factor to fit the cone within available width
        let max_spread = (field_half_angle.tan() * cone_height).max(cone_width / 2.0 * 0.9);
        let scale = (cone_width / 2.0 * 0.85) / max_spread;

        // Calculate X positions at floor level
        let beam_x_offset = beam_half_angle.tan() * cone_height * scale;
        let field_x_offset = field_half_angle.tan() * cone_height * scale;

        let mut svg = String::new();

        // SVG header
        svg.push_str(&format!(
            r#"<svg viewBox="0 0 {width} {height}" xmlns="http://www.w3.org/2000/svg" class="diagram-cone">"#
        ));

        // Background
        svg.push_str(&format!(
            r#"<rect x="0" y="0" width="{width}" height="{height}" fill="{}"/>"#,
            theme.background
        ));

        // Defs for gradients
        svg.push_str(&format!(
            r#"<defs>
    <linearGradient id="beamGradient" x1="50%" y1="0%" x2="50%" y2="100%">
        <stop offset="0%" stop-color="{}" stop-opacity="0.8"/>
        <stop offset="100%" stop-color="{}" stop-opacity="0.3"/>
    </linearGradient>
    <linearGradient id="fieldGradient" x1="50%" y1="0%" x2="50%" y2="100%">
        <stop offset="0%" stop-color="{}" stop-opacity="0.5"/>
        <stop offset="100%" stop-color="{}" stop-opacity="0.15"/>
    </linearGradient>
</defs>"#,
            "#fbbf24",
            "#fbbf24", // Beam: yellow/amber
            "#f97316",
            "#f97316" // Field: orange
        ));

        // Field angle cone (outer, draw first)
        svg.push_str(&format!(
            r#"<path d="M {cx} {luminaire_y} L {:.1} {floor_y} L {:.1} {floor_y} Z" fill="url(#fieldGradient)" stroke="{}" stroke-width="1.5" stroke-dasharray="6,3"/>"#,
            cx - field_x_offset,
            cx + field_x_offset,
            "#f97316"
        ));

        // Beam angle cone (inner, draw on top)
        svg.push_str(&format!(
            r#"<path d="M {cx} {luminaire_y} L {:.1} {floor_y} L {:.1} {floor_y} Z" fill="url(#beamGradient)" stroke="{}" stroke-width="2"/>"#,
            cx - beam_x_offset,
            cx + beam_x_offset,
            "#fbbf24"
        ));

        // Center line (dashed)
        svg.push_str(&format!(
            r#"<line x1="{cx}" y1="{luminaire_y}" x2="{cx}" y2="{floor_y}" stroke="{}" stroke-width="1" stroke-dasharray="4,4"/>"#,
            theme.text_secondary
        ));

        // Luminaire symbol (rectangle at top)
        let lum_width = 40.0;
        let lum_height = 12.0;
        svg.push_str(&format!(
            r#"<rect x="{:.1}" y="{:.1}" width="{lum_width}" height="{lum_height}" fill="{}" stroke="{}" stroke-width="1.5" rx="2"/>"#,
            cx - lum_width / 2.0,
            luminaire_y - lum_height / 2.0,
            theme.surface,
            theme.text
        ));

        // Light source dot
        svg.push_str(&format!(
            r#"<circle cx="{cx}" cy="{luminaire_y}" r="3" fill="{}"/>"#,
            "#fbbf24"
        ));

        // Floor line
        svg.push_str(&format!(
            r#"<line x1="{margin_side}" y1="{floor_y}" x2="{:.1}" y2="{floor_y}" stroke="{}" stroke-width="2"/>"#,
            width - margin_side,
            theme.axis
        ));

        // Floor label
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" text-anchor="end" font-size="11" fill="{}" font-family="{}">{}</text>"#,
            width - margin_side - 5.0,
            floor_y - 5.0,
            theme.text_secondary,
            theme.font_family,
            labels.floor
        ));

        // Height dimension line (left side)
        let dim_x = margin_side - 25.0;
        svg.push_str(&format!(
            r#"<line x1="{dim_x}" y1="{luminaire_y}" x2="{dim_x}" y2="{floor_y}" stroke="{}" stroke-width="1"/>"#,
            theme.text_secondary
        ));
        // Top tick
        svg.push_str(&format!(
            r#"<line x1="{:.1}" y1="{luminaire_y}" x2="{:.1}" y2="{luminaire_y}" stroke="{}" stroke-width="1"/>"#,
            dim_x - 5.0, dim_x + 5.0, theme.text_secondary
        ));
        // Bottom tick
        svg.push_str(&format!(
            r#"<line x1="{:.1}" y1="{floor_y}" x2="{:.1}" y2="{floor_y}" stroke="{}" stroke-width="1"/>"#,
            dim_x - 5.0, dim_x + 5.0, theme.text_secondary
        ));
        // Height label
        svg.push_str(&format!(
            r#"<text x="{dim_x}" y="{:.1}" text-anchor="middle" font-size="11" fill="{}" font-family="{}" transform="rotate(-90, {dim_x}, {:.1})">{:.1}{}</text>"#,
            luminaire_y + cone_height / 2.0,
            theme.text,
            theme.font_family,
            luminaire_y + cone_height / 2.0,
            self.mounting_height,
            labels.meter
        ));

        // Beam diameter dimension (below floor)
        let beam_dim_y = floor_y + 20.0;
        svg.push_str(&format!(
            r#"<line x1="{:.1}" y1="{floor_y}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="1"/>"#,
            cx - beam_x_offset, cx - beam_x_offset, beam_dim_y + 5.0, "#fbbf24"
        ));
        svg.push_str(&format!(
            r#"<line x1="{:.1}" y1="{floor_y}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="1"/>"#,
            cx + beam_x_offset, cx + beam_x_offset, beam_dim_y + 5.0, "#fbbf24"
        ));
        svg.push_str(&format!(
            r#"<line x1="{:.1}" y1="{beam_dim_y}" x2="{:.1}" y2="{beam_dim_y}" stroke="{}" stroke-width="1.5"/>"#,
            cx - beam_x_offset, cx + beam_x_offset, "#fbbf24"
        ));
        // Beam diameter label
        svg.push_str(&format!(
            r#"<text x="{cx}" y="{:.1}" text-anchor="middle" font-size="11" fill="{}" font-family="{}" font-weight="600">{} {:.2}{}</text>"#,
            beam_dim_y - 4.0,
            "#b45309", // Darker amber
            theme.font_family,
            labels.beam_diameter,
            self.beam_diameter,
            labels.meter
        ));

        // Field diameter dimension (further below)
        let field_dim_y = floor_y + 45.0;
        svg.push_str(&format!(
            r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="1"/>"#,
            cx - field_x_offset,
            beam_dim_y + 10.0,
            cx - field_x_offset,
            field_dim_y + 5.0,
            "#f97316"
        ));
        svg.push_str(&format!(
            r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="1"/>"#,
            cx + field_x_offset,
            beam_dim_y + 10.0,
            cx + field_x_offset,
            field_dim_y + 5.0,
            "#f97316"
        ));
        svg.push_str(&format!(
            r#"<line x1="{:.1}" y1="{field_dim_y}" x2="{:.1}" y2="{field_dim_y}" stroke="{}" stroke-width="1.5" stroke-dasharray="4,2"/>"#,
            cx - field_x_offset, cx + field_x_offset, "#f97316"
        ));
        // Field diameter label
        svg.push_str(&format!(
            r#"<text x="{cx}" y="{:.1}" text-anchor="middle" font-size="10" fill="{}" font-family="{}">{} {:.2}{}</text>"#,
            field_dim_y - 4.0,
            "#c2410c", // Darker orange
            theme.font_family,
            labels.field_diameter,
            self.field_diameter,
            labels.meter
        ));

        // Angle annotations (right side)

        // Beam angle arc and label
        let arc_radius = 50.0;
        let beam_arc_end_x = cx + arc_radius * beam_half_angle.sin();
        let beam_arc_end_y = luminaire_y + arc_radius * beam_half_angle.cos();
        svg.push_str(&format!(
            r#"<path d="M {cx} {:.1} A {arc_radius} {arc_radius} 0 0 1 {beam_arc_end_x:.1} {beam_arc_end_y:.1}" fill="none" stroke="{}" stroke-width="1.5"/>"#,
            luminaire_y + arc_radius,
            "#fbbf24"
        ));
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" font-size="11" fill="{}" font-family="{}" font-weight="600">{:.0}° ({})</text>"#,
            cx + arc_radius + 8.0,
            luminaire_y + arc_radius / 2.0 + 4.0,
            "#b45309",
            theme.font_family,
            self.beam_angle,
            labels.intensity_50
        ));

        // Field angle arc and label
        let field_arc_radius = 70.0;
        let field_arc_end_x = cx + field_arc_radius * field_half_angle.sin();
        let field_arc_end_y = luminaire_y + field_arc_radius * field_half_angle.cos();
        svg.push_str(&format!(
            r#"<path d="M {cx} {:.1} A {field_arc_radius} {field_arc_radius} 0 0 1 {field_arc_end_x:.1} {field_arc_end_y:.1}" fill="none" stroke="{}" stroke-width="1" stroke-dasharray="4,2"/>"#,
            luminaire_y + field_arc_radius,
            "#f97316"
        ));
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" font-size="10" fill="{}" font-family="{}">{:.0}° ({})</text>"#,
            cx + field_arc_radius + 8.0,
            luminaire_y + field_arc_radius / 2.0 + 20.0,
            "#c2410c",
            theme.font_family,
            self.field_angle,
            labels.intensity_10
        ));

        // Legend box (top left)
        let legend_x = 15.0;
        let legend_y = 15.0;
        svg.push_str(&format!(
            r#"<rect x="{legend_x}" y="{legend_y}" width="130" height="50" fill="{}" stroke="{}" stroke-width="1" rx="4" opacity="0.95"/>"#,
            theme.legend_bg,
            theme.axis
        ));
        // Beam legend
        svg.push_str(&format!(
            r#"<rect x="{:.1}" y="{:.1}" width="16" height="10" fill="{}" rx="2"/>"#,
            legend_x + 8.0,
            legend_y + 10.0,
            "#fbbf24"
        ));
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" font-size="10" fill="{}" font-family="{}">{} ({})</text>"#,
            legend_x + 30.0,
            legend_y + 18.0,
            theme.text,
            theme.font_family,
            labels.beam_angle,
            labels.intensity_50
        ));
        // Field legend
        svg.push_str(&format!(
            r#"<rect x="{:.1}" y="{:.1}" width="16" height="10" fill="{}" stroke="{}" stroke-dasharray="3,1" rx="2"/>"#,
            legend_x + 8.0, legend_y + 28.0, "#f9731640", "#f97316"
        ));
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" font-size="10" fill="{}" font-family="{}">{} ({})</text>"#,
            legend_x + 30.0,
            legend_y + 36.0,
            theme.text,
            theme.font_family,
            labels.field_angle,
            labels.intensity_10
        ));

        // Beam classification (top right)
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="30" text-anchor="end" font-size="12" fill="{}" font-family="{}" font-weight="600">{}</text>"#,
            width - 15.0,
            theme.text,
            theme.font_family,
            self.beam_classification()
        ));

        // C-plane annotation (below classification, top right)
        if let Some(c) = self.selected_c_plane {
            svg.push_str(&format!(
                r#"<text x="{:.1}" y="48" text-anchor="end" font-size="11" fill="{}" font-family="{}">{} C{:.0}°</text>"#,
                width - 15.0,
                theme.text_secondary,
                theme.font_family,
                labels.c_plane_label,
                c
            ));
        }

        svg.push_str("</svg>");
        svg
    }

    /// Generate a Wikipedia-quality educational SVG explaining beam and field angles.
    ///
    /// This creates a detailed illustration showing:
    /// - Side view of the light cone with beam (50%) and field (10%) angles
    /// - Clear visual distinction between beam and field zones
    /// - Intensity threshold labels
    /// - Mounting height and diameter dimensions
    /// - NEMA beam classification
    ///
    /// Ideal for educational materials and Wikipedia articles.
    pub fn to_svg_wikipedia(&self, width: f64, height: f64, theme: &SvgTheme) -> String {
        let _labels = ConeDiagramLabels::default();
        let margin_top = 70.0;
        let margin_bottom = 100.0;
        let margin_side = 80.0;

        let cone_height = height - margin_top - margin_bottom;
        let cone_width = width - 2.0 * margin_side;

        let cx = width / 2.0;
        let luminaire_y = margin_top;
        let floor_y = margin_top + cone_height;

        let beam_half_angle = self.beam_angle.to_radians();
        let field_half_angle = self.field_angle.to_radians();

        let max_spread = (field_half_angle.tan() * cone_height).max(cone_width / 2.0 * 0.9);
        let scale = (cone_width / 2.0 * 0.85) / max_spread;

        let beam_x_offset = beam_half_angle.tan() * cone_height * scale;
        let field_x_offset = field_half_angle.tan() * cone_height * scale;

        let mut svg = String::new();

        // SVG header
        svg.push_str(&format!(
            r#"<svg viewBox="0 0 {width} {height}" xmlns="http://www.w3.org/2000/svg" class="diagram-cone-wikipedia">"#
        ));

        // Background
        svg.push_str(&format!(
            r#"<rect x="0" y="0" width="{width}" height="{height}" fill="{}"/>"#,
            theme.background
        ));

        // Title
        svg.push_str(&format!(
            r#"<text x="{cx}" y="25" text-anchor="middle" font-size="16" font-weight="bold" fill="{}" font-family="{}">Beam Angle and Field Angle</text>"#,
            theme.text, theme.font_family
        ));
        svg.push_str(&format!(
            r#"<text x="{cx}" y="45" text-anchor="middle" font-size="12" fill="{}" font-family="{}">Light distribution from a luminaire</text>"#,
            theme.text_secondary, theme.font_family
        ));

        // Defs for gradients with Wikipedia-style colors
        let beam_color = "#22c55e"; // Green for beam
        let field_color = "#f97316"; // Orange for field
        let beam_fill = "rgba(34, 197, 94, 0.3)";
        let field_fill = "rgba(249, 115, 22, 0.15)";

        svg.push_str(&format!(
            r#"<defs>
    <linearGradient id="beamGradWiki" x1="50%" y1="0%" x2="50%" y2="100%">
        <stop offset="0%" stop-color="{beam_color}" stop-opacity="0.6"/>
        <stop offset="100%" stop-color="{beam_color}" stop-opacity="0.2"/>
    </linearGradient>
    <linearGradient id="fieldGradWiki" x1="50%" y1="0%" x2="50%" y2="100%">
        <stop offset="0%" stop-color="{field_color}" stop-opacity="0.4"/>
        <stop offset="100%" stop-color="{field_color}" stop-opacity="0.1"/>
    </linearGradient>
</defs>"#
        ));

        // Field angle cone (outer)
        svg.push_str(&format!(
            r#"<path d="M {cx} {luminaire_y} L {:.1} {floor_y} L {:.1} {floor_y} Z" fill="url(#fieldGradWiki)" stroke="{field_color}" stroke-width="2" stroke-dasharray="8,4"/>"#,
            cx - field_x_offset,
            cx + field_x_offset
        ));

        // Beam angle cone (inner)
        svg.push_str(&format!(
            r#"<path d="M {cx} {luminaire_y} L {:.1} {floor_y} L {:.1} {floor_y} Z" fill="url(#beamGradWiki)" stroke="{beam_color}" stroke-width="2.5"/>"#,
            cx - beam_x_offset,
            cx + beam_x_offset
        ));

        // Center beam axis (dashed)
        svg.push_str(&format!(
            r#"<line x1="{cx}" y1="{luminaire_y}" x2="{cx}" y2="{floor_y}" stroke="{}" stroke-width="1.5" stroke-dasharray="6,4"/>"#,
            theme.text_secondary
        ));
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" font-size="10" fill="{}" font-family="{}">Beam axis</text>"#,
            cx + 8.0, luminaire_y + cone_height * 0.6, theme.text_secondary, theme.font_family
        ));

        // Luminaire symbol
        let lum_width = 50.0;
        let lum_height = 14.0;
        svg.push_str(&format!(
            r#"<rect x="{:.1}" y="{:.1}" width="{lum_width}" height="{lum_height}" fill="{}" stroke="{}" stroke-width="2" rx="3"/>"#,
            cx - lum_width / 2.0,
            luminaire_y - lum_height / 2.0,
            theme.surface,
            theme.text
        ));
        svg.push_str(&format!(
            r#"<text x="{cx}" y="{:.1}" text-anchor="middle" font-size="9" fill="{}" font-family="{}">Luminaire</text>"#,
            luminaire_y - lum_height / 2.0 - 5.0, theme.text_secondary, theme.font_family
        ));

        // Light source point
        let light_color = "#fbbf24";
        svg.push_str(&format!(
            r#"<circle cx="{cx}" cy="{luminaire_y}" r="4" fill="{light_color}" stroke="{}" stroke-width="1"/>"#,
            theme.text
        ));

        // Floor/work plane
        svg.push_str(&format!(
            r#"<line x1="{margin_side}" y1="{floor_y}" x2="{:.1}" y2="{floor_y}" stroke="{}" stroke-width="2.5"/>"#,
            width - margin_side,
            theme.axis
        ));
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" font-size="11" fill="{}" font-family="{}">Work plane / Floor</text>"#,
            width - margin_side - 5.0, floor_y - 8.0, theme.text_secondary, theme.font_family
        ));

        // Beam angle annotation with arc
        let arc_r = 60.0;
        let beam_arc_x = cx + arc_r * beam_half_angle.sin();
        let beam_arc_y = luminaire_y + arc_r * beam_half_angle.cos();
        svg.push_str(&format!(
            r#"<path d="M {cx} {:.1} A {arc_r} {arc_r} 0 0 1 {beam_arc_x:.1} {beam_arc_y:.1}" fill="none" stroke="{beam_color}" stroke-width="2.5"/>"#,
            luminaire_y + arc_r
        ));
        // Beam angle arrow and label
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" font-size="13" font-weight="bold" fill="{beam_color}" font-family="{}">Beam angle</text>"#,
            cx + arc_r + 15.0, luminaire_y + arc_r / 2.0 - 5.0, theme.font_family
        ));
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" font-size="12" fill="{beam_color}" font-family="{}">{:.0}° (50% I_max)</text>"#,
            cx + arc_r + 15.0, luminaire_y + arc_r / 2.0 + 12.0, theme.font_family, self.beam_angle
        ));

        // Field angle annotation with arc
        let field_arc_r = 85.0;
        let field_arc_x = cx + field_arc_r * field_half_angle.sin();
        let field_arc_y = luminaire_y + field_arc_r * field_half_angle.cos();
        svg.push_str(&format!(
            r#"<path d="M {cx} {:.1} A {field_arc_r} {field_arc_r} 0 0 1 {field_arc_x:.1} {field_arc_y:.1}" fill="none" stroke="{field_color}" stroke-width="2" stroke-dasharray="6,3"/>"#,
            luminaire_y + field_arc_r
        ));
        // Field angle label
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" font-size="12" font-weight="bold" fill="{field_color}" font-family="{}">Field angle</text>"#,
            cx + field_arc_r + 15.0, luminaire_y + field_arc_r / 2.0 + 30.0, theme.font_family
        ));
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" font-size="11" fill="{field_color}" font-family="{}">{:.0}° (10% I_max)</text>"#,
            cx + field_arc_r + 15.0, luminaire_y + field_arc_r / 2.0 + 45.0, theme.font_family, self.field_angle
        ));

        // Mounting height dimension (left)
        let dim_x = margin_side - 35.0;
        svg.push_str(&format!(
            r#"<line x1="{dim_x}" y1="{luminaire_y}" x2="{dim_x}" y2="{floor_y}" stroke="{}" stroke-width="1"/>"#,
            theme.text_secondary
        ));
        svg.push_str(&format!(
            r#"<line x1="{:.1}" y1="{luminaire_y}" x2="{:.1}" y2="{luminaire_y}" stroke="{}" stroke-width="1"/>"#,
            dim_x - 6.0, dim_x + 6.0, theme.text_secondary
        ));
        svg.push_str(&format!(
            r#"<line x1="{:.1}" y1="{floor_y}" x2="{:.1}" y2="{floor_y}" stroke="{}" stroke-width="1"/>"#,
            dim_x - 6.0, dim_x + 6.0, theme.text_secondary
        ));
        svg.push_str(&format!(
            r#"<text x="{dim_x}" y="{:.1}" text-anchor="middle" font-size="11" fill="{}" font-family="{}" transform="rotate(-90, {dim_x}, {:.1})">Mounting height: {:.1}m</text>"#,
            luminaire_y + cone_height / 2.0,
            theme.text,
            theme.font_family,
            luminaire_y + cone_height / 2.0,
            self.mounting_height
        ));

        // Beam diameter (below floor)
        let beam_dim_y = floor_y + 25.0;
        svg.push_str(&format!(
            r#"<line x1="{:.1}" y1="{floor_y}" x2="{:.1}" y2="{:.1}" stroke="{beam_color}" stroke-width="1"/>"#,
            cx - beam_x_offset, cx - beam_x_offset, beam_dim_y + 5.0
        ));
        svg.push_str(&format!(
            r#"<line x1="{:.1}" y1="{floor_y}" x2="{:.1}" y2="{:.1}" stroke="{beam_color}" stroke-width="1"/>"#,
            cx + beam_x_offset, cx + beam_x_offset, beam_dim_y + 5.0
        ));
        svg.push_str(&format!(
            r#"<line x1="{:.1}" y1="{beam_dim_y}" x2="{:.1}" y2="{beam_dim_y}" stroke="{beam_color}" stroke-width="2"/>"#,
            cx - beam_x_offset, cx + beam_x_offset
        ));
        svg.push_str(&format!(
            r#"<text x="{cx}" y="{:.1}" text-anchor="middle" font-size="11" font-weight="bold" fill="{beam_color}" font-family="{}">Beam ⌀ {:.2}m</text>"#,
            beam_dim_y - 6.0, theme.font_family, self.beam_diameter
        ));

        // Field diameter
        let field_dim_y = floor_y + 55.0;
        svg.push_str(&format!(
            r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{field_color}" stroke-width="1"/>"#,
            cx - field_x_offset, beam_dim_y + 10.0, cx - field_x_offset, field_dim_y + 5.0
        ));
        svg.push_str(&format!(
            r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{field_color}" stroke-width="1"/>"#,
            cx + field_x_offset, beam_dim_y + 10.0, cx + field_x_offset, field_dim_y + 5.0
        ));
        svg.push_str(&format!(
            r#"<line x1="{:.1}" y1="{field_dim_y}" x2="{:.1}" y2="{field_dim_y}" stroke="{field_color}" stroke-width="1.5" stroke-dasharray="6,3"/>"#,
            cx - field_x_offset, cx + field_x_offset
        ));
        svg.push_str(&format!(
            r#"<text x="{cx}" y="{:.1}" text-anchor="middle" font-size="10" fill="{field_color}" font-family="{}">Field ⌀ {:.2}m</text>"#,
            field_dim_y - 6.0, theme.font_family, self.field_diameter
        ));

        // Legend box with definitions
        let legend_x = 15.0;
        let legend_y = 60.0;
        svg.push_str(&format!(
            r#"<rect x="{legend_x}" y="{legend_y}" width="180" height="78" fill="{}" stroke="{}" stroke-width="1" rx="4"/>"#,
            theme.legend_bg, theme.grid
        ));

        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" font-size="11" font-weight="bold" fill="{}" font-family="{}">Definitions (IES):</text>"#,
            legend_x + 8.0, legend_y + 16.0, theme.text, theme.font_family
        ));

        // Beam legend
        svg.push_str(&format!(
            r#"<rect x="{:.1}" y="{:.1}" width="14" height="10" fill="{beam_fill}" stroke="{beam_color}" stroke-width="1.5" rx="2"/>"#,
            legend_x + 8.0, legend_y + 26.0
        ));
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" font-size="10" fill="{}" font-family="{}">Beam: I ≥ 50% of I_max</text>"#,
            legend_x + 28.0, legend_y + 34.0, theme.text, theme.font_family
        ));

        // Field legend
        svg.push_str(&format!(
            r#"<rect x="{:.1}" y="{:.1}" width="14" height="10" fill="{field_fill}" stroke="{field_color}" stroke-width="1.5" stroke-dasharray="3,1" rx="2"/>"#,
            legend_x + 8.0, legend_y + 44.0
        ));
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" font-size="10" fill="{}" font-family="{}">Field: I ≥ 10% of I_max</text>"#,
            legend_x + 28.0, legend_y + 52.0, theme.text, theme.font_family
        ));

        // Classification
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" font-size="10" fill="{}" font-family="{}">Classification: {}</text>"#,
            legend_x + 8.0, legend_y + 70.0, theme.text, theme.font_family, self.beam_classification()
        ));

        // Formula note at bottom
        svg.push_str(&format!(
            r#"<text x="{cx}" y="{:.1}" text-anchor="middle" font-size="9" fill="{}" font-family="{}">Beam diameter = 2 × height × tan(beam_angle / 2)</text>"#,
            height - 12.0, theme.text_secondary, theme.font_family
        ));

        svg.push_str("</svg>");
        svg
    }
}

impl IsoluxDiagram {
    /// Generate complete SVG string for the isolux ground footprint diagram
    pub fn to_svg(&self, width: f64, height: f64, theme: &SvgTheme) -> String {
        self.to_svg_with_units(width, height, theme, UnitSystem::default())
    }

    /// Generate SVG with unit system for labels (lx/fc, m/ft).
    pub fn to_svg_with_units(
        &self,
        width: f64,
        height: f64,
        theme: &SvgTheme,
        units: UnitSystem,
    ) -> String {
        let margin_left = self.margin_left;
        let margin_top = self.margin_top;
        let plot_width = self.plot_width;
        let plot_height = self.plot_height;

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
        let dist_label = units.distance_label();
        let h_display = units.convert_meters(self.params.mounting_height);
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="22" text-anchor="middle" font-size="14" font-weight="bold" fill="{}" font-family="{}">Isolux Footprint (h={h_display:.1}{dist_label}, tilt={:.0}°)</text>"#,
            width / 2.0,
            theme.text,
            theme.font_family,
            self.params.tilt_angle
        ));

        // Heatmap cells
        for cell in &self.cells {
            svg.push_str(&format!(
                r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="{}"/>"#,
                cell.sx,
                cell.sy,
                cell.width,
                cell.height,
                cell.color.to_rgb_string()
            ));
        }

        // Contour lines overlay
        let contour_colors = [
            "rgba(255,255,255,0.9)",
            "rgba(255,255,255,0.85)",
            "rgba(255,255,255,0.8)",
            "rgba(255,255,255,0.75)",
            "rgba(255,255,255,0.7)",
            "rgba(255,255,255,0.65)",
            "rgba(255,255,255,0.6)",
            "rgba(255,255,255,0.55)",
            "rgba(255,255,255,0.5)",
        ];

        for (i, contour) in self.contours.iter().enumerate() {
            let color = contour_colors.get(i).unwrap_or(&"rgba(255,255,255,0.6)");
            for path in &contour.paths {
                svg.push_str(&format!(
                    r#"<path d="{}" fill="none" stroke="{}" stroke-width="1.5"/>"#,
                    path, color
                ));
            }
            // Label at the first path's start point
            if let Some(first_path) = contour.paths.first() {
                if let Some(coords) = first_path.strip_prefix("M ") {
                    if let Some(space_idx) = coords.find(' ') {
                        let x_str = &coords[..space_idx];
                        if let Ok(x) = x_str.parse::<f64>() {
                            let y_str = coords[space_idx + 1..].split(' ').next().unwrap_or("0");
                            if let Ok(y) = y_str.parse::<f64>() {
                                svg.push_str(&format!(
                                    r#"<text x="{:.1}" y="{:.1}" font-size="9" fill="white" font-family="{}" font-weight="bold" paint-order="stroke" stroke="{}" stroke-width="2">{}</text>"#,
                                    x + 3.0, y - 3.0,
                                    theme.font_family,
                                    "rgba(0,0,0,0.5)",
                                    contour.label
                                ));
                            }
                        }
                    }
                }
            }
        }

        // Luminaire marker at center
        let cx = margin_left + plot_width / 2.0;
        let cy = margin_top + plot_height / 2.0;
        svg.push_str(&format!(
            r#"<circle cx="{cx:.1}" cy="{cy:.1}" r="4" fill="white" stroke="black" stroke-width="1.5"/>"#
        ));

        // Axis labels (in meters or feet)
        let hw = self.params.area_half_width;
        let hd = self.params.area_half_depth;
        let x_label_positions = [-1.0, -0.5, 0.0, 0.5, 1.0];
        for &frac in &x_label_positions {
            let x = margin_left + plot_width * ((frac + 1.0) / 2.0);
            let val = units.convert_meters(hw * frac);
            svg.push_str(&format!(
                r#"<text x="{x:.1}" y="{:.1}" text-anchor="middle" font-size="10" fill="{}" font-family="{}">{val:.0}{dist_label}</text>"#,
                margin_top + plot_height + 16.0,
                theme.text_secondary,
                theme.font_family,
            ));
        }
        for &frac in &x_label_positions {
            let y = margin_top + plot_height * ((1.0 - frac) / 2.0);
            let val = units.convert_meters(hd * frac);
            svg.push_str(&format!(
                r#"<text x="{:.1}" y="{y:.1}" text-anchor="end" dominant-baseline="middle" font-size="10" fill="{}" font-family="{}">{val:.0}{dist_label}</text>"#,
                margin_left - 6.0,
                theme.text_secondary,
                theme.font_family,
            ));
        }

        // Axis titles
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" font-size="12" fill="{}" font-family="{}">X ({dist_label})</text>"#,
            margin_left + plot_width / 2.0,
            height - 6.0,
            theme.text,
            theme.font_family
        ));
        svg.push_str(&format!(
            r#"<text x="14" y="{:.1}" text-anchor="middle" font-size="12" fill="{}" font-family="{}" transform="rotate(-90, 14, {:.1})">Y ({dist_label})</text>"#,
            margin_top + plot_height / 2.0,
            theme.text,
            theme.font_family,
            margin_top + plot_height / 2.0
        ));

        // Color legend
        let legend_x = margin_left + plot_width + 10.0;
        let legend_h = plot_height;
        let num_segments = 50;
        let seg_h = legend_h / num_segments as f64;
        for i in 0..num_segments {
            let normalized = 1.0 - i as f64 / num_segments as f64;
            let color = super::color::heatmap_color(normalized);
            let y = margin_top + i as f64 * seg_h;
            svg.push_str(&format!(
                r#"<rect x="{legend_x:.1}" y="{y:.1}" width="15" height="{seg_h:.1}" fill="{}"/>"#,
                color.to_rgb_string()
            ));
        }
        // Legend labels
        let illu_label = units.illuminance_label();
        let legend_labels = [
            (
                0.0,
                format!("{} {illu_label}", fmt_lux(units.convert_lux(self.max_lux))),
            ),
            (
                0.5,
                format!("{} {illu_label}", fmt_lux(units.convert_lux(self.max_lux * 0.5))),
            ),
            (1.0, format!("0 {illu_label}")),
        ];
        for &(frac, ref label) in &legend_labels {
            let y = margin_top + frac * legend_h;
            svg.push_str(&format!(
                r#"<text x="{:.1}" y="{y:.1}" font-size="9" fill="{}" font-family="{}" dominant-baseline="middle">{}</text>"#,
                legend_x + 20.0,
                theme.text_secondary,
                theme.font_family,
                label
            ));
        }

        // Border
        svg.push_str(&format!(
            r#"<rect x="{margin_left}" y="{margin_top}" width="{plot_width}" height="{plot_height}" fill="none" stroke="{}" stroke-width="1"/>"#,
            theme.axis
        ));

        // Max lux annotation
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" text-anchor="end" font-size="10" fill="{}" font-family="{}">E_max = {} {illu_label}</text>"#,
            margin_left + plot_width,
            margin_top + plot_height + 38.0,
            theme.text_secondary,
            theme.font_family,
            fmt_lux(units.convert_lux(self.max_lux))
        ));

        svg.push_str("</svg>");
        svg
    }

    /// Generate SVG with AEC-style discrete colored contour bands.
    ///
    /// Instead of a continuous heatmap, renders filled contour zones in distinct
    /// colors (red → orange → yellow → green → cyan → purple) with a stepped
    /// legend showing percentage and lux values — matching common AEC photometric
    /// data sheets.
    pub fn to_svg_aec(
        &self,
        width: f64,
        height: f64,
        theme: &SvgTheme,
        units: UnitSystem,
    ) -> String {
        let margin_left = self.margin_left;
        let margin_top = self.margin_top;
        let plot_width = self.plot_width;
        let plot_height = self.plot_height;

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

        // AEC contour band definitions: (percentage of max, color, label)
        let bands: &[(f64, &str, &str)] = &[
            (0.50, "rgba(220,50,50,0.55)",   "50.0 %"),
            (0.30, "rgba(240,140,40,0.55)",   "30.0 %"),
            (0.10, "rgba(240,220,60,0.55)",   "10.0 %"),
            (0.06, "rgba(80,200,80,0.55)",    "6.0 %"),
            (0.03, "rgba(80,180,220,0.55)",   "3.0 %"),
            (0.01, "rgba(160,80,200,0.45)",   "1.0 %"),
        ];

        // Build lux grid from cells
        let n = self.params.grid_resolution;
        let hw = self.params.area_half_width;
        let hd = self.params.area_half_depth;
        let mut lux_grid: Vec<Vec<f64>> = vec![vec![0.0; n]; n];
        for cell in &self.cells {
            let col = ((cell.x_m + hw) / (2.0 * hw) * n as f64) as usize;
            let row = ((cell.y_m + hd) / (2.0 * hd) * n as f64) as usize;
            if col < n && row < n {
                lux_grid[row][col] = cell.lux;
            }
        }

        // Light gray background for the plot area
        svg.push_str(&format!(
            r#"<rect x="{margin_left}" y="{margin_top}" width="{plot_width}" height="{plot_height}" fill="{}"/>"#,
            theme.surface
        ));

        // Draw filled contour bands (from lowest threshold to highest so higher zones paint on top)
        let cell_w = plot_width / n as f64;
        let cell_h = plot_height / n as f64;

        for &(frac, color, _) in bands.iter().rev() {
            let threshold = self.max_lux * frac;
            for row in 0..n {
                for col in 0..n {
                    if lux_grid[row][col] >= threshold {
                        let x = margin_left + col as f64 * cell_w;
                        let y = margin_top + row as f64 * cell_h;
                        svg.push_str(&format!(
                            r#"<rect x="{x:.1}" y="{y:.1}" width="{:.1}" height="{:.1}" fill="{color}"/>"#,
                            cell_w + 0.5, cell_h + 0.5,
                        ));
                    }
                }
            }
        }

        // Contour lines at each threshold
        for &(frac, color, _) in bands {
            let threshold = self.max_lux * frac;
            // Use the same contour paths from the diagram if available, or draw simple boundary
            for contour in &self.contours {
                if (contour.lux_value - threshold).abs() < self.max_lux * 0.05 {
                    // Strip alpha from band color for the stroke
                    let stroke_color = color.replace("0.55", "0.9").replace("0.45", "0.9");
                    for path in &contour.paths {
                        svg.push_str(&format!(
                            r#"<path d="{}" fill="none" stroke="{stroke_color}" stroke-width="1.5"/>"#,
                            path
                        ));
                    }
                }
            }
        }

        // Grid lines
        let grid_steps = 5;
        let grid_color = &theme.grid;
        for i in 0..=grid_steps {
            let frac = i as f64 / grid_steps as f64;
            let x = margin_left + frac * plot_width;
            let y = margin_top + frac * plot_height;
            svg.push_str(&format!(
                r#"<line x1="{x:.1}" y1="{margin_top}" x2="{x:.1}" y2="{:.1}" stroke="{grid_color}" stroke-width="0.5" stroke-dasharray="2,2"/>"#,
                margin_top + plot_height
            ));
            svg.push_str(&format!(
                r#"<line x1="{margin_left}" y1="{y:.1}" x2="{:.1}" y2="{y:.1}" stroke="{grid_color}" stroke-width="0.5" stroke-dasharray="2,2"/>"#,
                margin_left + plot_width
            ));
        }

        // Luminaire marker at center
        let cx = margin_left + plot_width / 2.0;
        let cy = margin_top + plot_height / 2.0;
        svg.push_str(&format!(
            r#"<circle cx="{cx:.1}" cy="{cy:.1}" r="5" fill="rgba(255,220,60,0.8)" stroke="{}" stroke-width="1.5"/>"#,
            theme.text
        ));

        // Axis labels
        let dist_label = units.distance_label();
        let x_label_positions = [-1.0, -0.5, 0.0, 0.5, 1.0];
        for &frac in &x_label_positions {
            let x = margin_left + plot_width * ((frac + 1.0) / 2.0);
            let val = units.convert_meters(hw * frac);
            svg.push_str(&format!(
                r#"<text x="{x:.1}" y="{:.1}" text-anchor="middle" font-size="9" fill="{}" font-family="{}">{val:.0}</text>"#,
                margin_top + plot_height + 14.0,
                theme.text_secondary,
                theme.font_family,
            ));
        }
        for &frac in &x_label_positions {
            let y = margin_top + plot_height * ((1.0 - frac) / 2.0);
            let val = units.convert_meters(hd * frac);
            svg.push_str(&format!(
                r#"<text x="{:.1}" y="{y:.1}" text-anchor="end" dominant-baseline="middle" font-size="9" fill="{}" font-family="{}">{val:.0}</text>"#,
                margin_left - 4.0,
                theme.text_secondary,
                theme.font_family,
            ));
        }
        // Unit label on X axis
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" text-anchor="end" font-size="9" fill="{}" font-family="{}">{dist_label}</text>"#,
            margin_left + plot_width + 2.0,
            margin_top + plot_height + 14.0,
            theme.text_secondary,
            theme.font_family,
        ));

        // ── Color legend with percentage and lux values ──
        let illu_label = units.illuminance_label();
        let legend_x = margin_left + plot_width + 12.0;
        let swatch_size = 14.0;
        let legend_line_h = 18.0;

        for (i, &(frac, color, pct_label)) in bands.iter().enumerate() {
            let y = margin_top + i as f64 * legend_line_h;
            // Color swatch
            svg.push_str(&format!(
                r#"<rect x="{legend_x:.1}" y="{y:.1}" width="{swatch_size}" height="{swatch_size}" fill="{color}" stroke="{}" stroke-width="0.5" rx="2"/>"#,
                theme.text_secondary
            ));
            // Percentage label
            svg.push_str(&format!(
                r#"<text x="{:.1}" y="{:.1}" font-size="9" fill="{}" font-family="{}">{pct_label}</text>"#,
                legend_x + swatch_size + 4.0,
                y + swatch_size - 3.0,
                theme.text_secondary,
                theme.font_family,
            ));
            // Lux value
            let lux_val = fmt_lux(units.convert_lux(self.max_lux * frac));
            svg.push_str(&format!(
                r#"<text x="{:.1}" y="{:.1}" font-size="9" fill="{}" font-family="{}">{lux_val} {illu_label}</text>"#,
                legend_x + swatch_size + 46.0,
                y + swatch_size - 3.0,
                theme.text,
                theme.font_family,
            ));
        }

        // Info line
        let h_display = units.convert_meters(self.params.mounting_height);
        svg.push_str(&format!(
            r#"<text x="{margin_left}" y="{:.1}" font-size="9" fill="{}" font-family="{}">Peak illuminance: {} {illu_label}</text>"#,
            margin_top + plot_height + 28.0,
            theme.text_secondary,
            theme.font_family,
            fmt_lux(units.convert_lux(self.max_lux)),
        ));
        svg.push_str(&format!(
            r#"<text x="{margin_left}" y="{:.1}" font-size="9" fill="{}" font-family="{}">Mounting height: {h_display:.1} {dist_label}</text>"#,
            margin_top + plot_height + 40.0,
            theme.text_secondary,
            theme.font_family,
        ));
        svg.push_str(&format!(
            r#"<text x="{margin_left}" y="{:.1}" font-size="9" fill="{}" font-family="{}">Number of c-planes: {}</text>"#,
            margin_top + plot_height + 52.0,
            theme.text_secondary,
            theme.font_family,
            self.contours.len() + 1,
        ));

        // Border
        svg.push_str(&format!(
            r#"<rect x="{margin_left}" y="{margin_top}" width="{plot_width}" height="{plot_height}" fill="none" stroke="{}" stroke-width="1"/>"#,
            theme.axis
        ));

        svg.push_str("</svg>");
        svg
    }

    /// Generate an isometric 3D isolux diagram matching the AEC "ISO view" style.
    ///
    /// Renders the ground plane in isometric projection with:
    /// - Grayscale gradient cells (brightest at center)
    /// - Grid lines on the ground plane
    /// - Iso-illuminance contour lines
    /// - A small red polar diagram on a vertical pole at the luminaire position
    ///
    /// `ldt` is needed to draw the mini polar diagram on the pole.
    pub fn to_svg_isometric(
        &self,
        width: f64,
        height: f64,
        theme: &SvgTheme,
        config: &IsometricConfig,
        units: UnitSystem,
        ldt: &crate::Eulumdat,
        title: &str,
    ) -> String {
        let mut svg = String::new();

        // Isometric projection: rotate 45° around Z, then tilt ~30° toward viewer
        let iso_angle = std::f64::consts::FRAC_PI_6; // 30°
        let cos_a = iso_angle.cos();
        let sin_a = iso_angle.sin();

        let n = self.params.grid_resolution;
        let hw = self.params.area_half_width;
        let hd = self.params.area_half_depth;

        let max_screen_x = (hw + hd) * cos_a;
        let max_screen_y = (hw + hd) * sin_a;

        let available_w = width - 2.0 * config.margin_x - config.legend_width;
        let available_h = height - config.margin_top - config.margin_bottom;

        // Use separate X/Y scales so the ground plane fills the rectangular viewport
        let pole_px = config.pole_height_px;
        let sx = available_w / (2.0 * max_screen_x);
        let sy = (available_h - pole_px) / (2.0 * max_screen_y);

        let cx = config.margin_x + available_w / 2.0;
        let cy = config.margin_top + pole_px + (available_h - pole_px) * 0.5;

        // Isometric projection with non-uniform scaling to fill rectangle
        let iso = |wx: f64, wy: f64| -> (f64, f64) {
            let screen_x = cx + (wx - wy) * cos_a * sx;
            let screen_y = cy + (wx + wy) * sin_a * sy;
            (screen_x, screen_y)
        };

        // SVG header
        svg.push_str(&format!(
            r#"<svg viewBox="0 0 {width} {height}" xmlns="http://www.w3.org/2000/svg">"#
        ));

        // Background — white/light
        svg.push_str(&format!(
            r#"<rect x="0" y="0" width="{width}" height="{height}" fill="{}"/>"#,
            theme.background
        ));

        // Title
        svg.push_str(&format!(
            r#"<text x="20" y="28" font-size="{}" font-weight="bold" fill="{}" font-family="{}">{}</text>"#,
            config.title_font_size, theme.text, theme.font_family, title
        ));

        // Build the lux grid (already computed in self.cells, but we need the 2D array)
        let mut lux_grid: Vec<Vec<f64>> = vec![vec![0.0; n]; n];
        for cell in &self.cells {
            let col = ((cell.x_m + hw) / (2.0 * hw) * n as f64) as usize;
            let row = ((cell.y_m + hd) / (2.0 * hd) * n as f64) as usize;
            if col < n && row < n {
                lux_grid[row][col] = cell.lux;
            }
        }

        // Draw ground plane cells as isometric quadrilaterals (grayscale gradient)
        let dx = 2.0 * hw / n as f64;
        let dy = 2.0 * hd / n as f64;

        for row in 0..n {
            for col in 0..n {
                let wx0 = -hw + col as f64 * dx;
                let wy0 = -hd + row as f64 * dy;
                let wx1 = wx0 + dx;
                let wy1 = wy0 + dy;

                let (x0, y0) = iso(wx0, wy0);
                let (x1, y1) = iso(wx1, wy0);
                let (x2, y2) = iso(wx1, wy1);
                let (x3, y3) = iso(wx0, wy1);

                let lux = lux_grid[row][col];
                let norm = if self.max_lux > 0.0 { lux / self.max_lux } else { 0.0 };
                // Grayscale: bright center (white) → dark edges (gray)
                let gray = (255.0 * (1.0 - norm * config.grayscale_max_darken)) as u8;
                let fill = format!("rgb({gray},{gray},{gray})");

                svg.push_str(&format!(
                    r#"<polygon points="{x0:.1},{y0:.1} {x1:.1},{y1:.1} {x2:.1},{y2:.1} {x3:.1},{y3:.1}" fill="{fill}" stroke="none"/>"#,
                ));
            }
        }

        // Grid lines on the ground plane
        let grid_step = if n > 20 { n / 10 } else { 2 };
        let grid_color = &config.grid_color;
        let grid_sw = config.grid_stroke_width;
        // Lines along X (constant Y)
        for row_i in (0..=n).step_by(grid_step) {
            let wy = -hd + row_i as f64 * dy;
            let (x0, y0) = iso(-hw, wy);
            let (x1, y1) = iso(hw, wy);
            svg.push_str(&format!(
                r#"<line x1="{x0:.1}" y1="{y0:.1}" x2="{x1:.1}" y2="{y1:.1}" stroke="{grid_color}" stroke-width="{grid_sw}"/>"#,
            ));
        }
        // Lines along Y (constant X)
        for col_i in (0..=n).step_by(grid_step) {
            let wx = -hw + col_i as f64 * dx;
            let (x0, y0) = iso(wx, -hd);
            let (x1, y1) = iso(wx, hd);
            svg.push_str(&format!(
                r#"<line x1="{x0:.1}" y1="{y0:.1}" x2="{x1:.1}" y2="{y1:.1}" stroke="{grid_color}" stroke-width="{grid_sw}"/>"#,
            ));
        }

        // Contour lines in isometric projection
        // Re-trace contour paths: for each contour level, sample lux_grid and
        // draw isometric ellipses approximating the iso-illuminance curves.
        let contour_levels: Vec<f64> = if self.max_lux > 0.0 {
            config.contour_fractions.iter().map(|f| self.max_lux * f).collect()
        } else {
            vec![]
        };

        for &level in &contour_levels {
            // Trace the contour by scanning radially from center at many angles
            let num_angles = config.contour_num_angles;
            let mut contour_pts: Vec<(f64, f64)> = Vec::new();
            for ai in 0..num_angles {
                let angle = ai as f64 / num_angles as f64 * std::f64::consts::TAU;
                let dir_x = angle.cos();
                let dir_y = angle.sin();

                // March outward along this ray to find where lux crosses level
                let max_r = (hw * hw + hd * hd).sqrt();
                let steps = config.contour_march_steps;
                let mut found_r = max_r; // default to edge if never found
                for si in 1..=steps {
                    let r = max_r * si as f64 / steps as f64;
                    let wx = dir_x * r;
                    let wy = dir_y * r;
                    if wx.abs() > hw || wy.abs() > hd {
                        break;
                    }
                    let col = ((wx + hw) / (2.0 * hw) * (n - 1) as f64) as usize;
                    let row = ((wy + hd) / (2.0 * hd) * (n - 1) as f64) as usize;
                    if col < n && row < n && lux_grid[row][col] < level {
                        // Interpolate between this step and previous
                        let prev_r = max_r * (si - 1) as f64 / steps as f64;
                        found_r = (prev_r + r) / 2.0;
                        break;
                    }
                }
                let wx = dir_x * found_r;
                let wy = dir_y * found_r;
                if wx.abs() <= hw && wy.abs() <= hd {
                    contour_pts.push(iso(wx, wy));
                }
            }

            if contour_pts.len() > 3 {
                let norm = level / self.max_lux;
                // Darker lines for higher lux (inner contours)
                let gray = (config.contour_gray_min + config.contour_gray_range * (1.0 - norm)) as u8;
                let stroke = format!("rgb({gray},{gray},{gray})");
                let sw = config.contour_stroke_width_min + config.contour_stroke_width_range * norm;

                let mut path = String::from("M ");
                for (i, &(px, py)) in contour_pts.iter().enumerate() {
                    if i > 0 { path.push_str(" L "); }
                    path.push_str(&format!("{px:.1} {py:.1}"));
                }
                path.push_str(" Z");

                svg.push_str(&format!(
                    r#"<path d="{path}" fill="none" stroke="{stroke}" stroke-width="{sw:.1}"/>"#,
                ));
            }
        }

        // ── Distance labels along ground plane edges ────────────────
        let dist_label = units.distance_label();
        let illu_label = units.illuminance_label();
        let label_positions = [-1.0, -0.5, 0.0, 0.5, 1.0];

        // Labels along the front-left edge (constant Y = +hd, varying X)
        for &frac in &label_positions {
            let wx = hw * frac;
            let wy = hd; // front edge
            let (sx, sy) = iso(wx, wy);
            let val = units.convert_meters(wx);
            svg.push_str(&format!(
                r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" font-size="{}" fill="{}" font-family="{}">{val:.0}{dist_label}</text>"#,
                sx, sy + 14.0,
                config.label_font_size,
                theme.text_secondary,
                theme.font_family,
            ));
        }

        // Labels along the front-right edge (constant X = +hw, varying Y)
        for &frac in &label_positions {
            let wx = hw; // right edge
            let wy = hd * frac;
            let (sx, sy) = iso(wx, wy);
            let val = units.convert_meters(wy);
            svg.push_str(&format!(
                r#"<text x="{:.1}" y="{:.1}" text-anchor="start" font-size="{}" fill="{}" font-family="{}">{val:.0}{dist_label}</text>"#,
                sx + 6.0, sy + 4.0,
                config.label_font_size,
                theme.text_secondary,
                theme.font_family,
            ));
        }

        // ── Contour illuminance labels ───────────────────────────────
        for &level in &contour_levels {
            // Place label at the rightmost point of each contour
            let angle = 0.0_f64; // rightward (along +X axis)
            let dir_x = angle.cos();
            let dir_y = angle.sin();
            let max_r = (hw * hw + hd * hd).sqrt();
            let steps = config.contour_march_steps;
            let mut label_r = 0.0;
            for si in 1..=steps {
                let r = max_r * si as f64 / steps as f64;
                let wx = dir_x * r;
                let wy = dir_y * r;
                if wx.abs() > hw || wy.abs() > hd { break; }
                let col = ((wx + hw) / (2.0 * hw) * (n - 1) as f64) as usize;
                let row = ((wy + hd) / (2.0 * hd) * (n - 1) as f64) as usize;
                if col < n && row < n && lux_grid[row][col] < level {
                    label_r = (max_r * (si - 1) as f64 / steps as f64 + r) / 2.0;
                    break;
                }
            }
            if label_r > 0.0 {
                let (lx, ly) = iso(dir_x * label_r, dir_y * label_r);
                let display_val = fmt_lux(units.convert_lux(level));
                svg.push_str(&format!(
                    r#"<text x="{:.1}" y="{:.1}" font-size="{}" fill="{}" font-family="{}" font-weight="bold" paint-order="stroke" stroke="{}" stroke-width="2.5">{display_val} {illu_label}</text>"#,
                    lx + 4.0, ly - 2.0,
                    config.contour_label_font_size,
                    theme.text,
                    theme.font_family,
                    theme.background,
                ));
            }
        }

        // ── Illuminance legend (grayscale gradient bar) ──────────────
        let legend_x = width - config.legend_width + 5.0;
        let legend_y_top = config.margin_top + 10.0;
        let legend_h = available_h * config.legend_height_fraction;
        let legend_bar_w = config.legend_bar_width;
        let num_segments = config.legend_num_segments;
        let seg_h = legend_h / num_segments as f64;

        for i in 0..num_segments {
            let frac = i as f64 / num_segments as f64;
            // Top = max lux (dark), bottom = 0 (white)
            let gray = (255.0 * (1.0 - (1.0 - frac) * config.grayscale_max_darken)) as u8;
            let y = legend_y_top + i as f64 * seg_h;
            svg.push_str(&format!(
                r#"<rect x="{legend_x:.1}" y="{y:.1}" width="{legend_bar_w}" height="{seg_h:.1}" fill="rgb({gray},{gray},{gray})"/>"#,
            ));
        }
        // Legend border
        svg.push_str(&format!(
            r#"<rect x="{legend_x:.1}" y="{legend_y_top:.1}" width="{legend_bar_w}" height="{legend_h}" fill="none" stroke="{}" stroke-width="0.5"/>"#,
            theme.text_secondary
        ));
        // Legend labels: top (max), middle, bottom (0)
        let legend_labels = [
            (0.0, fmt_lux(units.convert_lux(self.max_lux))),
            (0.5, fmt_lux(units.convert_lux(self.max_lux * 0.5))),
            (1.0, "0".to_string()),
        ];
        for &(frac, ref label) in &legend_labels {
            let y = legend_y_top + frac * legend_h;
            svg.push_str(&format!(
                r#"<text x="{:.1}" y="{:.1}" text-anchor="end" font-size="{}" fill="{}" font-family="{}" dominant-baseline="middle">{}</text>"#,
                legend_x - 4.0, y,
                config.label_font_size,
                theme.text_secondary,
                theme.font_family,
                label
            ));
        }
        // Legend unit label
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" font-size="{}" fill="{}" font-family="{}">{illu_label}</text>"#,
            legend_x + legend_bar_w / 2.0,
            legend_y_top - 6.0,
            config.label_font_size,
            theme.text_secondary,
            theme.font_family,
        ));

        // ── Vertical pole at center ──────────────────────────────────
        let (pole_bx, pole_by) = iso(0.0, 0.0);
        let pole_top_y = pole_by - pole_px;
        svg.push_str(&format!(
            "<line x1=\"{pole_bx:.1}\" y1=\"{pole_by:.1}\" x2=\"{pole_bx:.1}\" y2=\"{pole_top_y:.1}\" stroke=\"{}\" stroke-width=\"{}\"/>",
            config.pole_color, config.pole_stroke_width,
        ));

        // ── Mini polar diagram at pole top ───────────────────────────
        let polar_radius = config.polar_radius;
        let polar_cx = pole_bx;
        let polar_cy = pole_top_y;

        let max_i = ldt.max_intensity().max(1.0);
        let mut polar_path = String::new();

        let g_angles = &ldt.g_angles;
        if !g_angles.is_empty() && !ldt.intensities.is_empty() {
            let c0_idx = 0;
            let intensities_c0 = &ldt.intensities[c0_idx.min(ldt.intensities.len() - 1)];

            let c180_idx = ldt.c_angles.iter().position(|&a| (a - 180.0).abs() < 1.0)
                .unwrap_or(0);
            let intensities_c180 = &ldt.intensities[c180_idx.min(ldt.intensities.len() - 1)];

            let mut pts_right: Vec<(f64, f64)> = Vec::new();
            for (gi, &gamma) in g_angles.iter().enumerate() {
                let intensity = if gi < intensities_c0.len() { intensities_c0[gi] } else { 0.0 };
                let r = intensity / max_i * polar_radius;
                let angle_rad = (gamma - 90.0).to_radians();
                let px = polar_cx + r * angle_rad.cos();
                let py = polar_cy - r * angle_rad.sin();
                pts_right.push((px, py));
            }

            let mut pts_left: Vec<(f64, f64)> = Vec::new();
            for (gi, &gamma) in g_angles.iter().enumerate() {
                let intensity = if gi < intensities_c180.len() { intensities_c180[gi] } else { 0.0 };
                let r = intensity / max_i * polar_radius;
                let angle_rad = (gamma - 90.0).to_radians();
                let px = polar_cx - r * angle_rad.cos();
                let py = polar_cy - r * angle_rad.sin();
                pts_left.push((px, py));
            }

            if !pts_right.is_empty() {
                polar_path.push_str(&format!("M {:.1} {:.1}", pts_right[0].0, pts_right[0].1));
                for &(px, py) in &pts_right[1..] {
                    polar_path.push_str(&format!(" L {px:.1} {py:.1}"));
                }
                for &(px, py) in pts_left.iter().rev() {
                    polar_path.push_str(&format!(" L {px:.1} {py:.1}"));
                }
                polar_path.push_str(" Z");
            }
        }

        if !polar_path.is_empty() {
            svg.push_str(&format!(
                r#"<path d="{polar_path}" fill="{}" stroke="{}" stroke-width="{}"/>"#,
                config.polar_fill, config.polar_stroke, config.polar_stroke_width,
            ));
        }

        // ── Info line: mounting height + E_max ───────────────────────
        let h_display = units.convert_meters(self.params.mounting_height);
        svg.push_str(&format!(
            r#"<text x="20" y="{:.1}" font-size="{}" fill="{}" font-family="{}">h = {h_display:.1}{dist_label}  |  E_max = {} {illu_label}</text>"#,
            height - 8.0,
            config.info_font_size,
            theme.text_secondary,
            theme.font_family,
            fmt_lux(units.convert_lux(self.max_lux))
        ));

        svg.push_str("</svg>");
        svg
    }
}

impl IsocandelaDiagram {
    /// Generate complete SVG string for the isocandela contour plot
    pub fn to_svg(&self, width: f64, height: f64, theme: &SvgTheme) -> String {
        let margin_left = self.margin_left;
        let margin_top = self.margin_top;
        let plot_width = self.plot_width;
        let plot_height = self.plot_height;

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
            r#"<text x="{:.1}" y="22" text-anchor="middle" font-size="14" font-weight="bold" fill="{}" font-family="{}">Isocandela Contour Plot</text>"#,
            width / 2.0,
            theme.text,
            theme.font_family
        ));

        // Heatmap cells
        for cell in &self.cells {
            svg.push_str(&format!(
                r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="{}"/>"#,
                cell.sx,
                cell.sy,
                cell.width,
                cell.height,
                cell.color.to_rgb_string()
            ));
        }

        // Contour lines
        let contour_colors = [
            "rgba(255,255,255,0.95)",
            "rgba(255,255,255,0.85)",
            "rgba(255,255,255,0.8)",
            "rgba(255,255,255,0.7)",
            "rgba(255,255,255,0.6)",
        ];

        for (i, contour) in self.contours.iter().enumerate() {
            let color = contour_colors.get(i).unwrap_or(&"rgba(255,255,255,0.6)");
            for path in &contour.paths {
                svg.push_str(&format!(
                    r#"<path d="{}" fill="none" stroke="{}" stroke-width="1.5"/>"#,
                    path, color
                ));
            }
            // Label
            if let Some(first_path) = contour.paths.first() {
                if let Some(coords) = first_path.strip_prefix("M ") {
                    if let Some(space_idx) = coords.find(' ') {
                        let x_str = &coords[..space_idx];
                        if let Ok(x) = x_str.parse::<f64>() {
                            let y_str = coords[space_idx + 1..].split(' ').next().unwrap_or("0");
                            if let Ok(y) = y_str.parse::<f64>() {
                                svg.push_str(&format!(
                                    r#"<text x="{:.1}" y="{:.1}" font-size="9" fill="white" font-family="{}" font-weight="bold" paint-order="stroke" stroke="{}" stroke-width="2">{}</text>"#,
                                    x + 3.0, y - 3.0,
                                    theme.font_family,
                                    "rgba(0,0,0,0.5)",
                                    contour.label
                                ));
                            }
                        }
                    }
                }
            }
        }

        // Axis grid lines and labels
        let h_ticks = [-90.0, -60.0, -30.0, 0.0, 30.0, 60.0, 90.0];
        let v_ticks = [-90.0, -60.0, -30.0, 0.0, 30.0, 60.0, 90.0];

        for &h in &h_ticks {
            let x = margin_left + plot_width * ((h - self.h_min) / (self.h_max - self.h_min));
            svg.push_str(&format!(
                r#"<line x1="{x:.1}" y1="{margin_top}" x2="{x:.1}" y2="{:.1}" stroke="rgba(255,255,255,0.2)" stroke-width="0.5"/>"#,
                margin_top + plot_height
            ));
            svg.push_str(&format!(
                r#"<text x="{x:.1}" y="{:.1}" text-anchor="middle" font-size="10" fill="{}" font-family="{}">{h:.0}°</text>"#,
                margin_top + plot_height + 16.0,
                theme.text_secondary,
                theme.font_family
            ));
        }

        for &v in &v_ticks {
            let y = margin_top + plot_height * ((self.v_max - v) / (self.v_max - self.v_min));
            svg.push_str(&format!(
                r#"<line x1="{margin_left}" y1="{y:.1}" x2="{:.1}" y2="{y:.1}" stroke="rgba(255,255,255,0.2)" stroke-width="0.5"/>"#,
                margin_left + plot_width
            ));
            svg.push_str(&format!(
                r#"<text x="{:.1}" y="{y:.1}" text-anchor="end" dominant-baseline="middle" font-size="10" fill="{}" font-family="{}">{v:.0}°</text>"#,
                margin_left - 6.0,
                theme.text_secondary,
                theme.font_family
            ));
        }

        // Axis titles
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" font-size="12" fill="{}" font-family="{}">Horizontal Angle H (°)</text>"#,
            margin_left + plot_width / 2.0,
            height - 6.0,
            theme.text,
            theme.font_family
        ));
        svg.push_str(&format!(
            r#"<text x="14" y="{:.1}" text-anchor="middle" font-size="12" fill="{}" font-family="{}" transform="rotate(-90, 14, {:.1})">Vertical Angle V (°)</text>"#,
            margin_top + plot_height / 2.0,
            theme.text,
            theme.font_family,
            margin_top + plot_height / 2.0
        ));

        // Color legend
        let legend_x = margin_left + plot_width + 10.0;
        let legend_h = plot_height;
        let num_segments = 50;
        let seg_h = legend_h / num_segments as f64;
        for i in 0..num_segments {
            let normalized = 1.0 - i as f64 / num_segments as f64;
            let color = super::color::heatmap_color(normalized);
            let y = margin_top + i as f64 * seg_h;
            svg.push_str(&format!(
                r#"<rect x="{legend_x:.1}" y="{y:.1}" width="15" height="{seg_h:.1}" fill="{}"/>"#,
                color.to_rgb_string()
            ));
        }
        // Legend labels
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" font-size="9" fill="{}" font-family="{}" dominant-baseline="middle">{:.0}</text>"#,
            legend_x + 20.0, margin_top,
            theme.text_secondary, theme.font_family,
            self.i_max
        ));
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" font-size="9" fill="{}" font-family="{}" dominant-baseline="middle">cd/klm</text>"#,
            legend_x + 20.0, margin_top + 12.0,
            theme.text_secondary, theme.font_family
        ));
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" font-size="9" fill="{}" font-family="{}" dominant-baseline="middle">0</text>"#,
            legend_x + 20.0, margin_top + legend_h,
            theme.text_secondary, theme.font_family
        ));

        // Border
        svg.push_str(&format!(
            r#"<rect x="{margin_left}" y="{margin_top}" width="{plot_width}" height="{plot_height}" fill="none" stroke="{}" stroke-width="1"/>"#,
            theme.axis
        ));

        // I_max annotation
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" text-anchor="end" font-size="10" fill="{}" font-family="{}">I_max = {:.0} cd/klm</text>"#,
            margin_left + plot_width,
            margin_top + plot_height + 38.0,
            theme.text_secondary,
            theme.font_family,
            self.i_max
        ));

        svg.push_str("</svg>");
        svg
    }
}

impl FloodlightCartesianDiagram {
    /// Generate complete SVG string for the floodlight V-H Cartesian diagram
    pub fn to_svg(&self, width: f64, height: f64, theme: &SvgTheme) -> String {
        let margin_left = self.margin_left;
        let margin_top = self.margin_top;
        let plot_width = self.plot_width;
        let plot_height = self.plot_height;

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
            theme.surface, theme.axis
        ));

        // Y-axis grid lines and labels
        for tick in &self.y_ticks {
            let y = self.map_y_tick(*tick, margin_top, plot_height);
            svg.push_str(&format!(
                r#"<line x1="{margin_left}" y1="{y:.1}" x2="{:.1}" y2="{y:.1}" stroke="{}" stroke-width="0.5" stroke-dasharray="4,3"/>"#,
                margin_left + plot_width,
                theme.grid
            ));
            let label = match self.y_scale {
                YScale::Logarithmic => {
                    if *tick >= 1.0 {
                        format!("{:.0}", tick)
                    } else {
                        format!("{:.1}", tick)
                    }
                }
                YScale::Linear => format!("{:.0}", tick),
            };
            svg.push_str(&format!(
                r#"<text x="{:.1}" y="{y:.1}" text-anchor="end" dominant-baseline="middle" font-size="11" fill="{}" font-family="{}">{label}</text>"#,
                margin_left - 8.0,
                theme.text_secondary,
                theme.font_family
            ));
        }

        // X-axis grid lines and labels
        for &angle in &self.x_ticks {
            let x = margin_left + plot_width * ((angle + 90.0) / 180.0);
            svg.push_str(&format!(
                r#"<line x1="{x:.1}" y1="{margin_top}" x2="{x:.1}" y2="{:.1}" stroke="{}" stroke-width="0.5" stroke-dasharray="4,3"/>"#,
                margin_top + plot_height,
                theme.grid
            ));
            svg.push_str(&format!(
                r#"<text x="{x:.1}" y="{:.1}" text-anchor="middle" font-size="11" fill="{}" font-family="{}">{angle:.0}°</text>"#,
                margin_top + plot_height + 18.0,
                theme.text_secondary,
                theme.font_family
            ));
        }

        // Zero axis emphasis (at angle=0)
        let x_zero = margin_left + plot_width * 0.5;
        svg.push_str(&format!(
            r#"<line x1="{x_zero:.1}" y1="{margin_top}" x2="{x_zero:.1}" y2="{:.1}" stroke="{}" stroke-width="1" opacity="0.5"/>"#,
            margin_top + plot_height,
            theme.axis
        ));

        // H-plane curve
        let h_path = self.h_curve.to_svg_path();
        svg.push_str(&format!(
            r#"<path d="{}" fill="none" stroke="{}" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/>"#,
            h_path,
            self.h_curve.color.to_rgb_string()
        ));

        // V-plane curve
        let v_path = self.v_curve.to_svg_path();
        svg.push_str(&format!(
            r#"<path d="{}" fill="none" stroke="{}" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/>"#,
            v_path,
            self.v_curve.color.to_rgb_string()
        ));

        // Axis labels
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" font-size="12" fill="{}" font-family="{}">Angle (°)</text>"#,
            margin_left + plot_width / 2.0,
            height - 8.0,
            theme.text,
            theme.font_family
        ));

        let y_label = match self.y_scale {
            YScale::Linear => "Intensity (cd/klm)",
            YScale::Logarithmic => "Intensity (cd/klm) — log",
        };
        svg.push_str(&format!(
            r#"<text x="18" y="{:.1}" text-anchor="middle" font-size="12" fill="{}" font-family="{}" transform="rotate(-90, 18, {:.1})">{}</text>"#,
            margin_top + plot_height / 2.0,
            theme.text,
            theme.font_family,
            margin_top + plot_height / 2.0,
            y_label
        ));

        // Title
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="20" text-anchor="middle" font-size="14" font-weight="bold" fill="{}" font-family="{}">Floodlight V-H Diagram</text>"#,
            width / 2.0,
            theme.text,
            theme.font_family
        ));

        // Legend
        let legend_x = margin_left + plot_width - 130.0;
        let legend_y = margin_top + 10.0;
        svg.push_str(&format!(
            r#"<g transform="translate({legend_x:.1}, {legend_y:.1})">"#
        ));
        svg.push_str(&format!(
            r#"<rect x="-5" y="-5" width="130" height="50" fill="{}" stroke="{}" stroke-width="1" rx="4" opacity="0.9"/>"#,
            theme.legend_bg, theme.axis
        ));
        // H-plane legend
        svg.push_str(&format!(
            r#"<line x1="0" y1="8" x2="18" y2="8" stroke="{}" stroke-width="2.5"/>"#,
            self.h_curve.color.to_rgb_string()
        ));
        svg.push_str(&format!(
            r#"<text x="24" y="12" font-size="11" fill="{}" font-family="{}">{}</text>"#,
            theme.text, theme.font_family, self.h_curve.label
        ));
        // V-plane legend
        svg.push_str(&format!(
            r#"<line x1="0" y1="28" x2="18" y2="28" stroke="{}" stroke-width="2.5"/>"#,
            self.v_curve.color.to_rgb_string()
        ));
        svg.push_str(&format!(
            r#"<text x="24" y="32" font-size="11" fill="{}" font-family="{}">{}</text>"#,
            theme.text, theme.font_family, self.v_curve.label
        ));
        svg.push_str("</g>");

        // Max intensity annotation
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" text-anchor="end" font-size="10" fill="{}" font-family="{}">I_max = {:.0} cd/klm</text>"#,
            margin_left + plot_width - 5.0,
            margin_top + plot_height + 38.0,
            theme.text_secondary,
            theme.font_family,
            self.i_max
        ));

        svg.push_str("</svg>");
        svg
    }

    /// Map a Y tick value to screen coordinate
    fn map_y_tick(&self, value: f64, margin_top: f64, plot_height: f64) -> f64 {
        match self.y_scale {
            YScale::Linear => {
                let y_max = self.scale.scale_max;
                if y_max > 0.0 {
                    margin_top + plot_height * (1.0 - value / y_max)
                } else {
                    margin_top + plot_height
                }
            }
            YScale::Logarithmic => {
                let y_max = self.scale.scale_max;
                let y_min = self.y_ticks.first().copied().unwrap_or(0.1).max(0.1);
                let log_range = y_max.log10() - y_min.log10();
                if log_range > 0.0 {
                    let normalized = (value.max(y_min).log10() - y_min.log10()) / log_range;
                    margin_top + plot_height * (1.0 - normalized)
                } else {
                    margin_top + plot_height
                }
            }
        }
    }
}

// ============================================================================
// Overlay SVG methods for comparison
// ============================================================================

impl PolarDiagram {
    /// Render two polar diagrams overlaid on a single SVG for comparison.
    ///
    /// File A uses the theme's default colors (blue/red), File B uses green/orange.
    /// A 4-entry legend identifies all curves.
    pub fn to_overlay_svg(
        a: &PolarDiagram,
        b: &PolarDiagram,
        width: f64,
        height: f64,
        theme: &SvgTheme,
        label_a: &str,
        label_b: &str,
    ) -> String {
        let size = width.min(height);
        let center = size / 2.0;
        let margin = 60.0;
        let radius = (size / 2.0) - margin;

        // Unified scale from maximum of both diagrams
        let max_val = a.scale.scale_max.max(b.scale.scale_max);
        let unified_scale = DiagramScale::from_max_intensity(max_val, 5);
        let scale = unified_scale.scale_max / radius;

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
        for (i, &value) in unified_scale.grid_values.iter().enumerate() {
            let r = value / scale;
            let is_major = i == unified_scale.grid_values.len() - 1;
            let stroke_color = if is_major { &theme.axis } else { &theme.grid };
            let stroke_width = if is_major { "1.5" } else { "1" };

            svg.push_str(&format!(
                r#"<circle cx="{center}" cy="{center}" r="{r:.1}" fill="none" stroke="{stroke_color}" stroke-width="{stroke_width}"/>"#
            ));

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
                continue;
            }
            let angle_deg = i as f64 * 30.0;
            let angle_rad = angle_deg.to_radians();
            let x_left = center - radius * angle_rad.sin();
            let y_left = center + radius * angle_rad.cos();
            let x_right = center + radius * angle_rad.sin();
            let y_right = center + radius * angle_rad.cos();

            svg.push_str(&format!(
                r#"<line x1="{center}" y1="{center}" x2="{x_left:.1}" y2="{y_left:.1}" stroke="{}" stroke-width="1"/>"#,
                theme.grid
            ));
            svg.push_str(&format!(
                r#"<line x1="{center}" y1="{center}" x2="{x_right:.1}" y2="{y_right:.1}" stroke="{}" stroke-width="1"/>"#,
                theme.grid
            ));
        }

        // 90° horizontal axis
        svg.push_str(&format!(
            r#"<line x1="{:.1}" y1="{center}" x2="{:.1}" y2="{center}" stroke="{}" stroke-width="1.5"/>"#,
            center - radius,
            center + radius,
            theme.axis
        ));

        // Colors: A = theme defaults (blue/red), B = green/orange
        let color_a_c0 = &theme.curve_c0_c180;
        let fill_a_c0 = &theme.curve_c0_c180_fill;
        let color_a_c90 = &theme.curve_c90_c270;
        let color_b_c0 = "#22c55e"; // green
        let fill_b_c0 = "rgba(34,197,94,0.12)";
        let color_b_c90 = "#f97316"; // orange

        // File A - C0-C180
        let path = a.c0_c180_curve.to_svg_path(center, center, scale);
        if !path.is_empty() {
            svg.push_str(&format!(
                r#"<path d="{}" fill="{}" stroke="{}" stroke-width="2.5"/>"#,
                path, fill_a_c0, color_a_c0
            ));
        }
        // File A - C90-C270
        if a.show_c90_c270() {
            let path = a.c90_c270_curve.to_svg_path(center, center, scale);
            if !path.is_empty() {
                svg.push_str(&format!(
                    r#"<path d="{}" fill="none" stroke="{}" stroke-width="2.5" stroke-dasharray="6,4"/>"#,
                    path, color_a_c90
                ));
            }
        }

        // File B - C0-C180
        let path = b.c0_c180_curve.to_svg_path(center, center, scale);
        if !path.is_empty() {
            svg.push_str(&format!(
                r#"<path d="{}" fill="{}" stroke="{}" stroke-width="2.5"/>"#,
                path, fill_b_c0, color_b_c0
            ));
        }
        // File B - C90-C270
        if b.show_c90_c270() {
            let path = b.c90_c270_curve.to_svg_path(center, center, scale);
            if !path.is_empty() {
                svg.push_str(&format!(
                    r#"<path d="{}" fill="none" stroke="{}" stroke-width="2.5" stroke-dasharray="6,4"/>"#,
                    path, color_b_c90
                ));
            }
        }

        // Center point
        svg.push_str(&format!(
            r#"<circle cx="{center}" cy="{center}" r="3" fill="{}"/>"#,
            theme.text
        ));

        // Legend — dynamic entries based on which curves are present
        let a_has_c90 = a.show_c90_c270();
        let b_has_c90 = b.show_c90_c270();
        let entry_count = 1 + a_has_c90 as usize + 1 + b_has_c90 as usize;
        let legend_height = entry_count as f64 * 18.0 + 10.0;
        let legend_y = size - legend_height - 15.0;
        svg.push_str(&format!(r#"<g transform="translate(15, {legend_y:.1})">"#));
        svg.push_str(&format!(
            r#"<rect x="-5" y="-5" width="170" height="{legend_height:.1}" fill="{}" stroke="{}" stroke-width="1" rx="4"/>"#,
            theme.legend_bg, theme.axis
        ));
        let mut row = 0;
        // A primary curve
        let y = row as f64 * 18.0 + 8.0;
        svg.push_str(&format!(
            r#"<line x1="0" y1="{y:.1}" x2="18" y2="{y:.1}" stroke="{}" stroke-width="2.5"/>"#,
            color_a_c0
        ));
        svg.push_str(&format!(
            r#"<text x="24" y="{:.1}" font-size="11" fill="{}" font-family="{}">{} {}</text>"#,
            y + 4.0,
            theme.text,
            theme.font_family,
            label_a,
            a.c0_c180_curve.label
        ));
        row += 1;
        // A secondary curve
        if a_has_c90 {
            let y = row as f64 * 18.0 + 8.0;
            svg.push_str(&format!(
                r#"<line x1="0" y1="{y:.1}" x2="18" y2="{y:.1}" stroke="{}" stroke-width="2.5" stroke-dasharray="4,2"/>"#,
                color_a_c90
            ));
            svg.push_str(&format!(
                r#"<text x="24" y="{:.1}" font-size="11" fill="{}" font-family="{}">{} {}</text>"#,
                y + 4.0,
                theme.text,
                theme.font_family,
                label_a,
                a.c90_c270_curve.label
            ));
            row += 1;
        }
        // B primary curve
        let y = row as f64 * 18.0 + 8.0;
        svg.push_str(&format!(
            r#"<line x1="0" y1="{y:.1}" x2="18" y2="{y:.1}" stroke="{}" stroke-width="2.5"/>"#,
            color_b_c0
        ));
        svg.push_str(&format!(
            r#"<text x="24" y="{:.1}" font-size="11" fill="{}" font-family="{}">{} {}</text>"#,
            y + 4.0,
            theme.text,
            theme.font_family,
            label_b,
            b.c0_c180_curve.label
        ));
        row += 1;
        // B secondary curve
        if b_has_c90 {
            let y = row as f64 * 18.0 + 8.0;
            svg.push_str(&format!(
                r#"<line x1="0" y1="{y:.1}" x2="18" y2="{y:.1}" stroke="{}" stroke-width="2.5" stroke-dasharray="4,2"/>"#,
                color_b_c90
            ));
            svg.push_str(&format!(
                r#"<text x="24" y="{:.1}" font-size="11" fill="{}" font-family="{}">{} {}</text>"#,
                y + 4.0,
                theme.text,
                theme.font_family,
                label_b,
                b.c90_c270_curve.label
            ));
        }
        svg.push_str("</g>");

        // Unit label
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" text-anchor="end" font-size="11" fill="{}" font-family="{}">{}</text>"#,
            size - 15.0,
            size - 15.0,
            theme.text_secondary,
            theme.font_family,
            theme.labels.intensity_unit
        ));

        svg.push_str("</svg>");
        svg
    }
}

impl CartesianDiagram {
    /// Render two cartesian diagrams overlaid on a single SVG for comparison.
    ///
    /// File A curves are solid, File B curves are dashed. Unified Y-axis scale.
    pub fn to_overlay_svg(
        a: &CartesianDiagram,
        b: &CartesianDiagram,
        width: f64,
        height: f64,
        theme: &SvgTheme,
        label_a: &str,
        label_b: &str,
    ) -> String {
        let margin_left = a.margin_left;
        let margin_top = a.margin_top;
        let plot_width = a.plot_width;
        let plot_height = a.plot_height;

        // Unified Y-axis from max of both
        let y_max = a.scale.scale_max.max(b.scale.scale_max);
        let max_gamma = a.max_gamma.max(b.max_gamma);

        let mut svg = String::new();

        svg.push_str(&format!(
            r#"<svg viewBox="0 0 {width} {height}" xmlns="http://www.w3.org/2000/svg">"#
        ));

        // Background
        svg.push_str(&format!(
            r#"<rect x="0" y="0" width="{width}" height="{height}" fill="{}"/>"#,
            theme.background
        ));

        // Plot area
        svg.push_str(&format!(
            r#"<rect x="{margin_left}" y="{margin_top}" width="{plot_width}" height="{plot_height}" fill="{}" stroke="{}" stroke-width="1"/>"#,
            theme.surface, theme.axis
        ));

        // Y-axis grid lines (use the larger scale)
        let unified_y = DiagramScale::from_max_intensity(y_max, 5);
        for &v in &unified_y.grid_values {
            let y = margin_top + plot_height * (1.0 - v / unified_y.scale_max);
            svg.push_str(&format!(
                r#"<line x1="{margin_left}" y1="{y:.1}" x2="{:.1}" y2="{y:.1}" stroke="{}" stroke-width="1"/>"#,
                margin_left + plot_width, theme.grid
            ));
            svg.push_str(&format!(
                r#"<text x="{:.1}" y="{y:.1}" text-anchor="end" dominant-baseline="middle" font-size="11" fill="{}" font-family="{}">{v:.0}</text>"#,
                margin_left - 8.0, theme.text_secondary, theme.font_family
            ));
        }

        // X-axis grid
        for &v in &a.x_ticks {
            let x = margin_left + plot_width * (v / max_gamma);
            svg.push_str(&format!(
                r#"<line x1="{x:.1}" y1="{margin_top}" x2="{x:.1}" y2="{:.1}" stroke="{}" stroke-width="1"/>"#,
                margin_top + plot_height, theme.grid
            ));
            svg.push_str(&format!(
                r#"<text x="{x:.1}" y="{:.1}" text-anchor="middle" font-size="11" fill="{}" font-family="{}">{v:.0}°</text>"#,
                margin_top + plot_height + 18.0, theme.text_secondary, theme.font_family
            ));
        }

        // Colors for B: green tones
        let b_colors = [
            "#22c55e", "#f97316", "#a855f7", "#06b6d4", "#eab308", "#ec4899", "#84cc16", "#6366f1",
        ];

        // File A curves (solid)
        for curve in &a.curves {
            let path = Self::rescale_curve_path(
                curve,
                margin_left,
                margin_top,
                plot_width,
                plot_height,
                unified_y.scale_max,
                max_gamma,
            );
            svg.push_str(&format!(
                r#"<path d="{}" fill="none" stroke="{}" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/>"#,
                path, curve.color.to_rgb_string()
            ));
        }

        // File B curves (dashed, different colors)
        for (i, curve) in b.curves.iter().enumerate() {
            let color = b_colors[i % b_colors.len()];
            let path = Self::rescale_curve_path(
                curve,
                margin_left,
                margin_top,
                plot_width,
                plot_height,
                unified_y.scale_max,
                max_gamma,
            );
            svg.push_str(&format!(
                r#"<path d="{}" fill="none" stroke="{}" stroke-width="2.5" stroke-dasharray="8,4" stroke-linecap="round" stroke-linejoin="round"/>"#,
                path, color
            ));
        }

        // Axis labels
        svg.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" font-size="12" fill="{}" font-family="{}">{}</text>"#,
            margin_left + plot_width / 2.0, height - 8.0,
            theme.text, theme.font_family, theme.labels.gamma_axis
        ));

        // Legend
        let a_count = a.curves.len().min(2);
        let b_count = b.curves.len().min(2);
        let legend_entries = a_count + b_count;
        let legend_height = legend_entries as f64 * 18.0 + 10.0;
        svg.push_str(&format!(
            r#"<g transform="translate({:.1}, {:.1})">"#,
            margin_left + 10.0,
            margin_top + 10.0
        ));
        svg.push_str(&format!(
            r#"<rect x="-5" y="-5" width="145" height="{legend_height:.1}" fill="{}" stroke="{}" stroke-width="1" rx="4"/>"#,
            theme.legend_bg, theme.axis
        ));

        let mut row = 0;
        // A curves
        for curve in a.curves.iter().take(2) {
            let y = row as f64 * 18.0 + 8.0;
            svg.push_str(&format!(
                r#"<line x1="0" y1="{y:.1}" x2="18" y2="{y:.1}" stroke="{}" stroke-width="2.5"/>"#,
                curve.color.to_rgb_string()
            ));
            svg.push_str(&format!(
                r#"<text x="24" y="{:.1}" font-size="11" fill="{}" font-family="{}">{} {}</text>"#,
                y + 4.0,
                theme.text,
                theme.font_family,
                label_a,
                curve.label
            ));
            row += 1;
        }
        // B curves
        for (i, curve) in b.curves.iter().take(2).enumerate() {
            let y = row as f64 * 18.0 + 8.0;
            let color = b_colors[i % b_colors.len()];
            svg.push_str(&format!(
                r#"<line x1="0" y1="{y:.1}" x2="18" y2="{y:.1}" stroke="{}" stroke-width="2.5" stroke-dasharray="4,2"/>"#,
                color
            ));
            svg.push_str(&format!(
                r#"<text x="24" y="{:.1}" font-size="11" fill="{}" font-family="{}">{} {}</text>"#,
                y + 4.0,
                theme.text,
                theme.font_family,
                label_b,
                curve.label
            ));
            row += 1;
        }
        svg.push_str("</g>");

        svg.push_str("</svg>");
        svg
    }

    /// Re-render a CartesianCurve path with a different Y scale and gamma range.
    fn rescale_curve_path(
        curve: &CartesianCurve,
        margin_left: f64,
        margin_top: f64,
        plot_width: f64,
        plot_height: f64,
        y_max: f64,
        max_gamma: f64,
    ) -> String {
        let mut path = String::new();
        for (i, pt) in curve.points.iter().enumerate() {
            let x = margin_left + plot_width * (pt.gamma / max_gamma);
            let y = margin_top + plot_height * (1.0 - pt.intensity / y_max);
            if i == 0 {
                path.push_str(&format!("M {x:.1} {y:.1}"));
            } else {
                path.push_str(&format!(" L {x:.1} {y:.1}"));
            }
        }
        path
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
