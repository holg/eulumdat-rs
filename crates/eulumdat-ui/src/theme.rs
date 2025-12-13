//! Theme settings for diagrams and UI

use egui::Color32;

/// Theme for diagram rendering
#[derive(Debug, Clone)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub struct Theme {
    /// Background color
    pub background: Color32,
    /// Grid line color
    pub grid: Color32,
    /// Primary curve color (C0-C180)
    pub primary_curve: Color32,
    /// Secondary curve color (C90-C270)
    pub secondary_curve: Color32,
    /// Text/label color
    pub text: Color32,
    /// Axis color
    pub axis: Color32,
    /// Whether this is a dark theme
    pub is_dark: bool,
}

impl Default for Theme {
    fn default() -> Self {
        Self::light()
    }
}

impl Theme {
    /// Light theme (white background)
    pub fn light() -> Self {
        Self {
            background: Color32::WHITE,
            grid: Color32::from_gray(220),
            primary_curve: Color32::from_rgb(220, 60, 60), // Red
            secondary_curve: Color32::from_rgb(60, 60, 220), // Blue
            text: Color32::from_gray(40),
            axis: Color32::from_gray(100),
            is_dark: false,
        }
    }

    /// Dark theme (dark background)
    pub fn dark() -> Self {
        Self {
            background: Color32::from_gray(30),
            grid: Color32::from_gray(60),
            primary_curve: Color32::from_rgb(255, 100, 100), // Light red
            secondary_curve: Color32::from_rgb(100, 150, 255), // Light blue
            text: Color32::from_gray(220),
            axis: Color32::from_gray(150),
            is_dark: true,
        }
    }

    /// Create theme from egui's visuals
    pub fn from_egui(ctx: &egui::Context) -> Self {
        if ctx.style().visuals.dark_mode {
            Self::dark()
        } else {
            Self::light()
        }
    }

    /// Get a color for a specific C-plane (for multi-curve diagrams)
    pub fn c_plane_color(&self, c_angle: f64, total_planes: usize) -> Color32 {
        let hue = (c_angle / 360.0) as f32;
        let (r, g, b) = hsl_to_rgb(hue, 0.7, if self.is_dark { 0.6 } else { 0.45 });
        Color32::from_rgb(r, g, b)
    }

    /// Get heatmap color for normalized intensity (0.0 - 1.0)
    pub fn heatmap_color(&self, normalized: f64) -> Color32 {
        let n = normalized.clamp(0.0, 1.0) as f32;

        // Blue -> Cyan -> Green -> Yellow -> Red
        let (r, g, b) = if n < 0.25 {
            let t = n / 0.25;
            (0.0, t, 1.0) // Blue to Cyan
        } else if n < 0.5 {
            let t = (n - 0.25) / 0.25;
            (0.0, 1.0, 1.0 - t) // Cyan to Green
        } else if n < 0.75 {
            let t = (n - 0.5) / 0.25;
            (t, 1.0, 0.0) // Green to Yellow
        } else {
            let t = (n - 0.75) / 0.25;
            (1.0, 1.0 - t, 0.0) // Yellow to Red
        };

        Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
    }
}

/// Convert HSL to RGB
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r, g, b) = match (h * 6.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}
