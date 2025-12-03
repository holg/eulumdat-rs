//! Cartesian diagram data generation
//!
//! Generates the data needed for X-Y plots showing intensity vs gamma angle
//! for each C-plane.

use super::color::{Color, ColorPalette};
use super::{DiagramScale, Point2D};
use crate::Eulumdat;

/// A point in a cartesian curve
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CartesianPoint {
    /// X coordinate (gamma angle, scaled to plot width)
    pub x: f64,
    /// Y coordinate (intensity, scaled to plot height)
    pub y: f64,
    /// Original gamma angle in degrees
    pub gamma: f64,
    /// Intensity value at this point
    pub intensity: f64,
}

/// A cartesian curve for one C-plane
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CartesianCurve {
    /// Points in the curve
    pub points: Vec<CartesianPoint>,
    /// C-plane angle this curve represents
    pub c_angle: f64,
    /// Color for this curve
    pub color: Color,
    /// Label for this curve (e.g., "C0°")
    pub label: String,
}

impl CartesianCurve {
    /// Check if the curve has valid data
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Convert to SVG path string (already in screen coordinates)
    pub fn to_svg_path(&self) -> String {
        if self.points.is_empty() {
            return String::new();
        }

        let mut path = String::new();

        for (i, point) in self.points.iter().enumerate() {
            if i == 0 {
                path.push_str(&format!("M {:.1} {:.1}", point.x, point.y));
            } else {
                path.push_str(&format!(" L {:.1} {:.1}", point.x, point.y));
            }
        }

        path
    }
}

/// Complete cartesian diagram data
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CartesianDiagram {
    /// Curves for each C-plane
    pub curves: Vec<CartesianCurve>,
    /// X-axis tick values (gamma angles)
    pub x_ticks: Vec<f64>,
    /// Y-axis tick values (intensity)
    pub y_ticks: Vec<f64>,
    /// Scale information
    pub scale: DiagramScale,
    /// Maximum gamma angle in the data
    pub max_gamma: f64,
    /// Plot dimensions (for reference)
    pub plot_width: f64,
    pub plot_height: f64,
    pub margin_left: f64,
    pub margin_top: f64,
}

impl CartesianDiagram {
    /// Generate cartesian diagram data from Eulumdat
    ///
    /// # Arguments
    /// * `ldt` - The Eulumdat data
    /// * `width` - Total output width in pixels
    /// * `height` - Total output height in pixels
    /// * `max_curves` - Maximum number of curves to include (for readability)
    pub fn from_eulumdat(ldt: &Eulumdat, width: f64, height: f64, max_curves: usize) -> Self {
        let margin_left = 60.0;
        let margin_right = 25.0;
        let margin_top = 35.0;
        let margin_bottom = 50.0;

        let plot_width = width - margin_left - margin_right;
        let plot_height = height - margin_top - margin_bottom;

        let max_intensity = ldt.max_intensity();
        let max_gamma = ldt.g_angles.last().copied().unwrap_or(90.0);

        // Calculate nice Y-axis ticks
        let y_ticks = if max_intensity > 0.0 {
            let step = DiagramScale::nice_step(max_intensity, 5);
            let mut ticks = Vec::new();
            let mut v = 0.0;
            while v <= max_intensity * 1.05 {
                ticks.push(v);
                v += step;
            }
            ticks
        } else {
            vec![0.0, 25.0, 50.0, 75.0, 100.0]
        };

        let y_max = y_ticks.last().copied().unwrap_or(100.0);

        // Calculate X-axis ticks
        let x_ticks = {
            let step = if max_gamma <= 90.0 { 15.0 } else { 30.0 };
            let mut ticks = Vec::new();
            let mut v = 0.0;
            while v <= max_gamma {
                ticks.push(v);
                v += step;
            }
            ticks
        };

        let scale = DiagramScale {
            max_intensity,
            scale_max: y_max,
            grid_values: y_ticks.clone(),
        };

        // Generate curves
        let palette = ColorPalette::default();
        let curves = generate_curves(
            ldt,
            margin_left,
            margin_top,
            plot_width,
            plot_height,
            y_max,
            max_gamma,
            max_curves,
            &palette,
        );

        Self {
            curves,
            x_ticks,
            y_ticks,
            scale,
            max_gamma,
            plot_width,
            plot_height,
            margin_left,
            margin_top,
        }
    }

    /// Get data points for all curves (useful for non-SVG rendering)
    pub fn all_data_points(&self) -> Vec<(&CartesianCurve, Vec<Point2D>)> {
        self.curves
            .iter()
            .map(|curve| {
                let points: Vec<Point2D> = curve
                    .points
                    .iter()
                    .map(|p| Point2D::new(p.x, p.y))
                    .collect();
                (curve, points)
            })
            .collect()
    }
}

/// Generate curves for each C-plane
#[allow(clippy::too_many_arguments)]
fn generate_curves(
    ldt: &Eulumdat,
    margin_left: f64,
    margin_top: f64,
    plot_width: f64,
    plot_height: f64,
    y_max: f64,
    max_gamma: f64,
    max_curves: usize,
    palette: &ColorPalette,
) -> Vec<CartesianCurve> {
    if ldt.intensities.is_empty() || ldt.g_angles.is_empty() || y_max <= 0.0 {
        return Vec::new();
    }

    let mut curves = Vec::new();
    let num_curves = ldt.intensities.len().min(max_curves);

    for (c_idx, intensities) in ldt.intensities.iter().take(num_curves).enumerate() {
        let mut points = Vec::new();

        for (i, (&g_angle, &intensity)) in ldt.g_angles.iter().zip(intensities.iter()).enumerate() {
            let _ = i; // unused but kept for clarity
            let x = margin_left + plot_width * (g_angle / max_gamma);
            let y = margin_top + plot_height * (1.0 - intensity / y_max);

            points.push(CartesianPoint {
                x,
                y,
                gamma: g_angle,
                intensity,
            });
        }

        let color = palette.color_at(c_idx);
        let label = if c_idx < ldt.c_angles.len() {
            format!("C{:.0}°", ldt.c_angles[c_idx])
        } else {
            format!("C{}", c_idx)
        };

        curves.push(CartesianCurve {
            points,
            c_angle: ldt.c_angles.get(c_idx).copied().unwrap_or(0.0),
            color,
            label,
        });
    }

    curves
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::field_reassign_with_default)]
    fn create_test_ldt() -> Eulumdat {
        let mut ldt = Eulumdat::default();
        ldt.c_angles = vec![0.0, 90.0, 180.0, 270.0];
        ldt.g_angles = vec![0.0, 15.0, 30.0, 45.0, 60.0, 75.0, 90.0];
        ldt.intensities = vec![
            vec![100.0, 95.0, 85.0, 70.0, 50.0, 25.0, 10.0],
            vec![90.0, 88.0, 80.0, 65.0, 45.0, 22.0, 8.0],
            vec![100.0, 95.0, 85.0, 70.0, 50.0, 25.0, 10.0],
            vec![90.0, 88.0, 80.0, 65.0, 45.0, 22.0, 8.0],
        ];
        ldt
    }

    #[test]
    fn test_cartesian_diagram_generation() {
        let ldt = create_test_ldt();
        let diagram = CartesianDiagram::from_eulumdat(&ldt, 500.0, 380.0, 8);

        // Should have curves for each C-plane
        assert_eq!(diagram.curves.len(), 4);

        // Each curve should have points for each gamma angle
        for curve in &diagram.curves {
            assert_eq!(curve.points.len(), ldt.g_angles.len());
        }

        // Should have tick values
        assert!(!diagram.x_ticks.is_empty());
        assert!(!diagram.y_ticks.is_empty());
    }

    #[test]
    fn test_cartesian_curve_to_svg() {
        let ldt = create_test_ldt();
        let diagram = CartesianDiagram::from_eulumdat(&ldt, 500.0, 380.0, 8);

        let path = diagram.curves[0].to_svg_path();
        assert!(path.starts_with("M "));
        assert!(!path.ends_with(" Z")); // Cartesian curves are open paths
    }

    #[test]
    fn test_max_curves_limit() {
        let ldt = create_test_ldt();
        let diagram = CartesianDiagram::from_eulumdat(&ldt, 500.0, 380.0, 2);

        // Should only have 2 curves despite 4 C-planes
        assert_eq!(diagram.curves.len(), 2);
    }

    #[test]
    fn test_nice_step() {
        assert!((DiagramScale::nice_step(100.0, 5) - 20.0).abs() < 0.01);
        assert!((DiagramScale::nice_step(47.0, 5) - 10.0).abs() < 0.01);
        assert!((DiagramScale::nice_step(1000.0, 5) - 200.0).abs() < 0.01);
    }
}
