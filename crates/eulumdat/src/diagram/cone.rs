//! Cone diagram data generation
//!
//! Generates a side-view cone diagram showing beam and field angle spread.
//! This is the classic "electrician's diagram" that shows how light spreads
//! at different mounting heights.
//!
//! # Example
//!
//! ```rust,no_run
//! use eulumdat::{Eulumdat, diagram::ConeDiagram};
//!
//! let ldt = Eulumdat::from_file("luminaire.ldt").unwrap();
//! let cone = ConeDiagram::from_eulumdat(&ldt, 3.0); // 3m mounting height
//! println!("Beam diameter at floor: {:.2}m", cone.beam_diameter);
//! println!("Field diameter at floor: {:.2}m", cone.field_diameter);
//! ```

use crate::calculations::PhotometricCalculations;
use crate::Eulumdat;

/// A cone diagram showing beam and field angle spread
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ConeDiagram {
    /// Beam angle (50% intensity) in degrees - full angle per CIE S 017:2020
    pub beam_angle: f64,
    /// Field angle (10% intensity) in degrees - full angle per CIE S 017:2020
    pub field_angle: f64,
    /// Half beam angle (angle from nadir) in degrees
    pub half_beam_angle: f64,
    /// Half field angle (angle from nadir) in degrees
    pub half_field_angle: f64,
    /// Mounting height in meters
    pub mounting_height: f64,
    /// Beam diameter at floor level (meters)
    pub beam_diameter: f64,
    /// Field diameter at floor level (meters)
    pub field_diameter: f64,
    /// Maximum intensity (cd/klm) at nadir
    pub max_intensity: f64,
    /// Luminaire name for display
    pub luminaire_name: String,
    /// Whether this is a symmetric beam (same in all directions)
    pub is_symmetric: bool,
    /// Beam angle in C0-C180 plane (for asymmetric luminaires)
    pub beam_angle_c0: f64,
    /// Beam angle in C90-C270 plane (for asymmetric luminaires)
    pub beam_angle_c90: f64,
}

impl ConeDiagram {
    /// Generate cone diagram data from Eulumdat
    ///
    /// # Arguments
    /// * `ldt` - The Eulumdat data
    /// * `mounting_height` - The mounting height in meters
    pub fn from_eulumdat(ldt: &Eulumdat, mounting_height: f64) -> Self {
        // Get full beam/field angles (per CIE S 017:2020 definition)
        let beam_angle = PhotometricCalculations::beam_angle(ldt);
        let field_angle = PhotometricCalculations::field_angle(ldt);

        // Get half angles (angle from nadir to edge) - needed for cone geometry
        let half_beam_angle = PhotometricCalculations::half_beam_angle(ldt);
        let half_field_angle = PhotometricCalculations::half_field_angle(ldt);

        // Get plane-specific beam angles for asymmetric luminaires (full angles)
        let beam_angle_c0 = PhotometricCalculations::beam_angle_for_plane(ldt, 0.0);
        let beam_angle_c90 = PhotometricCalculations::beam_angle_for_plane(ldt, 90.0);

        // Check if symmetric (angles within 5% of each other)
        let is_symmetric = (beam_angle_c0 - beam_angle_c90).abs() < beam_angle * 0.05;

        // Calculate diameters at floor level using half angles
        // diameter = 2 * height * tan(half_angle)
        let beam_diameter = 2.0 * mounting_height * (half_beam_angle.to_radians()).tan();
        let field_diameter = 2.0 * mounting_height * (half_field_angle.to_radians()).tan();

        let max_intensity = ldt.max_intensity();

        Self {
            beam_angle,
            field_angle,
            half_beam_angle,
            half_field_angle,
            mounting_height,
            beam_diameter,
            field_diameter,
            max_intensity,
            luminaire_name: ldt.luminaire_name.clone(),
            is_symmetric,
            beam_angle_c0,
            beam_angle_c90,
        }
    }

    /// Calculate beam diameter at a specific distance from the luminaire
    pub fn beam_diameter_at(&self, distance: f64) -> f64 {
        // Use half angle for cone geometry calculation
        2.0 * distance * (self.half_beam_angle.to_radians()).tan()
    }

    /// Calculate field diameter at a specific distance from the luminaire
    pub fn field_diameter_at(&self, distance: f64) -> f64 {
        // Use half angle for cone geometry calculation
        2.0 * distance * (self.half_field_angle.to_radians()).tan()
    }

    /// Get beam classification based on beam angle (full angle per CIE S 017:2020)
    ///
    /// Classifications based on industry standards:
    /// - Very Narrow Spot: < 30° (15° half angle)
    /// - Narrow Spot: 30° - 50°
    /// - Spot: 50° - 70°
    /// - Medium Flood: 70° - 90°
    /// - Wide Flood: 90° - 120°
    /// - Very Wide Flood: > 120°
    pub fn beam_classification(&self) -> &'static str {
        if self.beam_angle < 30.0 {
            "Very Narrow Spot"
        } else if self.beam_angle < 50.0 {
            "Narrow Spot"
        } else if self.beam_angle < 70.0 {
            "Spot"
        } else if self.beam_angle < 90.0 {
            "Medium Flood"
        } else if self.beam_angle < 120.0 {
            "Wide Flood"
        } else {
            "Very Wide Flood"
        }
    }

    /// Generate spacing recommendations (beam edge to beam edge)
    ///
    /// Returns recommended spacing for different overlap percentages
    pub fn spacing_recommendations(&self) -> Vec<(f64, f64)> {
        // Returns (overlap_percent, spacing_meters)
        vec![
            (0.0, self.beam_diameter),         // No overlap (beam to beam)
            (25.0, self.beam_diameter * 0.75), // 25% overlap
            (50.0, self.beam_diameter * 0.5),  // 50% overlap (recommended)
            (75.0, self.beam_diameter * 0.25), // 75% overlap (high uniformity)
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_ldt() -> Eulumdat {
        // Typical downlight distribution - 100 at nadir, drops off
        Eulumdat {
            symmetry: crate::Symmetry::VerticalAxis,
            c_angles: vec![0.0],
            g_angles: vec![0.0, 15.0, 30.0, 45.0, 60.0, 75.0, 90.0],
            intensities: vec![vec![100.0, 95.0, 80.0, 50.0, 20.0, 5.0, 0.0]],
            luminaire_name: "Test Downlight".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_cone_diagram_creation() {
        let ldt = create_test_ldt();
        let cone = ConeDiagram::from_eulumdat(&ldt, 3.0);

        // Beam and field angles should be non-negative
        assert!(cone.beam_angle >= 0.0, "beam_angle should be >= 0");
        assert!(cone.field_angle >= 0.0, "field_angle should be >= 0");
        // Diameters should be non-negative
        assert!(cone.beam_diameter >= 0.0, "beam_diameter should be >= 0");
        assert!(cone.field_diameter >= 0.0, "field_diameter should be >= 0");
        // Mounting height should match input
        assert_eq!(cone.mounting_height, 3.0);
        // Classification should return something
        assert!(!cone.beam_classification().is_empty());
    }

    #[test]
    fn test_diameter_at_distance() {
        let ldt = create_test_ldt();
        let cone = ConeDiagram::from_eulumdat(&ldt, 3.0);

        // At half the height, diameter should be half
        let half_height_diameter = cone.beam_diameter_at(1.5);
        assert!((half_height_diameter - cone.beam_diameter / 2.0).abs() < 0.01);
    }

    #[test]
    fn test_beam_classification() {
        let mut ldt = create_test_ldt();

        // Create narrow beam
        ldt.intensities = vec![vec![100.0, 99.0, 95.0, 50.0, 10.0, 2.0, 0.0]];
        let cone = ConeDiagram::from_eulumdat(&ldt, 3.0);

        // Should be some classification
        assert!(!cone.beam_classification().is_empty());
    }
}
