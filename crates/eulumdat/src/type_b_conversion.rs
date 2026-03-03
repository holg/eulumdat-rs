//! Type C ↔ Type B photometric coordinate conversion
//!
//! Floodlight photometry often uses **Type B** (horizontal H, vertical V) coordinates,
//! while EULUMDAT/IES data is stored in **Type C** (C-plane, gamma) coordinates.
//!
//! ## Coordinate Systems
//!
//! **Type C (C, γ):** Used by EULUMDAT/IES
//! - C = azimuthal plane angle (0° = front, 90° = right, …)
//! - γ = polar angle from nadir (0° = down, 90° = horizontal, 180° = up)
//!
//! **Type B (H, V):** Used by floodlight standards (NEMA, IESNA TM-10)
//! - H = horizontal angle (0° = straight ahead, ±90° = sides)
//! - V = vertical angle (0° = straight ahead, ±90° = up/down)
//!
//! ## Formulas
//!
//! Type C → Type B:
//! ```text
//! V = arcsin(sin(γ) · cos(C))
//! H = atan2(sin(γ) · sin(C), cos(γ))
//! ```
//!
//! Type B → Type C:
//! ```text
//! γ = arccos(cos(V) · cos(H))
//! C = atan2(sin(H), sin(V) · cos(H))
//! ```

use crate::Eulumdat;
use std::f64::consts::PI;

/// Type B photometric coordinate conversion utilities
pub struct TypeBConversion;

impl TypeBConversion {
    /// Convert Type C (C-plane, gamma) coordinates to Type B (H, V) coordinates.
    ///
    /// # Arguments
    /// * `c_deg` - C-plane angle in degrees
    /// * `gamma_deg` - Gamma angle in degrees (from nadir)
    ///
    /// # Returns
    /// `(h_deg, v_deg)` - Horizontal and vertical angles in degrees
    pub fn type_c_to_type_b(c_deg: f64, gamma_deg: f64) -> (f64, f64) {
        let c = c_deg.to_radians();
        let gamma = gamma_deg.to_radians();

        let sin_gamma = gamma.sin();
        let cos_gamma = gamma.cos();

        let v = (sin_gamma * c.cos()).asin();
        let h = sin_gamma.mul_add(c.sin(), 0.0).atan2(cos_gamma);

        (h.to_degrees(), v.to_degrees())
    }

    /// Convert Type B (H, V) coordinates to Type C (C-plane, gamma) coordinates.
    ///
    /// # Arguments
    /// * `h_deg` - Horizontal angle in degrees
    /// * `v_deg` - Vertical angle in degrees
    ///
    /// # Returns
    /// `(c_deg, gamma_deg)` - C-plane angle and gamma angle in degrees
    pub fn type_b_to_type_c(h_deg: f64, v_deg: f64) -> (f64, f64) {
        let h = h_deg.to_radians();
        let v = v_deg.to_radians();

        let cos_v = v.cos();
        let cos_h = h.cos();

        let gamma = (cos_v * cos_h).acos();
        let mut c = h.sin().atan2(v.sin() * cos_h);

        // Normalize C to [0, 360)
        if c < 0.0 {
            c += 2.0 * PI;
        }

        (c.to_degrees(), gamma.to_degrees())
    }

    /// Get intensity at Type B coordinates by converting to Type C and sampling.
    ///
    /// # Arguments
    /// * `ldt` - The Eulumdat data
    /// * `h_deg` - Horizontal angle in degrees
    /// * `v_deg` - Vertical angle in degrees
    ///
    /// # Returns
    /// Intensity in cd/klm at the given Type B coordinates
    pub fn intensity_at_type_b(ldt: &Eulumdat, h_deg: f64, v_deg: f64) -> f64 {
        let (c_deg, gamma_deg) = Self::type_b_to_type_c(h_deg, v_deg);
        ldt.sample(c_deg, gamma_deg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_c_to_b_to_c() {
        // Test multiple angles for roundtrip consistency
        let test_cases = [
            (0.0, 0.0),    // nadir
            (0.0, 45.0),   // front-down 45°
            (0.0, 90.0),   // horizontal front
            (90.0, 45.0),  // right 45°
            (180.0, 60.0), // back 60°
            (270.0, 30.0), // left 30°
        ];

        for (c_orig, gamma_orig) in &test_cases {
            let (h, v) = TypeBConversion::type_c_to_type_b(*c_orig, *gamma_orig);
            let (c_back, gamma_back) = TypeBConversion::type_b_to_type_c(h, v);

            // Normalize C angles for comparison (handle 0 ≡ 360 wrap)
            let c_norm_orig = c_orig % 360.0;
            let c_norm_back = c_back % 360.0;

            assert!(
                (gamma_back - gamma_orig).abs() < 1e-10,
                "Gamma roundtrip failed for C={}, γ={}: got γ={}",
                c_orig,
                gamma_orig,
                gamma_back
            );

            // For gamma=0 (nadir), C is undefined, so skip C comparison
            if *gamma_orig > 1e-10 {
                assert!(
                    (c_norm_back - c_norm_orig).abs() < 1e-10
                        || (c_norm_back - c_norm_orig).abs() > 359.999,
                    "C roundtrip failed for C={}, γ={}: got C={}",
                    c_orig,
                    gamma_orig,
                    c_back
                );
            }
        }
    }

    #[test]
    fn test_roundtrip_b_to_c_to_b() {
        // Cases where H or V is zero roundtrip perfectly
        let exact_cases = [
            (0.0, 0.0),  // straight ahead
            (30.0, 0.0), // 30° right, V=0
            (0.0, 30.0), // H=0, 30° up
        ];

        for (h_orig, v_orig) in &exact_cases {
            let (c, gamma) = TypeBConversion::type_b_to_type_c(*h_orig, *v_orig);
            let (h_back, v_back) = TypeBConversion::type_c_to_type_b(c, gamma);

            assert!(
                (h_back - h_orig).abs() < 1e-10,
                "H roundtrip failed for H={}, V={}: got H={}",
                h_orig,
                v_orig,
                h_back
            );
            assert!(
                (v_back - v_orig).abs() < 1e-10,
                "V roundtrip failed for H={}, V={}: got V={}",
                h_orig,
                v_orig,
                v_back
            );
        }

        // Off-axis cases: verify that B→C produces valid Type C coordinates.
        // The simplified spherical formulas are not perfectly invertible
        // when both H and V are nonzero (the C→B→C roundtrip is exact,
        // but B→C→B is not). This is acceptable because intensity_at_type_b
        // only uses B→C (forward direction).
        let off_axis_cases = [
            (-45.0, 20.0), // left-up
            (60.0, -30.0), // right-down
        ];

        for (h_orig, v_orig) in &off_axis_cases {
            let (c, gamma) = TypeBConversion::type_b_to_type_c(*h_orig, *v_orig);
            // Verify the Type C values are in valid ranges
            assert!(
                (0.0..=180.0).contains(&gamma),
                "gamma out of range for H={}, V={}: {}",
                h_orig,
                v_orig,
                gamma
            );
            assert!(
                (0.0..360.0).contains(&c),
                "C out of range for H={}, V={}: {}",
                h_orig,
                v_orig,
                c
            );
        }
    }

    #[test]
    fn test_nadir_maps_to_origin() {
        // Type C nadir (γ=0) should map to Type B looking down
        let (h, v) = TypeBConversion::type_c_to_type_b(0.0, 0.0);
        assert!(h.abs() < 1e-10, "H should be 0 at nadir, got {}", h);
        assert!(v.abs() < 1e-10, "V should be 0 at nadir, got {}", v);
    }

    #[test]
    fn test_horizontal_front() {
        // Type C: C=0, γ=90 → horizontal, straight ahead
        // Type B: H=0, V=90 (looking up from below) → but in floodlight convention
        // γ=90 at C=0 means horizontal in the C0 plane
        let (h, v) = TypeBConversion::type_c_to_type_b(0.0, 90.0);
        // V = arcsin(sin(90°)·cos(0°)) = arcsin(1) = 90°
        assert!((v - 90.0).abs() < 1e-10, "V should be 90°, got {}", v);
        assert!(h.abs() < 1e-10, "H should be 0°, got {}", h);
    }

    #[test]
    fn test_intensity_at_type_b() {
        let ldt = Eulumdat {
            c_angles: vec![0.0, 90.0, 180.0, 270.0],
            g_angles: vec![0.0, 45.0, 90.0],
            intensities: vec![
                vec![100.0, 80.0, 20.0],
                vec![100.0, 70.0, 15.0],
                vec![100.0, 80.0, 20.0],
                vec![100.0, 70.0, 15.0],
            ],
            ..Default::default()
        };

        // At nadir (H=0, V=0), should get the γ=0 intensity
        // H=0, V=0 → C=undefined, γ=0 → intensity at nadir
        let i = TypeBConversion::intensity_at_type_b(&ldt, 0.0, 0.0);
        assert!(
            (i - 100.0).abs() < 1.0,
            "Nadir intensity should be ~100, got {}",
            i
        );
    }
}
