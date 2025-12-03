//! Polar diagram data generation
//!
//! Generates the vectors and data needed for traditional polar intensity diagrams
//! showing C0-C180 and C90-C270 planes.

use super::{DiagramScale, Point2D};
use crate::{Eulumdat, Symmetry};
use std::f64::consts::FRAC_PI_2;

/// A point in a polar curve
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PolarPoint {
    /// X coordinate (intensity-weighted)
    pub x: f64,
    /// Y coordinate (intensity-weighted)
    pub y: f64,
    /// Original gamma angle in degrees
    pub gamma: f64,
    /// Intensity value at this point
    pub intensity: f64,
}

/// A polar curve for one half of the diagram
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PolarCurve {
    /// Points in the curve
    pub points: Vec<PolarPoint>,
    /// C-plane angle this curve represents
    pub c_angle: f64,
    /// Label for this curve (e.g., "C0-C180")
    pub label: String,
}

impl PolarCurve {
    /// Check if the curve has valid data
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Get maximum intensity in this curve
    pub fn max_intensity(&self) -> f64 {
        self.points.iter().map(|p| p.intensity).fold(0.0, f64::max)
    }

    /// Convert to SVG path string
    ///
    /// # Arguments
    /// * `center_x` - Center X coordinate
    /// * `center_y` - Center Y coordinate
    /// * `scale` - Scale factor (intensity units per pixel)
    pub fn to_svg_path(&self, center_x: f64, center_y: f64, scale: f64) -> String {
        if self.points.is_empty() {
            return String::new();
        }

        let mut path = String::new();

        for (i, point) in self.points.iter().enumerate() {
            let sx = center_x + point.x / scale;
            let sy = center_y + point.y / scale;

            if i == 0 {
                path.push_str(&format!("M {:.1} {:.1}", sx, sy));
            } else {
                path.push_str(&format!(" L {:.1} {:.1}", sx, sy));
            }
        }

        path.push_str(" Z");
        path
    }

    /// Get screen coordinates for all points
    ///
    /// # Arguments
    /// * `center_x` - Center X coordinate
    /// * `center_y` - Center Y coordinate
    /// * `scale` - Scale factor (intensity units per pixel)
    pub fn screen_points(&self, center_x: f64, center_y: f64, scale: f64) -> Vec<Point2D> {
        self.points
            .iter()
            .map(|p| Point2D::new(center_x + p.x / scale, center_y + p.y / scale))
            .collect()
    }
}

/// Complete polar diagram data
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PolarDiagram {
    /// Curve for the C0-C180 plane (typically shown as solid blue)
    pub c0_c180_curve: PolarCurve,
    /// Curve for the C90-C270 plane (typically shown as dashed red)
    pub c90_c270_curve: PolarCurve,
    /// Scale information
    pub scale: DiagramScale,
    /// Symmetry type of the source data
    pub symmetry: Symmetry,
}

impl PolarDiagram {
    /// Generate polar diagram data from Eulumdat
    pub fn from_eulumdat(ldt: &Eulumdat) -> Self {
        let (c0_c180_points, c90_c270_points, max_intensity) = calculate_vectors(ldt);

        let c0_c180_curve = PolarCurve {
            points: c0_c180_points,
            c_angle: 0.0,
            label: "C0-C180".to_string(),
        };

        let c90_c270_curve = PolarCurve {
            points: c90_c270_points,
            c_angle: 90.0,
            label: "C90-C270".to_string(),
        };

        let scale = DiagramScale::from_max_intensity(max_intensity, 5);

        Self {
            c0_c180_curve,
            c90_c270_curve,
            scale,
            symmetry: ldt.symmetry,
        }
    }

    /// Check if the C90-C270 curve should be displayed
    ///
    /// For rotationally symmetric luminaires (symmetry 1), the C90-C270 curve
    /// is identical to C0-C180 and shouldn't be shown separately.
    pub fn show_c90_c270(&self) -> bool {
        self.symmetry != Symmetry::VerticalAxis && !self.c90_c270_curve.is_empty()
    }
}

/// Calculate the polar vectors for C0-C180 and C90-C270 planes
///
/// This is the core algorithm extracted from the WASM component.
/// It handles all symmetry types correctly.
fn calculate_vectors(ldt: &Eulumdat) -> (Vec<PolarPoint>, Vec<PolarPoint>, f64) {
    if ldt.intensities.is_empty() || ldt.g_angles.is_empty() {
        return (Vec::new(), Vec::new(), 0.0);
    }

    let mut vector_c0_c180 = Vec::new();
    let mut vector_c90_c270 = Vec::new();
    let mut max_intensity: f64 = 0.0;

    // For symmetry 3 (C90-C270), intensities start at C90 rather than C0
    let c_start = match ldt.symmetry {
        Symmetry::PlaneC90C270 => ldt.c_angles.iter().position(|&c| c >= 90.0).unwrap_or(0),
        _ => 0,
    };

    // === C0-C180 plane ===
    let c0_idx = if ldt.symmetry == Symmetry::PlaneC90C270 {
        // For symmetry 3, find the C180 plane (which represents C0 reflected)
        let mc = ldt.intensities.len();
        (0..mc)
            .find(|&i| {
                ldt.c_angles
                    .get(c_start + i)
                    .is_some_and(|&a| (a - 180.0).abs() < 0.1)
            })
            .unwrap_or(mc / 2)
    } else {
        0
    };

    if c0_idx < ldt.intensities.len() {
        let intensities = &ldt.intensities[c0_idx];

        // Build C0 side (right side of diagram)
        for (j, &g_angle) in ldt.g_angles.iter().enumerate() {
            let intensity = intensities.get(j).copied().unwrap_or(0.0);
            max_intensity = max_intensity.max(intensity);

            // Convert gamma angle to polar coordinates
            // gamma=0 is center (nadir), gamma=90 is horizontal, gamma=180 is top
            let angle_rad = -g_angle.to_radians() + FRAC_PI_2;
            let x = intensity * angle_rad.cos();
            let y = intensity * angle_rad.sin();

            vector_c0_c180.push(PolarPoint {
                x,
                y,
                gamma: g_angle,
                intensity,
            });
        }

        // Build C180 side (left side of diagram)
        if matches!(
            ldt.symmetry,
            Symmetry::VerticalAxis | Symmetry::PlaneC90C270 | Symmetry::BothPlanes
        ) {
            // Mirror the C0 data
            for j in (0..ldt.g_angles.len()).rev() {
                let point = &vector_c0_c180[j];
                vector_c0_c180.push(PolarPoint {
                    x: -point.x,
                    y: point.y,
                    gamma: point.gamma,
                    intensity: point.intensity,
                });
            }
        } else {
            // Use actual C180 data
            let c180_idx = (0..ldt.intensities.len()).find(|&i| {
                ldt.c_angles
                    .get(i)
                    .is_some_and(|&a| (a - 180.0).abs() < 0.1)
            });

            if let Some(idx) = c180_idx {
                let intensities_180 = &ldt.intensities[idx];
                for j in (0..ldt.g_angles.len()).rev() {
                    let g_angle = ldt.g_angles[j];
                    let intensity = intensities_180.get(j).copied().unwrap_or(0.0);
                    max_intensity = max_intensity.max(intensity);

                    let angle_rad = -g_angle.to_radians() + FRAC_PI_2;
                    let x = intensity * angle_rad.cos();
                    let y = intensity * angle_rad.sin();

                    vector_c0_c180.push(PolarPoint {
                        x: -x,
                        y,
                        gamma: g_angle,
                        intensity,
                    });
                }
            }
        }
    }

    // === C90-C270 plane ===
    if ldt.symmetry == Symmetry::VerticalAxis {
        // Rotationally symmetric - don't draw separate C90-C270 curve
    } else if ldt.symmetry == Symmetry::PlaneC90C270 {
        // C90 is the first intensity set for symmetry 3
        let intensities = &ldt.intensities[0];

        // Build C90 side
        for (j, &g_angle) in ldt.g_angles.iter().enumerate() {
            let intensity = intensities.get(j).copied().unwrap_or(0.0);
            max_intensity = max_intensity.max(intensity);

            let angle_rad = -g_angle.to_radians() + FRAC_PI_2;
            let x = intensity * angle_rad.cos();
            let y = intensity * angle_rad.sin();

            vector_c90_c270.push(PolarPoint {
                x,
                y,
                gamma: g_angle,
                intensity,
            });
        }

        // Find and use C270 data
        let mc = ldt.intensities.len();
        let c270_idx = (0..mc).find(|&i| {
            ldt.c_angles
                .get(c_start + i)
                .is_some_and(|&a| (a - 270.0).abs() < 0.1)
        });

        if let Some(idx) = c270_idx {
            let intensities_270 = &ldt.intensities[idx];
            for j in (0..ldt.g_angles.len()).rev() {
                let g_angle = ldt.g_angles[j];
                let intensity = intensities_270.get(j).copied().unwrap_or(0.0);
                max_intensity = max_intensity.max(intensity);

                let angle_rad = -g_angle.to_radians() + FRAC_PI_2;
                let x = intensity * angle_rad.cos();
                let y = intensity * angle_rad.sin();

                vector_c90_c270.push(PolarPoint {
                    x: -x,
                    y,
                    gamma: g_angle,
                    intensity,
                });
            }
        }
    } else {
        // Find C90 in the intensity data
        let c90_idx = (0..ldt.intensities.len())
            .find(|&i| ldt.c_angles.get(i).is_some_and(|&a| (a - 90.0).abs() < 0.1));

        if let Some(idx) = c90_idx {
            let intensities = &ldt.intensities[idx];

            // Build C90 side
            for (j, &g_angle) in ldt.g_angles.iter().enumerate() {
                let intensity = intensities.get(j).copied().unwrap_or(0.0);
                max_intensity = max_intensity.max(intensity);

                let angle_rad = -g_angle.to_radians() + FRAC_PI_2;
                let x = intensity * angle_rad.cos();
                let y = intensity * angle_rad.sin();

                vector_c90_c270.push(PolarPoint {
                    x,
                    y,
                    gamma: g_angle,
                    intensity,
                });
            }

            // Build C270 side
            if matches!(ldt.symmetry, Symmetry::PlaneC0C180 | Symmetry::BothPlanes) {
                // Mirror the C90 data
                for j in (0..ldt.g_angles.len()).rev() {
                    let point = &vector_c90_c270[j];
                    vector_c90_c270.push(PolarPoint {
                        x: -point.x,
                        y: point.y,
                        gamma: point.gamma,
                        intensity: point.intensity,
                    });
                }
            } else if ldt.symmetry == Symmetry::None {
                // Use actual C270 data
                let c270_idx = (0..ldt.intensities.len()).find(|&i| {
                    ldt.c_angles
                        .get(i)
                        .is_some_and(|&a| (a - 270.0).abs() < 0.1)
                });

                if let Some(idx270) = c270_idx {
                    let intensities_270 = &ldt.intensities[idx270];
                    for j in (0..ldt.g_angles.len()).rev() {
                        let g_angle = ldt.g_angles[j];
                        let intensity = intensities_270.get(j).copied().unwrap_or(0.0);
                        max_intensity = max_intensity.max(intensity);

                        let angle_rad = -g_angle.to_radians() + FRAC_PI_2;
                        let x = intensity * angle_rad.cos();
                        let y = intensity * angle_rad.sin();

                        vector_c90_c270.push(PolarPoint {
                            x: -x,
                            y,
                            gamma: g_angle,
                            intensity,
                        });
                    }
                }
            }
        }
    }

    (vector_c0_c180, vector_c90_c270, max_intensity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::field_reassign_with_default)]
    fn create_test_ldt() -> Eulumdat {
        let mut ldt = Eulumdat::default();
        ldt.symmetry = Symmetry::BothPlanes;
        ldt.c_angles = vec![0.0, 30.0, 60.0, 90.0];
        ldt.g_angles = vec![0.0, 30.0, 60.0, 90.0];
        ldt.intensities = vec![
            vec![100.0, 90.0, 70.0, 40.0], // C0
            vec![95.0, 85.0, 65.0, 35.0],  // C30
            vec![90.0, 80.0, 60.0, 30.0],  // C60
            vec![85.0, 75.0, 55.0, 25.0],  // C90
        ];
        ldt
    }

    #[test]
    fn test_polar_diagram_generation() {
        let ldt = create_test_ldt();
        let polar = PolarDiagram::from_eulumdat(&ldt);

        // Should have points
        assert!(!polar.c0_c180_curve.is_empty());

        // Scale should be set correctly
        assert!(polar.scale.max_intensity > 0.0);
        assert!(polar.scale.scale_max >= polar.scale.max_intensity);
    }

    #[test]
    fn test_polar_curve_to_svg() {
        let ldt = create_test_ldt();
        let polar = PolarDiagram::from_eulumdat(&ldt);

        let path = polar.c0_c180_curve.to_svg_path(250.0, 250.0, 1.0);
        assert!(path.starts_with("M "));
        assert!(path.ends_with(" Z"));
    }

    #[test]
    fn test_symmetry_handling() {
        let mut ldt = create_test_ldt();
        ldt.symmetry = Symmetry::VerticalAxis;

        let polar = PolarDiagram::from_eulumdat(&ldt);

        // Should not show C90-C270 for vertical axis symmetry
        assert!(!polar.show_c90_c270());
    }
}
