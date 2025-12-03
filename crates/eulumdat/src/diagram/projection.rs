//! Projection utilities for 3D diagram rendering
//!
//! Provides isometric and other projection methods for converting
//! 3D photometric data to 2D screen coordinates.

use super::Point2D;

/// Isometric projection for 3D butterfly diagrams
///
/// Projects spherical coordinates (C-angle, gamma, intensity) to 2D screen space.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IsometricProjection {
    /// Tilt angle from vertical (viewing angle from above) in radians
    pub tilt: f64,
    /// Center X coordinate in output space
    pub center_x: f64,
    /// Center Y coordinate in output space
    pub center_y: f64,
    /// Scale factor for intensity to radius conversion
    pub scale: f64,
}

impl IsometricProjection {
    /// Create a new isometric projection
    ///
    /// # Arguments
    /// * `tilt_degrees` - Viewing angle from above (60° gives a good 3D effect)
    /// * `center_x` - Center X in output coordinates
    /// * `center_y` - Center Y in output coordinates
    /// * `max_intensity` - Maximum intensity value for scaling
    /// * `max_radius` - Maximum radius in output units
    pub fn new(
        tilt_degrees: f64,
        center_x: f64,
        center_y: f64,
        max_intensity: f64,
        max_radius: f64,
    ) -> Self {
        Self {
            tilt: tilt_degrees.to_radians(),
            center_x,
            center_y,
            scale: if max_radius > 0.0 {
                max_intensity / max_radius
            } else {
                1.0
            },
        }
    }

    /// Project a 3D point to 2D screen coordinates
    ///
    /// # Arguments
    /// * `c_angle` - C-plane angle in degrees (0-360°)
    /// * `g_angle` - Gamma angle in degrees (0-180°, 0=nadir, 90=horizontal, 180=zenith)
    /// * `intensity` - Luminous intensity value
    ///
    /// # Returns
    /// 2D point in screen coordinates
    pub fn project(&self, c_angle: f64, g_angle: f64, intensity: f64) -> Point2D {
        let c_rad = c_angle.to_radians();
        let g_rad = g_angle.to_radians();

        // Convert spherical to cartesian (lighting coordinates)
        // In lighting: gamma=0 is nadir (down), gamma=90 is horizontal, gamma=180 is zenith (up)
        let r = intensity / self.scale;

        // 3D coordinates where:
        // - X axis points to C0 direction
        // - Y axis points to C90 direction
        // - Z axis points up (zenith)
        let x3d = r * g_rad.sin() * c_rad.cos();
        let y3d = r * g_rad.sin() * c_rad.sin();
        let z3d = r * g_rad.cos(); // gamma=0 -> z=r (down), gamma=180 -> z=-r (up)

        // Isometric projection with tilt
        // Looking from above at an angle
        let cos_tilt = self.tilt.cos();
        let sin_tilt = self.tilt.sin();

        // Screen coordinates
        let sx = self.center_x + x3d;
        let sy = self.center_y - y3d * cos_tilt - z3d * sin_tilt;

        Point2D::new(sx, sy)
    }

    /// Project a point at the horizontal plane (gamma=90°) for grid drawing
    pub fn project_horizontal(&self, c_angle: f64, radius: f64) -> Point2D {
        self.project(c_angle, 90.0, radius * self.scale)
    }

    /// Get the Y-compression factor for ellipse drawing at horizontal plane
    pub fn horizontal_compression(&self) -> f64 {
        self.tilt.cos()
    }
}

/// Convert polar coordinates (angle, radius) to cartesian
///
/// # Arguments
/// * `angle_degrees` - Angle in degrees (0 = right, 90 = up)
/// * `radius` - Distance from origin
#[allow(dead_code)]
pub fn polar_to_cartesian(angle_degrees: f64, radius: f64) -> Point2D {
    let rad = angle_degrees.to_radians();
    Point2D::new(radius * rad.cos(), radius * rad.sin())
}

/// Convert gamma angle to polar diagram Y coordinate
///
/// In Eulumdat polar diagrams:
/// - gamma=0 is at center (nadir)
/// - gamma=90 is horizontal (sides)
/// - gamma=180 is at top (zenith)
#[allow(dead_code)]
pub fn gamma_to_polar_y(gamma_degrees: f64, center_y: f64, radius: f64) -> f64 {
    let gamma_rad = gamma_degrees.to_radians();
    center_y - radius * gamma_rad.sin()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isometric_projection() {
        let proj = IsometricProjection::new(60.0, 250.0, 250.0, 100.0, 200.0);

        // Point at center (gamma=0, intensity=0)
        let center = proj.project(0.0, 0.0, 0.0);
        assert!((center.x - 250.0).abs() < 0.01);

        // Point at C0, gamma=90 (horizontal, right side)
        let right = proj.project(0.0, 90.0, 100.0);
        assert!(right.x > 250.0); // Should be to the right

        // Point at C180, gamma=90 (horizontal, left side)
        let left = proj.project(180.0, 90.0, 100.0);
        assert!(left.x < 250.0); // Should be to the left
    }

    #[test]
    fn test_polar_to_cartesian() {
        let p0 = polar_to_cartesian(0.0, 100.0);
        assert!((p0.x - 100.0).abs() < 0.01);
        assert!(p0.y.abs() < 0.01);

        let p90 = polar_to_cartesian(90.0, 100.0);
        assert!(p90.x.abs() < 0.01);
        assert!((p90.y - 100.0).abs() < 0.01);
    }
}
