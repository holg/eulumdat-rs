//! Color utilities for photometric lighting.
//!
//! This module provides functions for:
//! - Converting color temperature (Kelvin) to RGB
//! - Parsing color temperature from strings
//! - CRI (Color Rendering Index) adjustments
//! - Heatmap color generation for visualization

use bevy::prelude::*;

/// Convert color temperature (Kelvin) to Bevy Color.
///
/// Uses Tanner Helland's algorithm for accurate Kelvin to RGB conversion.
///
/// # Arguments
/// * `kelvin` - Color temperature in Kelvin (1000-20000)
///
/// # Returns
/// Bevy Color in sRGB color space
///
/// # Example
/// ```ignore
/// let warm_white = kelvin_to_color(2700.0);  // Warm white
/// let neutral = kelvin_to_color(4000.0);     // Neutral white
/// let cool_white = kelvin_to_color(6500.0);  // Cool/daylight
/// ```
pub fn kelvin_to_color(kelvin: f32) -> Color {
    let temp = kelvin / 100.0;

    // Red
    let r = if temp <= 66.0 {
        255.0
    } else {
        let x = temp - 60.0;
        (329.698_73 * x.powf(-0.133_204_76)).clamp(0.0, 255.0)
    };

    // Green
    let g = if temp <= 66.0 {
        (99.470_8 * temp.ln() - 161.119_57).clamp(0.0, 255.0)
    } else {
        let x = temp - 60.0;
        (288.122_16 * x.powf(-0.075_514_846)).clamp(0.0, 255.0)
    };

    // Blue
    let b = if temp >= 66.0 {
        255.0
    } else if temp <= 19.0 {
        0.0
    } else {
        let x = temp - 10.0;
        (138.517_73 * x.ln() - 305.044_8).clamp(0.0, 255.0)
    };

    Color::srgb(r / 255.0, g / 255.0, b / 255.0)
}

/// Parse color temperature from a string.
///
/// Extracts a 4-digit Kelvin value from strings like:
/// - "3000K"
/// - "4000 Kelvin"
/// - "LED 5000K CRI90"
///
/// # Returns
/// Color temperature in Kelvin, or None if not found
///
/// # Example
/// ```ignore
/// assert_eq!(parse_color_temperature("3000K"), Some(3000.0));
/// assert_eq!(parse_color_temperature("LED 4000 Kelvin"), Some(4000.0));
/// assert_eq!(parse_color_temperature("unknown"), None);
/// ```
pub fn parse_color_temperature(appearance: &str) -> Option<f32> {
    // Try to extract a 4-digit number (typical CCT range 1800-10000)
    let mut digits = String::new();
    for ch in appearance.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
            if digits.len() == 4 {
                if let Ok(kelvin) = digits.parse::<f32>() {
                    if (1000.0..=20000.0).contains(&kelvin) {
                        return Some(kelvin);
                    }
                }
            }
        } else {
            digits.clear();
        }
    }
    None
}

/// Parse CRI (Color Rendering Index) from a group string.
///
/// CRI groups according to DIN standards:
/// - 1A: ≥90 (excellent)
/// - 1B: 80-89 (good)
/// - 2A: 70-79 (fair)
/// - 2B: 60-69 (moderate)
/// - 3: 40-59 (poor)
/// - 4: <40 (very poor)
///
/// # Returns
/// CRI value (0-100), defaults to 80.0 if unknown
///
/// # Example
/// ```ignore
/// assert_eq!(parse_cri("1A"), 95.0);
/// assert_eq!(parse_cri("1B"), 85.0);
/// assert_eq!(parse_cri("unknown"), 80.0);
/// ```
pub fn parse_cri(group: &str) -> f32 {
    let group = group.trim().to_uppercase();
    match group.as_str() {
        "1A" | "1" => 95.0,
        "1B" => 85.0,
        "2A" | "2" => 75.0,
        "2B" => 65.0,
        "3" => 50.0,
        "4" => 30.0,
        _ => {
            // Try to parse as a direct number
            group.parse::<f32>().unwrap_or(80.0)
        }
    }
}

/// Apply CRI-based color adjustment (desaturation).
///
/// Low CRI lights render colors less accurately, which we simulate
/// by desaturating the light color.
///
/// - CRI ≥90: Full saturation
/// - CRI 50: ~70% saturation
/// - CRI <50: Progressively more desaturated
///
/// # Arguments
/// * `color` - The original light color
/// * `cri` - Color Rendering Index (0-100)
///
/// # Returns
/// Adjusted color with appropriate saturation
pub fn apply_cri_adjustment(color: Color, cri: f32) -> Color {
    // CRI 100 = full saturation, CRI 0 = grayscale
    // We use a gentler curve: CRI 90+ = full sat, CRI 50 = ~70% sat
    let saturation_factor = if cri >= 90.0 {
        1.0
    } else {
        // Linear interpolation from CRI 50 (0.7) to CRI 90 (1.0)
        let t = ((cri - 50.0) / 40.0).clamp(0.0, 1.0);
        0.7 + 0.3 * t
    };

    // Convert to linear RGB, desaturate, convert back
    let linear = color.to_linear();
    let luminance = 0.2126 * linear.red + 0.7152 * linear.green + 0.0722 * linear.blue;

    let r = luminance + (linear.red - luminance) * saturation_factor;
    let g = luminance + (linear.green - luminance) * saturation_factor;
    let b = luminance + (linear.blue - luminance) * saturation_factor;

    Color::linear_rgb(r, g, b)
}

/// Generate a heatmap color for intensity visualization.
///
/// Maps a normalized value (0.0 - 1.0) to a color gradient:
/// - 0.00 - 0.25: Blue → Cyan
/// - 0.25 - 0.50: Cyan → Green
/// - 0.50 - 0.75: Green → Yellow
/// - 0.75 - 1.00: Yellow → Red
///
/// # Arguments
/// * `value` - Normalized intensity value (0.0 - 1.0)
///
/// # Returns
/// RGB color tuple (r, g, b) with values 0.0 - 1.0
pub fn heatmap_color(value: f64) -> (f32, f32, f32) {
    let v = value.clamp(0.0, 1.0) as f32;

    if v < 0.25 {
        let t = v / 0.25;
        (0.0, t, 1.0) // Blue to Cyan
    } else if v < 0.5 {
        let t = (v - 0.25) / 0.25;
        (0.0, 1.0, 1.0 - t) // Cyan to Green
    } else if v < 0.75 {
        let t = (v - 0.5) / 0.25;
        (t, 1.0, 0.0) // Green to Yellow
    } else {
        let t = (v - 0.75) / 0.25;
        (1.0, 1.0 - t, 0.0) // Yellow to Red
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_color_temperature() {
        assert_eq!(parse_color_temperature("3000K"), Some(3000.0));
        assert_eq!(parse_color_temperature("4000 Kelvin"), Some(4000.0));
        assert_eq!(parse_color_temperature("LED 5000K CRI90"), Some(5000.0));
        assert_eq!(parse_color_temperature("unknown"), None);
        assert_eq!(parse_color_temperature("123"), None); // Too short
    }

    #[test]
    fn test_parse_cri() {
        assert_eq!(parse_cri("1A"), 95.0);
        assert_eq!(parse_cri("1B"), 85.0);
        assert_eq!(parse_cri("2A"), 75.0);
        assert_eq!(parse_cri("2B"), 65.0);
        assert_eq!(parse_cri("3"), 50.0);
        assert_eq!(parse_cri("4"), 30.0);
        assert_eq!(parse_cri("90"), 90.0); // Direct number
        assert_eq!(parse_cri("unknown"), 80.0); // Default
    }

    #[test]
    fn test_heatmap_color() {
        let (r, g, b) = heatmap_color(0.0);
        assert_eq!((r, g, b), (0.0, 0.0, 1.0)); // Blue

        let (r, g, b) = heatmap_color(1.0);
        assert_eq!((r, g, b), (1.0, 0.0, 0.0)); // Red
    }
}
