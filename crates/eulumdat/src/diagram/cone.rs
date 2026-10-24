//! Cone diagram data generation
//!
//! Generates a side-view cone diagram showing beam and field angle spread.
//! This is the classic "electrician's diagram" that shows how light spreads
//! at different mounting heights.
//!
//! The cone is drawn in one vertical section C–(C+180): the right-hand side is
//! the C half-plane, the left-hand side the C+180 half-plane. Beam (field)
//! edges are where the intensity falls to 50 % (10 %) of the section peak
//! (see [`PhotometricCalculations::section_beam_field`]), so an asymmetric
//! lobe gives an asymmetric cone.
//!
//! # Example
//!
//! ```rust,no_run
//! use eulumdat::{Eulumdat, diagram::ConeDiagram};
//!
//! let ldt = Eulumdat::from_file("luminaire.ldt").unwrap();
//! let cone = ConeDiagram::from_eulumdat(&ldt, 3.0); // 3m mounting height
//! if let Some(d) = cone.beam_diameter {
//!     println!("Beam diameter at floor: {:.2}m", d);
//! }
//! ```

use crate::calculations::{PhotometricCalculations, SectionBeamField};
use crate::{Eulumdat, Symmetry};

/// A cone diagram showing beam and field angle spread
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ConeDiagram {
    /// Beam angle: full width at 50 % of the section peak (degrees);
    /// `None` when the lobe does not drop below 50 % on both sides
    pub beam_angle: Option<f64>,
    /// Field angle: full width at 10 % of the section peak (degrees)
    pub field_angle: Option<f64>,
    /// Half of the beam angle (degrees)
    pub half_beam_angle: Option<f64>,
    /// Half of the field angle (degrees)
    pub half_field_angle: Option<f64>,
    /// Signed γ of the left (C+180) and right (C) 50 % edges (degrees)
    pub beam_edges: Option<(f64, f64)>,
    /// Signed γ of the left (C+180) and right (C) 10 % edges (degrees)
    pub field_edges: Option<(f64, f64)>,
    /// C angle of the right-hand side of the drawn section
    pub section_c: f64,
    /// Mounting height in meters
    pub mounting_height: f64,
    /// Beam footprint width on the floor in the drawn section (meters);
    /// `None` when an edge points at or above the horizon
    pub beam_diameter: Option<f64>,
    /// Field footprint width on the floor in the drawn section (meters)
    pub field_diameter: Option<f64>,
    /// Maximum intensity (cd/klm)
    pub max_intensity: f64,
    /// Luminaire name for display
    pub luminaire_name: String,
    /// Whether the C0–C180 and C90–C270 beam angles agree within 5 %
    pub is_symmetric: bool,
    /// Beam angle of the C0–C180 section
    pub beam_angle_c0: Option<f64>,
    /// Beam angle of the C90–C270 section
    pub beam_angle_c90: Option<f64>,
    /// Selected C-plane: None = section through the peak, Some(angle) = that section
    pub selected_c_plane: Option<f64>,
}

impl ConeDiagram {
    /// Generate cone diagram data from Eulumdat, drawn in the section
    /// through the peak direction (C0–C180 for a peak on the vertical axis).
    ///
    /// # Arguments
    /// * `ldt` - The Eulumdat data
    /// * `mounting_height` - The mounting height in meters
    pub fn from_eulumdat(ldt: &Eulumdat, mounting_height: f64) -> Self {
        let section = PhotometricCalculations::peak_section(ldt);
        Self::from_section(ldt, mounting_height, section, None)
    }

    /// Generate cone diagram data for the section C–(C+180).
    pub fn from_eulumdat_for_plane(ldt: &Eulumdat, mounting_height: f64, c_plane: f64) -> Self {
        let section = PhotometricCalculations::section_beam_field(ldt, c_plane);
        Self::from_section(ldt, mounting_height, section, Some(c_plane))
    }

    fn from_section(
        ldt: &Eulumdat,
        mounting_height: f64,
        section: SectionBeamField,
        selected_c_plane: Option<f64>,
    ) -> Self {
        let sections = PhotometricCalculations::beam_field_sections(ldt);
        let beam_angle_c0 = sections.c0_c180.beam;
        let beam_angle_c90 = sections.c90_c270.beam;
        let is_symmetric = match (beam_angle_c0, beam_angle_c90) {
            (Some(a), Some(b)) => (a - b).abs() < a.max(b).max(1.0) * 0.05,
            (None, None) => true,
            _ => false,
        };

        Self {
            beam_angle: section.beam,
            field_angle: section.field,
            half_beam_angle: section.beam.map(|a| a / 2.0),
            half_field_angle: section.field.map(|a| a / 2.0),
            beam_edges: section.beam_edges,
            field_edges: section.field_edges,
            section_c: section.c_plane,
            mounting_height,
            beam_diameter: section
                .beam_edges
                .and_then(|e| footprint_width(e, mounting_height)),
            field_diameter: section
                .field_edges
                .and_then(|e| footprint_width(e, mounting_height)),
            max_intensity: ldt.max_intensity(),
            luminaire_name: ldt.luminaire_name.clone(),
            is_symmetric,
            beam_angle_c0,
            beam_angle_c90,
            selected_c_plane,
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

    /// Beam footprint width at a distance from the luminaire (meters).
    pub fn beam_diameter_at(&self, distance: f64) -> Option<f64> {
        self.beam_edges.and_then(|e| footprint_width(e, distance))
    }

    /// Field footprint width at a distance from the luminaire (meters).
    pub fn field_diameter_at(&self, distance: f64) -> Option<f64> {
        self.field_edges.and_then(|e| footprint_width(e, distance))
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
    /// - No downward beam: no beam angle in the section
    pub fn beam_classification(&self) -> &'static str {
        let Some(beam_angle) = self.beam_angle else {
            return "No downward beam";
        };
        if beam_angle < 30.0 {
            "Very Narrow Spot"
        } else if beam_angle < 50.0 {
            "Narrow Spot"
        } else if beam_angle < 70.0 {
            "Spot"
        } else if beam_angle < 90.0 {
            "Medium Flood"
        } else if beam_angle < 120.0 {
            "Wide Flood"
        } else {
            "Very Wide Flood"
        }
    }

    /// Generate spacing recommendations (beam edge to beam edge)
    ///
    /// Returns recommended spacing for different overlap percentages; empty
    /// when the beam has no floor footprint.
    pub fn spacing_recommendations(&self) -> Vec<(f64, f64)> {
        let Some(d) = self.beam_diameter else {
            return Vec::new();
        };
        // Returns (overlap_percent, spacing_meters)
        vec![
            (0.0, d),         // No overlap (beam to beam)
            (25.0, d * 0.75), // 25% overlap
            (50.0, d * 0.5),  // 50% overlap (recommended)
            (75.0, d * 0.25), // 75% overlap (high uniformity)
        ]
    }
}

/// Width of the floor footprint between two signed edge angles at distance
/// `h` below the luminaire; `None` if an edge points at or above the horizon.
fn footprint_width((left, right): (f64, f64), h: f64) -> Option<f64> {
    const MAX_EDGE: f64 = 89.0;
    if left.abs() >= MAX_EDGE || right.abs() >= MAX_EDGE {
        return None;
    }
    Some(h * (right.to_radians().tan() - left.to_radians().tan()))
}

/// A row in the illuminance table showing beam/field diameters and illuminance at a given height.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ConeIlluminanceRow {
    /// Distance from luminaire in meters
    pub height: f64,
    /// Beam footprint width at this height (meters)
    pub beam_diameter: Option<f64>,
    /// Field footprint width at this height (meters)
    pub field_diameter: Option<f64>,
    /// Illuminance at nadir (directly below, lux)
    pub e_nadir: f64,
    /// Illuminance at the C0-side 50 % edge of the C0–C180 section (lux)
    pub e_beam_c0: Option<f64>,
    /// Illuminance at the C90-side 50 % edge of the C90–C270 section (lux)
    pub e_beam_c90: Option<f64>,
}

/// Multi-height illuminance table computed from photometric data.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ConeIlluminanceTable {
    /// Rows at different heights
    pub rows: Vec<ConeIlluminanceRow>,
    /// Lamp flux of the primary lamp set used for the calculation (lumens)
    pub total_flux: f64,
}

impl ConeIlluminanceTable {
    /// Generate an illuminance table using the section through the peak.
    ///
    /// # Arguments
    /// * `ldt` - Eulumdat data
    /// * `step` - Height increment in meters
    /// * `max_height` - Maximum height in meters
    pub fn from_eulumdat(ldt: &Eulumdat, step: f64, max_height: f64) -> Self {
        let sections = PhotometricCalculations::beam_field_sections(ldt);
        let edge_c0 = sections.c0_c180.beam_edges.map(|(_, r)| r);
        let edge_c90 = sections.c90_c270.beam_edges.map(|(_, r)| r);
        let section = PhotometricCalculations::peak_section(ldt);
        Self::build(
            ldt,
            step,
            max_height,
            section,
            (0.0, edge_c0),
            (90.0, edge_c90),
        )
    }

    /// Generate an illuminance table for the section C–(C+180), with the
    /// edge illuminance in C and in the perpendicular plane C+90.
    pub fn from_eulumdat_for_plane(
        ldt: &Eulumdat,
        step: f64,
        max_height: f64,
        c_plane: f64,
    ) -> Self {
        let section = PhotometricCalculations::section_beam_field(ldt, c_plane);
        let perp_plane = (c_plane + 90.0) % 360.0;
        let perp = PhotometricCalculations::section_beam_field(ldt, perp_plane);
        let edge = section.beam_edges.map(|(_, r)| r);
        let edge_perp = perp.beam_edges.map(|(_, r)| r);
        Self::build(
            ldt,
            step,
            max_height,
            section,
            (c_plane, edge),
            (perp_plane, edge_perp),
        )
    }

    fn build(
        ldt: &Eulumdat,
        step: f64,
        max_height: f64,
        section: SectionBeamField,
        edge_a: (f64, Option<f64>),
        edge_b: (f64, Option<f64>),
    ) -> Self {
        let total_flux = ldt.luminous_flux();
        if total_flux <= 0.0 {
            return Self {
                rows: Vec::new(),
                total_flux,
            };
        }

        // Intensity in cd/klm → to get cd we multiply by (total_flux / 1000)
        let flux_scale = total_flux / 1000.0;
        // E at nadir: I(0°) in cd/klm × flux_scale / h²; cos³ = 1 at nadir
        let i_nadir = ldt.sample(0.0, 0.0);

        let mut rows = Vec::new();
        let mut h = step;
        while h <= max_height + 0.001 {
            let edge_e = |(c, gamma): (f64, Option<f64>)| {
                gamma.map(|g| Self::illuminance_at_angle(ldt, c, g, h, flux_scale))
            };
            rows.push(ConeIlluminanceRow {
                height: h,
                beam_diameter: section.beam_edges.and_then(|e| footprint_width(e, h)),
                field_diameter: section.field_edges.and_then(|e| footprint_width(e, h)),
                e_nadir: i_nadir * flux_scale / (h * h),
                e_beam_c0: edge_e(edge_a),
                e_beam_c90: edge_e(edge_b),
            });
            h += step;
        }

        Self { rows, total_flux }
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

        // A nadir-peaked downlight has a beam and field angle and a footprint
        let beam = cone.beam_angle.expect("beam angle");
        let field = cone.field_angle.expect("field angle");
        assert!(beam > 0.0 && field > beam);
        assert!(cone.beam_diameter.expect("beam footprint") > 0.0);
        assert!(cone.field_diameter.expect("field footprint") > 0.0);
        // Symmetric lobe: edges mirror each other
        let (l, r) = cone.beam_edges.unwrap();
        assert!((l + r).abs() < 1e-9);
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
        let half_height_diameter = cone.beam_diameter_at(1.5).unwrap();
        assert!((half_height_diameter - cone.beam_diameter.unwrap() / 2.0).abs() < 0.01);
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
        assert!(cone.beam_angle.is_some());
        assert!(cone.field_angle.is_some());
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
                row.e_beam_c0.unwrap() <= row.e_nadir,
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

    #[test]
    fn test_uplight_has_no_cone() {
        let mut ldt = create_test_ldt();
        ldt.g_angles = vec![0.0, 30.0, 60.0, 90.0, 120.0, 150.0, 180.0];
        ldt.intensities = vec![vec![0.0, 0.0, 0.0, 5.0, 40.0, 90.0, 100.0]];
        let cone = ConeDiagram::from_eulumdat(&ldt, 3.0);
        assert_eq!(cone.beam_angle, None);
        assert_eq!(cone.beam_diameter, None);
        assert_eq!(cone.beam_classification(), "No downward beam");
        assert!(cone.spacing_recommendations().is_empty());
    }
}
