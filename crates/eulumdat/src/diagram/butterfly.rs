//! Butterfly diagram data generation
//!
//! Generates the data needed for 3D butterfly diagrams that show
//! light distribution from a top-down isometric view with all C-planes.

use super::color::{hsl_to_rgb, Color};
use super::{DiagramScale, IsometricProjection, Point2D};
use crate::{Eulumdat, Symmetry};

/// Data for a single C-plane: (c_angle, intensities for each gamma)
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CPlaneData {
    /// C-plane angle in degrees
    pub c_angle: f64,
    /// Intensity values for each gamma angle
    pub intensities: Vec<f64>,
}

/// A butterfly wing (one C-plane slice in the 3D diagram)
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ButterflyWing {
    /// C-plane angle this wing represents
    pub c_angle: f64,
    /// Points forming the wing outline (in 3D-projected 2D space)
    pub points: Vec<Point2D>,
    /// Fill color for this wing
    pub fill_color: Color,
    /// Stroke color for this wing
    pub stroke_color: Color,
}

impl ButterflyWing {
    /// Convert to SVG path string
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

        path.push_str(" Z");
        path
    }
}

/// Complete butterfly diagram data
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ButterflyDiagram {
    /// All butterfly wings (one per C-plane after expansion)
    pub wings: Vec<ButterflyWing>,
    /// Grid circles for the diagram (at horizontal plane)
    pub grid_circles: Vec<Vec<Point2D>>,
    /// C-plane direction lines with labels
    pub c_plane_lines: Vec<(f64, Point2D, Point2D)>,
    /// Scale information
    pub scale: DiagramScale,
    /// Expanded C-plane data (useful for other visualizations)
    pub c_plane_data: Vec<CPlaneData>,
    /// Gamma angles from source data
    pub g_angles: Vec<f64>,
    /// Symmetry type
    pub symmetry: Symmetry,
}

impl ButterflyDiagram {
    /// Generate butterfly diagram data from Eulumdat
    ///
    /// # Arguments
    /// * `ldt` - The Eulumdat data
    /// * `width` - Output width in pixels
    /// * `height` - Output height in pixels
    /// * `tilt_degrees` - Viewing angle from above (60° is typical)
    pub fn from_eulumdat(ldt: &Eulumdat, width: f64, height: f64, tilt_degrees: f64) -> Self {
        let cx = width / 2.0;
        let cy = height / 2.0 + 25.0; // Offset down slightly for labels
        let margin = 70.0;
        let max_radius = (width.min(height) / 2.0) - margin;

        // Calculate max intensity
        let max_intensity = ldt
            .intensities
            .iter()
            .flat_map(|plane| plane.iter())
            .copied()
            .fold(0.0_f64, f64::max);

        let scale = DiagramScale::from_max_intensity(max_intensity, 4);

        // Create projection
        let projection =
            IsometricProjection::new(tilt_degrees, cx, cy, scale.scale_max, max_radius);

        // Expand C-plane data based on symmetry
        let c_plane_data = expand_c_planes(ldt);

        // Generate wings
        let wings = generate_wings(&c_plane_data, &ldt.g_angles, &projection);

        // Generate grid circles
        let grid_circles = generate_grid_circles(4, scale.scale_max, &projection);

        // Generate C-plane direction lines
        let c_plane_lines = generate_c_plane_lines(max_radius, &projection);

        Self {
            wings,
            grid_circles,
            c_plane_lines,
            scale,
            c_plane_data,
            g_angles: ldt.g_angles.clone(),
            symmetry: ldt.symmetry,
        }
    }
}

/// Expand C-plane data based on symmetry type
///
/// This is the core algorithm that converts reduced symmetry data
/// back to full 360° distribution.
pub fn expand_c_planes(ldt: &Eulumdat) -> Vec<CPlaneData> {
    if ldt.intensities.is_empty() || ldt.g_angles.is_empty() {
        return Vec::new();
    }

    let mut result = Vec::new();

    // Get starting index for C-angles based on symmetry
    let c_start = match ldt.symmetry {
        Symmetry::PlaneC90C270 => ldt.c_angles.iter().position(|&c| c >= 90.0).unwrap_or(0),
        _ => 0,
    };

    match ldt.symmetry {
        Symmetry::VerticalAxis => {
            // Symmetry 1: Rotationally symmetric - single plane applies to all C-angles
            // Generate data for 8 C-planes (every 45°)
            let intensities = ldt.intensities.first().cloned().unwrap_or_default();
            for i in 0..8 {
                let c_angle = i as f64 * 45.0;
                result.push(CPlaneData {
                    c_angle,
                    intensities: intensities.clone(),
                });
            }
        }
        Symmetry::PlaneC0C180 => {
            // Symmetry 2: Symmetric about C0-C180 plane
            // Mirror C0-C90 to C270-C360
            for (i, intensities) in ldt.intensities.iter().enumerate() {
                if let Some(&c_angle) = ldt.c_angles.get(c_start + i) {
                    result.push(CPlaneData {
                        c_angle,
                        intensities: intensities.clone(),
                    });
                    // Mirror to other side
                    if c_angle > 0.0 && c_angle < 180.0 {
                        result.push(CPlaneData {
                            c_angle: 360.0 - c_angle,
                            intensities: intensities.clone(),
                        });
                    }
                }
            }
        }
        Symmetry::PlaneC90C270 => {
            // Symmetry 3: Symmetric about C90-C270 plane
            // Data covers C90 to C270, mirror to C270-C90
            for (i, intensities) in ldt.intensities.iter().enumerate() {
                if let Some(&c_angle) = ldt.c_angles.get(c_start + i) {
                    result.push(CPlaneData {
                        c_angle,
                        intensities: intensities.clone(),
                    });
                    // Mirror to other side
                    if c_angle > 90.0 && c_angle < 270.0 {
                        let mirrored = if c_angle < 180.0 {
                            90.0 - (c_angle - 90.0) // Mirror around C90
                        } else {
                            270.0 + (270.0 - c_angle) // Mirror around C270
                        };
                        if (0.0..=360.0).contains(&mirrored) {
                            result.push(CPlaneData {
                                c_angle: mirrored,
                                intensities: intensities.clone(),
                            });
                        }
                    }
                }
            }
        }
        Symmetry::BothPlanes => {
            // Symmetry 4: Symmetric about both planes (quadrant symmetry)
            // Data covers C0-C90, expand to full circle
            for (i, intensities) in ldt.intensities.iter().enumerate() {
                if let Some(&c_angle) = ldt.c_angles.get(c_start + i) {
                    result.push(CPlaneData {
                        c_angle,
                        intensities: intensities.clone(),
                    });
                    // Mirror to other quadrants
                    if c_angle > 0.0 && c_angle < 90.0 {
                        result.push(CPlaneData {
                            c_angle: 180.0 - c_angle,
                            intensities: intensities.clone(),
                        }); // Q2
                        result.push(CPlaneData {
                            c_angle: 180.0 + c_angle,
                            intensities: intensities.clone(),
                        }); // Q3
                        result.push(CPlaneData {
                            c_angle: 360.0 - c_angle,
                            intensities: intensities.clone(),
                        }); // Q4
                    } else if (c_angle - 90.0).abs() < 0.1 {
                        result.push(CPlaneData {
                            c_angle: 270.0,
                            intensities: intensities.clone(),
                        });
                    }
                }
            }
        }
        Symmetry::None => {
            // No symmetry - use data as-is
            for (i, intensities) in ldt.intensities.iter().enumerate() {
                if let Some(&c_angle) = ldt.c_angles.get(c_start + i) {
                    result.push(CPlaneData {
                        c_angle,
                        intensities: intensities.clone(),
                    });
                }
            }
        }
    }

    // Sort by C-angle
    result.sort_by(|a, b| {
        a.c_angle
            .partial_cmp(&b.c_angle)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    result
}

/// Generate butterfly wings for each C-plane
fn generate_wings(
    c_plane_data: &[CPlaneData],
    g_angles: &[f64],
    projection: &IsometricProjection,
) -> Vec<ButterflyWing> {
    if c_plane_data.is_empty() || g_angles.is_empty() {
        return Vec::new();
    }

    let mut wings = Vec::new();

    for plane in c_plane_data {
        // Generate color based on C-angle
        let hue = (plane.c_angle / 360.0) * 240.0 + 180.0; // 180-420 (cyan to blue to purple)
        let hue = hue % 360.0;

        let fill_color = hsl_to_rgb(hue / 360.0, 0.7, 0.45);
        let stroke_color = hsl_to_rgb(hue / 360.0, 0.7, 0.55);

        // Build path: start from center, go out along gamma angles
        let mut points = Vec::new();

        // Start at center
        points.push(Point2D::new(projection.center_x, projection.center_y));

        // Draw outward along increasing gamma
        for (j, &g_angle) in g_angles.iter().enumerate() {
            let intensity = plane.intensities.get(j).copied().unwrap_or(0.0);
            let p = projection.project(plane.c_angle, g_angle, intensity);
            points.push(p);
        }

        wings.push(ButterflyWing {
            c_angle: plane.c_angle,
            points,
            fill_color,
            stroke_color,
        });
    }

    wings
}

/// Generate grid circles at the horizontal plane (gamma=90°)
fn generate_grid_circles(
    num_circles: usize,
    max_scale: f64,
    projection: &IsometricProjection,
) -> Vec<Vec<Point2D>> {
    let mut circles = Vec::new();

    for i in 1..=num_circles {
        let intensity = max_scale * (i as f64) / (num_circles as f64);
        let mut points = Vec::new();

        // Draw ellipse at gamma=90 (horizontal plane)
        let num_points = 36;
        for j in 0..=num_points {
            let c_angle = (j as f64) * 360.0 / (num_points as f64);
            let p = projection.project(c_angle, 90.0, intensity);
            points.push(p);
        }

        circles.push(points);
    }

    circles
}

/// Generate C-plane direction lines at 45° intervals
fn generate_c_plane_lines(
    max_radius: f64,
    projection: &IsometricProjection,
) -> Vec<(f64, Point2D, Point2D)> {
    let label_angles: Vec<f64> = vec![0.0, 45.0, 90.0, 135.0, 180.0, 225.0, 270.0, 315.0];
    let mut lines = Vec::new();

    for &c_angle in &label_angles {
        let start = Point2D::new(projection.center_x, projection.center_y);
        let end = projection.project(c_angle, 90.0, max_radius * projection.scale);
        lines.push((c_angle, start, end));
    }

    lines
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
            vec![100.0, 90.0, 70.0, 40.0],
            vec![95.0, 85.0, 65.0, 35.0],
            vec![90.0, 80.0, 60.0, 30.0],
            vec![85.0, 75.0, 55.0, 25.0],
        ];
        ldt
    }

    #[test]
    fn test_expand_c_planes_both_symmetry() {
        let ldt = create_test_ldt();
        let expanded = expand_c_planes(&ldt);

        // With BothPlanes symmetry, C0-C90 expands to full 360°
        // C0 -> C0, C180
        // C30 -> C30, C150, C210, C330
        // C60 -> C60, C120, C240, C300
        // C90 -> C90, C270
        assert!(expanded.len() > ldt.intensities.len());

        // Check that we have angles in all quadrants
        let has_q1 = expanded.iter().any(|p| p.c_angle > 0.0 && p.c_angle < 90.0);
        let has_q2 = expanded
            .iter()
            .any(|p| p.c_angle > 90.0 && p.c_angle < 180.0);
        let has_q3 = expanded
            .iter()
            .any(|p| p.c_angle > 180.0 && p.c_angle < 270.0);
        let has_q4 = expanded
            .iter()
            .any(|p| p.c_angle > 270.0 && p.c_angle < 360.0);

        assert!(has_q1 && has_q2 && has_q3 && has_q4);
    }

    #[test]
    fn test_expand_c_planes_vertical_symmetry() {
        let mut ldt = create_test_ldt();
        ldt.symmetry = Symmetry::VerticalAxis;

        let expanded = expand_c_planes(&ldt);

        // Should generate 8 C-planes at 45° intervals
        assert_eq!(expanded.len(), 8);

        // All should have same intensities
        let first_intensities = &expanded[0].intensities;
        for plane in &expanded {
            assert_eq!(&plane.intensities, first_intensities);
        }
    }

    #[test]
    fn test_butterfly_diagram_generation() {
        let ldt = create_test_ldt();
        let diagram = ButterflyDiagram::from_eulumdat(&ldt, 500.0, 450.0, 60.0);

        // Should have wings
        assert!(!diagram.wings.is_empty());

        // Should have grid circles
        assert_eq!(diagram.grid_circles.len(), 4);

        // Should have C-plane lines
        assert_eq!(diagram.c_plane_lines.len(), 8);
    }

    #[test]
    fn test_wing_to_svg_path() {
        let wing = ButterflyWing {
            c_angle: 0.0,
            points: vec![
                Point2D::new(250.0, 250.0),
                Point2D::new(300.0, 200.0),
                Point2D::new(350.0, 150.0),
            ],
            fill_color: Color::new(100, 150, 200),
            stroke_color: Color::new(80, 130, 180),
        };

        let path = wing.to_svg_path();
        assert!(path.starts_with("M "));
        assert!(path.ends_with(" Z"));
    }
}
