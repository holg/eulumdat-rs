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
use crate::{Eulumdat, Symmetry};

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
    /// Selected C-plane: None = overall (averaged), Some(angle) = specific plane
    pub selected_c_plane: Option<f64>,
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
            selected_c_plane: None,
        }
    }

    /// Generate cone diagram data for a specific C-plane
    ///
    /// Uses plane-specific beam/field angles instead of overall averages.
    pub fn from_eulumdat_for_plane(ldt: &Eulumdat, mounting_height: f64, c_plane: f64) -> Self {
        let beam_angle = PhotometricCalculations::beam_angle_for_plane(ldt, c_plane);
        let field_angle = PhotometricCalculations::field_angle_for_plane(ldt, c_plane);
        let half_beam_angle = PhotometricCalculations::half_beam_angle_for_plane(ldt, c_plane);
        let half_field_angle = PhotometricCalculations::half_field_angle_for_plane(ldt, c_plane);

        let beam_angle_c0 = PhotometricCalculations::beam_angle_for_plane(ldt, 0.0);
        let beam_angle_c90 = PhotometricCalculations::beam_angle_for_plane(ldt, 90.0);
        let is_symmetric = (beam_angle_c0 - beam_angle_c90).abs() < beam_angle.max(1.0) * 0.05;

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
            selected_c_plane: Some(c_plane),
        }
    }

    /// Check whether the luminaire has variation across C-planes.
    ///
    /// Returns `false` for rotationally symmetric luminaires (where all
    /// C-planes are identical), `true` for asymmetric luminaires where
    /// selecting a specific C-plane is meaningful.
    pub fn has_c_plane_variation(ldt: &Eulumdat) -> bool {
        !matches!(ldt.symmetry, Symmetry::VerticalAxis)
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

/// A row in the illuminance table showing beam/field diameters and illuminance at a given height.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ConeIlluminanceRow {
    /// Distance from luminaire in meters
    pub height: f64,
    /// Beam diameter at this height (meters)
    pub beam_diameter: f64,
    /// Field diameter at this height (meters)
    pub field_diameter: f64,
    /// Illuminance at nadir (directly below, lux)
    pub e_nadir: f64,
    /// Illuminance at beam edge in C0 plane (lux)
    pub e_beam_c0: f64,
    /// Illuminance at beam edge in C90 plane (lux)
    pub e_beam_c90: f64,
}

/// Multi-height illuminance table computed from photometric data.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ConeIlluminanceTable {
    /// Rows at different heights
    pub rows: Vec<ConeIlluminanceRow>,
    /// Total luminous flux used for the calculation (lumens)
    pub total_flux: f64,
}

impl ConeIlluminanceTable {
    /// Generate an illuminance table using overall beam/field angles.
    ///
    /// # Arguments
    /// * `ldt` - Eulumdat data
    /// * `step` - Height increment in meters
    /// * `max_height` - Maximum height in meters
    pub fn from_eulumdat(ldt: &Eulumdat, step: f64, max_height: f64) -> Self {
        let total_flux: f64 = ldt
            .lamp_sets
            .iter()
            .map(|ls| ls.total_luminous_flux * ls.num_lamps.unsigned_abs() as f64)
            .sum();

        let half_beam_c0 = PhotometricCalculations::half_beam_angle_for_plane(ldt, 0.0);
        let half_beam_c90 = PhotometricCalculations::half_beam_angle_for_plane(ldt, 90.0);

        let rows = Self::compute_rows(
            ldt,
            step,
            max_height,
            total_flux,
            half_beam_c0,
            half_beam_c90,
        );

        Self { rows, total_flux }
    }

    /// Generate an illuminance table for a specific C-plane.
    pub fn from_eulumdat_for_plane(
        ldt: &Eulumdat,
        step: f64,
        max_height: f64,
        c_plane: f64,
    ) -> Self {
        let total_flux: f64 = ldt
            .lamp_sets
            .iter()
            .map(|ls| ls.total_luminous_flux * ls.num_lamps.unsigned_abs() as f64)
            .sum();

        let half_beam = PhotometricCalculations::half_beam_angle_for_plane(ldt, c_plane);
        // For plane-specific, use the same half-beam for both columns
        // but use the perpendicular plane for c90
        let perp_plane = (c_plane + 90.0) % 360.0;
        let half_beam_perp = PhotometricCalculations::half_beam_angle_for_plane(ldt, perp_plane);

        let rows = Self::compute_rows(ldt, step, max_height, total_flux, half_beam, half_beam_perp);

        Self { rows, total_flux }
    }

    fn compute_rows(
        ldt: &Eulumdat,
        step: f64,
        max_height: f64,
        total_flux: f64,
        half_beam_c0_deg: f64,
        half_beam_c90_deg: f64,
    ) -> Vec<ConeIlluminanceRow> {
        if total_flux <= 0.0 {
            return Vec::new();
        }

        // Intensity in cd/klm → to get cd we multiply by (total_flux / 1000)
        let flux_scale = total_flux / 1000.0;

        let half_beam_overall = PhotometricCalculations::half_beam_angle(ldt);
        let half_field_overall = PhotometricCalculations::half_field_angle(ldt);

        let mut rows = Vec::new();
        let mut h = step;
        while h <= max_height + 0.001 {
            let beam_diameter = 2.0 * h * half_beam_overall.to_radians().tan();
            let field_diameter = 2.0 * h * half_field_overall.to_radians().tan();

            // E at nadir: I(0°) in cd/klm × flux_scale / h²
            // The cos³ factor is 1.0 at nadir (gamma=0)
            let i_nadir = ldt.sample(0.0, 0.0); // cd/klm
            let e_nadir = i_nadir * flux_scale / (h * h);

            // E at beam edge in C0: I(0, γ_beam_c0)
            let e_beam_c0 = Self::illuminance_at_angle(ldt, 0.0, half_beam_c0_deg, h, flux_scale);

            // E at beam edge in C90: I(90, γ_beam_c90)
            let e_beam_c90 =
                Self::illuminance_at_angle(ldt, 90.0, half_beam_c90_deg, h, flux_scale);

            rows.push(ConeIlluminanceRow {
                height: h,
                beam_diameter,
                field_diameter,
                e_nadir,
                e_beam_c0,
                e_beam_c90,
            });

            h += step;
        }

        rows
    }

    /// Calculate illuminance on a horizontal surface at distance h below the luminaire,
    /// at angle gamma from nadir in a given C-plane.
    ///
    /// E = I(c, γ) × cos(γ) / r²
    /// where r = h / cos(γ)  →  E = I(c, γ) × cos³(γ) / h²
    fn illuminance_at_angle(
        ldt: &Eulumdat,
        c_plane: f64,
        gamma_deg: f64,
        h: f64,
        flux_scale: f64,
    ) -> f64 {
        let gamma_rad = gamma_deg.to_radians();
        let cos_g = gamma_rad.cos();
        if cos_g <= 0.0 || h <= 0.0 {
            return 0.0;
        }
        let i = ldt.sample(c_plane, gamma_deg); // cd/klm
        i * flux_scale * cos_g * cos_g * cos_g / (h * h)
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

    #[test]
    fn test_from_eulumdat_for_plane() {
        let ldt = create_test_ldt();
        let cone = ConeDiagram::from_eulumdat_for_plane(&ldt, 3.0, 0.0);

        assert_eq!(cone.selected_c_plane, Some(0.0));
        assert!(cone.beam_angle >= 0.0);
        assert!(cone.field_angle >= 0.0);
        assert_eq!(cone.mounting_height, 3.0);
    }

    #[test]
    fn test_has_c_plane_variation() {
        let ldt = create_test_ldt();
        // VerticalAxis → no variation
        assert!(!ConeDiagram::has_c_plane_variation(&ldt));

        let mut asym_ldt = ldt.clone();
        asym_ldt.symmetry = crate::Symmetry::None;
        assert!(ConeDiagram::has_c_plane_variation(&asym_ldt));
    }

    #[test]
    fn test_overall_cone_has_no_selected_plane() {
        let ldt = create_test_ldt();
        let cone = ConeDiagram::from_eulumdat(&ldt, 3.0);
        assert_eq!(cone.selected_c_plane, None);
    }

    fn create_test_ldt_with_flux() -> Eulumdat {
        use crate::LampSet;
        Eulumdat {
            symmetry: crate::Symmetry::VerticalAxis,
            c_angles: vec![0.0],
            g_angles: vec![0.0, 15.0, 30.0, 45.0, 60.0, 75.0, 90.0],
            intensities: vec![vec![100.0, 95.0, 80.0, 50.0, 20.0, 5.0, 0.0]],
            luminaire_name: "Test Downlight".to_string(),
            lamp_sets: vec![LampSet {
                num_lamps: 1,
                total_luminous_flux: 1000.0,
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn test_illuminance_inverse_square() {
        let ldt = create_test_ldt_with_flux();
        let table = ConeIlluminanceTable::from_eulumdat(&ldt, 1.0, 4.0);

        assert!(!table.rows.is_empty());
        // Nadir illuminance at 1m vs 2m should follow inverse-square: E(2m) ≈ E(1m)/4
        let e1 = table.rows[0].e_nadir;
        let e2 = table.rows[1].e_nadir;
        assert!(e1 > 0.0, "illuminance at 1m should be positive");
        assert!(
            (e2 - e1 / 4.0).abs() < 0.01,
            "should follow inverse-square law"
        );
    }

    #[test]
    fn test_illuminance_beam_edge_less_than_nadir() {
        let ldt = create_test_ldt_with_flux();
        let table = ConeIlluminanceTable::from_eulumdat(&ldt, 1.0, 3.0);

        for row in &table.rows {
            assert!(
                row.e_beam_c0 <= row.e_nadir,
                "beam edge illuminance should be <= nadir (h={})",
                row.height
            );
        }
    }

    #[test]
    fn test_illuminance_zero_flux() {
        let mut ldt = create_test_ldt();
        // No lamp sets → zero flux
        let table = ConeIlluminanceTable::from_eulumdat(&ldt, 1.0, 3.0);
        assert!(table.rows.is_empty(), "zero flux should produce no rows");
        assert_eq!(table.total_flux, 0.0);

        // Explicit zero flux
        ldt.lamp_sets.push(crate::LampSet {
            num_lamps: 1,
            total_luminous_flux: 0.0,
            ..Default::default()
        });
        let table2 = ConeIlluminanceTable::from_eulumdat(&ldt, 1.0, 3.0);
        assert!(table2.rows.is_empty());
    }
}
