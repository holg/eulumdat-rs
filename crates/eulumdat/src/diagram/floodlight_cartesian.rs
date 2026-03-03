//! Floodlight V-H Cartesian diagram
//!
//! Cartesian plot in Type B coordinates showing intensity vs angle.
//! X axis: angle (-90° to +90°), Y axis: intensity (cd/klm).
//! Two curves: H-plane (V=0, H varying) and V-plane (H=0, V varying).
//! Supports linear and logarithmic Y-axis scaling.

use super::color::Color;
use super::DiagramScale;
use crate::type_b_conversion::TypeBConversion;
use crate::Eulumdat;

/// Y-axis scale mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum YScale {
    #[default]
    Linear,
    Logarithmic,
}

/// A point on a floodlight curve
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FloodlightPoint {
    /// Screen X coordinate
    pub x: f64,
    /// Screen Y coordinate
    pub y: f64,
    /// Angle in degrees (H or V depending on curve)
    pub angle: f64,
    /// Intensity in cd/klm
    pub intensity: f64,
}

/// A curve in the floodlight diagram
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FloodlightCurve {
    /// Points on the curve
    pub points: Vec<FloodlightPoint>,
    /// Label (e.g., "H-plane" or "V-plane")
    pub label: String,
    /// Color for rendering
    pub color: Color,
}

impl FloodlightCurve {
    /// Convert to SVG path string
    pub fn to_svg_path(&self) -> String {
        if self.points.is_empty() {
            return String::new();
        }
        let mut path = String::new();
        for (i, pt) in self.points.iter().enumerate() {
            if i == 0 {
                path.push_str(&format!("M {:.1} {:.1}", pt.x, pt.y));
            } else {
                path.push_str(&format!(" L {:.1} {:.1}", pt.x, pt.y));
            }
        }
        path
    }
}

/// Floodlight V-H Cartesian diagram data
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FloodlightCartesianDiagram {
    /// H-plane curve (V=0, H varying)
    pub h_curve: FloodlightCurve,
    /// V-plane curve (H=0, V varying)
    pub v_curve: FloodlightCurve,
    /// Y scale mode
    pub y_scale: YScale,
    /// Scale information
    pub scale: DiagramScale,
    /// Maximum intensity in the data
    pub i_max: f64,
    /// X-axis tick values (angles)
    pub x_ticks: Vec<f64>,
    /// Y-axis tick values (intensity)
    pub y_ticks: Vec<f64>,
    /// Plot dimensions
    pub plot_width: f64,
    pub plot_height: f64,
    pub margin_left: f64,
    pub margin_top: f64,
}

impl FloodlightCartesianDiagram {
    /// Generate from Eulumdat data.
    ///
    /// # Arguments
    /// * `ldt` - The Eulumdat data
    /// * `width` - Total SVG width
    /// * `height` - Total SVG height
    /// * `y_scale` - Linear or logarithmic Y axis
    pub fn from_eulumdat(ldt: &Eulumdat, width: f64, height: f64, y_scale: YScale) -> Self {
        let margin_left = 70.0;
        let margin_right = 30.0;
        let margin_top = 40.0;
        let margin_bottom = 55.0;

        let plot_width = width - margin_left - margin_right;
        let plot_height = height - margin_top - margin_bottom;

        // Sample at 1° increments from -90 to +90
        let angles: Vec<f64> = (-90..=90).map(|a| a as f64).collect();

        // Collect intensities for both planes
        let mut h_intensities = Vec::new();
        let mut v_intensities = Vec::new();
        let mut i_max: f64 = 0.0;

        for &angle in &angles {
            let h_i = TypeBConversion::intensity_at_type_b(ldt, angle, 0.0);
            let v_i = TypeBConversion::intensity_at_type_b(ldt, 0.0, angle);
            h_intensities.push(h_i);
            v_intensities.push(v_i);
            if h_i > i_max {
                i_max = h_i;
            }
            if v_i > i_max {
                i_max = v_i;
            }
        }

        // Calculate Y-axis ticks
        let (y_ticks, y_max, y_min_log) = match y_scale {
            YScale::Linear => {
                let step = DiagramScale::nice_step(i_max, 5);
                let mut ticks = Vec::new();
                let mut v = 0.0;
                while v <= i_max * 1.05 {
                    ticks.push(v);
                    v += step;
                }
                let y_max = ticks.last().copied().unwrap_or(100.0);
                (ticks, y_max, 1.0)
            }
            YScale::Logarithmic => {
                // Log scale: powers of 10
                let i_min = i_max * 0.001; // 3 decades below max
                let log_min = i_min.max(0.1).log10().floor() as i32;
                let log_max = i_max.max(1.0).log10().ceil() as i32;
                let ticks: Vec<f64> = (log_min..=log_max).map(|e| 10.0_f64.powi(e)).collect();
                let y_max_val = 10.0_f64.powi(log_max);
                let y_min_val = 10.0_f64.powi(log_min);
                (ticks, y_max_val, y_min_val)
            }
        };

        // X ticks
        let x_ticks = vec![-90.0, -60.0, -30.0, 0.0, 30.0, 60.0, 90.0];

        let scale = DiagramScale {
            max_intensity: i_max,
            scale_max: y_max,
            grid_values: y_ticks.clone(),
        };

        // Map data to screen coordinates
        let map_x = |angle: f64| -> f64 { margin_left + plot_width * ((angle + 90.0) / 180.0) };

        let map_y = |intensity: f64| -> f64 {
            match y_scale {
                YScale::Linear => {
                    if y_max > 0.0 {
                        margin_top + plot_height * (1.0 - intensity / y_max)
                    } else {
                        margin_top + plot_height
                    }
                }
                YScale::Logarithmic => {
                    let i_clamped = intensity.max(y_min_log);
                    let log_range = y_max.log10() - y_min_log.log10();
                    if log_range > 0.0 {
                        let normalized = (i_clamped.log10() - y_min_log.log10()) / log_range;
                        margin_top + plot_height * (1.0 - normalized)
                    } else {
                        margin_top + plot_height
                    }
                }
            }
        };

        let h_points: Vec<FloodlightPoint> = angles
            .iter()
            .zip(h_intensities.iter())
            .map(|(&angle, &intensity)| FloodlightPoint {
                x: map_x(angle),
                y: map_y(intensity),
                angle,
                intensity,
            })
            .collect();

        let v_points: Vec<FloodlightPoint> = angles
            .iter()
            .zip(v_intensities.iter())
            .map(|(&angle, &intensity)| FloodlightPoint {
                x: map_x(angle),
                y: map_y(intensity),
                angle,
                intensity,
            })
            .collect();

        let h_curve = FloodlightCurve {
            points: h_points,
            label: "H-plane (V=0)".to_string(),
            color: Color::new(59, 130, 246), // Blue
        };

        let v_curve = FloodlightCurve {
            points: v_points,
            label: "V-plane (H=0)".to_string(),
            color: Color::new(239, 68, 68), // Red
        };

        Self {
            h_curve,
            v_curve,
            y_scale,
            scale,
            i_max,
            x_ticks,
            y_ticks,
            plot_width,
            plot_height,
            margin_left,
            margin_top,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LampSet;

    fn create_test_ldt() -> Eulumdat {
        Eulumdat {
            c_angles: vec![0.0, 90.0, 180.0, 270.0],
            g_angles: vec![0.0, 15.0, 30.0, 45.0, 60.0, 75.0, 90.0],
            intensities: vec![
                vec![200.0, 180.0, 140.0, 90.0, 40.0, 10.0, 2.0],
                vec![200.0, 170.0, 120.0, 70.0, 30.0, 8.0, 1.0],
                vec![200.0, 180.0, 140.0, 90.0, 40.0, 10.0, 2.0],
                vec![200.0, 170.0, 120.0, 70.0, 30.0, 8.0, 1.0],
            ],
            lamp_sets: vec![LampSet {
                num_lamps: 1,
                total_luminous_flux: 10000.0,
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn test_floodlight_diagram_generation() {
        let ldt = create_test_ldt();
        let diagram = FloodlightCartesianDiagram::from_eulumdat(&ldt, 600.0, 400.0, YScale::Linear);

        assert!(!diagram.h_curve.points.is_empty());
        assert!(!diagram.v_curve.points.is_empty());
        assert_eq!(diagram.h_curve.points.len(), 181); // -90 to +90 at 1° step
        assert!(diagram.i_max > 0.0);
    }

    #[test]
    fn test_log_scale() {
        let ldt = create_test_ldt();
        let diagram =
            FloodlightCartesianDiagram::from_eulumdat(&ldt, 600.0, 400.0, YScale::Logarithmic);

        assert_eq!(diagram.y_scale, YScale::Logarithmic);
        // Y ticks should be powers of 10
        for tick in &diagram.y_ticks {
            let log_val = tick.log10();
            assert!(
                (log_val - log_val.round()).abs() < 1e-10,
                "Log tick {} is not a power of 10",
                tick
            );
        }
    }

    #[test]
    fn test_svg_path_generation() {
        let ldt = create_test_ldt();
        let diagram = FloodlightCartesianDiagram::from_eulumdat(&ldt, 600.0, 400.0, YScale::Linear);

        let h_path = diagram.h_curve.to_svg_path();
        assert!(h_path.starts_with("M "));
        assert!(h_path.contains("L "));
    }
}
