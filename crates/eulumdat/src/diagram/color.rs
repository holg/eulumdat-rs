//! Color utilities for diagram rendering
//!
//! Provides color conversion and palette generation functions.

/// RGB color representation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Convert to CSS rgb() string
    pub fn to_rgb_string(&self) -> String {
        format!("rgb({}, {}, {})", self.r, self.g, self.b)
    }

    /// Convert to CSS rgba() string
    pub fn to_rgba_string(&self, alpha: f64) -> String {
        format!("rgba({}, {}, {}, {})", self.r, self.g, self.b, alpha)
    }

    /// Convert to hex string (e.g., "#FF5500")
    pub fn to_hex_string(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    /// Lighten the color by a factor (0.0 = no change, 1.0 = white)
    pub fn lighten(&self, factor: f64) -> Self {
        let factor = factor.clamp(0.0, 1.0);
        Self {
            r: (self.r as f64 + (255.0 - self.r as f64) * factor) as u8,
            g: (self.g as f64 + (255.0 - self.g as f64) * factor) as u8,
            b: (self.b as f64 + (255.0 - self.b as f64) * factor) as u8,
        }
    }

    /// Darken the color by a factor (0.0 = no change, 1.0 = black)
    pub fn darken(&self, factor: f64) -> Self {
        let factor = factor.clamp(0.0, 1.0);
        Self {
            r: (self.r as f64 * (1.0 - factor)) as u8,
            g: (self.g as f64 * (1.0 - factor)) as u8,
            b: (self.b as f64 * (1.0 - factor)) as u8,
        }
    }
}

/// Convert HSL to RGB
///
/// # Arguments
/// * `h` - Hue in range 0.0..1.0 (0-360° mapped to 0-1)
/// * `s` - Saturation in range 0.0..1.0
/// * `l` - Lightness in range 0.0..1.0
pub fn hsl_to_rgb(h: f64, s: f64, l: f64) -> Color {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r, g, b) = match (h * 6.0) as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    Color {
        r: ((r + m) * 255.0) as u8,
        g: ((g + m) * 255.0) as u8,
        b: ((b + m) * 255.0) as u8,
    }
}

/// Generate heatmap color from normalized value (0.0-1.0)
///
/// Uses a perceptually pleasant colormap: dark blue -> cyan -> green -> yellow -> red
pub fn heatmap_color(value: f64) -> Color {
    let v = value.clamp(0.0, 1.0);

    if v < 0.25 {
        // Dark blue to cyan
        let t = v / 0.25;
        Color::new(
            (20.0 + t * 0.0) as u8,
            (20.0 + t * 150.0) as u8,
            (80.0 + t * 175.0) as u8,
        )
    } else if v < 0.5 {
        // Cyan to green
        let t = (v - 0.25) / 0.25;
        Color::new(
            (20.0 + t * 30.0) as u8,
            (170.0 + t * 50.0) as u8,
            (255.0 - t * 155.0) as u8,
        )
    } else if v < 0.75 {
        // Green to yellow
        let t = (v - 0.5) / 0.25;
        Color::new(
            (50.0 + t * 205.0) as u8,
            (220.0 + t * 35.0) as u8,
            (100.0 - t * 100.0) as u8,
        )
    } else {
        // Yellow to red
        let t = (v - 0.75) / 0.25;
        Color::new(255, (255.0 - t * 155.0) as u8, (0.0 + t * 50.0) as u8)
    }
}

/// Color palette for diagrams
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ColorPalette {
    /// Colors for different C-planes
    pub c_plane_colors: Vec<Color>,
    /// Color for C0-C180 plane (typically blue)
    pub c0_c180: Color,
    /// Color for C90-C270 plane (typically red)
    pub c90_c270: Color,
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self {
            c_plane_colors: vec![
                Color::new(59, 130, 246), // Blue (C0)
                Color::new(239, 68, 68),  // Red (C90)
                Color::new(34, 197, 94),  // Green (C180)
                Color::new(249, 115, 22), // Orange (C270)
                Color::new(139, 92, 246), // Purple
                Color::new(236, 72, 153), // Pink
                Color::new(14, 165, 233), // Sky
                Color::new(234, 179, 8),  // Yellow
            ],
            c0_c180: Color::new(59, 130, 246), // Blue
            c90_c270: Color::new(239, 68, 68), // Red
        }
    }
}

impl ColorPalette {
    /// Get color for a C-plane angle
    pub fn color_for_c_angle(&self, c_angle: f64) -> Color {
        // Generate color based on C-angle using HSL
        let hue = (c_angle / 360.0) * 240.0 + 180.0; // 180-420 (cyan to blue to purple)
        let hue = hue % 360.0;
        hsl_to_rgb(hue / 360.0, 0.7, 0.45)
    }

    /// Get color at index (cycles through palette)
    pub fn color_at(&self, index: usize) -> Color {
        self.c_plane_colors[index % self.c_plane_colors.len()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hsl_to_rgb() {
        // Red
        let red = hsl_to_rgb(0.0, 1.0, 0.5);
        assert_eq!(red.r, 255);
        assert!(red.g < 10);
        assert!(red.b < 10);

        // Green
        let green = hsl_to_rgb(1.0 / 3.0, 1.0, 0.5);
        assert!(green.r < 10);
        assert_eq!(green.g, 255);
        assert!(green.b < 10);
    }

    #[test]
    fn test_heatmap_color() {
        let low = heatmap_color(0.0);
        let high = heatmap_color(1.0);

        // Low values should be blue-ish
        assert!(low.b > low.r);
        // High values should be red-ish
        assert!(high.r > high.b);
    }

    #[test]
    fn test_color_strings() {
        let c = Color::new(255, 128, 0);
        assert_eq!(c.to_rgb_string(), "rgb(255, 128, 0)");
        assert_eq!(c.to_hex_string(), "#FF8000");
    }
}
