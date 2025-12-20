//! Spectral power distribution (SPD) diagram generation
//!
//! Creates SVG visualizations of spectral data showing wavelength (nm) on X-axis
//! and relative/absolute spectral power on Y-axis.
//!
//! Supports extended wavelength ranges including:
//! - UV (280-400nm): Material degradation concerns
//! - Visible (380-780nm): Standard photometry
//! - Near-IR (780-1400nm): Thermal/heat radiation
//!
//! Provides spectral metrics for energy distribution analysis.

use crate::types::{SpectralDistribution, SpectralUnits};

// ============================================================================
// Localized Labels for Spectral SVG
// ============================================================================

/// Localized labels for spectral diagram text
#[derive(Debug, Clone, PartialEq)]
pub struct SpectralSvgLabels {
    /// Wavelength axis label (e.g., "Wavelength (nm)")
    pub wavelength_axis: String,
    /// Relative power axis label (e.g., "Relative Power")
    pub relative_power_axis: String,
    /// SPD title (e.g., "Spectral Power Distribution")
    pub spd_title: String,
    /// UV-A region label
    pub uv_a: String,
    /// Near-IR region label
    pub near_ir: String,
    /// Watts per nm unit
    pub watts_per_nm: String,
    /// Relative unit
    pub relative: String,
}

impl Default for SpectralSvgLabels {
    fn default() -> Self {
        Self::english()
    }
}

impl SpectralSvgLabels {
    /// English labels (default)
    pub fn english() -> Self {
        Self {
            wavelength_axis: "Wavelength (nm)".to_string(),
            relative_power_axis: "Relative Power".to_string(),
            spd_title: "Spectral Power Distribution".to_string(),
            uv_a: "UV-A".to_string(),
            near_ir: "Near-IR".to_string(),
            watts_per_nm: "W/nm".to_string(),
            relative: "Relative".to_string(),
        }
    }

    /// Create labels from eulumdat-i18n Locale
    #[cfg(feature = "i18n")]
    pub fn from_locale(locale: &eulumdat_i18n::Locale) -> Self {
        Self {
            wavelength_axis: locale.spectral.axis.wavelength.clone(),
            relative_power_axis: locale.spectral.axis.relative_power.clone(),
            spd_title: locale.spectral.title.spd.clone(),
            uv_a: locale.spectral.region.uv_a.clone(),
            near_ir: locale.spectral.region.near_ir.clone(),
            watts_per_nm: locale.spectral.units.watts_per_nm.clone(),
            relative: locale.spectral.units.relative.clone(),
        }
    }
}

// ============================================================================
// Spectral Wavelength Regions (nm)
// ============================================================================

/// UV-A region: 315-400nm (black light, material degradation)
pub const UV_A_START: f64 = 315.0;
pub const UV_A_END: f64 = 400.0;

/// Visible spectrum: 380-780nm
pub const VISIBLE_START: f64 = 380.0;
pub const VISIBLE_END: f64 = 780.0;

/// Near-IR (IR-A): 780-1400nm (felt as heat)
pub const NIR_START: f64 = 780.0;
pub const NIR_END: f64 = 1400.0;

/// Far-red region (important for plants): 700-780nm
pub const FAR_RED_START: f64 = 700.0;
pub const FAR_RED_END: f64 = 780.0;

/// Red region for R:FR ratio: 655-665nm
pub const RED_START: f64 = 655.0;
pub const RED_END: f64 = 665.0;

// ============================================================================
// Spectral Metrics
// ============================================================================

/// Comprehensive spectral metrics for energy distribution analysis
#[derive(Debug, Clone, Default)]
pub struct SpectralMetrics {
    /// UV-A content (315-400nm) as percentage of total
    pub uv_a_percent: f64,
    /// Visible content (380-780nm) as percentage of total
    pub visible_percent: f64,
    /// Near-IR content (780-1400nm) as percentage of total
    pub nir_percent: f64,
    /// Far-red content (700-780nm) as percentage of total
    pub far_red_percent: f64,
    /// PAR (400-700nm) as percentage of total
    pub par_percent: f64,
    /// Red to Far-Red ratio (R:FR) - important for plant morphology
    pub r_fr_ratio: Option<f64>,
    /// Blue percentage (400-500nm) of PAR
    pub blue_par_percent: f64,
    /// Green percentage (500-600nm) of PAR
    pub green_par_percent: f64,
    /// Red percentage (600-700nm) of PAR
    pub red_par_percent: f64,
    /// Wavelength range in data
    pub wavelength_min: f64,
    pub wavelength_max: f64,
    /// Peak wavelength
    pub peak_wavelength: f64,
    /// Has UV data (below 400nm)
    pub has_uv: bool,
    /// Has IR data (above 780nm)
    pub has_ir: bool,
    /// Thermal hazard warning (high IR content)
    pub thermal_warning: bool,
    /// UV hazard warning (high UV content)
    pub uv_warning: bool,
}

impl SpectralMetrics {
    /// Calculate spectral metrics from SPD
    pub fn from_spd(spd: &SpectralDistribution) -> Self {
        let wavelengths = if !spd.wavelengths.is_empty() {
            spd.wavelengths.clone()
        } else if let (Some(start), Some(interval)) =
            (spd.start_wavelength, spd.wavelength_interval)
        {
            (0..spd.values.len())
                .map(|i| start + i as f64 * interval)
                .collect()
        } else {
            return Self::default();
        };

        if wavelengths.is_empty() || spd.values.is_empty() {
            return Self::default();
        }

        let wavelength_min = wavelengths.iter().copied().fold(f64::MAX, f64::min);
        let wavelength_max = wavelengths.iter().copied().fold(f64::MIN, f64::max);

        // Integrate power in each region using trapezoidal rule
        let mut total_power = 0.0;
        let mut uv_a_power = 0.0;
        let mut visible_power = 0.0;
        let mut nir_power = 0.0;
        let mut far_red_power = 0.0;
        let mut par_power = 0.0;
        let mut blue_power = 0.0;
        let mut green_power = 0.0;
        let mut red_power = 0.0;
        let mut r_band_power = 0.0; // 655-665nm for R:FR
        let mut fr_band_power = 0.0; // 725-735nm for R:FR

        let mut peak_wavelength = wavelengths[0];
        let mut peak_value = spd.values[0];

        for i in 0..wavelengths.len() - 1 {
            let wl1 = wavelengths[i];
            let wl2 = wavelengths[i + 1];
            let v1 = spd.values[i];
            let v2 = spd.values[i + 1];

            // Track peak
            if v1 > peak_value {
                peak_value = v1;
                peak_wavelength = wl1;
            }

            // Trapezoidal integration
            let avg_val = (v1 + v2) / 2.0;
            let delta_wl = wl2 - wl1;
            let power = avg_val * delta_wl;

            total_power += power;

            let mid_wl = (wl1 + wl2) / 2.0;

            // UV-A (315-400nm)
            if (UV_A_START..UV_A_END).contains(&mid_wl) {
                uv_a_power += power;
            }

            // Visible (380-780nm)
            if (VISIBLE_START..=VISIBLE_END).contains(&mid_wl) {
                visible_power += power;
            }

            // Near-IR (780-1400nm)
            if mid_wl > NIR_START && mid_wl <= NIR_END {
                nir_power += power;
            }

            // Far-red (700-780nm)
            if (FAR_RED_START..=FAR_RED_END).contains(&mid_wl) {
                far_red_power += power;
            }

            // PAR (400-700nm)
            if (400.0..=700.0).contains(&mid_wl) {
                par_power += power;

                // Blue (400-500nm)
                if mid_wl < 500.0 {
                    blue_power += power;
                }
                // Green (500-600nm)
                else if mid_wl < 600.0 {
                    green_power += power;
                }
                // Red (600-700nm)
                else {
                    red_power += power;
                }
            }

            // R:FR ratio bands
            if (RED_START..=RED_END).contains(&mid_wl) {
                r_band_power += power;
            }
            if (725.0..=735.0).contains(&mid_wl) {
                fr_band_power += power;
            }
        }

        let total_power = total_power.max(0.0001); // Avoid division by zero
        let par_power_safe = par_power.max(0.0001);

        let r_fr_ratio = if fr_band_power > 0.0001 {
            Some(r_band_power / fr_band_power)
        } else {
            None
        };

        let has_uv = wavelength_min < 400.0;
        let has_ir = wavelength_max > 780.0;

        // Warning thresholds
        let nir_percent = nir_power / total_power * 100.0;
        let uv_a_percent = uv_a_power / total_power * 100.0;
        let thermal_warning = nir_percent > 25.0; // More than 25% IR is significant
        let uv_warning = uv_a_percent > 5.0; // More than 5% UV-A is concerning

        Self {
            uv_a_percent,
            visible_percent: visible_power / total_power * 100.0,
            nir_percent,
            far_red_percent: far_red_power / total_power * 100.0,
            par_percent: par_power / total_power * 100.0,
            r_fr_ratio,
            blue_par_percent: blue_power / par_power_safe * 100.0,
            green_par_percent: green_power / par_power_safe * 100.0,
            red_par_percent: red_power / par_power_safe * 100.0,
            wavelength_min,
            wavelength_max,
            peak_wavelength,
            has_uv,
            has_ir,
            thermal_warning,
            uv_warning,
        }
    }

    /// Get hazard level description
    pub fn hazard_level(&self) -> Option<&'static str> {
        if self.uv_warning && self.thermal_warning {
            Some("UV + Thermal hazard")
        } else if self.uv_warning {
            Some("UV exposure risk")
        } else if self.thermal_warning {
            Some("High thermal output")
        } else {
            None
        }
    }
}

/// Theme for spectral diagram SVG output
#[derive(Debug, Clone)]
pub struct SpectralTheme {
    /// Background color
    pub background: String,
    /// Axis and text color
    pub foreground: String,
    /// Grid line color
    pub grid: String,
    /// Fill gradient start color (violet ~380nm)
    pub fill_start: String,
    /// Fill gradient end color (red ~780nm)
    pub fill_end: String,
    /// Stroke color for curve
    pub stroke: String,
    /// Font family
    pub font_family: String,
    /// Show PAR (Photosynthetically Active Radiation) zones for horticultural lighting
    pub show_par_zones: bool,
    /// Show UV zone (when data includes UV wavelengths)
    pub show_uv_zone: bool,
    /// Show IR zone (when data includes IR wavelengths)
    pub show_ir_zone: bool,
    /// Localized labels for diagram text
    pub labels: SpectralSvgLabels,
}

impl Default for SpectralTheme {
    fn default() -> Self {
        Self::light()
    }
}

impl SpectralTheme {
    /// Light theme (white background)
    pub fn light() -> Self {
        Self {
            background: "#ffffff".to_string(),
            foreground: "#333333".to_string(),
            grid: "#e0e0e0".to_string(),
            fill_start: "#7c3aed".to_string(), // violet
            fill_end: "#ef4444".to_string(),   // red
            stroke: "#1e40af".to_string(),     // blue
            font_family: "system-ui, sans-serif".to_string(),
            show_par_zones: false,
            show_uv_zone: true, // Show UV/IR zones by default when data exists
            show_ir_zone: true,
            labels: SpectralSvgLabels::default(),
        }
    }

    /// Dark theme (dark background)
    pub fn dark() -> Self {
        Self {
            background: "#1a1a2e".to_string(),
            foreground: "#e0e0e0".to_string(),
            grid: "#333355".to_string(),
            fill_start: "#a78bfa".to_string(), // light violet
            fill_end: "#f87171".to_string(),   // light red
            stroke: "#60a5fa".to_string(),     // light blue
            font_family: "system-ui, sans-serif".to_string(),
            show_par_zones: false,
            show_uv_zone: true,
            show_ir_zone: true,
            labels: SpectralSvgLabels::default(),
        }
    }

    /// Set labels for this theme (for i18n)
    pub fn with_labels(mut self, labels: SpectralSvgLabels) -> Self {
        self.labels = labels;
        self
    }

    /// Create theme with locale labels
    #[cfg(feature = "i18n")]
    pub fn light_with_locale(locale: &eulumdat_i18n::Locale) -> Self {
        Self::light().with_labels(SpectralSvgLabels::from_locale(locale))
    }

    /// Create dark theme with locale labels
    #[cfg(feature = "i18n")]
    pub fn dark_with_locale(locale: &eulumdat_i18n::Locale) -> Self {
        Self::dark().with_labels(SpectralSvgLabels::from_locale(locale))
    }

    /// Light theme with PAR zones for horticultural applications
    pub fn light_par() -> Self {
        Self {
            show_par_zones: true,
            ..Self::light()
        }
    }

    /// Dark theme with PAR zones for horticultural applications
    pub fn dark_par() -> Self {
        Self {
            show_par_zones: true,
            ..Self::dark()
        }
    }

    /// Light theme with full spectral zones (UV + Visible + IR)
    pub fn light_full_spectrum() -> Self {
        Self {
            show_uv_zone: true,
            show_ir_zone: true,
            ..Self::light()
        }
    }

    /// Dark theme with full spectral zones (UV + Visible + IR)
    pub fn dark_full_spectrum() -> Self {
        Self {
            show_uv_zone: true,
            show_ir_zone: true,
            ..Self::dark()
        }
    }
}

/// Spectral diagram data with SVG generation
#[derive(Debug, Clone)]
pub struct SpectralDiagram {
    /// Wavelength values in nm
    pub wavelengths: Vec<f64>,
    /// Normalized values (0.0-1.0)
    pub values: Vec<f64>,
    /// Units label
    pub units_label: String,
    /// X-axis tick values
    pub x_ticks: Vec<f64>,
    /// Y-axis tick values
    pub y_ticks: Vec<f64>,
    /// Peak wavelength
    pub peak_wavelength: Option<f64>,
    /// Peak value
    pub peak_value: Option<f64>,
}

impl SpectralDiagram {
    /// Create spectral diagram from SpectralDistribution
    pub fn from_spectral(spd: &SpectralDistribution) -> Self {
        let wavelengths = if !spd.wavelengths.is_empty() {
            spd.wavelengths.clone()
        } else if let (Some(start), Some(interval)) =
            (spd.start_wavelength, spd.wavelength_interval)
        {
            (0..spd.values.len())
                .map(|i| start + i as f64 * interval)
                .collect()
        } else {
            // Default visible spectrum range
            let n = spd.values.len();
            if n > 1 {
                (0..n)
                    .map(|i| 380.0 + i as f64 * (400.0 / (n - 1) as f64))
                    .collect()
            } else {
                vec![550.0]
            }
        };

        // Normalize values
        let max_val = spd.values.iter().copied().fold(0.0_f64, f64::max);
        let values: Vec<f64> = if max_val > 0.0 {
            spd.values.iter().map(|v| v / max_val).collect()
        } else {
            spd.values.clone()
        };

        // Find peak
        let (peak_wavelength, peak_value) = if !wavelengths.is_empty() && !values.is_empty() {
            let (idx, &peak_v) = values
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .unwrap_or((0, &0.0));
            (
                Some(wavelengths.get(idx).copied().unwrap_or(550.0)),
                Some(peak_v),
            )
        } else {
            (None, None)
        };

        // Generate ticks
        let min_wl = wavelengths.iter().copied().fold(f64::MAX, f64::min);
        let max_wl = wavelengths.iter().copied().fold(f64::MIN, f64::max);

        let x_ticks = generate_wavelength_ticks(min_wl, max_wl);
        let y_ticks = vec![0.0, 0.25, 0.5, 0.75, 1.0];

        let units_label = match spd.units {
            SpectralUnits::WattsPerNanometer => "W/nm".to_string(),
            SpectralUnits::Relative => "Relative".to_string(),
        };

        Self {
            wavelengths,
            values,
            units_label,
            x_ticks,
            y_ticks,
            peak_wavelength,
            peak_value,
        }
    }

    /// Generate SVG string
    pub fn to_svg(&self, width: f64, height: f64, theme: &SpectralTheme) -> String {
        let margin_left = 60.0;
        let margin_right = 30.0;
        let margin_top = 30.0;
        let margin_bottom = 50.0;

        let plot_width = width - margin_left - margin_right;
        let plot_height = height - margin_top - margin_bottom;

        let min_wl = self.wavelengths.iter().copied().fold(f64::MAX, f64::min);
        let max_wl = self.wavelengths.iter().copied().fold(f64::MIN, f64::max);
        let wl_range = if (max_wl - min_wl).abs() < f64::EPSILON {
            400.0
        } else {
            max_wl - min_wl
        };

        // Determine if this is a dark theme (for zone coloring)
        let is_dark = theme.background.contains("1a") || theme.background.contains("2e");

        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}">
  <defs>
    <linearGradient id="spd-gradient" x1="0%" y1="0%" x2="100%" y2="0%">
      <stop offset="0%" style="stop-color:{fill_start};stop-opacity:0.4"/>
      <stop offset="50%" style="stop-color:#10b981;stop-opacity:0.4"/>
      <stop offset="100%" style="stop-color:{fill_end};stop-opacity:0.4"/>
    </linearGradient>
    <linearGradient id="spectrum-gradient" x1="0%" y1="0%" x2="100%" y2="0%">
{spectrum_stops}
    </linearGradient>
  </defs>
  <rect width="{width}" height="{height}" fill="{bg}"/>
"#,
            width = width,
            height = height,
            fill_start = theme.fill_start,
            fill_end = theme.fill_end,
            bg = theme.background,
            spectrum_stops = generate_spectrum_gradient_stops_extended(min_wl, max_wl),
        );

        // UV zone (when data includes UV wavelengths)
        if theme.show_uv_zone && min_wl < UV_A_END {
            svg.push_str(&generate_uv_zone(
                margin_left,
                margin_top,
                plot_width,
                plot_height,
                min_wl,
                wl_range,
                is_dark,
                &theme.labels,
            ));
        }

        // IR zone (when data includes IR wavelengths)
        if theme.show_ir_zone && max_wl > NIR_START {
            svg.push_str(&generate_ir_zone(
                margin_left,
                margin_top,
                plot_width,
                plot_height,
                max_wl,
                min_wl,
                wl_range,
                is_dark,
                &theme.labels,
            ));
        }

        // PAR zones for horticultural lighting
        if theme.show_par_zones {
            svg.push_str(&generate_par_zones(
                margin_left,
                margin_top,
                plot_width,
                plot_height,
                min_wl,
                wl_range,
            ));
        }

        // Grid lines and Y-axis labels
        for &y_val in &self.y_ticks {
            let y = margin_top + plot_height * (1.0 - y_val);
            svg.push_str(&format!(
                r#"  <line x1="{}" y1="{:.1}" x2="{}" y2="{:.1}" stroke="{}" stroke-width="1"/>"#,
                margin_left,
                y,
                margin_left + plot_width,
                y,
                theme.grid
            ));
            svg.push('\n');
            svg.push_str(&format!(
                r#"  <text x="{}" y="{:.1}" fill="{}" font-size="11" font-family="{}" text-anchor="end" dominant-baseline="middle">{:.0}%</text>"#,
                margin_left - 8.0, y, theme.foreground, theme.font_family, y_val * 100.0
            ));
            svg.push('\n');
        }

        // X-axis labels
        for &wl in &self.x_ticks {
            if wl >= min_wl && wl <= max_wl {
                let x = margin_left + plot_width * ((wl - min_wl) / wl_range);
                svg.push_str(&format!(
                    r#"  <line x1="{:.1}" y1="{}" x2="{:.1}" y2="{}" stroke="{}" stroke-width="1"/>"#,
                    x, margin_top, x, margin_top + plot_height, theme.grid
                ));
                svg.push('\n');
                svg.push_str(&format!(
                    r#"  <text x="{:.1}" y="{}" fill="{}" font-size="11" font-family="{}" text-anchor="middle">{:.0}</text>"#,
                    x, margin_top + plot_height + 18.0, theme.foreground, theme.font_family, wl
                ));
                svg.push('\n');
            }
        }

        // Plot area border
        svg.push_str(&format!(
            r#"  <rect x="{}" y="{}" width="{}" height="{}" fill="none" stroke="{}" stroke-width="1"/>"#,
            margin_left, margin_top, plot_width, plot_height, theme.grid
        ));
        svg.push('\n');

        // Visible spectrum bar at bottom
        svg.push_str(&format!(
            r#"  <rect x="{}" y="{}" width="{}" height="8" fill="url(#spectrum-gradient)" rx="2"/>"#,
            margin_left, margin_top + plot_height + 30.0, plot_width
        ));
        svg.push('\n');

        // Generate path data
        if !self.wavelengths.is_empty() && !self.values.is_empty() {
            let mut path_data = String::new();
            let mut fill_path = String::new();

            for (i, (&wl, &val)) in self.wavelengths.iter().zip(self.values.iter()).enumerate() {
                let x = margin_left + plot_width * ((wl - min_wl) / wl_range);
                let y = margin_top + plot_height * (1.0 - val);

                if i == 0 {
                    path_data.push_str(&format!("M {:.1} {:.1}", x, y));
                    fill_path.push_str(&format!(
                        "M {:.1} {:.1}",
                        margin_left,
                        margin_top + plot_height
                    ));
                    fill_path.push_str(&format!(" L {:.1} {:.1}", x, y));
                } else {
                    path_data.push_str(&format!(" L {:.1} {:.1}", x, y));
                    fill_path.push_str(&format!(" L {:.1} {:.1}", x, y));
                }
            }

            // Close fill path
            let last_x =
                margin_left + plot_width * ((self.wavelengths.last().unwrap() - min_wl) / wl_range);
            fill_path.push_str(&format!(" L {:.1} {:.1}", last_x, margin_top + plot_height));
            fill_path.push_str(" Z");

            // Filled area
            svg.push_str(&format!(
                r#"  <path d="{}" fill="url(#spd-gradient)" stroke="none"/>"#,
                fill_path
            ));
            svg.push('\n');

            // Curve
            svg.push_str(&format!(
                r#"  <path d="{}" fill="none" stroke="{}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>"#,
                path_data, theme.stroke
            ));
            svg.push('\n');

            // Peak marker
            if let (Some(peak_wl), Some(peak_v)) = (self.peak_wavelength, self.peak_value) {
                let px = margin_left + plot_width * ((peak_wl - min_wl) / wl_range);
                let py = margin_top + plot_height * (1.0 - peak_v);
                svg.push_str(&format!(
                    r#"  <circle cx="{:.1}" cy="{:.1}" r="4" fill="{}" stroke="{}" stroke-width="2"/>"#,
                    px, py, theme.stroke, theme.background
                ));
                svg.push('\n');
                svg.push_str(&format!(
                    r#"  <text x="{:.1}" y="{:.1}" fill="{}" font-size="10" font-family="{}" text-anchor="middle">{:.0}nm</text>"#,
                    px, py - 10.0, theme.foreground, theme.font_family, peak_wl
                ));
                svg.push('\n');
            }
        }

        // X-axis title
        svg.push_str(&format!(
            r#"  <text x="{}" y="{}" fill="{}" font-size="12" font-family="{}" text-anchor="middle">{}</text>"#,
            margin_left + plot_width / 2.0, height - 8.0, theme.foreground, theme.font_family, theme.labels.wavelength_axis
        ));
        svg.push('\n');

        // Y-axis title
        svg.push_str(&format!(
            r#"  <text x="15" y="{}" fill="{}" font-size="12" font-family="{}" text-anchor="middle" transform="rotate(-90 15 {})">{}</text>"#,
            margin_top + plot_height / 2.0, theme.foreground, theme.font_family, margin_top + plot_height / 2.0, theme.labels.relative_power_axis
        ));
        svg.push('\n');

        // Title
        svg.push_str(&format!(
            r#"  <text x="{}" y="18" fill="{}" font-size="14" font-family="{}" font-weight="bold" text-anchor="middle">{}</text>"#,
            width / 2.0, theme.foreground, theme.font_family, theme.labels.spd_title
        ));
        svg.push('\n');

        svg.push_str("</svg>");
        svg
    }
}

/// Generate wavelength tick values
fn generate_wavelength_ticks(min_wl: f64, max_wl: f64) -> Vec<f64> {
    let range = max_wl - min_wl;
    let step = if range <= 100.0 {
        10.0
    } else if range <= 200.0 {
        25.0
    } else if range <= 400.0 {
        50.0
    } else {
        100.0
    };

    let start = (min_wl / step).floor() * step;
    let end = (max_wl / step).ceil() * step;

    let mut ticks = Vec::new();
    let mut wl = start;
    while wl <= end {
        ticks.push(wl);
        wl += step;
    }
    ticks
}

/// Generate extended spectrum gradient stops (UV to IR)
fn generate_spectrum_gradient_stops_extended(min_wl: f64, max_wl: f64) -> String {
    // Extended spectrum colors from UV through visible to IR
    let colors = [
        (280.0, "#4c1d95"),  // UV-B (deep purple)
        (315.0, "#6d28d9"),  // UV-A start
        (380.0, "#7c3aed"),  // violet
        (420.0, "#3b82f6"),  // blue
        (470.0, "#22d3ee"),  // cyan
        (530.0, "#22c55e"),  // green
        (580.0, "#eab308"),  // yellow
        (620.0, "#f97316"),  // orange
        (700.0, "#ef4444"),  // red
        (780.0, "#b91c1c"),  // far-red/NIR boundary
        (900.0, "#7f1d1d"),  // near-IR
        (1100.0, "#451a03"), // deep IR (brown)
        (1400.0, "#1c1917"), // far NIR (nearly black/heat)
    ];

    let wl_range = max_wl - min_wl;
    if wl_range <= 0.0 {
        return String::new();
    }

    let mut stops = String::new();
    for (wl, color) in &colors {
        if *wl >= min_wl && *wl <= max_wl {
            let offset = (wl - min_wl) / wl_range * 100.0;
            stops.push_str(&format!(
                r#"      <stop offset="{:.1}%" style="stop-color:{}"/>"#,
                offset, color
            ));
            stops.push('\n');
        }
    }

    // Add edge stops if needed
    if min_wl < colors[0].0 {
        stops = format!(
            r#"      <stop offset="0%" style="stop-color:{}"/>"#,
            colors[0].1
        ) + "\n"
            + &stops;
    }
    if max_wl > colors[colors.len() - 1].0 {
        stops.push_str(&format!(
            r#"      <stop offset="100%" style="stop-color:{}"/>"#,
            colors[colors.len() - 1].1
        ));
        stops.push('\n');
    }

    stops
}

/// Generate visible spectrum gradient stops (legacy, 380-700nm)
#[allow(dead_code)]
fn generate_spectrum_gradient_stops() -> String {
    generate_spectrum_gradient_stops_extended(380.0, 700.0)
}

/// Generate UV zone background (when data includes UV wavelengths)
#[allow(clippy::too_many_arguments)]
fn generate_uv_zone(
    margin_left: f64,
    margin_top: f64,
    plot_width: f64,
    plot_height: f64,
    min_wl: f64,
    wl_range: f64,
    is_dark: bool,
    labels: &SpectralSvgLabels,
) -> String {
    let mut svg = String::new();

    // Only show if data extends into UV
    if min_wl >= UV_A_END {
        return svg;
    }

    let uv_color = if is_dark { "#6d28d920" } else { "#7c3aed15" };
    let uv_border = if is_dark { "#6d28d9" } else { "#7c3aed" };

    // UV-A zone (315-400nm)
    let uv_start =
        margin_left + plot_width * ((UV_A_START.max(min_wl) - min_wl) / wl_range).clamp(0.0, 1.0);
    let uv_end = margin_left + plot_width * ((UV_A_END - min_wl) / wl_range).clamp(0.0, 1.0);
    let uv_width = uv_end - uv_start;

    if uv_width > 0.0 {
        // Zone background
        svg.push_str(&format!(
            r#"  <rect x="{:.1}" y="{}" width="{:.1}" height="{}" fill="{}" />"#,
            uv_start, margin_top, uv_width, plot_height, uv_color
        ));
        svg.push('\n');

        // Zone label
        if uv_width > 30.0 {
            svg.push_str(&format!(
                r#"  <text x="{:.1}" y="{}" fill="{}" font-size="9" font-family="system-ui, sans-serif" text-anchor="middle" opacity="0.8">{}</text>"#,
                uv_start + uv_width / 2.0, margin_top + 12.0, uv_border, labels.uv_a
            ));
            svg.push('\n');
        }

        // Hazard stripe pattern at top
        svg.push_str(&format!(
            r#"  <rect x="{:.1}" y="{}" width="{:.1}" height="4" fill="{}" opacity="0.6"/>"#,
            uv_start, margin_top, uv_width, uv_border
        ));
        svg.push('\n');
    }

    svg
}

/// Generate IR zone background (when data includes IR wavelengths)
#[allow(clippy::too_many_arguments)]
fn generate_ir_zone(
    margin_left: f64,
    margin_top: f64,
    plot_width: f64,
    plot_height: f64,
    max_wl: f64,
    min_wl: f64,
    wl_range: f64,
    is_dark: bool,
    labels: &SpectralSvgLabels,
) -> String {
    let mut svg = String::new();

    // Only show if data extends into IR
    if max_wl <= NIR_START {
        return svg;
    }

    let _ir_color = if is_dark { "#b91c1c20" } else { "#ef444415" };
    let ir_border = if is_dark { "#f87171" } else { "#ef4444" };

    // Near-IR zone (780-1400nm or data max)
    let ir_start = margin_left + plot_width * ((NIR_START - min_wl) / wl_range).clamp(0.0, 1.0);
    let ir_end =
        margin_left + plot_width * ((max_wl.min(NIR_END) - min_wl) / wl_range).clamp(0.0, 1.0);
    let ir_width = ir_end - ir_start;

    if ir_width > 0.0 {
        // Zone background with gradient (fades to warmer color)
        svg.push_str(
            r#"  <defs><linearGradient id="ir-gradient" x1="0%" y1="0%" x2="100%" y2="0%">
      <stop offset="0%" style="stop-color:#ef4444;stop-opacity:0.1"/>
      <stop offset="100%" style="stop-color:#7f1d1d;stop-opacity:0.2"/>
    </linearGradient></defs>"#,
        );
        svg.push('\n');

        svg.push_str(&format!(
            r#"  <rect x="{:.1}" y="{}" width="{:.1}" height="{}" fill="url(#ir-gradient)" />"#,
            ir_start, margin_top, ir_width, plot_height
        ));
        svg.push('\n');

        // Zone label
        if ir_width > 30.0 {
            svg.push_str(&format!(
                r#"  <text x="{:.1}" y="{}" fill="{}" font-size="9" font-family="system-ui, sans-serif" text-anchor="middle" opacity="0.8">{}</text>"#,
                ir_start + ir_width / 2.0, margin_top + 12.0, ir_border, labels.near_ir
            ));
            svg.push('\n');
        }

        // Heat indicator stripe at top
        svg.push_str(&format!(
            r#"  <rect x="{:.1}" y="{}" width="{:.1}" height="4" fill="{}" opacity="0.6"/>"#,
            ir_start, margin_top, ir_width, ir_border
        ));
        svg.push('\n');

        // Thermal symbol (flame/heat icon approximation)
        if ir_width > 50.0 {
            let icon_x = ir_start + ir_width - 15.0;
            let icon_y = margin_top + 20.0;
            svg.push_str(&format!(
                r#"  <text x="{:.1}" y="{:.1}" font-size="12" fill="{}" opacity="0.7">🔥</text>"#,
                icon_x, icon_y, ir_border
            ));
            svg.push('\n');
        }
    }

    svg
}

/// Generate PAR (Photosynthetically Active Radiation) zone background bands
/// for horticultural lighting applications
fn generate_par_zones(
    margin_left: f64,
    margin_top: f64,
    plot_width: f64,
    plot_height: f64,
    min_wl: f64,
    wl_range: f64,
) -> String {
    let mut svg = String::new();

    // PAR zones with colors and labels
    // Blue: 400-500nm (vegetative growth, chlorophyll b)
    // Green: 500-600nm (penetrates canopy, some photosynthesis)
    // Red: 600-700nm (flowering, chlorophyll a)
    // Far-red: 700-780nm (shade response, phytochrome)
    let zones = [
        (400.0, 500.0, "#3b82f620", "Blue"),    // Blue zone - 20% opacity
        (500.0, 600.0, "#22c55e15", "Green"),   // Green zone - 15% opacity
        (600.0, 700.0, "#ef444425", "Red"),     // Red zone - 25% opacity
        (700.0, 780.0, "#7c3aed15", "Far-Red"), // Far-red zone - 15% opacity
    ];

    for (start_wl, end_wl, color, label) in zones {
        // Calculate x positions
        let x_start = margin_left + plot_width * ((start_wl - min_wl) / wl_range).clamp(0.0, 1.0);
        let x_end = margin_left + plot_width * ((end_wl - min_wl) / wl_range).clamp(0.0, 1.0);
        let zone_width = x_end - x_start;

        if zone_width > 0.0 {
            // Zone background
            svg.push_str(&format!(
                r#"  <rect x="{:.1}" y="{}" width="{:.1}" height="{}" fill="{}" />"#,
                x_start, margin_top, zone_width, plot_height, color
            ));
            svg.push('\n');

            // Zone label at top
            let label_x = x_start + zone_width / 2.0;
            svg.push_str(&format!(
                "  <text x=\"{:.1}\" y=\"{}\" fill=\"#666\" font-size=\"9\" font-family=\"system-ui, sans-serif\" text-anchor=\"middle\" opacity=\"0.7\">{}</text>",
                label_x, margin_top + 12.0, label
            ));
            svg.push('\n');
        }
    }

    // PAR range indicator (400-700nm)
    let par_start = margin_left + plot_width * ((400.0 - min_wl) / wl_range).clamp(0.0, 1.0);
    let par_end = margin_left + plot_width * ((700.0 - min_wl) / wl_range).clamp(0.0, 1.0);
    let par_width = par_end - par_start;

    if par_width > 0.0 {
        svg.push_str(&format!(
            "  <line x1=\"{:.1}\" y1=\"{}\" x2=\"{:.1}\" y2=\"{}\" stroke=\"#22c55e\" stroke-width=\"2\" stroke-dasharray=\"4,2\" opacity=\"0.6\"/>",
            par_start, margin_top + plot_height + 2.0, par_end, margin_top + plot_height + 2.0
        ));
        svg.push('\n');
        svg.push_str(&format!(
            "  <text x=\"{:.1}\" y=\"{}\" fill=\"#22c55e\" font-size=\"10\" font-family=\"system-ui, sans-serif\" text-anchor=\"middle\" font-weight=\"bold\">PAR (400-700nm)</text>",
            par_start + par_width / 2.0, margin_top + plot_height + 14.0
        ));
        svg.push('\n');
    }

    svg
}

/// Synthesize an approximate spectral distribution from CCT and CRI
///
/// This creates a realistic LED-like spectrum based on color temperature
/// and color rendering characteristics.
pub fn synthesize_spectrum(cct: f64, cri: Option<f64>) -> SpectralDistribution {
    let wavelengths: Vec<f64> = (380..=780).step_by(5).map(|w| w as f64).collect();
    let cri_val = cri.unwrap_or(80.0);

    let values: Vec<f64> = wavelengths
        .iter()
        .map(|&wl| synthesize_spd_value(wl, cct, cri_val))
        .collect();

    // Normalize to peak = 1.0
    let max_val = values.iter().copied().fold(0.0_f64, f64::max);
    let normalized: Vec<f64> = if max_val > 0.0 {
        values.iter().map(|v| v / max_val).collect()
    } else {
        values
    };

    SpectralDistribution {
        wavelengths,
        values: normalized,
        units: SpectralUnits::Relative,
        start_wavelength: None,
        wavelength_interval: None,
    }
}

/// Calculate SPD value at a specific wavelength based on CCT and CRI
fn synthesize_spd_value(wavelength: f64, cct: f64, cri: f64) -> f64 {
    // Base: Planckian (blackbody) radiation approximation
    let planckian = planckian_approximation(wavelength, cct);

    // LED characteristics: blue pump + phosphor
    let blue_peak = if cct > 4000.0 {
        // Cool white: stronger blue peak around 450nm
        gaussian(wavelength, 450.0, 20.0) * 0.8
    } else {
        // Warm white: moderate blue peak
        gaussian(wavelength, 450.0, 18.0) * 0.5
    };

    // Phosphor emission (broadband yellow-red)
    let phosphor_center = if cct > 5000.0 {
        550.0 // Cool white: greener phosphor
    } else if cct > 3500.0 {
        570.0 // Neutral white
    } else {
        590.0 // Warm white: more orange phosphor
    };
    let phosphor_width = 80.0 + (cri - 80.0) * 0.5; // Higher CRI = broader emission
    let phosphor = gaussian(wavelength, phosphor_center, phosphor_width);

    // Red enhancement for high CRI (R9)
    let red_boost = if cri > 90.0 {
        gaussian(wavelength, 630.0, 30.0) * 0.3
    } else if cri > 80.0 {
        gaussian(wavelength, 630.0, 25.0) * 0.15
    } else {
        0.0
    };

    // Combine components
    let led_spectrum = blue_peak + phosphor * 1.2 + red_boost;

    // Blend with planckian for more natural shape
    let blend_factor = (cri - 70.0).clamp(0.0, 30.0) / 30.0 * 0.3;
    led_spectrum * (1.0 - blend_factor) + planckian * blend_factor
}

/// Gaussian function for spectral peaks
fn gaussian(x: f64, center: f64, width: f64) -> f64 {
    (-((x - center) / width).powi(2)).exp()
}

/// Simplified Planckian (blackbody) radiation approximation
fn planckian_approximation(wavelength: f64, cct: f64) -> f64 {
    // Wien's approximation (simplified)
    let wl_um = wavelength / 1000.0; // Convert nm to µm
    let c2 = 14388.0; // µm·K

    let exponent = c2 / (wl_um * cct);
    if exponent > 50.0 {
        return 0.0; // Avoid overflow
    }

    let intensity = 1.0 / (wl_um.powi(5) * (exponent.exp() - 1.0));

    // Normalize roughly to visible range
    intensity * 1e-10
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synthesize_spectrum() {
        let spd = synthesize_spectrum(3000.0, Some(90.0));
        assert_eq!(spd.wavelengths.len(), 81);
        assert_eq!(spd.values.len(), 81);
        // Check normalized
        let max_val = spd.values.iter().copied().fold(0.0_f64, f64::max);
        assert!((max_val - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_spectral_diagram_creation() {
        let spd = SpectralDistribution {
            wavelengths: vec![400.0, 450.0, 500.0, 550.0, 600.0, 650.0, 700.0],
            values: vec![0.1, 0.3, 0.7, 1.0, 0.8, 0.4, 0.1],
            units: SpectralUnits::Relative,
            start_wavelength: None,
            wavelength_interval: None,
        };

        let diagram = SpectralDiagram::from_spectral(&spd);
        assert_eq!(diagram.wavelengths.len(), 7);
        assert!(diagram.peak_wavelength.is_some());
        assert!((diagram.peak_wavelength.unwrap() - 550.0).abs() < 0.1);
    }

    #[test]
    fn test_spectral_diagram_svg() {
        let spd = SpectralDistribution {
            wavelengths: vec![400.0, 450.0, 500.0, 550.0, 600.0, 650.0, 700.0],
            values: vec![0.1, 0.3, 0.7, 1.0, 0.8, 0.4, 0.1],
            units: SpectralUnits::Relative,
            start_wavelength: None,
            wavelength_interval: None,
        };

        let diagram = SpectralDiagram::from_spectral(&spd);
        let svg = diagram.to_svg(600.0, 400.0, &SpectralTheme::light());

        assert!(svg.contains("<svg"));
        assert!(svg.contains("Spectral Power Distribution"));
        assert!(svg.contains("Wavelength (nm)"));
    }
}
