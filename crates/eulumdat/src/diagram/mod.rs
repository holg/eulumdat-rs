//! Diagram data generation for photometric visualizations
//!
//! This module provides platform-independent data generation for various
//! diagram types. The actual rendering is left to platform-specific code
//! (e.g., SVG for web, Core Graphics for iOS, Canvas for Android).
//!
//! # Supported Diagram Types
//!
//! - **Polar**: Traditional polar intensity diagram showing C0-C180 and C90-C270 planes
//! - **Butterfly**: 3D butterfly diagram with isometric projection
//! - **Cartesian**: X-Y plot of intensity vs gamma angle for each C-plane
//! - **Heatmap**: 2D grid showing intensity distribution across all angles
//!
//! # Example
//!
//! ```rust,no_run
//! use eulumdat::{Eulumdat, diagram::PolarDiagram};
//!
//! let ldt = Eulumdat::from_file("luminaire.ldt").unwrap();
//! let polar = PolarDiagram::from_eulumdat(&ldt);
//!
//! // Use the generated data for rendering
//! for point in &polar.c0_c180_curve.points {
//!     println!("x: {}, y: {}", point.x, point.y);
//! }
//! ```

mod butterfly;
mod cartesian;
mod color;
mod cone;
mod heatmap;
mod labels;
mod polar;
mod projection;
mod svg;
mod watchface;

pub use butterfly::{ButterflyDiagram, ButterflyWing, CPlaneData};
pub use cartesian::{CartesianCurve, CartesianDiagram, CartesianPoint};
pub use color::{heatmap_color, hsl_to_rgb, Color, ColorPalette};
pub use cone::ConeDiagram;
pub use heatmap::{HeatmapCell, HeatmapDiagram};
pub use labels::DiagramLabels;
pub use polar::{PolarCurve, PolarDiagram, PolarPoint};
pub use projection::IsometricProjection;
pub use svg::{ConeDiagramLabels, DetailLevel, SvgLabels, SvgTheme};
pub use watchface::WatchFaceStyle;

/// Common 2D point used across diagram types
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Point2D {
    pub x: f64,
    pub y: f64,
}

impl Point2D {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Scale information for diagrams
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DiagramScale {
    /// Maximum intensity value in the data
    pub max_intensity: f64,
    /// Rounded maximum for nice scale display
    pub scale_max: f64,
    /// Grid/tick values for the scale
    pub grid_values: Vec<f64>,
}

impl DiagramScale {
    /// Create a scale with nice round numbers
    pub fn from_max_intensity(max_intensity: f64, num_divisions: usize) -> Self {
        let scale_max = if max_intensity > 0.0 {
            let step = if max_intensity > 1000.0 {
                500.0
            } else if max_intensity > 100.0 {
                50.0
            } else {
                20.0
            };
            step * (max_intensity / step).ceil()
        } else {
            100.0
        };

        let grid_values: Vec<f64> = (1..=num_divisions)
            .map(|i| scale_max * (i as f64) / (num_divisions as f64))
            .collect();

        Self {
            max_intensity,
            scale_max,
            grid_values,
        }
    }

    /// Calculate a "nice" step value for axis ticks
    pub fn nice_step(max_value: f64, target_ticks: usize) -> f64 {
        if max_value <= 0.0 || target_ticks == 0 {
            return 1.0;
        }

        let rough_step = max_value / target_ticks as f64;
        let magnitude = 10.0_f64.powf(rough_step.log10().floor());
        let residual = rough_step / magnitude;

        let nice = if residual <= 1.5 {
            1.0
        } else if residual <= 3.0 {
            2.0
        } else if residual <= 7.0 {
            5.0
        } else {
            10.0
        };

        nice * magnitude
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagram_scale() {
        // 450 is already divisible by 50, so it stays at 450
        let scale = DiagramScale::from_max_intensity(450.0, 5);
        assert_eq!(scale.scale_max, 450.0);
        assert_eq!(scale.grid_values.len(), 5);

        // 451 should round up to 500
        let scale2 = DiagramScale::from_max_intensity(451.0, 5);
        assert_eq!(scale2.scale_max, 500.0);
    }

    #[test]
    fn test_nice_step() {
        assert!((DiagramScale::nice_step(100.0, 5) - 20.0).abs() < 0.01);
        assert!((DiagramScale::nice_step(1000.0, 5) - 200.0).abs() < 0.01);
    }
}
